use rusb::{ConfigDescriptor, DeviceHandle, Direction, GlobalContext, TransferType};

use std::time::Duration;

use crate::error::{Error, Result};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);

/// Bulk transfer endpoints used by Port-100, Port-400 and RC-S956.
#[derive(Debug, Clone, Copy)]
pub struct BulkEndpoints {
    pub in_ep: u8,
    pub out_ep: u8,
    pub max_packet_size: u16,
}

/// Opened USB device with its interface claimed.
pub(crate) struct ClaimedUsb<E> {
    pub handle: DeviceHandle<GlobalContext>,
    pub interface: u8,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub endpoints: E,
}

/// Maps a `rusb` failure onto the plugin error that callers already handle.
pub(crate) fn map_rusb(err: rusb::Error, context: &str) -> Error {
    match err {
        rusb::Error::Access => Error::DeviceAccessDenied(format!("{context}: {err}")),
        rusb::Error::Busy => Error::DeviceBusy,
        rusb::Error::NoDevice | rusb::Error::NotFound => Error::DeviceDisconnected,
        rusb::Error::Timeout => Error::CommunicationError(format!("{context} timed out")),
        other => Error::from(other),
    }
}

/// Bulk interfaces that can carry reader traffic, preferred in this order:
/// vendor-specific (RC-S300/P WinUSB), other bulk, then CCID (Sony PC/SC).
#[derive(Debug, Clone, Copy)]
struct BulkCandidate {
    interface: u8,
    class: u8,
    endpoints: BulkEndpoints,
}

const USB_CLASS_CCID: u8 = 0x0B;
const USB_CLASS_VENDOR: u8 = 0xFF;

fn bulk_preference(class: u8) -> u8 {
    match class {
        USB_CLASS_VENDOR => 0,
        USB_CLASS_CCID => 2,
        _ => 1,
    }
}

fn list_bulk_interfaces(config: &ConfigDescriptor) -> Vec<BulkCandidate> {
    let mut candidates = Vec::new();
    for interface_desc in config.interfaces() {
        for descriptor in interface_desc.descriptors() {
            let mut in_ep = None;
            let mut out_ep = None;
            let mut max_packet = 0;
            for endpoint in descriptor.endpoint_descriptors() {
                if endpoint.transfer_type() != TransferType::Bulk {
                    continue;
                }
                match endpoint.direction() {
                    Direction::In if in_ep.is_none() => {
                        in_ep = Some(endpoint.address());
                        max_packet = endpoint.max_packet_size();
                    }
                    Direction::Out if out_ep.is_none() => {
                        out_ep = Some(endpoint.address());
                        if max_packet == 0 {
                            max_packet = endpoint.max_packet_size();
                        }
                    }
                    _ => {}
                }
            }
            let (Some(in_ep), Some(out_ep)) = (in_ep, out_ep) else {
                continue;
            };
            candidates.push(BulkCandidate {
                interface: descriptor.interface_number(),
                class: descriptor.class_code(),
                endpoints: BulkEndpoints {
                    in_ep,
                    out_ep,
                    max_packet_size: max_packet,
                },
            });
        }
    }
    candidates.sort_by_key(|candidate| bulk_preference(candidate.class));
    candidates
}

/// Selects the first interface that exposes both a bulk IN and a bulk OUT endpoint.
pub fn select_bulk_endpoints(config: &ConfigDescriptor) -> Result<(u8, BulkEndpoints)> {
    let candidate = list_bulk_interfaces(config).into_iter().next().ok_or_else(|| {
        Error::DeviceNotFound("Missing bulk IN/OUT endpoints".into())
    })?;
    Ok((candidate.interface, candidate.endpoints))
}

/// Locates a Sony reader by VID/PID (and optional bus/address), detaches a
/// kernel driver when one is bound, sets the configuration if the device is
/// still in the Address state, and claims the interface selected by `select`.
pub(crate) fn open_usb_interface<E>(
    vendor_id: u16,
    product_id: u16,
    location: Option<(u8, u8)>,
    not_found: &str,
    select: impl FnOnce(&ConfigDescriptor) -> Result<(u8, E)>,
) -> Result<ClaimedUsb<E>> {
    let handle = open_handle(vendor_id, product_id, location, not_found)?;
    from_handle(handle, select)
}

fn open_handle(
    vendor_id: u16,
    product_id: u16,
    location: Option<(u8, u8)>,
    not_found: &str,
) -> Result<DeviceHandle<GlobalContext>> {
    if let Some((bus, address)) = location {
        let devices = rusb::devices().map_err(|err| map_rusb(err, "USB enumerate"))?;
        for device in devices.iter() {
            if device.bus_number() != bus || device.address() != address {
                continue;
            }
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if desc.vendor_id() != vendor_id || desc.product_id() != product_id {
                continue;
            }
            return device
                .open()
                .map_err(|err| map_rusb(err, "USB open"));
        }
        return Err(Error::DeviceNotFound(format!(
            "Device {vendor_id:04X}:{product_id:04X} at bus {bus} address {address} not found"
        )));
    }

    rusb::open_device_with_vid_pid(vendor_id, product_id)
        .ok_or_else(|| Error::DeviceNotFound(not_found.into()))
}

