# USB X — Feature and Compatibility Matrix

This matrix is a proposed release policy. It is not a statement that these features are already implemented. `Planned` means in scope; each cell must be changed to Supported only after the tests in `TEST_PLAN.md` pass.

| Feature | Windows | Linux | macOS | Notes |
|---|---|---|---|---|
| Device inventory | Planned | Planned | Planned | Stable ID and system-disk protection required. |
| USB / SD formatting | Planned | Planned | Planned | Only file systems the active backend can create and verify. |
| FAT32 | Planned | Planned | Planned | Broad compatibility; 4 GiB per-file ceiling. |
| NTFS format | Planned | Planned | Capability-gated | Native macOS NTFS write/format support not assumed. |
| exFAT | Planned | Planned | Planned | Good storage interoperability; not generic UEFI boot media. |
| UDF | Capability-gated | Planned | Unavailable until tested | Advanced option; not universal boot USB. |
| Raw ISO/IMG writer | Planned | Planned | Planned | Must verify image/target size and revalidate device. |
| Windows installer preparation | Planned | Planned | Planned via cross-host recipe | WIM split and UEFI bridge paths require individual validation. |
| Linux installer preparation | Planned | Planned | Planned | Only curated image recipes. |
| Legacy BIOS / UEFI | Planned | Planned | Planned | Bootability belongs to each image recipe. |
| Secure Boot reporting | Planned | Planned | Planned | Preserve and verify signed chain; no blanket guarantee. |
| FreeDOS | Planned | Planned | Planned | Official image/source; legacy boot focus. |
| User-supplied MS-DOS | Planned | Planned | Planned | User must supply lawful media; no bundled copyrighted binaries. |
| Windows To Go | Experimental | Experimental | Experimental | Legacy; Microsoft removed feature starting Windows 10 version 2004. |
| Linux persistence | Planned | Planned | Planned | Only supported distro/version recipes. |
| Image checksum | Planned | Planned | Planned | SHA-256/SHA-512; BLAKE3 optional. |
| Full device backup | Planned | Planned | Planned | Raw read with manifest and digest. |
| Restore image | Planned | Planned | Planned | Destructive confirmation + target revalidation. |
| Read-only bad-block scan | Planned | Planned | Planned | Not equivalent to full-capacity validation. |
| Destructive fake-capacity test | Planned | Planned | Capability-gated | Full write/read test; erases tested range. |
| Official Windows ISO download | Planned | Planned | Planned | Use official Microsoft download sources. |
| Windows setup privacy/local-account tweaks | Planned | Planned | Planned | Versioned answer-file templates, preview, no stored secrets. |
| Portable UI build | Planned | Planned | Planned | OS permissions still required for raw disk operations. |

## File-system selection quick guide

| User goal | Recommended choice | Important limitation |
|---|---|---|
| General USB data storage across devices | exFAT | Not a universal firmware boot file system. |
| Maximum legacy compatibility / many UEFI installers | FAT32 | A single file must remain below 4 GiB. |
| Windows-centric large-file storage | NTFS | Firmware and macOS support varies. |
| Specialized optical-media-style usage | UDF | Do not choose it as a generic bootable USB default. |
| Generic Linux installer USB | Raw write of the official hybrid ISO when supported | Do not reformat/extract unless a tested recipe calls for it. |

## UI policy

The application may display all four requested file systems in the selector, but unsupported choices must be disabled or explained for the current host, target, and operation. Do not let an unavailable backend fail only after the user has confirmed an erase.
