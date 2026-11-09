use usb_x_core::device::{DeviceId, DeviceInfo};
use usb_x_core::errors::Result;

pub trait StorageBackend: Send + Sync {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>>;
    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo>;
    fn unmount_and_lock(&self, id: &DeviceId) -> Result<()>;
    fn write_sectors(&mut self, id: &DeviceId, offset: u64, data: &[u8]) -> Result<usize>;
    fn read_sectors(&mut self, id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize>;
    fn flush(&mut self, id: &DeviceId) -> Result<()>;
    fn eject(&self, id: &DeviceId) -> Result<()>;
}

