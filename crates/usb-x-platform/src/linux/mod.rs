use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use usb_x_core::{BusType, DeviceId, DeviceInfo, SystemDiskRisk};
use usb_x_core::errors::{Result, UsbXError};
use crate::traits::StorageBackend;

pub struct LinuxStorageBackend;

impl LinuxStorageBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinuxStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for LinuxStorageBackend {
    fn enumerate_devices(&self) -> Result<Vec<DeviceInfo>> {
        let mut devices = Vec::new();
        let sys_block = Path::new("/sys/block");
        if !sys_block.exists() {
            return Ok(devices);
        }

        if let Ok(entries) = std::fs::read_dir(sys_block) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip loop devices, ram disks, and device mapper
                if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
                    continue;
                }

                let block_path = entry.path();
                let dev_node = format!("/dev/{}", name);

                // Size in 512-byte sectors
                let size_sectors: u64 = std::fs::read_to_string(block_path.join("size"))
                    .unwrap_or_default()
                    .trim()
                    .parse()
                    .unwrap_or(0);

                if size_sectors == 0 {
                    continue;
                }
                let size_bytes = size_sectors.saturating_mul(512);

                // Removable flag
                let is_removable = std::fs::read_to_string(block_path.join("removable"))
                    .unwrap_or_default()
                    .trim() == "1";

                // Model and Vendor
                let vendor = std::fs::read_to_string(block_path.join("device/vendor"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let model = std::fs::read_to_string(block_path.join("device/model"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let display_name = if !model.is_empty() {
                    if !vendor.is_empty() {
                        format!("{} {}", vendor, model)
                    } else {
                        model.clone()
                    }
                } else {
                    format!("USB Drive ({})", name)
                };

                // Check for system risk (root filesystem or boot partition)
                let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
                let is_root_or_boot = mounts.lines().any(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mount_dev = parts[0];
                        let mount_point = parts[1];
                        mount_dev.starts_with(&dev_node) && (mount_point == "/" || mount_point == "/boot" || mount_point == "/home")
                    } else {
                        false
                    }
                });

                let risk = if is_root_or_boot {
                    SystemDiskRisk::SystemBootDrive
                } else if is_removable {
                    SystemDiskRisk::SafeRemovable
                } else {
                    SystemDiskRisk::InternalFixedDrive
                };

                devices.push(DeviceInfo {
                    id: DeviceId::new(dev_node.clone()),
                    device_path: dev_node,
                    display_name,
                    vendor,
                    model,
                    size_bytes,
                    logical_sector_size: 512,
                    removable: is_removable,
                    bus_type: BusType::Usb,
                    system_risk: risk,
                    partitions: Vec::new(),
                    serial_redacted: None,
                });
            }
        }

        Ok(devices)
    }

    fn revalidate_device(&self, id: &DeviceId) -> Result<DeviceInfo> {
        let devs = self.enumerate_devices()?;
        devs.into_iter()
            .find(|d| d.id == *id)
            .ok_or_else(|| UsbXError::DeviceNotFound(id.as_str().to_string()))
    }

    fn unmount_and_lock(&self, id: &DeviceId) -> Result<()> {
        unmount_device_all(id.as_str())
    }

    fn write_sectors(&mut self, id: &DeviceId, offset: u64, data: &[u8]) -> Result<usize> {
        let mut file = OpenOptions::new()
            .write(true)
            .open(id.as_str())
            .map_err(|e| UsbXError::Io {
                operation: "open".into(),
                message: format!("Failed to open device {}: {}", id.as_str(), e),
            })?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| UsbXError::Io {
                operation: "seek".into(),
                message: format!("Seek failed: {}", e),
            })?;

        file.write_all(data)
            .map_err(|e| UsbXError::Io {
                operation: "write".into(),
                message: format!("Write failed: {}", e),
            })?;

        Ok(data.len())
    }

    fn read_sectors(&mut self, id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let mut file = File::open(id.as_str())
            .map_err(|e| UsbXError::Io {
                operation: "open".into(),
                message: format!("Failed to open device {}: {}", id.as_str(), e),
            })?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| UsbXError::Io {
                operation: "seek".into(),
                message: format!("Seek failed: {}", e),
            })?;

        let bytes = file.read(buf)
            .map_err(|e| UsbXError::Io {
                operation: "read".into(),
                message: format!("Read failed: {}", e),
            })?;

        Ok(bytes)
    }

    fn flush(&mut self, id: &DeviceId) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .open(id.as_str())
            .map_err(|e| UsbXError::Io {
                operation: "open".into(),
                message: format!("Failed to open device {}: {}", id.as_str(), e),
            })?;

        file.sync_all()
            .map_err(|e| UsbXError::Io {
                operation: "sync".into(),
                message: format!("Sync failed: {}", e),
            })?;

        Ok(())
    }

    fn eject(&self, id: &DeviceId) -> Result<()> {
        let _ = std::process::Command::new("eject")
            .arg(id.as_str())
            .status();
        Ok(())
    }
}

