# USB X — Project Rules

These rules apply to every contributor, automated coding agent, pull request, and release.

## 1. Product principles

1. **Safety before convenience.** A wrong-disk write is a release-blocking defect.
2. **Portable, not privileged by default.** The UI runs as a normal user. Elevation is requested only for an operation that needs it.
3. **Honest compatibility.** Never label an image as BIOS-, UEFI-, Secure-Boot-, or persistence-compatible unless its exact recipe has been tested.
4. **No silent customization.** Image modification, Windows setup tweaks, partition changes, and destructive tests must be explicit and reviewable.
5. **Small and understandable.** Prefer a few predictable workflows over hidden automation and a large plug-in ecosystem.
6. **Observable operations.** Every long task reports a stage, measurable progress where possible, warnings, and actionable errors.

## 2. Architecture rules

- Keep the Rust `egui`/`eframe` UI, domain logic, platform-specific disk access, and elevated helper separated.
- Use `egui` + `eframe` for the desktop UI. Do not add Tauri, Electron, or an embedded browser without an explicit architecture review.
- Keep the UI entirely in Rust. C is allowed only as a small, documented FFI shim or for a vetted native library when no practical safe Rust interface exists.
- UI code must never open a physical disk device directly.
- Physical device I/O must go through the `BlockDevice`/platform-backend abstraction.
- The engine receives typed operation plans. Do not pass arbitrary shell commands or command strings from the UI to a helper.
- Do not make one generic “make bootable” routine that guesses how to convert every ISO. Use explicit image recipes or byte-for-byte raw writing.
- Keep platform differences behind `cfg`-gated adapters and documented capability checks.
- The UI must consume operation events over a channel; it must not block while scanning, formatting, hashing, downloading, or writing.
- Keep all operations cancellable only at safe boundaries. A cancellation after writing begins must clearly report that the target may now be incomplete.

## 3. Disk safety rules — mandatory

- Never trust a display name, drive letter, `/dev/sdX`, or enumeration index as the sole device identity.
- Store an opaque OS-specific stable identity and revalidate it immediately before unmount, partitioning, formatting, write, restore, or destructive testing.
- Compare device identity, capacity, model, bus, and relevant hardware identifiers before each destructive stage. Abort if the device changed or the identity is ambiguous.
- Exclude the host system/boot disk, disks containing mounted system volumes, and unsupported device classes by default.
- Do not offer a physical disk as a target if the platform cannot determine whether it contains the running system.
- Require all mounted volumes on the target to be unmounted/locked before raw writes. If the lock cannot be obtained, stop.
- Show the exact target model, capacity, and device identifier in the final confirmation dialog. State that **all data will be erased**.
- Require a second explicit confirmation for restore-from-image and destructive flash-capacity tests. Typed confirmation such as `ERASE` is recommended for restore/test operations.
- Re-enumerate and revalidate after privilege elevation. Elevation can take time; the device may have been unplugged or replaced.
- Never continue against a different device after a disconnect/reconnect event.
- Flush writes, check every OS/API return value, and report short writes, read errors, timeouts, and verification mismatches.
- Do not automatically retry a destructive operation against a newly discovered device.
- A failed or cancelled write must be marked **Incomplete — do not boot/use until recreated**.

## 4. Process execution rules

- Never invoke a shell with concatenated user input.
- Prefer native APIs. Where a platform utility is required, invoke the executable directly with an argument vector and a fixed, reviewed command template.
- Validate every path, enum, numeric parameter, and device identifier at the boundary.
- Do not use the selected ISO filename, volume label, download URL, or device path as shell syntax.
- Capture stdout/stderr separately, set timeouts where appropriate, and check exit codes.
- Do not ship a privileged helper that accepts arbitrary command execution, arbitrary file writes, or unrestricted paths.
- External tools must be pinned to known versions, checksummed, license-reviewed, and packaged with required notices.

## 5. Image and boot rules

