use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbXError {
    DeviceNotFound(String),
    SafetyViolation(String),
    SystemDiskProtection(String),
    DeviceChangedOrRemoved,
    LockFailed(String),
    Io {
        operation: String,
        message: String,
    },
    ImageError(String),
    VerificationMismatch {
        expected: String,
        actual: String,
    },
    FakeFlashDetected {
        offset_bytes: u64,
        reported_bytes: u64,
    },
    Cancelled(String),
    UnsupportedCapability(String),
    InvalidParameter(String),
}

impl fmt::Display for UsbXError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound(dev) => write!(f, "Device not found: {}", dev),
            Self::SafetyViolation(msg) => write!(f, "Safety check failed: {}", msg),
            Self::SystemDiskProtection(msg) => write!(f, "Target is a protected system disk: {}", msg),
            Self::DeviceChangedOrRemoved => write!(f, "Device was disconnected or changed during operation"),
            Self::LockFailed(msg) => write!(f, "Volume lock failed: {}", msg),
            Self::Io { operation, message } => write!(f, "I/O error during {}: {}", operation, message),
            Self::ImageError(msg) => write!(f, "Image error: {}", msg),
            Self::VerificationMismatch { expected, actual } => {
                write!(f, "Verification mismatch! Expected: {}, Actual: {}", expected, actual)
            }
            Self::FakeFlashDetected { offset_bytes, reported_bytes } => {
                write!(
                    f,
                    "FAKE FLASH DETECTED! Sector wrapping at offset {} bytes (reported {} bytes)",
                    offset_bytes, reported_bytes
                )
            }
            Self::Cancelled(msg) => write!(f, "Operation cancelled: {}", msg),
            Self::UnsupportedCapability(msg) => write!(f, "Unsupported capability: {}", msg),
            Self::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
        }
    }
}

impl std::error::Error for UsbXError {}

pub type Result<T> = std::result::Result<T, UsbXError>;
