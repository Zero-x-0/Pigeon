# USB X — UI Specification

> **Implementation:** Build this screen in Rust with `egui` + `eframe`. Use reusable Rust components/functions for section headers, device selectors, file pickers, option rows, progress, and log panels. Keep disk operations off the UI thread and send typed events to the UI through bounded channels.

## 1. Visual direction

Use the supplied screenshot as the primary layout reference. USB X should feel like a familiar desktop utility: compact, quiet, clear, and practical. Keep the classic form-based layout rather than replacing it with a web dashboard.

- Light neutral background with restrained borders and separators.
- Blue is the primary action/accent color; use red/orange only for destructive warnings and errors.
- Clear section numbers and labels.
- Standard controls: dropdowns, checkboxes, file picker, tabs/segmented choices, progress bar, and a scrollable monospaced log.
- Simple USB/device symbols. Avoid decorative or overly detailed iconography.
- Use resizable window layouts with a sensible minimum size; no horizontal clipping at the minimum supported desktop size.

## 2. Main window layout

### Title bar

- USB X application icon and name.
- Subtitle: `Create Bootable USB Drives`.
- Standard minimize, maximize/restore, and close controls.
- Version and portable-mode marker in an About dialog, not as permanent clutter.

### Section 1 — Device

- Label: `1. Device`.
- Helper text: `Select USB drive, memory card or virtual drive`.
- Wide device selector.
- Refresh button.
- Each item displays a clear summary such as `SanDisk Ultra · USB 3.0 · 28.8 GB · /dev/sdb`.
- A small warning indicator identifies a suspicious or unsupported target.
- Never select a device automatically when there is more than one possible target. A single device may be preselected for convenience, but no operation may begin automatically.
- The selected device summary remains visible in the final confirmation and operation view.

### Section 2 — Boot Selection

- Label: `2. Boot Selection`.
- Mode selector: `ISO / disk image`, `FreeDOS`, `MS-DOS (user files)`, `Backup`, `Restore`, `Format only`, `Flash test`.
- For image mode: path field + Browse button.
- After scan, show detected OS/image name, architecture where known, file size, and image validation state.
- Provide a checksum action or expandable details area. Do not overload the primary screen with all digest algorithms.
- Display a green check only when a supported validation actually succeeded; use neutral “Detected” text for heuristic detection.

### Section 3 — Partition Scheme

Two aligned controls:

- Partition scheme: `Auto`, `GPT`, `MBR`.
- Target system: `Auto`, `UEFI`, `Legacy BIOS`, `UEFI + BIOS` (only show options supported by the selected image recipe).

When `Auto` is selected, show a short explanation of the detected recommendation. Do not silently force a mode if the input is ambiguous; explain the default and allow the user to change it.

### Section 4 — File System

Two aligned controls:

- File system: `Auto`, `FAT32`, `NTFS`, `exFAT`, `UDF`.
- Cluster size: `Default` plus only the values supported by the selected backend.

Options:

- `Quick format` — default on for ordinary formatting, explained as not a complete surface test.
- `Check device for bad blocks` — a separate destructive/slow workflow selection; if selected, present the exact type of scan and warning.
- `Create extended label and icon files` — optional compatibility convenience, with no untrusted executable content.

Show a context warning under the selector when the chosen file system is a poor match for the image or boot firmware. Examples: NTFS may require a signed UEFI bridge; exFAT and UDF are not universal UEFI boot file systems; FAT32 cannot hold a single 4 GiB-or-larger file.

### Section 5 — Additional Options

Use a compact tab/segmented row:

- `ISO (default)`
- `Windows To Go`
- `Linux Persistence`
- `Other`

Only enable a tab when relevant to the selected source image and host backend. Unsupported features show an explanation rather than a dead control.

**ISO tab**

- For supported Windows ISOs, optional checkbox: `Download official Windows ISO` with official Microsoft source link.
- Version/architecture selector populated from the current official catalog; never a stale hard-coded “latest” label.
- Windows 11 setup compatibility options presented in a secondary dialog after Start, not as a large group of permanent checkboxes.

**Windows To Go tab**

- Visible `Experimental / Legacy` label.
- Explain that Microsoft removed Windows To Go beginning with Windows 10 version 2004 and that the feature is unsupported.
- Show required image/device compatibility before allowing Start.

**Linux Persistence tab**

- Distribution/recipe (detected or explicitly selected).
- Persistence size slider/field.
- Storage type: persistent partition or persistent file, depending on recipe.
- Clear compatibility note and supported ISO version.

