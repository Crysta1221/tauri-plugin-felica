//! PC/SC command layer for Port-400 FeliCa (from felica-rs).

use std::thread::sleep;
use std::time::Duration;

use crate::hardware::usb::UsbTransport;
use crate::error::Result;

use super::ccid::CcidTransport;
use super::tlv::{
    self, EXTENDED_TAG_PREFIX, SWITCH_PROTOCOL_METADATA_TAG,
};

const START_TRANSPARENT_SESSION_TAG: u8 = 0x81;
const END_TRANSPARENT_SESSION_TAG: u8 = 0x82;
const TURN_OFF_RF_TAG: u8 = 0x83;
const TURN_ON_RF_TAG: u8 = 0x84;
const TRANSMISSION_AND_RECEPTION_FLAG_TAG: u8 = 0x90;
const TRANSCEIVE_TAG: u8 = 0x95;
const FDT_TLV_TAG: u8 = 0x46;
const GET_FIRMWARE_VERSION_INS: u8 = 0x56;
const MANAGE_SESSION_INS: u8 = 0x50;
const TRANSPARENT_SESSION_CHANNEL: u8 = 0x01;

const DEFAULT_RECEIVE_TIMEOUT: Duration = Duration::from_millis(1_500);
const RF_ON_GUARD_TIME: Duration = Duration::from_millis(21);
const RF_OFF_GUARD_TIME: Duration = Duration::from_millis(30);
const SWITCH_PROTOCOL_GUARD_TIME: Duration = Duration::from_millis(20);
const SLOT_BUSY_RETRY_COUNT: usize = 1;
const SLOT_BUSY_END_SESSION_RETRIES: usize = 4;

/// FeliCa transmission flags: CRC on, no parity, no ISO prologue.
const FELICA_FLAG_MASK: u16 = 0x001C;

pub(super) struct Pcsc {
    ccid: CcidTransport,
    receive_timeout: Duration,
}

impl Pcsc {
    pub(super) fn new(transport: UsbTransport) -> Self {
        Self {
            ccid: CcidTransport::new(transport),
            receive_timeout: DEFAULT_RECEIVE_TIMEOUT,
        }
    }

    pub(super) fn product_labels(&self) -> (Option<&str>, Option<&str>) {
        self.ccid.product_labels()
    }

    pub(super) fn start_transparent_session(&mut self, priority: bool) -> Result<()> {
        if priority {
            let _ = self.manage_session(
                &[(END_TRANSPARENT_SESSION_TAG, &[][..])],
                SLOT_BUSY_END_SESSION_RETRIES,
            );
        }
        self.manage_session(
            &[(START_TRANSPARENT_SESSION_TAG, &[][..])],
            SLOT_BUSY_RETRY_COUNT,
        )?;
        self.turn_off_rf()?;
        sleep(RF_OFF_GUARD_TIME);
        self.turn_on_rf()?;
        sleep(RF_ON_GUARD_TIME);
        Ok(())
    }

    pub(super) fn end_transparent_session(&mut self) -> Result<()> {
        let _ = self.turn_off_rf();
        self.manage_session(
            &[(END_TRANSPARENT_SESSION_TAG, &[][..])],
            SLOT_BUSY_END_SESSION_RETRIES,
        )?;
        Ok(())
    }

    pub(super) fn switch_protocol_type_f(&mut self) -> Result<()> {
        let mut payload = Vec::new();
        payload.push(SWITCH_PROTOCOL_METADATA_TAG);
        payload.push(2);
        payload.push(3);
        payload.push(0);
        let response = self.manage_session_raw(&payload, 0x02, SLOT_BUSY_RETRY_COUNT)?;
        tlv::parse_switch_protocol_response(&response)?;
        sleep(SWITCH_PROTOCOL_GUARD_TIME);
        Ok(())
    }

    pub(super) fn get_firmware_version(&mut self) -> Result<Vec<u8>> {
        let frame = [0xFF, GET_FIRMWARE_VERSION_INS, 0x00, 0x00];
        let response = self
            .ccid
            .escape(&frame, self.receive_timeout, SLOT_BUSY_RETRY_COUNT)?;
        tlv::verify_status(&response)?;
        Ok(response[..response.len() - 2].to_vec())
    }

    pub(super) fn transceive(&mut self, payload: &[u8], timeout: Duration) -> Result<Vec<u8>> {
        let mut fields = Vec::new();
        fields.push(TRANSMISSION_AND_RECEPTION_FLAG_TAG);
        fields.push(2);
        fields.push((FELICA_FLAG_MASK >> 8) as u8);
        fields.push((FELICA_FLAG_MASK & 0xFF) as u8);
        if timeout > Duration::from_millis(0) {
            let micros = (timeout.as_millis() * 1_000).min(u32::MAX as u128) as u32;
            fields.push(EXTENDED_TAG_PREFIX);
            fields.push(FDT_TLV_TAG);
            fields.push(4);
            fields.extend_from_slice(&micros.to_le_bytes());
        }
        if !payload.is_empty() {
            tlv::push_extended_tlv(&mut fields, TRANSCEIVE_TAG, payload);
        }
        let mut frame = vec![0xFF, MANAGE_SESSION_INS, 0x00, TRANSPARENT_SESSION_CHANNEL];
        frame.push(0x00);
        frame.push(((fields.len() >> 8) & 0xFF) as u8);
        frame.push((fields.len() & 0xFF) as u8);
        frame.extend_from_slice(&fields);
        frame.extend_from_slice(&[0x00, 0x00, 0x00]);
        let response = self
            .ccid
            .escape(&frame, self.receive_timeout, SLOT_BUSY_RETRY_COUNT)?;
        tlv::verify_status(&response)?;
        tlv::parse_transparent_response(&response[..response.len() - 2]).map(|r| r.payload)
    }

    pub(super) fn turn_off_rf(&mut self) -> Result<()> {
        self.manage_session(&[(TURN_OFF_RF_TAG, &[][..])], SLOT_BUSY_RETRY_COUNT)?;
        Ok(())
    }

    pub(super) fn turn_on_rf(&mut self) -> Result<()> {
        self.manage_session(&[(TURN_ON_RF_TAG, &[][..])], SLOT_BUSY_RETRY_COUNT)?;
        Ok(())
    }

    pub(super) fn close(&mut self) -> Result<()> {
        self.ccid.close()
    }

    fn manage_session(
        &mut self,
        commands: &[(u8, &[u8])],
        slot_busy_retries: usize,
    ) -> Result<Vec<u8>> {
        let mut payload = Vec::new();
        for (tag, value) in commands {
            payload.push(*tag);
            payload.push(value.len() as u8);
            payload.extend_from_slice(value);
        }
        let response = self.manage_session_raw(&payload, 0x00, slot_busy_retries)?;
        tlv::parse_manage_session_response(&response)?;
        Ok(response)
    }

    fn manage_session_raw(
        &mut self,
        payload: &[u8],
        channel: u8,
        slot_busy_retries: usize,
    ) -> Result<Vec<u8>> {
        let mut frame = vec![
            0xFF,
            MANAGE_SESSION_INS,
            0x00,
            channel,
            payload.len() as u8,
        ];
        frame.extend_from_slice(payload);
        frame.push(0x00);
        let response = self
            .ccid
            .escape(&frame, self.receive_timeout, slot_busy_retries)?;
        tlv::verify_status(&response)?;
        Ok(response[..response.len() - 2].to_vec())
    }
}
