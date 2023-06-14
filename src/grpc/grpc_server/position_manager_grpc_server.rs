use std::pin::Pin;

use tonic::codegen::futures_core::Stream;

use crate::{
    close_position, open_position,
    position_manager_grpc::{
        position_manager_grpc_service_server::PositionManagerGrpcService,
        PositionManagerActivePositionGrpcModel, PositionManagerClosePositionGrpcRequest,
        PositionManagerClosePositionGrpcResponse, PositionManagerGetActivePositionGrpcRequest,
        PositionManagerGetActivePositionGrpcResponse, PositionManagerGetActivePositionsGrpcRequest,
        PositionManagerOpenPositionGrpcRequest, PositionManagerOpenPositionGrpcResponse,
        PositionManagerOperationsCodes, PositionManagerUpdateSlTpGrpcRequest,
        PositionManagerUpdateSlTpGrpcResponse, PositionManagerClosedPositionGrpcModel, PositionManagerClosePositionReason,
    },
    EnginePosition, GrpcService, EngineBidAsk, ExecutionClosePositionReason, EnginePositionState,
};

#[tonic::async_trait]
impl PositionManagerGrpcService for GrpcService {
    type GetAccountActivePositionsStream = Pin<
        Box<
            dyn Stream<Item = Result<PositionManagerActivePositionGrpcModel, tonic::Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn open_position(
        &self,
        request: tonic::Request<PositionManagerOpenPositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerOpenPositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();
        let open_position_result = open_position(&self.app, request).await;

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

    async fn get_active_position(
        &self,
        request: tonic::Request<PositionManagerGetActivePositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerGetActivePositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();
        let reed = self.app.active_positions_cache.read().await;
        if let Some(position) = reed.get_position_by_id(&request.position_id) {
            return Ok(tonic::Response::new(
                PositionManagerGetActivePositionGrpcResponse {
                    position: Some(position.clone().into()),
                    status: PositionManagerOperationsCodes::Ok as i32,
                },
            ));
        };

        return Ok(tonic::Response::new(
            PositionManagerGetActivePositionGrpcResponse {
                position: None,
                status: PositionManagerOperationsCodes::PositionNotFound as i32,
            },
        ));
    }

    async fn close_position(
        &self,
        request: tonic::Request<PositionManagerClosePositionGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerClosePositionGrpcResponse>, tonic::Status> {
        let request = request.into_inner();

        let closed_position = close_position(
            &self.app,
            &request.account_id,
            &request.position_id,
            ExecutionClosePositionReason::ClientCommand,
            &request.process_id,
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

    async fn get_account_active_positions(
        &self,
        request: tonic::Request<PositionManagerGetActivePositionsGrpcRequest>,
    ) -> Result<tonic::Response<Self::GetAccountActivePositionsStream>, tonic::Status> {
        let request = request.into_inner();
        let mut active_cache = self.app.active_positions_cache.write().await;
        let account_positions = active_cache.get_account_active_positions(&request.account_id);

        let to_send = match account_positions {
            Some(positions) => positions
                .iter()
                .map(|position| {
                    let source_position: EnginePosition<EngineBidAsk> = position.to_owned().clone();
                    let grpc_position: PositionManagerActivePositionGrpcModel =
                        source_position.into();
                    grpc_position
                })
                .collect(),
            None => vec![],
        };

        return my_grpc_extensions::grpc_server::send_vec_to_stream(to_send, |x| x).await;
    }

    async fn update_sl_tp(
        &self,
        request: tonic::Request<PositionManagerUpdateSlTpGrpcRequest>,
    ) -> Result<tonic::Response<PositionManagerUpdateSlTpGrpcResponse>, tonic::Status> {
        let request = request.into_inner();
        let mut active_cache = self.app.active_positions_cache.write().await;

        let updated_position = active_cache.update_position(
            &request.position_id,
            |position: Option<&mut EnginePosition<EngineBidAsk>>| match position {
                Some(position_to_update) => {
                    position_to_update.update_sl(&request.sl_in_profit, &request.sl_in_asset_price);
                    position_to_update.update_tp(&request.tp_in_profit, &request.tp_in_asset_price);

                    return Some(position_to_update.clone());
                }
                None => None,
            },
        );

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
}


impl Into<PositionManagerClosedPositionGrpcModel> for EnginePosition<EngineBidAsk> {
    fn into(self) -> PositionManagerClosedPositionGrpcModel {
        let data = self.data;

        let EnginePositionState::Closed(closed_state) = self.state else{
            panic!("Position is not closed");
        };

        let close_position_reason: PositionManagerClosePositionReason =
            closed_state.close_reason.into();

        PositionManagerClosedPositionGrpcModel {
            id: data.id,
            asset_pair: data.asset_pair,
            side: data.side as i32,
            invest_amount: data.invest_amount,
            leverage: data.leverage,
            stop_out_percent: data.stop_out_percent,
            create_process_id: data.create_process_id,
            create_date_unix_timestamp_milis: data.create_date.unix_microseconds as u64,
            last_update_process_id: data.last_update_process_id,
            last_update_date: data.last_update_date.unix_microseconds as u64,
            tp_in_profit: data.take_profit_in_position_profit,
            sl_in_profit: data.stop_loss_in_position_profit,
            tp_in_asset_price: data.take_profit_in_asset_price,
            sl_in_asset_price: data.stop_loss_in_asset_price,
            open_price: closed_state.active_state.asset_open_price,
            open_bid_ask: Some(closed_state.active_state.asset_open_bid_ask.into()),
            open_process_id: closed_state.active_state.open_process_id,
            open_date: closed_state.active_state.open_date.unix_microseconds as u64,
            profit: closed_state.active_state.profit,
            close_price: closed_state.asset_close_price,
            close_bid_ask: Some(closed_state.asset_close_bid_ask.into()),
            close_process_id: closed_state.close_process_id,
            close_reason: close_position_reason as i32,
        }
    }
}