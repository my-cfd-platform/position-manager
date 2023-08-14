use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::{
    caches::{ExecutionBidAsk, ExecutionClosePositionReason, ExecutionPositionBase, PositionSide},
    execution::{dto::EngineBidAsk, trading::position::EnginePositionState},
    utils::get_close_price,
    ActiveExecutionPosition, ClosedPositionStates, PositionsStoreIndexAccessor,
};

use super::position::{ActivePositionState, EnginePosition};

impl<T: ExecutionBidAsk> EnginePosition<T> {
    pub fn get_close_reason(&self) -> Option<ExecutionClosePositionReason> {
        if self.is_so_triggered() {
            return Some(ExecutionClosePositionReason::StopOut);
        }

        if self.is_sl_triggered() {
            return Some(ExecutionClosePositionReason::StopLoss);
        }

        if self.is_tp_triggered() {
            return Some(ExecutionClosePositionReason::StopLoss);
        }

        return None;
    }

    fn is_so_triggered(&self) -> bool {
        let margin = self.calculate_margin();

        100.0 - margin >= self.data.stop_out_percent
    }

    fn is_sl_triggered(&self) -> bool {
        let active_state = self.get_active_state();

        if let Some(sl) = self.data.stop_loss_in_position_profit {
            return active_state.profit <= sl;
        }

        if let Some(sl) = self.data.stop_loss_in_asset_price {
            let side = &self.data.side;
            let close_price = get_close_price(&active_state.asset_last_bid_ask, side);

            return match side {
                PositionSide::Buy => sl >= close_price,
                PositionSide::Sell => sl <= close_price,
            };
        }

        return false;
    }

    fn is_tp_triggered(&self) -> bool {
        let active_state = self.get_active_state();

        if let Some(tp) = self.data.take_profit_in_position_profit {
            return active_state.profit >= tp;
        }

        if let Some(tp) = self.data.take_profit_in_asset_price {
            let side = &self.data.side;
            let close_price = get_close_price(&active_state.asset_last_bid_ask, side);

            return match side {
                PositionSide::Buy => tp <= close_price,
                PositionSide::Sell => tp >= close_price,
            };
        }

        return false;
    }

    fn calculate_margin(&self) -> f64 {
        let active_state = self.get_active_state();
        let margin = active_state.profit + self.data.invest_amount;
        return margin / self.data.invest_amount * 100.0;
    }

    pub fn charge_swap(&mut self, swap: f64) {
        self.data.swaps.add_swap(swap);
    }

    pub fn update_sl(&mut self, sl_in_profit: &Option<f64>, sl_in_asset_price: &Option<f64>) {
        self.data.stop_loss_in_asset_price = sl_in_asset_price.clone();
        self.data.stop_loss_in_position_profit = sl_in_profit.clone();
    }

    pub fn update_tp(&mut self, tp_in_profit: &Option<f64>, tp_in_asset_price: &Option<f64>) {
        self.data.take_profit_in_asset_price = tp_in_asset_price.clone();
        self.data.take_profit_in_position_profit = tp_in_profit.clone();
    }

    pub fn update_pl(&mut self) {
        let active_state = self.get_active_state();
        let volume = self.data.invest_amount * self.data.leverage;

        let collateral_invest = match &active_state.collateral_base_open_bid_ask {
            Some(collateral) => match self.data.base != collateral.get_quote() {
                true => volume * active_state.collateral_base_open_price,
                false => volume / active_state.collateral_base_open_price,
            },
            None => volume,
        };

        let price_change = active_state.asset_last_price - active_state.asset_open_price;

        let pl = match &active_state.collateral_quote_last_bid_ask {
            Some(collateral) => match self.data.quote != collateral.get_quote() {
                true => collateral_invest * price_change * active_state.collateral_quote_last_price,
                false => {
                    collateral_invest * price_change / active_state.collateral_quote_last_price
                }
            },
            None => collateral_invest * price_change,
        };

        let pl = match self.data.side {
            PositionSide::Buy => pl,
            PositionSide::Sell => pl * -1.0,
        };

        let pl = pl + self.data.swaps.total;

        self.state.update_profit(pl);
    }

