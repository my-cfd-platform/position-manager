use crate::{
    cancel_pending, charge_swaps, close_position, open_pending, open_position,
    position_manager_grpc::{
        position_manager_grpc_service_server::PositionManagerGrpcService,
        PositionManagerActivePositionGrpcModel, PositionManagerCancelPendingGrpcRequest,
        PositionManagerCancelPendingGrpcResponse, PositionManagerChargeSwapGrpcRequest,
        PositionManagerChargeSwapGrpcResponse, PositionManagerClosePositionGrpcRequest,
        PositionManagerClosePositionGrpcResponse, PositionManagerGetActivePositionGrpcRequest,
        PositionManagerGetActivePositionGrpcResponse, PositionManagerGetActivePositionsGrpcRequest,
        PositionManagerGetPendingPositionGrpcRequest,
        PositionManagerGetPendingPositionGrpcResponse,
        PositionManagerGetPendingPositionsGrpcRequest, PositionManagerOpenPendingGrpcRequest,
        PositionManagerOpenPendingGrpcResponse, PositionManagerOpenPositionGrpcRequest,
        PositionManagerOpenPositionGrpcResponse, PositionManagerOperationsCodes,
        PositionManagerPendingPositionGrpcModel, PositionManagerUpdateSlTpGrpcRequest,
        PositionManagerUpdateSlTpGrpcResponse,
    },
    GrpcService,
};
use my_grpc_extensions::server::with_telemetry;
use rust_extensions::date_time::DateTimeAsMicroseconds;
use service_sdk::futures_core;
use service_sdk::my_grpc_extensions::{self, server::generate_server_stream};
use trading_sdk::{core::EngineCacheQueryBuilder, mt_engine::MtPositionCloseReason};

#[tonic::async_trait]
impl PositionManagerGrpcService for GrpcService {
    generate_server_stream!(stream_name: "GetAccountActivePositionsStream", item_name: "PositionManagerActivePositionGrpcModel");
    generate_server_stream!(stream_name: "GetAccountPendingPositionsStream", item_name: "PositionManagerPendingPositionGrpcModel");

