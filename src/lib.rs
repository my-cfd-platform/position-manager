mod app_context;
mod bg;
mod flows;
mod grpc;
mod settings;
mod utils;

pub use app_context::*;
pub use bg::*;
pub use flows::*;
pub use grpc::*;
use serde::{Serialize, Deserialize};
pub use settings::*;
pub use utils::*;

pub mod position_manager_persistence {
    tonic::include_proto!("position_manager_persistence");
}

pub mod position_manager_grpc {
    tonic::include_proto!("position_manager");
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineError {
    NoLiquidity,
    PositionNotFound,
}
