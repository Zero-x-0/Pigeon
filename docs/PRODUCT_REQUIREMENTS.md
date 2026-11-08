# USB X — Product Requirements

## 1. Purpose

USB X is a compact cross-platform desktop utility for preparing removable storage and virtual disk images. It should make common workflows—formatting a drive, writing an ISO, checking an image, or restoring a backup—clear to a non-expert while preserving the control and diagnostics expected by advanced users.

## 2. Users and primary use cases

- A user creating a Windows or Linux installer USB.
- A technician formatting USB sticks and memory cards for ordinary storage.
- A developer writing a raw image to a removable device or virtual disk file.
- A user making a full-disk backup before changing a drive.
- A technician diagnosing a drive that reports the wrong capacity or has read/write errors.

## 3. Supported host platforms

The product is intended for Windows, Linux, and macOS. Do not claim complete parity on day one.

| Platform | Initial target | Notes |
|---|---|---|
| Windows | Windows 10 22H2+ and Windows 11, x64 first | Native Storage/Win32 backend; elevation through UAC only for privileged steps. Windows 10 itself is out of mainstream support, so document it as compatibility support, not a recommendation to keep using it. |
| Linux | Mainstream desktop distributions, x86_64 first; aarch64 after CI coverage | udev/DeviceKit or UDisks integration for enumeration and unmount; narrow privileged helper/polkit authorization for raw I/O. External format utilities may be required. |
| macOS | macOS 13+; Apple Silicon and Intel after hardware CI | Disk Arbitration / IOKit enumeration and diskutil-backed operations. NTFS formatting and some low-level tests may be unavailable without a separately reviewed backend. |

**Release policy:** A feature is enabled per host only when its backend reports that the exact operation is supported. Do not enable a menu item simply because the file-system name exists in the shared enum.

## 4. Device types

- USB flash drives and external SSD/HDDs.
- SD and microSD cards exposed as block storage.
- Removable USB card readers when the underlying media can be identified.
- File-backed virtual targets (raw `.img` initially; additional virtual-disk formats only after safe attach/detach support exists).
- OS-attached virtual disks only when the OS reports a stable identity and the backend can prove that the selected target is not the system disk.

Do not treat a mounted ISO, optical drive, system disk, network share, or arbitrary regular file as a physical drive.

## 5. Functional requirements

### 5.1 Device inventory

- Refresh device list on demand and subscribe to device-arrival/removal events where supported.
- Show model/vendor, media type, connection/bus, reported capacity, current partition/file-system summary, and a stable device identifier.
- Display a human-readable path (`Disk 2`, `/dev/disk4`, etc.) as a locator, not as the only identity.
- Identify and disable system/boot disks, ambiguous disks, and disks that cannot be locked.
- Detect target removal during any operation and fail safely.

**Acceptance:** The mock backend and hardware test suite prove that the selected stable ID is revalidated immediately before each destructive operation. A device-list reorder cannot redirect a write to a different disk.

### 5.2 Formatting

- Create a new MBR or GPT partition table where supported.
- Create, label, and format a volume.
- Support FAT32, NTFS, exFAT, and UDF only on host backends that can create and validate them.
- Allow quick format and full format as separately explained options.
- Provide allocation-unit/cluster-size choices only when supported and safe; default to OS/tool recommendation.
- Warn that formatting removes existing file-system data and that full format takes longer and writes across the volume.

**Compatibility notes:**

- FAT32 is the broadest interoperability choice, but a single file cannot reach 4 GiB; its maximum file size is 4 GiB (4,294,967,296 bytes), so a conservative installer workflow must keep individual files below the format limit.
- NTFS is a good Windows data/installer choice but is not a universally readable UEFI boot file system. macOS NTFS write/format support is not assumed.
- exFAT is a strong cross-platform data-storage choice, but it is not a universal firmware boot file system.
- UDF is an advanced/legacy option associated with optical-media workflows. UDF on a USB drive is not a general boot-compatibility promise.

