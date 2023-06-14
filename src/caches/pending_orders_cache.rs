// use std::collections::BTreeMap;

// use tokio::sync::RwLock;

// use crate::{
//     is_ready_to_execute_pending_order, ExecutionBidAsk, ExecutionPositionBase,
//     PendingExecutionOrder,
// };

// pub struct PendingOrdersCache<T>
// where
//     T: ExecutionPositionBase + PendingExecutionOrder,
// {
//     pub orders: RwLock<BTreeMap<String, BTreeMap<String, T>>>,
// }

// impl<T> PendingOrdersCache<T>
// where
//     T: ExecutionPositionBase + PendingExecutionOrder,
// {
//     pub fn new() -> Self {
//         Self {
//             orders: RwLock::new(BTreeMap::new()),
//         }
//     }

//     pub async fn count_orders_in_cache(&self) -> usize {
//         let positions = self.orders.read().await;
//         let mut count = 0;
//         for (_, positions) in positions.iter() {
//             count += positions.len();
//         }
//         count
//     }

//     pub async fn handle_bid_ask(&self, bid_ask: &impl ExecutionBidAsk) -> Vec<T> {

//         let order_ids_to_execute = self.get_orders_to_execute(bid_ask).await;
//         let mut orders_to_execute = vec![];

//         let mut write_lock = self.orders.write().await;

//         if let Some(instument_orders) = write_lock.get_mut(bid_ask.get_asset_pair()) {
//             for id in order_ids_to_execute {
//                 let order = instument_orders.remove(&id).unwrap();
//                 orders_to_execute.push(order);
//             }
//         }

//         return orders_to_execute;
//     }

//     pub async fn add_pending_order(&self, order: T) {
//         let asset_pair = order.get_asset_pair();
//         let mut write_lock = self.orders.write().await;

//         match write_lock.get_mut(asset_pair) {
//             Some(positions_by_id) => {
//                 positions_by_id.insert(order.get_id().to_string(), order);
//             }
//             None => {
//                 write_lock.insert(
//                     asset_pair.to_string(),
//                     BTreeMap::from([(order.get_id().to_string(), order)]),
//                 );
//             }
//         }
//     }

//     pub async fn remove_order(&self, id: &str, asset_pair_id: &str) -> Option<T> {
//         let mut lock = self.orders.write().await;
//         let instrument_positions = lock.get_mut(asset_pair_id)?;
//         return instrument_positions.remove(id);
//     }

//     async fn get_orders_to_execute(&self, bid_ask: &impl ExecutionBidAsk) -> Vec<String> {
//         let lock = self.orders.read().await;
//         let instrument_orders = lock.get(bid_ask.get_asset_pair());

//         if let None = instrument_orders {
//             return vec![];
//         }

//         let mut orders_to_execute = vec![];

//         for (id, order) in instrument_orders.unwrap().iter() {
//             if is_ready_to_execute_pending_order(order, bid_ask) {
//                 orders_to_execute.push(id.clone());
//             }
//         }

//         return orders_to_execute;
//     }
// }
