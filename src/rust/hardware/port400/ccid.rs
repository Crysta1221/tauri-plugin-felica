//! CCID escape framing for Port-400 (from felica-rs).

use std::collections::VecDeque;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::hardware::io;
use crate::hardware::usb::UsbTransport;
use crate::error::{Error, Result};

pub(super) const CCID_SLOT_NUMBER: u8 = 0;
const CCID_HEADER_LEN: usize = 10;
const SLOT_BUSY_ERROR: u8 = 0xE0;
const SLOT_BUSY_WAIT_TIME: Duration = Duration::from_millis(50);
const TIME_EXTENSION_WAIT: Duration = Duration::from_millis(20);
const SEQUENCE_ERROR_RETRY_COUNT: usize = 2;

pub(super) struct CcidTransport {
    transport: UsbTransport,
    sequence: u8,
    buffer: VecDeque<u8>,
}

impl CcidTransport {
    pub(super) fn new(transport: UsbTransport) -> Self {
        Self {
            transport,
            sequence: 0,
            buffer: VecDeque::new(),
        }
    }

    pub(super) fn product_labels(&self) -> (Option<&str>, Option<&str>) {
        (
            self.transport.manufacturer.as_deref(),
            self.transport.product.as_deref(),
        )
    }

    pub(super) fn escape(
        &mut self,
        payload: &[u8],
        timeout: Duration,
        slot_busy_retries: usize,
    ) -> Result<Vec<u8>> {
        let mut remaining_retries = slot_busy_retries + 1;
        while remaining_retries > 0 {
            let deadline = Instant::now() + timeout;
            let seq = self.next_sequence();
            let frame = build_escape_frame(payload, seq);
            self.transport.write(&frame, timeout)?;
            self.buffer.clear();
            let mut seq_retry = SEQUENCE_ERROR_RETRY_COUNT + 1;
            loop {
                let header = self.read_exact(CCID_HEADER_LEN, deadline)?;
                let (response, status) = CcidResponse::parse(&header, seq)?;
                if status == CommandStatus::SequenceMismatch {
                    seq_retry -= 1;
                    if seq_retry == 0 {
                        return Err(Error::CommunicationError(
                            "CCID sequence mismatch".into(),
                        ));
                    }
                    if response.length > 0 {
                        self.read_exact(response.length, deadline)?;
                    }
                    continue;
                }
                let data = if response.length > 0 {
                    self.read_exact(response.length, deadline)?
                } else {
                    Vec::new()
                };
                match status {
                    CommandStatus::Success => {
                        if data.len() < 2 {
                            return Err(Error::CommunicationError(
                                "escape response too short".into(),
                            ));
                        }
                        return Ok(data);
                    }
                    CommandStatus::SlotBusy => {
                        remaining_retries -= 1;
                        if remaining_retries == 0 {
                            return Err(Error::DeviceBusy);
                        }
                        sleep(SLOT_BUSY_WAIT_TIME);
                        break;
                    }
                    CommandStatus::TimeExtension => {
                        sleep(TIME_EXTENSION_WAIT);
                    }
                    CommandStatus::Failure(code) => {
                        return Err(Error::CommunicationError(format!(
                            "CCID failure {code:#04x}"
                        )));
                    }
                    CommandStatus::SequenceMismatch => unreachable!(),
                }
            }
        }
        Err(Error::DeviceBusy)
    }

    fn read_exact(&mut self, len: usize, deadline: Instant) -> Result<Vec<u8>> {
        let transport = &mut self.transport;
        io::read_exact(
            &mut |timeout| transport.read_packet(timeout),
            &mut self.buffer,
            len,
            deadline,
        )
    }

    fn next_sequence(&mut self) -> u8 {
        self.sequence = self.sequence.wrapping_add(1);
        self.sequence
    }

    pub(super) fn close(&mut self) -> Result<()> {
        self.transport.close()
    }
}

fn build_escape_frame(payload: &[u8], seq: u8) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.push(0x6B);
    let len = payload.len() as u32;
    frame.extend_from_slice(&len.to_le_bytes());
    frame.push(CCID_SLOT_NUMBER);
    frame.push(seq);
    frame.push(0);
    frame.push(0);
    frame.push(0);
    if !payload.is_empty() {
        frame.extend_from_slice(payload);
    }
    frame
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandStatus {
    Success,
    Failure(u8),
    SlotBusy,
    TimeExtension,
    SequenceMismatch,
}

struct CcidResponse {
    length: usize,
}

impl CcidResponse {
    fn parse(data: &[u8], expected_seq: u8) -> Result<(Self, CommandStatus)> {
        if data.len() < CCID_HEADER_LEN {
            return Err(Error::CommunicationError("short CCID header".into()));
        }
        if data[0] != 0x83 {
            return Err(Error::CommunicationError("invalid CCID message".into()));
        }
        let length = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
        if data[5] != CCID_SLOT_NUMBER {
            return Err(Error::CommunicationError("invalid CCID slot number".into()));
        }
        if data[6] != expected_seq {
            return Ok((Self { length }, CommandStatus::SequenceMismatch));
        }
        let status_byte = data[7];
        let error = data[8];
        let command_status = (status_byte >> 6) & 0x03;
        let status = match command_status {
            0 => CommandStatus::Success,
            1 => {
                if error == SLOT_BUSY_ERROR {
                    CommandStatus::SlotBusy
                } else {
                    CommandStatus::Failure(error)
                }
            }
            2 => CommandStatus::TimeExtension,
            _ => CommandStatus::Failure(error),
        };
        Ok((Self { length }, status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_escape_frame_matches_felica_rs() {
        let frame = build_escape_frame(&[0xAA, 0xBB], 0x05);
        assert_eq!(
            frame,
            vec![
                0x6B, 0x02, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0xAA, 0xBB
            ]
        );
    }
}
