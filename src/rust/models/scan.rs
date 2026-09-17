use serde::{Deserialize, Serialize};

use super::blocks::{BlockDataDto, BlockReadErrorDto};
use super::card::{EdyCardDto, LiteCardDto, NanacoCardDto, QuicpayCardDto, TransitCardDto, WaonCardDto};

/// Request payload to wait for and scan a card.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCardRequest {
    pub session_id: String,
    /// Timeout in milliseconds until a card is detected. None or 0 means wait indefinitely.
    pub timeout_ms: Option<u64>,
    /// Product types to read. Omitted means every detected known profile.
    pub targets: Option<Vec<String>>,
    /// When true, wait until `targets` match. Default false = `CARD_TYPE_MISMATCH`.
    #[serde(default)]
    pub require: bool,
    /// Read extra transit services 0x108F / 0x184B.
    #[serde(default)]
    pub detail: bool,
}

/// Parsed scan result. `products` is derived on the TypeScript side.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCardResponse {
    pub idm: String,
    pub pmm: String,
    pub system_codes: Vec<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transit: Option<TransitCardDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waon: Option<WaonCardDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edy: Option<EdyCardDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nanaco: Option<NanacoCardDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quicpay: Option<QuicpayCardDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lite: Option<LiteCardDto>,
    pub blocks: Vec<BlockDataDto>,
    pub errors: Vec<BlockReadErrorDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanRequest {
    pub session_id: String,
}
