use crate::device::DeviceId;
use crate::types::{
    BootSelectionMode, ClusterSize, FileSystem, FlashTestKind, ImageDetectedType,
    ImageWriteMode, PartitionScheme, SecureBootStatus, TargetSystem, Windows11Tweaks,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub path: String,
    pub file_size: u64,
    pub volume_label: Option<String>,
    pub detected_type: ImageDetectedType,
    pub architecture: Option<String>,
    pub has_efi_boot: bool,
    pub has_bios_boot: bool,
    pub sha256: Option<String>,
    pub secure_boot_status: SecureBootStatus,
    pub is_wim_split_needed: bool,
    pub persistence_supported: bool,
    pub is_isohybrid: bool,
    pub has_grub: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationKind {
    WriteImage {
        image: ImageInfo,
        partition_scheme: PartitionScheme,
        target_system: TargetSystem,
        file_system: FileSystem,
        verify_after_write: bool,
        win11_tweaks: Option<Windows11Tweaks>,
        linux_persistence_mb: Option<u64>,
        write_mode: ImageWriteMode,
    },
    CreateFreeDosMedia {
        partition_scheme: PartitionScheme,
        file_system: FileSystem,
    },
    CreateMsDosMedia {
        files_folder: String,
        partition_scheme: PartitionScheme,
        file_system: FileSystem,
    },
    CreateWindowsToGo {
        image: ImageInfo,
        partition_scheme: PartitionScheme,
        file_system: FileSystem,
    },
    FormatOnly {
        partition_scheme: PartitionScheme,
        file_system: FileSystem,
        cluster_size: ClusterSize,
        volume_label: String,
        quick_format: bool,
        extended_label: bool,
    },
    BackupDrive {
        output_file_path: String,
        compress_zstd: bool,
    },
    RestoreDrive {
        backup_image_path: String,
        verify: bool,
    },
    FlashTest {
        kind: FlashTestKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPlan {
    pub target_device_id: DeviceId,
    pub target_device_summary: String,
    pub target_size_bytes: u64,
    pub mode: BootSelectionMode,
    pub kind: OperationKind,
}
