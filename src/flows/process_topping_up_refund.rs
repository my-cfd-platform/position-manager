use std::sync::Arc;

use cfd_engine_sb_contracts::{PositionPersistenceEvent, PositionToppingUpEvent};
use service_sdk::{
    my_telemetry::MyTelemetryContext, rust_extensions::date_time::DateTimeAsMicroseconds,
};
use trading_sdk::mt_engine::return_topping_up;

use crate::{map_active_to_sb_model, AppContext};

pub async fn process_topping_up_refund(
    app: Arc<AppContext>,
    id: &str,
    trader_id: &str,
    account_id: &str,
    process_id: &str,
    topping_up_amount: f64,
    my_telemetry: &MyTelemetryContext,
) {
    let updated_position = {
        let mut active_cache = app.active_positions_cache.write().await;
        active_cache.0.update_position(&id, |x| {
            if let Some(src) = x {
                src.base_data.last_update_date = DateTimeAsMicroseconds::now();
                src.base_data.last_update_process_id = process_id.to_string();
                return_topping_up(topping_up_amount, src);
                return Some(src.clone());
            }

            return None;
        })
    };

    trade_log::trade_log!(
        &trader_id.to_string(),
        &account_id.to_string(),
        &process_id.to_string(),
        &id.to_string(),
        "process topping up refund",
        my_telemetry.clone(),
        "update_result" = &updated_position
    );

    if let Some(updated_position) = updated_position {
        let sb_model = PositionPersistenceEvent {
            process_id: process_id.to_string(),
            update_position: Some(map_active_to_sb_model(updated_position)),
            close_position: None,
            create_position: None,
        };

        app.active_positions_persistence_publisher
            .publish(&sb_model, Some(my_telemetry))
            .await
            .unwrap();

        app.topping_up_publisher
            .publish(
                &PositionToppingUpEvent {
                    process_id: process_id.to_string(),
                    position_id: id.to_string(),
                    trader_id: trader_id.to_string(),
                    account_id: account_id.to_string(),
                    delta: -topping_up_amount,
                },
                Some(my_telemetry),
            )
            .await
            .unwrap();
    }
}
