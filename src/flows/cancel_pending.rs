use std::sync::Arc;

use cfd_engine_sb_contracts::PendingPositionPersistenceEvent;
use service_sdk::my_telemetry::MyTelemetryContext;
use trading_sdk::mt_engine::{MtEngineError, MtPosition, MtPositionPendingState};

use crate::{
    map_pending_to_sb_model, position_manager_grpc::PositionManagerCancelPendingGrpcRequest,
    AppContext,
};

pub async fn cancel_pending(
    app: &Arc<AppContext>,
    request: PositionManagerCancelPendingGrpcRequest,
    telemetry: &MyTelemetryContext,
) -> Result<MtPosition<MtPositionPendingState>, MtEngineError> {
    let removed = {
        let mut write = app.pending_positions_cache.write().await;
        write.0.remove_position(&request.id)
    }
    .ok_or(MtEngineError::PositionNotFound)?;

    let sb_event = PendingPositionPersistenceEvent {
        process_id: uuid::Uuid::new_v4().to_string(),
        cancel: Some(map_pending_to_sb_model(removed.clone())),
        execute: None,
        create: None,
    };

    app.pending_positions_persistence_publisher
        .publish(&sb_event, Some(telemetry))
        .await
        .unwrap();

    return Ok(removed);
}
