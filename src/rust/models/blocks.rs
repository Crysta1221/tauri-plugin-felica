use serde::{Deserialize, Serialize};

/// Target service and blocks to read without encryption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceReadDto {
    pub system_code: u16,
    pub service_code: u16,
    pub blocks: Vec<u16>,
}

/// A single read block from a FeliCa card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDataDto {
    pub system_code: u16,
    pub service_code: u16,
    pub block_index: u16,
    pub data_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockReadReasonDto {
    StatusFlag,
    Communication,
    Protocol,
    WrongIdm,
}

/// A failed block read that did not produce payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockReadErrorDto {
    pub system_code: u16,
    pub service_code: u16,
    pub block_index: u16,
    pub reason: BlockReadReasonDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_flag1: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_flag2: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadBlocksResultDto {
    pub blocks: Vec<BlockDataDto>,
    pub errors: Vec<BlockReadErrorDto>,
}
