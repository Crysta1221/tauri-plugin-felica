use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::cards::profile::{all_profiles, CardProfile, Detection, ParsedProduct, ServiceRead};
use crate::cards::rf::{
    io_timeout, poll_system_timed, read_service, request_services, request_system_codes, POLL_RETRY,
    WILDCARD,
};
use crate::error::{Error, Result};
use crate::hardware::ReaderDevice;
use crate::models::{ReadBlocksResultDto, ScanCardResponse};
use crate::protocol::command::build_polling;
use crate::protocol::response::{parse_polling_response_for, PollingResponse};

pub struct ScanParams<'a> {
    pub timeout: Option<Duration>,
    pub targets: Option<Vec<String>>,
    pub require: bool,
    pub detail: bool,
    pub cancel: &'a AtomicBool,
}

#[derive(Clone)]
struct SystemIdentity {
    idm: [u8; 8],
}

struct MatchHit {
    profile: &'static dyn CardProfile,
    system_code: u16,
    found: Vec<u16>,
    idm: [u8; 8],
}

pub fn scan_card(device: &mut dyn ReaderDevice, params: ScanParams<'_>) -> Result<ScanCardResponse> {
    let start = Instant::now();
    let detect_deadline = params.timeout.map(|d| start + d);

    loop {
        if params.cancel.load(Ordering::SeqCst) {
            return Err(Error::ScanCancelled);
        }
        if let Some(deadline) = detect_deadline {
            if Instant::now() >= deadline {
                return Err(Error::ScanTimeout);
            }
        }

        match try_scan_once(device, &params, detect_deadline) {
            Ok(Some(result)) => return Ok(result),
            Ok(None) => sleep(POLL_RETRY),
            Err(Error::DeviceDisconnected) => return Err(Error::DeviceDisconnected),
            Err(Error::ScanCancelled) => return Err(Error::ScanCancelled),
            Err(Error::ScanTimeout) => return Err(Error::ScanTimeout),
            Err(err @ Error::UnsupportedCard { .. })
            | Err(err @ Error::CardTypeMismatch { .. })
            | Err(err @ Error::ProtocolError(_)) => return Err(err),
            Err(_) => sleep(POLL_RETRY),
        }
    }
}

fn try_scan_once(
    device: &mut dyn ReaderDevice,
    params: &ScanParams<'_>,
    detect_deadline: Option<Instant>,
) -> Result<Option<ScanCardResponse>> {
    let timeout = io_timeout(detect_deadline, 80)?;
    let poll_cmd = build_polling(WILDCARD, 0x01, 0x00);
    let poll = match device.transceive(&poll_cmd, timeout) {
        Ok(resp) => match parse_polling_response_for(&resp, None) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        },
        Err(Error::DeviceDisconnected) => return Err(Error::DeviceDisconnected),
        Err(_) => return Ok(None),
    };

    // Card present: dump is not bound by timeoutMs.
    complete_scan(device, params, poll)
}

