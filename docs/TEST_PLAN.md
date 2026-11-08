# USB X — Test Plan

## 1. Test philosophy

Most logic must be testable without physical hardware. Build a mock block-device backend and a regular-file-backed virtual device before enabling physical disk operations. Hardware tests must use explicitly designated spare media and must never run automatically in CI.

## 2. Unit tests

### Device identity and safety

- Device ID is not derived solely from a mutable path or drive letter.
- Reordered device enumeration does not change the selected target.
- Same model/capacity but different serial/unique ID is treated as a different device.
- Capacity or sector-size change after preflight aborts.
- Device disappearance aborts; a newly attached device is never substituted.
- System disk, boot disk, root backing device, and ambiguous device-mapper/RAID layouts are protected.
- A mounted or locked volume prevents raw write until it is safely unmounted/locked.

### Image handling

- ISO9660/UDF/MBR/GPT and hybrid-image metadata fixtures.
- Malformed partition tables and truncated images.
- Oversized file count, path length, and decompression limits.
- Archive path traversal and symlink-escape fixtures.
- Hash calculation for empty, small, large, sparse, and truncated files.
- Expected hash matching and mismatch.
- Unknown image reports unknown; heuristics never become a verified bootability claim.

### Layout and file-system validation

- FAT32 single-file 4 GiB boundary.
- NTFS/exFAT/UDF capability restrictions by platform.
- MBR/GPT layout validation and alignment rules.
- Partition size/offset overflow and arithmetic boundary cases.
- Unsupported BIOS/UEFI combinations are rejected.

### Operation engine

- Progress monotonicity and no fabricated 100% before verification.
- Indeterminate progress when total size is unknown.
- Safe cancellation before write, during write, during verification, and during cleanup.
- No retry to a newly enumerated target.
- Flush failure and short write are surfaced.
- Log events contain operation ID and redact sensitive fields.

## 3. Mock block-device integration tests

Use temporary regular files or a custom mock backend to simulate:

- 64 MiB, 1 GiB, and larger targets.
- Read-only target.
- Write-protected target.
- Short reads and short writes.
- I/O error at specified offsets.
- Disconnection during each stage.
- Capacity mismatch between discovery and write.
- Slow device and fluctuating throughput.
- Bad blocks and deterministic read-back corruption.

Assertions:

- No operation writes outside the validated range.
- The operation does not report success before required flush/verification.
- A failed operation reports the exact stage and last confirmed offset.
- A cancelled operation never reports a completed bootable drive.

## 4. Platform integration matrix

### Windows

- USB flash drive (USB 2.0 and USB 3.x).
- USB external SSD/HDD.
- SD/microSD card via reader.
- Removable and fixed-bus USB devices.
- Multiple partitions and mounted volumes.
- UAC cancelled, denied, and accepted.
- Drive letter changes and device reconnect.
- BitLocker-encrypted data volume (must not proceed without clear handling).
- System disk and recovery/EFI partitions protected.

### Linux

- udev hotplug and removal events.
- UDisks present and absent.
- Polkit authorization accepted/cancelled/denied.
- Root on a simple partition, LVM, dm-crypt, RAID, and device mapper.
- External USB and SD media.
- Missing formatter utility and unsupported file system.
- Busy mount points and automounter races.

### macOS

- Intel and Apple Silicon where supported.
- Disk Arbitration appeared/disappeared events.
- External USB storage, SD card reader, and external SSD.
- `diskutil` failure and permission denial.
- Startup disk and APFS container protection.
- Signed/notarized app and helper behavior on a clean machine.

## 5. Boot validation matrix

For each supported image recipe, keep a record of:

- Image exact version, architecture, source URL, and digest.
- Target device make/model/capacity and USB controller.
- Partition scheme and file system.
- BIOS/UEFI/CSM configuration.
- Secure Boot state and firmware vendor/model/version where available.
- Boot result, installer start result, installation result, and known limitations.

Test categories:

- Raw hybrid Linux ISO.
- Extracted Linux installer.
- Linux persistence for each supported distribution/version.
- Windows ISO with FAT32 and split WIM.
- Windows ISO with NTFS bridge, if implemented.
- Windows 11 setup customizations on supported test builds.
- FreeDOS on legacy BIOS/CSM hardware or emulator.
- User-supplied MS-DOS floppy/disk image (only if the user has appropriate rights).

Do not mark Secure Boot support verified after testing only with Secure Boot disabled.

## 6. Backup and restore tests

- Full raw image backup matches exact source bytes for known fixture devices.
- Compressed backup decompresses to the exact original byte stream.
- Manifest includes source geometry and digest.
- Existing backup filename is not overwritten silently.
- Corrupted backup hash is rejected before restore.
- Restore to equal-size and larger target.
- Restore to smaller target is rejected unless a separately designed and tested partition-aware restore mode exists.
- Restore to same source device is rejected.
- Interrupt backup and restore at several offsets; verify partial files are clearly marked incomplete.

## 7. Bad-block and fake-capacity test cases

- All blocks read correctly.
- Read error on a known range.
- Write error on a known range.
- Write succeeds but read-back returns different data.
- Reported capacity larger than writable capacity (simulated fake device).
- Full range test cancelled at 10%, 50%, and 99%.
- Power/disconnect interruption.
- Wear warning and typed confirmation are always shown.
- The result wording remains cautious and never overclaims genuineness.

## 8. Security tests

- Helper rejects invalid/unknown operation type.
- Helper rejects arbitrary executable/path arguments.
- Helper resolves and checks stable device ID itself.
- Helper blocks system disk independently of UI.
- Forged IPC request and stale request version are rejected.
- Image extraction rejects path traversal and symlink escape.
- HTTPS certificate validation is enforced.
- Untrusted checksum manifest is not treated as publisher-authenticated.
- Logs do not contain secrets, serials by default, or authorization data.
- Downloaded files are never executed or auto-mounted.

## 9. Manual destructive-test protocol

1. Use a spare, non-production device with no required data.
2. Record make/model, reported capacity, serial suffix, and OS path.
3. Confirm that the selected device is not the host system disk.
4. Start the test only after a fresh enumeration and explicit confirmation.
5. Photograph or export the final summary for the test record.
6. Reformat the device only after the test result is recorded.
7. Repeat on another brand/controller where possible.

## 10. Release gates

A feature is **Supported** only when unit tests, mock integration tests, platform tests, and the relevant hardware/boot tests pass. Use these states:

- **Supported:** All release gates passed for the listed platform and recipe.
- **Experimental:** Implemented but limited hardware/firmware coverage; user acknowledgement required.
- **Unavailable:** Backend or tests are missing.
- **Not verified:** Input metadata is insufficient to make a reliable compatibility claim.
