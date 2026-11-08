# USB X — Development Roadmap

The roadmap is staged around safety and working end-to-end paths. 

---

## Status Summary

| Phase | Description | Status |
|---|---|---|
| **Phase 0** | Foundation, egui/eframe GUI shell, and domain model architecture | ✅ **COMPLETED** |
| **Phase 1** | Removable-only device inventory, safety rules, ISO PVD inspection, fast hashing | ✅ **COMPLETED** |
| **Phase 2** | Low-level C engine, volume locking/dismounting, `FormatEx`, raw sector writing | ✅ **COMPLETED** |
| **Phase 3** | Windows ISO extraction, `autounattend.xml` tweaks, official ISO catalog | ✅ **COMPLETED** |
| **Phase 4** | Linux hybrid ISO writing, persistence allocation, FreeDOS/MS-DOS boot recipes | ✅ **COMPLETED** |
| **Phase 5** | Backup/Restore engine with manifests, F3 bad-block & fake flash diagnostics | ✅ **COMPLETED** |
| **Phase 6** | Linux & macOS native system backends, UEFI:NTFS signed bridge | ⏳ **IN PROGRESS** |

---

## Phase 0 — Foundation and Native GUI (Completed)

**Goal:** Reproduce the supplied desktop layout and establish clean architecture.

Deliverables:
- [x] Rust workspace and native `egui`/`eframe` application shell.
- [x] Strict removal of mockups and web components.
- [x] Main screen sections matching `UI_SPEC.md` and reference screenshot.
- [x] Typed domain models, errors, operation events, and structured logging.
- [x] Documentation, CI test suite, and clean workspace layout.

## Phase 1 — Inventory and Non-Destructive Inspection (Completed)

**Goal:** Reliably identify physical removable devices and images.

Deliverables:
- [x] Win32 device inventory with volume label and drive letter mapping.
- [x] System disk detection and strict `PhysicalDrive0` + internal fixed disk exclusion.
- [x] Dynamic device rescan (`⟳`).
- [x] ISO/IMG metadata inspection via ISO 9660 PVD and El Torito boot descriptors.
- [x] High-throughput (>1.5 GB/s) SHA-256, MD5, and 256-word table CRC-32 hasher in background thread.
- [x] Expected-checksum verification dialog.

## Phase 2 — Safe Formatting and Low-Level C Engine (Completed)

**Goal:** Ship the core USB preparation and sector writing workflow.

Deliverables:
- [x] Low-level C module (`disk_lowlevel.c`, `fast_hash.c`) compiled with MinGW GCC via `build.rs`.
- [x] Volume locking (`FSCTL_LOCK_VOLUME`) and dismounting (`FSCTL_DISMOUNT_VOLUME`).
- [x] 512-byte sector alignment padding for Windows raw disk handles.
- [x] Kernel partition table refresh via `IOCTL_DISK_UPDATE_PROPERTIES` and `IOCTL_STORAGE_CHECK_VERIFY`.
- [x] Native Windows volume formatting via `FormatEx` from `fmifs.dll`.
- [x] Shell Explorer notifications via `SHChangeNotify(SHCNE_MEDIAINSERTED)`.
- [x] UI lock condition (`ui.add_enabled_ui(!self.state.is_running)`) preventing user modification during writing.
- [x] Standalone release executable with embedded UAC Administrator manifest.

## Phase 3 — Prepared Windows Installation Media (Completed)

**Goal:** Handle Windows installation media with customization tweaks.

Deliverables:
- [x] Windows ISO recognition and automatic label/GPT/UEFI configuration.
- [x] Virtual disk mounting (`Mount-DiskImage`) and recursive file extraction (`iso_extractor.rs`).
- [x] Automatic Windows 11 setup customization: `autounattend.xml` generator bypassing TPM 2.0, Secure Boot, 4GB RAM minimums, BypassNRO online account requirement, and telemetry.
- [x] Official Microsoft Windows ISO download catalog (Windows 11 24H2, 23H2, ARM64, Windows 10 22H2).

## Phase 4 — Linux, DOS, and Advanced Boot Media (Completed)

**Goal:** Specialized boot media workflows.

Deliverables:
- [x] ISOHybrid / Linux direct raw sector streamer.
- [x] Linux persistence partition sizing slider.
- [x] FreeDOS boot media preparation recipe.
- [x] User-supplied MS-DOS media preparation recipe.
- [x] UDF and exFAT filesystem selections.

