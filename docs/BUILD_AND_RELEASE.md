# USB X — Build and Release Guide

This document describes a practical setup for the proposed Rust + egui/eframe workspace. Exact crate versions and system packages should be pinned when the repository is created.

## 1. Prerequisites

- Rust stable toolchain managed by `rustup`.
- Cargo and the platform build tools required by Rust native dependencies (including a C toolchain only where a dependency requires it).
- Git.
- Rust Analyzer support for your IDE (recommended).
- Platform SDKs for Windows/macOS/Linux CI jobs.

Use the official egui/eframe documentation and the selected renderer documentation for current desktop build prerequisites. Do not assume a dependency list copied from an older release remains current.

## 2. Suggested first workspace

Create a workspace with a small number of crates:

```text
usb-x/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── docs/
├── crates/
│   ├── usb-x-core/
│   ├── usb-x-engine/
│   ├── usb-x-platform/
│   ├── usb-x-image/
│   └── usb-x-app/
└── tests/
```

Example workspace manifest:

```toml
[workspace]
resolver = "2"
members = [
  "crates/usb-x-core",
  "crates/usb-x-engine",
  "crates/usb-x-platform",
  "crates/usb-x-image",
  "crates/usb-x-app",
]

[workspace.package]
edition = "2024"
license = "GPL-3.0-or-later"
version = "0.1.0"
```

The license above is only a starting proposal. egui/eframe are MIT OR Apache-2.0; confirm all dependency, bundled utility, bootloader, and asset licenses before publishing.

## 3. Initial local commands

After creating the crate manifests:

```bash
rustup update stable
rustc --version
cargo --version
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Run the UI crate during development:

```bash
cargo run -p usb-x-app
```

The first UI milestone must use a mock device provider only. Do not grant administrator/root privileges to the application during the mock UI phase.

## 4. Platform prerequisites and backends

### Windows

- Install the Visual Studio C++ Build Tools (MSVC) and Windows SDK required by the current Rust target and UI dependencies.
- Build x64 first; add ARM64 after the dependency matrix is verified.
- Use a separate helper executable for operations requiring elevation.
- The helper should use UAC elevation and documented Windows storage APIs. If reviewed PowerShell Storage cmdlets are used, call a fixed script with validated structured inputs; do not concatenate user-provided values into PowerShell source.

### Linux

- Install the native build packages required by eframe/egui and the Rust dependencies for the target distribution.
- Add runtime capability checks for `udev`, D-Bus/UDisks, Polkit, and the selected filesystem/partition utilities.
- Do not assume every Linux distribution ships the same formatter packages. The UI should display a helpful capability message when a tool is absent.
- Build and test both a package-friendly build and a portable archive/AppImage-style build only after the dependency and license review.

### macOS

- Install Xcode Command Line Tools and the SDK versions required by current eframe/egui and Rust dependencies.
- Use Disk Arbitration/IOKit for discovery and native disk management APIs for supported operations.
- Sign and notarize distributed application/helper binaries. Keep the privileged helper as small as possible.
- Do not declare macOS NTFS formatting or UDF formatting supported without a working, tested backend.

## 5. Portable mode

A portable build is an extracted application directory. Suggested behavior:

- `usb-x.exe` / `usb-x` launches the unprivileged UI.
- `portable.flag` next to the executable opts into storing non-secret settings/logs in a `data/` subfolder when writable.
- Without the flag, use the normal per-user application-data directory.
- The helper executable is signed and invoked only for an authorized operation.
- No background service is installed for ordinary UI use.

Never store secrets in portable settings. On a read-only USB or protected directory, fall back safely to the user-data location and tell the user.

## 6. Bundling external tools

Before bundling an external utility:

1. Confirm the license allows redistribution.
2. Pin the exact version and source URL.
3. Verify its upstream release checksum or signature.
4. Store its license and notice in `THIRD_PARTY_NOTICES.md`.
5. Limit the commands and arguments the application can invoke.
6. Add tests for expected output, exit codes, and malformed output.
7. Rebuild/review whenever the utility is updated.

Do not bundle Windows ISOs, proprietary DOS files, or broad collections of Linux images.

## 7. CI recommendations

Run on pull requests:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- Build UI on each supported host target.
- Dependency vulnerability/advisory check.
- License allow-list check.
- Secret scanning.
- Fuzz smoke tests for image/ISO/WIM metadata parsers.

For release builds:

- Build from a tagged commit with pinned dependencies.
- Produce SBOM and third-party notices.
- Sign application and helper binaries.
- Record compiler, SDK, dependency, and boot-recipe versions.
- Store hashes for all published artifacts.

## 8. Release formats

| Platform | Candidate delivery | Notes |
|---|---|---|
| Windows | Portable ZIP + optional MSIX/installer | Code-sign the app and privileged helper; UAC prompt only for operations requiring it. |
| Linux | Portable tar archive/AppImage-style package + optional distro packages | Bundle only reviewed dependencies; respect host Polkit/UDisks integration. |
| macOS | Signed and notarized `.app` in a DMG/ZIP | Verify helper authorization and Gatekeeper behavior on clean machines. |

## 9. Release checklist

- [ ] Version and build channel set.
- [ ] All supported platform builds compile and launch.
- [ ] Mock-device suite and unit tests pass.
- [ ] Hardware matrix passes for each feature marked supported.
- [ ] System-disk protection reviewed on each platform.
- [ ] Device identity race/disconnect tests pass.
- [ ] Raw write, formatting, restore, and destructive test confirmations reviewed.
- [ ] Read-back verification tested.
- [ ] Secure Boot status tied to recipe version and tested hardware.
- [ ] Official source links and hashes refreshed.
- [ ] Known limitations and per-host capability matrix published.
- [ ] Binary signatures/notarization verified.
- [ ] SBOM, hashes, and third-party notices published.
