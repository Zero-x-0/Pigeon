#include "disk_lowlevel.h"
#include <stdio.h>
#include <stdlib.h>
#include <shlobj.h>
#include <winioctl.h>
#include <virtdisk.h>

#ifndef IOCTL_DISK_UPDATE_PROPERTIES
#define IOCTL_DISK_UPDATE_PROPERTIES 0x00070140
#endif

#ifndef IOCTL_STORAGE_CHECK_VERIFY
#define IOCTL_STORAGE_CHECK_VERIFY 0x002D4800
#endif

#ifndef FSCTL_LOCK_VOLUME
#define FSCTL_LOCK_VOLUME 0x00090018
#endif

#ifndef FSCTL_DISMOUNT_VOLUME
#define FSCTL_DISMOUNT_VOLUME 0x00090020
#endif

#ifndef FSCTL_UNLOCK_VOLUME
#define FSCTL_UNLOCK_VOLUME 0x0009001C
#endif

// 1. Open PhysicalDisk
int c_open_physical_disk(unsigned int disk_number, HANDLE* out_handle) {
    if (!out_handle) return -1;
    *out_handle = INVALID_HANDLE_VALUE;

    wchar_t disk_path[64];
    swprintf(disk_path, 64, L"\\\\.\\PhysicalDrive%u", disk_number);

    HANDLE h = CreateFileW(
        disk_path,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH,
        NULL
    );

    if (h == INVALID_HANDLE_VALUE) {
        return (int)GetLastError();
    }

    *out_handle = h;
    return 0; // Success
}

int c_close_disk_handle(HANDLE h_disk) {
    if (h_disk != NULL && h_disk != INVALID_HANDLE_VALUE) {
        CloseHandle(h_disk);
        return 0;
    }
    return -1;
}

int c_flush_disk_buffers(HANDLE h_disk) {
    if (h_disk == NULL || h_disk == INVALID_HANDLE_VALUE) return -1;
    if (FlushFileBuffers(h_disk)) {
        return 0;
    }
    return (int)GetLastError();
}

int c_update_disk_properties(HANDLE h_disk) {
    if (h_disk == NULL || h_disk == INVALID_HANDLE_VALUE) return -1;
    DWORD bytes_returned = 0;

    // First flush
    FlushFileBuffers(h_disk);

    // Tell kernel partition manager to reload layout and dismantle old volume structures
    BOOL res1 = DeviceIoControl(
        h_disk,
        IOCTL_DISK_UPDATE_PROPERTIES,
        NULL,
        0,
        NULL,
        0,
        &bytes_returned,
        NULL
    );

    // Also send check verify
    DeviceIoControl(
        h_disk,
        IOCTL_STORAGE_CHECK_VERIFY,
        NULL,
        0,
        NULL,
        0,
        &bytes_returned,
        NULL
    );

    return res1 ? 0 : (int)GetLastError();
}

// 2. Volume lock & dismount
int c_lock_and_dismount_volume(const wchar_t* volume_path, HANDLE* out_handle) {
    if (!volume_path || !out_handle) return -1;
    *out_handle = INVALID_HANDLE_VALUE;

    HANDLE h = CreateFileW(
        volume_path,
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        NULL,
        OPEN_EXISTING,
        0,
        NULL
    );

    if (h == INVALID_HANDLE_VALUE) {
        return (int)GetLastError();
    }

    DWORD returned = 0;
    // Lock volume
    DeviceIoControl(h, FSCTL_LOCK_VOLUME, NULL, 0, NULL, 0, &returned, NULL);
    // Dismount volume
    DeviceIoControl(h, FSCTL_DISMOUNT_VOLUME, NULL, 0, NULL, 0, &returned, NULL);

    *out_handle = h;
    return 0;
}

int c_unlock_and_close_volume(HANDLE h_volume) {
    if (h_volume != NULL && h_volume != INVALID_HANDLE_VALUE) {
        DWORD returned = 0;
        DeviceIoControl(h_volume, FSCTL_UNLOCK_VOLUME, NULL, 0, NULL, 0, &returned, NULL);
        CloseHandle(h_volume);
        return 0;
    }
    return -1;
}

