use serde::{Deserialize, Serialize};

/// Supported reader preferences for connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderPreference {
    /// Try every supported PaSoRi reader in order.
    Auto,
    /// Connect only to Sony RC-S380.
    RcS380,
    /// Connect only to Sony RC-S300.
    RcS300,
    /// Connect only to Sony RC-S320.
    RcS320,
    /// Connect only to Sony RC-S330 / RC-S360 / RC-S370.
    RcS330,
}

impl Default for ReaderPreference {
    fn default() -> Self {
        Self::Auto
    }
}

/// Information about a connected or detected PaSoRi reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderInfoDto {
    /// Unique identifier for the reader (e.g. "port100-1-2-06c1").
    pub id: String,
    /// Human-readable device name.
    pub name: String,
    /// USB Vendor ID (e.g. 0x054C).
    pub vendor_id: u16,
    /// USB Product ID (e.g. 0x06C1).
    pub product_id: u16,
    /// Underlying chipset or protocol family.
    pub chipset: String,
}

/// Request payload to connect to a reader.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectReaderRequest {
    /// Specific reader ID to connect to, if specified.
    pub id: Option<String>,
    /// Reader preference filter.
    pub preference: Option<ReaderPreference>,
}

/// Response payload after establishing a connection.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectReaderResponse {
    /// Generated session ID for the connection.
    pub session_id: String,
    /// Information about the connected reader.
    pub reader: ReaderInfoDto,
}

/// Request payload to disconnect a reader session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectReaderRequest {
    /// Session ID to close.
    pub session_id: String,
}
