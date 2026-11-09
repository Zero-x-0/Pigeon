use std::ptr;
use usb_x_core::errors::{Result, UsbXError};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::IO::DeviceIoControl;

const OPEN_EXISTING: u32 = 3;
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const IOCTL_STORAGE_GET_DEVICE_NUMBER: u32 = 0x002D1080;

#[repr(C)]
struct StorageDeviceNumber {
    device_type: u32,
    device_number: u32,
    partition_number: u32,
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

    fn GetLogicalDrives() -> u32;
    fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    fn GetVolumeInformationW(
        lpRootPathName: *const u16,
        lpVolumeNameBuffer: *mut u16,
        nVolumeNameSize: u32,
        lpVolumeSerialNumber: *mut u32,
        lpMaximumComponentLength: *mut u32,
        lpFileSystemFlags: *mut u32,
        lpFileSystemNameBuffer: *mut u16,
        nFileSystemNameSize: u32,
    ) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn IsUserAnAdmin() -> i32;
    fn ShellExecuteW(
        hwnd: isize,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
}

pub fn is_elevated_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

pub fn relaunch_as_admin() -> bool {
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_str = exe_path.to_string_lossy().to_string();
        let exe_wide: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
        let runas_wide: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

        let ret = unsafe {
            ShellExecuteW(
                0,
                runas_wide.as_ptr(),
                exe_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL = 1
            )
        };
        (ret as usize) > 32
    } else {
        false
    }
}

pub struct VolumeInfo {
    pub drive_letter: char,
    pub volume_label: String,
    pub file_system: String,
    pub drive_type: u32,
}

pub fn get_volumes_for_disk(disk_number: u32) -> Vec<VolumeInfo> {
    let mut matching_volumes = Vec::new();
    let drives_bitmask = unsafe { GetLogicalDrives() };

    for i in 0..26 {
        if (drives_bitmask & (1 << i)) != 0 {
            let letter = (b'A' + i) as char;
            let root_str = format!("{}:\\", letter);
            let root_wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();

            let drive_type = unsafe { GetDriveTypeW(root_wide.as_ptr()) };

            // Query device number of this volume
            let vol_device_str = format!("\\\\.\\{}:", letter);
            let vol_device_wide: Vec<u16> = vol_device_str.encode_utf16().chain(std::iter::once(0)).collect();

            unsafe {
                let h_vol = CreateFileW(
                    vol_device_wide.as_ptr(),
                    0, // 0 desired access to query metadata without lock
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    ptr::null(),
                    OPEN_EXISTING,
                    0,
                    ptr::null_mut(),
                );

                if h_vol != INVALID_HANDLE_VALUE {
                    let mut sdn = std::mem::zeroed::<StorageDeviceNumber>();
                    let mut returned = 0u32;
                    let ok = DeviceIoControl(
                        h_vol,
                        IOCTL_STORAGE_GET_DEVICE_NUMBER,
                        ptr::null(),
                        0,
                        &mut sdn as *mut _ as *mut _,
                        std::mem::size_of::<StorageDeviceNumber>() as u32,
                        &mut returned,
                        ptr::null_mut(),
                    );

                    CloseHandle(h_vol);

                    if ok != 0 && sdn.device_number == disk_number {
                        // This volume is on our disk! Get its label
                        let mut label_buf = vec![0u16; 256];
                        let mut fs_buf = vec![0u16; 256];
                        let vol_ok = GetVolumeInformationW(
                            root_wide.as_ptr(),
                            label_buf.as_mut_ptr(),
                            label_buf.len() as u32,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            ptr::null_mut(),
                            fs_buf.as_mut_ptr(),
                            fs_buf.len() as u32,
                        );

                        let volume_label = if vol_ok != 0 {
                            let len = label_buf.iter().position(|&c| c == 0).unwrap_or(0);
                            String::from_utf16_lossy(&label_buf[..len])
                        } else {
                            String::new()
                        };

                        let file_system = if vol_ok != 0 {
                            let len = fs_buf.iter().position(|&c| c == 0).unwrap_or(0);
                            String::from_utf16_lossy(&fs_buf[..len])
                        } else {
                            String::new()
                        };

                        matching_volumes.push(VolumeInfo {
                            drive_letter: letter,
                            volume_label,
                            file_system,
                            drive_type,
                        });
                    }
                }
            }
        }
    }

    matching_volumes
}