// 3. Raw sector read / write
int c_write_disk_sectors(HANDLE h_disk, unsigned long long offset, const unsigned char* data, unsigned int length, unsigned int* bytes_written) {
    if (h_disk == NULL || h_disk == INVALID_HANDLE_VALUE || !data || length == 0) return -1;
    if (bytes_written) *bytes_written = 0;

    LARGE_INTEGER li;
    li.QuadPart = (LONGLONG)offset;
    if (!SetFilePointerEx(h_disk, li, NULL, FILE_BEGIN)) {
        return (int)GetLastError();
    }

    // Windows requires 512-byte sector alignment
    unsigned int remainder = length % 512;
    if (remainder == 0) {
        DWORD written = 0;
        if (!WriteFile(h_disk, data, length, &written, NULL)) {
            return (int)GetLastError();
        }
        if (bytes_written) *bytes_written = (unsigned int)written;
        return 0;
    } else {
        // Pad to 512 boundary
        unsigned int padded_len = length + (512 - remainder);
        unsigned char* padded_buf = (unsigned char*)malloc(padded_len);
        if (!padded_buf) return -2;

        memcpy(padded_buf, data, length);
        memset(padded_buf + length, 0, padded_len - length);

        DWORD written = 0;
        BOOL ok = WriteFile(h_disk, padded_buf, padded_len, &written, NULL);
        free(padded_buf);

        if (!ok) {
            return (int)GetLastError();
        }
        if (bytes_written) *bytes_written = length;
        return 0;
    }
}

int c_read_disk_sectors(HANDLE h_disk, unsigned long long offset, unsigned char* data, unsigned int length, unsigned int* bytes_read) {
    if (h_disk == NULL || h_disk == INVALID_HANDLE_VALUE || !data || length == 0) return -1;
    if (bytes_read) *bytes_read = 0;

    LARGE_INTEGER li;
    li.QuadPart = (LONGLONG)offset;
    if (!SetFilePointerEx(h_disk, li, NULL, FILE_BEGIN)) {
        return (int)GetLastError();
    }

    DWORD read = 0;
    if (!ReadFile(h_disk, data, length, &read, NULL)) {
        return (int)GetLastError();
    }
    if (bytes_read) *bytes_read = (unsigned int)read;
    return 0;
}

// 4. Shell Refresh Notification
int c_notify_shell_media_change(const wchar_t* root_path) {
    if (root_path) {
        SHChangeNotify(SHCNE_MEDIAINSERTED, SHCNF_PATHW, root_path, NULL);
        SHChangeNotify(SHCNE_DRIVEADD, SHCNF_PATHW, root_path, NULL);
        SHChangeNotify(SHCNE_UPDATEDIR, SHCNF_PATHW, root_path, NULL);
    } else {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_FLUSH, NULL, NULL);
    }
    return 0;
}

// 5. Windows Native Formatting via fmifs.dll
typedef enum _FMIFS_PACKET_TYPE {
    FmIfsPercentCompleted = 0,
    FmIfsFormatReport = 1,
    FmIfsInsertDisk = 2,
    FmIfsIncompatibleFileSystem = 3,
    FmIfsFormattingDestination = 4,
    FmIfsIncompatibleMedia = 5,
    FmIfsAccessDenied = 6,
    FmIfsMediaWriteProtected = 7,
    FmIfsCantLock = 8,
    FmIfsCantQuickFormat = 9,
    FmIfsIoError = 10,
    FmIfsFinished = 11,
    FmIfsBadLabel = 12,
    FmIfsCheckOnReboot = 13,
    FmIfsTextMessage = 14,
    FmIfsHiddenStatus = 15
} FMIFS_PACKET_TYPE;

typedef struct {
    BOOLEAN Success;
} FMIFS_FINISHED_INFORMATION;

typedef struct {
    ULONG PercentCompleted;
} FMIFS_PERCENT_COMPLETE_INFORMATION;

typedef BOOLEAN (WINAPI *PFMIFSCALLBACK)(
    FMIFS_PACKET_TYPE PacketType,
    DWORD PacketLength,
    PVOID PacketData
);