use std::path::PathBuf;

pub fn find_system_binary(name: &str) -> String {
    for dir in &[
        "/usr/local/sbin",
        "/usr/local/bin",
        "/usr/sbin",
        "/usr/bin",
        "/sbin",
        "/bin",
    ] {
        let p = Path::new(dir).join(name);
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    name.to_string()
}

pub fn system_cmd(name: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(find_system_binary(name));
    cmd.env(
        "PATH",
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games",
    );
    cmd
}

pub fn is_elevated_admin() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn relaunch_as_admin() -> bool {
    let exe = if let Ok(appimage) = std::env::var("APPIMAGE") {
        PathBuf::from(appimage)
    } else if let Ok(cur) = std::env::current_exe() {
        cur
    } else {
        return false;
    };

    let exe_str = exe.to_string_lossy().to_string();
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let xauth = std::env::var("XAUTHORITY").unwrap_or_default();
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();

    // In-place replacement via execvp so process is NOT orphaned ("dead parent" error in pkexec)
    use std::ffi::CString;
    let pkexec_path = find_system_binary("pkexec");
    if Path::new(&pkexec_path).exists() {
        if let (Ok(c_pk), Ok(c_env), Ok(c_disp), Ok(c_exe)) = (
            CString::new(pkexec_path.clone()),
            CString::new("env"),
            CString::new(format!("DISPLAY={}", display)),
            CString::new(exe_str.clone()),
        ) {
            let c_xauth = CString::new(format!("XAUTHORITY={}", xauth)).ok();
            let c_wayland = CString::new(format!("WAYLAND_DISPLAY={}", wayland)).ok();

            let mut args: Vec<*const libc::c_char> = vec![
                c_pk.as_ptr(),
                c_env.as_ptr(),
                c_disp.as_ptr(),
            ];
            if let Some(ref xa) = c_xauth {
                if !xauth.is_empty() {
                    args.push(xa.as_ptr());
                }
            }
            if let Some(ref wl) = c_wayland {
                if !wayland.is_empty() {
                    args.push(wl.as_ptr());
                }
            }
            args.push(c_exe.as_ptr());
            args.push(std::ptr::null());

            unsafe {
                libc::execvp(c_pk.as_ptr(), args.as_ptr());
            }
        }
    }

    // Fallback: try sudo via execvp
    let sudo_path = find_system_binary("sudo");
    if Path::new(&sudo_path).exists() {
        if let (Ok(c_sudo), Ok(c_flag), Ok(c_exe)) = (
            CString::new(sudo_path),
            CString::new("-E"),
            CString::new(exe_str),
        ) {
            let args: Vec<*const libc::c_char> = vec![
                c_sudo.as_ptr(),
                c_flag.as_ptr(),
                c_exe.as_ptr(),
                std::ptr::null(),
            ];
            unsafe {
                libc::execvp(c_sudo.as_ptr(), args.as_ptr());
            }
        }
    }

    false
}

pub fn is_system_dark_mode() -> bool {
    // 1. Check org.freedesktop.appearance color-scheme via gsettings
    if let Ok(output) = system_cmd("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if s.contains("dark") {
            return true;
        }
    }

    // 2. Check GTK theme name via gsettings
    if let Ok(output) = system_cmd("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if s.contains("dark") {
            return true;
        }
    }

    // 3. Check XFCE (standard on Kali Linux!) via xfconf-query
    if let Ok(output) = system_cmd("xfconf-query")
        .args(&["-c", "xsettings", "-p", "/Net/ThemeName"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if s.contains("dark") {
            return true;
        }
    }

    // 4. Check KDE Plasma theme
    for kcmd in &["kreadconfig6", "kreadconfig5"] {
        if let Ok(output) = system_cmd(kcmd)
            .args(&["--group", "General", "--key", "ColorScheme"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
            if s.contains("dark") {
                return true;
            }
        }
    }

    // 5. Check environment variables
    for var in &["GTK_THEME", "COLOR_SCHEME"] {
        if let Ok(val) = std::env::var(var) {
            if val.to_ascii_lowercase().contains("dark") {
                return true;
            }
        }
    }

    // 6. Default: Kali Linux default is dark mode
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        if content.to_ascii_lowercase().contains("kali") {
            return true;
        }
    }

    false
}