### 5.3 Image selection and inspection

- Browse for ISO, IMG, RAW, and supported disk-image formats.
- Show path, file size, detected OS/distribution where possible, architecture where detectable, partition-table indicators, and boot capabilities where detectable.
- Never execute image contents during inspection.
- Offer SHA-256 and SHA-512 by default; support BLAKE3 as an optional fast digest. MD5/SHA-1 may be available only for legacy comparison and must be marked cryptographically obsolete.
- Let users paste or import an expected hash. Compare in constant-time where applicable and report pass/fail/mismatch clearly.
- Distinguish “hash calculated” from “verified against trusted publisher checksum/signature”.

### 5.4 Raw image writing

- Write selected compatible disk images byte-for-byte to the selected target.
- Stream data with bounded memory and aligned buffers where appropriate.
- Validate target capacity before writing; explain when the image would not fit.
- Flush buffers and perform optional or automatic read-back verification.
- Show source, target, bytes written, rate, stage, and result.
- A failed write is reported as incomplete and must never be represented as bootable.

### 5.5 Prepared installer mode

- Use a known recipe for a supported Windows ISO or Linux distribution/version.
- Inspect ISO layout and detect files that exceed FAT32's per-file size limit.
- For Windows media, offer FAT32 plus WIM splitting where supported, or a tested NTFS + UEFI boot bridge path where appropriate. Never imply every firmware accepts NTFS booting.
- Preserve signed EFI files from the source image. Any bridge bootloader must have a documented signature chain and revocation policy.
- Show a preview/summary of files and partitions that will be created.
- Leave the original ISO untouched; work from a temporary extracted copy.

### 5.6 BIOS and UEFI

- Offer Auto/Recommended, Legacy BIOS, UEFI, and “Both” only when the selected recipe supports the requested mode.
- Support MBR/GPT layout profiles when supported by the image and host backend.
- Detect whether a selected image already contains an EFI System Partition or boot catalog where practical.
- Explain when CSM/Legacy boot is disabled by the target PC firmware.
- Secure Boot compatibility must be reported as: **Preserved signed chain**, **Signed bridge chain**, **Unsigned/likely blocked**, or **Not verified**.

### 5.7 FreeDOS and MS-DOS

- Provide a curated FreeDOS workflow using official FreeDOS images/packages. Do not silently download or bundle an image without displaying its source and license.
- Support classic MS-DOS media only from files supplied by the user, with a warning that the user is responsible for having the required license and rights.
- DOS boot media is primarily a legacy BIOS/CSM use case; do not advertise UEFI boot unless a specific verified UEFI-compatible DOS environment is provided.

### 5.8 Windows To Go / portable Windows workspace

- Include as an **Experimental / Legacy** workflow only.
- Microsoft removed Windows To Go starting with Windows 10 version 2004; it does not support feature updates and depends on hardware that may no longer be available. Clearly state that it is not a current Microsoft-supported feature.
- Require a compatible Windows image and a suitable external SSD/USB device; explain performance and hardware limitations.
- Do not include it in the default workflow or describe it as equivalent to a supported portable Windows installation.

### 5.9 Linux persistence

- Support a maintained list of distribution/version-specific persistence recipes.
- Display whether persistence is file-backed or partition-backed and the selected capacity.
- Check free target space and partition layout before enabling.
- Reject unknown or unsupported ISOs rather than applying a generic `persistence` label and assuming it works.
- Run boot tests in supported VM configurations and hardware smoke tests for every recipe.

### 5.10 Backup and restore

- Create a full-device raw backup including partition table and all addressable sectors, not only visible files.
- Save an adjacent manifest containing source device geometry, creation time, image format, and SHA-256 digest.
- Offer optional streaming compression (for example, zstd) without loading the whole image into memory.
- Verify the completed backup by hashing it; optionally re-read the source where practical.
- Restore only after a separate, high-visibility confirmation that names the destination device and states all data will be overwritten.
- Compare the backup image's required size with target capacity and explain smaller/larger target handling.

