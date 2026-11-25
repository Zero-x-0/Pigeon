use usb_x_core::device::{DeviceId, DeviceInfo};
use usb_x_core::errors::{Result, UsbXError};
use crate::traits::StorageBackend;

pub struct MacOsStorageBackend;

impl MacOsStorageBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for MacOsStorageBackend {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(Vec::new())
    }

    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo> {
        Err(UsbXError::DeviceNotFound(id.as_str().to_string()))
    }

    fn unmount_and_lock(&self, _id: &DeviceId) -> Result<()> {
        Err(UsbXError::UnsupportedCapability(
            "Native macOS storage backend is not implemented".to_string(),
        ))
    }

    fn write_sectors(&mut self, _id: &DeviceId, _offset: u64, _data: &[u8]) -> Result<usize> {
        Err(UsbXError::UnsupportedCapability(
            "Native macOS storage backend is not implemented".to_string(),
        ))
    }

    fn read_sectors(&mut self, _id: &DeviceId, _offset: u64, _buf: &mut [u8]) -> Result<usize> {
        Err(UsbXError::UnsupportedCapability(
            "Native macOS storage backend is not implemented".to_string(),
        ))
    }

    fn flush(&mut self, _id: &DeviceId) -> Result<()> {
        Ok(())
    }

    fn eject(&self, _id: &DeviceId) -> Result<()> {
        Err(UsbXError::UnsupportedCapability(
            "Native macOS storage backend is not implemented".to_string(),
        ))
    }
}