typedef void (WINAPI *PFORMATEX)(
    PWSTR DriveRoot,
    DWORD MediaType,
    PWSTR FileSystem,
    PWSTR Label,
    BOOL QuickFormat,
    DWORD ClusterSize,
    PFMIFSCALLBACK Callback
);

static CFormatCallback g_user_format_cb = NULL;
static int g_format_success = 0;

static BOOLEAN WINAPI InternalFormatCallback(
    FMIFS_PACKET_TYPE PacketType,
    DWORD PacketLength,
    PVOID PacketData
) {
    (void)PacketLength;
    switch (PacketType) {
        case FmIfsPercentCompleted: {
            if (PacketData && g_user_format_cb) {
                FMIFS_PERCENT_COMPLETE_INFORMATION* info = (FMIFS_PERCENT_COMPLETE_INFORMATION*)PacketData;
                g_user_format_cb((unsigned int)info->PercentCompleted, "Formatting file system structures...");
            }
            break;
        }
        case FmIfsFinished: {
            if (PacketData) {
                FMIFS_FINISHED_INFORMATION* info = (FMIFS_FINISHED_INFORMATION*)PacketData;
                g_format_success = info->Success ? 1 : 0;
                if (g_user_format_cb) {
                    g_user_format_cb(100, info->Success ? "Format completed successfully" : "Format failed");
                }
            } else {
                g_format_success = 1;
            }
            break;
        }
        case FmIfsAccessDenied: {
            if (g_user_format_cb) g_user_format_cb(0, "Access denied to volume");
            g_format_success = 0;
            return FALSE;
        }
        case FmIfsMediaWriteProtected: {
            if (g_user_format_cb) g_user_format_cb(0, "Media is write protected");
            g_format_success = 0;
            return FALSE;
        }
        case FmIfsCantLock: {
            if (g_user_format_cb) g_user_format_cb(0, "Volume in use or cannot be locked");
            g_format_success = 0;
            return FALSE;
        }
        case FmIfsCantQuickFormat: {
            if (g_user_format_cb) g_user_format_cb(0, "Cannot quick format this volume");
            g_format_success = 0;
            return FALSE;
        }
        case FmIfsIoError: {
            if (g_user_format_cb) g_user_format_cb(0, "I/O error encountered during formatting");
            g_format_success = 0;
            return FALSE;
        }
        case FmIfsBadLabel: {
            if (g_user_format_cb) g_user_format_cb(0, "Volume label is invalid or too long");
            g_format_success = 0;
            return FALSE;
        }
        default:
            break;
    }
    return TRUE; // Continue
}

