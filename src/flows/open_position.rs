use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use service_sdk::my_telemetry::MyTelemetryContext;
use trading_sdk::mt_engine::{
    make_active_position, MtEngineError, MtPosition, MtPositionActiveState, MtPositionOpenCommand,
};
use uuid::Uuid;

use crate::{
    map_active_to_sb_model,
    position_manager_grpc::{PositionManagerOpenPositionGrpcRequest, PositionManagerPositionSide},
    AppContext,
};

pub async fn open_position(
    app: &Arc<AppContext>,
    request: PositionManagerOpenPositionGrpcRequest,
    telemetry: &MyTelemetryContext,
) -> Result<MtPosition<MtPositionActiveState>, MtEngineError> {
    let prices_cache = app.active_prices_cache.read().await;

    let id = match &request.id {
        Some(src) => src.clone(),
        None => Uuid::new_v4().to_string(),
    };

    let side: PositionManagerPositionSide =
        PositionManagerPositionSide::try_from(request.side).unwrap();

    let trader_id = request.trader_id.clone();
    let account_id = request.trader_id.clone();
    let process_id = request.process_id.clone();

    let open_command = MtPositionOpenCommand {
        id: id.clone(),
        trader_id: request.trader_id,
        account_id: request.account_id,
        side: side.into(),
        asset_pair: request.asset_pair,
        base: request.base,
        quote: request.quote,
        collateral: request.collateral_currency,
        invest_amount: request.invest_amount,
        leverage: request.leverage,
        stop_out_percent: request.stop_out_percent,
        process_id: request.process_id.clone(),
        pending_state: None,
        tp_profit: request.tp_in_profit,
        tp_price: request.tp_in_asset_price,
        sl_profit: request.sl_in_profit,
        sl_price: request.sl_in_asset_price,
    };

    let position = make_active_position(open_command.clone(), &prices_cache)?;

    trade_log::trade_log!(
        &trader_id,
        &account_id,
        &process_id,
        &id,
        "Executing open position request",
        telemetry.clone(),
        "open_command" = &open_command,
        "active_position" = &position
    );

    let mut positions_cache = app.active_positions_cache.write().await;

    positions_cache.0.add_position(position.clone());

    let sb_model = PositionPersistenceEvent {
        process_id: request.process_id,
        update_position: None,
        close_position: None,
        create_position: Some(map_active_to_sb_model(position.clone())),
    };

    app.active_positions_persistence_publisher
        .publish(&sb_model, Some(telemetry))
        .await
        .unwrap();

    return Ok(position);
}
