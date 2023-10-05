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

    let open_command = MtPositionOpenCommand {
        id,
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

    let position = make_active_position(open_command, &prices_cache)?;

    let mut positions_cache = app.active_positions_cache.write().await;

    positions_cache.0.add_position(position.clone());

    let sb_model = PositionPersistenceEvent {
        process_id: request.process_id,
        update_position: None,
        close_position: Some(map_active_to_sb_model(position.clone())),
        create_position: None,
    };

    app.active_positions_persistence_publisher
        .publish(&sb_model, Some(telemetry))
        .await
        .unwrap();

    return Ok(position);
}
