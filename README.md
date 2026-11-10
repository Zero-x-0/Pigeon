<div align="center">

<img src="assets/pigeon.png" alt="Pigeon Logo" width="180"/>

# 🐦 Pigeon

**Next-Generation Bootable USB Creator & Disk Imaging Utility**  
**Version 1.11 (Production Release)**

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org)
[![Release: v1.11](https://img.shields.io/badge/Release-v1.11-brightgreen.svg)](https://github.com/Zero-x-0/Pigeon/releases/latest)
[![Platform: Windows & Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/Zero-x-0/Pigeon/releases/latest)

A blazingly fast, lightweight, and memory-safe desktop tool for flashing bootable media, creating raw disk images, bypassing Windows 11 restrictions, and checking drive health. Built in pure Rust with direct sector streaming.

[Downloads](#-downloads) • [Features](#-features) • [Quick Start](#-quick-start) • [Build from Source](#-build-from-source)

</div>

---

## 📦 Downloads

Pre-compiled production binaries for all platforms are available on the **[GitHub Releases Page](https://github.com/Zero-x-0/Pigeon/releases/latest)**:

| Platform / Distro | Package Format | Direct Download | Description |
| :--- | :--- | :--- | :--- |
| **Windows 10 / 11** | **Portable `.exe`** | [**`pigeon-1.11.0-windows-x64.exe`**](https://github.com/Zero-x-0/Pigeon/releases/latest) | Standalone portable executable (No install required) |
| **Linux (Universal)** | **AppImage** | [**`Pigeon-1.11.0-x86_64.AppImage`**](https://github.com/Zero-x-0/Pigeon/releases/latest) | Single portable executable for all modern Linux distros |
| **Debian / Ubuntu / Kali** | **`.deb` package** | [**`pigeon_1.11.0_amd64.deb`**](https://github.com/Zero-x-0/Pigeon/releases/latest) | Native package with desktop launcher & icon |
| **Fedora / RHEL / openSUSE** | **`.rpm` package** | [**`pigeon-1.11.0-1.x86_64.rpm`**](https://github.com/Zero-x-0/Pigeon/releases/latest) | Native RPM package |

---

## 🌟 Features

* **High Performance & Safe:** Direct sector streaming in pure Rust with zero garbage collection and strictly protected host system disks.
* **Windows 11 Customizer (OOBE Bypass):** Automatically removes restrictions for TPM 2.0, Secure Boot, and 4 GB+ RAM, bypasses mandatory online accounts (`BypassNRO`), and disables telemetry.
* **Desktop File Manager Integration:** Native integration with **Thunar** (XFCE / Kali Linux), **Dolphin** (KDE Plasma), **Nautilus** (GNOME / Ubuntu), and **Nemo** (Cinnamon / Mint).
* **Free-Floating Built-In File Manager:** When native dialogs are unavailable, Pigeon opens an independent, movable, and resizable desktop window with interactive breadcrumbs, direct path input, real-time search, and quick access to system drives.
* **Multi-Partition Auto-Cleaner:** Automatically cleans and repartitions USB drives with multiple existing partitions before writing to prevent boot corruption.
* **Background Checksums:** Computes **SHA-256**, **MD5**, and high-speed **CRC-32** hashes concurrently with instant matching against expected values.
* **Drive Health & Fake Flash Testing:** Non-destructive and pattern-based verification passes to detect bad blocks and counterfeit capacity drives.
* **Drag & Drop:** Drop any `.iso`, `.img`, `.bin`, `.raw`, or `.vhd` directly onto the window.
* **Adaptive Dark/Light Theme:** Matches OS appearance and system accent color on Windows and Linux.

---

## 🚀 Quick Start

### Linux

```bash
# Run standalone AppImage:
chmod +x Pigeon-1.11.0-x86_64.AppImage
sudo ./Pigeon-1.11.0-x86_64.AppImage

# Or install .deb on Kali / Debian / Ubuntu:
sudo dpkg -i pigeon_1.11.0_amd64.deb
sudo pigeon

# Or install .rpm on Fedora / RHEL:
sudo rpm -i pigeon-1.11.0-1.x86_64.rpm
sudo pigeon
```

### Windows

Download `pigeon-1.11.0-windows-x64.exe` from [Releases](https://github.com/Zero-x-0/Pigeon/releases/latest) and double-click or run from PowerShell:
```powershell
.\pigeon-1.11.0-windows-x64.exe
```
*(Prompts for Administrator privileges required for physical disk streaming)*

---

## 🛠️ Build from Source

```bash
# Clone the repository
git clone https://github.com/Zero-x-0/Pigeon.git
cd Pigeon

# Run tests
cargo test --workspace

# Build optimized binary
cargo build --release -p pigeon
```

To build all Linux release packages (`.deb`, `.rpm`, `.AppImage`):
```bash
bash dist/linux/build-all-linux.sh
```

---

## 📄 License

Pigeon is open-source software licensed under the [GNU General Public License v3.0 or later](LICENSE).
