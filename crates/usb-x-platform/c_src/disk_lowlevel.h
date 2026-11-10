#ifndef USB_X_DISK_LOWLEVEL_H
#define USB_X_DISK_LOWLEVEL_H

#include <windows.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Progress callback for formatting: percent (0-100), message string
typedef void (*CFormatCallback)(unsigned int percent, const char* message);

// 1. Physical Disk Handle Operations
int c_open_physical_disk(unsigned int disk_number, HANDLE* out_handle);
int c_close_disk_handle(HANDLE h_disk);
int c_flush_disk_buffers(HANDLE h_disk);
int c_update_disk_properties(HANDLE h_disk);

// 2. Volume Locking & Dismounting Operations
int c_lock_and_dismount_volume(const wchar_t* volume_path, HANDLE* out_handle);
int c_unlock_and_close_volume(HANDLE h_volume);

// 3. Raw Sector I/O
int c_write_disk_sectors(HANDLE h_disk, unsigned long long offset, const unsigned char* data, unsigned int length, unsigned int* bytes_written);
int c_read_disk_sectors(HANDLE h_disk, unsigned long long offset, unsigned char* data, unsigned int length, unsigned int* bytes_read);

// 4. Shell Refresh Notification
int c_notify_shell_media_change(const wchar_t* root_path);

// 5. Native Windows Volume Formatting via fmifs.dll
int c_format_volume_fmifs(
    const wchar_t* drive_root,    // e.g. L"F:\\"
    const wchar_t* fs_name,       // L"NTFS", L"FAT32", L"exFAT"
    const wchar_t* volume_label,  // L"USB_X_DRIVE"
    int quick_format,             // 1 = quick, 0 = full
    unsigned int cluster_size,    // e.g. 4096 or 0 for default
    CFormatCallback callback
);

// 6. Direct MBR Partition Creation & Deep Partition Cleaner
int c_create_mbr_partition(
    unsigned int disk_number,
    unsigned long long total_size_bytes,
    int is_fat32
);

int c_clean_and_repartition_disk(
    unsigned int disk_number,
    unsigned long long total_size_bytes,
    int is_fat32
);

// 7. Virtual Disk ISO Mounting via virtdisk.dll
int c_mount_iso_virtdisk(
    const wchar_t* iso_path,
    HANDLE* out_vdisk_handle,
    wchar_t* out_drive_root
);

int c_unmount_iso_virtdisk(HANDLE h_vdisk);

// 8. System Theme & Diagnostics
int c_is_system_dark_mode(void);
int c_get_windows_accent_color(unsigned int* out_abgr);
int c_get_windows_version_string(char* out_buf, unsigned int max_len);

#ifdef __cplusplus
}
#endif

#endif // USB_X_DISK_LOWLEVEL_H

