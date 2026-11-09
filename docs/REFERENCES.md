# USB X — Technical References

This file collects upstream/official documentation relevant to implementation decisions. It is a starting point for engineering research, not a substitute for reviewing the current documentation and release notes before shipping. Links checked for this project brief on **4 November 2026**.

## Rufus behavior and user experience

- [Rufus FAQ](https://github.com/pbatard/rufus/wiki/FAQ) — Windows 11 setup options, account/OOBE behavior, ISO download availability, and destructive-target warnings. Use as behavior research, not as code to copy.

## Windows installation and disk management

- [Microsoft — Download Windows 11](https://www.microsoft.com/en-in/software-download/windows11) — official Windows 11 download/ISO landing page. Use the current official page rather than pinning a stale “latest” URL/version in the app.
- [Microsoft — Create installation media for Windows](https://support.microsoft.com/en-us/windows/deployment/install-upgrade/create-installation-media-for-windows) — official guidance for Windows installation media.
- [Microsoft Learn — Automate Windows Setup](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/automate-windows-setup?view=windows-11) — answer-file based Windows Setup automation.
- [Microsoft Learn — Answer files (unattend.xml)](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/update-windows-settings-and-scripts-create-your-own-answer-file-sxs?view=windows-11) — supported answer-file approach and setup configuration.
- [Microsoft Learn — Unattended Windows Setup Reference](https://learn.microsoft.com/en-us/windows-hardware/customize/desktop/unattend/) — setting reference for answer files.
- [Microsoft Learn — Windows To Go overview](https://learn.microsoft.com/en-us/windows/deployment/planning/windows-to-go-overview) — states Windows To Go was removed in Windows 10 version 2004 and later and describes its limitations.
- [Microsoft Learn — Format-Volume](https://learn.microsoft.com/en-us/powershell/module/storage/format-volume) — Windows Storage file-system formatting support and parameters.
- [Microsoft Learn — Get-Disk](https://learn.microsoft.com/en-us/powershell/module/storage/get-disk) — Windows disk enumeration.
- [Microsoft Learn — Initialize-Disk](https://learn.microsoft.com/en-us/powershell/module/storage/initialize-disk) — disk initialization and partition style.
- [Microsoft Learn — WinPE: split a WIM image](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/winpe--use-a-single-usb-key-for-winpe-and-a-wim-file---wim?view=windows-11) — Microsoft guidance for splitting Windows images into `.swm` files, including FAT32 limits.
- [Microsoft Learn — File system functionality comparison](https://learn.microsoft.com/en-us/windows/win32/fileio/filesystem-functionality-comparison) — file-system capability and limits table, including FAT32, NTFS, exFAT, and UDF.

## UEFI and Secure Boot

- [UEFI Forum — Specifications](https://uefi.org/specifications) — official UEFI specification releases. The current specification at the time of this brief is UEFI Specification 2.11.
- [UEFI Specification 2.11 — Secure Boot and Driver Signing](https://uefi.org/specs/UEFI/2.11/32_Secure_Boot_and_Driver_Signing.html) — authenticated boot-related data structures and signature handling.
- [Ubuntu Security — Secure Boot](https://documentation.ubuntu.com/security/security-features/platform-protections/secure-boot/) — practical explanation of shim, signed GRUB, kernel validation, and the Linux boot chain.

## Linux and flash diagnostics

- [F3 — Fight Flash Fraud documentation](https://fight-flash-fraud.readthedocs.io/en/latest/introduction.html) — tests flash-card capacity/performance; documents file-based and destructive block-device tests. `f3probe --destructive` destroys existing data, so it must never run without a separate destructive confirmation.
- [util-linux project](https://github.com/util-linux/util-linux) — common Linux utilities including block-device and partition tools. Review the manual pages for the exact versions supported by each distribution.
- [UDisks project](https://www.freedesktop.org/wiki/Software/udisks/) — desktop storage service and D-Bus integration.
- [Polkit documentation](https://www.freedesktop.org/software/polkit/docs/latest/) — authorization model for privileged actions on Linux.

## FreeDOS and Windows imaging

- [FreeDOS official downloads](https://www.freedos.org/download/) — official media, including USB and floppy editions. Use current official packages and respect their licenses.
- [wimlib](https://wimlib.net/) — cross-platform Windows Imaging (WIM) library and `wimlib-imagex`; verify version and license before bundling.

## macOS storage

- [Apple Support — Restore a disk image to a disk using Disk Utility](https://support.apple.com/en-in/guide/disk-utility/dskutl14078/mac) — Apple's disk-image restore workflow and caveats. Programmatic USB X behavior still requires review of current Disk Arbitration/IOKit and diskutil documentation.

## Rust desktop UI framework

- [eframe documentation](https://docs.rs/eframe/latest/eframe/) — official egui application framework; native Windows, Linux and macOS builds.
- [egui documentation](https://docs.rs/egui/latest/egui/) — immediate-mode Rust GUI API.
- [egui repository](https://github.com/emilk/egui) — source, examples, renderer and license details.

## Engineering notes derived from the references

1. Microsoft documents FAT32's 4 GiB maximum file size. Windows installation media may contain a WIM file too large for FAT32; Microsoft documents splitting WIM into `.swm` files. USB X should support that as one explicit, tested installer recipe.
2. Microsoft documents Windows To Go as removed from Windows 10 version 2004 and later. Keep this feature legacy/experimental and never describe it as currently supported by Microsoft.
3. Secure Boot validates signed boot components through a trust chain. A FAT32 partition alone does not make arbitrary boot media Secure-Boot-compatible.
4. F3's direct block-device probe is explicitly destructive. Full-capacity validation must be treated as an erase operation.
5. egui/eframe provide a Rust-native desktop UI path; pin the selected release and verify renderer/backend support on every target OS.
6. Microsoft Windows Setup answer files are a documented mechanism for automating setup choices. Specific setup behavior changes across Windows releases, so generated answer files and compatibility bypasses must be versioned and tested.

## Update policy

- Review upstream docs when a platform API, Windows release, egui/eframe release, bootloader, or helper tool changes.
- Record the exact tested version and digest of every bundled helper and boot component.
- Refresh the Windows ISO source catalog rather than hardcoding one “latest” download.
- Re-check Secure Boot signing and revocation status before publishing a boot recipe update.

## Rust and FFI guidance

- [Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html) — rules and caveats for crossing the Rust/C ABI boundary.
- [Rustonomicon: External Blocks](https://doc.rust-lang.org/reference/items/external-blocks.html) — Rust declarations for foreign functions.
