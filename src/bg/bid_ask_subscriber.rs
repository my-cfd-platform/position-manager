use std::sync::Arc;

use cfd_engine_sb_contracts::{BidAskSbModel, PositionManagerPositionMarginCallHit};
use service_sdk::{
    my_service_bus::abstractions::subscriber::{
        MessagesReader, MySbSubscriberHandleError, SubscriberCallback,
    },
    my_telemetry::MyTelemetryContext,
};
use stopwatch::Stopwatch;
use trading_sdk::{
    core::EngineCacheQueryBuilder,
    mt_engine::{
        calculate_position_topping_up, can_return_topping_up_funds, get_close_reason,
        is_ready_to_execute_pending_position, update_active_position_rate, update_margin_call_hit,
        update_position_pl, MtPositionCloseReason,
    },
};

use crate::{
    close_position_background, execute_pending_positions, handle_position_margin_call, map_bid_ask,
    process_topping_up_refund, AppContext,
};

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
            service_sdk::metrics::counter!("bid_ask_messages_income", "bid-ask" => operation.id.clone())
                .increment(1);

            let telemetry = message.my_telemetry.engage_telemetry();
            message
                .my_telemetry
                .add_tag("bidask", format!("{operation:?}"));
            let write_telemetry = handle_bid_ask_message(&self.app, operation, &telemetry).await;

            if !write_telemetry {
                message.my_telemetry.ignore_this_event();
            }
        }

        return Ok(());
    }
}

enum UpdatePositionCase {
    Close(PositionsToCloseDto),
    MarginCallHit(PositionManagerPositionMarginCallHit),
    ReturnToppingUp(PositionsReturnToppingUp),
}

struct PositionsReturnToppingUp {
    pub id: String,
    pub trader_id: String,
    pub account_id: String,
    pub topping_up_amount: f64,
}

struct PositionsToCloseDto {
    pub trader_id: String,
    pub account_id: String,
    pub id: String,
    pub close_reason: MtPositionCloseReason,
}

