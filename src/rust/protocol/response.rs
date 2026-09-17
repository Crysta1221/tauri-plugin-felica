use crate::error::{Error, Result};

/// Locate a length-prefixed or bare FeliCa PDU by length + response code.
/// Does not scan every offset for a matching response code.
fn felica_pdu(bytes: &[u8], response_code: u8, min_len: usize) -> Result<&[u8]> {
    if bytes.is_empty() {
        return Err(Error::ProtocolError(format!(
            "FeliCa response 0x{response_code:02X} is empty"
        )));
    }

    if bytes[0] == response_code && bytes.len() >= min_len.saturating_sub(1).max(1) {
        return Ok(bytes);
    }

    if bytes.len() >= 2 && bytes[1] == response_code {
        let declared = bytes[0] as usize;
        if declared >= min_len && declared <= bytes.len() {
            return Ok(&bytes[..declared]);
        }
        if bytes.len() >= min_len {
            return Ok(bytes);
        }
    }

    // One leading wrapper byte used by some Port-400 transports.
    if bytes.len() >= 3 && bytes[2] == response_code {
        return felica_pdu(&bytes[1..], response_code, min_len);
    }

    Err(Error::ProtocolError(format!(
        "FeliCa response 0x{response_code:02X} not found (got {} bytes)",
        bytes.len()
    )))
}

fn payload_offset(frame: &[u8], response_code: u8) -> usize {
    if frame.first() == Some(&response_code) {
        0
    } else {
        1
    }
}

fn read_idm(frame: &[u8], response_code: u8) -> Result<[u8; 8]> {
    let start = payload_offset(frame, response_code) + 1;
    if frame.len() < start + 8 {
        return Err(Error::ProtocolError("Response missing IDm".into()));
    }
    let mut idm = [0u8; 8];
    idm.copy_from_slice(&frame[start..start + 8]);
    Ok(idm)
}

fn verify_idm(idm: &[u8; 8], expected: Option<&[u8; 8]>) -> Result<()> {
    if let Some(expected) = expected {
        if idm != expected {
            return Err(Error::ProtocolError(format!(
                "IDm mismatch: got {}, expected {}",
                hex::encode_upper(idm),
                hex::encode_upper(expected)
            )));
        }
    }
    Ok(())
}

/// Parsed Polling response (Response code 0x01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollingResponse {
    pub idm: [u8; 8],
    pub pmm: [u8; 8],
    pub system_code: Option<u16>,
}

/// Parses a FeliCa Polling response. Extra two bytes after PMm are the system code when RC=0x01.
pub fn parse_polling_response(resp: &[u8]) -> Result<PollingResponse> {
    parse_polling_response_for(resp, None)
}

pub fn parse_polling_response_for(
    resp: &[u8],
    expected_idm: Option<&[u8; 8]>,
) -> Result<PollingResponse> {
    let frame = felica_pdu(resp, 0x01, 17)?;
    let start = payload_offset(frame, 0x01) + 1;
    if frame.len() < start + 16 {
        return Err(Error::ProtocolError("Polling response too short".into()));
    }

    let mut idm = [0u8; 8];
    let mut pmm = [0u8; 8];
    idm.copy_from_slice(&frame[start..start + 8]);
    pmm.copy_from_slice(&frame[start + 8..start + 16]);
    verify_idm(&idm, expected_idm)?;

    let system_code = if frame.len() >= start + 18 {
        Some(u16::from_be_bytes([
            frame[start + 16],
            frame[start + 17],
        ]))
    } else {
        None
    };

    Ok(PollingResponse {
        idm,
        pmm,
        system_code,
    })
}

/// Parses a FeliCa Request System Code response (Response code 0x0D).
/// Does not invent `0xFFFF` when the card lists nothing.
pub fn parse_system_codes_response(resp: &[u8]) -> Result<Vec<u16>> {
    parse_system_codes_response_for(resp, None)
}

pub fn parse_system_codes_response_for(
    resp: &[u8],
    expected_idm: Option<&[u8; 8]>,
) -> Result<Vec<u16>> {
    let frame = felica_pdu(resp, 0x0D, 11)?;
    let idm = read_idm(frame, 0x0D)?;
    verify_idm(&idm, expected_idm)?;

    let start = payload_offset(frame, 0x0D) + 9;
    if frame.len() < start + 1 {
        return Err(Error::ProtocolError(
            "Request System Code response too short".into(),
        ));
    }
    let count = frame[start] as usize;
    let mut system_codes = Vec::with_capacity(count);
    let mut offset = start + 1;
    for _ in 0..count {
        if offset + 2 > frame.len() {
            break;
        }
        let sc = u16::from_be_bytes([frame[offset], frame[offset + 1]]);
        system_codes.push(sc);
        offset += 2;
    }
    Ok(system_codes)
}

