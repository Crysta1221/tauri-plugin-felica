use crate::cards::decode::{bcd_string, bits_be, format_date, format_time, hex_upper, purse_balance};
use crate::cards::profile::{blocks_for, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::{Error, Result};
use crate::models::{BlockDataDto, DateTimeDto, WaonCardDto, WaonHistoryEntryDto};

const SVC_BALANCE: u16 = 0x6817;
const SVC_HISTORY: u16 = 0x680B;
const SVC_POINTS: u16 = 0x684B;
const SVC_NUMBER: u16 = 0x684F;

pub struct WaonProfile;

impl CardProfile for WaonProfile {
    fn card_type(&self) -> &'static str {
        "waon"
    }

    fn detection(&self) -> Detection {
        Detection::Service {
            systems: Some(&[0xFE00, 0x8B61, 0x852B]),
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
                blocks: (0..9).collect(),
            });
        }
        if found.contains(&SVC_POINTS) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_POINTS,
                blocks: vec![0, 1, 2],
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
        parse_waon(system_code, idm, blocks).map(ParsedProduct::Waon)
    }
}

fn parse_waon(system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<WaonCardDto> {
    let balance_block = blocks_for(blocks, system_code, SVC_BALANCE)
        .into_iter()
        .next()
        .ok_or_else(|| Error::unsupported_card("WAON balance is missing", vec![system_code]))?;
    let bytes = hex::decode(&balance_block.data_hex)
        .map_err(|_| Error::ProtocolError("Invalid WAON balance hex".into()))?;
    let balance = purse_balance(&bytes).ok_or_else(|| {
        Error::unsupported_card("WAON balance is truncated", vec![system_code])
    })?;

    let history_blocks = blocks_for(blocks, system_code, SVC_HISTORY);
    let mut histories = Vec::new();
    // Pairs are blockIndex 0-5 only.
    for even in (0u16..=5).step_by(2) {
        let some_even = history_blocks.iter().find(|b| b.block_index == even);
        let some_odd = history_blocks.iter().find(|b| b.block_index == even + 1);
        let (Some(even_block), Some(odd_block)) = (some_even, some_odd) else {
            continue;
        };
        let Ok(even_bytes) = hex::decode(&even_block.data_hex) else {
            continue;
        };
        let Ok(odd_bytes) = hex::decode(&odd_block.data_hex) else {
            continue;
        };
        if odd_bytes.iter().all(|&b| b == 0) {
            continue;
        }
        histories.push(parse_pair(even, &even_bytes, &odd_bytes));
    }

    let waon_number = {
        let number_blocks = blocks_for(blocks, system_code, SVC_NUMBER);
        if number_blocks.is_empty() {
            None
        } else {
            let mut raw = Vec::new();
            for block in number_blocks {
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
        }
    };

    Ok(WaonCardDto {
        card_type: "waon".into(),
        system_code,
        idm: idm.to_string(),
        balance,
        points: None,
        waon_number,
        histories,
    })
}

fn parse_pair(even_index: u16, even: &[u8], odd: &[u8]) -> WaonHistoryEntryDto {
    let terminal_id = if even.is_empty() {
        None
    } else {
        Some(hex_upper(&even[..even.len().min(8)]))
    };
    let seq_number = even
        .get(6..8)
        .map(|s| u32::from(u16::from_be_bytes([s[0], s[1]])));

    let year = bits_be(odd, 0, 5).map(|v| 2005 + v);
    let month = bits_be(odd, 5, 4);
    let day = bits_be(odd, 9, 5);
    let hour = bits_be(odd, 14, 5);
    let minute = bits_be(odd, 19, 6);
    let balance = bits_be(odd, 25, 18);
    let amount = bits_be(odd, 43, 18);
    let charge_amount = bits_be(odd, 61, 17);
    let type_code = odd.get(0).copied();

    let date_time = match (year, month, day, hour, minute) {
        (Some(y), Some(m), Some(d), Some(h), Some(min)) => {
            format_date(y, m, d).and_then(|date| {
                format_time(h, min, None).map(|time| DateTimeDto { date, time })
            })
        }
        _ => None,
    };

    let mut raw = even.to_vec();
    raw.extend_from_slice(odd);

    WaonHistoryEntryDto {
        raw: hex_upper(&raw),
        block_index: even_index,
        terminal_id,
        seq_number,
        type_code,
        date_time,
        amount,
        charge_amount,
        balance,
    }
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
    fn balance_is_four_byte_le() {
        let mut data = [0u8; 16];
        data[0] = 0x10;
        data[1] = 0x27;
        data[2] = 0x00;
        data[3] = 0x00;
        let parsed = parse_waon(0xFE00, "IDM", &[block(SVC_BALANCE, 0, data)]).unwrap();
        assert_eq!(parsed.balance, 10000);
    }

    #[test]
    fn history_pairs_only_zero_to_five() {
        let mut balance = [0u8; 16];
        balance[0] = 0x01;
        let mut even = [0u8; 16];
        even[7] = 0x11;
        let mut odd = [0u8; 16];
        odd[0] = 0x08; // year bits 00001 -> 2006 if top 5 of first byte? 00001 = byte 0x08
        let parsed = parse_waon(
            0xFE00,
            "IDM",
            &[
                block(SVC_BALANCE, 0, balance),
                block(SVC_HISTORY, 0, even),
                block(SVC_HISTORY, 1, odd),
                block(SVC_HISTORY, 6, even),
                block(SVC_HISTORY, 7, odd),
            ],
        )
        .unwrap();
        assert_eq!(parsed.histories.len(), 1);
        assert_eq!(parsed.histories[0].block_index, 0);
    }
}
