use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use rust_extensions::date_time::DateTimeAsMicroseconds;
use service_sdk::my_telemetry::MyTelemetryContext;
use uuid::Uuid;

use crate::{
    get_close_price, get_open_price, position_manager_grpc::PositionManagerOpenPositionGrpcRequest,
    ActivePositionState, AppContext, EngineBidAsk, EngineError, EnginePosition, EnginePositionBase,
    EnginePositionState, PositionSide,
};

pub async fn open_position(
    app: &Arc<AppContext>,
    request: PositionManagerOpenPositionGrpcRequest,
    telemetry: &MyTelemetryContext,
) -> Result<EnginePosition<EngineBidAsk>, EngineError> {
    let mut active_cache = app.active_positions_cache.write().await;
    let active_position = make_active_order_from_request(app, request.clone()).await?;
    active_cache.add_position(active_position.clone());

    let sb_event = PositionPersistenceEvent {
        process_id: request.process_id,
        update_position: None,
        close_position: None,
        create_position: Some(active_position.clone().into()),
    };

    app.persistence_queue.publish(&sb_event, Some(telemetry)).await.unwrap();

    return Ok(active_position);
}

pub async fn make_active_order_from_request(
    app: &Arc<AppContext>,
    request: PositionManagerOpenPositionGrpcRequest,
) -> Result<EnginePosition<EngineBidAsk>, EngineError> {
    let pos_id = match &request.id {
        Some(src) => src.clone(),
        None => Uuid::new_v4().to_string(),
    };

    let date = DateTimeAsMicroseconds::now();
    let side = PositionSide::from(request.side);
    let read = app.active_prices_cache.read().await;

    let Some(active_asset_bid_ask) = read.get_by_id(&request.asset_pair) else{
        println!("base asset price not found. Request: {:?}", request);
        return Err(EngineError::NoLiquidity);
    };

    let (collateral_base_price, collateral_base_bid_ask) =
        match read.get_by_currencies(&request.base, &request.collateral_currency) {
            Some(src) => (
                get_open_price(src.as_ref(), &side),
                Some(src.as_ref().clone()),
            ),
            None => (1.0, None),
        };

    let (collateral_quote_price, collateral_quote_bid_ask) =
        match read.get_by_currencies(&request.quote, &request.collateral_currency) {
            Some(src) => (
                get_close_price(src.as_ref(), &side),
                Some(src.as_ref().clone()),
            ),
            None => (1.0, None),
        };

    if request.base != request.collateral_currency && collateral_base_bid_ask.is_none() {
        println!(
            "base collateral_currency price not found. Request: {:?}",
            request
        );
        return Err(EngineError::NoLiquidity);
    }

    if request.quote != request.collateral_currency && collateral_quote_bid_ask.is_none() {
        println!(
            "base collateral_currency price not found. Request: {:?}",
            request
        );
        return Err(EngineError::NoLiquidity);
    }

    let base_data = EnginePositionBase {
        id: pos_id,
        asset_pair: request.asset_pair,
        side: PositionSide::from(request.side),
        invest_amount: request.invest_amount,
        leverage: request.leverage,
        stop_out_percent: request.stop_out_percent,
        create_process_id: request.process_id.clone(),
        create_date: date.clone(),
        last_update_process_id: request.process_id.clone(),
        last_update_date: date,
        take_profit_in_position_profit: request.tp_in_profit,
        take_profit_in_asset_price: request.tp_in_asset_price,
        stop_loss_in_position_profit: request.sl_in_profit,
        stop_loss_in_asset_price: request.sl_in_asset_price,
        account_id: request.account_id.clone(),
        trader_id: request.trader_id.clone(),
        collateral_currency: request.collateral_currency.clone(),
        base: request.base.clone(),
        quote: request.quote.clone(),
        swaps: crate::EnginePositionSwaps::default(),
    };

    let state: ActivePositionState<EngineBidAsk> = ActivePositionState {
        asset_open_price: get_open_price(active_asset_bid_ask.as_ref(), &base_data.side),
        asset_open_bid_ask: active_asset_bid_ask.as_ref().clone(),
        collateral_base_open_price: collateral_base_price,
        collateral_base_open_bid_ask: collateral_base_bid_ask,
        open_process_id: request.process_id,
        open_date: DateTimeAsMicroseconds::now(),
        asset_last_price: get_open_price(active_asset_bid_ask.as_ref(), &base_data.side),
        asset_last_bid_ask: active_asset_bid_ask.as_ref().clone(),
        collateral_quote_last_price: collateral_quote_price,
        collateral_quote_last_bid_ask: collateral_quote_bid_ask,
        profit: 0.0,
        pending_state: None,
    };

    let state = EnginePositionState::Active(state);

    return Ok(EnginePosition {
        data: base_data,
        state,
    });
}