int c_format_volume_fmifs(
    const wchar_t* drive_root,
    const wchar_t* fs_name,
    const wchar_t* volume_label,
    int quick_format,
    unsigned int cluster_size,
    CFormatCallback callback
) {
    if (!drive_root || !fs_name) return -1;

    HMODULE hFmifs = LoadLibraryA("fmifs.dll");
    if (!hFmifs) {
        return -2;
    }

    FARPROC proc = GetProcAddress(hFmifs, "FormatEx");
    PFORMATEX pFormatEx = NULL;
    memcpy(&pFormatEx, &proc, sizeof(proc));
    if (!pFormatEx) {
        FreeLibrary(hFmifs);
        return -3;
    }

    g_user_format_cb = callback;
    g_format_success = 0;

    // Clean drive name: FormatEx strictly requires NO trailing backslash (e.g. L"E:" or L"\\\\?\\Volume{guid}")
    wchar_t root_clean[64];
    wcsncpy(root_clean, drive_root, 63);
    root_clean[63] = L'\0';
    size_t len = wcslen(root_clean);
    while (len > 0 && (root_clean[len - 1] == L'\\' || root_clean[len - 1] == L'/')) {
        root_clean[len - 1] = L'\0';
        len--;
    }

    // Attempt to resolve Volume GUID without trailing backslash
    wchar_t root_with_slash[16];
    swprintf(root_with_slash, 16, L"%s\\", root_clean);
    wchar_t vol_guid[MAX_PATH];
    const wchar_t* target_path = root_clean;
    if (GetVolumeNameForVolumeMountPointW(root_with_slash, vol_guid, MAX_PATH)) {
        size_t glen = wcslen(vol_guid);
        if (glen > 0 && vol_guid[glen - 1] == L'\\') {
            vol_guid[glen - 1] = L'\0';
        }
        target_path = vol_guid;
    }

    // If cluster size is 4096 on FAT32 volumes > 16GB, 4096 is invalid (exceeds max FAT32 clusters).
    // Passing 0 tells FormatEx to pick the optimal cluster size for the volume capacity.
    DWORD actual_cluster_size = (DWORD)cluster_size;

    // Attempt 1: Format using target_path and MediaType 0x0C (Removable)
    pFormatEx(
        (PWSTR)target_path,
        0x0C,
        (PWSTR)fs_name,
        (PWSTR)(volume_label ? volume_label : L""),
        quick_format ? TRUE : FALSE,
        actual_cluster_size,
        InternalFormatCallback
    );

    // Attempt 2: If GUID path failed or g_format_success is 0, retry directly with drive letter (e.g. "E:")
    if (!g_format_success && target_path != root_clean) {
        g_format_success = 0;
        pFormatEx(
            root_clean,
            0x0C,
            (PWSTR)fs_name,
            (PWSTR)(volume_label ? volume_label : L""),
            quick_format ? TRUE : FALSE,
            0, // Default cluster size
            InternalFormatCallback
        );
    }

    // Attempt 3: Retry with MediaType 0x0B (Fixed) and cluster size 0
    if (!g_format_success) {
        g_format_success = 0;
        pFormatEx(
            root_clean,
            0x0B,
            (PWSTR)fs_name,
            (PWSTR)(volume_label ? volume_label : L""),
            quick_format ? TRUE : FALSE,
            0, // Default cluster size
            InternalFormatCallback
        );
    }

    FreeLibrary(hFmifs);

    // Notify Windows Explorer that the drive formatting completed
    c_notify_shell_media_change(root_with_slash);

    return g_format_success ? 0 : -4;
}

