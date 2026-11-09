#[derive(Debug, Clone, PartialEq)]
pub struct WindowsIsoEntry {
    pub name: &'static str,
    pub release: &'static str,
    pub build: &'static str,
    pub architecture: &'static str,
    pub approx_size_gb: f32,
    pub official_url: &'static str,
    pub description: &'static str,
}

pub fn get_official_windows_catalog() -> Vec<WindowsIsoEntry> {
    vec![
        WindowsIsoEntry {
            name: "Windows 11 (24H2)",
            release: "24H2",
            build: "26100.1742",
            architecture: "x64 (64-bit)",
            approx_size_gb: 5.4,
            official_url: "https://www.microsoft.com/software-download/windows11",
            description: "Official Microsoft Windows 11 Multi-Edition ISO (Home, Pro, Edu)",
        },
        WindowsIsoEntry {
            name: "Windows 11 (24H2 ARM64)",
            release: "24H2",
            build: "26100.1742",
            architecture: "ARM64",
            approx_size_gb: 5.1,
            official_url: "https://www.microsoft.com/software-download/windows11arm64",
            description: "Windows 11 for ARM64 Devices (Snapdragon X Elite, etc.)",
        },
        WindowsIsoEntry {
            name: "Windows 11 (23H2)",
            release: "23H2",
            build: "22631.2861",
            architecture: "x64 (64-bit)",
            approx_size_gb: 6.2,
            official_url: "https://www.microsoft.com/software-download/windows11",
            description: "Windows 11 2023 Update",
        },
        WindowsIsoEntry {
            name: "Windows 10 (22H2)",
            release: "22H2",
            build: "19045.3803",
            architecture: "x64 (64-bit)",
            approx_size_gb: 5.7,
            official_url: "https://www.microsoft.com/software-download/windows10",
            description: "Official Windows 10 Home/Pro 22H2 Update",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_entries() {
        let entries = get_official_windows_catalog();
        assert!(entries.len() >= 4);
        assert!(entries.iter().any(|e| e.name.contains("24H2")));
        assert!(entries.iter().all(|e| e.official_url.starts_with("https://")));
    }
}
