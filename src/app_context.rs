use std::sync::Arc;

use cfd_engine_sb_contracts::{PendingPositionPersistenceEvent, PositionPersistenceEvent};
use rust_extensions::AppStates;
use service_sdk::{my_service_bus::abstractions::publisher::MyServiceBusPublisher, ServiceContext};
use tokio::sync::RwLock;

use crate::SettingsReader;

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
        let active_prices_cache = crate::flows::load_prices_cache(settings).await;
        Self {
            active_prices_cache,
            app_states: Arc::new(AppStates::create_initialized()),
            active_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
            pending_positions_persistence_publisher: service_context.get_sb_publisher(false).await,
            pending_positions_cache: Arc::new(RwLock::new(PendingPositionsCache::new())),
            active_positions_cache: Arc::new(RwLock::new(ActivePositionsCache::new())),
        }
    }
}
