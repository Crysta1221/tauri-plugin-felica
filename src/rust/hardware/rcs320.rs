//! Sony RC-S320 driver (control OUT + interrupt IN), following felica-rs.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::hardware::framing::{checksum, ACK_BYTES, SOF};
use crate::hardware::io;
use crate::hardware::usb::InterruptTransport;
use crate::error::{Error, Result};
use crate::models::reader::ReaderInfoDto;

use super::ReaderDevice;

pub const RCS320_PIDS: &[u16] = &[0x01BB];

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1000);
const ACK_TIMEOUT: Duration = Duration::from_millis(1000);
const MAX_DATA_SIZE: usize = 255;

mod cmd {
    pub const GET_FIRMWARE_VERSION: u8 = 0x58;
    pub const GET_FIRMWARE_VERSION_RES: u8 = 0x59;
    pub const SEND_PACKET: u8 = 0x5C;
    pub const SEND_PACKET_RES: u8 = 0x5D;
}

mod init_seq {
    pub const INIT0: &[u8] = &[0x62, 0x01, 0x82];
    pub const INIT1: &[u8] = &[0x62, 0x02, 0x80, 0x81];
    pub const INIT2: &[u8] = &[0x62, 0x22, 0x80, 0xcc, 0x81, 0x88];
    pub const INIT3: &[u8] = &[0x62, 0x02, 0x80, 0x81];
    pub const INIT4: &[u8] = &[0x62, 0x02, 0x82, 0x87];
    pub const INIT5: &[u8] = &[0x62, 0x21, 0x25, 0x58];
    pub const RF_ON: &[u8] = &[0x5a, 0x80];
    pub const RESET: &[u8] = &[0x54];
}

