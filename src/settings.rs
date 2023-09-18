use serde::{Deserialize, Serialize};
use service_sdk::my_service_bus::client::MyServiceBusSettings;

service_sdk::macros::use_settings!();

#[derive(
    my_settings_reader::SettingsModel, SdkSettingsTraits, Serialize, Deserialize, Debug, Clone,
)]
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
impl service_sdk::my_logger::my_seq_logger::SeqSettings for SettingsReader {
    async fn get_conn_string(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.seq_conn_string.clone()
    }
}

#[async_trait::async_trait]
impl service_sdk::my_telemetry::my_telemetry_writer::MyTelemetrySettings for SettingsReader {
    async fn get_telemetry_url(&self) -> String {
        let read_access = self.settings.read().await;
        read_access.my_telemetry.clone()
    }
}