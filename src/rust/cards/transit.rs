use crate::cards::decode::{
    be16, be24, bcd_time, cybernetics_date, format_time, hex_upper, labeled, le16, station_ref,
};
use crate::cards::profile::{blocks_for, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::error::{Error, Result};
use crate::models::{
    BlockDataDto, PaidTicketDto, TransitCardDto, TransitGateDto, TransitGateRecordDto,
    TransitHistoryEntryDto, TransitIntermediateGateDto, TransitSettingsDto,
};

const SVC_ATTR: u16 = 0x008B;
const SVC_HISTORY: u16 = 0x090F;
const SVC_SF_GATE: u16 = 0x10CB;
const SVC_GATE: u16 = 0x108F;
const SVC_TICKET: u16 = 0x184B;

const BUS_PROCESS: [u8; 4] = [0x0D, 0x0F, 0x1F, 0x23];
const RETAIL_PROCESS: [u8; 6] = [0x46, 0x49, 0x4A, 0x4B, 0xC6, 0xCB];
const RETAIL_TERMINAL: [u8; 4] = [0xC7, 0xC8, 0xC9, 0xCA];

pub struct TransitProfile;

impl CardProfile for TransitProfile {
    fn card_type(&self) -> &'static str {
        "transit"
    }

    fn detection(&self) -> Detection {
        Detection::Service {
            systems: None,
            any_of: &[SVC_ATTR, SVC_HISTORY],
        }
    }

    fn candidate_services(&self) -> &'static [u16] {
        &[SVC_ATTR, SVC_HISTORY, SVC_SF_GATE, SVC_GATE, SVC_TICKET]
    }

    fn dump_plan(&self, system_code: u16, found: &[u16], detail: bool) -> Vec<ServiceRead> {
        let mut plan = Vec::new();
        if found.contains(&SVC_ATTR) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_ATTR,
                blocks: vec![0],
            });
        }
        if found.contains(&SVC_HISTORY) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_HISTORY,
                blocks: (0..20).collect(),
            });
        }
        if detail && found.contains(&SVC_SF_GATE) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_SF_GATE,
                blocks: vec![0, 1],
            });
        }
        if detail && found.contains(&SVC_GATE) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_GATE,
                blocks: vec![0, 1, 2],
            });
        }
        if detail && found.contains(&SVC_TICKET) {
            plan.push(ServiceRead {
                system_code,
                service_code: SVC_TICKET,
                blocks: vec![0, 1],
            });
        }
        plan
    }

    fn parse(&self, system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<ParsedProduct> {
        parse_transit(system_code, idm, blocks).map(ParsedProduct::Transit)
    }
}

fn is_bus(terminal: u8, process: u8) -> bool {
    terminal == 0x05 || BUS_PROCESS.contains(&process)
}

fn is_retail(terminal: u8, process: u8) -> bool {
    RETAIL_TERMINAL.contains(&terminal) || RETAIL_PROCESS.contains(&process)
}

