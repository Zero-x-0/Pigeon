use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationStage};

pub struct MountedIsoSummary {
    pub has_windows_setup: bool,
    pub has_file_over_4gb: bool,
}

fn rel_normalized_lower(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
}

pub fn inspect_mounted_iso(source_root: &str) -> MountedIsoSummary {
    let src_path = Path::new(source_root);
    let mut files = Vec::new();
    collect_files_recursive(src_path, src_path, &mut files);

    let mut has_install_image = false;
    let mut has_boot_wim = false;
    let mut has_bootmgr = false;
    let mut has_file_over_4gb = false;

    for f in &files {
        let lower = rel_normalized_lower(&f.rel_path);
        if lower == "sources/install.wim"
            || lower == "sources/install.esd"
            || lower == "sources/install.swm"
        {
            has_install_image = true;
        }
        if lower == "sources/boot.wim" {
            has_boot_wim = true;
        }
        if lower == "bootmgr" || lower == "bootmgr.efi" {
            has_bootmgr = true;
        }
        if f.size >= 4_294_967_295 {
            has_file_over_4gb = true;
        }
    }

    MountedIsoSummary {
        has_windows_setup: has_install_image || (has_boot_wim && has_bootmgr),
        has_file_over_4gb,
    }
}

pub fn copy_mounted_iso_to_target(
    source_root: &str,
    target_root: &str,
    is_fat32: bool,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let target_dir = Path::new(target_root);
    if !target_dir.exists() {
        return Err(UsbXError::Io {
            operation: "verify_target_directory".to_string(),
            message: format!("Target drive path does not exist: {}", target_root),
        });
    }
    perform_file_copy(source_root, target_root, is_fat32, sender, cancelled)
}

/// Find a file case-insensitively relative to `root` by walking path segments
fn find_file_case_insensitive(root: &Path, segments: &[&str]) -> Option<PathBuf> {
    let mut cur = root.to_path_buf();
    for seg in segments {
        let target_lower = seg.to_ascii_lowercase();
        let mut matched = None;
        if let Ok(entries) = fs::read_dir(&cur) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().to_ascii_lowercase() == target_lower {
                    matched = Some(entry.path());
                    break;
                }
            }
        }
        cur = matched?;
    }
    if cur.is_file() { Some(cur) } else { None }
}

/// Ensure `/EFI/BOOT/bootx64.efi` exists (case-insensitive check) for reliable UEFI booting
pub fn ensure_uefi_boot_fallback(target_root: &str, sender: &Sender<OperationEvent>) {
    let root = Path::new(target_root);
    if find_file_case_insensitive(root, &["EFI", "BOOT", "bootx64.efi"]).is_some() {
        return;
    }

    let candidate = find_file_case_insensitive(root, &["EFI", "BOOT", "grubx64.efi"])
        .or_else(|| find_file_case_insensitive(root, &["EFI", "Microsoft", "Boot", "bootmgfw.efi"]))
        .or_else(|| find_file_case_insensitive(root, &["bootmgr.efi"]));

    if let Some(src_efi) = candidate {
        let efi_boot_dir = root.join("EFI").join("BOOT");
        let _ = fs::create_dir_all(&efi_boot_dir);
        let dest = efi_boot_dir.join("bootx64.efi");
        if fs::copy(&src_efi, &dest).is_ok() {
            let _ = sender.send(OperationEvent::Log {
                level: LogLevel::Info,
                timestamp: "Now".to_string(),
                message: format!(
                    "Configured UEFI boot fallback: copied {} -> EFI/BOOT/bootx64.efi",
                    src_efi.file_name().unwrap_or_default().to_string_lossy()
                ),
            });
        }
    }
}