fn complete_scan(
    device: &mut dyn ReaderDevice,
    params: &ScanParams<'_>,
    wildcard: PollingResponse,
) -> Result<Option<ScanCardResponse>> {
    let mut identities: HashMap<u16, SystemIdentity> = HashMap::new();
    let mut poll_failed: HashSet<u16> = HashSet::new();
    identities.insert(0, SystemIdentity { idm: wildcard.idm });
    if let Some(sc) = wildcard.system_code {
        identities.insert(sc, SystemIdentity { idm: wildcard.idm });
    }

    let mut system_codes = match request_system_codes(device, &wildcard.idm) {
        Ok(codes) if !codes.is_empty() => codes,
        _ => wildcard.system_code.into_iter().collect(),
    };
    let search_systems = ordered_systems(&system_codes, wildcard.system_code);

    let mut matches = Vec::new();
    for profile in all_profiles() {
        if !profile_wanted(profile.card_type(), params.targets.as_deref()) {
            continue;
        }
        match profile.detection() {
            Detection::Polling(codes) => {
                for &sc in codes {
                    let known = identities.contains_key(&sc) || system_codes.contains(&sc);
                    // Lite / QUICPay are dedicated chips. Skip speculative polls once
                    // another product is already on this card.
                    if !known && !matches.is_empty() {
                        continue;
                    }
                    let timeout_ms = if known { 80 } else { 40 };
                    let Some(id) =
                        cached_identity(device, &mut identities, &mut poll_failed, sc, timeout_ms)
                    else {
                        continue;
                    };
                    if !system_codes.contains(&sc) {
                        system_codes.push(sc);
                    }
                    let found =
                        request_services(device, &id.idm, profile.candidate_services()).unwrap_or_default();
                    matches.push(MatchHit {
                        profile: *profile,
                        system_code: sc,
                        found,
                        idm: id.idm,
                    });
                }
            }
            Detection::Service { systems, any_of } => {
                let systems: Vec<u16> = match systems {
                    Some(list) => list
                        .iter()
                        .copied()
                        .filter(|sc| identities.contains_key(sc) || system_codes.contains(sc))
                        .collect(),
                    None => search_systems.clone(),
                };
                for sc in systems {
                    let Some(id) =
                        cached_identity(device, &mut identities, &mut poll_failed, sc, 80)
                    else {
                        continue;
                    };
                    let Ok(found) = request_services(device, &id.idm, profile.candidate_services())
                    else {
                        continue;
                    };
                    if any_of.iter().any(|svc| found.contains(svc)) {
                        matches.push(MatchHit {
                            profile: *profile,
                            system_code: sc,
                            found,
                            idm: id.idm,
                        });
                        break;
                    }
                }
            }
        }
    }

    let detected: Vec<String> = unique_types(&matches);
    if detected.is_empty() {
        return Err(Error::unsupported_card(
            "No known FeliCa product on this card",
            system_codes,
        ));
    }

    if let Some(targets) = &params.targets {
        let hit = detected.iter().any(|t| targets.iter().any(|x| x == t));
        if !hit {
            if params.require {
                return Ok(None);
            }
            return Err(Error::CardTypeMismatch {
                system_codes,
                detected,
            });
        }
    }

    let to_read: Vec<&MatchHit> = matches
        .iter()
        .filter(|m| match &params.targets {
            Some(targets) => targets.iter().any(|t| t == m.profile.card_type()),
            None => true,
        })
        .collect();

    let mut plan: Vec<ServiceRead> = Vec::new();
    for hit in &to_read {
        for item in hit.profile.dump_plan(hit.system_code, &hit.found, params.detail) {
            if !plan.iter().any(|p| {
                p.system_code == item.system_code && p.service_code == item.service_code
            }) {
                plan.push(item);
            }
        }
    }

    let mut collected = ReadBlocksResultDto {
        blocks: Vec::new(),
        errors: Vec::new(),
    };
    for item in &plan {
        let idm = cached_identity(
            device,
            &mut identities,
            &mut poll_failed,
            item.system_code,
            80,
        )
        .map(|s| s.idm)
        .unwrap_or(wildcard.idm);
        read_service(device, item, &idm, &mut collected);
    }

    // Reads already check IDm. Extra polling after a dump is slow and flaky;
    // only confirm presence when nothing was read (e.g. QUICPay).
    if collected.blocks.is_empty() {
        verify_same_card(device, &wildcard.idm, plan.last().map(|item| item.system_code))?;
    }

    let mut response = ScanCardResponse {
        idm: hex::encode_upper(wildcard.idm),
        pmm: hex::encode_upper(wildcard.pmm),
        system_codes,
        transit: None,
        waon: None,
        edy: None,
        nanaco: None,
        quicpay: None,
        lite: None,
        blocks: collected.blocks.clone(),
        errors: collected.errors,
    };

    let mut parsed_any = false;
    for hit in to_read {
        let idm_hex = hex::encode_upper(hit.idm);
        match hit.profile.parse(hit.system_code, &idm_hex, &collected.blocks) {
            Ok(ParsedProduct::Transit(v)) => {
                response.transit = Some(v);
                parsed_any = true;
            }
            Ok(ParsedProduct::Waon(v)) => {
                response.waon = Some(v);
                parsed_any = true;
            }
            Ok(ParsedProduct::Edy(v)) => {
                response.edy = Some(v);
                parsed_any = true;
            }
            Ok(ParsedProduct::Nanaco(v)) => {
                response.nanaco = Some(v);
                parsed_any = true;
            }
            Ok(ParsedProduct::Quicpay(v)) => {
                response.quicpay = Some(v);
                parsed_any = true;
            }
            Ok(ParsedProduct::Lite(v)) => {
                response.lite = Some(v);
                parsed_any = true;
            }
            Err(_) => {}
        }
    }

    if !parsed_any {
        return Err(Error::unsupported_card(
            "Known services did not parse",
            response.system_codes,
        ));
    }

    Ok(Some(response))
}

/// Confirm the same card is still in the field when the dump produced no blocks.
fn verify_same_card(
    device: &mut dyn ReaderDevice,
    expected_idm: &[u8; 8],
    last_system: Option<u16>,
) -> Result<()> {
    let mut systems = Vec::new();
    if let Some(sc) = last_system {
        if sc != 0 && sc != WILDCARD {
            systems.push(sc);
        }
    }
    systems.push(WILDCARD);

    for &sc in &systems {
        if let Some(poll) = poll_system_timed(device, sc, 80) {
            if poll.idm != *expected_idm {
                return Err(Error::ProtocolError(
                    "Card IDm changed during dump; another card may have been presented".into(),
                ));
            }
            return Ok(());
        }
    }

    Ok(())
}

fn cached_identity(
    device: &mut dyn ReaderDevice,
    identities: &mut HashMap<u16, SystemIdentity>,
    failed: &mut HashSet<u16>,
    system_code: u16,
    timeout_ms: u16,
) -> Option<SystemIdentity> {
    if let Some(id) = identities.get(&system_code) {
        return Some(id.clone());
    }
    if failed.contains(&system_code) {
        return None;
    }
    match poll_system_timed(device, system_code, timeout_ms) {
        Some(p) => {
            let id = SystemIdentity { idm: p.idm };
            identities.insert(system_code, id.clone());
            Some(id)
        }
        None => {
            failed.insert(system_code);
            None
        }
    }
}

fn ordered_systems(system_codes: &[u16], wildcard_sc: Option<u16>) -> Vec<u16> {
    let mut out = Vec::new();
    if let Some(sc) = wildcard_sc {
        if system_codes.contains(&sc) || system_codes.is_empty() {
            out.push(sc);
        }
    }
    for &sc in system_codes {
        if !out.contains(&sc) {
            out.push(sc);
        }
    }
    out
}

fn profile_wanted(card_type: &str, targets: Option<&[String]>) -> bool {
    match targets {
        Some(list) if !list.is_empty() => list.iter().any(|t| t == card_type),
        _ => true,
    }
}

fn unique_types(matches: &[MatchHit]) -> Vec<String> {
    let mut out = Vec::new();
    for hit in matches {
        let name = hit.profile.card_type().to_string();
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}
