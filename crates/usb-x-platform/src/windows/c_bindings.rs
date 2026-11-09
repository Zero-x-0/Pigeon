use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_ulonglong};

pub type CFormatCallback = extern "C" fn(percent: c_uint, message: *const c_char);
pub type CHashProgressCallback = extern "C" fn(current_bytes: c_ulonglong, total_bytes: c_ulonglong);

extern "C" {
    // 1. Disk & Volume Low-Level
    pub fn c_open_physical_disk(disk_number: c_uint, out_handle: *mut *mut c_void) -> c_int;
    pub fn c_close_disk_handle(h_disk: *mut c_void) -> c_int;
    pub fn c_flush_disk_buffers(h_disk: *mut c_void) -> c_int;
    pub fn c_update_disk_properties(h_disk: *mut c_void) -> c_int;

    pub fn c_lock_and_dismount_volume(volume_path: *const u16, out_handle: *mut *mut c_void) -> c_int;
    pub fn c_unlock_and_close_volume(h_volume: *mut c_void) -> c_int;

    pub fn c_write_disk_sectors(
        h_disk: *mut c_void,
        offset: c_ulonglong,
        data: *const c_uchar,
        length: c_uint,
        bytes_written: *mut c_uint,
    ) -> c_int;

    pub fn c_read_disk_sectors(
        h_disk: *mut c_void,
        offset: c_ulonglong,
        data: *mut c_uchar,
        length: c_uint,
        bytes_read: *mut c_uint,
    ) -> c_int;

    pub fn c_notify_shell_media_change(root_path: *const u16) -> c_int;

    pub fn c_format_volume_fmifs(
        drive_root: *const u16,
        fs_name: *const u16,
        volume_label: *const u16,
        quick_format: c_int,
        cluster_size: c_uint,
        callback: Option<CFormatCallback>,
    ) -> c_int;

    pub fn c_create_mbr_partition(
        disk_number: c_uint,
        total_size_bytes: c_ulonglong,
        is_fat32: c_int,
    ) -> c_int;

    pub fn c_mount_iso_virtdisk(
        iso_path: *const u16,
        out_vdisk_handle: *mut *mut c_void,
        out_drive_root: *mut u16,
    ) -> c_int;

    pub fn c_unmount_iso_virtdisk(h_vdisk: *mut c_void) -> c_int;

    // 2. High-Speed Hashing
    pub fn c_fast_crc32(initial_crc: c_uint, data: *const c_uchar, length: usize) -> c_uint;
    pub fn c_fast_sha256(data: *const c_uchar, length: usize, output: *mut c_uchar);
    pub fn c_fast_md5(data: *const c_uchar, length: usize, output: *mut c_uchar);

    pub fn c_fast_hash_file(
        file_path: *const u16,
        sha256_out: *mut c_uchar,
        md5_out: *mut c_uchar,
        crc32_out: *mut c_uint,
        progress_cb: Option<CHashProgressCallback>,
        cancel_flag: *const c_int,
    ) -> c_int;

    // 3. System Theme & Diagnostics
    pub fn c_is_system_dark_mode() -> c_int;
    pub fn c_get_windows_accent_color(out_abgr: *mut c_uint) -> c_int;
    pub fn c_get_windows_version_string(out_buf: *mut c_char, max_len: c_uint) -> c_int;
}

