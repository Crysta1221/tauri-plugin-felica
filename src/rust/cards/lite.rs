use crate::cards::profile::{blocks_for, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::Result;
use crate::models::{BlockDataDto, LiteCardDto};
use std::collections::BTreeMap;

const SVC_PAD: u16 = 0x000B;

pub struct LiteProfile;

impl CardProfile for LiteProfile {
    fn card_type(&self) -> &'static str {
        "lite"
    }

    fn detection(&self) -> Detection {
        Detection::Polling(&[0x88B4])
    }

    fn candidate_services(&self) -> &'static [u16] {
        &[SVC_PAD]
    }

    fn dump_plan(&self, system_code: u16, _found: &[u16], _detail: bool) -> Vec<ServiceRead> {
        vec![ServiceRead {
            system_code,
            service_code: SVC_PAD,
            blocks: (0..=0x0D).collect(),
        }]
    }

    fn parse(&self, system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<ParsedProduct> {
        let mut s_pad = BTreeMap::new();
        for block in blocks_for(blocks, system_code, SVC_PAD) {
            s_pad.insert(block.block_index, block.data_hex.clone());
        }
        Ok(ParsedProduct::Lite(LiteCardDto {
            card_type: "lite".into(),
            system_code,
            idm: idm.to_string(),
            s_pad,
        }))
    }
}