pub fn extract_iso_to_target(
    iso_path: &str,
    target_root: &str,
    is_fat32: bool,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let target_dir = Path::new(target_root);
    if !target_dir.exists() {
        return Err(UsbXError::Io {
            operation: "verify_target_directory".to_string(),
            message: format!("Target drive path does not exist: {}", target_root),
        });
    }

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::WritingImage,
        message: "Mounting source ISO image natively...".to_string(),
    });

    #[cfg(windows)]
    let (source_root, _mount_guard) = {
        let mount = usb_x_platform::mount_iso_virtdisk(iso_path)?;
        let root = mount.drive_root.clone();
        (root, mount)
    };

    #[cfg(target_os = "linux")]
    let (source_root, _mount_guard) = {
        let mount = usb_x_platform::linux::mount_iso(iso_path)?;
        let root = mount.mount_point.to_string_lossy().to_string();
        (root, Some(mount))
    };

    #[cfg(not(any(windows, target_os = "linux")))]
    let (source_root, _mount_guard) = (iso_path.to_string(), ());

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Mounted source ISO to virtual drive {}", source_root),
    });

    // Run file extraction inside a guard to ensure ISO is ALWAYS dismounted even on error
    let copy_result = perform_file_copy(&source_root, target_root, is_fat32, sender, cancelled);

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Dismounted virtual drive {}", source_root),
    });

    copy_result
}

struct FileEntry {
    rel_path: PathBuf,
    size: u64,
}

fn collect_files_recursive(dir: &Path, base: &Path, list: &mut Vec<FileEntry>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, base, list);
            } else if path.is_file() {
                if let Ok(meta) = path.metadata() {
                    if let Ok(rel) = path.strip_prefix(base) {
                        list.push(FileEntry {
                            rel_path: rel.to_path_buf(),
                            size: meta.len(),
                        });
                    }
                }
            }
        }
    }
}

