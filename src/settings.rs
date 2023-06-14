use my_service_bus_tcp_client::MyServiceBusSettings;
use my_settings_reader::SettingsModel;
use serde::{Deserialize, Serialize};
use service_sdk::ServiceInfo;

#[derive(SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    #[serde(rename = "SbTcp")]
    pub sb_tcp: String,
    #[serde(rename = "PersistenceUrl")]
    pub persistence_url: String,
    #[serde(rename = "Seq")]
    pub seq_conn_string: String,
    #[serde(rename = "MyTelemetry")]
    pub my_telemetry: String,
}

#[async_trait::async_trait]
impl MyServiceBusSettings for SettingsReader {
    async fn get_host_port(&self) -> String {
        let settings = self.get_settings().await;
        settings.sb_tcp.clone()
    }
}



#[async_trait::async_trait]
impl my_seq_logger::SeqSettings for SettingsReader {
    async fn get_conn_string(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.seq_conn_string.clone()
    }
}


#[async_trait::async_trait]
impl my_telemetry_writer::MyTelemetrySettings for SettingsReader {
    async fn get_telemetry_url(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.my_telemetry.clone()
    }
}


#[async_trait::async_trait]
impl ServiceInfo for SettingsReader {
    fn get_service_name(&self) -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    fn get_service_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}