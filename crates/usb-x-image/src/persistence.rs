pub struct PersistenceConfig {
    pub label: &'static str,
    pub filesystem: &'static str,
    pub grub_kernel_arg: &'static str,
}

pub fn get_persistence_config_for_distro(distro_name: &str) -> PersistenceConfig {
    let lower = distro_name.to_lowercase();
    if lower.contains("ubuntu") || lower.contains("mint") || lower.contains("pop") {
        // Modern Ubuntu uses 'writable' partition label; older uses 'casper-rw'
        PersistenceConfig {
            label: "writable",
            filesystem: "ext4",
            grub_kernel_arg: "persistent",
        }
    } else if lower.contains("debian") || lower.contains("kali") {
        PersistenceConfig {
            label: "persistence",
            filesystem: "ext4",
            grub_kernel_arg: "persistence",
        }
    } else {
        PersistenceConfig {
            label: "persistence",
            filesystem: "ext4",
            grub_kernel_arg: "persistence",
        }
    }
}