fn parse_transit(system_code: u16, idm: &str, blocks: &[BlockDataDto]) -> Result<TransitCardDto> {
    let attr = blocks_for(blocks, system_code, SVC_ATTR);
    let mut balance = None;
    let mut seq_number = None;
    let mut settings = None;
    if let Some(block) = attr.first() {
        if let Ok(bytes) = hex::decode(&block.data_hex) {
            if bytes.len() >= 16 {
                let raw = bytes[8];
                settings = Some(TransitSettingsDto {
                    raw,
                    touch_de_go: raw & (1 << 2) != 0,
                    voice_guidance: raw & (1 << 4) != 0,
                    sf_outside_commuter: raw & (1 << 5) != 0,
                });
                balance = le16(&bytes, 11).map(u32::from);
                seq_number = be16(&bytes, 14).map(u32::from);
            }
        }
    }

    let mut histories = Vec::new();
    for block in blocks_for(blocks, system_code, SVC_HISTORY) {
        let Ok(bytes) = hex::decode(&block.data_hex) else {
            continue;
        };
        if bytes.len() < 16 {
            continue;
        }
        if bytes[0] == 0x00 {
            break;
        }
        let Some(entry) = parse_history(block.block_index, &bytes) else {
            continue;
        };
        histories.push(entry);
    }

    for i in 0..histories.len() {
        if i + 1 < histories.len() {
            let newer = histories[i].balance;
            let older = histories[i + 1].balance;
            histories[i].amount = Some(newer as i32 - older as i32);
        } else {
            histories[i].amount = None;
        }
    }

    let gate = parse_gate(system_code, blocks);
    let gate_records = parse_gate_records(system_code, blocks);
    let paid_tickets = parse_tickets(system_code, blocks);

    if attr.is_empty() && histories.is_empty() {
        return Err(Error::unsupported_card(
            "Transit attribute and history are missing",
            vec![system_code],
        ));
    }

    Ok(TransitCardDto {
        card_type: "transit".into(),
        system_code,
        idm: idm.to_string(),
        balance,
        seq_number,
        settings,
        gate,
        histories,
        gate_records: if gate_records.is_empty() {
            None
        } else {
            Some(gate_records)
        },
        paid_tickets: if paid_tickets.is_empty() {
            None
        } else {
            Some(paid_tickets)
        },
    })
}

fn parse_history(block_index: u16, bytes: &[u8]) -> Option<TransitHistoryEntryDto> {
    let terminal = bytes[0];
    let process_raw = bytes[1];
    let process = process_raw & 0x7F;
    let payment = bytes[2];
    let gate_ins = bytes[3];
    let date = cybernetics_date(be16(bytes, 4)?)?;
    let entry_region = (bytes[15] >> 6) & 0b11;
    let exit_region = (bytes[15] >> 4) & 0b11;
    let balance = u32::from(le16(bytes, 10)?);
    let seq_number = be24(bytes, 12)?;

    let mut time = None;
    let mut entry = None;
    let mut exit = None;
    let mut bus_company_code = None;
    let mut bus_stop_code = None;

    if is_retail(terminal, process) {
        let packed = be16(bytes, 6)?;
        let hour = u32::from((packed >> 11) & 0x1F);
        let minute = u32::from((packed >> 5) & 0x3F);
        let second = u32::from((packed & 0x1F) * 2);
        time = format_time(hour, minute, Some(second));
    } else if is_bus(terminal, process) {
        bus_company_code = be16(bytes, 6).filter(|&code| code != 0);
        bus_stop_code = be16(bytes, 8).filter(|&code| code != 0);
    } else {
        let entry_line = bytes[6];
        let entry_station = bytes[7];
        let exit_line = bytes[8];
        let exit_station = bytes[9];
        if entry_line != 0 || entry_station != 0 {
            entry = Some(station_ref(entry_line, entry_station, Some(entry_region)));
        }
        if exit_line != 0 || exit_station != 0 {
            exit = Some(station_ref(exit_line, exit_station, Some(exit_region)));
        }
    }

    Some(TransitHistoryEntryDto {
        raw_block_hex: hex_upper(bytes),
        block_index,
        terminal_type: labeled(u32::from(terminal)),
        process_type_raw: process_raw,
        process_type: labeled(u32::from(process)),
        payment_type: labeled(u32::from(payment)),
        gate_instruction_type: labeled(u32::from(gate_ins)),
        date,
        time,
        entry,
        exit,
        bus_company_code,
        bus_stop_code,
        balance,
        amount: None,
        seq_number,
        entry_region,
        exit_region,
    })
}

