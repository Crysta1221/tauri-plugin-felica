use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledCodeDto {
    pub code: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationCandidateDto {
    pub region: u8,
    pub company_name: String,
    pub line_name: String,
    pub station_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationRefDto {
    pub line: u8,
    pub station: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<u8>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<StationCandidateDto>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitSettingsDto {
    pub raw: u8,
    pub touch_de_go: bool,
    pub voice_guidance: bool,
    pub sf_outside_commuter: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitIntermediateGateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<StationRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<StationRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown1_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unknown2_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitGateDto {
    pub has_record: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<StationRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate: Option<TransitIntermediateGateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitHistoryEntryDto {
    pub raw_block_hex: String,
    pub block_index: u16,
    pub terminal_type: LabeledCodeDto,
    pub process_type_raw: u8,
    pub process_type: LabeledCodeDto,
    pub payment_type: LabeledCodeDto,
    pub gate_instruction_type: LabeledCodeDto,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<StationRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit: Option<StationRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_company_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_stop_code: Option<u16>,
    pub balance: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<i32>,
    pub seq_number: u32,
    pub entry_region: u8,
    pub exit_region: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitGateRecordDto {
    pub raw_block_hex: String,
    pub block_index: u16,
    pub entry_exit_type: LabeledCodeDto,
    pub intermediate_instruction_type: LabeledCodeDto,
    pub station: StationRefDto,
    pub equipment_id: u16,
    pub date: String,
    pub time: String,
    pub amount: u32,
    pub commuter_fare: u32,
    pub commuter_station: StationRefDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaidTicketDto {
    pub raw_block_hex: String,
    pub block_index: u16,
    pub origin: StationRefDto,
    pub destination: StationRefDto,
    pub expires_at: String,
    pub issued_at: String,
    pub issue_type: u8,
    pub amount: u32,
    pub equipment_id: u16,
    pub gate_station: StationRefDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<TransitSettingsDto>,
    pub gate: TransitGateDto,
    pub histories: Vec<TransitHistoryEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate_records: Option<Vec<TransitGateRecordDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_tickets: Option<Vec<PaidTicketDto>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeDto {
    pub date: String,
    pub time: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaonHistoryEntryDto {
    pub raw: String,
    pub block_index: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_code: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_time: Option<DateTimeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_amount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaonCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
    pub balance: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waon_number: Option<String>,
    pub histories: Vec<WaonHistoryEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmoneyHistoryEntryDto {
    pub raw_block_hex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_code: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdyCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
    pub balance: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edy_number: Option<String>,
    pub histories: Vec<EmoneyHistoryEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NanacoCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
    pub balance: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nanaco_number: Option<String>,
    pub histories: Vec<EmoneyHistoryEntryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuicpayCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteCardDto {
    #[serde(rename = "type")]
    pub card_type: String,
    pub system_code: u16,
    pub idm: String,
    pub s_pad: BTreeMap<u16, String>,
}