fn from_handle<E>(
    handle: DeviceHandle<GlobalContext>,
    select: impl FnOnce(&ConfigDescriptor) -> Result<(u8, E)>,
) -> Result<ClaimedUsb<E>> {
    let device = handle.device();
    let descriptor = device
        .device_descriptor()
        .map_err(|err| map_rusb(err, "USB descriptor"))?;
    // Index 0 rather than the active descriptor: on macOS a freshly opened
    // device with no bound driver is left in the Address state.
    let config = device
        .config_descriptor(0)
        .map_err(|err| map_rusb(err, "USB configuration"))?;

    let (interface, endpoints) = select(&config)?;
    prepare_and_claim(&handle, &config, interface)?;

    let manufacturer = descriptor
        .manufacturer_string_index()
        .and_then(|idx| handle.read_string_descriptor_ascii(idx).ok());
    let product = descriptor
        .product_string_index()
        .and_then(|idx| handle.read_string_descriptor_ascii(idx).ok());

    Ok(ClaimedUsb {
        handle,
        interface,
        manufacturer,
        product,
        endpoints,
    })
}

fn prepare_configuration(handle: &DeviceHandle<GlobalContext>, config: &ConfigDescriptor) {
    let needs_configuration = match handle.active_configuration() {
        Ok(active) => active != config.number(),
        Err(_) => true,
    };
    if needs_configuration {
        let _ = handle.set_active_configuration(config.number());
    }
}

fn prepare_and_claim(
    handle: &DeviceHandle<GlobalContext>,
    config: &ConfigDescriptor,
    interface: u8,
) -> Result<()> {
    if handle.kernel_driver_active(interface).unwrap_or(false) {
        let _ = handle.detach_kernel_driver(interface);
    }
    prepare_configuration(handle, config);
    handle
        .claim_interface(interface)
        .map_err(|err| map_rusb(err, "USB claim interface"))
}

fn claim_bulk_transport(handle: DeviceHandle<GlobalContext>) -> Result<UsbTransport> {
    let device = handle.device();
    let descriptor = device
        .device_descriptor()
        .map_err(|err| map_rusb(err, "USB descriptor"))?;
    let config = device
        .config_descriptor(0)
        .map_err(|err| map_rusb(err, "USB configuration"))?;

    let candidates = list_bulk_interfaces(&config);
    if candidates.is_empty() {
        return Err(Error::DeviceNotFound(
            "Missing bulk IN/OUT endpoints".into(),
        ));
    }

    prepare_configuration(&handle, &config);

    let mut last_err = None;
    for candidate in candidates {
        if handle.kernel_driver_active(candidate.interface).unwrap_or(false) {
            let _ = handle.detach_kernel_driver(candidate.interface);
        }
        match handle.claim_interface(candidate.interface) {
            Ok(()) => {
                let manufacturer = descriptor
                    .manufacturer_string_index()
                    .and_then(|idx| handle.read_string_descriptor_ascii(idx).ok());
                let product = descriptor
                    .product_string_index()
                    .and_then(|idx| handle.read_string_descriptor_ascii(idx).ok());
                return Ok(UsbTransport {
                    handle: Some(handle),
                    interface: candidate.interface,
                    endpoints: candidate.endpoints,
                    manufacturer,
                    product,
                });
            }
            Err(err) => last_err = Some(map_rusb(err, "USB claim interface")),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        Error::DeviceAccessDenied("USB claim interface failed".into())
    }))
}

/// Low-level USB bulk transport, matching felica-rs `UsbTransport`.
pub struct UsbTransport {
    handle: Option<DeviceHandle<GlobalContext>>,
    interface: u8,
    pub endpoints: BulkEndpoints,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

impl UsbTransport {
    /// Opens a USB device by VID/PID, optionally pinned to a bus/address from enumeration.
    ///
    /// RC-S300/P is a composite device: interface 0 is CCID (Sony PC/SC) and
    /// interface 1 is vendor/WinUSB. The CCID interface is claimed by the
    /// official driver, so this prefers the WinUSB bulk interface and falls
    /// back across every matching USB device.
    pub fn open(vendor_id: u16, product_id: u16, location: Option<(u8, u8)>) -> Result<Self> {
        let devices = rusb::devices().map_err(|err| map_rusb(err, "USB enumerate"))?;
        let mut last_err = None;
        let mut found = false;
        for device in devices.iter() {
            if let Some((bus, address)) = location {
                if device.bus_number() != bus || device.address() != address {
                    continue;
                }
            }
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if desc.vendor_id() != vendor_id || desc.product_id() != product_id {
                continue;
            }
            found = true;
            match device.open() {
                Ok(handle) => match claim_bulk_transport(handle) {
                    Ok(transport) => return Ok(transport),
                    Err(err) => last_err = Some(err),
                },
                Err(err) => last_err = Some(map_rusb(err, "USB open")),
            }
        }
        if !found {
            return Err(Error::DeviceNotFound(format!(
                "Device {vendor_id:04X}:{product_id:04X} not found"
            )));
        }
        Err(last_err.unwrap_or_else(|| {
            Error::DeviceAccessDenied(format!(
                "Device {vendor_id:04X}:{product_id:04X} could not be claimed"
            ))
        }))
    }