fn parse_gate(system_code: u16, blocks: &[BlockDataDto]) -> TransitGateDto {
    let sf = blocks_for(blocks, system_code, SVC_SF_GATE);
    let mut combined = [0u8; 32];
    if let Some(block) = sf.iter().find(|b| b.block_index == 0) {
        if let Ok(bytes) = hex::decode(&block.data_hex) {
            let n = bytes.len().min(16);
            combined[..n].copy_from_slice(&bytes[..n]);
        }
    }
    if let Some(block) = sf.iter().find(|b| b.block_index == 1) {
        if let Ok(bytes) = hex::decode(&block.data_hex) {
            let n = bytes.len().min(16);
            combined[16..16 + n].copy_from_slice(&bytes[..n]);
        }
    }
    let has_record = combined.iter().any(|&b| b != 0);
    if !has_record {
        return TransitGateDto {
            has_record: false,
            entry: None,
            intermediate: None,
        };
    }

    let entry = if combined[0] != 0 || combined[1] != 0 {
        Some(station_ref(combined[0], combined[1], None))
    } else {
        None
    };

    let mid = &combined[16..32];
    let intermediate = if mid.iter().any(|&b| b != 0) {
        let entry_date = be16(mid, 0).and_then(cybernetics_date);
        let entry_time = format_time(
            u32::from(mid.get(2).copied().unwrap_or(0)),
            u32::from(mid.get(3).copied().unwrap_or(0)),
            None,
        );
        let exit_time = format_time(
            u32::from(mid.get(7).copied().unwrap_or(0)),
            u32::from(mid.get(8).copied().unwrap_or(0)),
            None,
        );
        Some(TransitIntermediateGateDto {
            entry_date,
            entry_time,
            entry: Some(station_ref(mid[4], mid[5], None)),
            exit_time,
            exit: Some(station_ref(mid[9], mid[10], None)),
            unknown1_hex: Some(format!("{:02X}", mid[6])),
            unknown2_hex: Some(format!("{:02X}", mid[11])),
        })
    } else {
        None
    };

    TransitGateDto {
        has_record: true,
        entry,
        intermediate,
    }
}

fn parse_gate_records(system_code: u16, blocks: &[BlockDataDto]) -> Vec<TransitGateRecordDto> {
    let mut out = Vec::new();
    for block in blocks_for(blocks, system_code, SVC_GATE) {
        let Ok(bytes) = hex::decode(&block.data_hex) else {
            continue;
        };
        if bytes.len() < 16 || bytes.iter().all(|&b| b == 0) {
            continue;
        }
        let Some(date) = be16(&bytes, 6).and_then(cybernetics_date) else {
            continue;
        };
        let time = bcd_time(&bytes[8..10]).unwrap_or_else(|| "00:00".into());
        out.push(TransitGateRecordDto {
            raw_block_hex: hex_upper(&bytes),
            block_index: block.block_index,
            entry_exit_type: labeled(u32::from(bytes[0])),
            intermediate_instruction_type: labeled(u32::from(bytes[1])),
            station: station_ref(bytes[2], bytes[3], None),
            equipment_id: be16(&bytes, 4).unwrap_or(0),
            date,
            time,
            amount: u32::from(le16(&bytes, 10).unwrap_or(0)),
            commuter_fare: u32::from(le16(&bytes, 12).unwrap_or(0)),
            commuter_station: station_ref(bytes[14], bytes[15], None),
        });
    }
    out
}

