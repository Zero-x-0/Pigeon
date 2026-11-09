#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootSelectionMode {
    DiskImage,
    FreeDos,
    MsDosUserFiles,
    BackupDrive,
    RestoreDrive,
    FormatOnly,
    FlashTest,
}

impl BootSelectionMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DiskImage => "Disk or ISO image (Please select)",
            Self::FreeDos => "FreeDOS",
            Self::MsDosUserFiles => "MS-DOS (user-supplied files)",
            Self::BackupDrive => "Drive Image Backup (.img)",
            Self::RestoreDrive => "Restore Drive from Image",
            Self::FormatOnly => "Non bootable (Format only)",
            Self::FlashTest => "Bad Block & Fake Flash Drive Test",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionScheme {
    Auto,
    Mbr,
    Gpt,
}

impl PartitionScheme {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto (Recommended)",
            Self::Mbr => "MBR (Legacy BIOS / UEFI-CSM)",
            Self::Gpt => "GPT (UEFI non-CSM)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSystem {
    Auto,
    Uefi,
    Bios,
    BiosAndUefi,
}

impl TargetSystem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto (Target System)",
            Self::Uefi => "UEFI (non CSM)",
            Self::Bios => "BIOS (or UEFI-CSM)",
            Self::BiosAndUefi => "BIOS + UEFI (Dual mode)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystem {
    Auto,
    Fat32,
    Ntfs,
    ExFat,
    Udf,
}

impl FileSystem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto (Recommended)",
            Self::Fat32 => "FAT32 (Default)",
            Self::Ntfs => "NTFS",
            Self::ExFat => "exFAT",
            Self::Udf => "UDF",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto | Self::Fat32 => "FAT32",
            Self::Ntfs => "NTFS",
            Self::ExFat => "exFAT",
            Self::Udf => "UDF",
        }
    }

    pub fn max_file_size(&self) -> Option<u64> {
        match self {
            Self::Fat32 => Some(4_294_967_295), // 4 GiB - 1 byte
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterSize {
    Default,
    Bytes512,
    Bytes1024,
    Bytes2048,
    Bytes4096,
    Bytes8192,
    Bytes16384,
    Bytes32768,
    Bytes65536,
}

impl ClusterSize {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Default => "Default (Recommended)",
            Self::Bytes512 => "512 bytes",
            Self::Bytes1024 => "1024 bytes (1 KB)",
            Self::Bytes2048 => "2048 bytes (2 KB)",
            Self::Bytes4096 => "4096 bytes (4 KB)",
            Self::Bytes8192 => "8192 bytes (8 KB)",
            Self::Bytes16384 => "16 kilobytes",
            Self::Bytes32768 => "32 kilobytes",
            Self::Bytes65536 => "64 kilobytes",
        }
    }

    pub fn to_bytes(&self) -> u32 {
        match self {
            Self::Default | Self::Bytes4096 => 4096,
            Self::Bytes512 => 512,
            Self::Bytes1024 => 1024,
            Self::Bytes2048 => 2048,
            Self::Bytes8192 => 8192,
            Self::Bytes16384 => 16384,
            Self::Bytes32768 => 32768,
            Self::Bytes65536 => 65536,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDetectedType {
    Windows11Installer { version: String },
    Windows10Installer { version: String },
    WindowsLegacy { name: String },
    LinuxHybridIso { distro: String },
    FreeDosImage,
    GenericDiskImage,
    Unknown,
}

impl ImageDetectedType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Windows11Installer { .. } => "Windows 11 Installer",
            Self::Windows10Installer { .. } => "Windows 10 Installer",
            Self::WindowsLegacy { .. } => "Windows Installation Media",
            Self::LinuxHybridIso { .. } => "Linux ISOHybrid (GRUB)",
            Self::FreeDosImage => "FreeDOS",
            Self::GenericDiskImage => "Disk Image",
            Self::Unknown => "Unknown Image",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageWriteMode {
    Auto,
    DdImageMode,
    IsoImageMode,
}

impl ImageWriteMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "ISO Image mode (Recommended)",
            Self::DdImageMode => "DD Image mode (Raw Disk Copy)",
            Self::IsoImageMode => "ISO Image mode (Recommended - Bootable USB & Full Drive)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootStatus {
    PreservedSignedChain,
    SignedBridgeChain,
    UnsignedLikelyBlocked,
    NotApplicable,
}

impl SecureBootStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PreservedSignedChain => "Preserved signed chain (Secure Boot supported)",
            Self::SignedBridgeChain => "Signed UEFI bridge (Secure Boot compatible)",
            Self::UnsignedLikelyBlocked => "Custom / Unsigned (Requires Secure Boot disabled)",
            Self::NotApplicable => "N/A (Legacy BIOS or data only)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    Usb,
    SdCard,
    Nvme,
    Sata,
    Scsi,
    VirtualFile,
    Unknown,
}

impl BusType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Usb => "USB",
            Self::SdCard => "SD / Memory Card",
            Self::Nvme => "NVMe",
            Self::Sata => "SATA",
            Self::Scsi => "SCSI",
            Self::VirtualFile => "Virtual Disk Image",
            Self::Unknown => "Unknown Bus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemDiskRisk {
    SafeRemovable,
    InternalFixedDrive,
    SystemBootDrive,
    MountedSystemVolume,
}

impl SystemDiskRisk {
    pub fn is_destructive_blocked(&self) -> bool {
        matches!(self, Self::SystemBootDrive | Self::MountedSystemVolume)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Windows11Tweaks {
    pub bypass_tpm_secure_boot_ram: bool,
    pub bypass_online_account: bool,
    pub disable_telemetry_privacy: bool,
    pub disable_bitlocker_device_encryption: bool,
    pub create_local_account: bool,
    pub local_username: String,
}

impl Default for Windows11Tweaks {
    fn default() -> Self {
        Self {
            bypass_tpm_secure_boot_ram: true,
            bypass_online_account: true,
            disable_telemetry_privacy: true,
            disable_bitlocker_device_encryption: true,
            create_local_account: true,
            local_username: "User".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashTestKind {
    ReadOnlyBadBlockScan,
    DestructiveFakeCapacityTest,
}

impl FlashTestKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ReadOnlyBadBlockScan => "Read-only surface test (Safe)",
            Self::DestructiveFakeCapacityTest => "Destructive write & read test (Detects Fake Flash)",
        }
    }
}