    #[with_telemetry]
    async fn open_position(
        &self,
        request: tonic::Request<PositionManagerOpenPositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerOpenPositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();
        let open_position_result = open_position(&self.app, request, my_telemetry).await;

        let response = match open_position_result {
            Ok(position) => PositionManagerOpenPositionGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            Err(error) => {
                let grpc_status: PositionManagerOperationsCodes = error.into();
                PositionManagerOpenPositionGrpcResponse {
                    position: None,
                    status: grpc_status as i32,
                }
            }
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn get_active_position(
        &self,
        request: tonic::Request<PositionManagerGetActivePositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerGetActivePositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = {
            let reed = self.app.active_positions_cache.read().await;
            let position = reed.0.get_by_id(&request.position_id);

            match position {
                Some(src) => PositionManagerGetActivePositionGrpcResponse {
                    position: Some(src.clone().into()),
                    status: PositionManagerOperationsCodes::Ok as i32,
                },
                None => PositionManagerGetActivePositionGrpcResponse {
                    position: None,
                    status: PositionManagerOperationsCodes::PositionNotFound as i32,
                },
            }
        };
        return Ok(tonic::Response::new(result));
    }

    #[with_telemetry]
    async fn close_position(
        &self,
        request: tonic::Request<PositionManagerClosePositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerClosePositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let closed_position = close_position(
            &self.app,
            &request.account_id,
            &request.position_id,
            MtPositionCloseReason::ClientCommand,
            &request.process_id,
            my_telemetry,
        )
        .await;

        let response = match closed_position {
            Ok(position) => PositionManagerClosePositionGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            Err(error) => {
                let grpc_status: PositionManagerOperationsCodes = error.into();
                PositionManagerClosePositionGrpcResponse {
                    position: None,
                    status: grpc_status as i32,
                }
            }
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn charge_swap(
        &self,
        request: tonic::Request<PositionManagerChargeSwapGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerChargeSwapGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let updated_position = charge_swaps(
            &self.app,
            &format!(
                "process-{}",
                DateTimeAsMicroseconds::now().unix_microseconds
            ),
            &request.position_id,
            request.swap_amount,
            my_telemetry,
        )
        .await;

        let response = match updated_position {
            Some(position) => PositionManagerChargeSwapGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerChargeSwapGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn get_account_active_positions(
        &self,
        request: tonic::Request<PositionManagerGetActivePositionsGrpcRequest>,
    ) -> Result<tonic::Response<Self::GetAccountActivePositionsStream>, tonic::Status> {
        let request = request.into_inner();

        let mut query = EngineCacheQueryBuilder::new();
        query.with_client(&request.trader_id);
        query.with_account(&request.account_id);

        let result = {
            let active_cache = self.app.active_positions_cache.read().await;
            let account_positions = active_cache.0.query_positions(query);

            account_positions
                .iter()
                .map(|x| {
                    let item: PositionManagerActivePositionGrpcModel = x.to_owned().clone().into();

                    return item;
                })
                .collect()
        };

        return my_grpc_extensions::grpc_server::send_vec_to_stream(result, |x| x).await;
    }

    #[with_telemetry]
    async fn update_sl_tp(
        &self,
        request: tonic::Request<PositionManagerUpdateSlTpGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerUpdateSlTpGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let updated_position = {
            let mut active_cache = self.app.active_positions_cache.write().await;
            active_cache.0.update_position(&request.position_id, |x| {
                if let Some(src) = x {
                    src.base_data.sl_price = request.sl_in_asset_price;
                    src.base_data.tp_price = request.tp_in_asset_price;
                    src.base_data.sl_profit = request.sl_in_profit;
                    src.base_data.tp_profit = request.tp_in_profit;
                    return Some(src.clone());
                }

                return None;
            })
        };

        let response = match updated_position {
            Some(position) => PositionManagerUpdateSlTpGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerUpdateSlTpGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        };
        return Ok(tonic::Response::new(response));
    }

    async fn ping(&self, _: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> {
        return Ok(tonic::Response::new(()));
    }

    #[with_telemetry]
    async fn cancel_pending(
        &self,
        request: tonic::Request<PositionManagerCancelPendingGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerCancelPendingGrpcResponse>, tonic::Status> {
        let request = request.into_inner();
        let removed = cancel_pending(&self.app, request, my_telemetry).await;

        let response = match removed {
            Ok(position) => PositionManagerCancelPendingGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            Err(error) => {
                let grpc_status: PositionManagerOperationsCodes = error.into();
                PositionManagerCancelPendingGrpcResponse {
                    position: None,
                    status: grpc_status as i32,
                }
            }
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn open_pending(
        &self,
        request: tonic::Request<PositionManagerOpenPendingGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerOpenPendingGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let pending = open_pending(&self.app, request, my_telemetry).await;

        let response = match pending {
            Ok(position) => PositionManagerOpenPendingGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            Err(error) => {
                let grpc_status: PositionManagerOperationsCodes = error.into();
                PositionManagerOpenPendingGrpcResponse {
                    position: None,
                    status: grpc_status as i32,
                }
            }
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn get_account_pending_positions(
        &self,
        request: tonic::Request<PositionManagerGetPendingPositionsGrpcRequest>,
    ) -> Result<tonic::Response<Self::GetAccountPendingPositionsStream>, tonic::Status> {
        let request = request.into_inner();

        let mut query = EngineCacheQueryBuilder::new();
        query.with_client(&request.trader_id);
        query.with_account(&request.account_id);

        let result = {
            let pending_cache = self.app.pending_positions_cache.read().await;
            let positions = pending_cache.0.query_positions(query);

            let result: Vec<PositionManagerPendingPositionGrpcModel> = positions
                .iter()
                .map(|x| {
                    let item: PositionManagerPendingPositionGrpcModel = x.to_owned().clone().into();

                    return item;
                })
                .collect();

            result
        };

        return my_grpc_extensions::grpc_server::send_vec_to_stream(result, |x| x).await;
    }

    #[with_telemetry]
    async fn get_pending_position(
        &self,
        request: tonic::Request<PositionManagerGetPendingPositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerGetPendingPositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let result = {
            let reed = self.app.pending_positions_cache.read().await;
            let result = reed.0.get_by_id(&request.id);

            match result {
                Some(src) => PositionManagerGetPendingPositionGrpcResponse {
                    position: Some(src.clone().into()),
                    status: PositionManagerOperationsCodes::Ok as i32,
                },
                None => PositionManagerGetPendingPositionGrpcResponse {
                    position: None,
                    status: PositionManagerOperationsCodes::PositionNotFound as i32,
                },
            }
        };

        return Ok(tonic::Response::new(result));
    }
}
