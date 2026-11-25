# Changelog

All notable changes to **Pigeon** are documented in this file.

## [1.11.26] - 2026-11-26

### Added
- **Rufus-Style Single-Pass Windows Boot (`UEFI:NTFS`, Linux & Windows):** Embedded Rufus `uefi-ntfs.img` (`1 MiB` FAT12 image containing signed UEFI NTFS/exFAT drivers and `EFI/Boot/boot*.efi`) in `usb-x-engine` so Windows setup ISOs extract **strictly once** onto an NTFS data partition (`Main Data Partition`) while Partition 2 (`UEFI:NTFS`) holds `uefi-ntfs.img` with GPT `msftdata` + no-drive-letter (`GUID:63` / `GPT_BASIC_DATA_ATTRIBUTE_NO_DRIVE_LETTER`) attributes or MBR type `0xef`.
- **Dynamic Window Auto-Sizing:** Main window automatically expands and contracts its height cleanly when expanding/collapsing *Show advanced drive properties*, *Show advanced format options*, or Linux persistence options while keeping fixed `430` px width and disabling maximize/stretch.
- **FreeDesktop XDG Portal Live Theme Detection:** Added `org.freedesktop.portal.Settings.Read` (`org.freedesktop.appearance` / `color-scheme`) support, `PKEXEC_UID` session resolution alongside `SUDO_USER`, and asynchronous background theme probing every `650` ms.
- **Log Clipboard Export & Dynamic Packaging Metadata:** Added **Copy** button (`arboard`) in the Log viewport and automatic workspace version synchronization across Linux `.deb`, `.rpm`, and `.AppImage` builders.

### Fixed
- **Double-Write Bug on Windows ISOs:** Eliminated duplicate extraction (`0% → 100%` twice) caused by copying `boot.wim` / `efi/` / `boot/` to an ~846 MiB FAT32 ESP prior to NTFS extraction.
- **Log Scrolling Lag & GNOME "Not Responding" Freeze:** Removed `80` ms progress-percentage log spam, replaced wrapped `ui.label` widgets in the log viewport with zero-wrapping O(visible) `painter.text()` rows and matching `16.0` px spacing, and throttled active UI repaints (`120` ms).
- **Live Light/Dark Theme Switching Failure:** Fixed bug where GNOME `'default'` color-scheme (when turning Dark Style off) fell through to frozen launch-time `PIGEON_SYSTEM_DARK=1` and took 30 seconds to re-poll. Switching desktop Light/Dark mode now updates the entire Pigeon UI in under a second across elevated (`pkexec`/`sudo`) and regular sessions.
- **Main Window Scrollbar Clutter:** Eliminated vertical scrollbar inside the main window when expanding options by resizing the viewport height dynamically.

### Changed
- Bumped workspace version from `1.11.0` to `1.11.26` across `Cargo.toml`, window title (`Pigeon 1.11.26`), Windows `app.manifest`, Debian/RPM package specs, and build scripts.
- Simplified Linux Log output to Rufus-style plain lines (`Extracting: /path/to/file (size)`), removing `[Now] INFO` / `[Stage]` brackets.

### Removed
- Removed obsolete `USB X` pre-migration files: `PROGRESS.md` and the 13 stale draft specifications inside `docs/`.
- Removed redundant 2-pass FAT32 ESP copy scaffolding (`copy_windows_efi_boot_files`, `is_windows_esp_boot_rel`, `boot_files_bytes`) from `usb-x-engine/src/iso_extractor.rs` now superseded by `UEFI:NTFS`.
- Removed Windows Setup tweak UI/unattend generation (`autounattend.xml` bypass claims cleaned from packaging specs and docs).