async fn handle_bid_ask_message(
    app: &Arc<AppContext>,
    operation: BidAskSbModel,
    telemetry: &MyTelemetryContext,
) -> bool {
    let mut sw = Stopwatch::start_new();
    let bid_ask = map_bid_ask(operation);

    let mut write_telemetry = false;

    let process_id = format!("bg-bidask-processing.{}", bid_ask.date.unix_microseconds);
    {
        let mut prices = app.active_prices_cache.write().await;
        prices.handle_new(bid_ask.clone());
    }
    {
        let mut update_positions_result = vec![];
        let mut write = app.active_positions_cache.write().await;

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_quote(&bid_ask.quote);

        let update_result = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);

            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                let close_dto = PositionsToCloseDto {
                    trader_id: position.base_data.trader_id.clone(),
                    account_id: position.base_data.account_id.clone(),
                    id: position.base_data.id.clone(),
                    close_reason: cr,
                };

                return Some(UpdatePositionCase::Close(close_dto));
            };

            if let Some(margin_call_percent) = position.base_data.margin_call_percent.clone() {
                if update_margin_call_hit(position) {
                    return Some(UpdatePositionCase::MarginCallHit(
                        PositionManagerPositionMarginCallHit {
                            position_id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            margin_call_percent,
                        },
                    ));
                };
            };

            if let Some(topping_up_amount) = calculate_position_topping_up(&position.base_data) {
                if can_return_topping_up_funds(position) {
                    return Some(UpdatePositionCase::ReturnToppingUp(
                        PositionsReturnToppingUp {
                            id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            topping_up_amount,
                        },
                    ));
                }
            }
            return None;
        });

        update_positions_result.extend(update_result);

        let mut query = EngineCacheQueryBuilder::new();
        query.with_base(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        let update_result = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);

            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                let close_dto = PositionsToCloseDto {
                    trader_id: position.base_data.trader_id.clone(),
                    account_id: position.base_data.account_id.clone(),
                    id: position.base_data.id.clone(),
                    close_reason: cr,
                };

                return Some(UpdatePositionCase::Close(close_dto));
            };

            if let Some(margin_call_percent) = position.base_data.margin_call_percent.clone() {
                if update_margin_call_hit(position) {
                    return Some(UpdatePositionCase::MarginCallHit(
                        PositionManagerPositionMarginCallHit {
                            position_id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            margin_call_percent,
                        },
                    ));
                };
            };

            if let Some(topping_up_amount) = calculate_position_topping_up(&position.base_data) {
                if can_return_topping_up_funds(position) {
                    return Some(UpdatePositionCase::ReturnToppingUp(
                        PositionsReturnToppingUp {
                            id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            topping_up_amount,
                        },
                    ));
                }
            }
            return None;
        });

        update_positions_result.extend(update_result);

        let mut query = EngineCacheQueryBuilder::new();
        query.with_quote(&bid_ask.base);
        query.with_collateral(&bid_ask.quote);

        let update_result = write.0.update_positions(query, |position| {
            update_active_position_rate(position, &bid_ask);
            update_position_pl(position);

            let close_reason = get_close_reason(&position);
            if let Some(cr) = close_reason {
                let close_dto = PositionsToCloseDto {
                    trader_id: position.base_data.trader_id.clone(),
                    account_id: position.base_data.account_id.clone(),
                    id: position.base_data.id.clone(),
                    close_reason: cr,
                };

                return Some(UpdatePositionCase::Close(close_dto));
            };

            if let Some(margin_call_percent) = position.base_data.margin_call_percent.clone() {
                if update_margin_call_hit(position) {
                    return Some(UpdatePositionCase::MarginCallHit(
                        PositionManagerPositionMarginCallHit {
                            position_id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            margin_call_percent,
                        },
                    ));
                };
            };

            if let Some(topping_up_amount) = calculate_position_topping_up(&position.base_data) {
                if can_return_topping_up_funds(position) {
                    return Some(UpdatePositionCase::ReturnToppingUp(
                        PositionsReturnToppingUp {
                            id: position.base_data.id.clone(),
                            trader_id: position.base_data.trader_id.clone(),
                            account_id: position.base_data.account_id.clone(),
                            topping_up_amount,
                        },
                    ));
                }
            }
            return None;
        });
        update_positions_result.extend(update_result);

        if update_positions_result.len() > 0 {
            write_telemetry = true;
        }

        for update in update_positions_result {
            match update {
                UpdatePositionCase::Close(close_position) => {
                    let close_result = close_position_background(
                        app,
                        &close_position.trader_id,
                        &close_position.account_id,
                        &close_position.id,
                        close_position.close_reason.clone(),
                        &process_id,
                        telemetry,
                        &mut write,
                    )
                    .await;

                    trade_log::trade_log!(
                        &close_position.trader_id,
                        &close_position.account_id,
                        &process_id,
                        &close_position.id,
                        "Detected position to close while check bidask",
                        telemetry.clone(),
                        "close_reason" = &close_position.close_reason,
                        "close_result" = &close_result
                    );
                }
                UpdatePositionCase::MarginCallHit(margin_call_hit) => {
                    trade_log::trade_log!(
                        &margin_call_hit.trader_id,
                        &margin_call_hit.account_id,
                        &process_id,
                        &margin_call_hit.position_id,
                        "Detected margin call for position.",
                        telemetry.clone(),
                    );
                    handle_position_margin_call(app.clone(), margin_call_hit).await;
                }
                UpdatePositionCase::ReturnToppingUp(topping_up_return) => {
                    process_topping_up_refund(
                        app.clone(),
                        &topping_up_return.id,
                        &topping_up_return.trader_id,
                        &topping_up_return.account_id,
                        &process_id,
                        topping_up_return.topping_up_amount,
                        &telemetry,
                    )
                    .await;

                    trade_log::trade_log!(
                        &topping_up_return.trader_id,
                        &topping_up_return.account_id,
                        &process_id,
                        &topping_up_return.id,
                        "Detected topping up refund.",
                        telemetry.clone(),
                        "topping_up_amount" = &topping_up_return.topping_up_amount
                    );
                }
            }
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
    sw.stop();
    let duration = sw.elapsed();
    service_sdk::metrics::histogram!("handle_bid_ask_message_milis", "bid-ask" => bid_ask.asset_pair)
        .record(duration.as_millis() as f64);
    write_telemetry
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cfd_engine_sb_contracts::BidAskSbModel;
    use service_sdk::{
        my_service_bus::abstractions::{
            publisher::{MessageToPublish, MyServiceBusPublisher},
            MyServiceBusPublisherClient, PublishError,
        },
        my_telemetry::MyTelemetryContext,
        rust_extensions::AppStates,
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
            margin_call_publisher: MyServiceBusPublisher::new(
                "test".to_string(),
                Arc::new(TestPublisherClient {}),
                false,
                service_sdk::my_logger::LOGGER.clone(),
            ),
            topping_up_publisher: MyServiceBusPublisher::new(
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