fn format_rufus_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} bytes", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn perform_file_copy(
    source_root: &str,
    target_root: &str,
    is_fat32: bool,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let src_path = Path::new(source_root);
    let dst_path = Path::new(target_root);

    let mut files = Vec::new();
    collect_files_recursive(src_path, src_path, &mut files);
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let mut completed_bytes: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = Instant::now();
    let chunk_size = 4 * 1024 * 1024; // 4 MiB streaming buffer
    let mut buffer = vec![0u8; chunk_size];
    let mut bytes_since_sync: u64 = 0;

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::WritingImage,
        message: "Copying ISO files".to_string(),
    });

    for file_entry in &files {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Copying ISO files cancelled by user".to_string()));
        }

        let src_file_path = src_path.join(&file_entry.rel_path);
        let dst_file_path = dst_path.join(&file_entry.rel_path);

        if let Some(parent) = dst_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let rel_lower = rel_normalized_lower(&file_entry.rel_path);
        let is_wim = rel_lower.ends_with("install.wim") || rel_lower.ends_with("install.esd");

        if is_fat32 && is_wim && file_entry.size >= 4_294_967_295 {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::WritingImage,
                message: "FAT32 file limit exceeded (>4GB): Splitting install.wim into .swm parts for UEFI FAT32 compatibility...".to_string(),
            });

            let swm_dst = dst_file_path.with_extension("swm");
            #[cfg(windows)]
            let split_res = Command::new("dism.exe")
                .args(&[
                    "/Split-Image",
                    &format!("/ImageFile:{}", src_file_path.display()),
                    &format!("/SWMFile:{}", swm_dst.display()),
                    "/FileSize:3800",
                ])
                .output();

            #[cfg(not(windows))]
            let split_res = Command::new("wimlib-imagex")
                .args(&[
                    "split",
                    &src_file_path.to_string_lossy().to_string(),
                    &swm_dst.to_string_lossy().to_string(),
                    "3800",
                ])
                .output()
                .or_else(|_| {
                    Command::new("wimsplit")
                        .args(&[
                            &src_file_path.to_string_lossy().to_string(),
                            &swm_dst.to_string_lossy().to_string(),
                            "3800",
                        ])
                        .output()
                });

            if let Ok(out) = split_res {
                if out.status.success() {
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Successfully split WIM into FAT32-compatible .swm parts in {}", swm_dst.display()),
                    });
                    completed_bytes += file_entry.size;
                    continue;
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Warn,
                        timestamp: "Now".to_string(),
                        message: format!("WIM split warning (falling back to direct copy): {}", stderr.trim()),
                    });
                }
            }
        }

        let rel_unix = file_entry.rel_path.to_string_lossy().replace('\\', "/");
        let size_str = format_rufus_size(file_entry.size);
        let _ = sender.send(OperationEvent::Log {
            level: LogLevel::Info,
            timestamp: "Now".to_string(),
            message: format!("Extracting: /{} ({})", rel_unix, size_str),
        });

        let mut src_file = File::open(&src_file_path).map_err(|e| UsbXError::Io {
            operation: "open_iso_file".to_string(),
            message: format!("{}: {}", src_file_path.display(), e),
        })?;

        let mut dst_file = File::create(&dst_file_path).map_err(|e| UsbXError::Io {
            operation: "create_target_file".to_string(),
            message: format!("{}: {}", dst_file_path.display(), e),
        })?;

        #[cfg(target_os = "linux")]
        {
            usb_x_platform::linux::advise_sequential_read(&src_file);
            usb_x_platform::linux::preallocate_file(&dst_file, file_entry.size);
        }

        let mut file_remaining = file_entry.size;
        let mut file_offset: u64 = 0;
        let mut synced_offset: u64 = 0;

        let emit_progress = |done: u64, total: u64| {
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let bytes_per_sec = if elapsed_sec > 0.0 {
                (done as f64 / elapsed_sec) as u64
            } else {
                0
            };
            let remaining = total.saturating_sub(done);
            let eta = if bytes_per_sec > 0 {
                Some(remaining / bytes_per_sec)
            } else {
                None
            };
            let _ = sender.send(OperationEvent::Progress {
                completed_bytes: done,
                total_bytes: Some(total),
                bytes_per_second: Some(bytes_per_sec),
                eta_seconds: eta,
            });
        };

        while file_remaining > 0 {
            if cancelled.load(Ordering::Relaxed) {
                return Err(UsbXError::Cancelled("Copying ISO files cancelled by user".to_string()));
            }

            let to_read = (chunk_size as u64).min(file_remaining) as usize;
            let bytes_read = src_file.read(&mut buffer[..to_read]).map_err(|e| UsbXError::Io {
                operation: "read_iso_chunk".to_string(),
                message: e.to_string(),
            })?;

            if bytes_read == 0 {
                break;
            }

            dst_file.write_all(&buffer[..bytes_read]).map_err(|e| UsbXError::Io {
                operation: "write_target_chunk".to_string(),
                message: e.to_string(),
            })?;

            file_offset += bytes_read as u64;
            bytes_since_sync += bytes_read as u64;
            completed_bytes += bytes_read as u64;
            file_remaining = file_remaining.saturating_sub(bytes_read as u64);

            // Pipelined kernel writeback every 16 MiB globally (prevents small-file FTL stalls while keeping progress honest)
            if bytes_since_sync >= 16 * 1024 * 1024 {
                #[cfg(target_os = "linux")]
                if file_offset > synced_offset {
                    usb_x_platform::linux::sync_and_drop_range(&dst_file, synced_offset, file_offset - synced_offset);
                    synced_offset = file_offset;
                } else {
                    let _ = dst_file.sync_data();
                }
                #[cfg(not(target_os = "linux"))]
                let _ = dst_file.sync_data();
                bytes_since_sync = 0;
            } else {
                #[cfg(target_os = "linux")]
                if file_offset.saturating_sub(synced_offset) >= 8 * 1024 * 1024 {
                    usb_x_platform::linux::trigger_async_writeback(&dst_file, synced_offset, file_offset - synced_offset);
                }
            }

            if last_progress_time.elapsed().as_millis() >= 100 || completed_bytes == total_bytes {
                emit_progress(completed_bytes, total_bytes);
                last_progress_time = Instant::now();
            }
        }

        if file_entry.size == 0 && (last_progress_time.elapsed().as_millis() >= 100 || completed_bytes == total_bytes) {
            emit_progress(completed_bytes, total_bytes);
            last_progress_time = Instant::now();
        }

        #[cfg(target_os = "linux")]
        if file_offset > synced_offset && file_entry.size >= 8 * 1024 * 1024 {
            usb_x_platform::linux::sync_and_drop_range(&dst_file, synced_offset, file_offset - synced_offset);
        }
    }

    // Final filesystem sync once at end
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("sync").arg("-f").arg(target_root).status();

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::FlushingBuffers,
        message: "Finalizing USB filesystem...".to_string(),
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_windows_iso_inspection_and_uefi_fallback() {
        let base = std::env::temp_dir().join(format!("pigeon_win_iso_test_{}", std::process::id()));
        let src = base.join("iso");
        let dst = base.join("usb");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(src.join("efi/microsoft/boot")).unwrap();
        fs::create_dir_all(src.join("boot")).unwrap();
        fs::create_dir_all(src.join("sources")).unwrap();
        fs::create_dir_all(&dst).unwrap();

        fs::write(src.join("efi/microsoft/boot/bootmgfw.efi"), vec![1u8; 64]).unwrap();
        fs::write(src.join("efi/microsoft/boot/bcd"), vec![2u8; 32]).unwrap();
        fs::write(src.join("boot/boot.sdi"), vec![3u8; 32]).unwrap();
        fs::write(src.join("sources/boot.wim"), vec![4u8; 128]).unwrap();
        fs::write(src.join("sources/install.wim"), vec![5u8; 256]).unwrap();
        fs::write(src.join("bootmgr"), vec![6u8; 16]).unwrap();

        let summary = inspect_mounted_iso(&src.to_string_lossy());
        assert!(summary.has_windows_setup);
        assert!(!summary.has_file_over_4gb);

        let (tx, _rx) = channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        copy_mounted_iso_to_target(
            &src.to_string_lossy(),
            &dst.to_string_lossy(),
            false,
            &tx,
            &cancelled,
        )
        .unwrap();
        ensure_uefi_boot_fallback(&dst.to_string_lossy(), &tx);

        assert!(dst.join("EFI/BOOT/bootx64.efi").exists());
        assert!(dst.join("sources/boot.wim").exists());
        assert!(dst.join("sources/install.wim").exists());
        assert!(dst.join("boot/boot.sdi").exists());
        assert!(dst.join("bootmgr").exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_single_pass_iso_copy_emits_rufus_logs_without_stage_spam() {
        let base = std::env::temp_dir().join(format!("pigeon_single_pass_iso_test_{}", std::process::id()));
        let src = base.join("iso");
        let dst = base.join("usb");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(src.join("sources")).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("bootmgr"), vec![1u8; 64]).unwrap();
        fs::write(src.join("sources/boot.wim"), vec![2u8; 2048]).unwrap();

        let (tx, rx) = channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        copy_mounted_iso_to_target(
            &src.to_string_lossy(),
            &dst.to_string_lossy(),
            false,
            &tx,
            &cancelled,
        )
        .unwrap();

        let events: Vec<OperationEvent> = rx.try_iter().collect();
        let copy_stage_count = events
            .iter()
            .filter(|e| matches!(e, OperationEvent::StageChanged { message, .. } if message.starts_with("Copying ISO files")))
            .count();
        assert_eq!(copy_stage_count, 1, "progress percentages must never spam StageChanged/logs");

        let extracting_logs: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                OperationEvent::Log { message, .. } if message.starts_with("Extracting:") => Some(message.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(extracting_logs.len(), 2);
        assert!(extracting_logs.iter().any(|l| l == "Extracting: /bootmgr (64 bytes)"));
        assert!(extracting_logs.iter().any(|l| l == "Extracting: /sources/boot.wim (2.0 KB)"));

        let _ = fs::remove_dir_all(&base);
    }
}

