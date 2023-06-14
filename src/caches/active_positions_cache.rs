use std::collections::HashSet;

use super::{
    ActiveExecutionPosition, ActivePositionsStore, ExecutionBidAsk, ExecutionClosePositionReason,
    ExecutionPositionBase, PositionsStoreIndex, PositionsStoreIndexAccessor,
};

pub struct ActivePositionsCache<T>
where
    T: ExecutionPositionBase + ActiveExecutionPosition + PositionsStoreIndexAccessor + Clone,
{
    pub positions: ActivePositionsStore<T>,
}

impl<T> ActivePositionsCache<T>
where
    T: ExecutionPositionBase + ActiveExecutionPosition + PositionsStoreIndexAccessor + Clone,
{
    pub fn new() -> Self {
        Self {
            positions: ActivePositionsStore::new(),
        }
    }

    pub fn load_positions(&mut self, positions: Vec<T>) {
        for position in positions {
            self.add_position(position);
        }
    }

    pub fn count_positions_in_cache(&self) -> usize {
        self.positions.count_positions()
    }

    pub fn get_account_active_positions(&mut self, account_id: &str) -> Option<Vec<&T>> {
        self.positions
            .query_positions(|index| match index.get_account_positions(account_id) {
                Some(src) => Some(src.iter().map(|x| x.as_str()).collect()),
                None => None,
            })
    }

    pub fn add_position(&mut self, position: T) {
        self.positions.add_position(position);
    }

    pub fn update_position(
        &mut self,
        id: &str,
        update_function: impl Fn(Option<&mut T>) -> Option<T>,
    ) -> Option<T> {
        update_function(self.positions.get_position_mut(id))
    }

    pub fn remove_position(&mut self, id: &str) -> Option<T> {
        self.positions.remove_position(id)
    }

    pub fn get_position_by_id(&self, id: &str) -> Option<&T> {
        self.positions.get_position(id)
    }

    pub fn update_rate(
        &mut self,
        bid_ask: &impl ExecutionBidAsk,
        query: impl Fn(&PositionsStoreIndex) -> Option<HashSet<&str>>,
    ) -> Option<Vec<(String, ExecutionClosePositionReason)>> {
        let positions = self.positions.query_positions_mut(query);

        if let Some(positions) = positions {
            let mut positions_to_close = vec![];

            for position in positions {
                position.handle_bid_ask(bid_ask);

                if let Some(close_reason) = position.get_position_close_reason() {
                    positions_to_close.push((position.get_id().to_string(), close_reason));
                }
            }

            return Some(positions_to_close);
        }

        return None;
    }

    pub fn close_positions(
        &mut self,
        positions_close_reason: Vec<(String, ExecutionClosePositionReason)>,
    ) -> Vec<(T, ExecutionClosePositionReason)> {
        let mut result = Vec::with_capacity(positions_close_reason.len());

        for (id, reason) in positions_close_reason {
            let position = self.positions.remove_position(&id);
            result.push((position.unwrap(), reason));
        }

        return result;
    }
}
