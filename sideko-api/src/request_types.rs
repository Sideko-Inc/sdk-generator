use crate::schemas::*;
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct LoginUrlRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cli_port: Option<i64>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct CliCheckUpdatesRequest {
    pub cli_version: String,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone)]
pub struct StatelessGenerateSdkRequest {
    pub data: StatelessGenerateSdk,
}