// 6. Direct MBR Partition Creation
int c_create_mbr_partition(
    unsigned int disk_number,
    unsigned long long total_size_bytes,
    int is_fat32
) {
    if (total_size_bytes < (2048ULL * 512ULL + 1024ULL * 1024ULL)) {
        return -1; // Disk too small
    }

    HANDLE h_disk = INVALID_HANDLE_VALUE;
    int open_err = c_open_physical_disk(disk_number, &h_disk);
    if (open_err != 0 || h_disk == INVALID_HANDLE_VALUE) {
        return open_err != 0 ? open_err : -2;
    }

    // 1. Calculate geometry and sector count
    unsigned long long total_sectors = total_size_bytes / 512ULL;
    unsigned int starting_lba = 2048; // Standard 1MB offset alignment
    unsigned long long part_sectors_64 = (total_sectors > starting_lba) ? (total_sectors - starting_lba) : 0;
    unsigned int sector_count = (part_sectors_64 > 0xFFFFFFF0ULL) ? 0xFFFFFFFFU : (unsigned int)part_sectors_64;

    // 2. Build standard Sector 0 MBR
    unsigned char sector0[512];
    memset(sector0, 0, 512);

    // Standard x86 MBR bootloader code (loads active partition VBR to 0x7C00)
    static const unsigned char mbr_bootcode[] = {
        0x33, 0xC0, 0x8E, 0xD0, 0xBC, 0x00, 0x7C, 0xFB, 0x50, 0x07, 0x50, 0x1F, 0xFC, 0xBE, 0x1B, 0x7C,
        0xBF, 0x1B, 0x06, 0x50, 0x57, 0xB9, 0xE5, 0x01, 0xF3, 0xA4, 0xCB, 0xBD, 0xBE, 0x07, 0xB1, 0x04,
        0x38, 0x6E, 0x00, 0x7C, 0x09, 0x75, 0x13, 0x83, 0xC5, 0x10, 0xE2, 0xF4, 0xCD, 0x18, 0x8B, 0xF5,
        0x83, 0xC6, 0x10, 0x49, 0x74, 0x19, 0x38, 0x2C, 0x74, 0xF6, 0xA0, 0xB5, 0x07, 0xB4, 0x07, 0x8B,
        0xF0, 0xAC, 0x3C, 0x00, 0x74, 0xFC, 0xBB, 0x07, 0x00, 0xB4, 0x0E, 0xCD, 0x10, 0xEB, 0xF2, 0x88,
        0x4E, 0x10, 0xE8, 0x46, 0x00, 0x73, 0x2A, 0xFE, 0x46, 0x10, 0x80, 0x7E, 0x04, 0x0B, 0x74, 0x0B,
        0x80, 0x7E, 0x04, 0x0C, 0x74, 0x05, 0xA0, 0xB6, 0x07, 0x75, 0xD2, 0x80, 0x46, 0x02, 0x06, 0x83,
        0x46, 0x08, 0x06, 0x83, 0x56, 0x0A, 0x00, 0xE8, 0x21, 0x00, 0x73, 0x05, 0xA0, 0xB6, 0x07, 0xEB,
        0xBC, 0x81, 0x3E, 0xFE, 0x7D, 0x55, 0xAA, 0x75, 0xB4, 0xFF, 0x76, 0x00, 0xE8, 0x0D, 0x00, 0x75,
        0x17, 0xFA, 0xB0, 0xD1, 0xE6, 0x64, 0xE8, 0x0F, 0x00, 0xB0, 0xDF, 0xE6, 0x60, 0xE8, 0x07, 0x00,
        0xB0, 0xFF, 0xE6, 0x64, 0xE8, 0x00, 0x00, 0xC3
    };
    memcpy(sector0, mbr_bootcode, sizeof(mbr_bootcode));

    // Disk signature at offset 440 (0x1B8)
    sector0[440] = 0x55; // 'U'
    sector0[441] = 0x53; // 'S'
    sector0[442] = 0x42; // 'B'
    sector0[443] = 0x58; // 'X'

    // Partition Entry 1 at offset 446 (0x1BE)
    sector0[446] = 0x80; // Active / Bootable flag
    sector0[447] = 0x01; // Start Head
    sector0[448] = 0x01; // Start Sector
    sector0[449] = 0x00; // Start Cylinder
    sector0[450] = is_fat32 ? 0x0C : 0x07; // 0x0C = FAT32 LBA, 0x07 = NTFS/exFAT
    sector0[451] = 0xFE; // End Head
    sector0[452] = 0xFF; // End Sector
    sector0[453] = 0xFF; // End Cylinder

    // Starting LBA (little endian)
    sector0[454] = (unsigned char)(starting_lba & 0xFF);
    sector0[455] = (unsigned char)((starting_lba >> 8) & 0xFF);
    sector0[456] = (unsigned char)((starting_lba >> 16) & 0xFF);
    sector0[457] = (unsigned char)((starting_lba >> 24) & 0xFF);

    // Sector Count (little endian)
    sector0[458] = (unsigned char)(sector_count & 0xFF);
    sector0[459] = (unsigned char)((sector_count >> 8) & 0xFF);
    sector0[460] = (unsigned char)((sector_count >> 16) & 0xFF);
    sector0[461] = (unsigned char)((sector_count >> 24) & 0xFF);

    // MBR Boot Signature at 510-511
    sector0[510] = 0x55;
    sector0[511] = 0xAA;

    // 3. Write Sector 0 directly to PhysicalDrive
    unsigned int bytes_written = 0;
    int write_err = c_write_disk_sectors(h_disk, 0, sector0, 512, &bytes_written);
    if (write_err != 0) {
        c_close_disk_handle(h_disk);
        return write_err;
    }

    // 4. Also issue IOCTL_DISK_SET_DRIVE_LAYOUT_EX to ensure kernel partition table sync
    DWORD layout_size = sizeof(DRIVE_LAYOUT_INFORMATION_EX) + 3 * sizeof(PARTITION_INFORMATION_EX);
    DRIVE_LAYOUT_INFORMATION_EX* p_layout = (DRIVE_LAYOUT_INFORMATION_EX*)malloc(layout_size);
    if (p_layout) {
        memset(p_layout, 0, layout_size);
        p_layout->PartitionStyle = PARTITION_STYLE_MBR;
        p_layout->PartitionCount = 4;
        p_layout->Mbr.Signature = 0x58425355; // "USBX"

        p_layout->PartitionEntry[0].PartitionStyle = PARTITION_STYLE_MBR;
        p_layout->PartitionEntry[0].StartingOffset.QuadPart = (LONGLONG)starting_lba * 512ULL;
        p_layout->PartitionEntry[0].PartitionLength.QuadPart = (LONGLONG)sector_count * 512ULL;
        p_layout->PartitionEntry[0].PartitionNumber = 1;
        p_layout->PartitionEntry[0].RewritePartition = TRUE;
        p_layout->PartitionEntry[0].Mbr.PartitionType = is_fat32 ? 0x0C : 0x07;
        p_layout->PartitionEntry[0].Mbr.BootIndicator = TRUE;
        p_layout->PartitionEntry[0].Mbr.RecognizedPartition = TRUE;
        p_layout->PartitionEntry[0].Mbr.HiddenSectors = starting_lba;

        for (int i = 1; i < 4; i++) {
            p_layout->PartitionEntry[i].RewritePartition = TRUE;
        }

        DWORD returned = 0;
        DeviceIoControl(
            h_disk,
            IOCTL_DISK_SET_DRIVE_LAYOUT_EX,
            p_layout,
            layout_size,
            NULL,
            0,
            &returned,
            NULL
        );
        free(p_layout);
    }

    // 5. Flush and update properties
    c_flush_disk_buffers(h_disk);
    c_update_disk_properties(h_disk);
    c_close_disk_handle(h_disk);

    // 6. Notify Windows shell
    c_notify_shell_media_change(NULL);

    return 0;
}

