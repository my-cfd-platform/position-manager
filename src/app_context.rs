use std::{sync::Arc, thread::sleep, time::Duration};

use cfd_engine_sb_contracts::{
    PendingOrderNeedApproveEvent, PendingPositionPersistenceEvent, PositionManagerPositionMarginCallHit, PositionPersistenceEvent, PositionToppingUpEvent
};
use service_sdk::{
    my_service_bus::abstractions::publisher::MyServiceBusPublisher,
    my_telemetry::MyTelemetryContext, rust_extensions::{AppStates, StopWatch}, ServiceContext,
};
use tokio::sync::RwLock;

use crate::{PositionManagerPersistenceClient, SettingsReader};

use trading_sdk::mt_engine::{ActivePositionsCache, MtBidAskCache, PendingPositionsCache};

pub struct AppContext {
    pub active_positions_cache: Arc<RwLock<ActivePositionsCache>>,
    pub pending_execute_to_confirm_positions: Arc<RwLock<PendingPositionsCache>>,
    pub pending_positions_cache: Arc<RwLock<PendingPositionsCache>>,
    pub active_prices_cache: Arc<RwLock<MtBidAskCache>>,
    pub app_states: Arc<AppStates>,
    pub active_positions_persistence_publisher: MyServiceBusPublisher<PositionPersistenceEvent>,
    pub pending_need_confirm_publisher: MyServiceBusPublisher<PendingOrderNeedApproveEvent>,
    pub pending_positions_persistence_publisher:
        MyServiceBusPublisher<PendingPositionPersistenceEvent>,
    pub margin_call_publisher: MyServiceBusPublisher<PositionManagerPositionMarginCallHit>,
    pub topping_up_publisher: MyServiceBusPublisher<PositionToppingUpEvent>,
    pub debug: bool,
}

impl AppContext {
    pub async fn new(settings: &Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        let (active_prices_cache, active_positions_cache, pending_positions_cache) =
            load_data(settings).await;

        Self {
            active_prices_cache,
            pending_positions_cache,
            active_positions_cache,
            pending_execute_to_confirm_positions: Arc::new(RwLock::new(PendingPositionsCache::new())),
            app_states: Arc::new(AppStates::create_initialized()),
            active_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
            pending_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
            margin_call_publisher: service_context.get_sb_publisher(false).await,
            topping_up_publisher: service_context.get_sb_publisher(false).await,
            pending_need_confirm_publisher: service_context.get_sb_publisher(false).await,
            debug: std::env::var("DEBUG").is_ok(),
        }
    }
}

async fn load_data(
    settings: &Arc<SettingsReader>,
) -> (
    Arc<RwLock<MtBidAskCache>>,
    Arc<RwLock<ActivePositionsCache>>,
    Arc<RwLock<PendingPositionsCache>>,
) {
    let mut sw = StopWatch::new();
    sw.start();
    let telemetry = MyTelemetryContext::new();
    telemetry.start_event_tracking("PositionManagerPersistenceClient initialization");
    let grpc_client = PositionManagerPersistenceClient::new(settings.clone());
    sleep(Duration::from_secs(3));
    telemetry.start_event_tracking("Load start data");
    let active_prices_cache = crate::flows::load_prices_cache(&grpc_client, &telemetry).await;

    let (positions_cache, pending_positions_cache) = {
        let prices_reed = &active_prices_cache.read().await;

        let positions_cache =
            crate::flows::load_positions(&grpc_client, &prices_reed, &telemetry).await;

        let pending_positions_cache =
            crate::flows::load_pending_positions(&grpc_client, &prices_reed, &telemetry).await;

        (positions_cache, pending_positions_cache)
    };

    sw.pause();

    println!("Data loaded in: {} ms", sw.duration().as_millis());

    return (
        active_prices_cache,
        Arc::new(RwLock::new(positions_cache)),
        Arc::new(RwLock::new(pending_positions_cache)),
    );
}
