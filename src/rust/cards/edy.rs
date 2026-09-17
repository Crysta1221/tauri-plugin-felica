use crate::cards::decode::{bcd_string, be32, format_date, format_time, purse_balance};
use crate::cards::profile::{blocks_for, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::{Error, Result};
use crate::models::{BlockDataDto, EdyCardDto, EmoneyHistoryEntryDto};

const SVC_BALANCE: u16 = 0x1317;
const SVC_HISTORY: u16 = 0x170F;
const SVC_ATTR: u16 = 0x110B;

pub struct EdyProfile;

impl CardProfile for EdyProfile {
    fn card_type(&self) -> &'static str {
        "edy"
    }

    fn detection(&self) -> Detection {
        Detection::Service {
            systems: Some(&[0xFE00, 0x811D]),
            any_of: &[SVC_BALANCE],
        }
    }

    fn candidate_services(&self) -> &'static [u16] {
        &[SVC_BALANCE, SVC_HISTORY, SVC_ATTR]
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
                blocks: (0..6).collect(),
            });
        }
        if found.contains(&SVC_ATTR) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_ATTR,
                blocks: vec![0, 1],
            });
        }
        plan
    }

    fn parse(&self, system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<ParsedProduct> {
        parse_edy(system_code, idm, blocks).map(ParsedProduct::Edy)
    }
}

fn parse_edy(system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<EdyCardDto> {
    let balance_block = blocks_for(blocks, system_code, SVC_BALANCE)
        .into_iter()
        .next()
        .ok_or_else(|| Error::unsupported_card("Edy balance is missing", vec![system_code]))?;
    let bytes = hex::decode(&balance_block.data_hex)
        .map_err(|_| Error::ProtocolError("Invalid Edy balance hex".into()))?;
    let balance = purse_balance(&bytes)
        .ok_or_else(|| Error::unsupported_card("Edy balance is truncated", vec![system_code]))?;

    let mut histories = Vec::new();
    for block in blocks_for(blocks, system_code, SVC_HISTORY) {
        let Ok(raw) = hex::decode(&block.data_hex) else {
            continue;
        };
        if raw.len() < 16 || raw.iter().all(|&b| b == 0) {
            continue;
        }
        let stamp = edy_timestamp(&raw, 4);
        histories.push(EmoneyHistoryEntryDto {
            raw_block_hex: hex::encode_upper(&raw),
            type_code: Some(raw[0]),
            amount: be32(&raw, 8),
            balance: be32(&raw, 12),
            date: stamp.as_ref().map(|s| s.0.clone()),
            time: stamp.as_ref().map(|s| s.1.clone()),
        });
    }

    let edy_number = {
        let mut raw = Vec::new();
        for block in blocks_for(blocks, system_code, SVC_ATTR) {
            if let Ok(bytes) = hex::decode(&block.data_hex) {
                raw.extend(bytes);
            }
        }
        if raw.is_empty() {
            None
        } else {
            let value = bcd_string(&raw);
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
    };

    Ok(EdyCardDto {
        card_type: "edy".into(),
        system_code,
        idm: idm.to_string(),
        balance,
        edy_number,
        histories,
    })
}

fn edy_timestamp(bytes: &[u8], offset: usize) -> Option<(String, String)> {
    let packed = be32(bytes, offset)?;
    let days = (packed >> 17) & 0x7FFF;
    let seconds = packed & 0x1FFFF;
    let total = i64::from(days) * 86_400 + i64::from(seconds);
    let year_days = days_to_ymd(2000, total / 86_400)?;
    let rem = (total % 86_400) as u32;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    let date = format_date(year_days.0, year_days.1, year_days.2)?;
    let time = format_time(hour, minute, Some(second))?;
    Some((date, time))
}

fn days_to_ymd(start_year: u32, days: i64) -> Option<(u32, u32, u32)> {
    if days < 0 {
        return None;
    }
    let mut year = start_year;
    let mut remaining = days as u32;
    loop {
        let len = if is_leap(year) { 366 } else { 365 };
        if remaining < len {
            break;
        }
        remaining -= len;
        year += 1;
        if year > 2100 {
            return None;
        }
    }
    const MD: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for month in 1..=12 {
        let mut dim = MD[month as usize - 1];
        if month == 2 && is_leap(year) {
            dim = 29;
        }
        if remaining < dim {
            return Some((year, month, remaining + 1));
        }
        remaining -= dim;
    }
    None
}

fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
    fn purse_le_and_unsigned_history_amount() {
        let mut balance = [0u8; 16];
        balance[0] = 0xE8;
        balance[1] = 0x03;
        let mut hist = [0u8; 16];
        hist[0] = 0x02;
        hist[8] = 0x80;
        hist[9] = 0x00;
        hist[10] = 0x00;
        hist[11] = 0x00; // 0x80000000 as BE would be negative if signed-shifted
        // amount at 8..12 BE = 0x80000000
        hist[8] = 0x00;
        hist[9] = 0x00;
        hist[10] = 0x00;
        hist[11] = 0x64;
        hist[12] = 0x00;
        hist[13] = 0x00;
        hist[14] = 0x03;
        hist[15] = 0xE8;
        let parsed = parse_edy(
            0xFE00,
            "IDM",
            &[block(SVC_BALANCE, 0, balance), block(SVC_HISTORY, 0, hist)],
        )
        .unwrap();
        assert_eq!(parsed.balance, 1000);
        assert_eq!(parsed.histories[0].amount, Some(100));
        assert_eq!(parsed.histories[0].balance, Some(1000));
    }
}
