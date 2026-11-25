use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use usb_x_core::device::DeviceInfo;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationResult, OperationStage};
use usb_x_core::plan::{OperationKind, OperationPlan};
use usb_x_core::types::{ClusterSize, FileSystem, ImageWriteMode, PartitionScheme};
use usb_x_core::validation::validate_plan;
use usb_x_platform::StorageBackend;

pub fn execute_operation_pipeline(
    mut backend: Box<dyn StorageBackend>,
    plan: OperationPlan,
    device: DeviceInfo,
    sender: Sender<OperationEvent>,
    cancelled: Arc<AtomicBool>,
) {
    let result = run_pipeline(&mut *backend, &plan, &device, &sender, &cancelled);

    match result {
        Ok(_) => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Complete,
                message: "Ready".to_string(),
            });
            let _ = sender.send(OperationEvent::Finished {
                result: OperationResult::Success {
                    message: "Operation completed successfully!".to_string(),
                },
            });
        }
        Err(UsbXError::Cancelled(msg)) => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Cancelled,
                message: format!("Operation cancelled: {}", msg),
            });
            let _ = sender.send(OperationEvent::Finished {
                result: OperationResult::Cancelled,
            });
        }
        Err(e) => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Failed,
                message: format!("Operation failed: {}", e),
            });
            let _ = sender.send(OperationEvent::Finished {
                result: OperationResult::Failed {
                    error: e.to_string(),
                },
            });
        }
    }
}

fn format_cluster_opt(cluster_size: ClusterSize) -> Option<u32> {
    match cluster_size {
        ClusterSize::Default => None,
        other => Some(other.to_bytes()),
    }
}

/// Write a safe `[Autorun]` extended label file (`autorun.inf`) when enabled and not already present.
fn write_extended_label_if_enabled(
    target_root: &str,
    extended_label: bool,
    volume_label: &str,
    sender: &Sender<OperationEvent>,
) {
    if !extended_label || volume_label.trim().is_empty() {
        return;
    }
    let root = Path::new(target_root);
    let has_autorun = std::fs::read_dir(root)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("autorun.inf"))
        })
        .unwrap_or(false);
    if has_autorun {
        return;
    }

    let safe_label: String = volume_label
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    if safe_label.is_empty() {
        return;
    }

    let content = format!("[Autorun]\r\nLabel={}\r\n", safe_label);
    let dest = root.join("autorun.inf");
    if std::fs::write(&dest, content).is_ok() {
        let _ = sender.send(OperationEvent::Log {
            level: LogLevel::Info,
            timestamp: "Now".to_string(),
            message: format!("Created extended volume label file ({})", dest.display()),
        });
    }
}

