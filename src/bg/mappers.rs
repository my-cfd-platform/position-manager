use cfd_engine_sb_contracts::BidAskSbModel;
use trading_sdk::mt_engine::MtBidAsk;

use crate::position_manager_persistence::PositionManagerPersistenceBidAsk;

pub fn map_bid_ask(bid_ask: BidAskSbModel) -> MtBidAsk {
    MtBidAsk {
        asset_pair: bid_ask.id,
        bid: bid_ask.bid,
        ask: bid_ask.ask,
        date: bid_ask.date_time_unix_milis.into(),
        base: bid_ask.base,
        quote: bid_ask.quote,
    }
}

impl Into<MtBidAsk> for PositionManagerPersistenceBidAsk {
    fn into(self) -> MtBidAsk {
        MtBidAsk {
            asset_pair: self.asset_pair,
            bid: self.bid,
            ask: self.ask,
            base: self.base,
            quote: self.quote,
            date: self.date_time_unix_timestamp_milis.into(),
        }
    }
}
