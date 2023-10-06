use cfd_engine_sb_contracts::{OrderBidAskSbModel, OrderSbModel, OrderSide, PendingOrderSbModel};
use trading_sdk::mt_engine::{
    MtBidAsk, MtPosition, MtPositionActiveState, MtPositionPendingState, MtPositionSide,
};

use crate::position_manager_grpc::PositionManagerPositionSide;

impl Into<MtPositionSide> for PositionManagerPositionSide {
    fn into(self) -> MtPositionSide {
        match self {
            PositionManagerPositionSide::Buy => MtPositionSide::Buy,
            PositionManagerPositionSide::Sell => MtPositionSide::Sell,
        }
    }
}

pub fn map_sdk_side_tp_sb(side: MtPositionSide) -> OrderSide {
    match side {
        MtPositionSide::Buy => OrderSide::Buy,
        MtPositionSide::Sell => OrderSide::Sell,
    }
}

pub fn map_pending_to_sb_model(src: MtPosition<MtPositionPendingState>) -> PendingOrderSbModel {
    PendingOrderSbModel {
        id: src.base_data.id,
        trader_id: src.base_data.trader_id,
        account_id: src.base_data.account_id,
        asset_pair: src.base_data.asset_pair,
        invest_amount: src.base_data.invest_amount,
        side: map_sdk_side_tp_sb(src.base_data.side) as i32,
        leverage: src.base_data.leverage,
        stop_out_percent: src.base_data.stop_out_percent,
        create_date: src.base_data.crate_date.unix_microseconds as u64,
        tp_in_instrument_price: src.base_data.tp_price,
        tp_in_currency: src.base_data.tp_profit,
        sl_in_instrument_price: src.base_data.tp_price,
        sl_in_currency: src.base_data.tp_profit,
        create_process_id: src.base_data.create_process_id,
        metadata: vec![],
        last_update_date: src.base_data.last_update_date.unix_microseconds as u64,
        last_update_process_id: src.base_data.last_update_process_id,
        base: src.base_data.base,
        quote: src.base_data.quote,
        collateral_currency: src.base_data.collateral,
        desire_price: src.state.desire_price,
    }
}

pub fn map_bid_ask_to_sb_model(src: MtBidAsk) -> OrderBidAskSbModel {
    OrderBidAskSbModel {
        id: src.asset_pair,
        bid: src.bid,
        ask: src.ask,
        date: src.date.unix_microseconds as u64,
        base: src.base,
        quote: src.quote,
    }
}

pub fn map_active_to_sb_model(src: MtPosition<MtPositionActiveState>) -> OrderSbModel {
    let base_collateral_open_bid_ask = match src.state.open_data.base_collateral_open_bid_ask {
        Some(src) => Some(map_bid_ask_to_sb_model(src)),
        None => None,
    };

    OrderSbModel {
        id: src.base_data.id,
        trader_id: src.base_data.trader_id,
        account_id: src.base_data.account_id,
        asset_pair: src.base_data.asset_pair,
        invest_amount: src.base_data.invest_amount,
        side: map_sdk_side_tp_sb(src.base_data.side) as i32,
        leverage: src.base_data.leverage,
        stop_out_percent: src.base_data.stop_out_percent,
        create_date: src.base_data.crate_date.unix_microseconds as u64,
        tp_in_instrument_price: src.base_data.tp_price,
        tp_in_currency: src.base_data.tp_profit,
        sl_in_instrument_price: src.base_data.tp_price,
        sl_in_currency: src.base_data.tp_profit,
        create_process_id: src.base_data.create_process_id,
        metadata: vec![],
        last_update_date: src.base_data.last_update_date.unix_microseconds as u64,
        last_update_process_id: src.base_data.last_update_process_id,
        base: src.base_data.base,
        quote: src.base_data.quote,
        collateral_currency: src.base_data.collateral,
        profit: Some(src.state.profit),
        asset_open_price: src.state.open_data.asset_open_price,
        asset_open_bid_ask: Some(map_bid_ask_to_sb_model(
            src.state.open_data.asset_open_bid_ask,
        )),
        open_date: src.state.open_data.open_date.unix_microseconds as u64,
        open_process_id: src.state.open_data.open_process_id,
        close_date: None,
        close_reason: None,
        asset_close_price: None,
        asset_close_bid_ask: None,
        close_process_id: None,
        base_collateral_open_price: src.state.open_data.base_collateral_open_price,
        base_collateral_open_bid_ask: base_collateral_open_bid_ask,
        close_quote_collateral_price: src.state.quote_collateral_active_price,
        close_quote_collateral_bid_ask: None,
        swaps: vec![],
    }
}