- Support two distinct media-creation modes:
  - **Raw image mode:** exact byte-for-byte write of a compatible disk image to the selected target.
  - **Prepared installer mode:** image-specific extraction/customization with an explicit recipe and compatibility checks.
- Never silently convert from raw mode into extraction mode.
- Preserve original signed EFI boot files whenever possible.
- Secure Boot status is a capability, not a generic checkbox. Show whether the media uses an unmodified signed boot chain, a tested signed bridge, or an unverified/custom bootloader.
- Never disable Secure Boot in the host firmware, alter firmware keys, or silently replace trusted boot binaries.
- Use FAT32 only when all files fit or large WIM files are safely split into supported `.swm` segments. Respect FAT32's 4 GiB per-file limit.
- NTFS, exFAT, and UDF are not universal UEFI boot file systems. Show the compatibility trade-off before allowing them for boot media.
- Linux persistence is recipe-based and distribution/version-specific. Unsupported images must not receive a generic persistence toggle.
- Windows setup modifications must be generated from explicit user choices, previewed, and recorded in the log. Never patch the downloaded ISO in place.

## 6. Security and privacy rules

- Download only from an official vendor/distribution source selected from a maintained catalog or from a user-supplied URL with a clear warning.
- Use HTTPS with normal certificate validation. Do not disable TLS verification.
- Verify published checksums and signed metadata where available. A locally computed hash without a trusted expected hash is an integrity fingerprint, not proof of authenticity.
- Do not run downloaded ISO contents or scripts on the host.
- Never embed credentials, product keys, Microsoft account tokens, or local-account passwords in logs or generated media.
- Redact full serial numbers by default in UI logs and exported logs. Let the user explicitly include identifying hardware fields when needed for support.
- Logs must not contain secrets. Do not log access tokens, authorization headers, or signed download URLs containing tokens.
- Use safe temporary directories and remove temporary data after successful completion or a clearly reported cleanup failure.
- Keep dependency versions pinned and run vulnerability and license checks in CI.

## 7. Code quality rules

- Rust stable; format with `cargo fmt`; lint with `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Use `thiserror` for typed library errors. Add context at application boundaries without swallowing the root cause.
- Prefer explicit enums and validated newtypes over stringly typed mode/file-system values.
- No `unwrap()`/`expect()` in production I/O paths. Tests and proven invariants may use them with explanation.
- Avoid `unsafe`. Any unavoidable `unsafe` block must be minimal, documented with its safety invariants, and reviewed.
- Do not hold a mutex across disk I/O or blocking subprocess calls.
- Use bounded channels and bounded buffers. Stream large images and backups; do not load whole disk images into RAM.
- Add unit tests for every operation-plan validation and device-selection safeguard.
- All public types and non-obvious state transitions need concise documentation.

## 8. UI rules

- Follow `UI_SPEC.md`: compact single-window layout, numbered sections, familiar form controls, blue primary action, progress, and log.
- Device model and capacity must remain visible throughout a destructive workflow.
- Never make a destructive action the default response to a device selection.
- Disable controls that could change the target while an operation is running.
- Use plain language: “Erase and write”, not ambiguous “Start” alone in a confirmation dialog.
- Do not invent progress percentages. Use indeterminate progress if no reliable denominator exists.
- Keyboard focus, screen-reader labels, high-contrast states, and scalable text are required before release.
- Do not use flashy animation or decorative graphics that reduce clarity.

## 9. Git and delivery rules

- Every feature branch must include tests and update the relevant documentation.
- Each feature must define supported hosts, known limitations, failure states, and destructive behavior.
- Keep sample images, fake block devices, and fixture metadata in tests; never use a developer's real USB drive as an automated test fixture.
- Changes to disk identity, privilege boundaries, write/restore, partitioning, image recipes, downloads, or Secure Boot need a security-focused review.
- Never merge code that writes to a physical device by default in a test, example, or first-launch path.
