mod caches;
mod execution;
mod utils;
mod app_context;
mod grpc;
mod settings;
mod flows;

pub use caches::*;
pub use execution::*;
pub use utils::*;
pub use app_context::*;
pub use grpc::*;
pub use settings::*;
pub use flows::*;

pub mod position_manager_persistence {
    tonic::include_proto!("position_manager_persistence");
}


pub mod position_manager_grpc {
    tonic::include_proto!("position_manager");
}

pub enum EngineError{
    NoLiquidity,
    PositionNotFound
}