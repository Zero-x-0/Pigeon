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

    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
    let mut completed_bytes: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = Instant::now();
    let chunk_size = 1024 * 1024; // 1 MiB buffer
    let mut buffer = vec![0u8; chunk_size];

    for file_entry in &files {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Copying ISO files cancelled by user".to_string()));
        }

        let src_file_path = src_path.join(&file_entry.rel_path);
        let dst_file_path = dst_path.join(&file_entry.rel_path);

        // Ensure target subdirectory exists
        if let Some(parent) = dst_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let rel_lower = file_entry.rel_path.to_string_lossy().to_ascii_lowercase();
        let is_wim = rel_lower.ends_with("install.wim") || rel_lower.ends_with("install.esd");

        // Only split install.wim/.esd if target filesystem is FAT32 AND file size exceeds FAT32 4GB limit (4,294,967,295 bytes).
        // On NTFS or exFAT, large files (>4GB) are natively supported so we copy directly without splitting!
        if is_fat32 && is_wim && file_entry.size >= 4_294_967_295 {
            let _ = sender.send(OperationEvent::StageChanged {
                stage: OperationStage::WritingImage,
                message: "FAT32 file limit exceeded (>4GB): Splitting install.wim into .swm parts for UEFI FAT32 compatibility...".to_string(),
            });

            let swm_dst = dst_file_path.with_extension("swm");
            let dism_res = Command::new("dism.exe")
                .args(&[
                    "/Split-Image",
                    &format!("/ImageFile:{}", src_file_path.display()),
                    &format!("/SWMFile:{}", swm_dst.display()),
                    "/FileSize:3800",
                ])
                .output();

            if let Ok(out) = dism_res {
                if out.status.success() {
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Info,
                        timestamp: "Now".to_string(),
                        message: format!("Successfully split install.wim into FAT32-compatible .swm parts in {}", swm_dst.display()),
                    });
                    completed_bytes += file_entry.size;
                    continue;
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Warn,
                        timestamp: "Now".to_string(),
                        message: format!("dism.exe split warning (falling back to direct copy): {}", stderr.trim()),
                    });
                }
            }
        }

        let mut src_file = File::open(&src_file_path).map_err(|e| UsbXError::Io {
            operation: "open_iso_file".to_string(),
            message: format!("{}: {}", src_file_path.display(), e),
        })?;

        let mut dst_file = File::create(&dst_file_path).map_err(|e| UsbXError::Io {
            operation: "create_target_file".to_string(),
            message: format!("{}: {}", dst_file_path.display(), e),
        })?;

        let mut file_remaining = file_entry.size;

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

            completed_bytes += bytes_read as u64;
            file_remaining = file_remaining.saturating_sub(bytes_read as u64);

            if last_progress_time.elapsed().as_millis() >= 80 || completed_bytes == total_bytes {
                let elapsed_sec = start_time.elapsed().as_secs_f64();
                let bytes_per_sec = if elapsed_sec > 0.0 {
                    (completed_bytes as f64 / elapsed_sec) as u64
                } else {
                    0
                };

                let remaining = total_bytes.saturating_sub(completed_bytes);
                let eta = if bytes_per_sec > 0 {
                    Some(remaining / bytes_per_sec)
                } else {
                    None
                };

                let pct = if total_bytes > 0 {
                    (completed_bytes as f64 / total_bytes as f64) * 100.0
                } else {
                    100.0
                };

                let size_mb = file_entry.size as f64 / (1024.0 * 1024.0);
                let size_str = if size_mb >= 1024.0 {
                    format!("{:.1} GB", size_mb / 1024.0)
                } else {
                    format!("{:.1} MB", size_mb)
                };

                let _ = sender.send(OperationEvent::StageChanged {
                    stage: OperationStage::WritingImage,
                    message: format!("Copying ISO files: {:.1}%", pct),
                });

                let _ = sender.send(OperationEvent::Progress {
                    completed_bytes,
                    total_bytes: Some(total_bytes),
                    bytes_per_second: Some(bytes_per_sec),
                    eta_seconds: eta,
                });

                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Debug,
                    timestamp: "Now".to_string(),
                    message: format!("{}:\\{} ({})", target_root.trim_end_matches(['\\', '/']), file_entry.rel_path.display(), size_str),
                });

                last_progress_time = Instant::now();
            }
        }

        let _ = dst_file.flush();
    }

    Ok(())
}
