use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use my_grpc_extensions::GrpcClientSettings;
use my_service_bus_abstractions::publisher::MyServiceBusPublisher;
use my_telemetry::MyTelemetryContext;
use rust_extensions::{date_time::DateTimeAsMicroseconds, AppStates};
use service_sdk::ServiceContext;
use tokio::sync::RwLock;

use crate::{
    get_close_price, ActivePositionState, ActivePositionsCache, ActivePricesCache, EngineBidAsk,
    EnginePosition, EnginePositionBase, EnginePositionState, EnginePositionSwap,
    PositionManagerPersistenceClient, PositionSide, SettingsReader,
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

    let persistence_grpc = PositionManagerPersistenceClient::new(GrpcSettings::new_arc(
        settings.persistence_url.clone(),
    ));

    let positions = persistence_grpc
        .get_active_positions((), &MyTelemetryContext::new())
        .await;

    let positions = {
        match positions {
            Ok(src) => match src {
                Some(src) => src,
                None => vec![],
            },
            Err(_) => vec![],
        }
    };

    let prices_cache = active_prices_cache.read().await;

    let positions: Vec<EnginePosition<EngineBidAsk>> = positions
        .iter()
        .map(|x| {
            let data = EnginePositionBase {
                id: x.id.to_owned(),
                trader_id: x.trader_id.to_owned(),
                account_id: x.account_id.to_owned(),
                asset_pair: x.asset_pair.to_owned(),
                side: PositionSide::from(x.side),
                invest_amount: x.invest_amount,
                leverage: x.leverage,
                stop_out_percent: x.stop_out_percent,
                create_process_id: x.create_process_id.to_owned(),
                create_date: x.create_date_unix_timestamp_milis.into(),
                last_update_process_id: x.last_update_process_id.to_owned(),
                last_update_date: x.last_update_date.into(),
                take_profit_in_position_profit: x.tp_in_profit,
                take_profit_in_asset_price: x.tp_in_asset_price,
                stop_loss_in_position_profit: x.sl_in_profit,
                stop_loss_in_asset_price: x.sl_in_asset_price,
                collateral_currency: x.collateral.clone(),
                base: x.base.clone(),
                quote: x.quote.clone(),
                swaps: crate::EnginePositionSwaps {
                    swaps: x
                        .swaps
                        .iter()
                        .map(|x| EnginePositionSwap {
                            date: x.date_time_unix_timestamp_milis.into(),
                            amount: x.amount,
                        })
                        .collect(),
                    total: 0.0,
                },
            };

            let collateral_base_open_bid_ask: Option<EngineBidAsk> =
                match x.collateral_base_open_bid_ask.clone() {
                    Some(src) => Some(src.into()),
                    None => None,
                };

            let asset_active_bid_ask = prices_cache.get_by_id(&x.asset_pair).unwrap();
            let (collateral_quote_price, collateral_quote_bid_ask) = {
                if data.collateral_currency == data.quote {
                    (1.0, None)
                } else {
                    let target_bid_ask = prices_cache.get_by_currencies(&data.base, &data.quote);

                    if target_bid_ask.is_none() {
                        panic!("Not found price for {}/{}", data.base, data.quote);
                    }

                    let target_bid_ask = target_bid_ask.unwrap();

                    let result = (
                        get_close_price(target_bid_ask.as_ref(), &data.side),
                        Some(target_bid_ask.as_ref().clone()),
                    );

                    result
                }
            };

            let active_state = ActivePositionState {
                asset_open_price: x.asset_open_price,
                asset_open_bid_ask: x.asset_open_bid_ask.clone().unwrap().into(),
                collateral_base_open_price: x.collateral_base_open_price,
                collateral_base_open_bid_ask,
                open_process_id: x.open_process_id.clone(),
                open_date: x.open_date_unix_timestamp_milis.into(),
                asset_last_price: get_close_price(asset_active_bid_ask.as_ref(), &data.side),
                asset_last_bid_ask: asset_active_bid_ask.as_ref().clone(),
                collateral_quote_last_price: collateral_quote_price,
                collateral_quote_last_bid_ask: collateral_quote_bid_ask.clone(),
                profit: 0.0,
                pending_state: None,
            };

            let mut position: EnginePosition<EngineBidAsk> = EnginePosition {
                data,
                state: EnginePositionState::Active(active_state),
            };

            position.update_pl();

            position
        })
        .collect();

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
