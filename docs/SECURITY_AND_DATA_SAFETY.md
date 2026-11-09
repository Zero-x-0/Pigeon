# USB X — Security and Data Safety

## 1. Threat model

USB X handles untrusted disk images, removable devices, firmware boot files, and operations that can destroy user data. Consider these threat classes:

- A user selects the wrong drive or a drive is replaced between selection and write.
- A hostile or malformed image exploits an ISO/WIM/parser or causes path traversal during extraction.
- A fake-capacity drive reports a larger capacity than it can safely store.
- A removable drive disconnects or returns corrupted data during write/backup/test.
- A downloaded image or bootloader is tampered with or served from an untrusted mirror.
- The privileged helper is abused to run arbitrary commands or overwrite files outside the selected target.
- Logs or generated setup files expose hardware identifiers or secrets.
- A custom boot chain is blocked by Secure Boot or revoked by a firmware DBX update.

The app cannot guarantee a drive's physical integrity or universal boot compatibility. It must communicate uncertainty and stop when identity or state is ambiguous.

## 2. Destructive-operation gate

Every destructive workflow must pass these checks in order:

1. **Selection:** User chooses an explicit device or virtual image target.
2. **Discovery snapshot:** Record stable device ID, hardware identity, capacity, sector sizes, bus, partitions, mounted volumes, and system-disk risk.
3. **Preflight:** Validate source size, selected recipe, file-system capability, requested boot mode, and target capacity.
4. **Confirmation:** Display device model and size prominently, identify the operation, and state what will be erased.
5. **Privilege:** Request only the required authorization.
6. **Revalidation:** Re-enumerate from the privileged context and compare the selected identity and geometry. Abort on mismatch.
7. **Lock:** Unmount volumes and acquire an exclusive lock. Abort if any volume remains busy.
8. **Operation:** Write/format only the validated target range.
9. **Flush and verify:** Flush caches and perform the promised verification.
10. **Release:** Release locks, safely eject if requested, and store the result.

No operation may skip the privileged-side revalidation.

## 3. System-disk protection

- On Windows, identify the OS boot/system disk and disks containing pagefile, crash dump, recovery, or running system volumes where the platform exposes that information.
- On Linux, identify the root filesystem and the backing block device(s), including device-mapper, LVM, RAID, and encrypted layers. Do not assume root is always on `/dev/sda`.
- On macOS, identify the startup volume/container and physical backing store where possible.
- Exclude internal disks and ambiguous virtual disks by default. Advanced override is not part of the initial release.
- If the host OS cannot reliably identify the system disk in a storage topology, disable physical writes for that ambiguous target.

## 4. Privilege boundary

- Keep the main GUI unprivileged.
- Use a versioned local IPC protocol; authenticate the client process as the current interactive user where supported.
- The helper should accept only predefined operations with typed parameters. No `exec`, shell, script, arbitrary device path, or arbitrary destination path request is allowed.
- Allow-list operations and validate each enum/range/length in the helper.
- The helper independently resolves the stable device ID and repeats the safety checks.
- The helper must reject requests for protected/system disks regardless of what the UI displays.
- Rate-limit or serialize concurrent writes; only one mutating operation may target a device at a time.
- Use short-lived privilege sessions and close handles promptly.

## 5. Untrusted image handling

- Treat ISO, IMG, WIM, ESD, ZIP, and archive contents as untrusted.
- Parse metadata without executing scripts or binaries.
- Use safe extraction APIs that reject absolute paths, `..` traversal, symlinks escaping the extraction root, device files, and unexpected hard links.
- Enforce limits for entry count, path length, nested archives, decompressed size, and total extraction size.
- Never extract to a user-controlled root path or overwrite an unrelated file.
- Keep temporary working directories unique, private, and cleaned on success/failure.
- Pin parser/library versions and fuzz file-format parsers with malformed fixtures.

## 6. Downloads and authenticity

- Maintain an allow-list of official distribution/vendor landing pages and documented download hosts.
- Require HTTPS and standard certificate validation.
- Do not silently follow a download to an unrelated domain. Display the final hostname and source page.
- Verify signed metadata or a publisher checksum where available.
- If the publisher provides no expected hash/signature for the exact file, report only the locally computed digest and state that authenticity was not established.
- Do not accept checksum values from the same untrusted mirror as the sole proof of integrity.
- Store downloaded image files in a user-selected folder and use exclusive creation to avoid overwriting a file.
- Never auto-mount and execute downloaded media.

