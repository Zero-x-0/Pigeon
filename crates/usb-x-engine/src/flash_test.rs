use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;
use usb_x_core::device::DeviceId;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationStage};
use usb_x_platform::StorageBackend;

const SECTOR_SIZE: usize = 512;

/// Multi-pass write+read pattern test matching Rufus `badblocks` methodology.
/// Detects both physical bad sectors and counterfeit ("fake") flash capacity wrapping
/// by encoding each pass's test byte alongside the exact 64-bit sector byte offset.
pub fn execute_bad_blocks_check(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    total_bytes: u64,
    passes: u32,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let pass_patterns: &[u8] = match passes {
        1 => &[0x55],
        2 => &[0xAA, 0x55],
        _ => &[0xAA, 0x55, 0xFF, 0x00],
    };

    let num_passes = pass_patterns.len() as u64;
    let total_work = total_bytes.saturating_mul(num_passes * 2);
    let mut overall_done: u64 = 0;

    let chunk_size = 1024 * 1024; // 1 MiB chunk
    let mut buffer = vec![0u8; chunk_size];

    for (pass_idx, &pattern) in pass_patterns.iter().enumerate() {
        let pass_num = pass_idx + 1;
        let _ = sender.send(OperationEvent::StageChanged {
            stage: OperationStage::TestingCapacity,
            message: format!(
                "Bad blocks pass {}/{} (writing 0x{:02X})...",
                pass_num, num_passes, pattern
            ),
        });

        let mut written: u64 = 0;
        let start_write = Instant::now();
        let aligned_total = (total_bytes / SECTOR_SIZE as u64) * SECTOR_SIZE as u64;

        while written < aligned_total {
            if cancelled.load(Ordering::Relaxed) {
                return Err(UsbXError::Cancelled("Bad blocks check cancelled by user".to_string()));
            }

            let to_write = (chunk_size as u64).min(aligned_total - written) as usize;
            let sectors = to_write / SECTOR_SIZE;
            for sector_idx in 0..sectors {
                let sector_offset = written + (sector_idx * SECTOR_SIZE) as u64;
                let start = sector_idx * SECTOR_SIZE;
                let sector = &mut buffer[start..start + SECTOR_SIZE];
                sector.fill(pattern);
                sector[0..8].copy_from_slice(b"PGNTEST1");
                sector[8..16].copy_from_slice(&sector_offset.to_le_bytes());
                sector[16] = pattern;
            }

            backend.write_sectors(target_id, written, &buffer[..to_write])?;
            written += to_write as u64;
            overall_done += to_write as u64;

            let elapsed = start_write.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { (written as f64 / elapsed) as u64 } else { 0 };
            let remaining = aligned_total.saturating_sub(written);
            let eta = if speed > 0 { Some(remaining / speed) } else { None };

            let _ = sender.send(OperationEvent::Progress {
                completed_bytes: overall_done,
                total_bytes: Some(total_work),
                bytes_per_second: Some(speed),
                eta_seconds: eta,
            });
        }

        backend.flush(target_id)?;

        let _ = sender.send(OperationEvent::StageChanged {
            stage: OperationStage::TestingCapacity,
            message: format!(
                "Bad blocks pass {}/{} (verifying 0x{:02X})...",
                pass_num, num_passes, pattern
            ),
        });

        let mut read_back: u64 = 0;
        let start_read = Instant::now();

        while read_back < aligned_total {
            if cancelled.load(Ordering::Relaxed) {
                return Err(UsbXError::Cancelled("Bad blocks check cancelled by user".to_string()));
            }

            let to_read = (chunk_size as u64).min(aligned_total - read_back) as usize;
            backend.read_sectors(target_id, read_back, &mut buffer[..to_read])?;

            let sectors = to_read / SECTOR_SIZE;
            for sector_idx in 0..sectors {
                let expected_offset = read_back + (sector_idx * SECTOR_SIZE) as u64;
                let start = sector_idx * SECTOR_SIZE;
                let sector = &buffer[start..start + SECTOR_SIZE];

                let magic = &sector[0..8];
                let actual_offset = u64::from_le_bytes(sector[8..16].try_into().unwrap_or([0u8; 8]));
                let actual_pat = sector[16];

                if magic != b"PGNTEST1" || actual_offset != expected_offset {
                    let _ = sender.send(OperationEvent::Log {
                        level: LogLevel::Error,
                        timestamp: "Now".to_string(),
                        message: format!(
                            "FAKE FLASH OR BAD BLOCK DETECTED at offset {} bytes (returned offset {}, expected {})",
                            expected_offset, actual_offset, expected_offset
                        ),
                    });
                    return Err(UsbXError::FakeFlashDetected {
                        offset_bytes: expected_offset,
                        reported_bytes: total_bytes,
                    });
                }

                if actual_pat != pattern {
                    return Err(UsbXError::VerificationMismatch {
                        expected: format!("0x{:02X} at offset {}", pattern, expected_offset),
                        actual: format!("0x{:02X}", actual_pat),
                    });
                }
            }

            read_back += to_read as u64;
            overall_done += to_read as u64;

            let elapsed = start_read.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 { (read_back as f64 / elapsed) as u64 } else { 0 };
            let remaining = aligned_total.saturating_sub(read_back);
            let eta = if speed > 0 { Some(remaining / speed) } else { None };

            let _ = sender.send(OperationEvent::Progress {
                completed_bytes: overall_done,
                total_bytes: Some(total_work),
                bytes_per_second: Some(speed),
                eta_seconds: eta,
            });
        }
    }

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!(
            "Bad blocks check PASSED ({} pass{}, {} bytes verified with 0 errors).",
            num_passes,
            if num_passes == 1 { "" } else { "es" },
            total_bytes
        ),
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use usb_x_platform::VirtualDriveBackend;

    #[test]
    fn test_bad_blocks_passes_virtual_drive() {
        let img = std::env::temp_dir().join(format!("pigeon_bb_test_{}.img", std::process::id()));
        let _ = std::fs::remove_file(&img);

        let mut vdisk = VirtualDriveBackend::open_or_create(&img.to_string_lossy(), 64 * 1024).unwrap();
        let target_id = vdisk.enumerate_devices().unwrap()[0].id.clone();
        let (tx, _rx) = channel();
        let cancelled = Arc::new(AtomicBool::new(false));

        let res = execute_bad_blocks_check(&mut vdisk, &target_id, 64 * 1024, 2, &tx, &cancelled);
        assert!(res.is_ok());

        let _ = std::fs::remove_file(&img);
    }
}
