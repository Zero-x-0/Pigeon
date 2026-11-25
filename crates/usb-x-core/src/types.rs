#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootSelectionMode {
    DiskImage,
    FormatOnly,
    BackupDrive,
}

impl BootSelectionMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DiskImage => "Disk or ISO image (Please select)",
            Self::FormatOnly => "Non bootable",
            Self::BackupDrive => "Drive Image Backup (.img)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionScheme {
    Mbr,
    Gpt,
}

impl PartitionScheme {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Mbr => "MBR",
            Self::Gpt => "GPT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSystem {
    Uefi,
    Bios,
}

impl TargetSystem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Uefi => "UEFI (non CSM)",
            Self::Bios => "BIOS or UEFI",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystem {
    Fat32,
    Ntfs,
    ExFat,
}

impl FileSystem {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Fat32 => "FAT32 (Default)",
            Self::Ntfs => "NTFS",
            Self::ExFat => "exFAT",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fat32 => "FAT32",
            Self::Ntfs => "NTFS",
            Self::ExFat => "exFAT",
        }
    }

    pub fn max_file_size(&self) -> Option<u64> {
        match self {
            Self::Fat32 => Some(4_294_967_295),
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
            Self::Default => "4096 bytes (Default)",
            Self::Bytes512 => "512 bytes",
            Self::Bytes1024 => "1024 bytes",
            Self::Bytes2048 => "2048 bytes",
            Self::Bytes4096 => "4096 bytes",
            Self::Bytes8192 => "8192 bytes",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressedFormat {
    Gzip,
    Xz,
    Bzip2,
    Zstd,
}

impl CompressedFormat {
    pub fn from_path(path: &str) -> Option<Self> {
        let lower = path.to_ascii_lowercase();
        if lower.ends_with(".gz") {
            Some(Self::Gzip)
        } else if lower.ends_with(".xz") {
            Some(Self::Xz)
        } else if lower.ends_with(".bz2") {
            Some(Self::Bzip2)
        } else if lower.ends_with(".zst") {
            Some(Self::Zstd)
        } else {
            None
        }
    }

    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Xz => "xz",
            Self::Bzip2 => "bzip2",
            Self::Zstd => "zstd",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Gzip => "GZIP",
            Self::Xz => "XZ",
            Self::Bzip2 => "BZIP2",
            Self::Zstd => "ZSTD",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageDetectedType {
    Windows11Installer { version: String },
    Windows10Installer { version: String },
    WindowsLegacy { name: String },
    LinuxHybridIso { distro: String },
    GenericDiskImage,
    Unknown,
}

impl ImageDetectedType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Windows11Installer { .. } => "Windows 11 Installer",
            Self::Windows10Installer { .. } => "Windows 10 Installer",
            Self::WindowsLegacy { .. } => "Windows Installation Media",
            Self::LinuxHybridIso { .. } => "Linux ISOHybrid",
            Self::GenericDiskImage => "Disk Image",
            Self::Unknown => "Unknown Image",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageWriteMode {
    IsoImageMode,
    DdImageMode,
}

impl ImageWriteMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::IsoImageMode => "Write in ISO Image mode (Recommended)",
            Self::DdImageMode => "Write in DD Image mode",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn label(&self) -> &'static str {
        match self {
            Self::System => "System Default",
            Self::Light => "Light (Classic Rufus)",
            Self::Dark => "Dark",
        }
    }
}