// 7. Virtual Disk ISO Mounting via virtdisk.dll
typedef DWORD (WINAPI *PFN_OpenVirtualDisk)(
    PVIRTUAL_STORAGE_TYPE VirtualStorageType,
    PCWSTR Path,
    VIRTUAL_DISK_ACCESS_MASK VirtualDiskAccessMask,
    OPEN_VIRTUAL_DISK_FLAG Flags,
    POPEN_VIRTUAL_DISK_PARAMETERS Parameters,
    PHANDLE Handle
);

typedef DWORD (WINAPI *PFN_AttachVirtualDisk)(
    HANDLE VirtualDiskHandle,
    PSECURITY_DESCRIPTOR SecurityDescriptor,
    ATTACH_VIRTUAL_DISK_FLAG Flags,
    ULONG ProviderSpecificFlags,
    PATTACH_VIRTUAL_DISK_PARAMETERS Parameters,
    LPOVERLAPPED Overlapped
);

typedef DWORD (WINAPI *PFN_DetachVirtualDisk)(
    HANDLE VirtualDiskHandle,
    DETACH_VIRTUAL_DISK_FLAG Flags,
    ULONG ProviderSpecificFlags
);

int c_mount_iso_virtdisk(
    const wchar_t* iso_path,
    HANDLE* out_vdisk_handle,
    wchar_t* out_drive_root
) {
    if (!iso_path || !out_vdisk_handle || !out_drive_root) {
        return -1;
    }
    *out_vdisk_handle = INVALID_HANDLE_VALUE;
    out_drive_root[0] = L'\0';

    HMODULE hVirtdisk = LoadLibraryA("virtdisk.dll");
    if (!hVirtdisk) {
        return -2; // virtdisk.dll not available
    }

    PFN_OpenVirtualDisk pOpen = NULL;
    FARPROC procOpen = GetProcAddress(hVirtdisk, "OpenVirtualDisk");
    memcpy(&pOpen, &procOpen, sizeof(procOpen));

    PFN_AttachVirtualDisk pAttach = NULL;
    FARPROC procAttach = GetProcAddress(hVirtdisk, "AttachVirtualDisk");
    memcpy(&pAttach, &procAttach, sizeof(procAttach));

    PFN_DetachVirtualDisk pDetach = NULL;
    FARPROC procDetach = GetProcAddress(hVirtdisk, "DetachVirtualDisk");
    memcpy(&pDetach, &procDetach, sizeof(procDetach));

    if (!pOpen || !pAttach || !pDetach) {
        FreeLibrary(hVirtdisk);
        return -3;
    }

    // Snapshot existing logical drives mask
    DWORD drives_before = GetLogicalDrives();

    // 1. Open virtual disk (ISO)
    VIRTUAL_STORAGE_TYPE st;
    memset(&st, 0, sizeof(st));
    st.DeviceId = VIRTUAL_STORAGE_TYPE_DEVICE_ISO; // 1
    static const GUID s_vendor_ms = {
        0xEC984AEC, 0xA0F9, 0x47e9, { 0x90, 0x1F, 0x71, 0x41, 0x5A, 0x66, 0x34, 0x5B }
    };
    memcpy(&st.VendorId, &s_vendor_ms, sizeof(GUID));

    OPEN_VIRTUAL_DISK_PARAMETERS op;
    memset(&op, 0, sizeof(op));
    op.Version = OPEN_VIRTUAL_DISK_VERSION_1; // 1
    op.Version1.RWDepth = 1;

    HANDLE h_vdisk = INVALID_HANDLE_VALUE;
    DWORD status = pOpen(
        &st,
        iso_path,
        VIRTUAL_DISK_ACCESS_READ,
        OPEN_VIRTUAL_DISK_FLAG_NONE,
        &op,
        &h_vdisk
    );

    if (status != ERROR_SUCCESS || h_vdisk == INVALID_HANDLE_VALUE) {
        FreeLibrary(hVirtdisk);
        return (int)status;
    }

    // 2. Attach virtual disk (mount as read-only CD-ROM drive)
    ATTACH_VIRTUAL_DISK_PARAMETERS ap;
    memset(&ap, 0, sizeof(ap));
    ap.Version = ATTACH_VIRTUAL_DISK_VERSION_1; // 1

    status = pAttach(
        h_vdisk,
        NULL,
        ATTACH_VIRTUAL_DISK_FLAG_READ_ONLY,
        0,
        &ap,
        NULL
    );

    if (status != ERROR_SUCCESS) {
        CloseHandle(h_vdisk);
        FreeLibrary(hVirtdisk);
        return (int)status;
    }

    // 3. Poll for the newly mounted logical drive letter (up to 4 seconds)
    wchar_t assigned_letter = 0;
    for (int i = 0; i < 40; i++) {
        Sleep(100);
        DWORD drives_after = GetLogicalDrives();
        DWORD diff = drives_after & ~drives_before;
        if (diff != 0) {
            for (int b = 0; b < 26; b++) {
                if (diff & (1 << b)) {
                    assigned_letter = (wchar_t)(L'A' + b);
                    break;
                }
            }
            if (assigned_letter != 0) {
                break;
            }
        }
    }

    // If diff didn't catch it, scan for CD-ROM drive that wasn't in drives_before
    if (assigned_letter == 0) {
        for (int b = 2; b < 26; b++) { // C: to Z:
            if ((GetLogicalDrives() & (1 << b)) && !(drives_before & (1 << b))) {
                assigned_letter = (wchar_t)(L'A' + b);
                break;
            }
        }
    }

    if (assigned_letter == 0) {
        // Detach on failure to find drive letter
        pDetach(h_vdisk, DETACH_VIRTUAL_DISK_FLAG_NONE, 0);
        CloseHandle(h_vdisk);
        FreeLibrary(hVirtdisk);
        return -4; // Drive letter was not assigned
    }

    out_drive_root[0] = assigned_letter;
    out_drive_root[1] = L':';
    out_drive_root[2] = L'\\';
    out_drive_root[3] = L'\0';
    *out_vdisk_handle = h_vdisk;

    return 0; // Success!
}

