pub mod traits;
pub mod virtual_drive;
pub mod file_dialog;

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

pub use traits::StorageBackend;
pub use virtual_drive::VirtualDriveBackend;
pub use file_dialog::{has_native_file_dialog, open_file_dialog, save_file_dialog};

#[cfg(windows)]
pub use windows::disk_io::{
    create_mbr_partition_on_disk, format_drive_volume, get_volumes_for_disk, get_windows_accent_color,
    get_windows_version_string, is_elevated_admin, is_system_dark_mode, mount_iso_virtdisk,
    notify_drive_ready, relaunch_as_admin, VirtualIsoMount, VolumeInfo,
};
#[cfg(windows)]
pub use windows::c_bindings;

#[cfg(target_os = "linux")]
pub use linux::{
    get_system_accent_color, get_system_version_string, is_elevated_admin,
    is_system_dark_mode, relaunch_as_admin,
};

#[cfg(target_os = "linux")]
pub fn get_windows_accent_color() -> Option<(u8, u8, u8)> {
    linux::get_system_accent_color()
}

#[cfg(target_os = "linux")]
pub fn get_windows_version_string() -> String {
    linux::get_system_version_string()
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn is_elevated_admin() -> bool {
    true
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn relaunch_as_admin() -> bool {
    false
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn is_system_dark_mode() -> bool {
    false
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn get_windows_accent_color() -> Option<(u8, u8, u8)> {
    None
}

#[cfg(all(not(windows), not(target_os = "linux")))]
pub fn get_windows_version_string() -> String {
    "Generic OS".to_string()
}

pub fn create_default_backend() -> Box<dyn StorageBackend> {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsStorageBackend::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxStorageBackend::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsStorageBackend::new())
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        panic!("Unsupported operating system");
    }
}
