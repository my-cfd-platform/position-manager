use cfd_engine_sb_contracts::{OrderBidAskSbModel, OrderSbModel, OrderSide};
use trading_sdk::mt_engine::{
    MtBidAsk, MtEngineError, MtPosition, MtPositionActiveState, MtPositionCloseReason,
    MtPositionClosedState, MtPositionPendingState, MtPositionSide,
};

use crate::{
    position_manager_grpc::{
        PositionManagerActivePositionGrpcModel, PositionManagerBidAsk,
        PositionManagerClosePositionReason, PositionManagerClosedPositionGrpcModel,
        PositionManagerOperationsCodes, PositionManagerPendingPositionGrpcModel,
        PositionManagerPositionSide,
    },
    EngineError,
};

impl Into<PositionManagerPositionSide> for MtPositionSide {
    fn into(self) -> PositionManagerPositionSide {
        match self {
            MtPositionSide::Buy => PositionManagerPositionSide::Buy,
            MtPositionSide::Sell => PositionManagerPositionSide::Sell,
        }
    }
}

impl Into<PositionManagerBidAsk> for MtBidAsk {
    fn into(self) -> PositionManagerBidAsk {
        PositionManagerBidAsk {
            asset_pair: self.asset_pair,
            bid: self.bid,
            ask: self.ask,
            date_time_unix_timestamp_milis: self.date.unix_microseconds as u64,
        }
    }
}

impl Into<PositionManagerActivePositionGrpcModel> for MtPosition<MtPositionActiveState> {
    fn into(self) -> PositionManagerActivePositionGrpcModel {
        let side: PositionManagerPositionSide = self.base_data.side.into();

        PositionManagerActivePositionGrpcModel {
            id: self.base_data.id,
            account_id: self.base_data.account_id,
            trader_id: self.base_data.trader_id,
            asset_pair: self.base_data.asset_pair,
            side: side as i32,
            invest_amount: self.base_data.invest_amount,
            leverage: self.base_data.leverage,
            stop_out_percent: self.base_data.stop_out_percent,
            create_process_id: self.base_data.create_process_id,
            create_date_unix_timestamp_milis: self.base_data.crate_date.unix_microseconds as u64,
            last_update_process_id: self.base_data.last_update_process_id,
            last_update_date: self.base_data.last_update_date.unix_microseconds as u64,
            tp_in_profit: self.base_data.tp_profit,
            sl_in_profit: self.base_data.sl_profit,
            tp_in_asset_price: self.base_data.tp_price,
            sl_in_asset_price: self.base_data.sl_price,
            open_price: self.state.open_data.asset_open_price,
            open_bid_ask: Some(self.state.open_data.asset_open_bid_ask.into()),
            open_process_id: self.state.open_data.open_process_id,
            open_date: self.state.open_data.open_date.unix_microseconds as u64,
            profit: self.state.profit,
            base: self.base_data.base,
            quote: self.base_data.quote,
            collateral: self.base_data.collateral,
            base_collateral_open_price: self.state.open_data.base_collateral_open_price,
        }
    }
}

impl Into<PositionManagerClosePositionReason> for MtPositionCloseReason {
    fn into(self) -> PositionManagerClosePositionReason {
        match self {
            MtPositionCloseReason::ClientCommand => {
                PositionManagerClosePositionReason::ClientCommand
            }
            MtPositionCloseReason::StopOut => PositionManagerClosePositionReason::StopOut,
            MtPositionCloseReason::TakeProfit => PositionManagerClosePositionReason::TakeProfit,
            MtPositionCloseReason::StopLoss => PositionManagerClosePositionReason::StopLoss,
            MtPositionCloseReason::ForceClose => PositionManagerClosePositionReason::ForceClose,
        }
    }
}

impl Into<PositionManagerClosedPositionGrpcModel> for MtPosition<MtPositionClosedState> {
    fn into(self) -> PositionManagerClosedPositionGrpcModel {
        let side: PositionManagerPositionSide = self.base_data.side.into();
        let reason: PositionManagerClosePositionReason = self.state.close_reason.into();

        PositionManagerClosedPositionGrpcModel {
            id: self.base_data.id,
            asset_pair: self.base_data.asset_pair,
            side: side as i32,
            invest_amount: self.base_data.invest_amount,
            leverage: self.base_data.leverage,
            stop_out_percent: self.base_data.stop_out_percent,
            create_process_id: self.base_data.create_process_id,
            create_date_unix_timestamp_milis: self.base_data.crate_date.unix_microseconds as u64,
            last_update_process_id: self.base_data.last_update_process_id,
            last_update_date: self.base_data.last_update_date.unix_microseconds as u64,
            tp_in_profit: self.base_data.tp_profit,
            sl_in_profit: self.base_data.sl_profit,
            tp_in_asset_price: self.base_data.tp_price,
            sl_in_asset_price: self.base_data.sl_price,
            open_price: self.state.active_state.open_data.asset_open_price,
            open_bid_ask: Some(self.state.active_state.open_data.asset_open_bid_ask.into()),
            open_process_id: self.state.active_state.open_data.open_process_id,
            open_date: self
                .state
                .active_state
                .open_data
                .open_date
                .unix_microseconds as u64,
            profit: self.state.active_state.profit,
            close_price: self.state.asset_close_price,
            close_bid_ask: Some(self.state.asset_close_bid_ask.into()),
            close_process_id: self.state.close_process_id,
            close_reason: reason as i32,
        }
    }
}