int c_unmount_iso_virtdisk(HANDLE h_vdisk) {
    if (h_vdisk == NULL || h_vdisk == INVALID_HANDLE_VALUE) {
        return -1;
    }

    HMODULE hVirtdisk = LoadLibraryA("virtdisk.dll");
    if (hVirtdisk) {
        PFN_DetachVirtualDisk pDetach = NULL;
        FARPROC procDetach = GetProcAddress(hVirtdisk, "DetachVirtualDisk");
        memcpy(&pDetach, &procDetach, sizeof(procDetach));
        if (pDetach) {
            pDetach(h_vdisk, DETACH_VIRTUAL_DISK_FLAG_NONE, 0);
        }
        FreeLibrary(hVirtdisk);
    }

    CloseHandle(h_vdisk);
    c_notify_shell_media_change(NULL);
    return 0;
}

// 8. System Theme & Diagnostics
int c_is_system_dark_mode(void) {
    HKEY hKey;
    DWORD value = 1;
    DWORD size = sizeof(value);
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        if (RegQueryValueExW(hKey, L"AppsUseLightTheme", NULL, NULL, (LPBYTE)&value, &size) == ERROR_SUCCESS) {
            RegCloseKey(hKey);
            return (value == 0) ? 1 : 0;
        }
        RegCloseKey(hKey);
    }
    return 0;
}

