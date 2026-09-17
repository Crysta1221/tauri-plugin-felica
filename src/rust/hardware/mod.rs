pub mod framing;
mod io;
pub mod port100;
pub mod port400;
pub mod rcs320;
pub mod rcs956;
pub mod usb;

use crate::hardware::usb::UsbTransport;
use crate::error::{Error, Result};
use crate::models::reader::{ReaderInfoDto, ReaderPreference};
pub use port100::Port100Device;
pub use port400::Port400Device;
pub use rcs320::Rcs320Device;
pub use rcs956::Rcs956Device;

pub const SONY_VENDOR_ID: u16 = 0x054C;

/// USB bus/address taken from a listed reader id (`family-bus-address-pid`).
pub type UsbLocation = (u8, u8);

/// Common capability trait for all supported PaSoRi reader hardware.
pub trait ReaderDevice: Send {
    /// Returns static device metadata.
    fn info(&self) -> &ReaderInfoDto;

    /// Transceives raw FeliCa command bytes to the card via RF field and returns response bytes.
    fn transceive(&mut self, felica_cmd: &[u8], timeout_ms: u16) -> Result<Vec<u8>>;

    /// Closes the device and turns off the RF field.
    fn close(&mut self) -> Result<()>;
}

/// Enumerates all connected Sony PaSoRi readers.
pub fn list_pasori_devices() -> Result<Vec<ReaderInfoDto>> {
    let mut readers = Vec::new();

    let devices = rusb::devices().map_err(|err| crate::hardware::usb::map_rusb(err, "USB enumerate"))?;

    for device in devices.iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        if desc.vendor_id() != SONY_VENDOR_ID {
            continue;
        }

        let pid = desc.product_id();
        let bus = device.bus_number();
        let address = device.address();

        if port100::PORT100_PIDS.contains(&pid) {
            readers.push(ReaderInfoDto {
                id: format!("port100-{}-{}-{:04x}", bus, address, pid),
                name: "Sony RC-S380 (Port-100)".to_string(),
                vendor_id: SONY_VENDOR_ID,
                product_id: pid,
                chipset: "NFC Port-100".to_string(),
            });
        } else if port400::PORT400_PIDS.contains(&pid) {
            readers.push(ReaderInfoDto {
                id: format!("port400-{}-{}-{:04x}", bus, address, pid),
                name: "Sony RC-S300 (Port-400)".to_string(),
                vendor_id: SONY_VENDOR_ID,
                product_id: pid,
                chipset: "NFC Port-400".to_string(),
            });
        } else if rcs956::RCS956_PIDS.contains(&pid) {
            readers.push(ReaderInfoDto {
                id: format!("rcs956-{}-{}-{:04x}", bus, address, pid),
                name: "Sony RC-S330/360/370 (RC-S956)".to_string(),
                vendor_id: SONY_VENDOR_ID,
                product_id: pid,
                chipset: "RC-S956".to_string(),
            });
        } else if rcs320::RCS320_PIDS.contains(&pid) {
            readers.push(ReaderInfoDto {
                id: format!("rcs320-{}-{}-{:04x}", bus, address, pid),
                name: "Sony RC-S320".to_string(),
                vendor_id: SONY_VENDOR_ID,
                product_id: pid,
                chipset: "RC-S320".to_string(),
            });
        }

    }

    Ok(readers)
}

fn parse_listed_id(id: &str) -> Option<(&str, u8, u8, u16)> {
    let mut parts = id.splitn(4, '-');
    let family = parts.next()?;
    let bus = parts.next()?.parse().ok()?;
    let address = parts.next()?.parse().ok()?;
    let pid = u16::from_str_radix(parts.next()?, 16).ok()?;
    Some((family, bus, address, pid))
}

fn open_with_pids<T>(
    pids: &[u16],
    pid: Option<u16>,
    location: Option<UsbLocation>,
    init: impl Fn(UsbTransport, u16) -> Result<T>,
    not_found: &str,
) -> Result<T> {
    let pids: Vec<u16> = pid.map(|value| vec![value]).unwrap_or_else(|| pids.to_vec());
    let mut last_err = None;
    for &product_id in &pids {
        match UsbTransport::open(SONY_VENDOR_ID, product_id, location) {
            Ok(transport) => match init(transport, product_id) {
                Ok(device) => return Ok(device),
                Err(err) => last_err = Some(err),
            },
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.unwrap_or_else(|| Error::DeviceNotFound(not_found.into())))
}

/// Opens a reader device based on optional ID and preference.
pub fn open_reader(id: Option<&str>, preference: Option<ReaderPreference>) -> Result<Box<dyn ReaderDevice>> {
    let pref = preference.unwrap_or_default();

    if let Some(id_str) = id {
        if !id_str.eq_ignore_ascii_case("auto") {
            if let Some((family, bus, address, pid)) = parse_listed_id(id_str) {
                let location = Some((bus, address));
                return match family {
                    "port100" => Ok(Box::new(Port100Device::open(Some(pid), location)?)),
                    "port400" => Ok(Box::new(Port400Device::open(Some(pid), location)?)),
                    "rcs956" => Ok(Box::new(Rcs956Device::open(Some(pid), location)?)),
                    "rcs320" => Ok(Box::new(Rcs320Device::open(Some(pid), location)?)),
                    _ => Err(Error::DeviceNotFound(format!("Unknown reader family '{family}'"))),
                };
            }
            if id_str.starts_with("port100") {
                return Ok(Box::new(Port100Device::open(None, None)?));
            }
            if id_str.starts_with("port400") {
                return Ok(Box::new(Port400Device::open(None, None)?));
            }
            if id_str.starts_with("rcs956") {
                return Ok(Box::new(Rcs956Device::open(None, None)?));
            }
            if id_str.starts_with("rcs320") {
                return Ok(Box::new(Rcs320Device::open(None, None)?));
            }
            return Err(Error::DeviceNotFound(format!(
                "Unknown reader id '{id_str}'"
            )));
        }
    }

    match pref {
        ReaderPreference::RcS380 => Ok(Box::new(Port100Device::open(None, None)?)),
        ReaderPreference::RcS300 => Ok(Box::new(Port400Device::open(None, None)?)),
        ReaderPreference::RcS330 => Ok(Box::new(Rcs956Device::open(None, None)?)),
        ReaderPreference::RcS320 => Ok(Box::new(Rcs320Device::open(None, None)?)),
        ReaderPreference::Auto => open_first_available(),
    }
}

fn open_first_available() -> Result<Box<dyn ReaderDevice>> {
    let probes: [fn() -> Result<Box<dyn ReaderDevice>>; 4] = [
        || Port100Device::open(None, None).map(|d| Box::new(d) as _),
        || Port400Device::open(None, None).map(|d| Box::new(d) as _),
        || Rcs956Device::open(None, None).map(|d| Box::new(d) as _),
        || Rcs320Device::open(None, None).map(|d| Box::new(d) as _),
    ];
    let mut last_err = None;
    let mut access_err = None;
    for open in probes {
        match open() {
            Ok(device) => return Ok(device),
            Err(err @ Error::DeviceAccessDenied(_)) | Err(err @ Error::DeviceBusy) => {
                access_err = Some(err);
            }
            Err(err) => last_err = Some(err),
        }
    }
    if let Some(err) = access_err {
        return Err(err);
    }
    Err(last_err.unwrap_or_else(|| {
        Error::DeviceNotFound("No compatible PaSoRi reader was detected".into())
    }))
}
