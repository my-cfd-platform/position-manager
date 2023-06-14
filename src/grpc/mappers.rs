use cfd_engine_sb_contracts::{OrderBidAskSbModel, OrderSbModel};

use crate::{
    position_manager_grpc::{
        PositionManagerActivePositionGrpcModel, PositionManagerBidAsk,
        PositionManagerClosePositionReason, PositionManagerOperationsCodes,
    },
    position_manager_persistence::PositionManagerPersistenceBidAsk,
    ActivePositionState, ClosedPositionStates, EngineBidAsk, EngineError, EnginePosition,
    EnginePositionState, ExecutionClosePositionReason, PendingPositionState,
};

impl Into<EngineBidAsk> for PositionManagerPersistenceBidAsk {
    fn into(self) -> EngineBidAsk {
        EngineBidAsk {
            asset_pair: self.asset_pair,
            bid: self.bid,
            ask: self.ask,
            datetime: self.date_time_unix_timestamp_milis.into(),
            base: self.base,
            quote: self.quote,
        }
    }
}

impl Into<PositionManagerBidAsk> for EngineBidAsk {
    fn into(self) -> PositionManagerBidAsk {
        PositionManagerBidAsk {
            asset_pair: self.asset_pair.clone(),
            bid: self.bid,
            ask: self.ask,
            date_time_unix_timestamp_milis: self.datetime.unix_microseconds as u64,
        }
    }
}

impl Into<OrderBidAskSbModel> for EngineBidAsk {
    fn into(self) -> OrderBidAskSbModel {
        OrderBidAskSbModel {
            id: self.asset_pair,
            bid: self.bid,
            ask: self.ask,
            date: self.datetime.unix_microseconds as u64,
            base: self.base,
            quote: self.quote,
        }
    }
}

impl Into<OrderSbModel> for EnginePosition<EngineBidAsk> {
    fn into(self) -> OrderSbModel {
        return match self.state {
            EnginePositionState::Pending(PendingPositionState { .. }) => {
                panic!("Cant convert pending to order sb")
            }

            EnginePositionState::Active(ActivePositionState {
                asset_open_price,
                open_date,
                open_process_id,
                asset_open_bid_ask,
                collateral_base_open_bid_ask,
                collateral_base_open_price,
                collateral_quote_last_bid_ask,
                collateral_quote_last_price,
                ..
            }) => {
                let base_collateral_open_bid_ask: Option<OrderBidAskSbModel> =
                    match collateral_base_open_bid_ask {
                        Some(src) => Some(src.into()),
                        None => None,
                    };

                let collateral_quote_last_bid_ask: Option<OrderBidAskSbModel> =
                    match collateral_quote_last_bid_ask {
                        Some(src) => Some(src.into()),
                        None => None,
                    };

                OrderSbModel {
                    id: self.data.id,
                    trader_id: self.data.trader_id,
                    account_id: self.data.account_id,
                    asset_pair: self.data.asset_pair,
                    invest_amount: self.data.invest_amount,
                    side: self.data.side as i32,
                    leverage: self.data.leverage,
                    stop_out_percent: self.data.stop_out_percent,
                    create_date: self.data.create_date.unix_microseconds as u64,
                    tp_in_instrument_price: self.data.take_profit_in_position_profit,
                    tp_in_currency: self.data.take_profit_in_asset_price,
                    sl_in_instrument_price: self.data.stop_loss_in_position_profit,
                    sl_in_currency: self.data.stop_loss_in_asset_price,
                    create_process_id: self.data.create_process_id,
                    profit: None,
                    metadata: vec![],
                    last_update_date: self.data.last_update_date.unix_microseconds as u64,
                    last_update_process_id: self.data.last_update_process_id,
                    asset_open_price,
                    asset_open_bid_ask: Some(asset_open_bid_ask.into()),
                    open_date: open_date.unix_microseconds as u64,
                    open_process_id: open_process_id,
                    close_date: None,
                    close_reason: None,
                    asset_close_price: None,
                    asset_close_bid_ask: None,
                    close_process_id: None,
                    base: self.data.base,
                    quote: self.data.quote,
                    collateral_currency: self.data.collateral_currency,
                    base_collateral_open_price: collateral_base_open_price,
                    base_collateral_open_bid_ask: base_collateral_open_bid_ask,
                    close_quote_collateral_price: collateral_quote_last_price,
                    close_quote_collateral_bid_ask: collateral_quote_last_bid_ask,
                }
            }
            EnginePositionState::Closed(ClosedPositionStates {
                active_state,
                close_process_id,
                asset_close_price,
                asset_close_bid_ask,
                close_date,
                close_reason,
                ..
            }) => {
                let base_collateral_open_bid_ask: Option<OrderBidAskSbModel> =
                    match active_state.collateral_base_open_bid_ask {
                        Some(src) => Some(src.into()),
                        None => None,
                    };

                let collateral_quote_last_bid_ask: Option<OrderBidAskSbModel> =
                    match active_state.collateral_quote_last_bid_ask {
                        Some(src) => Some(src.into()),
                        None => None,
                    };

                OrderSbModel {
                    id: self.data.id,
                    trader_id: self.data.trader_id,
                    account_id: self.data.account_id,
                    asset_pair: self.data.asset_pair,
                    invest_amount: self.data.invest_amount,
                    side: self.data.side as i32,
                    leverage: self.data.leverage,
                    stop_out_percent: self.data.stop_out_percent,
                    create_date: self.data.create_date.unix_microseconds as u64,
                    tp_in_instrument_price: self.data.take_profit_in_position_profit,
                    tp_in_currency: self.data.take_profit_in_asset_price,
                    sl_in_instrument_price: self.data.stop_loss_in_position_profit,
                    sl_in_currency: self.data.stop_loss_in_asset_price,
                    create_process_id: self.data.create_process_id,
                    profit: Some(active_state.profit),
                    metadata: vec![],
                    last_update_date: self.data.last_update_date.unix_microseconds as u64,
                    last_update_process_id: self.data.last_update_process_id,
                    asset_open_price: active_state.asset_open_price,
                    asset_open_bid_ask: Some(active_state.asset_open_bid_ask.into()),
                    open_date: active_state.open_date.unix_microseconds as u64,
                    open_process_id: active_state.open_process_id,
                    close_date: Some(close_date.unix_microseconds as u64),
                    close_reason: Some(close_reason as i32),
                    asset_close_price: Some(asset_close_price),
                    asset_close_bid_ask: Some(asset_close_bid_ask.into()),
                    close_process_id: Some(close_process_id),
                    base: self.data.base,
                    quote: self.data.quote,
                    collateral_currency: self.data.collateral_currency,
                    base_collateral_open_price: active_state.collateral_base_open_price,
                    base_collateral_open_bid_ask: base_collateral_open_bid_ask,
                    close_quote_collateral_price: active_state.collateral_base_open_price,
                    close_quote_collateral_bid_ask: collateral_quote_last_bid_ask,
                }
            }
        };
    }
}

