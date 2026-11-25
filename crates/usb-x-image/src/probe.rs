use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use usb_x_core::{
    errors::{Result, UsbXError},
    plan::ImageInfo,
    types::{CompressedFormat, ImageDetectedType},
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

    let stem_label = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty());

    // Compressed raw/ISO images (.gz, .xz, .bz2, .zst) -> stream directly in DD mode
    if CompressedFormat::from_path(path_str).is_some() {
        return Ok(ImageInfo {
            path: path_str.to_string(),
            file_size,
            volume_label: stem_label,
            detected_type: ImageDetectedType::GenericDiskImage,
            architecture: Some("x64".to_string()),
            has_efi_boot: true,
            has_bios_boot: true,
            is_wim_split_needed: false,
            persistence_supported: false,
            is_isohybrid: false,
            has_grub: false,
        });
    }

    let mut volume_label = None;
    let mut detected_type = ImageDetectedType::Unknown;
    let mut has_efi_boot = false;
    let mut has_bios_boot = false;
    let mut is_wim_split_needed = false;
    let mut persistence_supported = false;
    let mut architecture = Some("x64".to_string());

    // 1. Inspect Sector 0 (MBR partition table) & Sector 1 (GPT header)
    let mut sector0 = [0u8; 512];
    let has_sector0 = file.seek(SeekFrom::Start(0)).is_ok() && file.read_exact(&mut sector0).is_ok();
    let has_mbr_magic = has_sector0 && sector0[510] == 0x55 && sector0[511] == 0xAA;

    let mut sector1 = [0u8; 512];
    let has_gpt_header = file.seek(SeekFrom::Start(512)).is_ok()
        && file.read_exact(&mut sector1).is_ok()
        && &sector1[0..8] == b"EFI PART";

    let mut has_mbr_partitions = false;
    let mut has_efi_partition = has_gpt_header;
    if has_mbr_magic {
        for part_idx in 0..4 {
            let offset = 446 + part_idx * 16;
            let p_type = sector0[offset + 4];
            let p_sectors = u32::from_le_bytes(
                sector0[offset + 12..offset + 16].try_into().unwrap_or([0; 4]),
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
    let mut has_iso_pvd = false;
    if file_size > 0x8000 + 2048 {
        let mut pvd = [0u8; 2048];
        if file.seek(SeekFrom::Start(0x8000)).is_ok()
            && file.read_exact(&mut pvd).is_ok()
            && &pvd[1..6] == b"CD001"
        {
            has_iso_pvd = true;
            has_bios_boot = true;

            let raw_label = String::from_utf8_lossy(&pvd[40..72]).trim().to_string();
            if !raw_label.is_empty() {
                volume_label = Some(raw_label.clone());
            }

            let label_upper = raw_label.to_uppercase();
            let filename_upper = path
                .file_name()
                .map(|n| n.to_string_lossy().to_uppercase())
                .unwrap_or_default();
            let pvd_meta_upper = String::from_utf8_lossy(&pvd[318..702]).to_uppercase();
            let is_microsoft_pvd = pvd_meta_upper.contains("MICROSOFT");

            let is_win11 = label_upper.contains("WIN11")
                || label_upper.contains("WIN_11")
                || label_upper.contains("WINDOWS_11")
                || label_upper.contains("CCCOMA")
                || label_upper.contains("IR5_")
                || filename_upper.contains("WIN11")
                || filename_upper.contains("WIN_11")
                || filename_upper.contains("WINDOWS_11")
                || filename_upper.contains("WINDOWS 11")
                || filename_upper.contains("TINY11");

            let is_win10 = label_upper.contains("WIN10")
                || label_upper.contains("WIN_10")
                || label_upper.contains("WINDOWS_10")
                || label_upper.contains("CCSA")
                || label_upper.contains("J_CCSA")
                || label_upper.contains("ESD-ISO")
                || filename_upper.contains("WIN10")
                || filename_upper.contains("WIN_10")
                || filename_upper.contains("WINDOWS_10")
                || filename_upper.contains("WINDOWS 10")
                || filename_upper.contains("TINY10");

            let is_other_win = !is_win11
                && !is_win10
                && (is_microsoft_pvd
                    || label_upper.contains("X64FRE")
                    || label_upper.contains("X86FRE")
                    || label_upper.contains("A64FRE")
                    || label_upper.contains("WINDOWS")
                    || filename_upper.contains("WINDOWS")
                    || filename_upper.contains("WIN7")
                    || filename_upper.contains("WIN8")
                    || filename_upper.contains("SERVER_EVAL")
                    || filename_upper.contains("WINPE"));

            if label_upper.contains("A64FRE") || filename_upper.contains("ARM64") {
                architecture = Some("arm64".to_string());
            } else if label_upper.contains("X86FRE")
                || filename_upper.contains("X86.")
                || filename_upper.contains("I386")
                || filename_upper.contains("I686")
            {
                architecture = Some("x86".to_string());
            }

            if is_win11 {
                detected_type = ImageDetectedType::Windows11Installer {
                    version: "Windows 11 (64-bit)".to_string(),
                };
                has_efi_boot = true;
                if file_size > 4_000_000_000 {
                    is_wim_split_needed = true;
                }
            } else if is_win10 {
                detected_type = ImageDetectedType::Windows10Installer {
                    version: "Windows 10".to_string(),
                };
                has_efi_boot = true;
                if file_size > 4_000_000_000 {
                    is_wim_split_needed = true;
                }
            } else if is_other_win {
                detected_type = ImageDetectedType::Windows10Installer {
                    version: "Windows Setup Installer".to_string(),
                };
                has_efi_boot = true;
                if file_size > 4_000_000_000 {
                    is_wim_split_needed = true;
                }
            } else if is_isohybrid
                || label_upper.contains("UBUNTU")
                || filename_upper.contains("UBUNTU")
                || label_upper.contains("DEBIAN")
                || filename_upper.contains("DEBIAN")
                || label_upper.contains("FEDORA")
                || filename_upper.contains("FEDORA")
                || label_upper.contains("ARCH")
                || filename_upper.contains("ARCH")
                || label_upper.contains("MINT")
                || filename_upper.contains("MINT")
                || label_upper.contains("KALI")
                || filename_upper.contains("KALI")
                || label_upper.contains("MANJARO")
                || filename_upper.contains("MANJARO")
                || label_upper.contains("POP")
                || filename_upper.contains("POP")
            {
                let distro_name =
                    if label_upper.contains("UBUNTU") || filename_upper.contains("UBUNTU") {
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
                        "Linux ISOHybrid".to_string()
                    };

                detected_type = ImageDetectedType::LinuxHybridIso { distro: distro_name };
                has_efi_boot = true;
                has_bios_boot = true;
                persistence_supported = true;
            } else {
                detected_type = ImageDetectedType::GenericDiskImage;
                has_efi_boot = true;
            }
        }
    }

    // 3. Fallback for raw disk images (.img, .raw, .bin, .vhd) without ISO 9660 PVD
    if !has_iso_pvd && matches!(detected_type, ImageDetectedType::Unknown) {
        if volume_label.is_none() {
            volume_label = stem_label;
        }
        if has_mbr_magic || has_gpt_header {
            has_bios_boot = has_mbr_magic;
            has_efi_boot = has_efi_partition || has_gpt_header;
        }
        detected_type = ImageDetectedType::GenericDiskImage;
    }

    Ok(ImageInfo {
        path: path_str.to_string(),
        file_size,
        volume_label,
        detected_type,
        architecture,
        has_efi_boot,
        has_bios_boot,
        is_wim_split_needed,
        persistence_supported,
        is_isohybrid,
        has_grub,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_probe_gpt_raw_image_and_compressed_extension() {
        let base = std::env::temp_dir().join(format!("pigeon_probe_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let raw_path = base.join("disk_gpt.img");
        {
            let mut f = File::create(&raw_path).unwrap();
            let mut sectors = vec![0u8; 2048];
            sectors[510] = 0x55;
            sectors[511] = 0xAA;
            sectors[512..520].copy_from_slice(b"EFI PART");
            f.write_all(&sectors).unwrap();
        }
        let info = probe_image(&raw_path.to_string_lossy()).unwrap();
        assert!(info.has_efi_boot);
        assert!(info.is_raw_disk_image());
        assert!(!info.is_windows());

        let xz_path = base.join("ubuntu.img.xz");
        std::fs::write(&xz_path, b"dummy").unwrap();
        let xz_info = probe_image(&xz_path.to_string_lossy()).unwrap();
        assert_eq!(xz_info.compressed_format(), Some(CompressedFormat::Xz));
        assert!(xz_info.is_raw_disk_image());

        let _ = std::fs::remove_dir_all(&base);
    }
}
