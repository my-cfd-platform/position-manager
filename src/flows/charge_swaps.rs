use cfd_engine_sb_contracts::PositionPersistenceEvent;
use rust_extensions::date_time::DateTimeAsMicroseconds;
use service_sdk::my_telemetry::MyTelemetryContext;
use trading_sdk::mt_engine::{MtPosition, MtPositionActiveState};

use crate::{map_active_to_sb_model, AppContext};

pub async fn charge_swaps(
    app: &AppContext,
    process_id: &str,
    id: &str,
    amount: f64,
    telemetry: &MyTelemetryContext,
) -> Option<MtPosition<MtPositionActiveState>> {
    let mut write = app.active_positions_cache.write().await;

    let updated_position = write.0.update_position(id, |pos| {
        if let Some(pos) = pos {
            pos.state.swaps.add_swap(amount);
            pos.base_data.last_update_date = DateTimeAsMicroseconds::now();
            pos.base_data.last_update_process_id = process_id.to_string();
            return Some(pos.clone());
        }

        return None;
    });

    if let Some(updated_position) = updated_position {
        app.active_positions_persistence_publisher
            .publish(
                &PositionPersistenceEvent {
                    process_id: process_id.to_string(),
                    update_position: Some(map_active_to_sb_model(updated_position.clone())),
                    close_position: None,
                    create_position: None,
                },
                Some(telemetry),
            )
            .await
            .unwrap();

        return Some(updated_position);
    }

    return None;
}
