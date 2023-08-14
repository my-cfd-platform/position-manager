use std::{sync::Arc, time::Duration};

use crate::{
    get_close_price,
    position_manager_persistence::position_manager_persistence_grpc_service_client::PositionManagerPersistenceGrpcServiceClient,
    ActivePositionState, ActivePricesCache, EngineBidAsk, EnginePosition, EnginePositionBase,
    EnginePositionState, EnginePositionSwap, PositionSide,
};
use tokio::sync::RwLock;
use tonic::transport::Channel;

pub struct PositionManagerPersistenceClient {
    channel: Channel,
    timeout: Duration,
    active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
}

impl PositionManagerPersistenceClient {
    pub async fn new(
        grpc_address: String,
        active_prices_cache: Arc<RwLock<ActivePricesCache<EngineBidAsk>>>,
    ) -> Self {
        let channel = Channel::from_shared(grpc_address)
            .unwrap()
            .connect()
            .await
            .unwrap();
        Self {
            channel,
            timeout: Duration::from_secs(1),
            active_prices_cache,
        }
    }

    fn create_grpc_service(&self) -> PositionManagerPersistenceGrpcServiceClient<Channel> {
        let client: PositionManagerPersistenceGrpcServiceClient<Channel> =
            PositionManagerPersistenceGrpcServiceClient::new(self.channel.clone());

        client
    }

    pub async fn get_active_positions(&self) -> Vec<EnginePosition<EngineBidAsk>> {
        let mut client = self.create_grpc_service();

        let response = client.get_active_positions(()).await.unwrap();
        let prices_cache = self.active_prices_cache.read().await;

        return match my_grpc_extensions::read_grpc_stream::as_vec(
            response.into_inner(),
            self.timeout,
        )
        .await
        .unwrap()
        {
            Some(result) => result
                .iter()
                .map(|x| {
                    let data = EnginePositionBase {
                        id: x.id.to_owned(),
                        trader_id: x.trader_id.to_owned(),
                        account_id: x.account_id.to_owned(),
                        asset_pair: x.asset_pair.to_owned(),
                        side: PositionSide::from(x.side),
                        invest_amount: x.invest_amount,
                        leverage: x.leverage,
                        stop_out_percent: x.stop_out_percent,
                        create_process_id: x.create_process_id.to_owned(),
                        create_date: x.create_date_unix_timestamp_milis.into(),
                        last_update_process_id: x.last_update_process_id.to_owned(),
                        last_update_date: x.last_update_date.into(),
                        take_profit_in_position_profit: x.tp_in_profit,
                        take_profit_in_asset_price: x.tp_in_asset_price,
                        stop_loss_in_position_profit: x.sl_in_profit,
                        stop_loss_in_asset_price: x.sl_in_asset_price,
                        collateral_currency: x.collateral.clone(),
                        base: x.base.clone(),
                        quote: x.quote.clone(),
                        swaps: crate::EnginePositionSwaps {
                            swaps: x
                                .swaps
                                .iter()
                                .map(|x| EnginePositionSwap {
                                    date: x.date_time_unix_timestamp_milis.into(),
                                    amount: x.amount,
                                })
                                .collect(),
                            total: 0.0,
                        },
                    };

                    let collateral_base_open_bid_ask: Option<EngineBidAsk> =
                        match x.collateral_base_open_bid_ask.clone() {
                            Some(src) => Some(src.into()),
                            None => None,
                        };

                    let asset_active_bid_ask = prices_cache.get_by_id(&x.asset_pair).unwrap();
                    let (collateral_quote_price, collateral_quote_bid_ask) = {
                        if data.collateral_currency == data.quote {
                            (1.0, None)
                        } else {
                            let target_bid_ask =
                                prices_cache.get_by_currencies(&data.base, &data.quote);

                            if target_bid_ask.is_none() {
                                panic!("Not found price for {}/{}", data.base, data.quote);
                            }

                            let target_bid_ask = target_bid_ask.unwrap();

                            let result = (
                                get_close_price(target_bid_ask.as_ref(), &data.side),
                                Some(target_bid_ask.as_ref().clone()),
                            );

                            result
                        }
                    };

                    let active_state = ActivePositionState {
                        asset_open_price: x.asset_open_price,
                        asset_open_bid_ask: x.asset_open_bid_ask.clone().unwrap().into(),
                        collateral_base_open_price: x.collateral_base_open_price,
                        collateral_base_open_bid_ask,
                        open_process_id: x.open_process_id.clone(),
                        open_date: x.open_date_unix_timestamp_milis.into(),
                        asset_last_price: get_close_price(
                            asset_active_bid_ask.as_ref(),
                            &data.side,
                        ),
                        asset_last_bid_ask: asset_active_bid_ask.as_ref().clone(),
                        collateral_quote_last_price: collateral_quote_price,
                        collateral_quote_last_bid_ask: collateral_quote_bid_ask.clone(),
                        profit: 0.0,
                        pending_state: None,
                    };

                    let mut position: EnginePosition<EngineBidAsk> = EnginePosition {
                        data,
                        state: EnginePositionState::Active(active_state),
                    };

                    position.update_pl();

                    position
                })
                .collect(),
            None => vec![],
        };
    }
}
