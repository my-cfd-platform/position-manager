use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use service_sdk::my_telemetry::MyTelemetryContext;
use trading_sdk::mt_engine::{
    convert_position_to_closed, ActivePositionsCache, MtPosition, MtPositionCloseReason,
    MtPositionClosedState,
};

use crate::{map_closed_to_sb, AppContext, EngineError};

pub async fn close_position(
    app: &Arc<AppContext>,
    trader_id: &str,
    account_id: &str,
    position_id: &str,
    close_position_reason: MtPositionCloseReason,
    process_id: &str,
    telemetry: &MyTelemetryContext,
) -> Result<MtPosition<MtPositionClosedState>, EngineError> {
    let mut cache = app.active_positions_cache.write().await;

    let active_position = cache
        .0
        .remove_position(position_id)
        .ok_or(EngineError::PositionNotFound)?;

    let closed = convert_position_to_closed(
        active_position.clone(),
        close_position_reason,
        process_id.to_string(),
    );

    trade_log::trade_log!(
        trader_id,
        account_id,
        process_id,
        position_id,
        "Executing close position",
        telemetry.clone(),
        "active_position" = &active_position,
        "closed_position" = &closed
    );

    let sb_model = map_closed_to_sb(&closed);

    let sb_event = PositionPersistenceEvent {
        process_id: process_id.to_string(),
        update_position: None,
        close_position: Some(sb_model),
        create_position: None,
    };

    app.active_positions_persistence_publisher
        .publish(&sb_event, Some(telemetry))
        .await
        .unwrap();

    return Ok(closed);
}

pub async fn close_position_background(
    app: &Arc<AppContext>,
    trader_id: &str,
    account_id: &str,
    position_id: &str,
    close_position_reason: MtPositionCloseReason,
    process_id: &str,
    telemetry: &MyTelemetryContext,
    cache: &mut ActivePositionsCache
) -> Result<MtPosition<MtPositionClosedState>, EngineError> {
    let active_position = cache
        .0
        .remove_position(position_id)
        .ok_or(EngineError::PositionNotFound)?;

    let closed = convert_position_to_closed(
        active_position.clone(),
        close_position_reason,
        process_id.to_string(),
    );

    trade_log::trade_log!(
        trader_id,
        account_id,
        process_id,
        position_id,
        "Executing close position background",
        telemetry.clone(),
        "active_position" = &active_position,
        "closed_position" = &closed
    );

    let sb_model = map_closed_to_sb(&closed);

    let sb_event = PositionPersistenceEvent {
        process_id: process_id.to_string(),
        update_position: None,
        close_position: Some(sb_model),
        create_position: None,
    };

    app.active_positions_persistence_publisher
        .publish(&sb_event, None)
        .await
        .unwrap();

    return Ok(closed);
}
