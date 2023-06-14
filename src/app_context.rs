use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use my_service_bus_abstractions::publisher::MyServiceBusPublisher;
use rust_extensions::{date_time::DateTimeAsMicroseconds, AppStates};
use service_sdk::ServiceContext;
use tokio::sync::RwLock;

use crate::{
    ActivePositionsCache, ActivePricesCache, EngineBidAsk, EnginePosition,
    PositionManagerPersistenceClient, SettingsReader,
};

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub active_positions_cache: Arc<RwLock<ActivePositionsCache<EnginePosition<EngineBidAsk>>>>,
    pub active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
    pub app_states: Arc<AppStates>,
    pub persistence_queue: MyServiceBusPublisher<PositionPersistenceEvent>,
}

impl AppContext {
    pub async fn new(settings: &Arc<SettingsReader>, service_context: &ServiceContext) -> Self {
        let active_prices_cache = Arc::new(RwLock::new(ActivePricesCache::new()));
        Self {
            active_positions_cache: Arc::new(RwLock::new(
                load_positions(settings, active_prices_cache.clone()).await,
            )),
            active_prices_cache,
            app_states: Arc::new(AppStates::create_initialized()),
            persistence_queue: service_context.get_sb_publisher(false).await,
        }
    }
}

pub async fn load_positions(
    settings: &Arc<SettingsReader>,
    active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
) -> ActivePositionsCache<EnginePosition<EngineBidAsk>> {
    let settings = settings.get_settings().await;
    let mut cache = ActivePositionsCache::new();

    let persistence_grpc = PositionManagerPersistenceClient::new(
        settings.persistence_url.clone(),
        active_prices_cache,
    )
    .await;

    let positions = persistence_grpc.get_active_positions().await;
    let pos_len = positions.len();
    println!(
        "Got {} positions. {}",
        pos_len,
        DateTimeAsMicroseconds::now().unix_microseconds
    );
    cache.load_positions(positions);
    println!(
        "Done loading {} positions. {}",
        pos_len,
        DateTimeAsMicroseconds::now().unix_microseconds
    );
    return cache;
}
