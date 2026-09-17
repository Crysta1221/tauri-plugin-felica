//! Sony NFC Port-400 (RC-S300) USB CCID/PC/SC backend.

mod ccid;
mod pcsc;
mod tlv;

use std::time::Duration;

use crate::hardware::usb::UsbTransport;
use crate::error::Result;
use crate::models::reader::ReaderInfoDto;

use super::ReaderDevice;
use pcsc::Pcsc;

pub const PORT400_PIDS: &[u16] = &[0x0DC8, 0x0DC9, 0x0D8F];

/// Driver for Sony RC-S300 (Port-400).
pub struct Port400Device {
    pcsc: Pcsc,
    info: ReaderInfoDto,
    session_open: bool,
}

impl Port400Device {
    pub fn open(pid: Option<u16>, location: Option<super::UsbLocation>) -> Result<Self> {
        super::open_with_pids(
            PORT400_PIDS,
            pid,
            location,
            Self::init_device,
            "RC-S300 reader not found",
        )
    }

    fn init_device(transport: UsbTransport, pid: u16) -> Result<Self> {
        let mut pcsc = Pcsc::new(transport);
        let version = pcsc.get_firmware_version()?;
        let chipset = format_firmware(&version)
            .map(|fw| format!("NFC Port-400 {fw}"))
            .unwrap_or_else(|| "NFC Port-400".to_string());

        // End a leftover session first, then open Type-F the way felica-rs does.
        pcsc.start_transparent_session(true)?;
        pcsc.switch_protocol_type_f()?;

        let (manufacturer, product) = pcsc.product_labels();
        let name = match (manufacturer, product) {
            (Some(vendor), Some(product)) => format!("{vendor} {product}"),
            (None, Some(product)) => product.to_string(),
            _ => "Sony RC-S300 (Port-400)".to_string(),
        };

        Ok(Self {
            pcsc,
            info: ReaderInfoDto {
                id: format!("port400-{pid:04x}"),
                name,
                vendor_id: 0x054C,
                product_id: pid,
                chipset,
            },
            session_open: true,
        })
    }
}

impl ReaderDevice for Port400Device {
    fn info(&self) -> &ReaderInfoDto {
        &self.info
    }

    fn transceive(&mut self, felica_cmd: &[u8], timeout_ms: u16) -> Result<Vec<u8>> {
        let timeout = Duration::from_millis(timeout_ms.max(50) as u64);
        self.pcsc.transceive(felica_cmd, timeout)
    }

    fn close(&mut self) -> Result<()> {
        if self.session_open {
            let _ = self.pcsc.end_transparent_session();
            self.session_open = false;
        }
        self.pcsc.close()
    }
}

fn format_firmware(version: &[u8]) -> Option<String> {
    if version.len() < 2 {
        return None;
    }
    Some(format!("v{}.{}", version[0], version[1]))
}

#[cfg(test)]
mod tests {
    use super::format_firmware;

    #[test]
    fn format_firmware_needs_two_bytes() {
        assert_eq!(format_firmware(&[1, 2]), Some("v1.2".to_string()));
        assert_eq!(format_firmware(&[1]), None);
    }
}