### 5.11 Bad-block and fake-capacity testing

Provide two distinct tests:

1. **Read-only scan:** Reads accessible address ranges and records I/O errors. It does not prove the entire device can store the capacity it reports.
2. **Full write/read capacity test:** Writes test patterns and reads them back across the selected range. This is destructive and can overwrite every tested byte; it must be opt-in, require confirmation, and warn of flash wear.

- Use deterministic pseudorandom test data, streaming chunks, and a recorded seed so mismatches can be reproduced.
- Report first/last failing offsets, count of failed blocks, tested range, read/write rates, and whether the test covered the full reported capacity.
- Never infer genuine capacity from the device's reported size alone.
- Where supported, integrate a maintained upstream tool such as F3 rather than inventing a heuristic. Do not automatically shrink partitions or “repair” suspicious drives.
- Fake-capacity result must use cautious wording: **Suspected capacity mismatch**, **No mismatch observed in tested range**, or **Inconclusive**.

### 5.12 Official Windows ISO download

- Provide a Windows download screen that opens or uses a maintained catalog of official Microsoft download pages.
- Never scrape undocumented/private endpoints or bypass vendor download restrictions.
- Show edition/release, architecture, source domain, file size when known, and download progress.
- If Microsoft does not publish a checksum for the selected artifact, state that USB X can calculate a hash but cannot authenticate the file against a publisher hash.
- Keep download functionality independent from the disk-write engine; download first, verify, then ask the user to select the target.

The official Microsoft download site is the source of truth; the catalog should not hardcode a “latest” version forever.

### 5.13 Windows 11 setup options

- Present optional setup customizations after the user selects a supported Windows image and presses the main action, similar to the Rufus interaction pattern.
- Options may include compatibility bypasses for TPM/Secure Boot/RAM checks where legally and technically appropriate, privacy/OOBE choices, and an offline local-account path.
- Use documented Windows Setup answer-file mechanisms where possible. Version-sensitive compatibility bypasses must be isolated in versioned, tested profiles and explicitly described as unsupported by Microsoft.
- Preview the planned answer file and summarize each selected change before writing media.
- Never save credentials, account passwords, product keys, or personal information in the answer file by default.
- Do not disable Defender, Windows Update, firewall, or security protections as a hidden side effect.
- Warn that installing Windows on unsupported hardware can create reliability, update, and support risks. USB X cannot guarantee installation or future updates.

### 5.14 Progress and logs

- Every operation must show current stage, progress, bytes processed/total where known, speed, and ETA when it can be calculated honestly.
- Stages include discovery, safety validation, unmount/lock, partitioning, formatting, writing, verification, flush/eject, and cleanup.
- A detailed log records timestamp, operation ID, stage, target alias, source filename, important parameters, warnings, errors, and final status.
- Log export is supported. Clear Log is available when no operation is active.
- Do not record credentials or full serial numbers by default.

## 6. Quality attributes

- **Lightweight:** No embedded browser runtime required for the primary UI; no required account, cloud sync, or background daemon.
- **Portable:** The normal UI can run from an extracted folder. Operations requiring raw disk access may request OS permission; true privilege-free operation is not promised.
- **Reliable:** All long I/O is streamed; errors are surfaced; writes are flushed; optional read-back verification is supported.
- **Secure:** Minimize privileged code; signed releases; trusted downloads; no shell injection; safe device revalidation.
- **Accessible:** Keyboard navigation, readable scaling, screen-reader labels, high contrast, and clear warnings.

## 7. Out of scope for v1

- Automatic online image catalog for every Linux distribution.
- Arbitrary ISO-to-USB conversion.
- Firmware-level Secure Boot key management.
- Automatic bad-sector remapping or fake-drive repair.
- Remote device provisioning or cloud management.
- A built-in operating-system image mirror.