    pub fn close_position(
        mut self,
        process_id: &str,
        close_reason: ExecutionClosePositionReason,
    ) -> Self {
        let EnginePositionState::Active(active_state) = self.state else{
            panic!("Can't close position. Position is not active");
        };

        let closed_state = ClosedPositionStates {
            active_state: active_state.clone(),
            asset_close_price: active_state.asset_last_price,
            asset_close_bid_ask: active_state.asset_last_bid_ask.clone(),
            close_reason,
            close_date: DateTimeAsMicroseconds::now(),
            close_process_id: process_id.to_string(),
        };

        self.state = EnginePositionState::Closed(closed_state);

        return self;
    }

    pub fn get_active_state(&self) -> &ActivePositionState<T> {
        if let EnginePositionState::Active(active_state) = &self.state {
            return active_state;
        }

        panic!("Can't get active data. State is not active");
    }

    pub fn handle_bid_ask(&mut self, bid_ask: &T) {
        if let EnginePositionState::Active(active_state) = &mut self.state {
            if active_state.asset_last_bid_ask.get_asset_pair() == bid_ask.get_asset_pair() {
                active_state.asset_last_bid_ask = bid_ask.clone();
                active_state.asset_last_price = get_close_price(bid_ask, &self.data.side);
            }

            match &active_state.collateral_quote_last_bid_ask {
                Some(collateral) => {
                    if collateral.get_asset_pair() == bid_ask.get_asset_pair() {
                        active_state.collateral_quote_last_bid_ask = Some(bid_ask.clone());
                        active_state.collateral_quote_last_price =
                            get_close_price(bid_ask, &self.data.side);
                    }
                }
                None => {}
            }
        }
    }
}

impl PositionsStoreIndexAccessor for EnginePosition<EngineBidAsk> {
    fn get_account_index(&self) -> Option<String> {
        Some(self.data.account_id.clone())
    }

    fn get_base_coll_index(&self) -> Option<String> {
        None
    }

    fn get_quote_coll_index(&self) -> Option<String> {
        let active_state = self.get_active_state();
        match &active_state.collateral_quote_last_bid_ask {
            Some(collateral) => Some(collateral.get_asset_pair().to_string()),
            None => None,
        }
    }

    fn get_instrument_index(&self) -> Option<String> {
        Some(self.data.asset_pair.clone())
    }
}

impl ExecutionPositionBase for EnginePosition<EngineBidAsk> {
    fn get_id(&self) -> &str {
        &self.data.id
    }

    fn get_asset_pair(&self) -> &str {
        &self.data.asset_pair
    }

    fn get_side(&self) -> &PositionSide {
        &self.data.side
    }

    fn get_invest_amount(&self) -> f64 {
        self.data.invest_amount
    }

    fn get_so_percent(&self) -> f64 {
        self.data.stop_out_percent
    }

    fn get_account_id(&self) -> &str {
        &self.data.account_id
    }

    fn get_position_close_reason(&self) -> Option<ExecutionClosePositionReason> {
        self.get_close_reason()
    }
}

impl ActiveExecutionPosition for EnginePosition<EngineBidAsk> {
    fn get_profit(&self) -> f64 {
        let state = self.get_active_state();
        state.profit
    }

    fn get_open_price(&self) -> f64 {
        let state = self.get_active_state();
        state.asset_open_price
    }

    fn get_take_profit_in_order_profit(&self) -> Option<f64> {
        self.data.take_profit_in_position_profit
    }

    fn get_take_profit_in_asset_price(&self) -> Option<f64> {
        self.data.take_profit_in_asset_price
    }

    fn get_stop_loss_in_order_profit(&self) -> Option<f64> {
        self.data.stop_loss_in_position_profit
    }

    fn get_stop_loss_in_asset_price(&self) -> Option<f64> {
        self.data.stop_loss_in_asset_price
    }