## 7. Secure Boot policy

Secure Boot verifies the signatures of boot components against trust databases in platform firmware and subsequent boot stages. It is not a property that can be guaranteed merely by formatting a USB as FAT32.

- Prefer unmodified official ISO boot files and preserve their signature chain.
- If a bridge bootloader is needed, document the certificate chain, signer, source, version, hash, revocation status check, and supported firmware assumptions.
- Do not present a custom or unsigned EFI binary as Secure-Boot-compatible.
- Re-check the current revocation/DBX implications before each release of a boot bridge; firmware updates can change what is trusted.
- Report Secure Boot compatibility as a verified state tied to a recipe version, not a universal device property.
- Never instruct the application to edit the host firmware's Secure Boot keys or disable Secure Boot automatically.

## 8. Windows installer customization

- Generate an answer file from explicit toggles and reviewed versioned templates.
- Preview the exact settings and generated file before media creation.
- Validate XML structure and ensure values are permitted for the targeted Windows version/image.
- Never store user passwords, product keys, or Microsoft account tokens in the answer file or logs.
- When a local account is requested, do not silently create an insecure static password. Prefer user completion during setup or a secure one-time prompt whose handling is tested and not logged.
- TPM/Secure Boot/RAM bypass options are unsupported configuration choices and must be clearly marked as such, with version-specific testing and a warning about compatibility, support, and updates.
- Do not make unrelated security changes such as disabling Defender, firewall, UAC, or Windows Update.

## 9. Fake flash and bad-block testing

- Explain clearly whether the selected test is read-only or destructive.
- A full capacity test writes data across a range and can erase existing content. Require an explicit confirmation with the exact target.
- Use pseudorandom test patterns and verify the data read back; report offsets and ranges.
- Do not treat device-reported capacity as evidence that capacity is genuine.
- Do not claim “genuine” solely because a quick test passed. Use the cautious status **No mismatch observed in tested range**.
- Do not automatically resize partitions or write repair metadata based on a test.
- Warn that full write/read scans create write wear and may take a long time.

## 10. Backups and restores

- Backup files are user data; use exclusive create and never silently overwrite an existing backup.
- Verify source identity and geometry before reading.
- Record source byte length, logical sector size, image digest, and compression details in the manifest.
- Restore is destructive and requires a second confirmation. The final confirmation must state the exact destination device and backup filename.
- Never restore to the source device by accident. If source and destination identities match, abort.
- Never infer that a backup is restorable merely because its hash matches; validate image length, manifest schema, and supported block geometry.

## 11. Logs and temporary files

- Log structured operation IDs, stage transitions, byte counts, rates, warnings, and error codes.
- Redact serial numbers, full device IDs, user home paths, download tokens, passwords, and product keys by default.
- Do not log block contents, test patterns, or file contents.
- Export diagnostic logs only on user request.
- Use per-operation temporary directories with restrictive permissions; clean them after successful completion and make cleanup failure visible.
- Never put temporary files in the target drive before its identity and file-system state are validated.

## 12. Failure and recovery semantics

- On device removal, I/O error, write protection, lock failure, or identity mismatch: stop writes and report the last confirmed offset/stage.
- Do not auto-retry on a different path or new device.
- Do not report success until flush and required verification complete.
- On power loss, the USB may be partially overwritten. The next launch must not assume a previous job completed.
- If recovery is possible, offer safe guidance such as re-running the operation or using a separate backup. Never promise data recovery.

## 13. Release security checklist

- [ ] Signed application and helper binaries for each platform.
- [ ] Reproducible or documented release build process.
- [ ] Pinned dependencies, vulnerability scan, license scan, SBOM.
- [ ] Fuzzing for image/archive metadata parsers.
- [ ] Privilege-boundary review and helper protocol tests.
- [ ] No shell-string injection paths.
- [ ] Disk identity race and disconnect tests.
- [ ] System-disk block tests on Windows/Linux/macOS.
- [ ] Verified hashes for all bundled helper utilities and EFI components.
- [ ] Public documentation describes unsupported media and destructive tests.
