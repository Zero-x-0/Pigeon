use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use usb_x_core::device::{DeviceId, DeviceInfo};
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::types::{BusType, SystemDiskRisk};
use crate::traits::StorageBackend;

pub struct VirtualDriveBackend {
    file_path: String,
    device_info: DeviceInfo,
}

impl VirtualDriveBackend {
    pub fn open_or_create(path_str: &str, size_bytes: u64) -> Result<Self> {
        let path = Path::new(path_str);
        if !path.exists() {
            let file = File::create(path).map_err(|e| UsbXError::Io {
                operation: "create_virtual_drive".to_string(),
                message: e.to_string(),
            })?;
            file.set_len(size_bytes).map_err(|e| UsbXError::Io {
                operation: "set_virtual_drive_len".to_string(),
                message: e.to_string(),
            })?;
        }

        let metadata = std::fs::metadata(path).map_err(|e| UsbXError::Io {
            operation: "metadata_virtual_drive".to_string(),
            message: e.to_string(),
        })?;

        let id = DeviceId::new(format!("vdisk:{}", path_str));
        let device_info = DeviceInfo {
            id,
            display_name: format!("Virtual Disk ({})", path.file_name().unwrap_or_default().to_string_lossy()),
            model: "Virtual Disk File".to_string(),
            vendor: "USB X VDisk".to_string(),
            size_bytes: metadata.len(),
            logical_sector_size: 512,
            bus_type: BusType::VirtualFile,
            removable: true,
            system_risk: SystemDiskRisk::SafeRemovable,
            partitions: vec![],
            serial_redacted: None,
            device_path: path_str.to_string(),
        };

        Ok(Self {
            file_path: path_str.to_string(),
            device_info,
        })
    }
}

impl StorageBackend for VirtualDriveBackend {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>> {
        Ok(vec![self.device_info.clone()])
    }

    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo> {
        if id == &self.device_info.id {
            Ok(self.device_info.clone())
        } else {
            Err(UsbXError::DeviceNotFound(id.as_str().to_string()))
        }
    }

    fn unmount_and_lock(&self, _id: &DeviceId) -> Result<()> {
        Ok(())
    }

    fn write_sectors(&mut self, _id: &DeviceId, offset: u64, data: &[u8]) -> Result<usize> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.file_path)
            .map_err(|e| UsbXError::Io {
                operation: "open_virtual_drive_write".to_string(),
                message: e.to_string(),
            })?;

        file.seek(SeekFrom::Start(offset)).map_err(|e| UsbXError::Io {
            operation: "seek_virtual_drive".to_string(),
            message: e.to_string(),
        })?;

        file.write_all(data).map_err(|e| UsbXError::Io {
            operation: "write_virtual_drive".to_string(),
            message: e.to_string(),
        })?;

        Ok(data.len())
    }

    fn read_sectors(&mut self, _id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let mut file = File::open(&self.file_path).map_err(|e| UsbXError::Io {
            operation: "open_virtual_drive_read".to_string(),
            message: e.to_string(),
        })?;

        file.seek(SeekFrom::Start(offset)).map_err(|e| UsbXError::Io {
            operation: "seek_virtual_drive".to_string(),
            message: e.to_string(),
        })?;

        file.read_exact(buf).map_err(|e| UsbXError::Io {
            operation: "read_virtual_drive".to_string(),
            message: e.to_string(),
        })?;

        Ok(buf.len())
    }

    fn flush(&mut self, _id: &DeviceId) -> Result<()> {
        let file = OpenOptions::new().write(true).open(&self.file_path).map_err(|e| UsbXError::Io {
            operation: "flush_virtual_drive".to_string(),
            message: e.to_string(),
        })?;
        file.sync_all().map_err(|e| UsbXError::Io {
            operation: "sync_virtual_drive".to_string(),
            message: e.to_string(),
        })?;
        Ok(())
    }

    fn eject(&self, _id: &DeviceId) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_drive_create_read_write() {
        let temp_dir = std::env::temp_dir();
        let test_img = temp_dir.join("usb_x_test_vdisk.img");
        let test_img_str = test_img.to_string_lossy().to_string();

        let _ = std::fs::remove_file(&test_img);

        let mut vdisk = VirtualDriveBackend::open_or_create(&test_img_str, 64 * 1024).expect("create vdisk");
        let devs = vdisk.enumerate_devices().expect("enumerate");
        assert_eq!(devs.len(), 1);
        assert_eq!(devs[0].size_bytes, 64 * 1024);

        let dev_id = devs[0].id.clone();
        let payload = [0xAAu8; 512];
        vdisk.write_sectors(&dev_id, 0, &payload).expect("write sectors");
        vdisk.flush(&dev_id).expect("flush");

        let mut read_back = [0u8; 512];
        vdisk.read_sectors(&dev_id, 0, &mut read_back).expect("read sectors");
        assert_eq!(read_back, payload);

        let _ = std::fs::remove_file(&test_img);
    }
}

