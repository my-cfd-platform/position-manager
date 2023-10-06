use std::sync::Arc;

use cfd_engine_sb_contracts::BidAskSbModel;
use service_sdk::my_service_bus::abstractions::subscriber::{
    MessagesReader, MySbSubscriberHandleError, SubscriberCallback,
};
use trading_sdk::{
    core::EngineCacheQueryBuilder,
    mt_engine::{
        get_close_reason, is_ready_to_execute_pending_position, update_active_position_rate,
        update_position_pl,
    },
};

use crate::{execute_pending_positions, map_bid_ask, AppContext};

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
            let bid_ask = map_bid_ask(operation);
            let mut prices = self.app.active_prices_cache.write().await;

            prices.handle_new(bid_ask.clone());

            {
                let mut write = self.app.active_positions_cache.write().await;

                let mut query = EngineCacheQueryBuilder::new();
                query.with_base(&bid_ask.base);
                query.with_quote(&bid_ask.quote);

                write.0.update_positions(query, |position| {
                    update_active_position_rate(position, &bid_ask);
                    update_position_pl(position);
                    let close_reason = get_close_reason(&position);
                    if let Some(cr) = close_reason {
                        return Some((position.base_data.id.clone(), cr));
                    };

                    return None;
                });

                let mut query = EngineCacheQueryBuilder::new();
                query.with_base(&bid_ask.base);
                query.with_collateral(&bid_ask.quote);

                write.0.update_positions(query, |position| {
                    update_active_position_rate(position, &bid_ask);
                    update_position_pl(position);
                    let close_reason = get_close_reason(&position);
                    if let Some(cr) = close_reason {
                        return Some((position.base_data.id.clone(), cr));
                    };

                    return None;
                });

                let mut query = EngineCacheQueryBuilder::new();
                query.with_quote(&bid_ask.base);
                query.with_collateral(&bid_ask.quote);

                write.0.update_positions(query, |position| {
                    update_active_position_rate(position, &bid_ask);
                    update_position_pl(position);
                    let close_reason = get_close_reason(&position);
                    if let Some(cr) = close_reason {
                        return Some((position.base_data.id.clone(), cr));
                    };

                    return None;
                });
            }

            let mut positions_cache = self.app.pending_positions_cache.write().await;

            let mut query = EngineCacheQueryBuilder::new();
            query.with_base(&bid_ask.base);
            query.with_quote(&bid_ask.quote);

            let positions_to_execute = positions_cache.0.query_and_select_remove(query, |x| {
                return is_ready_to_execute_pending_position(x, &bid_ask);
            });

            let process_id = format!("bg-bidask-processing.{}", bid_ask.date.unix_microseconds);
            execute_pending_positions(&self.app, positions_to_execute, &process_id).await;
        }

        return Ok(());
    }
}

async fn handle_bid_ask_message(app: &Arc<AppContext>, operation: BidAskSbModel) {
    let bid_ask = map_bid_ask(operation);
    {
        let mut prices = app.active_prices_cache.write().await;
        prices.handle_new(bid_ask.clone());
    }

    {
        let mut write = app.active_positions_cache.write().await;

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_quote(&bid_ask.quote);

        write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((position.base_data.id.clone(), cr));
            };

            return None;
        });

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((position.base_data.id.clone(), cr));
            };

            return None;
        });

        let mut query = EngineCacheQueryBuilder::new();
        query.with_quote(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);
            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                return Some((position.base_data.id.clone(), cr));
            };

            return None;
        });
    }

    let mut positions_cache = app.pending_positions_cache.write().await;

    let mut query = EngineCacheQueryBuilder::new();
    query.with_base(&bid_ask.base);
    query.with_quote(&bid_ask.quote);

    let positions_to_execute = positions_cache.0.query_and_select_remove(query, |x| {
        return is_ready_to_execute_pending_position(x, &bid_ask);
    });

    let process_id = format!("bg-bidask-processing.{}", bid_ask.date.unix_microseconds);
    execute_pending_positions(&app, positions_to_execute, &process_id).await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cfd_engine_sb_contracts::BidAskSbModel;
    use rust_extensions::AppStates;
    use service_sdk::my_service_bus::abstractions::{
        publisher::{MessageToPublish, MyServiceBusPublisher},
        MyServiceBusPublisherClient, PublishError,
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
        )
        .await;

        println!("Done");
    }
}