    fn get_next_charge_settlement_fee_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get_last_charge_settlement_fee_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }

    fn get_charge_settlement_fee_period_in_seconds(&self) -> Option<chrono::Duration> {
        None
    }

    fn get_last_close_price(&self) -> f64 {
        let active_state = self.get_active_state();

        active_state.asset_last_price
    }

    fn handle_bid_ask<T: ExecutionBidAsk>(&mut self, bid_ask: &T) {
        self.handle_bid_ask(&EngineBidAsk::from_abstraction(bid_ask));
        self.update_pl();
    }

    // fn handle_bid_ask<T: EngineBidAsk>(&mut self, bid_ask: &EngineBidAsk) {

    // }
}

// #[cfg(test)]
// mod tests {
//     use rust_extensions::date_time::DateTimeAsMicroseconds;

//     use crate::{
//         caches::PositionSide,
//         execution::{
//             dto::EngineBidAsk,
//             trading::position::{
//                 ActivePositionState, EnginePosition, EnginePositionBase, EnginePositionState,
//             },
//         },
//     };

//     #[test]
//     fn test_pl_calculate() {
//         let mut order = EnginePosition {
//             data: EnginePositionBase {
//                 id: "test".to_string(),
//                 trader_id: "test".to_string(),
//                 account_id: "test".to_string(),
//                 asset_pair: "test".to_string(),
//                 side: PositionSide::Buy,
//                 invest_amount: 100000.0,
//                 leverage: 5.0,
//                 stop_out_percent: 0.0,
//                 create_process_id: "test".to_string(),
//                 create_date: DateTimeAsMicroseconds::now(),
//                 last_update_process_id: "test".to_string(),
//                 last_update_date: DateTimeAsMicroseconds::now(),
//                 take_profit_in_position_profit: None,
//                 take_profit_in_asset_price: None,
//                 stop_loss_in_position_profit: None,
//                 stop_loss_in_asset_price: None,
//                 collateral_currency: "USD".to_string(),
//                 base: "CHF".to_string(),
//                 quote: "JPY".to_string(),
//             },
//             state: EnginePositionState::Active(ActivePositionState {
//                 asset_open_price: 149.00,
//                 asset_open_bid_ask: EngineBidAsk {
//                     asset_pair: "CHFJPY".to_string(),
//                     bid: 149.00,
//                     ask: 149.00,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "CHF".to_string(),
//                     quote: "GPY".to_string(),
//                 },
//                 collateral_base_open_price: 1.08696,
//                 collateral_base_open_bid_ask: Some(EngineBidAsk {
//                     asset_pair: "USDCHF".to_string(),
//                     bid: 0.9200,
//                     ask: 0.9200,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "CHF".to_string(),
//                 }),
//                 open_process_id: "test".to_string(),
//                 open_date: DateTimeAsMicroseconds::now(),
//                 asset_close_price: 149.20,
//                 asset_close_bid_ask: EngineBidAsk {
//                     asset_pair: "CHFJPY".to_string(),
//                     bid: 149.20,
//                     ask: 149.20,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "CHF".to_string(),
//                     quote: "JPY".to_string(),
//                 },
//                 collateral_quote_last_price: 132.1,
//                 collateral_quote_last_bid_ask: Some(EngineBidAsk {
//                     asset_pair: "USDJPY".to_string(),
//                     bid: 132.1,
//                     ask: 132.1,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "JPY".to_string(),
//                 }),
//                 profit: 0.0,
//                 pending_state: None,
//             }),
//         };

//         order.update_pl();

