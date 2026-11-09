# USB X — Recommended Technology Stack (Rust-first)

## 1. Primary decision

**Rust is the primary and default language for the entire application.** Build the desktop interface with **egui + eframe**. Use C only when a platform API or mature native library has no practical, safe Rust interface; keep any C code in a small, documented FFI boundary.

| Layer | Choice | Why |
|---|---|---|
| Primary language | Stable Rust | Memory safety, strong types, cross-platform systems support and streaming I/O. |
| Desktop UI | `egui` + `eframe` | Pure-Rust immediate-mode GUI; native Windows, Linux and macOS builds; no embedded browser. |
| Renderer | Start with `glow` for a smaller graphics stack; evaluate `wgpu` only if required | Avoid enabling multiple renderers by default; benchmark actual builds. |
| Async/task coordination | Tokio only where async coordination is useful; dedicated blocking workers for disk I/O | Raw disk operations must never block the UI thread. Use bounded channels for events. |
| Serialization | `serde`, `serde_json`, `toml` | Typed IPC protocol, manifests and configuration. |
| Errors | `thiserror` in libraries; `anyhow` at application boundaries | Typed errors and contextual diagnostics. |
| Logging | `tracing`, `tracing-subscriber` | Structured, filterable logs with operation IDs. |
| Hashing | `sha2`; optional `blake3` | SHA-256/SHA-512 interoperability and fast local checksums. |
| Downloads | `reqwest` with rustls | HTTPS with certificate validation and streaming. |
| Compression | `zstd` | Optional streaming image-backup compression. |
| WIM support | `wimlib` / `wimlib-imagex`, version-pinned | Inspect, extract, split and apply WIM/ESD content. Review license and notices. |
| Linux discovery | udev/libudev and/or UDisks D-Bus | Device discovery and desktop storage integration. |
| Windows discovery | SetupAPI, DeviceIoControl, Windows Storage APIs | Stable disk identity, properties, locking and raw I/O. |
| macOS discovery | Disk Arbitration and IOKit | Device notifications, identity and unmount/eject. |

The application must not require an embedded browser, account, cloud backend or permanently running daemon. Portable mode does not bypass OS privilege controls.

## 2. Rust crate shortlist

Choose exact versions during implementation, check MSRV/platform support/advisories/licenses, and commit `Cargo.lock` for applications.

- UI: `eframe`, `egui`
- Async/events: `tokio`, `crossbeam-channel` or Tokio channels
- Data: `serde`, `serde_json`, `toml`
- Errors: `thiserror`, `anyhow`
- IDs: `uuid`
- Hashing: `sha2`, optional `blake3`, `hex`
- Downloads: `reqwest`, `url`
- Logging: `tracing`, `tracing-subscriber`
- Temporary paths: `tempfile`
- Compression: `zstd`
- Windows APIs: `windows` crate with only required features
- Linux D-Bus/udev: maintained Rust crates or narrow bindings around stable system interfaces
- macOS: maintained Rust bindings where practical; narrow Objective-C/IOKit FFI only where necessary

Prefer the `eframe` `glow` renderer profile for a lightweight desktop binary, and measure release size/startup/RAM on all target OSes. `eframe` defaults and feature flags can change; pin and review the selected release.

## 3. C language policy

C is **optional, not a second application language**. Use it only for one of these cases:

1. A documented OS interface is exposed through a C ABI and no maintained Rust crate provides a suitable binding.
2. A mature, audited C library (for example, a filesystem or image utility) is safer to integrate than reimplementing its format.
3. A tiny platform shim is needed for an API that Rust cannot conveniently express.

Rules for C/FFI:
- Keep C source under `native/` or a dedicated crate; do not spread C logic through the app.
- Expose a small `extern "C"` interface with fixed-width types, explicit ownership, lengths and error codes.
- Never pass Rust references, trait objects, `String`, `Vec`, or Rust enums directly across the ABI.
- Validate all pointers, lengths, integer conversions and buffer bounds on both sides.
- Document every `unsafe` block and its invariants; isolate unsafe code in the smallest possible module.
- Prefer safe Rust wrappers around native functions; fuzz and test the boundary.
- Do not write custom partition, FAT/NTFS/exFAT/UDF, WIM, ISO or bootloader implementations in C for v1.

