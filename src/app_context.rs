use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use rust_extensions::AppStates;
use service_sdk::{
    my_grpc_extensions::GrpcClientSettings,
    my_service_bus::abstractions::publisher::MyServiceBusPublisher, ServiceContext,
};
use tokio::sync::RwLock;

use crate::{
    ActivePositionsCache, ActivePricesCache, EngineBidAsk, EnginePosition, SettingsReader,
};

pub struct AppContext {
    pub active_positions_cache: Arc<RwLock<ActivePositionsCache<EnginePosition<EngineBidAsk>>>>,
    pub active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
    pub app_states: Arc<AppStates>,
    pub persistence_queue: MyServiceBusPublisher<PositionPersistenceEvent>,
}

impl AppContext {
    pub async fn new(settings: &Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        let active_prices_cache = crate::flows::load_prices_cache(settings).await;
        Self {
            active_positions_cache: Arc::new(RwLock::new(
                crate::flows::load_positions(settings, active_prices_cache.clone()).await,
            )),
            active_prices_cache,
            app_states: Arc::new(AppStates::create_initialized()),
            persistence_queue: service_context.get_sb_publisher(false).await,
        }
    }
}
pub struct GrpcSettings(String);

impl GrpcSettings {
    pub fn new_arc(url: String) -> Arc<Self> {
        Arc::new(Self(url))
    }
}

#[tonic::async_trait]
impl GrpcClientSettings for GrpcSettings {
    async fn get_grpc_url(&self, name: &'static str) -> String {
        self.0.clone()
    }
}
