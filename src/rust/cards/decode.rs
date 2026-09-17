use crate::models::{LabeledCodeDto, StationRefDto};

pub fn le16(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes.get(offset..offset + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

pub fn be16(bytes: &[u8], offset: usize) -> Option<u16> {
    bytes.get(offset..offset + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}

pub fn be24(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 3)
        .map(|s| ((s[0] as u32) << 16) | ((s[1] as u32) << 8) | s[2] as u32)
}

pub fn le32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn be32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// Purse balance: first 4 bytes little-endian.
pub fn purse_balance(bytes: &[u8]) -> Option<u32> {
    le32(bytes, 0)
}

pub fn labeled(code: u32) -> LabeledCodeDto {
    LabeledCodeDto {
        code,
        label: format!("0x{code:02X}"),
    }
}

pub fn station_ref(line: u8, station: u8, region: Option<u8>) -> StationRefDto {
    StationRefDto {
        line,
        station,
        region,
        label: format!("0x{line:02X}/0x{station:02X}"),
        candidates: None,
    }
}

pub fn pad2(value: u32) -> String {
    format!("{value:02}")
}

pub fn format_date(year: u32, month: u32, day: u32) -> Option<String> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || year < 2000 || year > 2099 {
        return None;
    }
    Some(format!("{year}-{}-{}", pad2(month), pad2(day)))
}

pub fn format_time(hour: u32, minute: u32, second: Option<u32>) -> Option<String> {
    if hour > 23 || minute > 59 {
        return None;
    }
    match second {
        Some(s) if s <= 59 => Some(format!("{}:{}:{}", pad2(hour), pad2(minute), pad2(s))),
        Some(_) => None,
        None => Some(format!("{}:{}", pad2(hour), pad2(minute))),
    }
}

/// Cybernetics packed date: 7-bit year + 4-bit month + 5-bit day, year origin 2000.
pub fn cybernetics_date(packed: u16) -> Option<String> {
    let year = 2000 + u32::from(packed >> 9);
    let month = u32::from((packed >> 5) & 0x0F);
    let day = u32::from(packed & 0x1F);
    format_date(year, month, day)
}

pub fn bcd_nibble(byte: u8, high: bool) -> Option<u8> {
    let nibble = if high { byte >> 4 } else { byte & 0x0F };
    if nibble <= 9 {
        Some(nibble)
    } else {
        None
    }
}

pub fn bcd_string(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        let high = byte >> 4;
        let low = byte & 0x0F;
        if high <= 9 {
            out.push(char::from(b'0' + high));
        } else {
            break;
        }
        if low <= 9 {
            out.push(char::from(b'0' + low));
        } else {
            break;
        }
    }
    out
}

pub fn bcd_time(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 2 {
        return None;
    }
    let hour = bcd_nibble(bytes[0], true)? * 10 + bcd_nibble(bytes[0], false)?;
    let minute = bcd_nibble(bytes[1], true)? * 10 + bcd_nibble(bytes[1], false)?;
    format_time(u32::from(hour), u32::from(minute), None)
}

pub fn hex_upper(bytes: &[u8]) -> String {
    hex::encode_upper(bytes)
}

pub fn bits_be(bytes: &[u8], start_bit: usize, width: usize) -> Option<u32> {
    if width == 0 || width > 32 {
        return None;
    }
    let mut value = 0u32;
    for i in 0..width {
        let bit_index = start_bit + i;
        let byte_index = bit_index / 8;
        let shift = 7 - (bit_index % 8);
        let bit = *bytes.get(byte_index)? as u32;
        value = (value << 1) | ((bit >> shift) & 1);
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purse_is_little_endian() {
        assert_eq!(purse_balance(&[0x78, 0x56, 0x34, 0x12]), Some(0x12345678));
    }

    #[test]
    fn be24_reads_three_bytes() {
        assert_eq!(be24(&[0x01, 0x02, 0x03], 0), Some(0x010203));
    }
}
