use std::sync::Arc;

use service_sdk::my_telemetry::MyTelemetryContext;
use tokio::sync::RwLock;
use trading_sdk::mt_engine::{ActivePositionsCache, MtBidAskCache, PendingPositionsCache};

use crate::{map_active_persistence, PositionManagerPersistenceClient, map_pending_persistence};

pub async fn load_prices_cache(
    grpc_client: &PositionManagerPersistenceClient,
    telemetry: &MyTelemetryContext,
) -> Arc<RwLock<MtBidAskCache>> {
    telemetry.start_event_tracking("load_prices_cache");

    let prices = grpc_client
        .get_prices_snapshot((), &telemetry)
        .await
        .unwrap();

    if let Some(prices) = prices {
        return Arc::new(RwLock::new(MtBidAskCache::from_iter(
            prices.into_iter().map(|x| x.into()),
        )));
    }

    return Arc::new(RwLock::new(MtBidAskCache::new()));
}

pub async fn load_positions(
    grpc_client: &PositionManagerPersistenceClient,
    prices_cache: &MtBidAskCache,
    telemetry: &MyTelemetryContext,
) -> ActivePositionsCache {
    telemetry.start_event_tracking("load_positions");

    let positions = grpc_client
        .get_active_positions((), &telemetry)
        .await
        .unwrap();

    let mut positions_cache = ActivePositionsCache::new();

    if let Some(positions) = positions {
        for position in positions {
            positions_cache
                .0
                .add_position(map_active_persistence(position, prices_cache).await);
        }
    }

    return positions_cache;
}

pub async fn load_pending_positions(
    grpc_client: &PositionManagerPersistenceClient,
    prices_cache: &MtBidAskCache,
    telemetry: &MyTelemetryContext,
) -> PendingPositionsCache {
    telemetry.start_event_tracking("load_pending_positions");

    let positions = grpc_client
        .get_pending_positions((), &telemetry)
        .await
        .unwrap();

    let mut positions_cache = PendingPositionsCache::new();

    if let Some(positions) = positions {
        for position in positions {
            positions_cache
                .0
                .add_position(map_pending_persistence(position, prices_cache).await);
        }
    }

    return positions_cache;
}