//         let active = order.get_active_state();
//         println!("profit: {}", active.profit);
//     }
//     #[test]
//     fn test_pl_calculate2() {
//         let mut order = EnginePosition {
//             data: EnginePositionBase {
//                 id: "test".to_string(),
//                 trader_id: "test".to_string(),
//                 account_id: "test".to_string(),
//                 asset_pair: "test".to_string(),
//                 side: PositionSide::Sell,
//                 invest_amount: 100000.0,
//                 leverage: 5.0,
//                 stop_out_percent: 0.0,
//                 create_process_id: "test".to_string(),
//                 create_date: DateTimeAsMicroseconds::now(),
//                 last_update_process_id: "test".to_string(),
//                 last_update_date: DateTimeAsMicroseconds::now(),
//                 take_profit_in_position_profit: None,
//                 take_profit_in_asset_price: None,
//                 stop_loss_in_position_profit: None,
//                 stop_loss_in_asset_price: None,
//                 collateral_currency: "USD".to_string(),
//                 base: "CHF".to_string(),
//                 quote: "JPY".to_string(),
//             },
//             state: EnginePositionState::Active(ActivePositionState {
//                 asset_open_price: 149.00,
//                 asset_open_bid_ask: EngineBidAsk {
//                     asset_pair: "CHFJPY".to_string(),
//                     bid: 149.00,
//                     ask: 149.00,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "CHF".to_string(),
//                     quote: "GPY".to_string(),
//                 },
//                 collateral_base_open_price: 1.08696,
//                 collateral_base_open_bid_ask: Some(EngineBidAsk {
//                     asset_pair: "USDCHF".to_string(),
//                     bid: 0.9200,
//                     ask: 0.9200,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "CHF".to_string(),
//                 }),
//                 open_process_id: "test".to_string(),
//                 open_date: DateTimeAsMicroseconds::now(),
//                 asset_close_price: 149.20,
//                 asset_close_bid_ask: EngineBidAsk {
//                     asset_pair: "CHFJPY".to_string(),
//                     bid: 149.20,
//                     ask: 149.20,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "CHF".to_string(),
//                     quote: "JPY".to_string(),
//                 },
//                 collateral_quote_last_price: 132.1,
//                 collateral_quote_last_bid_ask: Some(EngineBidAsk {
//                     asset_pair: "USDJPY".to_string(),
//                     bid: 132.1,
//                     ask: 132.1,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "JPY".to_string(),
//                 }),
//                 profit: 0.0,
//                 pending_state: None,
//             }),
//         };

//         order.update_pl();

//         let active = order.get_active_state();
//         println!("profit: {}", active.profit);
//     }

//     #[test]
//     fn test_pl_calculate3() {
//         let mut order = EnginePosition {
//             data: EnginePositionBase {
//                 id: "test".to_string(),
//                 trader_id: "test".to_string(),
//                 account_id: "test".to_string(),
//                 asset_pair: "test".to_string(),
//                 side: PositionSide::Buy,
//                 invest_amount: 100000.0,
//                 leverage: 5.0,
//                 stop_out_percent: 0.0,
//                 create_process_id: "test".to_string(),
//                 create_date: DateTimeAsMicroseconds::now(),
//                 last_update_process_id: "test".to_string(),
//                 last_update_date: DateTimeAsMicroseconds::now(),
//                 take_profit_in_position_profit: None,
//                 take_profit_in_asset_price: None,
//                 stop_loss_in_position_profit: None,
//                 stop_loss_in_asset_price: None,
//                 collateral_currency: "USD".to_string(),
//                 base: "USD".to_string(),
//                 quote: "CAD".to_string(),
//             },
//             state: EnginePositionState::Active(ActivePositionState {
//                 asset_open_price: 149.00,
//                 asset_open_bid_ask: EngineBidAsk {
//                     asset_pair: "USDCAD".to_string(),
//                     bid: 149.00,
//                     ask: 149.00,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "CAD".to_string(),
//                 },
//                 collateral_base_open_price: 1.0,
//                 collateral_base_open_bid_ask: None,
//                 open_process_id: "test".to_string(),
//                 open_date: DateTimeAsMicroseconds::now(),
//                 asset_close_price: 149.20,
//                 asset_close_bid_ask: EngineBidAsk {
//                     asset_pair: "USDCAD".to_string(),
//                     bid: 149.20,
//                     ask: 149.20,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "CAD".to_string(),
//                 },
//                 collateral_quote_last_price: 149.20,
//                 collateral_quote_last_bid_ask: Some(EngineBidAsk {
//                     asset_pair: "USDCAD".to_string(),
//                     bid: 149.20,
//                     ask: 149.20,
//                     datetime: DateTimeAsMicroseconds::now(),
//                     base: "USD".to_string(),
//                     quote: "CAD".to_string(),
//                 }),
//                 profit: 0.0,
//                 pending_state: None,
//             }),
//         };

//         order.update_pl();

//         let active = order.get_active_state();
//         println!("profit: {}", active.profit);
//     }
// }
