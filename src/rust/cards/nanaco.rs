use crate::cards::decode::{bcd_string, be32, format_date, format_time, purse_balance};
use crate::cards::profile::{blocks_for, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::{Error, Result};
use crate::models::{BlockDataDto, EmoneyHistoryEntryDto, NanacoCardDto};

const SVC_BALANCE: u16 = 0x5597;
const SVC_HISTORY: u16 = 0x564F;
const SVC_POINTS: u16 = 0x560B;
const SVC_NUMBER: u16 = 0x558B;

pub struct NanacoProfile;

impl CardProfile for NanacoProfile {
    fn card_type(&self) -> &'static str {
        "nanaco"
    }

    fn detection(&self) -> Detection {
        Detection::Service {
            systems: Some(&[0xFE00, 0x04C7]),
            any_of: &[SVC_BALANCE],
        }
    }

    fn candidate_services(&self) -> &'static [u16] {
        &[SVC_BALANCE, SVC_HISTORY, SVC_POINTS, SVC_NUMBER]
    }

    fn dump_plan(&self, system_code: u16, found: &[u16], _detail: bool) -> Vec<ServiceRead> {
        let mut plan = Vec::new();
        if found.contains(&SVC_BALANCE) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_BALANCE,
                blocks: vec![0],
            });
        }
        if found.contains(&SVC_HISTORY) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_HISTORY,
                blocks: (0..5).collect(),
            });
        }
        if found.contains(&SVC_POINTS) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_POINTS,
                blocks: vec![0, 1],
            });
        }
        if found.contains(&SVC_NUMBER) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_NUMBER,
                blocks: vec![0, 1],
            });
        }
        plan
    }

    fn parse(&self, system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<ParsedProduct> {
        parse_nanaco(system_code, idm, blocks).map(ParsedProduct::Nanaco)
    }
}

fn parse_nanaco(system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<NanacoCardDto> {
    let balance_block = blocks_for(blocks, system_code, SVC_BALANCE)
        .into_iter()
        .next()
        .ok_or_else(|| Error::unsupported_card("nanaco balance is missing", vec![system_code]))?;
    let bytes = hex::decode(&balance_block.data_hex)
        .map_err(|_| Error::ProtocolError("Invalid nanaco balance hex".into()))?;
    let balance = purse_balance(&bytes).ok_or_else(|| {
        Error::unsupported_card("nanaco balance is truncated", vec![system_code])
    })?;

    let mut histories = Vec::new();
    for block in blocks_for(blocks, system_code, SVC_HISTORY) {
        let Ok(raw) = hex::decode(&block.data_hex) else {
            continue;
        };
        if raw.len() < 16 || raw.iter().all(|&b| b == 0) {
            continue;
        }
        let packed = be32(&raw, 9);
        let (date, time) = packed
            .and_then(decode_nanaco_stamp)
            .map(|(d, t)| (Some(d), Some(t)))
            .unwrap_or((None, None));
        histories.push(EmoneyHistoryEntryDto {
            raw_block_hex: hex::encode_upper(&raw),
            type_code: Some(raw[0]),
            amount: be32(&raw, 1),
            balance: be32(&raw, 5),
            date,
            time,
        });
    }

    let nanaco_number = {
        let mut raw = Vec::new();
        for block in blocks_for(blocks, system_code, SVC_NUMBER) {
            if let Ok(bytes) = hex::decode(&block.data_hex) {
                raw.extend(bytes);
            }
        }
        let value = bcd_string(&raw);
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    };

    Ok(NanacoCardDto {
        card_type: "nanaco".into(),
        system_code,
        idm: idm.to_string(),
        balance,
        points: None,
        nanaco_number,
        histories,
    })
}

/// Year bit width is not frozen; drop the stamp when the calendar looks implausible.
fn decode_nanaco_stamp(packed: u32) -> Option<(String, String)> {
    let year = 2000 + ((packed >> 21) & 0x7FF);
    let month = (packed >> 17) & 0x0F;
    let day = (packed >> 12) & 0x1F;
    let hour = (packed >> 6) & 0x3F;
    let minute = packed & 0x3F;
    let date = format_date(year, month, day)?;
    let time = format_time(hour, minute, None)?;
    Some((date, time))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BlockDataDto;

    fn block(service: u16, index: u16, data: [u8; 16]) -> BlockDataDto {
        BlockDataDto {
            system_code: 0xFE00,
            service_code: service,
            block_index: index,
            data_hex: hex::encode_upper(data),
        }
    }

    #[test]
    fn balance_is_little_endian_not_big() {
        let mut data = [0u8; 16];
        data[0] = 0xE8;
        data[1] = 0x03;
        data[2] = 0x00;
        data[3] = 0x00;
        let parsed = parse_nanaco(0xFE00, "IDM", &[block(SVC_BALANCE, 0, data)]).unwrap();
        assert_eq!(parsed.balance, 1000);
        assert_ne!(parsed.balance, 0xE8030000);
    }
}
