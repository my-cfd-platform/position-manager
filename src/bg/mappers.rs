use cfd_engine_sb_contracts::BidAskSbModel;
use trading_sdk::mt_engine::MtBidAsk;

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
