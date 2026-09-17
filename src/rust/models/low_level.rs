use serde::{Deserialize, Serialize};

use super::blocks::ServiceReadDto;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadBlocksRequest {
    pub session_id: String,
    pub timeout_ms: Option<u64>,
    pub services: Vec<ServiceReadDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollCardRequest {
    pub session_id: String,
    pub timeout_ms: Option<u64>,
    pub system_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PollCardResponse {
    pub idm: String,
    pub pmm: String,
    pub system_codes: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOnlyRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestServiceCommand {
    pub session_id: String,
    pub service_codes: Vec<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatusDto {
    pub service_code: u16,
    pub key_version: u16,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestServiceResponseDto {
    pub results: Vec<ServiceStatusDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchServicesRequest {
    pub session_id: String,
    pub start_index: Option<u16>,
    pub max_nodes: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceNodeDto {
    pub index: u16,
    pub code: u16,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchServicesResponse {
    pub nodes: Vec<ServiceNodeDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectSystemRequest {
    pub session_id: String,
    pub system_code: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectSystemResponse {
    pub idm: String,
    pub pmm: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardReadRequest {
    pub session_id: String,
    pub service_code: u16,
    pub blocks: Vec<u16>,
}
