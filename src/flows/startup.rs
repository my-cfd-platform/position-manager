use std::sync::Arc;

use rust_extensions::date_time::DateTimeAsMicroseconds;
use service_sdk::my_telemetry::MyTelemetryContext;
use tokio::sync::RwLock;

use crate::{
    get_close_price, ActivePositionState, ActivePositionsCache, ActivePricesCache, EngineBidAsk,
    EnginePosition, EnginePositionBase, EnginePositionState, GrpcSettings,
    PositionManagerPersistenceClient, PositionSide, SettingsReader,
};

pub async fn load_prices_cache(
    settings: &SettingsReader,
) -> Arc<RwLock<ActivePricesCache<EngineBidAsk>>> {
    println!(
        "Start loading prices. {}",
        DateTimeAsMicroseconds::now().unix_microseconds
    );

    let settings = settings.get_settings().await;
    let persistence_grpc = PositionManagerPersistenceClient::new(GrpcSettings::new_arc(
        settings.persistence_url.clone(),
    ));

    let prices = persistence_grpc
        .get_prices_snapshot((), &MyTelemetryContext::new())
        .await
        .unwrap();

    let Some(prices) = prices else {
        println!("Prices not found. Start with empty cache.");
        return Arc::new(RwLock::new(ActivePricesCache::new(vec![])));
    };

    let prices: Vec<EngineBidAsk> = prices
        .iter()
        .map(|x| EngineBidAsk {
            asset_pair: x.asset_pair.clone(),
            bid: x.bid,
            ask: x.ask,
            datetime: x.date_time_unix_timestamp_milis.into(),
            base: x.base.clone(),
            quote: x.quote.clone(),
        })
        .collect();

    println!(
        "Done load {} prices. {}. Snapshot: \n {:#?}",
        prices.len(),
        DateTimeAsMicroseconds::now().unix_microseconds,
        prices
    );
    return Arc::new(RwLock::new(ActivePricesCache::new(prices)));
}

pub async fn load_positions(
    settings: &SettingsReader,
    active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
) -> ActivePositionsCache<EnginePosition<EngineBidAsk>> {
    println!(
        "Start loading positions. {}",
        DateTimeAsMicroseconds::now().unix_microseconds
    );
    let settings = settings.get_settings().await;
    let mut cache = ActivePositionsCache::new();
    let persistence_grpc = PositionManagerPersistenceClient::new(GrpcSettings::new_arc(
        settings.persistence_url.clone(),
    ));

    let positions = persistence_grpc
        .get_active_positions((), &MyTelemetryContext::new())
        .await
        .unwrap();

    let positions = match positions {
        Some(src) => src,
        None => vec![],
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
                swaps: crate::EnginePositionSwaps::default(),
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