/// Parsed Request Service response (Response code 0x03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestServiceResponse {
    pub idm: [u8; 8],
    pub key_versions: Vec<u16>,
}

pub fn parse_request_service_response(resp: &[u8]) -> Result<RequestServiceResponse> {
    parse_request_service_response_for(resp, None)
}

pub fn parse_request_service_response_for(
    resp: &[u8],
    expected_idm: Option<&[u8; 8]>,
) -> Result<RequestServiceResponse> {
    let frame = felica_pdu(resp, 0x03, 11)?;
    let idm = read_idm(frame, 0x03)?;
    verify_idm(&idm, expected_idm)?;
    let start = payload_offset(frame, 0x03) + 9;
    if frame.len() < start + 1 {
        return Err(Error::ProtocolError(
            "Request Service response too short".into(),
        ));
    }
    let count = frame[start] as usize;
    let mut key_versions = Vec::with_capacity(count);
    let mut offset = start + 1;
    for _ in 0..count {
        if offset + 2 > frame.len() {
            break;
        }
        key_versions.push(u16::from_le_bytes([frame[offset], frame[offset + 1]]));
        offset += 2;
    }
    Ok(RequestServiceResponse { idm, key_versions })
}

/// Parsed Read Without Encryption response (Response code 0x07).
/// Block payloads are present only when SF1 is 0x00.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadWithoutEncryptionResponse {
    pub idm: [u8; 8],
    pub status_flag1: u8,
    pub status_flag2: u8,
    pub blocks: Vec<[u8; 16]>,
}

pub fn parse_read_without_encryption_response(
    resp: &[u8],
) -> Result<ReadWithoutEncryptionResponse> {
    parse_read_without_encryption_response_for(resp, None)
}

pub fn parse_read_without_encryption_response_for(
    resp: &[u8],
    expected_idm: Option<&[u8; 8]>,
) -> Result<ReadWithoutEncryptionResponse> {
    let frame = felica_pdu(resp, 0x07, 12)?;
    let idm = read_idm(frame, 0x07)?;
    verify_idm(&idm, expected_idm)?;

    let status_at = payload_offset(frame, 0x07) + 9;
    if frame.len() < status_at + 2 {
        return Err(Error::ProtocolError("Read response too short".into()));
    }

    let status_flag1 = frame[status_at];
    let status_flag2 = frame[status_at + 1];
    if status_flag1 != 0x00 {
        return Ok(ReadWithoutEncryptionResponse {
            idm,
            status_flag1,
            status_flag2,
            blocks: Vec::new(),
        });
    }

    if frame.len() < status_at + 3 {
        return Err(Error::ProtocolError(
            "Read response missing block count".into(),
        ));
    }

    let num_blocks = frame[status_at + 2] as usize;
    let mut blocks = Vec::with_capacity(num_blocks);
    let mut offset = status_at + 3;
    for _ in 0..num_blocks {
        if offset + 16 > frame.len() {
            return Err(Error::ProtocolError(format!(
                "Read response block count mismatch: expected {num_blocks}, got {}",
                blocks.len()
            )));
        }
        let mut block = [0u8; 16];
        block.copy_from_slice(&frame[offset..offset + 16]);
        blocks.push(block);
        offset += 16;
    }

    if blocks.len() != num_blocks {
        return Err(Error::ProtocolError(
            "Read response block count mismatch".into(),
        ));
    }

    Ok(ReadWithoutEncryptionResponse {
        idm,
        status_flag1,
        status_flag2,
        blocks,
    })
}

/// Search Service Code node (Response code 0x0B). `None` means no more nodes (`0xFFFF`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchServiceNode {
    pub code: u16,
    pub kind: SearchNodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchNodeKind {
    Area,
    Service,
}

pub fn parse_search_service_code_response(resp: &[u8]) -> Result<Option<SearchServiceNode>> {
    parse_search_service_code_response_for(resp, None)
}

