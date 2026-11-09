# USB X — Rust-first architecture decision

This project is Rust-first. The former UI framework has been removed from the project.

## Approved stack

- **Rust stable:** primary language for application logic, disk workflows, image handling, validation, downloads, checksums, logging, and platform integrations.
- **egui + eframe:** Rust-native desktop UI for the Windows, Linux, and macOS targets.
- **C:** optional and limited to small, documented FFI shims or native APIs/libraries when no practical maintained Rust interface exists.

## Architecture rules

1. Keep UI presentation separate from disk operations.
2. Run long operations on worker threads and communicate with the UI through typed channels.
3. Put OS-specific storage access behind platform backend traits/modules.
4. Keep privileged operations in a minimal helper; never grant the whole UI process elevated privileges.
5. Implement and test a mock device backend before enabling writes to physical media.
6. Revalidate the target device immediately before every destructive operation.
7. Do not claim a filesystem alone guarantees BIOS, UEFI, or Secure Boot compatibility.

## Explicitly excluded

- The previously selected declarative UI framework
- Electron or an embedded browser UI
- Unnecessary C/C++ rewrites of Rust functionality