## 4. Platform approach

### Windows

- Enumerate disks with SetupAPI and/or documented Windows storage APIs. Capture disk number, unique ID, bus, model, serial where available and capacity.
- Use `DeviceIoControl` and documented storage IOCTLs for geometry, layout, volume lock/dismount and raw I/O.
- Request UAC elevation only for the narrowly scoped helper operation. Keep the normal UI unelevated.
- Never rely on drive letters as device identity; they can change.

### Linux

- Subscribe to udev events and enumerate using sysfs/udev metadata; optionally integrate UDisks for user-session unmount/eject.
- Revalidate stable identity (serial/WWN plus major/minor and capacity) before every destructive operation; abort if ambiguous.
- Use a narrowly scoped Polkit-authorized helper for raw block writes and partition changes.
- Use reviewed system utilities/libraries for partitioning and filesystem creation. Missing utilities disable only the related capability and show an install hint; never download executable packages automatically.

### macOS

- Use Disk Arbitration notifications and IOKit properties for physical/external media.
- Prefer documented `diskutil` workflows for supported erase/partition operations, with a narrowly scoped privileged helper for raw access.
- Use `/dev/rdiskN` only as an I/O path after revalidating disk identity; never as identity itself.
- Native macOS NTFS write/format support is not assumed. Gate NTFS/UDF and full-capacity testing on tested backend support.

## 5. File-system capability policy

| File system | Data storage | Boot-media suitability | Initial policy |
|---|---|---|---|
| FAT32 | Broad compatibility; 4 GiB maximum single file | Common UEFI choice | Implement first where native formatter is supported; split oversized Windows WIM files using a tested workflow. |
| NTFS | Strong Windows support; Linux tools available; macOS varies | Not universally readable by UEFI firmware | Windows/Linux first; other hosts capability-gated. |
| exFAT | Good removable-data compatibility | Not a generic UEFI boot filesystem | Format as data volume where supported; do not claim universal bootability. |
| UDF | Optical-media and specialized data use | Not a universal USB boot filesystem | Advanced option; enable only on tested host backends. |

## 6. Image and boot utilities

- `wimlib-imagex`: inspect/extract/split/apply WIM content; validate commands against supported Windows Setup behavior.
- `xorriso` / `libarchive`: inspect and extract ISO filesystems; extraction alone does not prove bootability.
- `mtools`: manipulate FAT images without mounting where useful.
- GRUB/Syslinux tooling: use only reviewed per-image recipes with license and Secure Boot provenance understood.
- F3: optional integration for full-capacity testing. Its destructive mode requires explicit consent and a clear warning.

Do not bundle every utility in v1. Bundle only what is required by implemented, tested workflows and include notices.

## 7. Licensing and portability

- egui/eframe are MIT OR Apache-2.0; retain applicable notices and verify every dependency independently.
- Track notices for wimlib, F3, partition tools, filesystem utilities, bootloaders and assets. Publish third-party notices and an SBOM.
- Do not copy Rufus implementation code or assets. Use public behavior/documentation as a reference unless license compatibility and attribution have been reviewed.
- Portable means the app can run from an extracted folder without a traditional installer or account. It does not mean disk operations can bypass UAC, Polkit or macOS authorization.
- Keep configuration/logs in the OS user-data directory by default; support an explicit `portable.flag` mode for non-secret settings beside the executable when writable.

## 8. UI implementation notes

Use `eframe::App` as the application entry point and build the main window from `egui::CentralPanel`, `egui::Grid`, `egui::ComboBox`, `egui::ProgressBar`, and scrollable log regions. Keep reusable widgets in a small `ui/` module. Use an application state model that receives typed events from worker threads; never mutate UI state from a disk worker. Avoid enabling persistence unless the user explicitly opts in to saving preferences.

For the supplied Rufus-inspired screen, preserve the numbered workflow sections, compact form controls, blue primary Start button, clear Close/Cancel action, progress bar, and timestamped log. Do not sacrifice device identity and destructive-operation warnings to make the layout look compact.
