use std::ffi::CStr;
use std::ptr;
use usb_x_core::device::{DeviceId, DeviceInfo, PartitionInfo};
use usb_x_core::errors::Result;
use usb_x_core::types::{BusType, SystemDiskRisk};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Ioctl::{
    DISK_GEOMETRY_EX, IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const OPEN_EXISTING: u32 = 3;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;

#[repr(C)]
struct StoragePropertyQuery {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

#[repr(C)]
struct StorageDeviceDescriptorHeader {
    version: u32,
    size: u32,
    device_type: u8,
    device_type_modifier: u8,
    removable_media: u8,
    command_queueing: u8,
    vendor_id_offset: u32,
    product_id_offset: u32,
    product_revision_offset: u32,
    serial_number_offset: u32,
    bus_type: u32,
    raw_properties_length: u32,
}

extern "system" {
    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: HANDLE,
    ) -> HANDLE;
}

pub fn enumerate_windows_drives() -> Result<Vec<DeviceInfo>> {
    let mut devices = Vec::new();

    // Check PhysicalDrive0 through PhysicalDrive16
    for disk_idx in 0..16 {
        // ALWAYS exclude PhysicalDrive0: it is universally the host system OS boot drive
        if disk_idx == 0 {
            continue;
        }

        let path_str = format!("\\\\.\\PhysicalDrive{}", disk_idx);
        let path_wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

        unsafe {
            // Open handle with query metadata access (0 desired access allows querying geometry without admin lock)
            let handle = CreateFileW(
                path_wide.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                continue;
            }

            // 1. Query Geometry and Capacity via IOCTL_DISK_GET_DRIVE_GEOMETRY_EX
            let mut geom = std::mem::zeroed::<DISK_GEOMETRY_EX>();
            let mut bytes_returned: u32 = 0;
            let geom_success = DeviceIoControl(
                handle,
                IOCTL_DISK_GET_DRIVE_GEOMETRY_EX,
                ptr::null(),
                0,
                &mut geom as *mut _ as *mut _,
                std::mem::size_of::<DISK_GEOMETRY_EX>() as u32,
                &mut bytes_returned,
                ptr::null_mut(),
            );

            let disk_size = if geom_success != 0 {
                geom.DiskSize as u64
            } else {
                0
            };

            let sector_size = if geom_success != 0 && geom.Geometry.BytesPerSector > 0 {
                geom.Geometry.BytesPerSector
            } else {
                512
            };

            // 2. Query Storage Property (BusType, RemovableMedia flag, Vendor/Product string)
            let mut prop_query = std::mem::zeroed::<StoragePropertyQuery>();
            prop_query.property_id = 0; // StorageDeviceProperty
            prop_query.query_type = 0;  // PropertyStandardQuery

            let mut desc_buf = vec![0u8; 1024];
            let mut desc_returned = 0u32;
            let prop_success = DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &prop_query as *const _ as *const _,
                std::mem::size_of::<StoragePropertyQuery>() as u32,
                desc_buf.as_mut_ptr() as *mut _,
                desc_buf.len() as u32,
                &mut desc_returned,
                ptr::null_mut(),
            );

            CloseHandle(handle);

            if disk_size == 0 {
                continue;
            }

            let mut is_removable = false;
            let mut bus_type_code = 0u32;
            let mut vendor_str = String::new();
            let mut product_str = String::new();

            if prop_success != 0 && desc_returned >= std::mem::size_of::<StorageDeviceDescriptorHeader>() as u32 {
                let header = &*(desc_buf.as_ptr() as *const StorageDeviceDescriptorHeader);
                is_removable = header.removable_media != 0;
                bus_type_code = header.bus_type;

                // BusTypeUsb = 7, BusTypeSd = 12, BusTypeMmc = 13, BusTypeVirtual = 14
                if bus_type_code == 7 || bus_type_code == 12 || bus_type_code == 13 || bus_type_code == 14 {
                    is_removable = true;
                }

                if header.vendor_id_offset > 0 && (header.vendor_id_offset as usize) < desc_buf.len() {
                    let ptr = desc_buf.as_ptr().add(header.vendor_id_offset as usize);
                    if let Ok(s) = CStr::from_ptr(ptr as *const i8).to_str() {
                        vendor_str = s.trim().to_string();
                    }
                }

                if header.product_id_offset > 0 && (header.product_id_offset as usize) < desc_buf.len() {
                    let ptr = desc_buf.as_ptr().add(header.product_id_offset as usize);
                    if let Ok(s) = CStr::from_ptr(ptr as *const i8).to_str() {
                        product_str = s.trim().to_string();
                    }
                }
            }

            // 3. Query mounted drive letters and labels for this physical disk
            let volumes = super::disk_io::get_volumes_for_disk(disk_idx);

            // If drive has volume letters marked DRIVE_REMOVABLE (2), mark as removable
            for vol in &volumes {
                if vol.drive_type == 2 {
                    is_removable = true;
                }
            }

            // RULE: Exclude internal hard drives. ONLY show removable drives (USB sticks, memory cards, virtual drives).
            // Non-USB internal SATA, NVMe, IDE, SCSI, RAID fixed hard drives are strictly ignored.
            let is_internal_hard_drive = !is_removable
                && bus_type_code != 7
                && bus_type_code != 12
                && bus_type_code != 13
                && bus_type_code != 14;

            if is_internal_hard_drive {
                continue; // Do not show internal hard drives!
            }

            let bus_type = match bus_type_code {
                7 => BusType::Usb,
                12 | 13 => BusType::SdCard,
                14 | 15 => BusType::VirtualFile,
                17 => BusType::Nvme,
                11 => BusType::Sata,
                1 => BusType::Scsi,
                _ => if is_removable { BusType::Usb } else { BusType::Unknown },
            };

            let id = DeviceId::new(format!("PhysicalDrive{}", disk_idx));

            let device_label = if let Some(first_vol) = volumes.first() {
                if !first_vol.volume_label.is_empty() {
                    format!("{} ({}:)", first_vol.volume_label, first_vol.drive_letter)
                } else if !product_str.is_empty() {
                    format!("{} ({}:)", product_str, first_vol.drive_letter)
                } else {
                    format!("USB Drive ({}:)", first_vol.drive_letter)
                }
            } else if !product_str.is_empty() {
                if !vendor_str.is_empty() {
                    format!("{} {}", vendor_str, product_str)
                } else {
                    product_str.clone()
                }
            } else {
                format!("Disk {} · Removable USB Storage", disk_idx)
            };

            let model = if !product_str.is_empty() {
                if !vendor_str.is_empty() {
                    format!("{} {}", vendor_str, product_str)
                } else {
                    product_str
                }
            } else {
                format!("USB Mass Storage (PhysicalDrive{})", disk_idx)
            };

            let vendor = if !vendor_str.is_empty() {
                vendor_str
            } else {
                "Removable USB Media".to_string()
            };

            let partitions = volumes.iter().enumerate().map(|(p_idx, v)| {
                PartitionInfo {
                    number: (p_idx + 1) as u32,
                    mount_point: Some(format!("{}:\\", v.drive_letter)),
                    size_bytes: disk_size,
                    file_system: if v.file_system.is_empty() { None } else { Some(v.file_system.clone()) },
                    label: if v.volume_label.is_empty() { None } else { Some(v.volume_label.clone()) },
                }
            }).collect();

            devices.push(DeviceInfo {
                id,
                display_name: device_label,
                model,
                vendor,
                size_bytes: disk_size,
                logical_sector_size: sector_size,
                bus_type,
                removable: true,
                system_risk: SystemDiskRisk::SafeRemovable,
                partitions,
                serial_redacted: Some(format!("DISK{:02}****", disk_idx)),
                device_path: path_str,
            });
        }
    }

    Ok(devices)
}