pub fn parse_search_service_code_response_for(
    resp: &[u8],
    expected_idm: Option<&[u8; 8]>,
) -> Result<Option<SearchServiceNode>> {
    let frame = felica_pdu(resp, 0x0B, 12)?;
    let idm = read_idm(frame, 0x0B)?;
    verify_idm(&idm, expected_idm)?;
    let start = payload_offset(frame, 0x0B) + 9;
    if frame.len() < start + 2 {
        return Err(Error::ProtocolError(
            "Search Service Code response too short".into(),
        ));
    }

    let first = u16::from_le_bytes([frame[start], frame[start + 1]]);
    if first == 0xFFFF {
        return Ok(None);
    }

    if frame.len() >= start + 4 {
        Ok(Some(SearchServiceNode {
            code: first,
            kind: SearchNodeKind::Area,
        }))
    } else {
        Ok(Some(SearchServiceNode {
            code: first,
            kind: SearchNodeKind::Service,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_polling_response() {
        let mut resp = vec![0x12, 0x01];
        let idm = [1, 2, 3, 4, 5, 6, 7, 8];
        let pmm = [9, 10, 11, 12, 13, 14, 15, 16];
        resp.extend_from_slice(&idm);
        resp.extend_from_slice(&pmm);

        let parsed = parse_polling_response(&resp).unwrap();
        assert_eq!(parsed.idm, idm);
        assert_eq!(parsed.pmm, pmm);
        assert_eq!(parsed.system_code, None);
    }

    #[test]
    fn test_parse_polling_response_with_system_code() {
        let mut resp = vec![0x14, 0x01];
        let idm = [1, 2, 3, 4, 5, 6, 7, 8];
        let pmm = [9, 10, 11, 12, 13, 14, 15, 16];
        resp.extend_from_slice(&idm);
        resp.extend_from_slice(&pmm);
        resp.extend_from_slice(&[0x00, 0x03]);
        let parsed = parse_polling_response(&resp).unwrap();
        assert_eq!(parsed.system_code, Some(0x0003));
    }

    #[test]
    fn test_parse_polling_response_without_length_prefix() {
        let mut resp = vec![0x01];
        let idm = [1, 2, 3, 4, 5, 6, 7, 8];
        let pmm = [9, 10, 11, 12, 13, 14, 15, 16];
        resp.extend_from_slice(&idm);
        resp.extend_from_slice(&pmm);
        let parsed = parse_polling_response(&resp).unwrap();
        assert_eq!(parsed.idm, idm);
        assert_eq!(parsed.pmm, pmm);
    }

    #[test]
    fn test_parse_system_codes_response() {
        let resp = vec![
            0x0F, 0x0D, 1, 2, 3, 4, 5, 6, 7, 8, 0x02, 0x00, 0x03, 0xFE, 0x00,
        ];
        let codes = parse_system_codes_response(&resp).unwrap();
        assert_eq!(codes, vec![0x0003, 0xFE00]);
    }

    #[test]
    fn test_parse_system_codes_verifies_idm() {
        let resp = vec![
            0x0F, 0x0D, 1, 2, 3, 4, 5, 6, 7, 8, 0x01, 0x00, 0x03,
        ];
        let expected = [9, 9, 9, 9, 9, 9, 9, 9];
        assert!(parse_system_codes_response_for(&resp, Some(&expected)).is_err());
    }

    #[test]
    fn test_parse_request_service() {
        let resp = vec![
            0x11, 0x03, 1, 2, 3, 4, 5, 6, 7, 8, 0x02, 0x01, 0x00, 0xFF, 0xFF,
        ];
        let parsed = parse_request_service_response(&resp).unwrap();
        assert_eq!(parsed.key_versions, vec![0x0001, 0xFFFF]);
    }

    #[test]
    fn test_parse_read_without_encryption_response() {
        let mut resp = vec![0x21, 0x07, 1, 2, 3, 4, 5, 6, 7, 8, 0x00, 0x00, 0x01];
        let block = [0xAA; 16];
        resp.extend_from_slice(&block);

        let parsed = parse_read_without_encryption_response(&resp).unwrap();
        assert_eq!(parsed.blocks.len(), 1);
        assert_eq!(parsed.blocks[0], block);
        assert_eq!(parsed.status_flag1, 0x00);
    }

    #[test]
    fn test_parse_read_status_flag_has_no_blocks() {
        let resp = vec![0x0C, 0x07, 1, 2, 3, 4, 5, 6, 7, 8, 0xA8, 0x01];
        let parsed = parse_read_without_encryption_response(&resp).unwrap();
        assert_eq!(parsed.status_flag1, 0xA8);
        assert!(parsed.blocks.is_empty());
    }

    #[test]
    fn test_parse_read_verifies_idm() {
        let mut resp = vec![0x21, 0x07, 1, 2, 3, 4, 5, 6, 7, 8, 0x00, 0x00, 0x01];
        resp.extend_from_slice(&[0xAA; 16]);
        let expected = [0; 8];
        assert!(parse_read_without_encryption_response_for(&resp, Some(&expected)).is_err());
    }
}
