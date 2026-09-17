use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::cards::profile::ServiceRead;
use crate::error::{Error, Result};
use crate::hardware::ReaderDevice;
use crate::models::{
    BlockDataDto, BlockReadErrorDto, BlockReadReasonDto, ReadBlocksResultDto, ServiceNodeDto,
    ServiceReadDto,
};
use crate::protocol::command::{
    build_polling, build_read_without_encryption_single, build_request_service,
    build_request_system_code, build_search_service_code,
};
use crate::protocol::response::{
    parse_polling_response_for, parse_read_without_encryption_response_for,
    parse_request_service_response_for, parse_search_service_code_response_for,
    parse_system_codes_response_for, PollingResponse, SearchNodeKind,
};

pub(crate) const POLL_RETRY: Duration = Duration::from_millis(200);
const CHUNK: usize = 4;
pub(crate) const WILDCARD: u16 = 0xFFFF;

pub(crate) fn io_timeout(deadline: Option<Instant>, default_ms: u16) -> Result<u16> {
    let Some(deadline) = deadline else {
        return Ok(default_ms);
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::ScanTimeout);
    }
    Ok(remaining.as_millis().min(u128::from(default_ms)) as u16)
}

pub(crate) fn poll_system(device: &mut dyn ReaderDevice, system_code: u16) -> Option<PollingResponse> {
    poll_system_timed(device, system_code, 80)
}

pub(crate) fn poll_system_timed(
    device: &mut dyn ReaderDevice,
    system_code: u16,
    timeout_ms: u16,
) -> Option<PollingResponse> {
    let cmd = build_polling(system_code, 0x01, 0x00);
    let resp = device.transceive(&cmd, timeout_ms).ok()?;
    parse_polling_response_for(&resp, None).ok()
}

pub(crate) fn request_system_codes(device: &mut dyn ReaderDevice, idm: &[u8; 8]) -> Result<Vec<u16>> {
    let cmd = build_request_system_code(idm);
    let resp = device.transceive(&cmd, 100)?;
    parse_system_codes_response_for(&resp, Some(idm))
}

pub(crate) fn request_services(
    device: &mut dyn ReaderDevice,
    idm: &[u8; 8],
    codes: &[u16],
) -> Result<Vec<u16>> {
    if codes.is_empty() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for chunk in codes.chunks(32) {
        let cmd = build_request_service(idm, chunk);
        let resp = device.transceive(&cmd, 120)?;
        let parsed = parse_request_service_response_for(&resp, Some(idm))?;
        for (code, version) in chunk.iter().zip(parsed.key_versions.iter()) {
            if *version != 0xFFFF {
                found.push(*code);
            }
        }
    }
    Ok(found)
}

pub(crate) fn read_service(
    device: &mut dyn ReaderDevice,
    item: &ServiceRead,
    idm: &[u8; 8],
    out: &mut ReadBlocksResultDto,
) {
    for chunk in item.blocks.chunks(CHUNK) {
        let cmd = build_read_without_encryption_single(idm, item.service_code, chunk);
        match device.transceive(&cmd, 250) {
            Ok(resp) => match parse_read_without_encryption_response_for(&resp, Some(idm)) {
                Ok(parsed) if parsed.status_flag1 == 0x00 => {
                    if parsed.blocks.len() != chunk.len() {
                        for &block_index in chunk {
                            out.errors.push(BlockReadErrorDto {
                                system_code: item.system_code,
                                service_code: item.service_code,
                                block_index,
                                reason: BlockReadReasonDto::Protocol,
                                status_flag1: None,
                                status_flag2: None,
                            });
                        }
                    } else {
                        for (block_index, data) in chunk.iter().zip(parsed.blocks.iter()) {
                            out.blocks.push(BlockDataDto {
                                system_code: item.system_code,
                                service_code: item.service_code,
                                block_index: *block_index,
                                data_hex: hex::encode_upper(data),
                            });
                        }
                    }
                }
                Ok(parsed) => {
                    for &block_index in chunk {
                        out.errors.push(BlockReadErrorDto {
                            system_code: item.system_code,
                            service_code: item.service_code,
                            block_index,
                            reason: BlockReadReasonDto::StatusFlag,
                            status_flag1: Some(parsed.status_flag1),
                            status_flag2: Some(parsed.status_flag2),
                        });
                    }
                }
                Err(Error::ProtocolError(msg)) if msg.contains("IDm mismatch") => {
                    for &block_index in chunk {
                        out.errors.push(BlockReadErrorDto {
                            system_code: item.system_code,
                            service_code: item.service_code,
                            block_index,
                            reason: BlockReadReasonDto::WrongIdm,
                            status_flag1: None,
                            status_flag2: None,
                        });
                    }
                }
                Err(_) => {
                    for &block_index in chunk {
                        out.errors.push(BlockReadErrorDto {
                            system_code: item.system_code,
                            service_code: item.service_code,
                            block_index,
                            reason: BlockReadReasonDto::Protocol,
                            status_flag1: None,
                            status_flag2: None,
                        });
                    }
                }
            },
            Err(_) => {
                for &block_index in chunk {
                    out.errors.push(BlockReadErrorDto {
                        system_code: item.system_code,
                        service_code: item.service_code,
                        block_index,
                        reason: BlockReadReasonDto::Communication,
                        status_flag1: None,
                        status_flag2: None,
                    });
                }
            }
        }
    }
}

