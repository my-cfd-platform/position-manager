use std::{collections::HashSet, sync::Arc};

use cfd_engine_sb_contracts::{BidAskSbModel, PositionPersistenceEvent};
use my_service_bus_abstractions::subscriber::{
    MessagesReader, MySbSubscriberHandleError, SubscriberCallback,
};

use crate::{AppContext, EngineBidAsk, ExecutionBidAsk};

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
            let process_id = format!(
                "bg_bidask_processing_{}_{}",
                operation.id, operation.date_time_unix_milis
            );
            let bid_ask: EngineBidAsk = operation.into();
            let mut prices = self.app.active_prices_cache.write().await;
            prices.update(bid_ask.clone());

            let mut positions_to_close = None;

            {
                let close_result =
                    self.app
                        .active_positions_cache
                        .write()
                        .await
                        .update_rate(&bid_ask, |index| {
                            let mut positions_to_update = HashSet::new();

                            match index.get_instrument_positions(bid_ask.get_asset_pair()) {
                                Some(src) => positions_to_update.extend(src),
                                None => {}
                            };

                            match index.get_base_coll_positions(bid_ask.get_asset_pair()) {
                                Some(src) => positions_to_update.extend(src),
                                None => {}
                            };

                            match index.get_quote_coll_positions(bid_ask.get_asset_pair()) {
                                Some(src) => positions_to_update.extend(src),
                                None => {}
                            };

                            Some(positions_to_update.iter().map(|x| x.as_str()).collect())
                        });

                if close_result.is_none() {
                    continue;
                }

                positions_to_close = close_result;
            }

            {
                let closed_positions = self
                    .app
                    .active_positions_cache
                    .write()
                    .await
                    .close_positions(positions_to_close.unwrap());

                let messages_to_send: Vec<PositionPersistenceEvent> = closed_positions
                    .iter()
                    .map(|(position, reason)| {
                        let closed_position = position.to_owned().close_position(
                            &process_id,
                            reason.to_owned(),
                        );

                        return PositionPersistenceEvent {
                            process_id: process_id.clone(),
                            update_position: None,
                            close_position: Some(closed_position.into()),
                            create_position: None,
                        };
                    })
                    .collect();

                self.app
                    .persistence_queue
                    .publish_messages(&messages_to_send)
                    .await
                    .unwrap();
            }
        }

        return Ok(());
    }
}
