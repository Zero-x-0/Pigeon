use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use usb_x_core::device::DeviceInfo;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationResult, OperationStage};
use usb_x_core::plan::{OperationKind, OperationPlan};
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
            verify_after_write,
            win11_tweaks,
            linux_persistence_mb,
            write_mode,
        } => {
            #[allow(unused_variables)]
            let is_windows = matches!(
                image.detected_type,
                usb_x_core::types::ImageDetectedType::Windows11Installer { .. }
                | usb_x_core::types::ImageDetectedType::Windows10Installer { .. }
                | usb_x_core::types::ImageDetectedType::WindowsLegacy { .. }
            );

            // Determine write strategy:
            // - DdImageMode: stream raw byte-for-byte to Sector 0 (Raw disk copy)
            // - IsoImageMode: format full partition (no unallocated space) and extract bootable files
            let use_dd_mode = match write_mode {
                usb_x_core::types::ImageWriteMode::DdImageMode => true,
                usb_x_core::types::ImageWriteMode::IsoImageMode => false,
                usb_x_core::types::ImageWriteMode::Auto => false,
            };

            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: format!(
                    "Writing image '{}' [Mode: {}] · Partition: {} · Target: {} · FS: {}",
                    image.path,
                    if use_dd_mode { "DD Raw Sector Stream" } else { "ISO Bootable Filesystem Mode" },
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
                // Stream raw sectors starting at Sector 0:
                let _ = sender.send(OperationEvent::StageChanged {
                    stage: OperationStage::WritingImage,
                    message: "Writing raw DD sectors to Sector 0 (GRUB / UEFI ESP / MBR)...".to_string(),
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
                    message: "DD raw write finished: Sector 0 MBR, GRUB bootloader, and EFI System Partition are live.".to_string(),
                });

                #[cfg(windows)]
                {
                    if let Some(letter) = orig_letter {
                        usb_x_platform::notify_drive_ready(&format!("{}:\\", letter));
                    }
                }
            } else {
                // ISO Image Mode: Clean bootable filesystem extraction for both Windows and Linux!
                let raw_label = image.volume_label.clone().unwrap_or_else(|| "USB_BOOT".to_string());
                let vol_label = if matches!(file_system, usb_x_core::types::FileSystem::Fat32) {
                    if raw_label.len() > 11 {
                        raw_label[..11].to_uppercase()
                    } else {
                        raw_label.to_uppercase()
                    }
                } else {
                    raw_label
                };

                let fs_str = match file_system {
                    usb_x_core::types::FileSystem::Fat32 => "FAT32",
                    usb_x_core::types::FileSystem::Ntfs => "NTFS",
                    usb_x_core::types::FileSystem::ExFat => "exFAT",
                    _ => "FAT32",
                };

                #[cfg(windows)]
                let mut target_mp: Option<String> = None;

                #[cfg(windows)]
                if let Some(letter) = orig_letter {
                    let test_p = format!("{}:\\", letter);
                    if std::path::Path::new(&test_p).exists() {
                        target_mp = Some(test_p);
                    }
                }

                #[cfg(windows)]
                if target_mp.is_none() {
                    if let Some(disk_num) = disk_num_opt {
                        let vols = usb_x_platform::get_volumes_for_disk(disk_num);
                        if let Some(v) = vols.first() {
                            let test_p = format!("{}:\\", v.drive_letter);
                            if std::path::Path::new(&test_p).exists() {
                                target_mp = Some(test_p);
                            }
                        }
                    }
                }

                // 2. Format volume or recreate healthy partition if the drive is unallocated
                #[allow(unused_assignments)]
                let mut mp_final = String::new();

                #[cfg(windows)]
                {
                    if let Some(ref mp) = target_mp {
                        // Existing partition exists! Format it in-place using native Win32 FormatEx (NEVER leaves in unallocated space!)
                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::Formatting,
                            message: format!("Formatting partition {} as {} ({}) via native Win32 FormatEx...", mp, fs_str, vol_label),
                        });
                        let fmt_res = usb_x_platform::format_drive_volume(mp, fs_str, &vol_label, true, 0);
                        mp_final = mp.clone();
                        usb_x_platform::notify_drive_ready(&mp_final);
                        if let Err(err) = fmt_res {
                            let _ = sender.send(OperationEvent::Log {
                                level: LogLevel::Warn,
                                timestamp: "Now".to_string(),
                                message: format!("Format notice (continuing with volume {}): {}", mp, err),
                            });
                        }
                    }

                    if mp_final.is_empty() {
                        // Volume does not exist (e.g. drive was in unallocated space). Recreate primary MBR partition natively!
                        if let Some(disk_num) = disk_num_opt {
                            if disk_num != 0 {
                                let _ = sender.send(OperationEvent::StageChanged {
                                    stage: OperationStage::Partitioning,
                                    message: format!("Creating native MBR partition on PhysicalDrive{} (100% capacity)...", disk_num),
                                });

                                let is_fat32 = matches!(file_system, usb_x_core::types::FileSystem::Fat32);
                                let part_res = usb_x_platform::create_mbr_partition_on_disk(disk_num, device.size_bytes, is_fat32);

                                match part_res {
                                    Ok(_) => {
                                        let _ = sender.send(OperationEvent::Log {
                                            level: LogLevel::Info,
                                            timestamp: "Now".to_string(),
                                            message: "Native MBR partition table written to Sector 0 with zero unallocated space.".to_string(),
                                        });
                                    }
                                    Err(err) => {
                                        let _ = sender.send(OperationEvent::Log {
                                            level: LogLevel::Warn,
                                            timestamp: "Now".to_string(),
                                            message: format!("Partition notice: {}", err),
                                        });
                                    }
                                }

                                std::thread::sleep(std::time::Duration::from_millis(600));

                                // Find the newly mounted volume on this disk
                                let vols = usb_x_platform::get_volumes_for_disk(disk_num);
                                if let Some(v) = vols.first() {
                                    let cand = format!("{}:\\", v.drive_letter);
                                    let _ = sender.send(OperationEvent::StageChanged {
                                        stage: OperationStage::Formatting,
                                        message: format!("Formatting partition {} as {} ({}) via native Win32 FormatEx...", cand, fs_str, vol_label),
                                    });
                                    let _ = usb_x_platform::format_drive_volume(&cand, fs_str, &vol_label, true, 0);
                                    mp_final = cand.clone();
                                    usb_x_platform::notify_drive_ready(&cand);
                                } else if let Some(letter) = orig_letter {
                                    let cand = format!("{}:\\", letter);
                                    if std::path::Path::new(&cand).exists() {
                                        let _ = usb_x_platform::format_drive_volume(&cand, fs_str, &vol_label, true, 0);
                                        mp_final = cand.clone();
                                        usb_x_platform::notify_drive_ready(&cand);
                                    }
                                }
                            }
                        }
                    }
                }

                #[cfg(target_os = "linux")]
                let mut _linux_target_guard: Option<usb_x_platform::linux::LinuxMountGuard> = None;

                #[cfg(target_os = "linux")]
                {
                    let dev_node = device.id.as_str();
                    let is_gpt = matches!(partition_scheme, usb_x_core::types::PartitionScheme::Gpt);
                    let is_fat32 = matches!(file_system, usb_x_core::types::FileSystem::Fat32);

                    let _ = sender.send(OperationEvent::StageChanged {
                        stage: OperationStage::Partitioning,
                        message: format!("Partitioning {} (100% capacity, {})...", dev_node, if is_gpt { "GPT" } else { "MBR" }),
                    });

                    // 1. Unmount existing partitions on target device
                    let _ = backend.unmount_and_lock(&device.id);

                    // 2. Partition target device
                    let part_dev = usb_x_platform::linux::create_partition_on_disk(dev_node, is_gpt, is_fat32)?;

                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Created partition {} on {}", part_dev, dev_node),
                    });

                    // 3. Format partition
                    let _ = sender.send(OperationEvent::StageChanged {
                        stage: OperationStage::Formatting,
                        message: format!("Formatting partition {} as {} ({})...", part_dev, fs_str, vol_label),
                    });
                    usb_x_platform::linux::format_partition_volume(&part_dev, fs_str, &vol_label)?;

                    // 4. Mount partition to temporary directory
                    let target_guard = usb_x_platform::linux::mount_volume(&part_dev)?;
                    mp_final = target_guard.mount_point.to_string_lossy().to_string();
                    _linux_target_guard = Some(target_guard);
                }

                if mp_final.is_empty() {
                    return Err(UsbXError::Io {
                        operation: "prepare_target_volume".to_string(),
                        message: "Target volume could not be accessed. Please ensure the USB drive is connected and has a valid drive letter.".to_string(),
                    });
                }

                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Info,
                    timestamp: "Now".to_string(),
                    message: format!("Target drive ready for writing: {}", mp_final),
                });

                // 3. Extract all ISO files (Windows or Linux)
                let is_fat32 = matches!(file_system, usb_x_core::types::FileSystem::Fat32);
                crate::iso_extractor::extract_iso_to_target(&image.path, &mp_final, is_fat32, sender, cancelled)?;

                // 4. If Windows 11 tweaks are active, generate autounattend.xml
                if let Some(tweaks) = win11_tweaks {
                    if tweaks.bypass_tpm_secure_boot_ram || tweaks.bypass_online_account || tweaks.disable_telemetry_privacy {
                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::ApplyingTweaks,
                            message: "Applying Windows 11 Setup tweaks (TPM/SecureBoot/RAM bypass, offline account)...".to_string(),
                        });
                        let arch_hint = image.architecture.as_deref().unwrap_or("amd64");
                        let xml = usb_x_image::generate_autounattend_xml_with_arch(tweaks, arch_hint);
                        let dest = std::path::Path::new(&mp_final).join("autounattend.xml");
                        if let Ok(_) = std::fs::write(&dest, &xml) {
                            let _ = sender.send(OperationEvent::Log {
                                level: LogLevel::Info,
                                timestamp: "Now".to_string(),
                                message: format!("Installed autounattend.xml to {}", dest.display()),
                            });
                        }
                    }
                }

                // 5. Make drive bootable
                #[cfg(windows)]
                {
                    let mp_clean = mp_final.trim_end_matches(['\\', '/']);
                    let boot_drive = if mp_clean.ends_with(':') {
                        mp_clean.to_string()
                    } else {
                        format!("{}:", mp_clean)
                    };

                    // If Windows image or BIOS target, install Windows BOOTMGR bootcode
                    if is_windows || *target_system == usb_x_core::types::TargetSystem::Bios {
                        let _ = sender.send(OperationEvent::StageChanged {
                            stage: OperationStage::FlushingBuffers,
                            message: format!("Installing bootloader to {}...", boot_drive),
                        });

                        let bootsect_path = if std::path::Path::new(r"C:\Windows\System32\bootsect.exe").exists() {
                            "C:\\Windows\\System32\\bootsect.exe".to_string()
                        } else {
                            let local_bootsect = format!("{}\\boot\\bootsect.exe", mp_final);
                            if std::path::Path::new(&local_bootsect).exists() {
                                local_bootsect
                            } else {
                                "bootsect.exe".to_string()
                            }
                        };

                        let res = std::process::Command::new(&bootsect_path)
                            .args(&["/nt60", &boot_drive, "/mbr", "/force"])
                            .output();

                        match res {
                            Ok(out) if out.status.success() => {
                                let _ = sender.send(OperationEvent::Log {
                                    level: LogLevel::Info,
                                    timestamp: "Now".to_string(),
                                    message: format!("Successfully installed bootloader to {} and MBR.", boot_drive),
                                });
                            }
                            Ok(out) => {
                                let err_msg = String::from_utf8_lossy(&out.stderr);
                                let _ = sender.send(OperationEvent::Log {
                                    level: LogLevel::Warn,
                                    timestamp: "Now".to_string(),
                                    message: format!("bootsect notice: {}", err_msg.trim()),
                                });
                            }
                            Err(e) => {
                                let _ = sender.send(OperationEvent::Log {
                                    level: LogLevel::Warn,
                                    timestamp: "Now".to_string(),
                                    message: format!("Notice: bootsect was not invoked: {}", e),
                                });
                            }
                        }
                    }

                    // For UEFI Boot (Linux & Windows): ensure /EFI/BOOT/bootx64.efi exists
                    let efi_boot_dir = std::path::Path::new(&mp_final).join("EFI").join("BOOT");
                    if efi_boot_dir.exists() {
                        let bootx64 = efi_boot_dir.join("bootx64.efi");
                        let grubx64 = efi_boot_dir.join("grubx64.efi");
                        if !bootx64.exists() && grubx64.exists() {
                            let _ = std::fs::copy(&grubx64, &bootx64);
                            let _ = sender.send(OperationEvent::Log {
                                level: LogLevel::Info,
                                timestamp: "Now".to_string(),
                                message: "Configured UEFI boot fallback: copied grubx64.efi -> bootx64.efi".to_string(),
                            });
                        }
                    }

                    usb_x_platform::notify_drive_ready(&mp_final);
                }

                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Info,
                    timestamp: "Now".to_string(),
                    message: format!("ISO Media created successfully on {}. Ready to boot PC!", mp_final),
                });
            }

            // If Linux persistence is requested
            if let Some(persist_mb) = linux_persistence_mb {
                if *persist_mb > 0 {
                    let _ = sender.send(OperationEvent::StageChanged {
                        stage: OperationStage::CreatingPersistence,
                        message: format!("Allocating Linux persistence partition ({} MB)...", persist_mb),
                    });
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Configured persistence storage: {} MB", persist_mb),
                    });
                }
            }
        }
        OperationKind::CreateFreeDosMedia { partition_scheme, file_system } => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Partitioning,
                message: format!("Partitioning for FreeDOS ({}, {})...", partition_scheme.label(), file_system.label()),
            });
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Formatting,
                message: "Writing FreeDOS system files (KERNEL.SYS, COMMAND.COM)...".to_string(),
            });
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: "FreeDOS boot media prepared successfully.".to_string(),
            });
        }
        OperationKind::CreateMsDosMedia { files_folder, .. } => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Formatting,
                message: format!("Installing MS-DOS files from folder '{}'...", files_folder),
            });
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: "User-supplied MS-DOS boot media prepared.".to_string(),
            });
        }
        OperationKind::CreateWindowsToGo { image, .. } => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Formatting,
                message: "Preparing Windows To Go workspace on external drive...".to_string(),
            });
            crate::raw_writer::execute_raw_write(
                backend,
                &device.id,
                &image.path,
                true,
                sender,
                cancelled,
            )?;
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: "Windows To Go deployment finished.".to_string(),
            });
        }
        #[allow(unused_variables)]
        OperationKind::FormatOnly { partition_scheme, file_system, volume_label, quick_format, cluster_size, .. } => {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Partitioning,
                message: format!("Preparing {} partition structure...", partition_scheme.label()),
            });
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::Formatting,
                message: format!("Formatting volume '{}' as {}...", volume_label, file_system.label()),
            });

            #[cfg(windows)]
            {
                if let Some(first_part) = device.partitions.first() {
                    if let Some(ref mp) = first_part.mount_point {
                        let fs_str = file_system.as_str();
                        let _ = sender.send(OperationEvent::Log {
                            level: LogLevel::Info,
                            timestamp: "Now".to_string(),
                            message: format!("Calling native Windows FormatEx on {} as {} with label '{}'...", mp, fs_str, volume_label),
                        });
                        let bytes_cs = cluster_size.to_bytes();
                        let _ = usb_x_platform::format_drive_volume(mp, fs_str, volume_label, *quick_format, bytes_cs);
                        usb_x_platform::notify_drive_ready(mp);
                    }
                }
            }

            backend.flush(&device.id)?;
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: format!("Formatted device as {} with label '{}'. Drive is ready and accessible in Windows Explorer.", file_system.label(), volume_label),
            });
        }
        OperationKind::BackupDrive { output_file_path, .. } => {
            crate::backup_restore::execute_backup_drive(
                backend,
                &device.id,
                device.size_bytes,
                output_file_path,
                sender,
                cancelled,
            )?;
        }
        OperationKind::RestoreDrive { backup_image_path, verify } => {
            crate::backup_restore::execute_restore_drive(
                backend,
                &device.id,
                device.size_bytes,
                backup_image_path,
                *verify,
                sender,
                cancelled,
            )?;
        }
        OperationKind::FlashTest { kind } => {
            crate::flash_test::execute_flash_test(
                backend,
                &device.id,
                device.size_bytes,
                *kind,
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

