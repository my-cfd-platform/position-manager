use cfd_engine_sb_contracts::BidAskSbModel;

use crate::EngineBidAsk;

impl From<BidAskSbModel> for EngineBidAsk {
    fn from(value: BidAskSbModel) -> Self {
        Self {
            asset_pair: value.id,
            bid: value.bid,
            ask: value.ask,
            datetime: value.date_time_unix_milis.into(),
            base: value.base,
            quote: value.quote,
        }
    }
}
