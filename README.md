<div align="center">

<img src="assets/pigeon.png" alt="Pigeon Logo" width="150"/>

# Pigeon

**Fast, Lightweight Bootable USB Creator & Disk Imaging Utility**
**Version 1.11.26**

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org)
[![Release: v1.11.26](https://img.shields.io/badge/Release-v1.11.26-brightgreen.svg)](https://github.com/Zero-x-0/Pigeon/releases/latest)
[![Platform: Linux & Windows](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-lightgrey.svg)](https://github.com/Zero-x-0/Pigeon/releases/latest)

A native Rust (`egui` + `eframe`) desktop utility modeled after Rufus for creating bootable USB media, streaming raw or compressed disk images, and verifying flash drives on Linux and Windows.

[Downloads](#-downloads) • [Current Functionality](#-current-functionality) • [Shortcuts](#-keyboard-shortcuts) • [Quick Start](#-quick-start) • [Build from Source](#-build-from-source) • [Changelog](CHANGELOG.md)

</div>

---

## Downloads

Release builds are automated via [`.github/workflows/release.yml`](.github/workflows/release.yml) and published on **[GitHub Releases](https://github.com/Zero-x-0/Pigeon/releases/latest)**:

| Platform / Distro | Format | Artifact |
| :--- | :--- | :--- |
| **Linux (Universal)** | **AppImage** | `Pigeon-1.11.26-x86_64.AppImage` |
| **Debian / Ubuntu / Kali** | **`.deb`** | `pigeon_1.11.26_amd64.deb` |
| **Fedora / RHEL / openSUSE** | **`.rpm`** | `pigeon-1.11.26-1.x86_64.rpm` |
| **Windows 10 / 11** | **Portable `.exe`** | `pigeon-1.11.26-windows-x64.exe` |

---

## Current Functionality

### Bootable USB & Disk Imaging
- **Single-Pass Windows Boot (`UEFI:NTFS`, Linux & Windows):**
  - Creates a dual-partition layout combining an **NTFS** data partition (`Main Data Partition`) with Rufus's embedded 1 MiB FAT12 `uefi-ntfs.img` partition (`UEFI:NTFS`) on both **GPT** (`msftdata` + no-drive-letter attribute) and **MBR** (`0x07` active + `0xEF`).
  - Extracts Windows setup ISOs in a single `0% → 100%` pass without duplicate `boot.wim` copying and supports `>4 GB` `install.wim` natively.
  - Supports Legacy BIOS boot on MBR (sector 0 boot code / optional Linux `grub-install` chainloader when available).
- **Linux ISOHybrid & Persistent Storage:**
  - Supports both **ISO Image Mode** (filesystem extraction + UEFI `/EFI/BOOT/bootx64.efi` fallback) and **DD Image Mode** (raw sector write).
  - Creates an optional secondary `ext4` persistence partition (`casper-rw` for Ubuntu derivatives or `persistence` with `/persistence.conf` for Debian/Kali).
- **Compressed & Raw DD Streaming:**
  - Streams `.img`, `.bin`, `.raw`, `.vhd`, `.dd`, `.gz`, `.xz`, `.bz2`, and `.zst` directly to block devices with 4 MiB chunks and optional post-write SHA-256 verification.
- **High-Throughput Linux I/O:**
  - Uses kernel `ntfs3` (`prealloc` / `posix_fallocate`) and pipelined asynchronous `sync_file_range` writeback to avoid flush stalls.
- **Non-Bootable Formatting & Options:**
  - Formats drives as `FAT32`, `NTFS`, or `exFAT` with selectable cluster sizes, quick/full format, optional `autorun.inf` extended label/icon files, and MBR old-BIOS partition alignment.

### Interface, Theme & Diagnostics
- **Dynamic Auto-Sizing Window:**
  - Fixed `430` px width with non-maximizable, non-stretchable bounds that automatically grow and shrink vertically when collapsible panels (*Show advanced drive properties*, *Show advanced format options*) or persistence sliders are shown.
- **Live Light & Dark Theme Sync:**
  - Automatically syncs with FreeDesktop XDG Portal (`org.freedesktop.appearance.color-scheme`), GNOME `color-scheme`/`gtk-theme`, XFCE, and KDE every `650` ms across both `pkexec`/`sudo` and regular Linux sessions, and Win32 `AppsUseLightTheme`/`AccentColor` on Windows (overridable via Settings or `Ctrl+Alt+D`).
- **Log Window:**
  - Displays clean `Extracting: /path/to/file (size)` logs without progress-percentage clutter, smooth single-line virtualized scrolling, **Copy**, **Clear**, and async **Save** (`pigeon.log`).
- **ISO Selector, Downloader & Built-In File Browser:**
  - Supports drag-and-drop, native desktop file dialogs (with a floating cross-platform browser fallback), and an **ISO Download** dialog supporting direct official Ubuntu/Arch ISO downloads or opening Microsoft's official Windows download portal.
- **Drive Safety, Checksums, Backup & Health Checks:**
  - Lists external USB and SD storage devices (`<Name> [<Size>]`) with optional USB HDD toggle (`Alt+F`), blocks destructive writes to host OS/system disks, and requires typed confirmation before erasing media.
  - Includes background image hashing (`MD5`, `SHA-1`, `SHA-256`, `SHA-512`, `CRC-32`), raw drive backup to `.img`, and multi-pass bad-blocks / counterfeit-capacity verification (`1`, `2`, or `4` passes).

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| **`F5`** | Refresh connected external USB / SD devices |
| **`Alt+H`** | Compute and display image checksums (with SHA-512) |
| **`Alt+F`** | Toggle listing external USB hard drives |
| **`Ctrl+Alt+D`** | Toggle Light / Dark theme override |

---

## Quick Start

### Linux
```bash
# Portable AppImage
chmod +x Pigeon-1.11.26-x86_64.AppImage
./Pigeon-1.11.26-x86_64.AppImage

# Debian / Ubuntu / Kali (.deb)
sudo dpkg -i pigeon_1.11.26_amd64.deb
pigeon

# Fedora / RHEL / openSUSE (.rpm)
sudo rpm -i pigeon-1.11.26-1.x86_64.rpm
pigeon
```
*(Prompts for root elevation via `pkexec` or `sudo` on launch for raw block-device access.)*

### Windows
Download and run `pigeon-1.11.26-windows-x64.exe` (requests Administrator UAC elevation for raw physical drive access).

---

## Build from Source

```bash
# Check workspace and run unit tests
cargo check --workspace --all-targets
cargo test --workspace

# Build Linux packages (.AppImage, .deb, .rpm) into ./release/
bash dist/linux/build-all-linux.sh
```

To compile the Windows x64 executable (requires MSYS2 MinGW64 on Windows, or `mingw-w64` on Linux):
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release -p pigeon --target x86_64-pc-windows-gnu
```

---

## Changelog & License

- Release history: [CHANGELOG.md](CHANGELOG.md)
- License: [GNU General Public License v3.0 or later](LICENSE)