impl Into<PositionManagerActivePositionGrpcModel> for EnginePosition<EngineBidAsk> {
    fn into(self) -> PositionManagerActivePositionGrpcModel {
        let data = self.data;

        let EnginePositionState::Active(active_state) = self.state else{
            panic!("Position is not active");
        };

        PositionManagerActivePositionGrpcModel {
            id: data.id,
            account_id: data.account_id,
            trader_id: data.trader_id,
            asset_pair: data.asset_pair,
            side: data.side as i32,
            invest_amount: data.invest_amount,
            leverage: data.leverage,
            stop_out_percent: data.stop_out_percent,
            create_process_id: data.create_process_id,
            create_date_unix_timestamp_milis: (data.create_date.unix_microseconds / 1000) as u64,
            last_update_process_id: data.last_update_process_id,
            last_update_date: (data.last_update_date.unix_microseconds / 1000) as u64,
            tp_in_profit: data.take_profit_in_position_profit,
            sl_in_profit: data.stop_loss_in_position_profit,
            tp_in_asset_price: data.take_profit_in_asset_price,
            sl_in_asset_price: data.stop_loss_in_asset_price,
            open_price: active_state.asset_open_price,
            open_bid_ask: Some(active_state.asset_open_bid_ask.into()),
            open_process_id: active_state.open_process_id,
            open_date: (active_state.open_date.unix_microseconds / 1000) as u64,
            profit: active_state.profit,
        }
    }
}

impl Into<PositionManagerOperationsCodes> for EngineError {
    fn into(self) -> PositionManagerOperationsCodes {
        match self {
            EngineError::NoLiquidity => PositionManagerOperationsCodes::NoLiquidity,
            EngineError::PositionNotFound => PositionManagerOperationsCodes::PositionNotFound,
        }
    }
}

impl Into<PositionManagerClosePositionReason> for ExecutionClosePositionReason {
    fn into(self) -> PositionManagerClosePositionReason {
        match self {
            ExecutionClosePositionReason::ClientCommand => {
                PositionManagerClosePositionReason::ClientCommand
            }
            ExecutionClosePositionReason::StopOut => PositionManagerClosePositionReason::StopOut,
            ExecutionClosePositionReason::TakeProfit => {
                PositionManagerClosePositionReason::TakeProfit
            }
            ExecutionClosePositionReason::StopLoss => PositionManagerClosePositionReason::StopLoss,
            ExecutionClosePositionReason::ForceClose => {
                PositionManagerClosePositionReason::ForceClose
            }
        }
    }
}
