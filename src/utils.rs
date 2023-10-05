// use std::sync::Arc;

// use crate::caches::ExecutionBidAsk;

// pub fn get_open_price<T: ExecutionBidAsk>(bid_ask: &T, side: &PositionSide) -> f64 {
//     match side {
//         PositionSide::Buy => bid_ask.get_ask(),
//         PositionSide::Sell => bid_ask.get_bid(),
//     }
// }

// pub fn get_open_price_arc<T: ExecutionBidAsk>(bid_ask: &Arc<T>, side: &PositionSide) -> f64 {
//     match side {
//         PositionSide::Buy => bid_ask.get_ask(),
//         PositionSide::Sell => bid_ask.get_bid(),
//     }
// }

// pub fn get_close_price<T: ExecutionBidAsk>(bid_ask: &T, side: &PositionSide) -> f64 {
//     match side {
//         PositionSide::Buy => bid_ask.get_bid(),
//         PositionSide::Sell => bid_ask.get_ask(),
//     }
// }

// pub fn is_inverted(collateral: &str, base: &str, quote: &str) -> bool {
//     return quote != collateral && base == collateral;
// }