pub fn get_system_accent_color() -> Option<(u8, u8, u8)> {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let lower = content.to_ascii_lowercase();
        if lower.contains("kali") {
            return Some((54, 123, 240)); // Kali Electric Blue
        } else if lower.contains("ubuntu") {
            return Some((233, 84, 32)); // Ubuntu Orange
        } else if lower.contains("fedora") {
            return Some((41, 65, 165)); // Fedora Blue
        } else if lower.contains("arch") {
            return Some((23, 147, 209)); // Arch Cyan
        }
    }
    Some((0, 120, 215))
}

pub fn get_system_version_string() -> String {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return rest.trim_matches('"').to_string();
            }
        }
    }
    "Linux (x86_64)".to_string()
}

pub struct LinuxMountGuard {
    pub mount_point: PathBuf,
}

impl LinuxMountGuard {
    pub fn new(mount_point: PathBuf) -> Self {
        Self { mount_point }
    }
}

impl Drop for LinuxMountGuard {
    fn drop(&mut self) {
        let _ = system_cmd("sync").status();
        let _ = system_cmd("umount").arg("-l").arg(&self.mount_point).status();
        let _ = system_cmd("umount").arg("-f").arg(&self.mount_point).status();
        let _ = std::fs::remove_dir(&self.mount_point);
    }
}

pub fn unmount_device_all(dev_node: &str) -> Result<()> {
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0].starts_with(dev_node) {
            let _ = system_cmd("umount").arg("-l").arg(parts[1]).status();
            let _ = system_cmd("umount").arg("-f").arg(parts[1]).status();
        }
    }
    let _ = system_cmd("umount").arg("-l").arg(dev_node).status();
    Ok(())
}

pub fn get_partition_device(dev_node: &str, part_num: u32) -> String {
    let clean = dev_node.trim_end_matches('/');
    if clean.chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("{}p{}", clean, part_num)
    } else {
        format!("{}{}", clean, part_num)
    }
}

pub fn create_partition_on_disk(dev_node: &str, is_gpt: bool, is_fat32: bool) -> Result<String> {
    // 1. Forcefully unmount all existing partitions on this device
    let _ = unmount_device_all(dev_node);

    // 2. Wipe old filesystem/partition signatures so parted doesn't prompt or abort
    let _ = system_cmd("wipefs").arg("-a").arg(dev_node).status();

    // 3. Try parted first
    let label = if is_gpt { "gpt" } else { "msdos" };
    let fs_type = if is_fat32 { "fat32" } else { "ntfs" };

    let mut success = false;
    let parted_status = system_cmd("parted")
        .arg("-s")
        .arg(dev_node)
        .arg("mklabel")
        .arg(label)
        .status();

    if let Ok(st) = parted_status {
        if st.success() {
            let part_status = system_cmd("parted")
                .arg("-s")
                .arg(dev_node)
                .arg("mkpart")
                .arg("primary")
                .arg(fs_type)
                .arg("1MiB")
                .arg("100%")
                .status();

            if let Ok(pst) = part_status {
                if pst.success() {
                    let flag = if is_gpt { "msftdata" } else { "boot" };
                    let _ = system_cmd("parted")
                        .arg("-s")
                        .arg(dev_node)
                        .arg("set")
                        .arg("1")
                        .arg(flag)
                        .arg("on")
                        .status();
                    success = true;
                }
            }
        }
    }

    // 4. Fallback to sfdisk if parted was not available or failed
    if !success {
        let sfdisk_input = if is_gpt {
            "label: gpt\n,,\n"
        } else {
            "label: dos\n,,7,*\n"
        };

        if let Ok(mut child) = system_cmd("sfdisk")
            .arg(dev_node)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(sfdisk_input.as_bytes());
            }
            let _ = child.wait();
        }
    }

    // 5. Notify kernel to reread partition table
    let _ = system_cmd("partprobe").arg(dev_node).status();
    let _ = system_cmd("blockdev").arg("--rereadpt").arg(dev_node).status();
    let _ = system_cmd("udevadm").arg("settle").status();
    std::thread::sleep(std::time::Duration::from_millis(600));

    let part_dev = get_partition_device(dev_node, 1);
    Ok(part_dev)
}

