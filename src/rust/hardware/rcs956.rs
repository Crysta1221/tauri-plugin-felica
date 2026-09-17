use std::time::Duration;
use crate::hardware::framing::{build_normal_frame, parse_frame, ACK_BYTES};
use crate::hardware::usb::UsbTransport;
use crate::error::{Error, Result};
use crate::models::reader::ReaderInfoDto;
use super::ReaderDevice;

pub const RCS956_PIDS: &[u16] = &[0x02E1, 0x0193];

/// Driver for Sony RC-S330 / RC-S360 / RC-S370 (RC-S956 / PN533 family).
pub struct Rcs956Device {
    transport: UsbTransport,
    info: ReaderInfoDto,
}

impl Rcs956Device {
    pub fn open(pid: Option<u16>, location: Option<super::UsbLocation>) -> Result<Self> {
        super::open_with_pids(
            RCS956_PIDS,
            pid,
            location,
            Self::init_device,
            "RC-S956 reader not found",
        )
    }

    fn init_device(mut transport: UsbTransport, pid: u16) -> Result<Self> {
        // Send ACK and flush
        let _ = transport.write(&ACK_BYTES, Duration::from_millis(100));
        let mut trash = [0u8; 64];
        while transport.read(&mut trash, Duration::from_millis(20)).is_ok() {}

        // Get Firmware Version: D4 02
        let fw = Self::send_command(&mut transport, &[0xD4, 0x02])?;
        let fw_str = if fw.len() >= 4 {
            format!("v{}.{}", fw[2], fw[3])
        } else {
            "v1.0".to_string()
        };

        // RF Configuration - Turn ON RF field: D4 32 01 01
        let _ = Self::send_command(&mut transport, &[0xD4, 0x32, 0x01, 0x01]);

        let info = ReaderInfoDto {
            id: format!("rcs956-{:04x}", pid),
            name: "Sony RC-S330/360/370 (RC-S956)".to_string(),
            vendor_id: 0x054C,
            product_id: pid,
            chipset: format!("RC-S956 {}", fw_str),
        };

        Ok(Self { transport, info })
    }

    fn send_command(transport: &mut UsbTransport, cmd: &[u8]) -> Result<Vec<u8>> {
        let frame = build_normal_frame(cmd);
        transport.write(&frame, Duration::from_millis(500))?;

        // Read ACK
        let mut ack_buf = [0u8; 6];
        let _ = transport.read(&mut ack_buf, Duration::from_millis(300));

        // Read response
        let mut resp_buf = [0u8; 512];
        let n = transport.read(&mut resp_buf, Duration::from_millis(1000))?;
        let resp = parse_frame(&resp_buf[..n])
            .ok_or_else(|| Error::CommunicationError("Failed to parse RC-S956 response frame".into()))?;

        if resp.len() >= 2 && resp[0] == 0xD5 && resp[1] == cmd[1].wrapping_add(1) {
            Ok(resp[2..].to_vec())
        } else {
            Err(Error::CommunicationError("Unexpected RC-S956 response".into()))
        }
    }
}

impl ReaderDevice for Rcs956Device {
    fn info(&self) -> &ReaderInfoDto {
        &self.info
    }

    fn transceive(&mut self, felica_cmd: &[u8], _timeout_ms: u16) -> Result<Vec<u8>> {
        // InCommunicateThru: D4 42 [data...]
        let mut cmd = Vec::with_capacity(2 + felica_cmd.len());
        cmd.push(0xD4);
        cmd.push(0x42);
        cmd.extend_from_slice(felica_cmd);

        let resp = Self::send_command(&mut self.transport, &cmd)?;
        if resp.first() == Some(&0x00) {
            // Byte 0 is status (0x00 = success), bytes 1.. are FeliCa response
            Ok(resp[1..].to_vec())
        } else {
            Err(Error::CommunicationError("RC-S956 InCommunicateThru failed".into()))
        }
    }

    fn close(&mut self) -> Result<()> {
        // Turn RF field OFF: D4 32 01 00
        let _ = Self::send_command(&mut self.transport, &[0xD4, 0x32, 0x01, 0x00]);
        let _ = self.transport.write(&ACK_BYTES, Duration::from_millis(100));
        self.transport.close()
    }
}
