use std::sync::Arc;

use cfd_engine_sb_contracts::{
    PendingOrderNeedApproveEvent, PendingPositionPersistenceEvent, PositionPersistenceEvent,
};
use trading_sdk::mt_engine::{
    execute_pending_position, MtEngineError, MtPosition, MtPositionActiveState,
    MtPositionPendingState,
};
use uuid::Uuid;

use crate::AppContext;

pub async fn handle_pending_rdy_to_execute(
    app: &Arc<AppContext>,
    positions: Vec<MtPosition<MtPositionPendingState>>,
    process_id: &str,
) {
    let mut write = app.pending_execute_to_confirm_positions.write().await;

    let mut messages = vec![];

    for pos in positions {
        messages.push(PendingOrderNeedApproveEvent {
            process_id: process_id.to_string(),
            order: Some(crate::map_pending_to_sb_model(pos.clone())),
        });
        trade_log::trade_log!(
            &pos.base_data.trader_id,
            &pos.base_data.account_id,
            process_id,
            &pos.base_data.id,
            "Detected pending order ready to execute",
            service_sdk::my_telemetry::MyTelemetryContext::new(),
            "position" = &pos
        );
        write.0.add_position(pos);
    }

    app.pending_need_confirm_publisher
        .publish_messages(messages.iter().map(|x| (x, None)))
        .await
        .unwrap();
}

pub async fn confirm_pending_execution(
    app: &Arc<AppContext>,
    position_id: &str,
) -> Result<MtPosition<MtPositionActiveState>, MtEngineError> {
    let process_id = Uuid::new_v4().to_string();

    let target_position = app
        .pending_execute_to_confirm_positions
        .write()
        .await
        .0
        .remove_position(position_id)
        .ok_or(MtEngineError::PositionNotFound)?;

    let active_position = {
        let prices_cache = app.active_prices_cache.read().await;
        execute_pending_position(
            target_position.clone(),
            &prices_cache,
            process_id.to_string(),
        )
    }?;

    {
        let mut active_positions_cache = app.active_positions_cache.write().await;
        active_positions_cache
            .0
            .add_position(active_position.clone());
    }

    let pending_sb_model = crate::map_pending_to_sb_model(target_position);
    let active_sb_model = crate::map_active_to_sb_model(active_position.clone());

    app.pending_positions_persistence_publisher
        .publish(
            &PendingPositionPersistenceEvent {
                process_id: process_id.to_string(),
                cancel: None,
                create: None,
                execute: Some(pending_sb_model),
            },
            None,
        )
        .await
        .unwrap();

    app.active_positions_persistence_publisher
        .publish(
            &PositionPersistenceEvent {
                process_id: process_id.to_string(),
                update_position: None,
                close_position: None,
                create_position: Some(active_sb_model),
            },
            None,
        )
        .await
        .unwrap();

    return Ok(active_position);
}
