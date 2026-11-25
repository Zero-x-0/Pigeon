use crate::device::DeviceId;
use crate::types::{
    BootSelectionMode, ClusterSize, CompressedFormat, FileSystem, ImageDetectedType,
    ImageWriteMode, PartitionScheme, TargetSystem,
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
    pub is_wim_split_needed: bool,
    pub persistence_supported: bool,
    pub is_isohybrid: bool,
    pub has_grub: bool,
}

impl ImageInfo {
    pub fn compressed_format(&self) -> Option<CompressedFormat> {
        CompressedFormat::from_path(&self.path)
    }

    pub fn is_windows(&self) -> bool {
        matches!(
            self.detected_type,
            ImageDetectedType::Windows11Installer { .. }
                | ImageDetectedType::Windows10Installer { .. }
                | ImageDetectedType::WindowsLegacy { .. }
        )
    }

    pub fn is_linux(&self) -> bool {
        if self.is_windows() {
            return false;
        }
        matches!(self.detected_type, ImageDetectedType::LinuxHybridIso { .. })
            || self.has_grub
            || self.persistence_supported
    }

    /// True for raw disk images (.img/.bin/.raw/.vhd/.dd) or compressed images (.gz/.xz/.bz2/.zst)
    pub fn is_raw_disk_image(&self) -> bool {
        if self.compressed_format().is_some() {
            return true;
        }
        let lower = self.path.to_ascii_lowercase();
        lower.ends_with(".img")
            || lower.ends_with(".raw")
            || lower.ends_with(".bin")
            || lower.ends_with(".vhd")
            || lower.ends_with(".dd")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationKind {
    WriteImage {
        image: ImageInfo,
        partition_scheme: PartitionScheme,
        target_system: TargetSystem,
        file_system: FileSystem,
        cluster_size: ClusterSize,
        volume_label: String,
        quick_format: bool,
        extended_label: bool,
        verify_after_write: bool,
        check_bad_blocks_passes: Option<u32>,
        linux_persistence_mb: Option<u64>,
        write_mode: ImageWriteMode,
        old_bios_fixes: bool,
        rufus_mbr_bios_id: Option<u8>,
    },
    FormatOnly {
        partition_scheme: PartitionScheme,
        file_system: FileSystem,
        cluster_size: ClusterSize,
        volume_label: String,
        quick_format: bool,
        extended_label: bool,
        check_bad_blocks_passes: Option<u32>,
        old_bios_fixes: bool,
        rufus_mbr_bios_id: Option<u8>,
    },
    BackupDrive {
        output_file_path: String,
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