pub fn read_services(
    device: &mut dyn ReaderDevice,
    services: &[ServiceReadDto],
) -> Result<ReadBlocksResultDto> {
    let poll = poll_system(device, WILDCARD)
        .ok_or_else(|| Error::CommunicationError("No card present".into()))?;
    let mut identities: HashMap<u16, [u8; 8]> = HashMap::new();
    identities.insert(0, poll.idm);

    let mut out = ReadBlocksResultDto {
        blocks: Vec::new(),
        errors: Vec::new(),
    };
    for service in services {
        let idm = if let Some(idm) = identities.get(&service.system_code) {
            *idm
        } else if let Some(p) = poll_system(device, service.system_code) {
            identities.insert(service.system_code, p.idm);
            p.idm
        } else {
            poll.idm
        };
        let item = ServiceRead {
            system_code: service.system_code,
            service_code: service.service_code,
            blocks: service.blocks.clone(),
        };
        read_service(device, &item, &idm, &mut out);
    }
    Ok(out)
}

pub fn poll_card(
    device: &mut dyn ReaderDevice,
    system_code: Option<u16>,
    timeout: Option<Duration>,
    cancel: &AtomicBool,
) -> Result<PollingResponse> {
    let start = Instant::now();
    let deadline = timeout.map(|d| start + d);
    let sc = system_code.unwrap_or(WILDCARD);
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(Error::ScanCancelled);
        }
        if let Some(deadline) = deadline {
            if Instant::now() >= deadline {
                return Err(Error::ScanTimeout);
            }
        }
        let t = io_timeout(deadline, 80)?;
        let cmd = build_polling(sc, 0x01, 0x00);
        match device.transceive(&cmd, t) {
            Ok(resp) => {
                if let Ok(parsed) = parse_polling_response_for(&resp, None) {
                    return Ok(parsed);
                }
            }
            Err(Error::DeviceDisconnected) => return Err(Error::DeviceDisconnected),
            Err(_) => {}
        }
        sleep(POLL_RETRY);
    }
}

pub fn request_service_codes(
    device: &mut dyn ReaderDevice,
    idm: &[u8; 8],
    codes: &[u16],
) -> Result<Vec<(u16, u16)>> {
    let mut out = Vec::new();
    for chunk in codes.chunks(32) {
        let cmd = build_request_service(idm, chunk);
        let resp = device.transceive(&cmd, 120)?;
        let parsed = parse_request_service_response_for(&resp, Some(idm))?;
        for (code, version) in chunk.iter().zip(parsed.key_versions.iter()) {
            out.push((*code, *version));
        }
    }
    Ok(out)
}

pub fn search_services(
    device: &mut dyn ReaderDevice,
    idm: &[u8; 8],
    start_index: u16,
    max_nodes: u32,
) -> Result<Vec<ServiceNodeDto>> {
    let mut nodes = Vec::new();
    let mut index = start_index;
    let max = max_nodes.max(1);
    while nodes.len() < max as usize {
        let cmd = build_search_service_code(idm, index);
        let resp = device.transceive(&cmd, 120)?;
        match parse_search_service_code_response_for(&resp, Some(idm))? {
            None => break,
            Some(node) => {
                let kind = match node.kind {
                    SearchNodeKind::Area => "area",
                    SearchNodeKind::Service => "service",
                };
                nodes.push(ServiceNodeDto {
                    index,
                    code: node.code,
                    kind: kind.into(),
                });
                index = index.saturating_add(1);
            }
        }
    }
    Ok(nodes)
}

pub fn select_system(device: &mut dyn ReaderDevice, system_code: u16) -> Result<PollingResponse> {
    poll_system(device, system_code).ok_or_else(|| {
        Error::CommunicationError(format!("System 0x{system_code:04X} did not respond"))
    })
}