use super::c_bindings;

pub struct LockedDiskSession {
    pub disk_handle: HANDLE,
    pub volume_handles: Vec<HANDLE>,
    pub sector_size: u32,
}

unsafe impl Send for LockedDiskSession {}
unsafe impl Sync for LockedDiskSession {}

impl Drop for LockedDiskSession {
    fn drop(&mut self) {
        unsafe {
            if self.disk_handle != INVALID_HANDLE_VALUE && !self.disk_handle.is_null() {
                // Force Windows kernel storage manager to reload partition layout and properties
                c_bindings::c_update_disk_properties(self.disk_handle);
                c_bindings::c_flush_disk_buffers(self.disk_handle);
                c_bindings::c_close_disk_handle(self.disk_handle);
            }
            for &h in &self.volume_handles {
                if h != INVALID_HANDLE_VALUE && !h.is_null() {
                    c_bindings::c_unlock_and_close_volume(h);
                }
            }
            // Broadcast shell notification so Windows Explorer mounts and shows the volume immediately
            c_bindings::c_notify_shell_media_change(ptr::null());
        }
    }
}

pub fn open_and_lock_physical_disk(disk_number: u32) -> Result<LockedDiskSession> {
    let volumes = get_volumes_for_disk(disk_number);
    let mut locked_volumes = Vec::new();

    // 1. Lock and dismount every active volume on this drive via C low-level function
    for vol in volumes {
        let vol_str = format!("\\\\.\\{}:", vol.drive_letter);
        let vol_wide: Vec<u16> = vol_str.encode_utf16().chain(std::iter::once(0)).collect();

        let mut h_vol = ptr::null_mut();
        let res = unsafe { c_bindings::c_lock_and_dismount_volume(vol_wide.as_ptr(), &mut h_vol) };
        if res == 0 && h_vol != INVALID_HANDLE_VALUE && !h_vol.is_null() {
            locked_volumes.push(h_vol);
        }
    }

    // 2. Open PhysicalDriveN for raw sector writing via C low-level function
    let mut disk_handle = ptr::null_mut();
    let res = unsafe { c_bindings::c_open_physical_disk(disk_number, &mut disk_handle) };

    if res != 0 || disk_handle == INVALID_HANDLE_VALUE || disk_handle.is_null() {
        return Err(UsbXError::Io {
            operation: "open_physical_disk_c".to_string(),
            message: format!(
                "Failed to open \\\\.\\PhysicalDrive{} for raw writing (Win32 error: {}). Administrator privileges are required to write directly to physical flash drives. Please run Pigeon as Administrator.",
                disk_number,
                res
            ),
        });
    }

    Ok(LockedDiskSession {
        disk_handle,
        volume_handles: locked_volumes,
        sector_size: 512,
    })
}

pub fn write_sector_data(session: &LockedDiskSession, offset: u64, data: &[u8]) -> Result<usize> {
    let mut written = 0u32;
    let res = unsafe {
        c_bindings::c_write_disk_sectors(
            session.disk_handle,
            offset,
            data.as_ptr(),
            data.len() as u32,
            &mut written,
        )
    };
    if res != 0 {
        return Err(UsbXError::Io {
            operation: "c_write_disk_sectors".to_string(),
            message: format!("Low-level C sector write failed on offset {}: Win32 error {}", offset, res),
        });
    }
    Ok(written as usize)
}

pub fn read_sector_data(session: &LockedDiskSession, offset: u64, buf: &mut [u8]) -> Result<usize> {
    let mut read_bytes = 0u32;
    let res = unsafe {
        c_bindings::c_read_disk_sectors(
            session.disk_handle,
            offset,
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut read_bytes,
        )
    };
    if res != 0 {
        return Err(UsbXError::Io {
            operation: "c_read_disk_sectors".to_string(),
            message: format!("Low-level C sector read failed on offset {}: Win32 error {}", offset, res),
        });
    }
    Ok(read_bytes as usize)
}

