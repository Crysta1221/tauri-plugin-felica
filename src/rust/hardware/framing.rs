/// Start-of-frame sequence for Sony PaSoRi readers.
pub const SOF: [u8; 3] = [0x00, 0x00, 0xFF];

/// ACK frame bytes.
pub const ACK_BYTES: [u8; 6] = [0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00];

/// Error frame bytes.
pub const ERROR_BYTES: [u8; 6] = [0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00];

/// Calculates the 2's complement checksum of a byte slice.
pub fn checksum(bytes: &[u8]) -> u8 {
    let sum: u16 = bytes.iter().map(|b| *b as u16).sum();
    ((256 - (sum % 256)) % 256) as u8
}

/// Builds an extended little-endian frame (used by Port-100 / RC-S380).
pub fn build_port100_frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u16;
    let len_bytes = len.to_le_bytes();
    let lcs = checksum(&len_bytes);
    let dcs = checksum(payload);

    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.extend_from_slice(&SOF);
    frame.extend_from_slice(&[0xFF, 0xFF]);
    frame.extend_from_slice(&len_bytes);
    frame.push(lcs);
    frame.extend_from_slice(payload);
    frame.push(dcs);
    frame.push(0x00); // Postamble
    frame
}

/// Builds a normal frame (1-byte length, used by RC-S956 / RC-S330 etc.).
pub fn build_normal_frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u8;
    let lcs = checksum(&[len]);
    let dcs = checksum(payload);

    let mut frame = Vec::with_capacity(payload.len() + 7);
    frame.extend_from_slice(&SOF);
    frame.push(len);
    frame.push(lcs);
    frame.extend_from_slice(payload);
    frame.push(dcs);
    frame.push(0x00);
    frame
}

/// Parses a frame from raw bytes, returning the payload if valid data frame.
pub fn parse_frame(data: &[u8]) -> Option<Vec<u8>> {
    if data == ACK_BYTES || data == ERROR_BYTES {
        return None;
    }

    // Find SOF
    let sof_idx = data.windows(3).position(|w| w == SOF)?;
    let slice = &data[sof_idx..];
    if slice.len() < 7 {
        return None;
    }

    // Check if extended frame
    if slice.len() >= 10 && slice[3] == 0xFF && slice[4] == 0xFF {
        let len = u16::from_le_bytes([slice[5], slice[6]]) as usize;
        let lcs = slice[7];
        if (slice[5] as u16 + slice[6] as u16 + lcs as u16) % 256 != 0 {
            return None;
        }
        if slice.len() < 8 + len + 2 {
            return None;
        }
        let payload = &slice[8..8 + len];
        let dcs = slice[8 + len];
        let sum: u16 = payload.iter().map(|b| *b as u16).sum();
        if (sum + dcs as u16) % 256 != 0 {
            return None;
        }
        return Some(payload.to_vec());
    }

    // Normal frame (single length byte)
    let len = slice[3] as usize;
    let lcs = slice[4];
    if (len as u16 + lcs as u16) % 256 != 0 {
        return None;
    }
    if slice.len() < 5 + len + 2 {
        return None;
    }
    let payload = &slice[5..5 + len];
    let dcs = slice[5 + len];
    let sum: u16 = payload.iter().map(|b| *b as u16).sum();
    if (sum + dcs as u16) % 256 != 0 {
        return None;
    }
    Some(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_calculation() {
        let data = [0x01, 0x02, 0x03];
        let cs = checksum(&data);
        let sum: u16 = data.iter().map(|b| *b as u16).sum::<u16>() + cs as u16;
        assert_eq!(sum % 256, 0);
    }

    #[test]
    fn test_normal_frame_roundtrip() {
        let payload = vec![0xD4, 0x32, 0x01, 0x00];
        let frame = build_normal_frame(&payload);
        let parsed = parse_frame(&frame);
        assert_eq!(parsed, Some(payload));
    }

    #[test]
    fn test_port100_frame_roundtrip() {
        let payload = vec![0xD4, 0x4A, 0x01, 0x01, 0x00, 0x03, 0x00, 0x00];
        let frame = build_port100_frame(&payload);
        let parsed = parse_frame(&frame);
        assert_eq!(parsed, Some(payload));
    }

    #[test]
    fn test_ack_and_error_bytes() {
        assert_eq!(parse_frame(&ACK_BYTES), None);
        assert_eq!(parse_frame(&ERROR_BYTES), None);
    }
}

