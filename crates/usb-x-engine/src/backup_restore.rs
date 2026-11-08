use std::fs::File;
use std::io::Write;
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

pub fn execute_backup_drive(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    total_bytes: u64,
    output_path: &str,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let mut out_file = File::create(output_path).map_err(|e| UsbXError::Io {
        operation: "create_backup_file".to_string(),
        message: e.to_string(),
    })?;

    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::WritingImage,
        message: format!("Backing up drive to image: {} bytes", total_bytes),
    });

    let chunk_size = 1024 * 1024; // 1 MiB chunk
    let mut buffer = vec![0u8; chunk_size];
    let mut read_bytes: u64 = 0;
    let start_time = Instant::now();
    let mut hasher = Sha256::new();

    while read_bytes < total_bytes {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Backup cancelled by user".to_string()));
        }

        let to_read = (chunk_size as u64).min(total_bytes - read_bytes) as usize;
        let slice = &mut buffer[..to_read];
        backend.read_sectors(target_id, read_bytes, slice)?;

        out_file.write_all(slice).map_err(|e| UsbXError::Io {
            operation: "write_backup_chunk".to_string(),
            message: e.to_string(),
        })?;

        hasher.update(slice);
        read_bytes += to_read as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            (read_bytes as f64 / elapsed) as u64
        } else {
            0
        };

        let remaining = total_bytes.saturating_sub(read_bytes);
        let eta = if speed > 0 { Some(remaining / speed) } else { None };

        let _ = sender.send(OperationEvent::Progress {
            completed_bytes: read_bytes,
            total_bytes: Some(total_bytes),
            bytes_per_second: Some(speed),
            eta_seconds: eta,
        });
    }

    out_file.sync_all().map_err(|e| UsbXError::Io {
        operation: "sync_backup".to_string(),
        message: e.to_string(),
    })?;

    let digest = hasher.finalize_hex();
    let manifest_path = format!("{}.manifest.json", output_path);
    let manifest_content = format!(
        "{{\n  \"backup_file\": \"{}\",\n  \"total_bytes\": {},\n  \"sha256\": \"{}\"\n}}",
        output_path, total_bytes, digest
    );

    let _ = std::fs::write(&manifest_path, manifest_content);

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Backup completed! SHA256: {}. Saved to {}", digest, output_path),
    });

    Ok(())
}

pub fn execute_restore_drive(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    target_size: u64,
    backup_path: &str,
    verify: bool,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let path = Path::new(backup_path);
    let file = File::open(path).map_err(|e| UsbXError::Io {
        operation: "open_backup_for_restore".to_string(),
        message: e.to_string(),
    })?;

    let backup_size = file.metadata().map_err(|e| UsbXError::Io {
        operation: "metadata".to_string(),
        message: e.to_string(),
    })?.len();

    if backup_size > target_size {
        return Err(UsbXError::SafetyViolation(format!(
            "Backup image size ({} bytes) is larger than target device ({} bytes)",
            backup_size, target_size
        )));
    }

    crate::raw_writer::execute_raw_write(
        backend,
        target_id,
        backup_path,
        verify,
        sender,
        cancelled,
    )
}
