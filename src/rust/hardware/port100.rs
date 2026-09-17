use std::time::Duration;
use crate::hardware::framing::{ACK_BYTES, build_port100_frame, parse_frame};


use crate::hardware::usb::UsbTransport;
use crate::error::{Error, Result};
use crate::models::reader::ReaderInfoDto;
use super::ReaderDevice;

pub const PORT100_PIDS: &[u16] = &[0x06C1, 0x06C3];

/// Driver for Sony RC-S380 (Port-100).
pub struct Port100Device {
    transport: UsbTransport,
    info: ReaderInfoDto,
}

impl Port100Device {
    /// Attempts to open an RC-S380 device.
    pub fn open(pid: Option<u16>, location: Option<super::UsbLocation>) -> Result<Self> {
        super::open_with_pids(
            PORT100_PIDS,
            pid,
            location,
            Self::init_device,
            "RC-S380 reader not found",
        )
    }

    fn init_device(mut transport: UsbTransport, pid: u16) -> Result<Self> {
        // Initial handshake: send ACK and clear residual data
        let _ = transport.write(&ACK_BYTES, Duration::from_millis(100));
        let mut trash = [0u8; 64];
        while transport.read(&mut trash, Duration::from_millis(20)).is_ok() {}

        // Set command type 1 (Normal)
        let _ = Self::send_command(&mut transport, 0x2A, &[0x01]);

        // Query firmware version
        let fw = Self::send_command(&mut transport, 0x20, &[])?;
        let fw_str = if fw.len() >= 2 {
            format!("v{:x}.{:02x}", fw[1], fw[0])
        } else {
            "v1.0".to_string()
        };

        // Turn RF field ON
        Self::send_command(&mut transport, 0x06, &[0x01])?;

        // Set FeliCa (212 kbps) communication defaults
        // Code 0x00: InSetProtocol
        let proto_defaults = [
            0x00, 0x18, 0x01, 0x01, 0x02, 0x01, 0x03, 0x00, 0x04, 0x00,
            0x05, 0x00, 0x06, 0x00, 0x07, 0x08, 0x08, 0x00, 0x09, 0x00,
            0x0A, 0x00, 0x0B, 0x00, 0x0C, 0x00, 0x0E, 0x04, 0x0F, 0x00,
            0x10, 0x00, 0x11, 0x00, 0x12, 0x00, 0x13, 0x06,
        ];
        let _ = Self::send_command(&mut transport, 0x00, &proto_defaults);

        let info = ReaderInfoDto {
            id: format!("port100-{:04x}", pid),
            name: "Sony RC-S380 (Port-100)".to_string(),
            vendor_id: 0x054C,
            product_id: pid,
            chipset: format!("NFC Port-100 {}", fw_str),
        };

        Ok(Self { transport, info })
    }

    fn send_command(transport: &mut UsbTransport, code: u8, payload: &[u8]) -> Result<Vec<u8>> {
        let mut cmd = Vec::with_capacity(payload.len() + 2);
        cmd.push(0xD6);
        cmd.push(code);
        cmd.extend_from_slice(payload);

        let frame = build_port100_frame(&cmd);
        transport.write(&frame, Duration::from_millis(500))?;

        // Read ACK first
        let mut ack_buf = [0u8; 6];
        let _ = transport.read(&mut ack_buf, Duration::from_millis(300));

        // Read response frame
        let mut resp_buf = [0u8; 512];
        let n = transport.read(&mut resp_buf, Duration::from_millis(1000))?;
        let resp = parse_frame(&resp_buf[..n])
            .ok_or_else(|| Error::CommunicationError("Failed to parse Port-100 response frame".into()))?;

        if resp.first() == Some(&0xD7) && resp.get(1) == Some(&code.wrapping_add(1)) {
            Ok(resp[2..].to_vec())
        } else {
            Err(Error::CommunicationError(format!(
                "Unexpected response code for Port-100 command 0x{:02X}",
                code
            )))
        }
    }
}

impl ReaderDevice for Port100Device {
    fn info(&self) -> &ReaderInfoDto {
        &self.info
    }

    fn transceive(&mut self, felica_cmd: &[u8], timeout_ms: u16) -> Result<Vec<u8>> {
        // Code 0x04: initiator_exchange_rf
        let timeout_units = if timeout_ms > 0 {
            (((timeout_ms as u32) + 1) * 10).min(0xFFFF) as u16
        } else {
            100 // default ~10ms
        };

        let mut payload = timeout_units.to_le_bytes().to_vec();
        payload.extend_from_slice(felica_cmd);

        let rsp = Self::send_command(&mut self.transport, 0x04, &payload)?;
        if rsp.len() >= 4 && rsp[0..4] != [0, 0, 0, 0] {
            return Err(Error::CommunicationError("Port-100 RF communication fault".into()));
        }

        if rsp.len() >= 6 {
            // Byte 4 is payload length, bytes 5.. are FeliCa response
            Ok(rsp[5..].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    fn close(&mut self) -> Result<()> {
        let _ = Self::send_command(&mut self.transport, 0x06, &[0x00]); // Turn RF field OFF
        let _ = self.transport.write(&ACK_BYTES, Duration::from_millis(100));
        self.transport.close()
    }
}
