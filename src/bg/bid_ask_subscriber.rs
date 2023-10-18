use std::sync::Arc;

use cfd_engine_sb_contracts::BidAskSbModel;
use service_sdk::{
    my_service_bus::abstractions::subscriber::{
        MessagesReader, MySbSubscriberHandleError, SubscriberCallback,
    },
    my_telemetry::MyTelemetryContext,
};
use trading_sdk::{
    core::EngineCacheQueryBuilder,
    mt_engine::{
        get_close_reason, is_ready_to_execute_pending_position, update_active_position_rate,
        update_position_pl,
    },
};

use crate::{close_position_background, execute_pending_positions, map_bid_ask, AppContext};

pub struct PricesListener {
    pub app: Arc<AppContext>,
}

impl PricesListener {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl SubscriberCallback<BidAskSbModel> for PricesListener {
    async fn handle_messages(
        &self,
        messages_reader: &mut MessagesReader<BidAskSbModel>,
    ) -> Result<(), MySbSubscriberHandleError> {
        while let Some(message) = messages_reader.get_next_message() {
            let operation = message.take_message();

            let telemetry = message.my_telemetry.engage_telemetry();
            message.my_telemetry.add_tag("bidask", format!("{operation:?}"));
            let write_telemetry = handle_bid_ask_message(&self.app, operation, &telemetry).await;

            if !write_telemetry {
                message.my_telemetry.ignore_this_event();
            }
        }

        return Ok(());
    }
}

async fn handle_bid_ask_message(
    app: &Arc<AppContext>,
    operation: BidAskSbModel,
    telemetry: &MyTelemetryContext,
) -> bool {
    let bid_ask = map_bid_ask(operation);

    let mut write_telemetry = false;
    let process_id = format!("bg-bidask-processing.{}", bid_ask.date.unix_microseconds);
    {
        let mut prices = app.active_prices_cache.write().await;
        prices.handle_new(bid_ask.clone());
    }
    {
        let mut positions_to_close = vec![];
        let mut write = app.active_positions_cache.write().await;

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_quote(&bid_ask.quote);

        let close_positions = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((
                    position.base_data.trader_id.clone(),
                    position.base_data.account_id.clone(),
                    position.base_data.id.clone(),
                    cr,
                ));
            };

            return None;
        });

        positions_to_close.extend(close_positions);

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        let close_positions = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((
                    position.base_data.trader_id.clone(),
                    position.base_data.account_id.clone(),
                    position.base_data.id.clone(),
                    cr,
                ));
            };

            return None;
        });

        positions_to_close.extend(close_positions);

        let mut query = EngineCacheQueryBuilder::new();
        query.with_quote(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        let close_positions = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((
                    position.base_data.trader_id.clone(),
                    position.base_data.account_id.clone(),
                    position.base_data.id.clone(),
                    cr,
                ));
            };

            return None;
        });
        positions_to_close.extend(close_positions);

        if positions_to_close.len() > 0 {
            write_telemetry = true;
        }

        for (trader_id, account_id, id, reason) in positions_to_close {
            let close_result = close_position_background(
                app,
                &trader_id,
                &account_id,
                &id,
                reason.clone(),
                &process_id,
                telemetry,
                &mut write,
            )
            .await;

            trade_log::trade_log!(
                &trader_id,
                &account_id,
                &process_id,
                &id,
                "Detected position to close while check bidask",
                telemetry.clone(),
                "close_reason" = &reason,
                "close_result" = &close_result
            );
        }
    }

    let mut positions_cache = app.pending_positions_cache.write().await;

    let mut query = EngineCacheQueryBuilder::new();
    query.with_base(&bid_ask.base);
    query.with_quote(&bid_ask.quote);

    let positions_to_execute = positions_cache.0.query_and_select_remove(query, |x| {
        return is_ready_to_execute_pending_position(x, &bid_ask);
    });

    if positions_to_execute.len() > 0 {
        write_telemetry = true;
    }

    for pending in &positions_to_execute {
        trade_log::trade_log!(
            &pending.base_data.trader_id,
            &pending.base_data.account_id,
            &process_id,
            &pending.base_data.id,
            "Detected position to close while check bidask",
            telemetry.clone(),
            "position" = pending,
            "bidask" = &bid_ask
        );
    }

    execute_pending_positions(&app, positions_to_execute, &process_id).await;

    write_telemetry
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cfd_engine_sb_contracts::BidAskSbModel;
    use rust_extensions::AppStates;
    use service_sdk::{
        my_service_bus::abstractions::{
            publisher::{MessageToPublish, MyServiceBusPublisher},
            MyServiceBusPublisherClient, PublishError,
        },
        my_telemetry::MyTelemetryContext,
    };
    use tokio::sync::RwLock;
    use trading_sdk::mt_engine::{ActivePositionsCache, MtBidAskCache, PendingPositionsCache};

    use crate::AppContext;

    use super::handle_bid_ask_message;

    pub struct TestPublisherClient {}

    #[async_trait::async_trait]
    impl MyServiceBusPublisherClient for TestPublisherClient {
        async fn publish_message(
            &self,
            topic_id: &str,
            message: MessageToPublish,
            do_retry: bool,
        ) -> Result<(), PublishError> {
            todo!();
        }

        async fn publish_messages(
            &self,
            topic_id: &str,
            message: &[MessageToPublish],
            do_retry: bool,
        ) -> Result<(), PublishError> {
            todo!();
        }
    }

    #[tokio::test]
    async fn test_handle_bid_ask() {
        let app = Arc::new(AppContext {
            active_positions_cache: Arc::new(RwLock::new(ActivePositionsCache::new())),
            pending_positions_cache: Arc::new(RwLock::new(PendingPositionsCache::new())),
            active_prices_cache: Arc::new(RwLock::new(MtBidAskCache::new())),
            app_states: Arc::new(AppStates::create_initialized()),
            active_positions_persistence_publisher: MyServiceBusPublisher::new(
                "test".to_string(),
                Arc::new(TestPublisherClient {}),
                false,
                service_sdk::my_logger::LOGGER.clone(),
            ),
            pending_positions_persistence_publisher: MyServiceBusPublisher::new(
                "test".to_string(),
                Arc::new(TestPublisherClient {}),
                false,
                service_sdk::my_logger::LOGGER.clone(),
            ),
        });

        handle_bid_ask_message(
            &app,
            BidAskSbModel {
                id: "id".to_string(),
                date_time_unix_milis: 0,
                bid: 0.0,
                ask: 0.0,
                base: "base".to_string(),
                quote: "quote".to_string(),
            },
            &MyTelemetryContext::new(),
        )
        .await;

        println!("Done");
    }
}
