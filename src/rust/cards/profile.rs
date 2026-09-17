use crate::error::Result;
use crate::models::{
    BlockDataDto, EdyCardDto, LiteCardDto, NanacoCardDto, QuicpayCardDto, TransitCardDto, WaonCardDto,
};

use super::edy::EdyProfile;
use super::lite::LiteProfile;
use super::nanaco::NanacoProfile;
use super::quicpay::QuicpayProfile;
use super::transit::TransitProfile;
use super::waon::WaonProfile;

#[derive(Debug, Clone)]
pub struct ServiceRead {
    pub system_code: u16,
    pub service_code: u16,
    pub blocks: Vec<u16>,
}

pub enum Detection {
    Polling(&'static [u16]),
    Service {
        systems: Option<&'static [u16]>,
        any_of: &'static [u16],
    },
}

pub enum ParsedProduct {
    Transit(TransitCardDto),
    Waon(WaonCardDto),
    Edy(EdyCardDto),
    Nanaco(NanacoCardDto),
    Quicpay(QuicpayCardDto),
    Lite(LiteCardDto),
}

pub trait CardProfile: Send + Sync {
    fn card_type(&self) -> &'static str;
    fn detection(&self) -> Detection;
    fn candidate_services(&self) -> &'static [u16];
    fn dump_plan(&self, system_code: u16, found: &[u16], detail: bool) -> Vec<ServiceRead>;
    fn parse(&self, system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<ParsedProduct>;
}

pub fn all_profiles() -> &'static [&'static dyn CardProfile] {
    &[
        &TransitProfile,
        &WaonProfile,
        &EdyProfile,
        &NanacoProfile,
        &QuicpayProfile,
        &LiteProfile,
    ]
}

pub fn blocks_for(
    blocks: &[BlockDataDto],
    system_code: u16,
    service_code: u16,
) -> Vec<&BlockDataDto> {
    let mut matched: Vec<&BlockDataDto> = blocks
        .iter()
        .filter(|b| b.system_code == system_code && b.service_code == service_code)
        .collect();
    matched.sort_by_key(|b| b.block_index);
    matched
}