int c_get_windows_accent_color(unsigned int* out_abgr) {
    HKEY hKey;
    DWORD color = 0;
    DWORD size = sizeof(color);
    if (RegOpenKeyExW(HKEY_CURRENT_USER, L"Software\\Microsoft\\Windows\\DWM", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        if (RegQueryValueExW(hKey, L"AccentColor", NULL, NULL, (LPBYTE)&color, &size) == ERROR_SUCCESS) {
            RegCloseKey(hKey);
            if (out_abgr) {
                *out_abgr = (unsigned int)color;
            }
            return 1;
        }
        RegCloseKey(hKey);
    }
    return 0;
}

int c_get_windows_version_string(char* out_buf, unsigned int max_len) {
    if (!out_buf || max_len < 16) return -1;
    out_buf[0] = '\0';

    HKEY hKey;
    wchar_t prod_name[128] = {0};
    wchar_t build_num[32] = {0};
    DWORD ubr = 0;
    DWORD size_prod = sizeof(prod_name);
    DWORD size_build = sizeof(build_num);
    DWORD size_ubr = sizeof(ubr);

    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", 0, KEY_READ, &hKey) == ERROR_SUCCESS) {
        RegQueryValueExW(hKey, L"ProductName", NULL, NULL, (LPBYTE)prod_name, &size_prod);
        RegQueryValueExW(hKey, L"CurrentBuild", NULL, NULL, (LPBYTE)build_num, &size_build);
        RegQueryValueExW(hKey, L"UBR", NULL, NULL, (LPBYTE)&ubr, &size_ubr);
        RegCloseKey(hKey);
    }

    SYSTEM_INFO si;
    GetNativeSystemInfo(&si);
    const char* arch_str = "x64";
    if (si.wProcessorArchitecture == PROCESSOR_ARCHITECTURE_ARM64) {
        arch_str = "ARM64";
    } else if (si.wProcessorArchitecture == PROCESSOR_ARCHITECTURE_INTEL) {
        arch_str = "x86";
    }

    char prod_ascii[128] = "Windows 11";
    char build_ascii[32] = "26100";
    if (prod_name[0] != L'\0') {
        WideCharToMultiByte(CP_UTF8, 0, prod_name, -1, prod_ascii, sizeof(prod_ascii), NULL, NULL);
    }
    if (build_num[0] != L'\0') {
        WideCharToMultiByte(CP_UTF8, 0, build_num, -1, build_ascii, sizeof(build_ascii), NULL, NULL);
    }

    // Windows 11 builds start at 22000. Microsoft kept "Windows 10" in ProductName registry for app compatibility.
    // Replace "Windows 10" with "Windows 11" if build >= 22000, matching Rufus behavior.
    int build_int = atoi(build_ascii);
    if (build_int >= 22000) {
        char* pos = strstr(prod_ascii, "Windows 10");
        if (pos) {
            char temp[128] = {0};
            snprintf(temp, sizeof(temp), "Windows 11%s", pos + 10);
            strncpy(prod_ascii, temp, sizeof(prod_ascii) - 1);
            prod_ascii[sizeof(prod_ascii) - 1] = '\0';
        }
    }

    if (ubr > 0) {
        snprintf(out_buf, max_len, "%s %s (Build %s.%lu)", prod_ascii, arch_str, build_ascii, (unsigned long)ubr);
    } else {
        snprintf(out_buf, max_len, "%s %s (Build %s)", prod_ascii, arch_str, build_ascii);
    }

    return 0;
}
