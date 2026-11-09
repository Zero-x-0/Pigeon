use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;
use usb_x_core::device::DeviceId;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationStage};
use usb_x_image::hash::Sha256;
use usb_x_platform::StorageBackend;

pub fn execute_raw_write(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    source_path: &str,
    verify: bool,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let path = Path::new(source_path);
    let mut file = File::open(path).map_err(|e| UsbXError::Io {
        operation: "open_image_file".to_string(),
        message: e.to_string(),
    })?;

    let total_bytes = file.metadata().map_err(|e| UsbXError::Io {
        operation: "file_metadata".to_string(),
        message: e.to_string(),
    })?.len();

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::WritingImage,
        message: format!("Writing image data: 0 / {} bytes", total_bytes),
    });

    let chunk_size = 1024 * 1024; // 1 MiB chunk
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;
    let start_time = Instant::now();
    let mut last_progress_time = Instant::now();

    let mut write_hasher = Sha256::new();

    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Write cancelled by user".to_string()));
        }

        let bytes_read = file.read(&mut buffer).map_err(|e| UsbXError::Io {
            operation: "read_source_image".to_string(),
            message: e.to_string(),
        })?;

        if bytes_read == 0 {
            break;
        }

        let chunk = &buffer[..bytes_read];
        write_hasher.update(chunk);

        backend.write_sectors(target_id, written, chunk)?;
        written += bytes_read as u64;

        if last_progress_time.elapsed().as_millis() >= 100 || written == total_bytes {
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let bytes_per_sec = if elapsed_sec > 0.0 {
                (written as f64 / elapsed_sec) as u64
            } else {
                0
            };

            let remaining_bytes = total_bytes.saturating_sub(written);
            let eta = if bytes_per_sec > 0 {
                Some(remaining_bytes / bytes_per_sec)
            } else {
                None
            };

            let _ = sender.send(OperationEvent::Progress {
                completed_bytes: written,
                total_bytes: Some(total_bytes),
                bytes_per_second: Some(bytes_per_sec),
                eta_seconds: eta,
            });

            last_progress_time = Instant::now();
        }
    }

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::FlushingBuffers,
        message: "Flushing buffers to physical device...".to_string(),
    });
    backend.flush(target_id)?;

    let source_sha256 = write_hasher.finalize_hex();
    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Write complete ({} bytes). Source SHA256: {}", written, source_sha256),
    });

    if verify {
        let _ = sender.send(OperationEvent::StageChanged {
            stage: OperationStage::Verifying,
            message: "Verifying written data against source image...".to_string(),
        });

        let mut read_hasher = Sha256::new();
        let mut read_back: u64 = 0;
        let mut verify_buffer = vec![0u8; chunk_size];

        while read_back < written {
            if cancelled.load(Ordering::Relaxed) {
                return Err(UsbXError::Cancelled("Verification cancelled by user".to_string()));
            }

            let to_read = (chunk_size as u64).min(written - read_back) as usize;
            let read_slice = &mut verify_buffer[..to_read];
            backend.read_sectors(target_id, read_back, read_slice)?;
            read_hasher.update(read_slice);
            read_back += to_read as u64;

            let _ = sender.send(OperationEvent::Progress {
                completed_bytes: read_back,
                total_bytes: Some(written),
                bytes_per_second: None,
                eta_seconds: None,
            });
        }

        let target_sha256 = read_hasher.finalize_hex();
        if source_sha256 != target_sha256 {
            return Err(UsbXError::VerificationMismatch {
                expected: source_sha256,
                actual: target_sha256,
            });
        }

        let _ = sender.send(OperationEvent::Log {
            level: LogLevel::Info,
            timestamp: "Now".to_string(),
            message: format!("Verification PASSED! Target SHA256: {}", target_sha256),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use usb_x_platform::VirtualDriveBackend;

    #[test]
    fn test_raw_write_and_verify() {
        let temp_dir = std::env::temp_dir();
        let src_file = temp_dir.join("usb_x_test_src.bin");
        let vdisk_file = temp_dir.join("usb_x_test_dest.img");

        let test_payload = vec![0x42u8; 16 * 1024]; // 16 KiB
        {
            let mut f = File::create(&src_file).unwrap();
            f.write_all(&test_payload).unwrap();
        }

        let mut vdisk = VirtualDriveBackend::open_or_create(&vdisk_file.to_string_lossy(), 64 * 1024).unwrap();
        let target_id = vdisk.enumerate_devices().unwrap()[0].id.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));

        let res = execute_raw_write(
            &mut vdisk,
            &target_id,
            &src_file.to_string_lossy(),
            true, // verify sha256
            &tx,
            &cancelled,
        );

        assert!(res.is_ok());

        // Drain events to verify completion
        let mut got_pass = false;
        while let Ok(evt) = rx.try_recv() {
            if let OperationEvent::Log { message, .. } = evt {
                if message.contains("Verification PASSED") {
                    got_pass = true;
                }
            }
        }
        assert!(got_pass);

        let _ = std::fs::remove_file(src_file);
        let _ = std::fs::remove_file(vdisk_file);
    }
}
