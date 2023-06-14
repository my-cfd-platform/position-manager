use std::collections::{BTreeMap, HashSet};

use super::{ActiveExecutionPosition, ExecutionPositionBase};

pub trait PositionsStoreIndexAccessor {
    fn get_account_index(&self) -> Option<String>;
    fn get_base_coll_index(&self) -> Option<String>;
    fn get_quote_coll_index(&self) -> Option<String>;
    fn get_instrument_index(&self) -> Option<String>;
}

#[derive(Debug)]
pub struct PositionsStoreIndex {
    accounts_index: BTreeMap<String, HashSet<String>>,
    base_coll_index: BTreeMap<String, HashSet<String>>,
    quote_coll_index: BTreeMap<String, HashSet<String>>,
    instruments_index: BTreeMap<String, HashSet<String>>,
}

impl PositionsStoreIndex {
    pub fn new() -> Self {
        Self {
            accounts_index: BTreeMap::new(),
            base_coll_index: BTreeMap::new(),
            quote_coll_index: BTreeMap::new(),
            instruments_index: BTreeMap::new(),
        }
    }

    pub fn add<T: PositionsStoreIndexAccessor + ExecutionPositionBase>(&mut self, item: &T) {
        if let Some(account_id) = item.get_account_index() {
            if let Some(accounts_index) = self.accounts_index.get_mut(&account_id) {
                accounts_index.insert(item.get_id().to_string());
            } else {
                self.accounts_index.insert(
                    account_id.to_string(),
                    HashSet::from_iter(vec![item.get_id().to_string()]),
                );
            }
        };

        if let Some(quote_coll) = item.get_quote_coll_index() {
            if let Some(quote_coll_index) = self.quote_coll_index.get_mut(&quote_coll) {
                quote_coll_index.insert(item.get_id().to_string());
            } else {
                self.quote_coll_index.insert(
                    quote_coll.to_string(),
                    HashSet::from_iter(vec![item.get_id().to_string()]),
                );
            }
        };

        if let Some(base_coll) = item.get_base_coll_index() {
            if let Some(base_coll_index) = self.base_coll_index.get_mut(&base_coll) {
                base_coll_index.insert(item.get_id().to_string());
            } else {
                self.base_coll_index.insert(
                    base_coll.to_string(),
                    HashSet::from_iter(vec![item.get_id().to_string()]),
                );
            }
        };

        if let Some(asset_pair) = item.get_instrument_index() {
            if let Some(instruments_index) = self.instruments_index.get_mut(&asset_pair) {
                instruments_index.insert(item.get_id().to_string());
            } else {
                self.instruments_index.insert(
                    asset_pair.to_string(),
                    HashSet::from_iter(vec![item.get_id().to_string()]),
                );
            }
        };
    }

    pub fn remove<T: PositionsStoreIndexAccessor + ExecutionPositionBase>(&mut self, item: &T) {
        if let Some(account_id) = item.get_account_index() {
            self.accounts_index
                .get_mut(&account_id)
                .unwrap()
                .remove(item.get_id());
        }

        if let Some(quote_coll) = item.get_quote_coll_index() {
            self.quote_coll_index
                .get_mut(&quote_coll)
                .unwrap()
                .remove(item.get_id());
        }

        if let Some(base_coll) = item.get_base_coll_index() {
            self.base_coll_index
                .get_mut(&base_coll)
                .unwrap()
                .remove(item.get_id());
        }

        if let Some(asset_pair) = item.get_instrument_index() {
            self.instruments_index
                .get_mut(&asset_pair)
                .unwrap()
                .remove(item.get_id());
        }
    }

    pub fn get_quote_coll_positions(&self, currency: &str) -> Option<&HashSet<String>> {
        self.quote_coll_index.get(currency)
    }

    pub fn get_base_coll_positions(&self, currency: &str) -> Option<&HashSet<String>> {
        self.base_coll_index.get(currency)
    }

    pub fn get_account_positions(&self, account_id: &str) -> Option<&HashSet<String>> {
        self.accounts_index.get(account_id)
    }

    pub fn get_instrument_positions(&self, asset_pair: &str) -> Option<&HashSet<String>> {
        self.instruments_index.get(asset_pair)
    }
}
// (pub BTreeMap<String, T>, pub PositionsStoreIndex)
pub struct ActivePositionsStore<T>
where
    T: ExecutionPositionBase + ActiveExecutionPosition + PositionsStoreIndexAccessor + Clone,
{
    pub positions: BTreeMap<String, T>,
    pub index: PositionsStoreIndex,
}