fn parse_tickets(system_code: u16, blocks: &[BlockDataDto]) -> Vec<PaidTicketDto> {
    let mut out = Vec::new();
    for block in blocks_for(blocks, system_code, SVC_TICKET) {
        let Ok(bytes) = hex::decode(&block.data_hex) else {
            continue;
        };
        if bytes.len() < 16 || bytes.iter().all(|&b| b == 0) {
            continue;
        }
        let Some(expires_at) = be16(&bytes, 4).and_then(cybernetics_date) else {
            continue;
        };
        out.push(PaidTicketDto {
            raw_block_hex: hex_upper(&bytes),
            block_index: block.block_index,
            origin: station_ref(bytes[0], bytes[1], None),
            destination: station_ref(bytes[2], bytes[3], None),
            expires_at,
            issued_at: be16(&bytes, 6)
                .and_then(cybernetics_date)
                .unwrap_or_default(),
            issue_type: bytes[8],
            amount: u32::from(bytes[9]) * 10,
            equipment_id: be16(&bytes, 10).unwrap_or(0),
            gate_station: station_ref(bytes[12], bytes[13], None),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::BlockDataDto;

    fn block(service: u16, index: u16, data: [u8; 16]) -> BlockDataDto {
        BlockDataDto {
            system_code: 0x0003,
            service_code: service,
            block_index: index,
            data_hex: hex::encode_upper(data),
        }
    }

    #[test]
    fn attribute_balance_le_and_settings() {
        let mut data = [0u8; 16];
        data[8] = 0b0011_0100; // voice + SF-out + touch-de-go bits 2/4/5
        data[11] = 0xE8;
        data[12] = 0x03; // 1000 LE
        data[14] = 0x00;
        data[15] = 0x2A;
        let parsed = parse_transit(0x0003, "IDM", &[block(SVC_ATTR, 0, data)]).unwrap();
        assert_eq!(parsed.balance, Some(1000));
        assert_eq!(parsed.seq_number, Some(0x002A));
        let settings = parsed.settings.unwrap();
        assert!(settings.touch_de_go);
        assert!(settings.voice_guidance);
        assert!(settings.sf_outside_commuter);
    }

    #[test]
    fn history_breaks_on_terminal_zero_and_splits_regions() {
        let mut used = [0u8; 16];
        used[0] = 0x16;
        used[1] = 0x01;
        used[4] = 0x28; // 2020-01-01 packed-ish; decoder validates month/day
        used[5] = 0x21;
        used[6] = 0x01;
        used[7] = 0x01;
        used[8] = 0x01;
        used[9] = 0x02;
        used[10] = 0xE8;
        used[11] = 0x03;
        used[12] = 0x00;
        used[13] = 0x12;
        used[14] = 0x34;
        used[15] = 0b01_10_0000; // entry region 1, exit region 2

        let empty = [0u8; 16];
        let parsed = parse_transit(
            0x0003,
            "IDM",
            &[block(SVC_ATTR, 0, [0u8; 16]), block(SVC_HISTORY, 0, used), block(SVC_HISTORY, 1, empty)],
        )
        .unwrap();
        assert_eq!(parsed.histories.len(), 1);
        assert_eq!(parsed.histories[0].process_type.code, 0x01);
        assert_eq!(parsed.histories[0].seq_number, 0x001234);
        assert_eq!(parsed.histories[0].entry_region, 1);
        assert_eq!(parsed.histories[0].exit_region, 2);
        assert_eq!(parsed.histories[0].amount, None);
        assert_eq!(parsed.histories[0].balance, 1000);
    }

    #[test]
    fn history_amount_is_newer_minus_older() {
        fn hist(balance: u16, seq: u8) -> [u8; 16] {
            let mut data = [0u8; 16];
            data[0] = 0x16;
            data[1] = 0x01;
            data[4] = 0x28;
            data[5] = 0x21;
            data[10] = balance as u8;
            data[11] = (balance >> 8) as u8;
            data[14] = seq;
            data
        }
        let parsed = parse_transit(
            0x0003,
            "IDM",
            &[
                block(SVC_HISTORY, 0, hist(800, 2)),
                block(SVC_HISTORY, 1, hist(1000, 1)),
            ],
        )
        .unwrap();
        assert_eq!(parsed.histories[0].amount, Some(-200));
        assert_eq!(parsed.histories[1].amount, None);
    }

    #[test]
    fn bus_history_reads_company_code() {
        let mut data = [0u8; 16];
        data[0] = 0x05; // onboard bus terminal
        data[1] = 0x0D; // bus process
        data[4] = 0x28;
        data[5] = 0x21;
        data[6] = 0x0E;
        data[7] = 0x51; // Nara Kotsu
        data[8] = 0x00;
        data[9] = 0x12;
        data[10] = 0xE8;
        data[11] = 0x03;
        data[14] = 0x01;
        let entry = parse_history(0, &data).unwrap();
        assert_eq!(entry.bus_company_code, Some(0x0E51));
        assert_eq!(entry.bus_stop_code, Some(0x0012));
        assert!(entry.entry.is_none());
        assert!(entry.exit.is_none());
    }
}
