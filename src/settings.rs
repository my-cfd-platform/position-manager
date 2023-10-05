use serde::{Deserialize, Serialize};

service_sdk::macros::use_settings!();

#[derive(
    my_settings_reader::SettingsModel,
    AutoGenerateSettingsTraits,
    SdkSettingsTraits,
    Serialize,
    Deserialize,
    Debug,
    Clone,
)]
pub struct SettingsModel {
    #[serde(rename = "SbTcp")]
    pub my_sb_tcp_host_port: String,
    #[serde(rename = "PersistenceUrl")]
    pub persistence_url: String,
    #[serde(rename = "Seq")]
    pub seq_conn_string: String,
    #[serde(rename = "MyTelemetry")]
    pub my_telemetry: String,
}
