use std::sync::Arc;

use position_manager::{
    position_manager_grpc::position_manager_grpc_service_server::PositionManagerGrpcServiceServer,
    AppContext, GrpcService, PricesListener, SettingsReader,
};
use service_sdk::ServiceContext;

#[tokio::main]
async fn main() {
    let settings_reader = Arc::new(SettingsReader::new(".my-cfd").await);

    let mut service_context = ServiceContext::new(settings_reader.clone());
    let app_ctx = Arc::new(AppContext::new(&settings_reader, &service_context).await);
    service_context.register_sb_subscribe(
        Arc::new(PricesListener::new(app_ctx.clone())),
        my_service_bus_abstractions::subscriber::TopicQueueType::PermanentWithSingleConnection,
    ).await;
    service_context.add_grpc_service(PositionManagerGrpcServiceServer::new(GrpcService::new(
        app_ctx,
    )));

    service_context.start_application().await;
}