**Other tab**

- FreeDOS / user-supplied MS-DOS media.
- Create image backup.
- Restore image backup.
- Checksum verification.
- Read-only surface scan or destructive capacity test.
- Virtual disk image creation/target selection where supported.

### Primary actions

- Large blue `Start` button.
- Secondary `Close` button.
- Before any destructive operation, Start opens a review/confirmation dialog containing:
  - Operation type.
  - Exact target model, capacity, and stable identity summary.
  - Source image/backup and expected size.
  - Partition scheme and file system.
  - Whether verification or destructive testing is enabled.
  - A plain-language statement that target data will be erased/overwritten.
  - Explicit `Cancel` and `Erase and Start` actions.

For full-device restore and destructive capacity tests, require the user to type `ERASE` or the displayed device name.

### Progress

- Label: `Progress`.
- A determinate bar only when total work is known; otherwise an animated indeterminate indicator with the current phase text.
- Show percent, bytes processed, total bytes, write/read rate, and ETA when reliable.
- Show current phase separately (for example `Writing image`, `Verifying`, or `Flushing data`).
- Provide `Cancel` with clear wording about the current safe cancellation point.

### Log

- Label: `Log`.
- Scrollable monospaced event view, newest events appended at the bottom.
- Each line includes time and severity; sample:

```text
[16:42:10] INFO  USB X v0.1.0 (Portable)
[16:42:10] INFO  Target: SanDisk Ultra · USB 3.0 · 28.8 GB
[16:42:10] INFO  Source: ubuntu-24.04-desktop-amd64.iso · 2.8 GB
[16:42:10] INFO  Mode: Raw image write · GPT/UEFI detected
[16:42:11] INFO  Safety checks passed; target locked and unmounted
[16:42:12] INFO  Writing image: 64 MiB / 2.8 GiB
```

- `Copy Log` and `Save Log` buttons.
- `Clear Log` disabled during an operation.
- Logs should use severity labels and readable wording, not expose raw stack traces in the default view. Full diagnostic details can be included in an export.

### Status bar

- Bottom-left state: `Ready`, `Scanning`, `Writing`, `Verifying`, `Cancelled`, `Completed`, or `Failed`.
- Optional version/build channel on the right.

## 3. Secondary dialogs

### Windows setup customization

Display only for a recognized Windows installation image when the selected media recipe supports customization. Present grouped toggles for compatibility, privacy/OOBE, and local-account setup. Each option has an explanation and a compatibility note. Include `Preview answer file` and a clear reset-to-default action. Require explicit confirmation before applying the options to the USB media.

### Checksum dialog

- Algorithms: SHA-256, SHA-512, BLAKE3; optional legacy MD5/SHA-1 labelled as non-secure.
- Expected hash input or manifest selection.
- Result states: `Calculated`, `Matches`, `Mismatch`, `Publisher signature verified`, `No trusted reference available`, `Read error`.

### Device confirmation dialog

Make the target impossible to overlook. Use the drive model and size as the strongest text, show the OS path as a secondary identifier, and state that all existing data will be lost. If more than one device has a similar model/capacity, include the serial suffix or hardware ID to distinguish them.

## 4. Responsive behavior

- At wide desktop sizes, use two-column rows for paired controls, as shown in the reference.
- At narrower window widths, stack paired controls vertically.
- The log can grow vertically and has a minimum useful height; it must not push the Start/Cancel control off-screen.
- Long paths truncate with a tooltip and remain copyable.
- Device names and capacities must not truncate to the point of ambiguity.

## 5. Interaction states

- `Ready`: device and source selection enabled.
- `Scanning image`: source controls disabled; cancel scan available.
- `Awaiting confirmation`: no disk operation has started.
- `Preparing`: revalidate device, acquire permissions, unmount, lock.
- `Writing/Formatting/Testing`: target selection and configuration locked.
- `Verifying`: write progress complete but operation not yet successful.
- `Completed`: show summary and verification result.
- `Failed`: show the failed stage, likely cause, and safe next action.
- `Cancelled`: state whether no write began, partial data was written, or the target is left in an unknown state.

## 6. Accessibility

- Every control has a programmatic label and keyboard-accessible focus.
- Do not communicate status by color alone.
- Support larger text and high-contrast themes.
- Confirmation dialogs must be reachable and usable without a mouse.
- Announce operation stage changes to assistive technology without announcing every progress tick.
