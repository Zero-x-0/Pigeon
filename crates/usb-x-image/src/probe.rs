use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use usb_x_core::{
    errors::{Result, UsbXError},
    plan::ImageInfo,
    types::{ImageDetectedType, SecureBootStatus},
};

pub fn probe_image(path_str: &str) -> Result<ImageInfo> {
    let path = Path::new(path_str);
    if !path.exists() {
        return Err(UsbXError::ImageError(format!("File does not exist: {}", path_str)));
    }

    let mut file = File::open(path).map_err(|e| UsbXError::Io {
        operation: "open_image".to_string(),
        message: e.to_string(),
    })?;

    let file_size = file.metadata().map_err(|e| UsbXError::Io {
        operation: "metadata".to_string(),
        message: e.to_string(),
    })?.len();

    let mut volume_label = None;
    let mut detected_type = ImageDetectedType::Unknown;
    let mut has_efi_boot = false;
    let mut has_bios_boot = false;
    let mut secure_boot_status = SecureBootStatus::NotApplicable;
    let mut is_wim_split_needed = false;
    let mut persistence_supported = false;
    let mut architecture = Some("x64".to_string());

    // 1. Inspect Sector 0 (MBR partition table & boot signatures)
    let mut sector0 = [0u8; 512];
    let has_sector0 = file.seek(SeekFrom::Start(0)).is_ok() && file.read_exact(&mut sector0).is_ok();
    let has_mbr_magic = has_sector0 && sector0[510] == 0x55 && sector0[511] == 0xAA;

    let mut has_mbr_partitions = false;
    let mut has_efi_partition = false;
    if has_mbr_magic {
        for part_idx in 0..4 {
            let offset = 446 + part_idx * 16;
            let p_type = sector0[offset + 4];
            let p_sectors = u32::from_le_bytes(
                sector0[offset + 12..offset + 16].try_into().unwrap_or([0; 4])
            );
            if p_type != 0 && p_sectors > 0 {
                has_mbr_partitions = true;
                if p_type == 0xEF {
                    has_efi_partition = true;
                }
            }
        }
    }

    let has_grub = sector0.windows(4).any(|w| w == b"GRUB" || w == b"Geom");
    let has_syslinux = sector0.windows(8).any(|w| w == b"ISOLINUX" || w == b"SYSLINUX");
    let is_isohybrid = (has_mbr_magic && has_mbr_partitions) || has_grub || has_syslinux;

    // 2. Inspect ISO 9660 PVD (Primary Volume Descriptor) at 0x8000 (32768)
    if file_size > 0x8800 + 2048 {
        let mut pvd = [0u8; 2048];
        if file.seek(SeekFrom::Start(0x8000)).is_ok() && file.read_exact(&mut pvd).is_ok() {
            // Check for standard "CD001" identifier
            if &pvd[1..6] == b"CD001" {
                has_bios_boot = true;
                // Volume ID is at offset 40..72
                let raw_label = String::from_utf8_lossy(&pvd[40..72]).trim().to_string();
                if !raw_label.is_empty() {
                    volume_label = Some(raw_label.clone());
                }

                let label_upper = raw_label.to_uppercase();
                let filename_upper = path.file_name()
                    .map(|n| n.to_string_lossy().to_uppercase())
                    .unwrap_or_default();

                // Detect OS from volume label / filename
                if label_upper.contains("WIN11") || label_upper.contains("CCCOMA_X64FRE") || filename_upper.contains("WIN11") {
                    detected_type = ImageDetectedType::Windows11Installer {
                        version: "Windows 11 (64-bit)".to_string(),
                    };
                    has_efi_boot = true;
                    secure_boot_status = SecureBootStatus::PreservedSignedChain;
                    architecture = Some("x64".to_string());
                    if file_size > 4_000_000_000 {
                        is_wim_split_needed = true;
                    }
                } else if label_upper.contains("WIN10") || label_upper.contains("CCSA_X64FRE") || filename_upper.contains("WIN10") {
                    detected_type = ImageDetectedType::Windows10Installer {
                        version: "Windows 10".to_string(),
                    };
                    has_efi_boot = true;
                    secure_boot_status = SecureBootStatus::PreservedSignedChain;
                    if file_size > 4_000_000_000 {
                        is_wim_split_needed = true;
                    }
                } else if is_isohybrid || label_upper.contains("UBUNTU") || filename_upper.contains("UBUNTU")
                    || label_upper.contains("DEBIAN") || filename_upper.contains("DEBIAN")
                    || label_upper.contains("FEDORA") || filename_upper.contains("FEDORA")
                    || label_upper.contains("ARCH") || filename_upper.contains("ARCH")
                    || label_upper.contains("MINT") || filename_upper.contains("MINT")
                    || label_upper.contains("KALI") || filename_upper.contains("KALI")
                    || label_upper.contains("MANJARO") || filename_upper.contains("MANJARO")
                    || label_upper.contains("POP") || filename_upper.contains("POP")
                {
                    let distro_name = if label_upper.contains("UBUNTU") || filename_upper.contains("UBUNTU") {
                        "Ubuntu Linux".to_string()
                    } else if label_upper.contains("DEBIAN") || filename_upper.contains("DEBIAN") {
                        "Debian GNU/Linux".to_string()
                    } else if label_upper.contains("FEDORA") || filename_upper.contains("FEDORA") {
                        "Fedora Workstation".to_string()
                    } else if label_upper.contains("ARCH") || filename_upper.contains("ARCH") {
                        "Arch Linux".to_string()
                    } else if label_upper.contains("MINT") || filename_upper.contains("MINT") {
                        "Linux Mint".to_string()
                    } else if label_upper.contains("KALI") || filename_upper.contains("KALI") {
                        "Kali Linux".to_string()
                    } else {
                        "Linux ISOHybrid (GRUB / Syslinux)".to_string()
                    };

                    detected_type = ImageDetectedType::LinuxHybridIso { distro: distro_name };
                    has_efi_boot = true;
                    has_bios_boot = true;
                    secure_boot_status = SecureBootStatus::PreservedSignedChain;
                    persistence_supported = true;
                } else if label_upper.contains("FD1") || filename_upper.contains("FREEDOS") {
                    detected_type = ImageDetectedType::FreeDosImage;
                    has_bios_boot = true;
                } else {
                    detected_type = ImageDetectedType::GenericDiskImage;
                    has_efi_boot = true;
                    secure_boot_status = SecureBootStatus::PreservedSignedChain;
                }
            }
        }
    }

    // 3. Inspect raw disk image if PVD was not found
    if matches!(detected_type, ImageDetectedType::Unknown) {
        if has_mbr_magic {
            has_bios_boot = true;
            if has_efi_partition {
                has_efi_boot = true;
            }
            detected_type = ImageDetectedType::GenericDiskImage;
        }
    }

    Ok(ImageInfo {
        path: path_str.to_string(),
        file_size,
        volume_label,
        detected_type,
        architecture,
        has_efi_boot,
        has_bios_boot,
        sha256: None,
        secure_boot_status,
        is_wim_split_needed,
        persistence_supported,
        is_isohybrid,
        has_grub,
    })
}

