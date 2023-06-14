use my_service_bus_tcp_client::MyServiceBusSettings;
use my_settings_reader::SettingsModel;
use serde::{Deserialize, Serialize};

#[derive(SettingsModel, Serialize, Deserialize, Debug, Clone)]
pub struct SettingsModel {
    #[serde(rename = "SbTcp")]
    pub sb_tcp: String,
    #[serde(rename = "PersistenceUrl")]
    pub persistence_url: String,
}

#[async_trait::async_trait]
impl MyServiceBusSettings for SettingsReader {
    async fn get_host_port(&self) -> String {
        let settings = self.get_settings().await;
        settings.sb_tcp.clone()
    }
}