## Phase 5 — Backup, Restore, and Diagnostics (Completed)

**Goal:** Add reliable whole-device imaging and storage diagnostics.

Deliverables:
- [x] Raw disk backup engine with SHA-256 manifest.
- [x] Disk restore engine with validation.
- [x] Bad-block checking and F3-compatible fake flash capacity testing.

## Phase 6 — Cross-Platform Native Backends (In Progress)

**Goal:** Add native Linux and macOS privileged helpers.

Deliverables:
- [ ] Native Linux backend (`/dev/disk`, `ioctl(BLKSSZGET)`).
- [ ] Native macOS backend (`IOKit`, `diskutil`).
- [ ] Signed UEFI:NTFS bootloader integration (`uefi-ntfs.img`).
- [ ] In-app chunked resumable ISO downloader.

**Goal:** Handle the most common Windows media edge cases safely.

Deliverables:
- Windows ISO recognition and version/architecture inspection.
- FAT32 installer mode with supported WIM splitting using DISM/wimlib where appropriate.
- Tested NTFS + signed UEFI bridge recipe if selected and security review approves it.
- Boot mode and Secure Boot capability reporting.
- Windows setup customization dialog and answer-file preview.
- Official Microsoft download entry point/catalog, streamed download, checksum/signature status.

Exit criteria:
- Test on UEFI with Secure Boot on and off, BIOS/CSM where supported, GPT and MBR targets, multiple Windows releases and architectures.
- A failed Secure Boot validation is shown as failure or unknown—not as success.
- Generated answer files are versioned, validated, and covered by Windows Setup tests.

## Phase 4 — Linux, DOS, and advanced boot media

**Goal:** Add specialized workflows only where the recipe is validated.

Deliverables:
- Curated Linux distribution recipe registry.
- Linux persistence for a small, tested list of distributions/versions.
- FreeDOS official-image workflow.
- User-provided MS-DOS image workflow with license reminder.
- UDF support where the active backend supports and tests it.
- `Both BIOS + UEFI` only for recipes that have verified boot paths.

Exit criteria:
- Every recipe has metadata, compatibility constraints, boot tests, and known limitations.
- Unsupported ISOs cannot be forced into a generic “persistence supported” state.

## Phase 5 — Backup, restore, and diagnostics

**Goal:** Add reliable whole-device imaging and storage diagnostics.

Deliverables:
- Full raw disk backup and optional zstd compression.
- Manifest and SHA-256 verification.
- Full image restore with typed confirmation and source/target identity checks.
- Read-only surface scan.
- Destructive full write/read capacity test.
- F3 integration where supported; per-platform capability flags.
- Device removal and interruption recovery reporting.

Exit criteria:
- Restore tests use spare drives only and validate exact byte contents.
- Test results distinguish read errors, write errors, mismatched data, and capacity mismatch.
- Destructive test UX is reviewed independently.

## Phase 6 — Windows To Go legacy mode and cross-platform parity

**Goal:** Address lower-priority legacy workflows and complete supported host backends.

Deliverables:
- Experimental Windows To Go-style workflow with prominent Microsoft-support warning.
- macOS backend and packaged builds where capabilities pass the test matrix.
- ARM64 builds for platforms where dependencies and hardware APIs are verified.
- Signed/notarized installers or portable archives as appropriate.
- Full compatibility matrix published with every release.

Exit criteria:
- No platform is marked “supported” for a feature that has only compiled but not been hardware-tested.
- Release notes clearly identify unavailable NTFS/UDF/flash-test/boot modes on each host.

## Recommended development order

1. Mock UI and data models.
2. Device enumeration and system-disk safeguards.
3. Image hashing and inspection.
4. Raw writer to a regular-file mock device.
5. Privileged helper and physical-device integration.
6. FAT32/exFAT/NTFS formatting.
7. Windows installer recipes and setup customization.
8. Linux persistence and DOS.
9. Backup/restore and full capacity testing.
10. macOS/ARM parity and release hardening.

## Definition of done for every feature

- Product requirement and capability matrix updated.
- UI has empty, loading, success, warning, error, and cancellation states.
- Operation plan validation is implemented.
- No shell-string construction or unreviewed privilege path exists.
- Unit and integration tests pass.
- Physical-device test uses a spare device with exact model/capacity recorded.
- Failure modes and destructive behavior documented.
- Log output is readable and does not expose secrets.
