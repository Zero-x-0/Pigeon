pub mod c_bindings;
pub mod disk_io;
pub mod enumeration;

use usb_x_core::device::{DeviceId, DeviceInfo};
use usb_x_core::errors::{Result, UsbXError};
use crate::traits::StorageBackend;
use disk_io::LockedDiskSession;

pub struct WindowsStorageBackend {
    active_session: Option<LockedDiskSession>,
    active_device_id: Option<DeviceId>,
}

impl WindowsStorageBackend {
    pub fn new() -> Self {
        Self {
            active_session: None,
            active_device_id: None,
        }
    }

    fn parse_disk_number(path: &str) -> Option<u32> {
        let prefix = "\\\\.\\PhysicalDrive";
        if path.starts_with(prefix) {
            path[prefix.len()..].parse::<u32>().ok()
        } else {
            None
        }
    }
}

impl StorageBackend for WindowsStorageBackend {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>> {
        enumeration::enumerate_windows_drives()
    }

    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo> {
        let devs = self.enumerate_devices()?;
        devs.into_iter()
            .find(|d| &d.id == id)
            .ok_or_else(|| UsbXError::DeviceNotFound(id.as_str().to_string()))
    }

    fn unmount_and_lock(&self, id: &DeviceId) -> Result<()> {
        let dev = self.revalidate_device(id)?;
        if dev.system_risk.is_destructive_blocked() {
            return Err(UsbXError::SystemDiskProtection(format!(
                "Cannot lock or format host system drive: {}", dev.display_name
            )));
        }
        Ok(())
    }

    fn write_sectors(&mut self, id: &DeviceId, offset: u64, data: &[u8]) -> Result<usize> {
        let dev = self.revalidate_device(id)?;
        if dev.system_risk.is_destructive_blocked() {
            return Err(UsbXError::SystemDiskProtection(format!(
                "Cannot write to host system drive: {}", dev.display_name
            )));
        }

        // Ensure we have an active locked disk session for this device
        if self.active_session.is_none() || self.active_device_id.as_ref() != Some(id) {
            let disk_number = Self::parse_disk_number(&dev.device_path).ok_or_else(|| {
                UsbXError::DeviceNotFound(format!("Invalid Windows device path: {}", dev.device_path))
            })?;

            let session = disk_io::open_and_lock_physical_disk(disk_number)?;
            self.active_session = Some(session);
            self.active_device_id = Some(id.clone());
        }

        if let Some(ref session) = self.active_session {
            disk_io::write_sector_data(session, offset, data)
        } else {
            Err(UsbXError::DeviceNotFound("No active disk session".to_string()))
        }
    }

    fn read_sectors(&mut self, id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let dev = self.revalidate_device(id)?;

        if self.active_session.is_none() || self.active_device_id.as_ref() != Some(id) {
            let disk_number = Self::parse_disk_number(&dev.device_path).ok_or_else(|| {
                UsbXError::DeviceNotFound(format!("Invalid Windows device path: {}", dev.device_path))
            })?;

            let session = disk_io::open_and_lock_physical_disk(disk_number)?;
            self.active_session = Some(session);
            self.active_device_id = Some(id.clone());
        }

        if let Some(ref session) = self.active_session {
            disk_io::read_sector_data(session, offset, buf)
        } else {
            Err(UsbXError::DeviceNotFound("No active disk session".to_string()))
        }
    }

    fn flush(&mut self, _id: &DeviceId) -> Result<()> {
        if let Some(session) = self.active_session.take() {
            drop(session); // Dropping session calls FlushFileBuffers, unlocks volumes, and closes disk handle
        }
        self.active_device_id = None;
        Ok(())
    }

    fn eject(&self, _id: &DeviceId) -> Result<()> {
        Ok(())
    }
}
