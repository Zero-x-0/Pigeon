use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;
use usb_x_core::device::DeviceId;
use usb_x_core::errors::{Result, UsbXError};
use usb_x_core::event::{LogLevel, OperationEvent, OperationStage};
use usb_x_core::types::FlashTestKind;
use usb_x_platform::StorageBackend;

const SECTOR_SIZE: usize = 512;

pub fn execute_flash_test(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    total_bytes: u64,
    kind: FlashTestKind,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    match kind {
        FlashTestKind::ReadOnlyBadBlockScan => {
            execute_readonly_scan(backend, target_id, total_bytes, sender, cancelled)
        }
        FlashTestKind::DestructiveFakeCapacityTest => {
            execute_fake_capacity_test(backend, target_id, total_bytes, sender, cancelled)
        }
    }
}

fn execute_readonly_scan(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    total_bytes: u64,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::SurfaceScanning,
        message: format!("Scanning {} bytes for read errors / bad blocks...", total_bytes),
    });

    let chunk_size = 1024 * 1024; // 1 MiB chunk
    let mut buffer = vec![0u8; chunk_size];
    let mut scanned: u64 = 0;
    let mut bad_blocks_count = 0;
    let start_time = Instant::now();

    while scanned < total_bytes {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Scan cancelled by user".to_string()));
        }

        let to_read = (chunk_size as u64).min(total_bytes - scanned) as usize;
        let slice = &mut buffer[..to_read];

        match backend.read_sectors(target_id, scanned, slice) {
            Ok(_) => {}
            Err(e) => {
                bad_blocks_count += 1;
                let _ = sender.send(OperationEvent::Warning {
                    message: format!("Bad block / I/O error at offset {} bytes: {}", scanned, e),
                });
            }
        }

        scanned += to_read as u64;

        let elapsed = start_time.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 {
            (scanned as f64 / elapsed) as u64
        } else {
            0
        };

        let remaining = total_bytes.saturating_sub(scanned);
        let eta = if speed > 0 { Some(remaining / speed) } else { None };

        let _ = sender.send(OperationEvent::Progress {
            completed_bytes: scanned,
            total_bytes: Some(total_bytes),
            bytes_per_second: Some(speed),
            eta_seconds: eta,
        });
    }

    let _ = sender.send(OperationEvent::Log {
        level: if bad_blocks_count == 0 { LogLevel::Info } else { LogLevel::Warn },
        timestamp: "Now".to_string(),
        message: format!("Surface scan complete. Bad blocks detected: {}", bad_blocks_count),
    });

    Ok(())
}

fn execute_fake_capacity_test(
    backend: &mut dyn StorageBackend,
    target_id: &DeviceId,
    total_bytes: u64,
    sender: &Sender<OperationEvent>,
    cancelled: &Arc<AtomicBool>,
) -> Result<()> {
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::TestingCapacity,
        message: format!("Phase 1/2: Writing test patterns across {} bytes...", total_bytes),
    });

    let chunk_size = 1024 * 1024; // 1 MiB
    let mut buffer = vec![0u8; chunk_size];
    let mut written: u64 = 0;
    let start_write = Instant::now();

    // Fill buffer with deterministic pattern tagged by sector offset
    while written < total_bytes {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Capacity test cancelled".to_string()));
        }

        let to_write = (chunk_size as u64).min(total_bytes - written) as usize;
        for sector_idx in 0..(to_write / SECTOR_SIZE) {
            let sector_offset = written + (sector_idx * SECTOR_SIZE) as u64;
            let start = sector_idx * SECTOR_SIZE;
            let sector = &mut buffer[start..start + SECTOR_SIZE];
            // Write magic and offset into first 16 bytes
            sector[0..8].copy_from_slice(b"USBXTEST");
            sector[8..16].copy_from_slice(&sector_offset.to_le_bytes());
            // Fast pattern in remainder
            for i in 16..SECTOR_SIZE {
                sector[i] = (sector_offset.wrapping_add(i as u64) & 0xFF) as u8;
            }
        }

        backend.write_sectors(target_id, written, &buffer[..to_write])?;
        written += to_write as u64;

        let elapsed = start_write.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { (written as f64 / elapsed) as u64 } else { 0 };
        let remaining = total_bytes.saturating_sub(written);
        let eta = if speed > 0 { Some(remaining / speed) } else { None };

        let _ = sender.send(OperationEvent::Progress {
            completed_bytes: written / 2, // Half total progress
            total_bytes: Some(total_bytes),
            bytes_per_second: Some(speed),
            eta_seconds: eta,
        });
    }

    backend.flush(target_id)?;

    // Phase 2: Read back and verify every sector
    let _ = sender.send(OperationEvent::StageChanged {
        stage: OperationStage::TestingCapacity,
        message: "Phase 2/2: Reading back and verifying flash capacity integrity...".to_string(),
    });

    let mut read_back: u64 = 0;
    let start_read = Instant::now();

    while read_back < total_bytes {
        if cancelled.load(Ordering::Relaxed) {
            return Err(UsbXError::Cancelled("Capacity test cancelled".to_string()));
        }

        let to_read = (chunk_size as u64).min(total_bytes - read_back) as usize;
        backend.read_sectors(target_id, read_back, &mut buffer[..to_read])?;

        for sector_idx in 0..(to_read / SECTOR_SIZE) {
            let expected_offset = read_back + (sector_idx * SECTOR_SIZE) as u64;
            let start = sector_idx * SECTOR_SIZE;
            let sector = &buffer[start..start + SECTOR_SIZE];

            let magic = &sector[0..8];
            let actual_offset = u64::from_le_bytes(sector[8..16].try_into().unwrap_or([0u8; 8]));

            if magic != b"USBXTEST" || actual_offset != expected_offset {
                let _ = sender.send(OperationEvent::Log {
                    level: LogLevel::Error,
                    timestamp: "Now".to_string(),
                    message: format!(
                        "FAKE FLASH DRIVE DETECTED! Sector at offset {} returned offset {}. The drive controller is looping/wrapping!",
                        expected_offset, actual_offset
                    ),
                });
                return Err(UsbXError::FakeFlashDetected {
                    offset_bytes: expected_offset,
                    reported_bytes: total_bytes,
                });
            }
        }

        read_back += to_read as u64;

        let elapsed = start_read.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { (read_back as f64 / elapsed) as u64 } else { 0 };

        let _ = sender.send(OperationEvent::Progress {
            completed_bytes: total_bytes / 2 + read_back / 2,
            total_bytes: Some(total_bytes),
            bytes_per_second: Some(speed),
            eta_seconds: None,
        });
    }

    let _ = sender.send(OperationEvent::Log {
        level: LogLevel::Info,
        timestamp: "Now".to_string(),
        message: format!("Capacity test PASSED! Device has genuine full capacity ({} bytes).", total_bytes),
    });

    Ok(())
}

