use std::sync::Arc;

use cfd_engine_sb_contracts::PendingPositionPersistenceEvent;
use service_sdk::my_telemetry::MyTelemetryContext;
use trading_sdk::mt_engine::{
    create_pending_position, MtEngineError, MtPosition, MtPositionOpenPendingCommand,
    MtPositionPendingState, MtPositionSide,
};
use uuid::Uuid;

use crate::{
    map_pending_to_sb_model,
    position_manager_grpc::{PositionManagerOpenPendingGrpcRequest, PositionManagerPositionSide},
    AppContext,
};

pub async fn open_pending(
    app: &Arc<AppContext>,
    request: PositionManagerOpenPendingGrpcRequest,
    telemetry: &MyTelemetryContext,
) -> Result<MtPosition<MtPositionPendingState>, MtEngineError> {
    let id = match &request.id {
        Some(src) => src.clone(),
        None => Uuid::new_v4().to_string(),
    };

    let side: MtPositionSide = match request.side() {
        PositionManagerPositionSide::Buy => MtPositionSide::Buy,
        PositionManagerPositionSide::Sell => MtPositionSide::Sell,
    };

    let reed = app.active_prices_cache.read().await;

    let pending_position_command = MtPositionOpenPendingCommand {
        id,
        trader_id: request.trader_id,
        account_id: request.account_id,
        side,
        asset_pair: request.asset_pair,
        base: request.base,
        quote: request.quote,
        collateral: request.collateral_currency,
        invest_amount: request.invest_amount,
        leverage: request.leverage,
        stop_out_percent: request.stop_out_percent,
        process_id: request.process_id.clone(),
        tp_profit: request.tp_in_profit,
        tp_price: request.tp_in_asset_price,
        sl_profit: request.sl_in_profit,
        sl_price: request.sl_in_asset_price,
        desired_open_price: request.desire_price,
    };

    let position = create_pending_position(pending_position_command, &reed)?;

    app.pending_positions_cache
        .write()
        .await
        .0
        .add_position(position.clone());

    let sb_event = PendingPositionPersistenceEvent {
        process_id: request.process_id.clone(),
        cancel: None,
        execute: None,
        create: Some(map_pending_to_sb_model(position.clone())),
    };

    app.pending_positions_persistence_publisher
        .publish(&sb_event, Some(telemetry))
        .await
        .unwrap();

    return Ok(position);
}
