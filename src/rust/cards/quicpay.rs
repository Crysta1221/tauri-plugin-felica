use crate::cards::profile::{CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::Result;
use crate::models::{BlockDataDto, QuicpayCardDto};

pub struct QuicpayProfile;

impl CardProfile for QuicpayProfile {
    fn card_type(&self) -> &'static str {
        "quicpay"
    }

    fn detection(&self) -> Detection {
        Detection::Polling(&[0x04C1])
    }

    fn candidate_services(&self) -> &'static [u16] {
        &[]
    }

    fn dump_plan(&self, _system_code: u16, _found: &[u16], _detail: bool) -> Vec<ServiceRead> {
        Vec::new()
    }

    fn parse(&self, system_code: u16, idm: &str, _blocks: &[BlockDataDto]) -> Result<ParsedProduct> {
        Ok(ParsedProduct::Quicpay(QuicpayCardDto {
            card_type: "quicpay".into(),
            system_code,
            idm: idm.to_string(),
        }))
    }
}