    /// Writes raw bytes to the bulk OUT endpoint. A zero-length packet is
    /// appended when the payload is an exact multiple of the max packet size.
    pub fn write(&mut self, data: &[u8], timeout: Duration) -> Result<usize> {
        let handle = self.handle.as_mut().ok_or(Error::DeviceDisconnected)?;
        let written = handle
            .write_bulk(self.endpoints.out_ep, data, timeout)
            .map_err(|err| map_rusb(err, "USB bulk write"))?;
        if !data.is_empty()
            && self.endpoints.max_packet_size > 0
            && data.len() as u16 % self.endpoints.max_packet_size == 0
        {
            let _ = handle
                .write_bulk(self.endpoints.out_ep, &[], timeout)
                .map_err(|err| map_rusb(err, "USB bulk ZLP"))?;
        }
        Ok(written)
    }

    /// Reads raw bytes from the bulk IN endpoint.
    pub fn read(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize> {
        let handle = self.handle.as_mut().ok_or(Error::DeviceDisconnected)?;
        handle
            .read_bulk(self.endpoints.in_ep, buf, timeout)
            .map_err(|err| map_rusb(err, "USB bulk read"))
    }

    /// Reads one bulk packet into a newly allocated buffer.
    pub fn read_packet(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let mut buffer = [0u8; 512];
        let n = self.read(&mut buffer, timeout)?;
        if n == 0 {
            return Err(Error::CommunicationError(
                "USB bulk read returned zero".into(),
            ));
        }
        Ok(buffer[..n].to_vec())
    }

    /// Releases the interface and closes the device handle.
    pub fn close(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            let _ = handle.release_interface(self.interface);
        }
        Ok(())
    }
}

impl Drop for UsbTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// USB control-OUT / interrupt-IN transport used by the RC-S320.
pub struct InterruptTransport {
    handle: Option<DeviceHandle<GlobalContext>>,
    interrupt_ep_in: u8,
    interface: u8,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

fn select_interrupt_in(config: &ConfigDescriptor) -> Result<(u8, u8)> {
    for interface_desc in config.interfaces() {
        for descriptor in interface_desc.descriptors() {
            for endpoint in descriptor.endpoint_descriptors() {
                if endpoint.transfer_type() == TransferType::Interrupt
                    && endpoint.direction() == Direction::In
                {
                    return Ok((descriptor.interface_number(), endpoint.address()));
                }
            }
        }
    }
    Err(Error::DeviceNotFound(
        "Missing interrupt IN endpoint".into(),
    ))
}

impl InterruptTransport {
    pub fn open(
        vendor_id: u16,
        product_id: u16,
        location: Option<(u8, u8)>,
        not_found: &str,
    ) -> Result<Self> {
        let claimed = open_usb_interface(
            vendor_id,
            product_id,
            location,
            not_found,
            select_interrupt_in,
        )?;
        Ok(Self {
            handle: Some(claimed.handle),
            interrupt_ep_in: claimed.endpoints,
            interface: claimed.interface,
            manufacturer: claimed.manufacturer,
            product: claimed.product,
        })
    }

    pub fn write_control(&mut self, data: &[u8], timeout: Duration) -> Result<usize> {
        let handle = self.handle.as_mut().ok_or(Error::DeviceDisconnected)?;
        let request_type = rusb::request_type(
            Direction::Out,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );
        handle
            .write_control(request_type, 0, 0, 0, data, timeout)
            .map_err(|err| map_rusb(err, "USB control write"))
    }

    pub fn read_interrupt(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let handle = self.handle.as_mut().ok_or(Error::DeviceDisconnected)?;
        let mut buffer = [0u8; 256];
        let len = handle
            .read_interrupt(self.interrupt_ep_in, &mut buffer, timeout)
            .map_err(|err| map_rusb(err, "USB interrupt read"))?;
        if len == 0 {
            return Err(Error::CommunicationError(
                "USB interrupt read returned zero".into(),
            ));
        }
        Ok(buffer[..len].to_vec())
    }

    pub fn close(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            let _ = handle.release_interface(self.interface);
        }
        Ok(())
    }
}

impl Drop for InterruptTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_specific_bulk_is_preferred_over_ccid() {
        assert!(bulk_preference(USB_CLASS_VENDOR) < bulk_preference(USB_CLASS_CCID));
        assert!(bulk_preference(USB_CLASS_VENDOR) < bulk_preference(0x00));
    }
}
