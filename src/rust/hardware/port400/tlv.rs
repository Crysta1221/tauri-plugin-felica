//! TLV parsing for Port-400 PC/SC responses (from felica-rs).

use crate::error::{Error, Result};

pub(super) const STATUS_TLV_TAG: u8 = 0xC0;
const DEVICE_STATE_TLV_TAG: u8 = 0x80;
pub(super) const SWITCH_PROTOCOL_METADATA_TAG: u8 = 0x8F;
pub(super) const EXTENDED_TAG_PREFIX: u8 = 0x5F;
const ATR_TLV_TAG: u8 = 0x51;
const RESPONSE_BIT_FRAMING_TAG: u8 = 0x92;
const RESPONSE_STATUS_TAG: u8 = 0x96;
const RESPONSE_DATA_TAG: u8 = 0x97;
pub(super) const VENDOR_SPECIFIC_TAG: u8 = 0xFF;
const VENDOR_TAG_RESPONSE: u8 = 0x6D;

#[derive(Default)]
pub(super) struct TransparentExchangeResult {
    pub payload: Vec<u8>,
}

pub(super) fn push_extended_tlv(buf: &mut Vec<u8>, tag: u8, value: &[u8]) {
    buf.push(tag);
    buf.push(0x82);
    buf.push(((value.len() >> 8) & 0xFF) as u8);
    buf.push((value.len() & 0xFF) as u8);
    buf.extend_from_slice(value);
}

pub(super) fn verify_status(data: &[u8]) -> Result<()> {
    if data.len() < 2 {
        return Err(Error::CommunicationError("short CCID status".into()));
    }
    let sw1 = data[data.len() - 2];
    let sw2 = data[data.len() - 1];
    if sw1 == 0x90 && sw2 == 0x00 {
        return Ok(());
    }
    Err(Error::CommunicationError(format!(
        "CCID status {sw1:02X}{sw2:02X}"
    )))
}

fn take_tlv_value<'a>(data: &'a [u8], idx: &mut usize, context: &str) -> Result<&'a [u8]> {
    let len_index = *idx;
    let len = *data
        .get(len_index)
        .ok_or_else(|| Error::CommunicationError(format!("{context}: TLV length missing")))?
        as usize;
    if len_index + len >= data.len() {
        return Err(Error::CommunicationError(format!(
            "{context}: TLV length out of range"
        )));
    }
    let start = len_index + 1;
    *idx = start + len;
    Ok(&data[start..start + len])
}

fn take_status_tlv(data: &[u8], idx: &mut usize, context: &str) -> Result<()> {
    let value = take_tlv_value(data, idx, context)?;
    if value.len() != 3 {
        return Err(Error::CommunicationError(format!(
            "{context}: malformed status TLV"
        )));
    }
    if value != [0x00, 0x90, 0x00] {
        return Err(status_error(value));
    }
    Ok(())
}

fn skip_vendor_tlv(data: &[u8], idx: &mut usize, context: &str) -> Result<bool> {
    let Some(&subtag) = data.get(*idx) else {
        return Ok(false);
    };
    *idx += 1;
    if subtag != VENDOR_TAG_RESPONSE {
        return Ok(false);
    }
    let value = take_tlv_value(data, idx, context)?;
    if value.len() != 3 && value.len() != 6 {
        return Err(Error::CommunicationError(format!(
            "{context}: malformed vendor TLV"
        )));
    }
    Ok(true)
}

pub(super) fn parse_manage_session_response(data: &[u8]) -> Result<()> {
    const CONTEXT: &str = "manageSession";
    let mut idx = 0;
    while idx + 1 < data.len() {
        let tag = data[idx];
        idx += 1;
        match tag {
            STATUS_TLV_TAG => take_status_tlv(data, &mut idx, CONTEXT)?,
            DEVICE_STATE_TLV_TAG => {
                let value = take_tlv_value(data, &mut idx, CONTEXT)?;
                if value.len() != 3 {
                    return Err(Error::CommunicationError(format!(
                        "{CONTEXT}: malformed device state TLV"
                    )));
                }
            }
            VENDOR_SPECIFIC_TAG => {
                if !skip_vendor_tlv(data, &mut idx, CONTEXT)? {
                    break;
                }
            }
            _ => break,
        }
    }
    Ok(())
}

