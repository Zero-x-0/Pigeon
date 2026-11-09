#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationStage {
    Idle,
    ValidatingSafety,
    DismountingAndLocking,
    Partitioning,
    Formatting,
    WritingImage,
    ApplyingTweaks,
    CreatingPersistence,
    Verifying,
    SurfaceScanning,
    TestingCapacity,
    FlushingBuffers,
    Ejecting,
    Complete,
    Failed,
    Cancelled,
}

impl OperationStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Ready",
            Self::ValidatingSafety => "Validating device safety & locks...",
            Self::DismountingAndLocking => "Dismounting volumes and locking device...",
            Self::Partitioning => "Creating partition layout (MBR/GPT)...",
            Self::Formatting => "Formatting file system...",
            Self::WritingImage => "Writing image data...",
            Self::ApplyingTweaks => "Applying Windows 11 setup bypasses & unattend...",
            Self::CreatingPersistence => "Configuring Linux persistence partition...",
            Self::Verifying => "Verifying checksums and data integrity...",
            Self::SurfaceScanning => "Scanning surface for bad blocks...",
            Self::TestingCapacity => "Running write/read fake flash capacity test...",
            Self::FlushingBuffers => "Finalizing and flushing buffers to disk...",
            Self::Ejecting => "Safely ejecting media...",
            Self::Complete => "Ready",
            Self::Failed => "Operation failed",
            Self::Cancelled => "Operation cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationResult {
    Success { message: String },
    Cancelled,
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationEvent {
    StageChanged {
        stage: OperationStage,
        message: String,
    },
    Progress {
        completed_bytes: u64,
        total_bytes: Option<u64>,
        bytes_per_second: Option<u64>,
        eta_seconds: Option<u64>,
    },
    Log {
        level: LogLevel,
        timestamp: String,
        message: String,
    },
    Warning {
        message: String,
    },
    Finished {
        result: OperationResult,
    },
}
