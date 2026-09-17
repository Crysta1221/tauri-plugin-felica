/// Builds a FeliCa Polling command packet (Command code 0x00).
/// `request_code`: 0x00 = IDm+PMm, 0x01 = IDm+PMm+system code, 0x02 = communication performance.
pub fn build_polling(system_code: u16, request_code: u8, time_slot: u8) -> Vec<u8> {
    let sc_bytes = system_code.to_be_bytes();
    vec![
        0x06, // Length
        0x00, // Command code: Polling
        sc_bytes[0],
        sc_bytes[1],
        request_code,
        time_slot,
    ]
}

/// Builds a FeliCa Request System Code command packet (Command code 0x0C).
pub fn build_request_system_code(idm: &[u8; 8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(10);
    packet.push(0x0A); // Length: 10 bytes
    packet.push(0x0C); // Command code: Request System Code
    packet.extend_from_slice(idm);
    packet
}

/// Builds a FeliCa Request Service command packet (Command code 0x02).
/// Node codes (service or area) are encoded little-endian. 1–32 nodes.
pub fn build_request_service(idm: &[u8; 8], node_codes: &[u16]) -> Vec<u8> {
    let count = node_codes.len().min(32);
    let total_len = 1 + 1 + 8 + 1 + (count * 2);
    let mut packet = Vec::with_capacity(total_len);
    packet.push(total_len as u8);
    packet.push(0x02);
    packet.extend_from_slice(idm);
    packet.push(count as u8);
    for &code in node_codes.iter().take(count) {
        packet.extend_from_slice(&code.to_le_bytes());
    }
    packet
}

/// One Read Without Encryption block-list element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockListEntry {
    pub service_index: u8,
    pub block: u16,
}

/// Builds a FeliCa Read Without Encryption command (Command code 0x06).
/// Service codes are little-endian. Block numbers below 256 use a 2-byte list
/// element; 256 and above use a 3-byte element (block number little-endian).
pub fn build_read_without_encryption(
    idm: &[u8; 8],
    service_codes: &[u16],
    blocks: &[BlockListEntry],
) -> Vec<u8> {
    let num_services = service_codes.len().min(16);
    let num_blocks = blocks.len().min(16);
    let block_bytes: usize = blocks.iter().take(num_blocks).map(|entry| {
        if entry.block < 256 {
            2usize
        } else {
            3
        }
    }).sum();
    let total_len = 1 + 1 + 8 + 1 + (num_services * 2) + 1 + block_bytes;

    let mut packet = Vec::with_capacity(total_len);
    packet.push(total_len as u8);
    packet.push(0x06);
    packet.extend_from_slice(idm);
    packet.push(num_services as u8);

    for &sc in service_codes.iter().take(num_services) {
        packet.extend_from_slice(&sc.to_le_bytes());
    }

    packet.push(num_blocks as u8);
    for entry in blocks.iter().take(num_blocks) {
        let index = entry.service_index & 0x0F;
        if entry.block < 256 {
            packet.push(0x80 | index);
            packet.push(entry.block as u8);
        } else {
            packet.push(index);
            packet.extend_from_slice(&entry.block.to_le_bytes());
        }
    }

    packet
}

/// Convenience: one service, block numbers as `u16`.
pub fn build_read_without_encryption_single(
    idm: &[u8; 8],
    service_code: u16,
    blocks: &[u16],
) -> Vec<u8> {
    let entries: Vec<BlockListEntry> = blocks
        .iter()
        .map(|&block| BlockListEntry {
            service_index: 0,
            block,
        })
        .collect();
    build_read_without_encryption(idm, &[service_code], &entries)
}

/// Builds a FeliCa Search Service Code command (Command code 0x0A).
/// Packet layout follows felica-rs (UM excerpt does not disclose it).
pub fn build_search_service_code(idm: &[u8; 8], index: u16) -> Vec<u8> {
    let idx = index.to_le_bytes();
    let mut packet = Vec::with_capacity(12);
    packet.push(0x0C);
    packet.push(0x0A);
    packet.extend_from_slice(idm);
    packet.extend_from_slice(&idx);
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_polling() {
        let packet = build_polling(0x0003, 0x00, 0x00);
        assert_eq!(packet, vec![0x06, 0x00, 0x00, 0x03, 0x00, 0x00]);
    }

    #[test]
    fn test_build_polling_request_code_system() {
        let packet = build_polling(0xFFFF, 0x01, 0x00);
        assert_eq!(packet, vec![0x06, 0x00, 0xFF, 0xFF, 0x01, 0x00]);
    }

    #[test]
    fn test_build_request_system_code() {
        let idm = [0x01, 0x2E, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        let packet = build_request_system_code(&idm);
        assert_eq!(packet.len(), 10);
        assert_eq!(packet[0], 0x0A);
        assert_eq!(packet[1], 0x0C);
        assert_eq!(&packet[2..10], &idm);
    }

    #[test]
    fn test_build_request_service() {
        let idm = [1, 2, 3, 4, 5, 6, 7, 8];
        let packet = build_request_service(&idm, &[0x008B, 0x090F]);
        assert_eq!(packet[1], 0x02);
        assert_eq!(&packet[2..10], &idm);
        assert_eq!(packet[10], 2);
        assert_eq!(&packet[11..13], &[0x8B, 0x00]);
        assert_eq!(&packet[13..15], &[0x0F, 0x09]);
    }

    #[test]
    fn test_build_read_without_encryption() {
        let idm = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let packet = build_read_without_encryption_single(&idm, 0x090F, &[0, 1]);
        assert_eq!(packet[1], 0x06);
        assert_eq!(&packet[2..10], &idm);
        assert_eq!(packet[10], 1);
        assert_eq!(&packet[11..13], &[0x0F, 0x09]);
        assert_eq!(packet[13], 2);
        assert_eq!(&packet[14..18], &[0x80, 0x00, 0x80, 0x01]);
    }

    #[test]
    fn test_build_read_three_byte_block_and_service_index() {
        let idm = [1, 2, 3, 4, 5, 6, 7, 8];
        let packet = build_read_without_encryption(
            &idm,
            &[0x008B, 0x090F],
            &[
                BlockListEntry {
                    service_index: 1,
                    block: 300,
                },
            ],
        );
        assert_eq!(packet[10], 2);
        assert_eq!(&packet[11..15], &[0x8B, 0x00, 0x0F, 0x09]);
        assert_eq!(packet[15], 1);
        // 3-byte element: length bit 0, service index 1, block 300 LE
        assert_eq!(&packet[16..19], &[0x01, 0x2C, 0x01]);
    }
}