impl Into<PositionManagerOperationsCodes> for MtEngineError {
    fn into(self) -> PositionManagerOperationsCodes {
        match self {
            MtEngineError::NoLiquidity => PositionManagerOperationsCodes::NoLiquidity,
            MtEngineError::PositionNotFound => PositionManagerOperationsCodes::PositionNotFound,
        }
    }
}

fn map_bid_ask_to_sb(src: MtBidAsk) -> OrderBidAskSbModel {
    OrderBidAskSbModel {
        id: src.asset_pair,
        bid: src.bid,
        ask: src.ask,
        date: src.date.unix_microseconds as u64,
        base: src.base,
        quote: src.quote,
    }
}

pub fn map_closed_tp_sb(src: &MtPosition<MtPositionClosedState>) -> OrderSbModel {
    let side = match src.base_data.side {
        MtPositionSide::Buy => OrderSide::Buy,
        MtPositionSide::Sell => OrderSide::Sell,
    };
    let base_collateral_open_bid_ask = match src
        .state
        .active_state
        .open_data
        .base_collateral_open_bid_ask
        .clone()
    {
        Some(src) => Some(map_bid_ask_to_sb(src)),
        None => None,
    };

    let close_quote_collateral_bid_ask = match src
        .state
        .active_state
        .quote_collateral_active_bid_ask
        .clone()
    {
        Some(src) => Some(map_bid_ask_to_sb(src)),
        None => None,
    };

    OrderSbModel {
        id: src.base_data.id.clone(),
        trader_id: src.base_data.trader_id.clone(),
        account_id: src.base_data.account_id.clone(),
        asset_pair: src.base_data.asset_pair.clone(),
        invest_amount: src.base_data.invest_amount,
        side: side as i32,
        leverage: src.base_data.leverage,
        stop_out_percent: src.base_data.stop_out_percent,
        create_date: src.base_data.crate_date.unix_microseconds as u64,
        tp_in_instrument_price: src.base_data.tp_price,
        tp_in_currency: src.base_data.tp_profit,
        sl_in_instrument_price: src.base_data.sl_price,
        sl_in_currency: src.base_data.sl_profit,
        create_process_id: src.base_data.create_process_id.clone(),
        profit: Some(src.state.active_state.profit),
        metadata: vec![],
        last_update_date: src.base_data.last_update_date.unix_microseconds as u64,
        last_update_process_id: src.base_data.last_update_process_id.clone(),
        asset_open_price: src.state.active_state.open_data.asset_open_price,
        asset_open_bid_ask: Some(map_bid_ask_to_sb(
            src.state.active_state.open_data.asset_open_bid_ask.clone(),
        )),
        open_date: src.state.active_state.open_data.open_date.unix_microseconds as u64,
        open_process_id: src.state.active_state.open_data.open_process_id.clone(),
        close_date: Some(src.state.close_date.unix_microseconds as u64),
        close_reason: Some(src.state.close_reason.clone() as i32),
        asset_close_price: Some(src.state.asset_close_price),
        asset_close_bid_ask: Some(map_bid_ask_to_sb(src.state.asset_close_bid_ask.clone())),
        close_process_id: Some(src.state.close_process_id.clone()),
        base: src.base_data.base.clone(),
        quote: src.base_data.quote.clone(),
        collateral_currency: src.base_data.collateral.clone(),
        base_collateral_open_price: src.state.active_state.open_data.base_collateral_open_price,
        base_collateral_open_bid_ask: base_collateral_open_bid_ask,
        close_quote_collateral_price: src.state.active_state.quote_collateral_active_price,
        close_quote_collateral_bid_ask: close_quote_collateral_bid_ask,
        swaps: vec![],
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

impl Into<PositionManagerPendingPositionGrpcModel> for MtPosition<MtPositionPendingState> {
    fn into(self) -> PositionManagerPendingPositionGrpcModel {
        PositionManagerPendingPositionGrpcModel {
            id: self.base_data.id,
            account_id: self.base_data.account_id,
            trader_id: self.base_data.trader_id,
            asset_pair: self.base_data.asset_pair,
            side: self.base_data.side as i32,
            invest_amount: self.base_data.invest_amount,
            leverage: self.base_data.leverage,
            stop_out_percent: self.base_data.stop_out_percent,
            create_process_id: self.base_data.create_process_id,
            create_date_unix_timestamp_milis: self.base_data.crate_date.unix_microseconds as u64,
            last_update_process_id: self.base_data.last_update_process_id,
            last_update_date: self.base_data.last_update_date.unix_microseconds as u64,
            tp_in_profit: self.base_data.tp_profit,
            sl_in_profit: self.base_data.sl_profit,
            tp_in_asset_price: self.base_data.tp_price,
            sl_in_asset_price: self.base_data.sl_price,
            desire_price: self.state.desire_price,
        }
    }
}
