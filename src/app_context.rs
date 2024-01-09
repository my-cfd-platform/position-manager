use std::{sync::Arc, thread::sleep, time::Duration};

use cfd_engine_sb_contracts::{PendingPositionPersistenceEvent, PositionPersistenceEvent};
use service_sdk::{
    my_service_bus::abstractions::publisher::MyServiceBusPublisher,
    my_telemetry::MyTelemetryContext, ServiceContext, rust_extensions::AppStates,
};
use tokio::sync::RwLock;

use crate::{PositionManagerPersistenceClient, SettingsReader};

use trading_sdk::mt_engine::{ActivePositionsCache, MtBidAskCache, PendingPositionsCache};

pub struct AppContext {
    pub active_positions_cache: Arc<RwLock<ActivePositionsCache>>,
    pub pending_positions_cache: Arc<RwLock<PendingPositionsCache>>,
    pub active_prices_cache: Arc<RwLock<MtBidAskCache>>,
    pub app_states: Arc<AppStates>,
    pub active_positions_persistence_publisher: MyServiceBusPublisher<PositionPersistenceEvent>,
    pub pending_positions_persistence_publisher:
        MyServiceBusPublisher<PendingPositionPersistenceEvent>,
}

impl AppContext {
    pub async fn new(settings: &Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        let (active_prices_cache, active_positions_cache, pending_positions_cache) =
            load_data(settings).await;

        Self {
            active_prices_cache,
            pending_positions_cache,
            active_positions_cache,
            app_states: Arc::new(AppStates::create_initialized()),
            active_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
            pending_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
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

    return (
        active_prices_cache,
        Arc::new(RwLock::new(positions_cache)),
        Arc::new(RwLock::new(pending_positions_cache)),
    );
}
