use std::{collections::HashMap, sync::Arc};

use super::ExecutionBidAsk;

pub struct ActivePricesCache<T>
where
    T: ExecutionBidAsk + Clone,
{
    prices: HashMap<String, Arc<T>>,
    base_quote_index: HashMap<String, HashMap<String, String>>,
}

impl<T> ActivePricesCache<T>
where
    T: ExecutionBidAsk + Clone,
{
    pub fn new() -> Self {
        Self {
            prices: HashMap::new(),
            base_quote_index: HashMap::new(),
        }
    }

    pub fn update(&mut self, price: T) {
        self.prices
            .insert(price.get_asset_pair().to_string(), Arc::new(price.clone()));

        if let Some(quote_index) = self.base_quote_index.get_mut(price.get_base()) {
            quote_index.insert(
                price.get_quote().to_string(),
                price.get_asset_pair().to_string(),
            );
        } else {
            self.base_quote_index.insert(
                price.get_base().to_string(),
                HashMap::from([(
                    price.get_quote().to_string(),
                    price.get_asset_pair().to_string(),
                )]),
            );
        }
    }

    pub fn get_by_id(&self, asset: &str) -> Option<Arc<T>> {
        let price = self.prices.get(asset)?;
        return Some(price.clone());
    }

    pub fn get_all(&self) -> Vec<Arc<T>> {
        return self.prices.values().map(|price| price.clone()).collect();
    }

    pub fn get_by_currencies(&self, base: &str, quote: &str) -> Option<Arc<T>> {
        if let Some(price) = self.get_by_base_quote(base, quote) {
            return Some(price.clone());
        }

        if let Some(price) = self.get_by_quote_base(base, quote) {
            return Some(price.clone());
        }

        return None;
    }

    fn get_by_base_quote(&self, base: &str, quote: &str) -> Option<&Arc<T>> {
        let quote_indexes = self.base_quote_index.get(base)?;
        let id = quote_indexes.get(quote)?;
        self.prices.get(id)
    }

    fn get_by_quote_base(&self, base: &str, quote: &str) -> Option<&Arc<T>> {
        let quote_indexes = self.base_quote_index.get(quote)?;
        let id = quote_indexes.get(base)?;
        self.prices.get(id)
    }
}
