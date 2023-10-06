use std::sync::Arc;

use tokio::sync::RwLock;
use trading_sdk::mt_engine::{ActivePositionsCache, MtBidAskCache};

use crate::SettingsReader;

pub async fn load_prices_cache(
    _: &SettingsReader,
) -> Arc<RwLock<MtBidAskCache>> {
    return Arc::new(RwLock::new(MtBidAskCache::new()));
}

pub async fn load_positions(
    _: &SettingsReader,
) -> ActivePositionsCache {
    ActivePositionsCache::new()
}
