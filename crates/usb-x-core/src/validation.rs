use crate::device::DeviceInfo;
use crate::errors::{Result, UsbXError};
use crate::plan::{OperationKind, OperationPlan};

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
    if let OperationKind::WriteImage { image, .. } = &plan.kind {
        // Uncompressed image must fit within target device capacity; compressed size may expand
        if image.compressed_format().is_none() && image.file_size > device.size_bytes {
            return Err(UsbXError::SafetyViolation(format!(
                "Image file size ({} bytes) exceeds target device capacity ({} bytes).",
                image.file_size, device.size_bytes
            )));
        }
    }

    Ok(())
}
