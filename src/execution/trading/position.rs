use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::caches::{
    ExecutionBidAsk, ExecutionClosePositionReason, ExecutionPendingOrderType, PositionSide,
};

#[derive(Debug, Clone)]
pub enum EnginePositionState<T: ExecutionBidAsk> {
    Pending(PendingPositionState),
    Active(ActivePositionState<T>),
    Closed(ClosedPositionStates<T>),
}
impl<T: ExecutionBidAsk> EnginePositionState<T> {
    pub fn update_profit(&mut self, profit: f64) {
        if let EnginePositionState::Active(active_state) = self {
            active_state.profit = profit;
            return;
        }

        panic!("Can't update profit. Profit can be updated only for active positions");
    }
}
#[derive(Debug, Clone)]
pub struct EnginePosition<T: ExecutionBidAsk> {
    pub data: EnginePositionBase,
    pub state: EnginePositionState<T>,
}

#[derive(Debug, Clone)]
pub struct EnginePositionSwap {
    pub date: DateTimeAsMicroseconds,
    pub amount: f64,
}

#[derive(Debug, Clone)]
pub struct EnginePositionSwaps {
    pub swaps: Vec<EnginePositionSwap>,
    pub total: f64,
}

impl Default for EnginePositionSwaps {
    fn default() -> Self {
        Self {
            swaps: Vec::new(),
            total: 0.0,
        }
    }
}

impl EnginePositionSwaps {
    pub fn add_swap(&mut self, amount: f64) {
        let swap = EnginePositionSwap {
            date: DateTimeAsMicroseconds::now(),
            amount,
        };
        self.total += swap.amount;
        self.swaps.push(swap);
    }
}

#[derive(Debug, Clone)]
pub struct EnginePositionBase {
    pub id: String,
    pub trader_id: String,
    pub account_id: String,
    pub asset_pair: String,
    pub side: PositionSide,
    pub invest_amount: f64,
    pub leverage: f64,
    pub stop_out_percent: f64,
    pub create_process_id: String,
    pub create_date: DateTimeAsMicroseconds,
    pub last_update_process_id: String,
    pub last_update_date: DateTimeAsMicroseconds,
    pub take_profit_in_position_profit: Option<f64>,
    pub take_profit_in_asset_price: Option<f64>,
    pub stop_loss_in_position_profit: Option<f64>,
    pub stop_loss_in_asset_price: Option<f64>,
    pub collateral_currency: String,
    pub base: String,
    pub quote: String,
    pub swaps: EnginePositionSwaps,
}

#[derive(Debug, Clone)]
pub struct PendingPositionState {
    pub desire_price: f64,
    pub pending_order_type: ExecutionPendingOrderType,
}

#[derive(Debug, Clone)]
pub struct ActivePositionState<T: ExecutionBidAsk> {
    pub asset_open_price: f64,
    pub asset_open_bid_ask: T,
    pub collateral_base_open_price: f64,
    pub collateral_base_open_bid_ask: Option<T>,
    pub open_process_id: String,
    pub open_date: DateTimeAsMicroseconds,
    pub asset_last_price: f64,
    pub asset_last_bid_ask: T,
    pub collateral_quote_last_price: f64,
    pub collateral_quote_last_bid_ask: Option<T>,
    pub profit: f64,
    pub pending_state: Option<PendingPositionState>,
}
#[derive(Debug, Clone)]
pub struct ClosedPositionStates<T: ExecutionBidAsk> {
    pub active_state: ActivePositionState<T>,
    pub asset_close_price: f64,
    pub asset_close_bid_ask: T,
    pub close_reason: ExecutionClosePositionReason,
    pub close_date: DateTimeAsMicroseconds,
    pub close_process_id: String,
}