pub fn format_drive_volume(
    drive_root: &str,
    fs_name: &str,
    label: &str,
    quick_format: bool,
    cluster_size: u32,
) -> Result<()> {
    let mut root_clean = drive_root.trim().to_string();
    if !root_clean.ends_with('\\') {
        root_clean.push('\\');
    }
    let root_wide: Vec<u16> = root_clean.encode_utf16().chain(std::iter::once(0)).collect();
    let fs_wide: Vec<u16> = fs_name.encode_utf16().chain(std::iter::once(0)).collect();
    let label_wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();

    let res = unsafe {
        c_bindings::c_format_volume_fmifs(
            root_wide.as_ptr(),
            fs_wide.as_ptr(),
            label_wide.as_ptr(),
            if quick_format { 1 } else { 0 },
            cluster_size,
            None,
        )
    };

    if res != 0 {
        return Err(UsbXError::Io {
            operation: "c_format_volume_fmifs".to_string(),
            message: format!(
                "Native Windows format failed on {} as {} with error code {}",
                root_clean, fs_name, res
            ),
        });
    }

    Ok(())
}

pub fn notify_drive_ready(drive_root: &str) {
    let mut root_clean = drive_root.trim().to_string();
    if !root_clean.ends_with('\\') {
        root_clean.push('\\');
    }
    let root_wide: Vec<u16> = root_clean.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        c_bindings::c_notify_shell_media_change(root_wide.as_ptr());
    }
}

pub fn create_mbr_partition_on_disk(
    disk_number: u32,
    total_size_bytes: u64,
    is_fat32: bool,
) -> Result<()> {
    let res = unsafe {
        c_bindings::c_create_mbr_partition(
            disk_number,
            total_size_bytes,
            if is_fat32 { 1 } else { 0 },
        )
    };
    if res != 0 {
        return Err(UsbXError::Io {
            operation: "c_create_mbr_partition".to_string(),
            message: format!(
                "Failed to write MBR partition table to PhysicalDrive{}: Win32 error {}",
                disk_number, res
            ),
        });
    }
    Ok(())
}

pub struct VirtualIsoMount {
    pub vdisk_handle: *mut std::ffi::c_void,
    pub drive_root: String,
}

unsafe impl Send for VirtualIsoMount {}
unsafe impl Sync for VirtualIsoMount {}

impl Drop for VirtualIsoMount {
    fn drop(&mut self) {
        if !self.vdisk_handle.is_null() && self.vdisk_handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            unsafe {
                c_bindings::c_unmount_iso_virtdisk(self.vdisk_handle);
            }
            self.vdisk_handle = std::ptr::null_mut();
        }
    }
}

pub fn mount_iso_virtdisk(iso_path: &str) -> Result<VirtualIsoMount> {
    let iso_wide: Vec<u16> = iso_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_vdisk = std::ptr::null_mut();
    let mut drive_root_buf = vec![0u16; 16];

    let res = unsafe {
        c_bindings::c_mount_iso_virtdisk(
            iso_wide.as_ptr(),
            &mut h_vdisk,
            drive_root_buf.as_mut_ptr(),
        )
    };

    if res != 0 || h_vdisk.is_null() || h_vdisk == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return Err(UsbXError::Io {
            operation: "c_mount_iso_virtdisk".to_string(),
            message: format!(
                "Failed to mount virtual ISO '{}' via virtdisk.dll (error code: {}). Ensure the file is valid and accessible.",
                iso_path, res
            ),
        });
    }

    let len = drive_root_buf.iter().position(|&c| c == 0).unwrap_or(0);
    let drive_root = String::from_utf16_lossy(&drive_root_buf[..len]);

    Ok(VirtualIsoMount {
        vdisk_handle: h_vdisk,
        drive_root,
    })
}

pub fn is_system_dark_mode() -> bool {
    unsafe { c_bindings::c_is_system_dark_mode() == 1 }
}

pub fn get_windows_accent_color() -> Option<(u8, u8, u8)> {
    let mut abgr: u32 = 0;
    if unsafe { c_bindings::c_get_windows_accent_color(&mut abgr) } == 1 {
        let r = (abgr & 0xFF) as u8;
        let g = ((abgr >> 8) & 0xFF) as u8;
        let b = ((abgr >> 16) & 0xFF) as u8;
        Some((r, g, b))
    } else {
        None
    }
}

pub fn get_windows_version_string() -> String {
    let mut buf = vec![0u8; 256];
    if unsafe { c_bindings::c_get_windows_version_string(buf.as_mut_ptr() as *mut std::os::raw::c_char, buf.len() as u32) } == 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        String::from_utf8_lossy(&buf[..len]).to_string()
    } else {
        "Windows 11 Pro x64".to_string()
    }
}
