use std::sync::Arc;

use cfd_engine_sb_contracts::PositionManagerPositionMarginCallHit;

use crate::AppContext;

pub async fn handle_position_margin_call(
    app: Arc<AppContext>,
    event: PositionManagerPositionMarginCallHit,
) {
    app.margin_call_publisher
        .publish(&event, None)
        .await
        .unwrap();
}