fn run_pipeline(
    backend: &mut dyn StorageBackend,
    plan: &OperationPlan,
    device: &DeviceInfo,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    // 1. Safety check
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::ValidatingSafety,
        message: "Validating device target and system disk protection...".to_string(),
    });
    validate_plan(plan, device)?;

    // 2. Unmount & Lock
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::DismountingAndLocking,
        message: format!("Acquiring exclusive lock on {}...", device.device_path),
    });
    backend.unmount_and_lock(&device.id)?;

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Target device locked: {}", device.summary()),
    });

    // 3. Dispatch Operation
    match &plan.kind {
        OperationKind::WriteImage {
            image,
            partition_scheme,
            target_system,
            file_system,
            cluster_size,
            volume_label,
            quick_format,
            extended_label,
            verify_after_write,
            check_bad_blocks_passes,
            linux_persistence_mb,
            write_mode,
            old_bios_fixes,
            rufus_mbr_bios_id,
        } => {
            if let Some(passes) = check_bad_blocks_passes {
                crate::flash_test::execute_bad_blocks_check(
                    backend,
                    &device.id,
                    device.size_bytes,
                    *passes,
                    sender,
                    cancelled,
                )?;
                let _ = backend.unmount_and_lock(&device.id);
            }

            let is_windows = image.is_windows();
            let use_dd_mode = *write_mode == ImageWriteMode::DdImageMode || image.is_raw_disk_image();

            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: format!(
                    "Writing image '{}' [Mode: {}] · Partition: {} · Target: {} · FS: {}",
                    image.path,
                    if use_dd_mode { "DD Image Mode" } else { "ISO Image Mode" },
                    partition_scheme.label(),
                    target_system.label(),
                    file_system.label()
                ),
            });

            let is_arch = image.path.to_ascii_lowercase().contains("arch")
                || image.volume_label.as_deref().unwrap_or("").to_ascii_lowercase().contains("arch");
            if is_arch {
                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Warn,
                    timestamp: "Now".to_string(),
                    message: "Arch Linux Notice: Arch Linux does not contain a Microsoft Secure Boot certificate. Please disable 'Secure Boot' in your PC BIOS/UEFI settings to boot Arch Linux.".to_string(),
                });
            }

            #[allow(unused_variables)]
            let orig_letter: Option<char> = device.partitions.iter()
                .find_map(|p| p.mount_point.as_ref())
                .and_then(|mp| mp.chars().next())
                .filter(|c| c.is_ascii_alphabetic())
                .or_else(|| {
                    let s = &device.display_name;
                    if let Some(start) = s.find('(') {
                        if let Some(end) = s[start..].find(')') {
                            let inner = &s[start + 1..start + end];
                            if inner.len() >= 2 && inner.ends_with(':') {
                                return inner.chars().next();
                            }
                        }
                    }
                    None
                });

            #[cfg(windows)]
            let disk_num_opt: Option<u32> = device.id.as_str()
                .strip_prefix("PhysicalDrive")
                .and_then(|s| s.parse::<u32>().ok());

            if use_dd_mode {
                let _ = sender.send(OperationEvent::StageChanged {
                    stage: OperationStage::WritingImage,
                    message: "Writing raw image sectors starting at Sector 0...".to_string(),
                });

                crate::raw_writer::execute_raw_write(
                    backend,
                    &device.id,
                    &image.path,
                    *verify_after_write,
                    sender,
                    cancelled,
                )?;

                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Info,
                    timestamp: "Now".to_string(),
                    message: "DD image write finished: Sector 0 partition table and boot sectors are live.".to_string(),
                });

                #[cfg(windows)]
                if let Some(letter) = orig_letter {
                    usb_x_platform::notify_drive_ready(&format!("{}:\\", letter));
                }
            } else {
                let raw_label = if !volume_label.trim().is_empty() {
                    volume_label.trim().to_string()
                } else {
                    image.volume_label.clone().unwrap_or_else(|| "USB_BOOT".to_string())
                };
                let effective_label = if matches!(file_system, FileSystem::Fat32) {
                    let fat_clean: String = raw_label
                        .chars()
                        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                        .take(11)
                        .collect();
                    if fat_clean.is_empty() { "USB_BOOT".to_string() } else { fat_clean.to_uppercase() }
                } else {
                    raw_label.clone()
                };

                let fs_str = file_system.as_str();
                let cluster_bytes_opt = format_cluster_opt(*cluster_size);

                #[allow(unused_assignments)]
                let mut mp_final = String::new();

                #[cfg(windows)]
                {
                    let _ = (cluster_bytes_opt, target_system, old_bios_fixes, rufus_mbr_bios_id, linux_persistence_mb);
                    let req_fat32 = matches!(file_system, FileSystem::Fat32);
                    let is_gpt = matches!(partition_scheme, PartitionScheme::Gpt);

                    let _ = sender.send(OperationEvent::StageChanged {
                        stage: OperationStage::WritingImage,
                        message: "Mounting source ISO image virtually...".to_string(),
                    });
                    let iso_mount = usb_x_platform::mount_iso_virtdisk(&image.path)?;
                    let source_root = iso_mount.drive_root.clone();
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Mounted source ISO to virtual drive {}", source_root),
                    });

                    let iso_summary = crate::iso_extractor::inspect_mounted_iso(&source_root);
                    let is_win_effective = is_windows || iso_summary.has_windows_setup;
                    let use_windows_dual_part = is_win_effective && (!req_fat32 || iso_summary.has_file_over_4gb);

                    if use_windows_dual_part {
                        const UEFI_NTFS_IMG: &[u8] = include_bytes!("../assets/uefi-ntfs.img");
                        let disk_num = disk_num_opt.filter(|&n| n != 0).ok_or_else(|| UsbXError::Io {
                            operation: "resolve_physical_drive".to_string(),
                            message: "Valid target PhysicalDrive number required for UEFI:NTFS layout.".to_string(),
                        })?;

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Partitioning,
                            message: format!(
                                "Partitioning PhysicalDrive{} (NTFS + UEFI:NTFS, {})...",
                                disk_num,
                                if is_gpt { "GPT" } else { "MBR" }
                            ),
                        });
                        usb_x_platform::create_windows_uefi_ntfs_layout(
                            disk_num,
                            device.size_bytes,
                            is_gpt,
                            UEFI_NTFS_IMG,
                        )?;
                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!(
                                "Created NTFS + UEFI:NTFS layout on PhysicalDrive{} (wrote {:.1} MB uefi-ntfs.img)",
                                disk_num,
                                UEFI_NTFS_IMG.len() as f64 / (1024.0 * 1024.0)
                            ),
                        });

                        let p1_mp = usb_x_platform::wait_for_partition_volume_ready(disk_num, 1, 10_000)
                            .ok_or_else(|| UsbXError::Io {
                                operation: "wait_for_ntfs_volume".to_string(),
                                message: format!("Partition 1 on PhysicalDrive{} did not receive a drive letter.", disk_num),
                            })?;

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Formatting {} as NTFS ({})...", p1_mp, raw_label),
                        });
                        usb_x_platform::format_drive_volume(
                            &p1_mp,
                            "NTFS",
                            &raw_label,
                            *quick_format,
                            cluster_size.to_bytes(),
                        )?;
                        mp_final = p1_mp.clone();
                        usb_x_platform::notify_drive_ready(&mp_final);

                        crate::iso_extractor::copy_mounted_iso_to_target(&source_root, &mp_final, false, sender, cancelled)?;
                        write_extended_label_if_enabled(&mp_final, *extended_label, &raw_label, sender);
                        crate::iso_extractor::ensure_uefi_boot_fallback(&mp_final, sender);
                        usb_x_platform::notify_drive_ready(&mp_final);
                    } else {
                        let mut target_mp: Option<String> = orig_letter.and_then(|letter| {
                            let p = format!("{}:\\", letter);
                            if Path::new(&p).exists() { Some(p) } else { None }
                        });
                        if target_mp.is_none() {
                            if let Some(disk_num) = disk_num_opt {
                                if let Some(v) = usb_x_platform::get_volumes_for_disk(disk_num).first() {
                                    let p = format!("{}:\\", v.drive_letter);
                                    if Path::new(&p).exists() {
                                        target_mp = Some(p);
                                    }
                                }
                            }
                        }

                        let mut has_multiple_partitions = device.partitions.len() > 1;
                        if let Some(disk_num) = disk_num_opt {
                            if disk_num != 0 && usb_x_platform::get_volumes_for_disk(disk_num).len() > 1 {
                                has_multiple_partitions = true;
                            }
                        }

                        if has_multiple_partitions {
                            if let Some(disk_num) = disk_num_opt.filter(|&n| n != 0) {
                                let _ = sender.send(OperationEvent::StageChanged {
                                    stage: OperationStage::Partitioning,
                                    message: format!("Wiping old partitions on PhysicalDrive{}...", disk_num),
                                });
                                let _ = usb_x_platform::clean_and_repartition_disk(disk_num, device.size_bytes, req_fat32);
                                if let Some(cand) = usb_x_platform::wait_for_volume_ready(disk_num, 4000) {
                                    let _ = usb_x_platform::format_drive_volume(&cand, fs_str, &effective_label, *quick_format, cluster_size.to_bytes());
                                    mp_final = cand.clone();
                                    usb_x_platform::notify_drive_ready(&cand);
                                }
                            }
                        } else if let Some(ref mp) = target_mp {
                            let _ = sender.send(OperationEvent::StageChanged {
                                stage: OperationStage::Formatting,
                                message: format!("Formatting {} as {}...", mp, fs_str),
                            });
                            let _ = usb_x_platform::format_drive_volume(mp, fs_str, &effective_label, *quick_format, cluster_size.to_bytes());
                            mp_final = mp.clone();
                            usb_x_platform::notify_drive_ready(&mp_final);
                        }

                        if mp_final.is_empty() {
                            if let Some(disk_num) = disk_num_opt.filter(|&n| n != 0) {
                                let _ = usb_x_platform::clean_and_repartition_disk(disk_num, device.size_bytes, req_fat32);
                                if let Some(cand) = usb_x_platform::wait_for_volume_ready(disk_num, 4000) {
                                    let _ = usb_x_platform::format_drive_volume(&cand, fs_str, &effective_label, *quick_format, cluster_size.to_bytes());
                                    mp_final = cand.clone();
                                    usb_x_platform::notify_drive_ready(&cand);
                                }
                            }
                        }

                        if mp_final.is_empty() {
                            return Err(UsbXError::Io {
                                operation: "prepare_target_volume".to_string(),
                                message: "Target volume could not be mounted.".to_string(),
                            });
                        }

                        crate::iso_extractor::copy_mounted_iso_to_target(&source_root, &mp_final, req_fat32, sender, cancelled)?;
                        write_extended_label_if_enabled(&mp_final, *extended_label, &raw_label, sender);
                        crate::iso_extractor::ensure_uefi_boot_fallback(&mp_final, sender);
                        usb_x_platform::notify_drive_ready(&mp_final);
                    }

                    drop(iso_mount);
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Dismounted virtual drive {}", source_root),
                    });
                }

                #[cfg(target_os = "linux")]
                {
                    let dev_node = device.id.as_str();
                    let is_gpt = matches!(partition_scheme, PartitionScheme::Gpt);
                    let req_fat32 = matches!(file_system, FileSystem::Fat32);

                    let _ = sender.send(OperationEvent::StageChanged {
                        stage: OperationStage::WritingImage,
                        message: "Mounting source ISO image natively...".to_string(),
                    });
                    let iso_mount = usb_x_platform::linux::mount_iso(&image.path)?;
                    let source_root = iso_mount.mount_point.to_string_lossy().to_string();
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Mounted source ISO to {}", source_root),
                    });

                    let iso_summary = crate::iso_extractor::inspect_mounted_iso(&source_root);
                    let is_win_effective = is_windows || iso_summary.has_windows_setup;

                    // Dual-partition NTFS Data + 1 MiB UEFI:NTFS (`uefi-ntfs.img`) single-pass extraction (matching Rufus)
                    let use_windows_dual_part = is_win_effective && (!req_fat32 || iso_summary.has_file_over_4gb);

                    let _ = backend.unmount_and_lock(&device.id);

                    if use_windows_dual_part {
                        const UEFI_NTFS_IMG: &[u8] = include_bytes!("../assets/uefi-ntfs.img");

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Partitioning,
                            message: format!(
                                "Partitioning {} (NTFS + UEFI:NTFS, {})...",
                                dev_node,
                                if is_gpt { "GPT" } else { "MBR" }
                            ),
                        });

                        let (part1, part2) = usb_x_platform::linux::create_windows_dual_partition_on_disk(
                            dev_node,
                            is_gpt,
                            device.size_bytes,
                            1,
                        )?;

                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!(
                                "Created NTFS + UEFI:NTFS partition layout on {}: {} (NTFS) + {} (UEFI:NTFS)",
                                dev_node, part1, part2
                            ),
                        });

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Writing UEFI:NTFS bootloader to {}...", part2),
                        });
                        usb_x_platform::linux::write_uefi_ntfs_partition(&part2, UEFI_NTFS_IMG)?;
                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!(
                                "Wrote UEFI:NTFS boot image to {} ({:.1} MB)",
                                part2,
                                UEFI_NTFS_IMG.len() as f64 / (1024.0 * 1024.0)
                            ),
                        });

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Formatting {} as NTFS ({})...", part1, raw_label),
                        });
                        usb_x_platform::linux::format_partition_volume_with_options(
                            &part1,
                            "NTFS",
                            &raw_label,
                            cluster_bytes_opt,
                            *quick_format,
                        )?;

                        // Extract full Windows ISO ONCE onto Partition 1 (NTFS Data) via native kernel ntfs3
                        {
                            let p1_guard = usb_x_platform::linux::mount_volume_fs(&part1, Some("NTFS"))?;
                            let p1_mp = p1_guard.mount_point.to_string_lossy().to_string();
                            mp_final = p1_mp.clone();

                            let _ = sender.send(OperationEvent::Log {
                                level: LogLevel::Info,
                                timestamp: "Now".to_string(),
                                message: format!("Target partition {} mounted at {}", part1, p1_mp),
                            });

                            crate::iso_extractor::copy_mounted_iso_to_target(&source_root, &p1_mp, false, sender, cancelled)?;
                            write_extended_label_if_enabled(&p1_mp, *extended_label, &raw_label, sender);
                            crate::iso_extractor::ensure_uefi_boot_fallback(&p1_mp, sender);

                            if !is_gpt {
                                match usb_x_platform::linux::install_legacy_bios_windows_grub(
                                    dev_node,
                                    &p1_guard.mount_point,
                                    Some(&part1),
                                    *rufus_mbr_bios_id,
                                ) {
                                    Ok(_) => {
                                        let _ = sender.send(OperationEvent::Log {
                                            level: LogLevel::Info,
                                            timestamp: "Now".to_string(),
                                            message: format!(
                                                "Installed Legacy BIOS MBR bootloader (GRUB -> BOOTMGR{}).",
                                                rufus_mbr_bios_id.map(|id| format!(", BIOS ID 0x{:02X}", id)).unwrap_or_default()
                                            ),
                                        });
                                    }
                                    Err(e) => {
                                        let _ = sender.send(OperationEvent::Log {
                                            level: LogLevel::Warn,
                                            timestamp: "Now".to_string(),
                                            message: format!("Legacy BIOS MBR notice: {}", e),
                                        });
                                    }
                                }
                            }

                            drop(p1_guard);
                        }
                    } else {
                        // Single-Partition Layout (or Linux ISO + Persistence Partition when linux_persistence_mb > 0)
                        let persist_mb_opt = if !is_win_effective {
                            linux_persistence_mb.filter(|&mb| mb > 0)
                        } else {
                            None
                        };

                        let (part_dev, persist_part_opt) = if let Some(pmb) = persist_mb_opt {
                            let _ = sender.send(OperationEvent::StageChanged {
                                stage: OperationStage::Partitioning,
                                message: format!(
                                    "Partitioning {} (ISO + {} MB persistence, {})...",
                                    dev_node, pmb, if is_gpt { "GPT" } else { "MBR" }
                                ),
                            });
                            let (p1, p2) = usb_x_platform::linux::create_partition_with_persistence_on_disk(
                                dev_node,
                                is_gpt,
                                req_fat32,
                                device.size_bytes,
                                pmb,
                            )?;
                            (p1, Some((p2, pmb)))
                        } else {
                            let _ = sender.send(OperationEvent::StageChanged {
                                stage: OperationStage::Partitioning,
                                message: format!(
                                    "Partitioning {} ({} capacity, {})...",
                                    dev_node,
                                    if *old_bios_fixes && !is_gpt { "MBR + old-BIOS alignment" } else { "100%" },
                                    if is_gpt { "GPT" } else { "MBR" }
                                ),
                            });
                            let p1 = usb_x_platform::linux::create_partition_on_disk_ex(
                                dev_node,
                                is_gpt,
                                req_fat32,
                                *old_bios_fixes,
                                device.size_bytes,
                            )?;
                            (p1, None)
                        };

                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!("Created partition {} on {}", part_dev, dev_node),
                        });

                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Formatting {} as {} ({})...", part_dev, fs_str, effective_label),
                        });
                        usb_x_platform::linux::format_partition_volume_with_options(
                            &part_dev,
                            fs_str,
                            &effective_label,
                            cluster_bytes_opt,
                            *quick_format,
                        )?;

                        let target_guard = usb_x_platform::linux::mount_volume_fs(&part_dev, Some(fs_str))?;
                        mp_final = target_guard.mount_point.to_string_lossy().to_string();

                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!("Target drive ready for writing: {}", mp_final),
                        });

                        crate::iso_extractor::copy_mounted_iso_to_target(&source_root, &mp_final, req_fat32, sender, cancelled)?;
                        write_extended_label_if_enabled(&mp_final, *extended_label, &raw_label, sender);

                        if is_win_effective && !is_gpt {
                            let _ = usb_x_platform::linux::install_legacy_bios_windows_grub(
                                dev_node,
                                &target_guard.mount_point,
                                Some(&part_dev),
                                *rufus_mbr_bios_id,
                            );
                        }

                        crate::iso_extractor::ensure_uefi_boot_fallback(&mp_final, sender);
                        drop(target_guard);

                        if let Some((persist_part, pmb)) = persist_part_opt {
                            let _ = sender.send(OperationEvent::StageChanged {
                                stage: OperationStage::CreatingPersistence,
                                message: format!("Formatting Linux persistence partition {} ({} MB ext4)...", persist_part, pmb),
                            });
                            let lower_p = image.path.to_ascii_lowercase();
                            let persist_label = if lower_p.contains("debian") || lower_p.contains("kali") {
                                "persistence"
                            } else {
                                "writable"
                            };
                            usb_x_platform::linux::setup_linux_persistence_partition(&persist_part, persist_label)?;
                            let _ = sender.send(OperationEvent::Log {
                                level: LogLevel::Info,
                                timestamp: "Now".to_string(),
                                message: format!(
                                    "Created ext4 persistence storage ({} MB, label '{}') on {}",
                                    pmb, persist_label, persist_part
                                ),
                            });
                        }
                    }

                    drop(iso_mount);
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Dismounted source ISO {}", source_root),
                    });

                    let _ = usb_x_platform::linux::unmount_device_all(device.id.as_str());
                }

                #[cfg(all(not(windows), not(target_os = "linux")))]
                {
                    let _ = (target_system, cluster_bytes_opt, quick_format, extended_label, old_bios_fixes, rufus_mbr_bios_id, linux_persistence_mb);
                    return Err(UsbXError::UnsupportedCapability(
                        "Native ISO flashing is not supported on this OS".to_string(),
                    ));
                }

                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Info,
                    timestamp: "Now".to_string(),
                    message: format!("ISO media created successfully ({})", mp_final),
                });
            }
        }
        OperationKind::FormatOnly {
            partition_scheme,
            file_system,
            volume_label,
            quick_format,
            extended_label,
            cluster_size,
            check_bad_blocks_passes,
            old_bios_fixes,
            rufus_mbr_bios_id: _,
        } => {
            if let Some(passes) = check_bad_blocks_passes {
                crate::flash_test::execute_bad_blocks_check(
                    backend,
                    &device.id,
                    device.size_bytes,
                    *passes,
                    sender,
                    cancelled,
                )?;
                let _ = backend.unmount_and_lock(&device.id);
            }

            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Partitioning,
                message: format!("Partitioning {} ({})...", device.device_path, partition_scheme.label()),
            });

            let fs_str = file_system.as_str();
            let is_fat32 = matches!(file_system, FileSystem::Fat32);

            #[cfg(windows)]
            {
                let disk_num_opt: Option<u32> = device.id.as_str()
                    .strip_prefix("PhysicalDrive")
                    .and_then(|s| s.parse::<u32>().ok());
                if let Some(disk_num) = disk_num_opt {
                    let _ = usb_x_platform::clean_and_repartition_disk(disk_num, device.size_bytes, is_fat32);
                    if let Some(cand) = usb_x_platform::wait_for_volume_ready(disk_num, 4000) {
                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Formatting {} as {} ({})...", cand, fs_str, volume_label),
                        });
                        let _ = usb_x_platform::format_drive_volume(&cand, fs_str, volume_label, *quick_format, cluster_size.to_bytes());
                        write_extended_label_if_enabled(&cand, *extended_label, volume_label, sender);
                        usb_x_platform::notify_drive_ready(&cand);
                    }
                }
            }

            #[cfg(target_os = "linux")]
            {
                let is_gpt = matches!(partition_scheme, PartitionScheme::Gpt);
                let part_dev = usb_x_platform::linux::create_partition_on_disk_ex(
                    device.id.as_str(),
                    is_gpt,
                    is_fat32,
                    *old_bios_fixes,
                    device.size_bytes,
                )?;

                let _ = sender.send(OperationEvent::StageChanged {
                    stage: OperationStage::Formatting,
                    message: format!("Formatting partition {} as {} ({})...", part_dev, fs_str, volume_label),
                });
                usb_x_platform::linux::format_partition_volume_with_options(
                    &part_dev,
                    fs_str,
                    volume_label,
                    format_cluster_opt(*cluster_size),
                    *quick_format,
                )?;

                if *extended_label && !volume_label.trim().is_empty() {
                    if let Ok(guard) = usb_x_platform::linux::mount_volume_fs(&part_dev, Some(fs_str)) {
                        let mp = guard.mount_point.to_string_lossy().to_string();
                        write_extended_label_if_enabled(&mp, true, volume_label, sender);
                        let _ = std::process::Command::new("sync").arg("-f").arg(&mp).status();
                    }
                }
                let _ = usb_x_platform::linux::unmount_device_all(device.id.as_str());
            }

            #[cfg(all(not(windows), not(target_os = "linux")))]
            let _ = (volume_label, quick_format, extended_label, cluster_size, old_bios_fixes, fs_str, is_fat32);

            backend.flush(&device.id)?;
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: format!("Formatted {} as {} ('{}'). Ready.", device.device_path, file_system.as_str(), volume_label),
            });
        }
        OperationKind::BackupDrive { output_file_path } => {
            crate::backup_restore::execute_backup_drive(
                backend,
                &device.id,
                device.size_bytes,
                output_file_path,
                sender,
                cancelled,
            )?;
        }
    }

    // Flush and finish
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::FlushingBuffers,
        message: "Synchronizing device buffers...".to_string(),
    });
    backend.flush(&device.id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn test_uefi_ntfs_img_embedded_and_writable() {
        const UEFI_NTFS_IMG: &[u8] = include_bytes!("../assets/uefi-ntfs.img");
        assert_eq!(UEFI_NTFS_IMG.len(), 1_048_576);
        assert_eq!(UEFI_NTFS_IMG[510], 0x55);
        assert_eq!(UEFI_NTFS_IMG[511], 0xAA);
        assert_eq!(&UEFI_NTFS_IMG[54..59], b"FAT12");

        let tmp = std::env::temp_dir().join(format!("pigeon_uefi_ntfs_test_{}.img", std::process::id()));
        std::fs::write(&tmp, vec![0u8; UEFI_NTFS_IMG.len()]).unwrap();
        usb_x_platform::linux::write_uefi_ntfs_partition(&tmp.to_string_lossy(), UEFI_NTFS_IMG).unwrap();
        let written = std::fs::read(&tmp).unwrap();
        assert_eq!(written, UEFI_NTFS_IMG);
        let _ = std::fs::remove_file(&tmp);
    }
}
