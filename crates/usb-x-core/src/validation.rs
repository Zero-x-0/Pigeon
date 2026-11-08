use crate::device::DeviceInfo;
use crate::errors::{Result, UsbXError};
use crate::plan::{OperationKind, OperationPlan};
use crate::types::FileSystem;

pub fn validate_plan(plan: &OperationPlan, device: &DeviceInfo) -> Result<()> {
    // 1. Safety check: target device must not be a protected system disk
    if device.system_risk.is_destructive_blocked() {
        return Err(UsbXError::SystemDiskProtection(format!(
            "Disk '{}' ({}) contains the host operating system or system boot volumes. Destructive operations are strictly blocked to protect your system.",
            device.display_name, device.id
        )));
    }

    // 2. Validate device ID match
    if plan.target_device_id != device.id {
        return Err(UsbXError::SafetyViolation(
            "Target device identity mismatch. Re-enumeration required.".to_string(),
        ));
    }

    // 3. Operation-specific validations
    match &plan.kind {
        OperationKind::WriteImage {
            image,
            file_system,
            ..
        } => {
            if image.file_size > device.size_bytes {
                return Err(UsbXError::SafetyViolation(format!(
                    "Image file size ({} bytes) exceeds target device capacity ({} bytes).",
                    image.file_size, device.size_bytes
                )));
            }

            if *file_system == FileSystem::Fat32 && image.is_wim_split_needed {
                // Not necessarily fatal if split WIM or Rufus UEFI:NTFS bridge is used,
                // but validation notes it
            }
        }
        OperationKind::RestoreDrive { .. } => {
            // Restore requires explicit confirmation
        }
        _ => {}
    }

    Ok(())
}