//Base implement
impl<T> ActivePositionsStore<T>
where
    T: ExecutionPositionBase + ActiveExecutionPosition + PositionsStoreIndexAccessor + Clone,
{
    pub fn new() -> Self {
        Self {
            positions: BTreeMap::new(),
            index: PositionsStoreIndex::new(),
        }
    }

    pub fn count_positions(&self) -> usize {
        self.positions.len()
    }

    pub fn add_position(&mut self, position: T) {
        self.positions
            .insert(position.get_id().to_string(), position.clone());
        self.index.add(&position);
    }

    pub fn get_position(&self, id: &str) -> Option<&T> {
        self.positions.get(id)
    }

    pub fn get_position_mut(&mut self, id: &str) -> Option<&mut T> {
        self.positions.get_mut(id)
    }

    pub fn remove_position(&mut self, id: &str) -> Option<T> {
        let removed_position = self.positions.remove(id);
        if let Some(position) = removed_position.as_ref() {
            self.index.remove(position);
        }

        removed_position
    }

    pub fn query_positions<'a>(
        &self,
        query: impl Fn(&PositionsStoreIndex) -> Option<HashSet<&str>>,
    ) -> Option<Vec<&T>> {
        let ids_to_search = query(&self.index);

        if ids_to_search.is_none() || self.positions.is_empty() {
            return None;
        }

        let mut result = vec![];

        for (id, position) in &self.positions {
            if ids_to_search.as_ref().unwrap().contains(id.as_str()) {
                result.push(position);
            }
        }

        return Some(result);
    }

    pub fn query_positions_mut(
        &mut self,
        query: impl Fn(&PositionsStoreIndex) -> Option<HashSet<&str>>,
    ) -> Option<Vec<&mut T>> {
        let ids_to_search = query(&self.index);

        if ids_to_search.is_none() || self.positions.is_empty() {
            return None;
        }

        let mut result = vec![];

        for (id, position) in self.positions.iter_mut() {
            if ids_to_search.as_ref().unwrap().contains(id.as_str()) {
                result.push(position);
            }
        }

        return Some(result);
    }
}

#[cfg(test)]
mod tests {
    use crate::caches::{
        ExecutionClosePositionReason, ExecutionPositionBase, PositionSide, PositionsStoreIndex,
    };

    use super::PositionsStoreIndexAccessor;

    pub struct TestIndex {
        pub account: String,
        pub id: String,
        pub base: String,
        pub quote: String,
        pub asset_pair: String,
    }

    impl PositionsStoreIndexAccessor for TestIndex {
        fn get_account_index(&self) -> Option<String> {
            Some(self.account.clone())
        }

        fn get_base_coll_index(&self) -> Option<String> {
            Some(self.base.clone())
        }

        fn get_quote_coll_index(&self) -> Option<String> {
            Some(self.quote.clone())
        }

        fn get_instrument_index(&self) -> Option<String> {
            Some(self.asset_pair.clone())
        }
    }

    impl ExecutionPositionBase for TestIndex {
        fn get_id(&self) -> &str {
            &self.id
        }

        fn get_account_id(&self) -> &str {
            todo!()
        }

        fn get_asset_pair(&self) -> &str {
            todo!()
        }

        fn get_side(&self) -> &PositionSide {
            todo!()
        }

        fn get_invest_amount(&self) -> f64 {
            todo!()
        }

        fn get_so_percent(&self) -> f64 {
            todo!()
        }

        fn get_position_close_reason(&self) -> Option<ExecutionClosePositionReason> {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_account_index() {
        let indx = TestIndex {
            account: "ac1".to_string(),
            id: "id1".to_string(),
            base: "test".to_string(),
            quote: "test".to_string(),
            asset_pair: "test".to_string(),
        };

        let indx2 = TestIndex {
            account: "ac1".to_string(),
            id: "id2".to_string(),
            base: "test".to_string(),
            quote: "test".to_string(),
            asset_pair: "test".to_string(),
        };

        let mut indexses = PositionsStoreIndex::new();
        indexses.add(&indx);
        indexses.add(&indx2);

        let account_positions = indexses.get_account_positions("ac1");

        assert_eq!(true, account_positions.is_some());

        let positions = account_positions.unwrap();
        assert_eq!(2, positions.len());

        let positions = positions.into_iter().collect::<Vec<&String>>();

        assert_eq!("id1", positions[0]);
        assert_eq!("id2", positions[1]);
    }
}
