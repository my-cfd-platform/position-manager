use std::sync::Arc;

use cfd_engine_sb_contracts::PositionPersistenceEvent;
use service_sdk::my_telemetry::MyTelemetryContext;

use crate::{
    AppContext, EngineBidAsk, EngineError, EnginePosition, ExecutionClosePositionReason,
    ExecutionPositionBase,
};

pub async fn close_position(
    app: &Arc<AppContext>,
    _: &str,
    position_id: &str,
    close_position_reason: ExecutionClosePositionReason,
    process_id: &str,
    telemetry: &MyTelemetryContext,
) -> Result<EnginePosition<EngineBidAsk>, EngineError> {
    let mut position_to_close: Option<EnginePosition<EngineBidAsk>> = None;
    {
        let active_cache = app.active_positions_cache.read().await;
        let position = active_cache.get_position_by_id(&position_id);

        let Some(cache_position) = position else {
            return Err(EngineError::PositionNotFound);
        };

        position_to_close = Some(cache_position.clone());
    }

    let Some(position_to_close) = position_to_close else {
        return Err(EngineError::PositionNotFound);
    };

    let reed = app.active_prices_cache.read().await;

    let Some(_) = reed.get_by_id(position_to_close.get_asset_pair()) else {
        return Err(EngineError::NoLiquidity);
    };

    let mut active_cache = app.active_positions_cache.write().await;

    let Some(removed_position) = active_cache.remove_position(position_to_close.get_id()) else {
        return Err(EngineError::PositionNotFound);
    };

    let closed_position = removed_position.close_position(process_id, close_position_reason);

    let sb_event = PositionPersistenceEvent {
        process_id: process_id.to_string(),
        update_position: None,
        close_position: Some(closed_position.clone().into()),
        create_position: None,
    };

    app.persistence_queue
        .publish(&sb_event, Some(telemetry))
        .await
        .unwrap();

    return Ok(closed_position);
}