pub fn format_partition_volume(part_dev: &str, fs_str: &str, label: &str) -> Result<()> {
    let clean_label = if label.is_empty() { "USB_BOOT" } else { label };
    let status = match fs_str.to_ascii_uppercase().as_str() {
        "NTFS" => {
            // Pass -F (force) and -f (fast) so mkfs.ntfs never fails on existing filesystems
            system_cmd("mkfs.ntfs")
                .arg("-F")
                .arg("-f")
                .arg("-L")
                .arg(clean_label)
                .arg(part_dev)
                .status()
                .or_else(|_| {
                    system_cmd("mkfs.ntfs")
                        .arg("-Q")
                        .arg("-L")
                        .arg(clean_label)
                        .arg(part_dev)
                        .status()
                })
        }
        "EXFAT" => {
            system_cmd("mkfs.exfat")
                .arg("-n")
                .arg(clean_label)
                .arg(part_dev)
                .status()
        }
        _ => {
            // Pass -I (ignore existing filesystem) and -F 32 (FAT32)
            system_cmd("mkfs.vfat")
                .arg("-I")
                .arg("-F")
                .arg("32")
                .arg("-n")
                .arg(clean_label)
                .arg(part_dev)
                .status()
        }
    };

    match status {
        Ok(st) if st.success() => Ok(()),
        Ok(_) | Err(_) => {
            let fb = system_cmd("mkfs.vfat")
                .arg("-I")
                .arg("-F")
                .arg("32")
                .arg("-n")
                .arg(clean_label)
                .arg(part_dev)
                .status();
            if let Ok(fst) = fb {
                if fst.success() {
                    return Ok(());
                }
            }
            Err(UsbXError::Io {
                operation: "format_partition".into(),
                message: format!("Failed to format partition {} as {}", part_dev, fs_str),
            })
        }
    }
}

pub fn mount_volume(part_dev: &str) -> Result<LinuxMountGuard> {
    let temp_dir = std::env::temp_dir().join(format!("usb_x_target_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);

    for _ in 0..15 {
        if Path::new(part_dev).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let mut status = system_cmd("mount")
        .arg(part_dev)
        .arg(&temp_dir)
        .status();

    if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
        // Try with ntfs-3g / ntfs3 explicitly if NTFS
        status = system_cmd("mount")
            .arg("-t")
            .arg("ntfs-3g")
            .arg(part_dev)
            .arg(&temp_dir)
            .status()
            .or_else(|_| {
                system_cmd("mount")
                    .arg("-t")
                    .arg("ntfs3")
                    .arg(part_dev)
                    .arg(&temp_dir)
                    .status()
            });
    }

    match status {
        Ok(st) if st.success() => Ok(LinuxMountGuard::new(temp_dir)),
        _ => Err(UsbXError::Io {
            operation: "mount_target_volume".into(),
            message: format!("Mount command failed for partition {}. Ensure root privileges (sudo / pkexec) are granted.", part_dev),
        }),
    }
}

pub fn mount_iso(iso_path: &str) -> Result<LinuxMountGuard> {
    let temp_dir = std::env::temp_dir().join(format!("usb_x_iso_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);

    // 1. Try native kernel loop mount
    let status = system_cmd("mount")
        .arg("-o")
        .arg("loop,ro")
        .arg(iso_path)
        .arg(&temp_dir)
        .status();

    if let Ok(st) = status {
        if st.success() {
            return Ok(LinuxMountGuard::new(temp_dir));
        }
    }

    // 2. Fallback: Try 7z extraction (works without root/loop device drivers)
    let p7z = system_cmd("7z")
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", temp_dir.display()))
        .arg(iso_path)
        .status();

    if let Ok(st) = p7z {
        if st.success() {
            return Ok(LinuxMountGuard::new(temp_dir));
        }
    }

    // 3. Fallback: Try bsdtar extraction
    let tar = system_cmd("bsdtar")
        .arg("-xf")
        .arg(iso_path)
        .arg("-C")
        .arg(&temp_dir)
        .status();

    if let Ok(st) = tar {
        if st.success() {
            return Ok(LinuxMountGuard::new(temp_dir));
        }
    }

    Err(UsbXError::Io {
        operation: "mount_source_iso".into(),
        message: format!("Failed to mount source ISO '{}'. Loop mount and extraction fallbacks failed. Ensure root privileges (sudo/pkexec) are granted.", iso_path),
    })
}
