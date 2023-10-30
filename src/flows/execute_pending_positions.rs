use std::sync::Arc;

use cfd_engine_sb_contracts::{PendingPositionPersistenceEvent, PositionPersistenceEvent};
use service_sdk::my_logger::LogEventCtx;
use trading_sdk::mt_engine::{
    execute_pending_position, MtPosition, MtPositionActiveState, MtPositionPendingState,
};

use crate::AppContext;

pub async fn execute_pending_positions(
    app: &Arc<AppContext>,
    positions: Vec<MtPosition<MtPositionPendingState>>,
    process_id: &str,
) {
    let positions = {
        let prices_cache = app.active_prices_cache.read().await;
        positions
            .into_iter()
            .filter_map(|x| {
                let position =
                    execute_pending_position(x.clone(), &prices_cache, process_id.to_string());

                match position {
                    Ok(position) => Some((position, x.clone())),
                    Err(err) => {
                        let ctx = LogEventCtx::new()
                            .add_object("order", &x)
                            .add_object("err", &err);

                        service_sdk::my_logger::LOGGER.write_error(
                            "execute_pending_positions",
                            "Cant execute pending position",
                            ctx,
                        );
                        None
                    }
                }
            })
            .collect::<Vec<(
                MtPosition<MtPositionActiveState>,
                MtPosition<MtPositionPendingState>,
            )>>()
    };

    let active_persist_events = positions
        .iter()
        .map(|x| {
            let sb_model = crate::map_active_to_sb_model(x.0.clone());

            cfd_engine_sb_contracts::PositionPersistenceEvent {
                process_id: process_id.to_string(),
                update_position: None,
                close_position: None,
                create_position: Some(sb_model),
            }
        })
        .collect::<Vec<PositionPersistenceEvent>>();

    let pending_persist_events = positions
        .iter()
        .map(|x| {
            let sb_model = crate::map_pending_to_sb_model(x.1.clone());

            cfd_engine_sb_contracts::PendingPositionPersistenceEvent {
                process_id: process_id.to_string(),
                cancel: None,
                create: None,
                execute: Some(sb_model),
            }
        })
        .collect::<Vec<PendingPositionPersistenceEvent>>();

    {
        let mut active_positions_cache = app.active_positions_cache.write().await;
        for (active, _) in positions {
            active_positions_cache.0.add_position(active);
        }
    }

    if pending_persist_events.len() > 0 {
        app.pending_positions_persistence_publisher
        .publish_messages(pending_persist_events.iter().map(|x| (x, None)))
        .await
        .unwrap();
    }

    if active_persist_events.len() > 0 {
        app.active_positions_persistence_publisher
        .publish_messages(active_persist_events.iter().map(|x| (x, None)))
        .await
        .unwrap();
    }
}
