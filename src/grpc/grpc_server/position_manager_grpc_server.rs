use crate::{
    cancel_pending, charge_swaps, close_position, map_active_to_sb_model, open_pending,
    open_position,
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
        PositionManagerPendingPositionGrpcModel, PositionManagerTopUpPositionGrpcRequest,
        PositionManagerTopUpPositionGrpcResponse, PositionManagerUpdateSlTpGrpcRequest,
        PositionManagerUpdateSlTpGrpcResponse, PositionManagerUpdateToppingUpGrpcRequest,
        PositionManagerUpdateToppingUpGrpcResponse,
    },
    GrpcService,
};
use cfd_engine_sb_contracts::{PositionPersistenceEvent, PositionToppingUpEvent};
use my_grpc_extensions::server::with_telemetry;
use service_sdk::{futures_core, my_telemetry::MyTelemetryContext};
use service_sdk::{
    my_grpc_extensions::{self, server::generate_server_stream},
    rust_extensions::date_time::DateTimeAsMicroseconds,
};
use trading_sdk::{
    core::EngineCacheQueryBuilder,
    mt_engine::{apply_position_topping_up, sanitize_sl_tp, MtPositionCloseReason},
};

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

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            "",
            "Got open position request",
            my_telemetry.clone(),
            "request" = &request
        );

        let open_position_result =
            open_position(&self.app, request.clone(), &MyTelemetryContext::new()).await;
        let response = match open_position_result.clone() {
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

        trade_log::trade_log!(
            request.trader_id,
            request.account_id,
            request.process_id,
            "",
            "Returning open position grpc response",
            my_telemetry.clone(),
            "response" = &response,
            "open_position_result" = &open_position_result
        );

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

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            "",
            "Got close position request",
            my_telemetry.clone(),
            "request" = &request
        );

        let closed_position = close_position(
            &self.app,
            &request.trader_id,
            &request.account_id,
            &request.position_id,
            MtPositionCloseReason::ClientCommand,
            &request.process_id,
            my_telemetry,
        )
        .await;

        let response = match &closed_position {
            Ok(position) => PositionManagerClosePositionGrpcResponse {
                position: Some(position.to_owned().into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            Err(error) => {
                let grpc_status: PositionManagerOperationsCodes = error.clone().into();
                PositionManagerClosePositionGrpcResponse {
                    position: None,
                    status: grpc_status as i32,
                }
            }
        };

        trade_log::trade_log!(
            request.trader_id,
            request.account_id,
            request.process_id,
            "",
            "Returning close position grpc response",
            my_telemetry.clone(),
            "close_result" = &closed_position,
            "response" = &response
        );

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn top_up_position(
        &self,
        request: tonic::Request<PositionManagerTopUpPositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerTopUpPositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            &request.position_id,
            "Handled top up position request",
            my_telemetry.clone(),
            "request" = &request
        );

        let updated_position = {
            let mut active_cache = self.app.active_positions_cache.write().await;
            active_cache.0.update_position(&request.position_id, |x| {
                if let Some(src) = x {
                    if src.base_data.topping_up_percent.is_none()
                        && src.base_data.margin_call_percent.is_none()
                    {
                        return None;
                    };

                    src.base_data.last_update_date = DateTimeAsMicroseconds::now();
                    src.base_data.last_update_process_id = request.process_id.clone();
                    apply_position_topping_up(request.topping_up_amount, src);

                    return Some(src.clone());
                }

                return None;
            })
        };

        if let Some(position) = &updated_position {
            let sb_model = PositionPersistenceEvent {
                process_id: request.process_id.clone(),
                update_position: Some(map_active_to_sb_model(position.clone())),
                close_position: None,
                create_position: None,
            };

            self.app
                .active_positions_persistence_publisher
                .publish(&sb_model, Some(my_telemetry))
                .await
                .unwrap();
        }

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            &request.position_id,
            "Processed top up position request",
            my_telemetry.clone(),
            "update_position_result" = &updated_position
        );

        let topping_up_event = PositionToppingUpEvent {
            process_id: request.process_id.clone(),
            position_id: request.position_id.clone(),
            trader_id: request.trader_id.clone(),
            account_id: request.account_id.clone(),
            delta: request.topping_up_amount.clone(),
        };

        self.app
            .topping_up_publisher
            .publish(&topping_up_event, Some(my_telemetry))
            .await
            .unwrap();

        let response = match updated_position.clone() {
            Some(position) => PositionManagerTopUpPositionGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerTopUpPositionGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        };

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn update_topping_up_settings(
        &self,
        request: tonic::Request<PositionManagerUpdateToppingUpGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerUpdateToppingUpGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            &request.position_id,
            "Update topping up settings",
            my_telemetry.clone(),
            "request" = &request
        );

        let updated_position = {
            let mut active_cache = self.app.active_positions_cache.write().await;
            active_cache.0.update_position(&request.position_id, |x| {
                if let Some(src) = x {
                    src.base_data.last_update_date = DateTimeAsMicroseconds::now();
                    src.base_data.last_update_process_id = request.process_id.clone();

                    if request.is_topping_up {
                        if let Some(topping_up_percent) = request.topping_up_percent {
                            src.base_data.topping_up_percent = Some(topping_up_percent);
                        }
                    } else {
                        src.base_data.topping_up_percent = None;
                    }
                    return Some(src.clone());
                }

                return None;
            })
        };

        if let Some(position) = &updated_position {
            let sb_model = PositionPersistenceEvent {
                process_id: request.process_id.clone(),
                update_position: Some(map_active_to_sb_model(position.clone())),
                close_position: None,
                create_position: None,
            };

            self.app
                .active_positions_persistence_publisher
                .publish(&sb_model, Some(my_telemetry))
                .await
                .unwrap();
        };

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            &request.position_id,
            "Processed topping up settings",
            my_telemetry.clone(),
            "update_result" = &updated_position
        );

        let response = match updated_position.clone() {
            Some(position) => PositionManagerUpdateToppingUpGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerUpdateToppingUpGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
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

        let response = match updated_position.clone() {
            Some(position) => PositionManagerChargeSwapGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerChargeSwapGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        };

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            "",
            "Got charge swap request",
            my_telemetry.clone(),
            "request" = &request,
            "updated_position" = &updated_position,
            "response" = &response
        );

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn get_account_active_positions(
        &self,
        request: tonic::Request<PositionManagerGetActivePositionsGrpcRequest>,
    ) -> Result<tonic::Response<Self::GetAccountActivePositionsStream>, tonic::Status> {
        let request = request.into_inner();

        let query = EngineCacheQueryBuilder::new()
            .with_client(&request.trader_id)
            .with_account(&request.account_id);

        let result: Vec<PositionManagerActivePositionGrpcModel> = {
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

        return my_grpc_extensions::grpc_server::send_vec_to_stream(result.into_iter(), |x| x)
            .await;
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
                    src.base_data.last_update_date = DateTimeAsMicroseconds::now();
                    src.base_data.last_update_process_id = request.process_id.clone();
                    sanitize_sl_tp(&mut src.base_data);
                    return Some(src.clone());
                }

                return None;
            })
        };

        if let Some(position) = &updated_position {
            let sb_model = PositionPersistenceEvent {
                process_id: request.process_id.clone(),
                update_position: Some(map_active_to_sb_model(position.clone())),
                close_position: None,
                create_position: None,
            };

            self.app
                .active_positions_persistence_publisher
                .publish(&sb_model, Some(my_telemetry))
                .await
                .unwrap();
        }

        let response = match updated_position.clone() {
            Some(position) => PositionManagerUpdateSlTpGrpcResponse {
                position: Some(position.into()),
                status: PositionManagerOperationsCodes::Ok as i32,
            },
            None => PositionManagerUpdateSlTpGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        };

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            &request.position_id,
            "Got update sl tp request",
            my_telemetry.clone(),
            "request" = &request,
            "updated_position" = &updated_position,
            "response" = &response
        );
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
        let removed = cancel_pending(&self.app, request.clone(), my_telemetry).await;

        let response = match removed.clone() {
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

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            "",
            &request.id,
            "Got cancel pending request",
            my_telemetry.clone(),
            "request" = &request,
            "remove_position" = &removed,
            "response" = &response
        );

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn open_pending(
        &self,
        request: tonic::Request<PositionManagerOpenPendingGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerOpenPendingGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let pending = open_pending(&self.app, request.clone(), my_telemetry).await;

        let response = match pending.clone() {
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

        trade_log::trade_log!(
            &request.trader_id,
            &request.account_id,
            &request.process_id,
            "",
            "Got open pending request",
            my_telemetry.clone(),
            "request" = &request,
            "pending" = &pending,
            "response" = &response
        );

        return Ok(tonic::Response::new(response));
    }

    #[with_telemetry]
    async fn get_account_pending_positions(
        &self,
        request: tonic::Request<PositionManagerGetPendingPositionsGrpcRequest>,
    ) -> Result<tonic::Response<Self::GetAccountPendingPositionsStream>, tonic::Status> {
        let request = request.into_inner();

        let query = EngineCacheQueryBuilder::new()
            .with_client(&request.trader_id)
            .with_account(&request.account_id);

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

        return my_grpc_extensions::grpc_server::send_vec_to_stream(result.into_iter(), |x| x)
            .await;
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
