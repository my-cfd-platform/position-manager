use std::sync::Arc;

use position_manager::{
    position_manager_grpc::position_manager_grpc_service_server::PositionManagerGrpcServiceServer,
    AppContext, GrpcService, PricesListener, SettingsReader,
};
use service_sdk::ServiceInfo;

#[tokio::main]
async fn main() {
    let settings_reader = SettingsReader::new(".my-cfd-platform").await;
    let settings_reader = Arc::new(settings_reader);

    let mut service_context = service_sdk::ServiceContext::new(settings_reader.clone()).await;
    let app_context = Arc::new(AppContext::new(&settings_reader, &service_context).await);

    service_context.configure_grpc_server(|builder| {
        builder.add_grpc_service(PositionManagerGrpcServiceServer::new(GrpcService::new(
            app_context.clone(),
        )))
    });

    let db_job = PricesListener::new(app_context.clone());

    trade_log::core::TRADE_LOG.init_component_name(settings_reader.get_service_name().as_str()).await;

    service_context.register_sb_subscribe(Arc::new(db_job), service_sdk::my_service_bus::abstractions::subscriber::TopicQueueType::PermanentWithSingleConnection).await;

    service_context.start_application().await;
}