pub(super) fn parse_switch_protocol_response(data: &[u8]) -> Result<()> {
    const CONTEXT: &str = "switchProtocol";
    let mut idx = 0;
    while idx + 1 < data.len() {
        let tag = data[idx];
        idx += 1;
        match tag {
            STATUS_TLV_TAG => take_status_tlv(data, &mut idx, CONTEXT)?,
            SWITCH_PROTOCOL_METADATA_TAG => {
                let value = take_tlv_value(data, &mut idx, CONTEXT)?;
                if value.len() != 1 && value.len() != 3 {
                    return Err(Error::CommunicationError(format!(
                        "{CONTEXT}: malformed protocol TLV"
                    )));
                }
            }
            EXTENDED_TAG_PREFIX => {
                let subtag = data.get(idx).copied();
                idx += 1;
                if subtag != Some(ATR_TLV_TAG) {
                    return Err(Error::CommunicationError(format!("{CONTEXT}: ATR error")));
                }
                take_tlv_value(data, &mut idx, CONTEXT)?;
            }
            VENDOR_SPECIFIC_TAG => {
                if !skip_vendor_tlv(data, &mut idx, CONTEXT)? {
                    break;
                }
            }
            _ => break,
        }
    }
    Ok(())
}

pub(super) fn parse_transparent_response(data: &[u8]) -> Result<TransparentExchangeResult> {
    const CONTEXT: &str = "transparentExchange";
    let mut idx = 0;
    let mut result = TransparentExchangeResult::default();
    while idx + 1 < data.len() {
        let tag = data[idx];
        idx += 1;
        match tag {
            STATUS_TLV_TAG => take_status_tlv(data, &mut idx, CONTEXT)?,
            RESPONSE_BIT_FRAMING_TAG => {
                take_tlv_value(data, &mut idx, CONTEXT)?;
            }
            RESPONSE_STATUS_TAG => {
                let value = take_tlv_value(data, &mut idx, CONTEXT)?;
                if value.len() != 2 {
                    return Err(Error::CommunicationError(format!(
                        "{CONTEXT}: Response Status error"
                    )));
                }
            }
            RESPONSE_DATA_TAG => {
                let (len, consumed) = parse_length(&data[idx..]).map_err(|_| {
                    Error::CommunicationError(format!("{CONTEXT}: Response Data error"))
                })?;
                idx += consumed;
                if idx + len > data.len() {
                    return Err(Error::CommunicationError(format!(
                        "{CONTEXT}: Response Data out of range"
                    )));
                }
                result.payload.extend_from_slice(&data[idx..idx + len]);
                idx += len;
            }
            VENDOR_SPECIFIC_TAG => {
                if !skip_vendor_tlv(data, &mut idx, CONTEXT)? {
                    break;
                }
            }
            _ => break,
        }
    }
    Ok(result)
}

fn parse_length(data: &[u8]) -> Result<(usize, usize)> {
    if data.is_empty() {
        return Err(Error::CommunicationError("missing length field".into()));
    }
    let first = data[0];
    if first < 0x80 {
        return Ok((first as usize, 1));
    }
    let count = match first {
        0x81 => 1,
        0x82 => 2,
        0x83 => 3,
        0x84 => 4,
        _ => {
            return Err(Error::CommunicationError(
                "unsupported TLV length encoding".into(),
            ));
        }
    };
    if data.len() < 1 + count {
        return Err(Error::CommunicationError(
            "incomplete TLV length field".into(),
        ));
    }
    let mut len = 0usize;
    for &byte in &data[1..=count] {
        len = (len << 8) | byte as usize;
    }
    Ok((len, 1 + count))
}

fn status_error(value: &[u8]) -> Error {
    let sw1 = value.get(1).copied().unwrap_or_default();
    let sw2 = value.get(2).copied().unwrap_or_default();
    let text = format!(
        "status {:02X}{:02X}{:02X}",
        value.first().copied().unwrap_or_default(),
        sw1,
        sw2
    );
    match (sw1, sw2) {
        (0x64, 0x00 | 0x01) => Error::CommunicationError(format!(
            "{text} (no response packet received)"
        )),
        (0x63, 0x01) => Error::CommunicationError(format!("{text} (invalid status)")),
        (0x69, 0x8A) => Error::DeviceAccessDenied(format!("{text} (failed to get access authority)")),
        _ => Error::CommunicationError(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_transparent_response_reads_payload() {
        let parsed = parse_transparent_response(&[
            0xC0, 0x03, 0x00, 0x90, 0x00, 0x97, 0x03, 0xAA, 0xBB, 0xCC,
        ])
        .expect("payload TLV");
        assert_eq!(parsed.payload, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn verify_status_accepts_9000() {
        verify_status(&[0x90, 0x00]).unwrap();
        assert!(verify_status(&[0x6A, 0x82]).is_err());
    }
}
