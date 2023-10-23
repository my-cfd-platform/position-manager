use cfd_engine_sb_contracts::{
    OrderBidAskSbModel, OrderSbModel, OrderSide, OrderSwap, PendingOrderSbModel,
};
use trading_sdk::mt_engine::{
    get_close_price, get_pending_position_type, update_position_pl, MtBidAsk, MtBidAskCache,
    MtPosition, MtPositionActiveState, MtPositionActiveStateOpenData, MtPositionBaseData,
    MtPositionPendingState, MtPositionSide, MtPositionSwaps, MtPositionSwap,
};

use crate::{
    position_manager_grpc::PositionManagerPositionSide,
    position_manager_persistence::{
        PositionManagerPersistenceActivePositionGrpcModel, PositionManagerPersistenceBidAsk,
        PositionManagerPersistencePendingPositionGrpcModel,
    },
};

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
        sl_in_instrument_price: src.base_data.sl_price,
        sl_in_currency: src.base_data.sl_profit,
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
        swaps: src
            .state
            .swaps
            .swaps
            .iter()
            .map(|x| OrderSwap {
                amount: x.amount,
                date: x.date.unix_microseconds as u64,
            })
            .collect(),
    }
}

fn map_bid_ask_option(bid_ask: Option<PositionManagerPersistenceBidAsk>) -> Option<MtBidAsk> {
    if let Some(bid_ask) = bid_ask {
        let bid_ask = MtBidAsk {
            asset_pair: bid_ask.asset_pair,
            bid: bid_ask.bid,
            ask: bid_ask.ask,
            base: bid_ask.base,
            quote: bid_ask.quote,
            date: bid_ask.date_time_unix_timestamp_milis.into(),
        };

        return Some(bid_ask);
    };

    return None;
}

fn map_side(
    src: &crate::position_manager_persistence::PositionManagerPersistencePositionSide,
) -> MtPositionSide {
    match src {
        crate::position_manager_persistence::PositionManagerPersistencePositionSide::Buy => {
            MtPositionSide::Buy
        }
        crate::position_manager_persistence::PositionManagerPersistencePositionSide::Sell => {
            MtPositionSide::Sell
        }
    }
}

pub async fn map_pending_persistence(
    src: PositionManagerPersistencePendingPositionGrpcModel,
    prices_cache: &MtBidAskCache,
) -> MtPosition<MtPositionPendingState> {
    let side = map_side(&src.side());

    let base_data = MtPositionBaseData {
        id: src.id,
        trader_id: src.trader_id,
        account_id: src.account_id,
        asset_pair: src.asset_pair,
        side: side.clone(),
        invest_amount: src.invest_amount,
        leverage: src.leverage,
        stop_out_percent: src.stop_out_percent,
        create_process_id: src.create_process_id,
        crate_date: src.create_date_unix_timestamp_milis.into(),
        last_update_process_id: src.last_update_process_id,
        last_update_date: src.last_update_date.into(),
        collateral: src.collateral,
        base: src.base,
        quote: src.quote,
        tp_profit: src.tp_in_profit,
        tp_price: src.tp_in_asset_price,
        sl_profit: src.sl_in_profit,
        sl_price: src.sl_in_asset_price,
    };

    let current_price = prices_cache.get_by_id(&base_data.asset_pair).unwrap();
    let close_price = get_close_price(current_price.as_ref(), &side);

    let state = MtPositionPendingState {
        desire_price: src.desire_price,
        position_type: get_pending_position_type(close_price, src.desire_price, &side),
    };

    let position = MtPosition { state, base_data };

    return position;
}

pub async fn map_active_persistence(
    src: PositionManagerPersistenceActivePositionGrpcModel,
    prices_cache: &MtBidAskCache,
) -> MtPosition<MtPositionActiveState> {

    let mut total_swaps = 0.0;
    let swaps = src.swaps.iter().map(|x| {
        total_swaps += x.amount;
        MtPositionSwap{
            date: x.date.into(),
            amount: x.amount,
        }
    }).collect();

    let swaps = MtPositionSwaps{
        swaps,
        total: total_swaps,
    };

    let side = map_side(&src.side());
    let open_data = MtPositionActiveStateOpenData {
        asset_open_price: src.asset_open_price,
        asset_open_bid_ask: map_bid_ask_option(src.asset_open_bid_ask).unwrap(),
        base_collateral_open_price: src.collateral_base_open_price,
        base_collateral_open_bid_ask: map_bid_ask_option(src.collateral_base_open_bid_ask),
        open_process_id: src.open_process_id,
        open_date: src.open_date_unix_timestamp_milis.into(),
        pending_state: None,
    };

    let base_data = MtPositionBaseData {
        id: src.id,
        trader_id: src.trader_id,
        account_id: src.account_id,
        asset_pair: src.asset_pair,
        side: side.clone(),
        invest_amount: src.invest_amount,
        leverage: src.leverage,
        stop_out_percent: src.stop_out_percent,
        create_process_id: src.create_process_id,
        crate_date: src.create_date_unix_timestamp_milis.into(),
        last_update_process_id: src.last_update_process_id,
        last_update_date: src.last_update_date.into(),
        collateral: src.collateral,
        base: src.base,
        quote: src.quote,
        tp_profit: src.tp_in_profit,
        tp_price: src.tp_in_asset_price,
        sl_profit: src.sl_in_profit,
        sl_price: src.sl_in_asset_price,
    };

    let (asset, quote_collateral) = get_active_prices(
        prices_cache,
        &base_data.base,
        &base_data.quote,
        &base_data.collateral,
    )
    .await;

    let (bid_ask, price) = get_quote_collateral(&base_data, quote_collateral, &side);

    let state = MtPositionActiveState {
        open_data,
        asset_active_price: get_close_price(&asset, &side),
        asset_active_bid_ask: asset,
        quote_collateral_active_price: price,
        quote_collateral_active_bid_ask: bid_ask,
        profit: 0.0,
        swaps,
    };

    let mut position = MtPosition {
        state,
        base_data,
    };

    update_position_pl(&mut position);

    return position;
}

fn get_quote_collateral(
    base_data: &MtPositionBaseData,
    quote_collateral: Option<MtBidAsk>,
    side: &MtPositionSide,
) -> (Option<MtBidAsk>, f64) {
    if base_data.quote == base_data.collateral {
        return (None, 1.0);
    }

    if let Some(quote_collateral) = quote_collateral {
        return (
            Some(quote_collateral.clone()),
            get_close_price(&quote_collateral, &side),
        );
    }

    panic!("No quote collateral");
}

pub async fn get_active_prices(
    cache: &MtBidAskCache,
    base: &str,
    quote: &str,
    collateral: &str,
) -> (MtBidAsk, Option<MtBidAsk>) {
    let asset_bid_ask = cache.get_base_quote(base, quote).unwrap();

    let quote_collateral = cache.get_base_quote(quote, collateral);
    let collateral_quote = cache.get_base_quote(collateral, quote);

    let quote_collateral = match (quote_collateral, collateral_quote) {
        (None, None) => None,
        (Some(src), None) => Some(src.as_ref().clone()),
        (None, Some(src)) => Some(src.as_ref().clone()),
        (Some(src), Some(_)) => Some(src.as_ref().clone()),
    };

    return (asset_bid_ask.as_ref().clone(), quote_collateral);
}
