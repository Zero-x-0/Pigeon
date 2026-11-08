<div align="center">

# ⚡ USB X

**Next-Generation Bootable USB Creator & Disk Imaging Utility**

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org)
[![Platform: Windows & Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-lightgrey.svg)](https://github.com)
[![Architecture: x86_64](https://img.shields.io/badge/Architecture-x86__64-brightgreen.svg)](https://github.com)

A high-performance, memory-safe, portable desktop utility for writing, formatting, verifying, and customizing bootable USB media and raw disk images. Engineered from the ground up in Rust with native low-level platform I/O.

[Features](#-key-features) • [Downloads](#-downloads--releases) • [Quick Start](#-quick-start) • [Architecture](#-architecture) • [Building](#-building-from-source)

</div>

---

## 🌟 Key Features

### ⚡ Blazing Fast & Memory Safe
- **Pure Rust Engine:** Sector-aligned zero-copy streaming with direct memory-mapped I/O.
- **Low Footprint:** Standalone, single-binary portable executable without runtime dependencies or heavy web-view bloat.

### 🪟 Windows 11 Setup Customizer (Bypass OOBE)
- **Automatic Tweaks Window:** Automatically activates when a Windows 10 or Windows 11 ISO is detected.
- **TPM & Secure Boot Bypass:** Removes minimum requirements for TPM 2.0, Secure Boot, and 4 GB+ RAM.
- **Offline Local Account:** Bypasses mandatory Microsoft account login requirements (`BypassNRO`) and provisions a local user.
- **Privacy & Encryption Tweaks:** Disables diagnostic data collection (privacy questions) and automatic BitLocker device encryption.
- **Zero In-Place Modification:** Generates clean, standard `autounattend.xml` embedded directly in the filesystem root.

### 🐧 Full Linux & Windows Parity
- **True Cross-Platform:** Complete feature set across Windows 10/11, Kali Linux, Ubuntu, Debian, Fedora, and openSUSE.
- **Universal File Chooser:** Automatically integrates with native file pickers (`zenity`, `kdialog`, Windows Explorer) and includes a **built-in visual file browser** for minimal or air-gapped systems (like out-of-the-box Kali Linux in VirtualBox).
- **Drag & Drop:** Drag any `.iso`, `.img`, `.bin`, `.vhd`, or `.raw` image directly from your desktop or file manager into the app.

### 🛡️ Defensive Drive Safety Guard
- **Host OS Isolation:** Strictly filters out internal fixed drives (`PhysicalDrive0`, primary SATA/NVMe disks) to prevent catastrophic data loss.
- **Safe Volume Handling:** Locks and dismounts volume handles before write operations and broadcasts filesystem refresh notifications upon completion.

### 🔍 Real-Time Checksums & Verification
- **Multi-Algorithm Hashing:** Background computation of **MD5**, **SHA-256**, and ultra-fast hardware-accelerated **CRC-32**.
- **Instant Matching:** Paste expected hashes directly into the verification box with automatic matching indicators.

### 🧪 Drive Health & Fake Flash Testing
- **Bad Block Scanning:** Zero-fill and pattern verification passes to detect bad sectors.
- **Counterfeit Detection:** Identifies fake-capacity USB drives before critical data loss occurs.

### 🎨 Adaptive Modern Interface
- **Dynamic Theme Engine:** Auto-detects system Dark and Light themes across Windows and Linux (GNOME, XFCE, KDE).
- **System Accent Synchronization:** Adopts native OS accent colors for a cohesive look and feel.

---

## 📦 Downloads & Releases

USB X is packaged as portable, self-contained standalone binaries:

| Platform | Package Format | Distribution Targets |
| :--- | :--- | :--- |
| **Linux (Universal)** | [**`.AppImage`**](dist/linux/USB_X-0.1.0-x86_64.AppImage) | All x86_64 Linux distributions (glibc 2.31+) |
| **Debian / Ubuntu / Kali** | [**`.deb`**](dist/linux/usb-x_0.1.0_amd64.deb) | Debian, Ubuntu, Kali Linux, Linux Mint, Pop!_OS |
| **Fedora / RHEL / SUSE** | [**`.rpm`**](dist/linux/usb-x-0.1.0-1.x86_64.rpm) | Fedora, CentOS, RHEL, openSUSE |
| **Windows** | [**`usb-x-app.exe`**](target/release/usb-x-app.exe) | Windows 10, Windows 11 (Portable Standalone) |

---

## 🚀 Quick Start

### Linux

#### Option 1: AppImage (No installation required)
```bash
chmod +x USB_X-0.1.0-x86_64.AppImage
./USB_X-0.1.0-x86_64.AppImage
```

#### Option 2: Debian / Kali / Ubuntu Package
```bash
sudo apt install ./usb-x_0.1.0_amd64.deb
usb-x
```

#### Option 3: Fedora / RPM Package
```bash
sudo rpm -i ./usb-x-0.1.0-1.x86_64.rpm
usb-x
```

### Windows

Simply run the standalone executable:
```cmd
usb-x-app.exe
```
*(Prompts for Administrator elevation automatically to access raw block storage devices)*

---

## 🏗️ Architecture

The workspace is organized into six decoupled, modular Rust crates:

```
USB X/
├── crates/
│   ├── usb-x-core/       # Domain types, partition schemes, hardware risk engine, event bus
│   ├── usb-x-image/      # ISO9660, UDF, El Torito parser, WIM detection, autounattend generator, hashing
│   ├── usb-x-platform/   # OS storage backends (Win32 IOCTL / Linux sysfs), elevation, file dialogs
│   ├── usb-x-engine/     # Multi-stage execution pipeline, sector streaming, verification
│   ├── usb-x-catalog/    # Official OS download catalog metadata (Windows, Ubuntu, Arch, etc.)
│   └── usb-x-app/        # Egui/Eframe desktop interface, dynamic theming, custom dialogs
├── dist/
│   └── linux/            # Packaging scripts and specifications for AppImage, DEB, and RPM
└── docs/                 # Detailed architectural and engineering specifications
```

---

## 🛠️ Building From Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (1.75+ recommended)
- **Windows:** MSVC or MinGW GCC
- **Linux:** `build-essential`, `pkg-config`, `libx11-dev`, `libwayland-dev`, `libgl1-mesa-dev`

### Compile for Local Platform
```bash
# Clone the repository
git clone https://github.com/usb-x/usb-x.git
cd usb-x

# Run all unit and integration tests
cargo test --workspace

# Build optimized release executable
cargo build --release -p usb-x-app
```

The resulting binary will be located at:
- **Windows:** `target/release/usb-x-app.exe`
- **Linux:** `target/release/usb-x-app`

### Build All Linux Packages (.AppImage, .deb, .rpm)
```bash
# Build the native Linux ELF binary
cargo build --release -p usb-x-app

# Package AppImage, Debian, and RPM bundles
python dist/linux/package_appimage.py
python dist/linux/package_deb.py
python dist/linux/package_rpm.py
```

Outputs are generated in `dist/linux/`.

---

## 🔒 Security & Data Safety

- **Destructive Operations:** Writing, formatting, and bad-block tests permanently overwrite target media.
- **Target Locking:** Physical drive enumeration isolates system storage. Flash actions require explicit confirmation.
- **Elevation Model:** The application requests administrative root permissions exclusively for low-level block device access and volume dismounting.

---

## 📄 License

This project is licensed under the [GNU General Public License v3.0 or later](LICENSE).
