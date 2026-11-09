use crate::types::{BusType, SystemDiskRisk};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

impl DeviceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionInfo {
    pub number: u32,
    pub mount_point: Option<String>,
    pub size_bytes: u64,
    pub file_system: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub display_name: String,
    pub model: String,
    pub vendor: String,
    pub size_bytes: u64,
    pub logical_sector_size: u32,
    pub bus_type: BusType,
    pub removable: bool,
    pub system_risk: SystemDiskRisk,
    pub partitions: Vec<PartitionInfo>,
    pub serial_redacted: Option<String>,
    pub device_path: String,
}

impl DeviceInfo {
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size_bytes)
    }

    pub fn summary(&self) -> String {
        format!("{} [{}]", self.display_name, self.formatted_size())
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.1} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