fn build_rcs320_frame(payload: &[u8]) -> Vec<u8> {
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

/// Driver for Sony RC-S320.
pub struct Rcs320Device {
    transport: InterruptTransport,
    read_buffer: VecDeque<u8>,
    info: ReaderInfoDto,
}

impl Rcs320Device {
    pub fn open(pid: Option<u16>, location: Option<super::UsbLocation>) -> Result<Self> {
        let product_id = pid.unwrap_or(RCS320_PIDS[0]);
        if !RCS320_PIDS.contains(&product_id) {
            return Err(Error::DeviceNotFound("RC-S320 reader not found".into()));
        }
        let transport = InterruptTransport::open(
            super::SONY_VENDOR_ID,
            product_id,
            location,
            "RC-S320 reader not found",
        )?;
        Self::init_device(transport, product_id)
    }

    fn init_device(transport: InterruptTransport, pid: u16) -> Result<Self> {
        let name = match (&transport.manufacturer, &transport.product) {
            (Some(vendor), Some(product)) => format!("{vendor} {product}"),
            (None, Some(product)) => product.clone(),
            _ => "Sony RC-S320".to_string(),
        };
        let mut device = Self {
            transport,
            read_buffer: VecDeque::new(),
            info: ReaderInfoDto {
                id: format!("rcs320-{pid:04x}"),
                name,
                vendor_id: super::SONY_VENDOR_ID,
                product_id: pid,
                chipset: "RC-S320".to_string(),
            },
        };

        device.send_init_command(init_seq::INIT0)?;
        device.send_init_command(init_seq::INIT1)?;
        device.send_init_command(init_seq::INIT2)?;
        device.send_init_command(init_seq::INIT3)?;
        device.send_init_command(init_seq::INIT4)?;
        device.send_init_command(init_seq::INIT5)?;
        device.send_init_command(init_seq::RF_ON)?;

        let (major, minor) = device.get_firmware_version()?;
        device.info.chipset = format!("RCS320v{major}.{minor}");
        Ok(device)
    }

    fn send_init_command(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.packet_write(data)?;
        self.recv_response(DEFAULT_TIMEOUT)
    }

    fn get_firmware_version(&mut self) -> Result<(u8, u8)> {
        self.packet_write(&[cmd::GET_FIRMWARE_VERSION])?;
        let response = self.packet_read(DEFAULT_TIMEOUT)?;
        if response.first() != Some(&cmd::GET_FIRMWARE_VERSION_RES) || response.len() < 3 {
            return Err(Error::CommunicationError(
                "invalid RC-S320 firmware version response".into(),
            ));
        }
        Ok((response[2], response[1]))
    }

    fn packet_write(&mut self, data: &[u8]) -> Result<()> {
        self.transport
            .write_control(&build_rcs320_frame(data), DEFAULT_TIMEOUT)?;
        self.wait_for_ack()
    }

    fn packet_read(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        self.recv_response(timeout)
    }

    fn wait_for_ack(&mut self) -> Result<()> {
        let deadline = Instant::now() + ACK_TIMEOUT;
        let bytes = self.read_exact(ACK_BYTES.len(), deadline)?;
        if bytes == ACK_BYTES {
            return Ok(());
        }
        if bytes.get(0..3) == Some(&SOF) {
            for byte in bytes.into_iter().rev() {
                self.read_buffer.push_front(byte);
            }
            return Ok(());
        }
        Err(Error::CommunicationError(format!(
            "expected ACK, got: {bytes:02X?}"
        )))
    }

    fn recv_response(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        let frame_bytes = self.read_frame_bytes(deadline)?;
        if frame_bytes == ACK_BYTES {
            let frame_bytes = self.read_frame_bytes(deadline)?;
            return parse_data_payload(&frame_bytes);
        }
        parse_data_payload(&frame_bytes)
    }

    fn read_frame_bytes(&mut self, deadline: Instant) -> Result<Vec<u8>> {
        let mut frame = self.read_exact(5, deadline)?;
        if frame.get(0..3) != Some(&SOF) {
            return Err(Error::CommunicationError(
                "invalid RC-S320 frame preamble".into(),
            ));
        }
        if frame[3] == 0x00 && frame[4] == 0xFF {
            let postamble = self.read_exact(1, deadline)?;
            frame.extend_from_slice(&postamble);
            return Ok(frame);
        }
        let len = frame[3] as usize;
        let tail = self.read_exact(len + 2, deadline)?;
        frame.extend_from_slice(&tail);
        Ok(frame)
    }

    fn read_exact(&mut self, len: usize, deadline: Instant) -> Result<Vec<u8>> {
        let transport = &mut self.transport;
        io::read_exact(
            &mut |timeout| transport.read_interrupt(timeout),
            &mut self.read_buffer,
            len,
            deadline,
        )
    }

    fn communicate_thru(&mut self, data: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        if data.len() > MAX_DATA_SIZE - 2 {
            return Err(Error::CommunicationError(
                "data too long for RC-S320 send_to_card".into(),
            ));
        }
        let mut cmd = Vec::with_capacity(data.len() + 2);
        cmd.push(cmd::SEND_PACKET);
        cmd.push((data.len() + 1) as u8);
        cmd.extend_from_slice(data);
        self.packet_write(&cmd)?;

        let response = self.packet_read(timeout)?;
        if response.first() != Some(&cmd::SEND_PACKET_RES) || response.len() < 2 {
            return Err(Error::CommunicationError(
                "invalid RC-S320 card response".into(),
            ));
        }
        let len = response[1] as usize;
        let available = response.len() - 2;
        let data_len = len.min(available);
        Ok(response[2..2 + data_len].to_vec())
    }
}

fn parse_data_payload(frame: &[u8]) -> Result<Vec<u8>> {
    if frame.len() < 7 || frame.get(0..3) != Some(&SOF) {
        return Err(Error::CommunicationError(
            "invalid RC-S320 response frame".into(),
        ));
    }
    let len = frame[3] as usize;
    if frame.len() < 5 + len + 2 {
        return Err(Error::CommunicationError("short RC-S320 response frame".into()));
    }
    let payload = frame[5..5 + len].to_vec();
    if payload.first() == Some(&0x7F) {
        return Err(Error::CommunicationError("RC-S320 error frame".into()));
    }
    Ok(payload)
}

impl ReaderDevice for Rcs320Device {
    fn info(&self) -> &ReaderInfoDto {
        &self.info
    }

    fn transceive(&mut self, felica_cmd: &[u8], timeout_ms: u16) -> Result<Vec<u8>> {
        if felica_cmd.is_empty() {
            return Err(Error::CommunicationError("empty transceive data".into()));
        }
        let timeout = Duration::from_millis(timeout_ms as u64 + 100);
        let card_data = if felica_cmd[0] as usize == felica_cmd.len() {
            &felica_cmd[1..]
        } else {
            felica_cmd
        };
        let response = self.communicate_thru(card_data, timeout)?;
        let mut result = Vec::with_capacity(response.len() + 1);
        result.push((response.len() + 1) as u8);
        result.extend_from_slice(&response);
        Ok(result)
    }

    fn close(&mut self) -> Result<()> {
        let _ = self.send_init_command(init_seq::RESET);
        self.transport.close()
    }
}
