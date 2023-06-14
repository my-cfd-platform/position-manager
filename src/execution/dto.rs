use rust_extensions::date_time::DateTimeAsMicroseconds;

use crate::caches::ExecutionBidAsk;

#[derive(Debug, Clone)]
pub struct EngineBidAsk {
    pub asset_pair: String,
    pub bid: f64,
    pub ask: f64,
    pub datetime: DateTimeAsMicroseconds,
    pub base: String,
    pub quote: String,
}

impl EngineBidAsk {
    pub fn from_abstraction(bid_ask: &impl ExecutionBidAsk) -> EngineBidAsk {
        EngineBidAsk {
            asset_pair: bid_ask.get_asset_pair().to_string(),
            bid: bid_ask.get_bid(),
            ask: bid_ask.get_ask(),
            datetime: bid_ask.get_date().into(),
            base: bid_ask.get_base().to_string(),
            quote: bid_ask.get_quote().to_string(),
        }
    }
}

impl ExecutionBidAsk for EngineBidAsk {
    fn get_asset_pair(&self) -> &str {
        &self.asset_pair
    }

    fn get_bid(&self) -> f64 {
        self.bid
    }

    fn get_ask(&self) -> f64 {
        self.ask
    }

    fn get_date(&self) -> u64 {
        self.datetime.unix_microseconds as u64
    }

    fn get_base(&self) -> &str {
        &self.base
    }

    fn get_quote(&self) -> &str {
        &self.quote
    }
}
