<div align="center">

<img src="assets/pigeon.png" alt="Pigeon Logo" width="190"/>

# 🐦 Pigeon

**Next-Generation Bootable USB Creator & Disk Imaging Utility**  
**Version 1.0 (Production Release)**

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org)
[![Version: 1.0](https://img.shields.io/badge/Release-v1.0-brightgreen.svg)](release/)
[![Platform: Windows & Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-lightgrey.svg)](release/)
[![Architecture: x86_64](https://img.shields.io/badge/Architecture-x86__64-brightgreen.svg)](release/)

A blazingly fast, memory-safe, portable desktop application for creating bootable media, flashing raw disk images, verifying flash drive integrity, and customizing OS installations. Engineered in pure Rust with direct hardware-aligned sector streaming.

[Releases & Downloads](#-downloads--releases-v10) • [Key Features](#-key-features) • [Quick Start](#-quick-start) • [Architecture](#-architecture) • [Building](#-building-from-source)

</div>

---

## 📦 Downloads & Releases (v1.0)

All official version 1.0 release executables and distribution packages are available directly in the [`release/`](release/) folder:

| Platform / Distro | Package Format | Direct Download Link | Description |
| :--- | :--- | :--- | :--- |
| **Windows 10 / 11** | **Standalone `.exe`** | [**`pigeon-1.0.0-windows-x64.exe`**](release/pigeon-1.0.0-windows-x64.exe) | Single portable executable (No installer needed) |
| **Linux (Universal)** | **Portable `.AppImage`** | [**`Pigeon-1.0.0-x86_64.AppImage`**](release/Pigeon-1.0.0-x86_64.AppImage) | Runs on any modern x86_64 Linux distribution |
| **Debian / Ubuntu / Kali** | **Debian Package `.deb`** | [**`pigeon_1.0.0_amd64.deb`**](release/pigeon_1.0.0_amd64.deb) | Native package with app launcher & desktop icon |
| **Fedora / RHEL / SUSE** | **RPM Package `.rpm`** | [**`pigeon-1.0.0-1.x86_64.rpm`**](release/pigeon-1.0.0-1.x86_64.rpm) | Native package for Red Hat, Fedora & openSUSE |

---

## 🌟 Key Features

### ⚡ Blazing Fast & Memory Safe
- **Pure Rust Engine:** Buffered, sector-aligned raw block streaming with zero memory garbage collection.
- **Ultra-Lightweight:** 100% self-contained binaries without electron, webview, or bulky runtime dependencies.

### 🪟 Windows 11 Setup Customizer (Bypass OOBE)
- **Automatic Tweaks Detection:** Activates automatically whenever a Windows 10 or Windows 11 installation ISO is selected.
- **Hardware Requirement Bypasses:** Removes mandatory restrictions for TPM 2.0, Secure Boot, and 4 GB+ RAM.
- **Offline Local Account:** Bypasses mandatory Microsoft online account sign-in (`BypassNRO`) and creates a local user account.
- **Privacy & Encryption Protections:** Automatically disables telemetry / privacy question prompts and BitLocker automatic disk encryption.
- **Non-Destructive XML Injection:** Generates clean, standard `autounattend.xml` placed directly at the filesystem root.

### 🐧 Complete Linux & Windows Parity
- **True Multi-Platform Support:** Works identically across Windows 10/11, Kali Linux (bare metal & VirtualBox), Ubuntu, Debian, Fedora, and openSUSE.
- **Dual-Mode File Chooser:** Automatically detects and calls native desktop dialogs (`zenity`, `kdialog`, `yad`, Windows Explorer). If no system dialog exists (such as on minimal Kali Linux installations), Pigeon seamlessly opens its **built-in zero-dependency File Browser**.
- **Native Drag & Drop:** Drag any `.iso`, `.img`, `.bin`, `.raw`, or `.vhd` disk image directly from your file manager (Thunar, Nautilus, Explorer) into Pigeon to load and probe it immediately.

### 🛡️ Defensive Drive Safety Guard
- **Physical Drive Isolation:** Strictly protects and hides host system disks (`PhysicalDrive0`, internal NVMe and SATA boot drives) to safeguard you against accidental data loss.
- **Clean Volume Lock & Dismount:** Locks and unmounts partition volumes before formatting or streaming to ensure zero write contention.

### 🔍 Real-Time Background Checksums
- **Multi-Algorithm Hashing:** Calculates **MD5**, **SHA-256**, and high-speed hardware **CRC-32** in parallel background threads.
- **Interactive Verification:** Paste an expected hash into the dialog to immediately verify cryptographic image integrity.

### 🧪 Drive Health & Fake Flash Testing
- **Bad Sector Verification:** Runs pattern-based read/write passes to detect physical drive defects.
- **Counterfeit Drive Detection:** Exposes modified controller firmware and "fake capacity" USB drives before data corruption happens.

### 🎨 Adaptive Modern UI
- **Dynamic Theming:** Seamlessly synchronizes with system Dark and Light appearance preferences.
- **OS Accent Harmonization:** Adopts native desktop accent highlights across Windows and Linux.

---

## 🚀 Quick Start

### Running on Linux

#### Universal AppImage (Recommended)
```bash
chmod +x release/Pigeon-1.0.0-x86_64.AppImage
./release/Pigeon-1.0.0-x86_64.AppImage
```

#### Debian / Kali Linux / Ubuntu
```bash
sudo apt install ./release/pigeon_1.0.0_amd64.deb
pigeon
```

#### Fedora / RHEL / openSUSE
```bash
sudo rpm -i ./release/pigeon-1.0.0-1.x86_64.rpm
pigeon
```

### Running on Windows

Double-click `pigeon-1.0.0-windows-x64.exe` or execute from Command Prompt / PowerShell:
```powershell
.\release\pigeon-1.0.0-windows-x64.exe
```
*(Automatically prompts for Administrator permissions to access raw physical storage drives)*

---

## 🏗️ Architecture

Pigeon is structured as a modular workspace consisting of 6 decoupled Rust crates:

```
pigeon/
├── assets/               # High-resolution logos, brand artwork & icons
├── crates/
│   ├── usb-x-core/       # Domain models, partition schemes, risk classification & validation
│   ├── usb-x-image/      # ISO9660, UDF, El Torito, WIM inspection, autounattend generator, hashing
│   ├── usb-x-platform/   # OS storage backends (Win32 IOCTL / Linux sysfs/lsblk), UAC elevation, dialogs
│   ├── usb-x-engine/     # Execution pipeline, sector streaming, verification & progress events
│   ├── usb-x-catalog/    # OS download catalog metadata (Windows, Ubuntu, Arch, Fedora)
│   └── usb-x-app/        # Egui/Eframe desktop interface, dynamic themes, Pigeon icon & dialogs
├── dist/
│   └── linux/            # Linux packaging pipelines (AppImage, DEB, RPM specifications)
├── docs/                 # Architectural specifications, security models, and test plans
└── release/              # Ready-to-use production binaries for Windows and Linux
```

---

## 🛠️ Building From Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (1.75+ recommended)
- **Windows:** MSVC or MinGW GCC
- **Linux:** `build-essential`, `pkg-config`, `libx11-dev`, `libwayland-dev`, `libgl1-mesa-dev`, `python3`

### Compile for Your Platform
```bash
# Clone the repository
git clone https://github.com/pigeon-usb/pigeon.git
cd pigeon

# Run the complete test suite
cargo test --workspace

# Build the release executable
cargo build --release -p pigeon
```

Compiled binaries will be generated at:
- **Windows:** `target/release/pigeon.exe`
- **Linux:** `target/release/pigeon`

### Generate All Linux Release Packages (.AppImage, .deb, .rpm)
```bash
# Build the native Linux ELF binary
cargo build --release -p pigeon

# Package all formats
python3 dist/linux/package_deb.py
python3 dist/linux/package_rpm.py
python3 dist/linux/package_appimage.py
```

Generated packages are placed in `dist/linux/` and `release/`.

---

## 🔒 Security & Data Safety

- **Destructive Operations:** Writing or formatting drives permanently erases existing target media data.
- **Automatic System Disk Protection:** Internal operating system boot disks (`PhysicalDrive0`, `/dev/sda` when root mounted, NVMe system partitions) are locked and cannot be selected.
- **Explicit Confirmation:** Non-reversible actions prompt a clear confirmation dialog prior to initializing partition destruction.

---

## 📄 License

Pigeon is open-source software licensed under the [GNU General Public License v3.0 or later](LICENSE).
