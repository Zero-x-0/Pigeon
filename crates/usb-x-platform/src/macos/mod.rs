use usb_x_core::device::{DeviceId, DeviceInfo};
use usb_x_core::errors::Result;
use crate::mock::MockStorageBackend;
use crate::traits::StorageBackend;

pub struct MacOsStorageBackend {
    fallback: MockStorageBackend,
}

impl MacOsStorageBackend {
    pub fn new() -> Self {
        Self {
            fallback: MockStorageBackend::new(),
        }
    }
}

impl StorageBackend for MacOsStorageBackend {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.fallback.enumerate_devices()
    }

    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo> {
        self.fallback.revalidate_device(id)
    }

    fn unmount_and_lock(&self, id: &DeviceId) -> Result<()> {
        self.fallback.unmount_and_lock(id)
    }

    fn write_sectors(&mut self, id: &DeviceId, offset: u64, data: &[u8]) -> Result<usize> {
        self.fallback.write_sectors(id, offset, data)
    }

    fn read_sectors(&mut self, id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize> {
        self.fallback.read_sectors(id, offset, buf)
    }

    fn flush(&mut self, id: &DeviceId) -> Result<()> {
        self.fallback.flush(id)
    }

    fn eject(&self, id: &DeviceId) -> Result<()> {
        self.fallback.eject(id)
    }
}

