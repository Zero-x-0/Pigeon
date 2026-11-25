use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use usb_x_core::{BusType, DeviceId, DeviceInfo, SystemDiskRisk};
use usb_x_core::errors::{Result, UsbXError};
use crate::traits::StorageBackend;

pub struct LinuxStorageBackend {
    _devices: Vec<DeviceInfo>,
    active_handle: Option<(String, File)>,
    bytes_since_sync: u64,
}

impl LinuxStorageBackend {
    pub fn new() -> Self {
        Self {
            _devices: Vec::new(),
            active_handle: None,
            bytes_since_sync: 0,
        }
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

        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();

        if let Ok(entries) = std::fs::read_dir(sys_block) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip virtual, optical, swap, and non-flash block devices
                if name.starts_with("loop")
                    || name.starts_with("ram")
                    || name.starts_with("dm-")
                    || name.starts_with("zram")
                    || name.starts_with("sr")
                    || name.starts_with("md")
                    || name.starts_with("nbd")
                    || name.starts_with("fd")
                {
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

                // Removable flag & external bus detection
                let is_removable = std::fs::read_to_string(block_path.join("removable"))
                    .unwrap_or_default()
                    .trim() == "1";

                let sysfs_canon = std::fs::canonicalize(&block_path)
                    .or_else(|_| std::fs::canonicalize(block_path.join("device")))
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let is_usb_bus = sysfs_canon.contains("/usb");
                let is_external_sd = name.starts_with("mmcblk") && is_removable && sysfs_canon.contains("/mmc");

                // Only expose external USB drives and removable SD cards to flash
                if !is_usb_bus && !is_external_sd {
                    continue;
                }

                // Hardware Vendor and Model (Device Name only)
                let vendor = std::fs::read_to_string(block_path.join("device/vendor"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let model = std::fs::read_to_string(block_path.join("device/model"))
                    .unwrap_or_else(|_| std::fs::read_to_string(block_path.join("device/name")).unwrap_or_default())
                    .trim()
                    .to_string();

                let display_name = if !model.is_empty() {
                    if !vendor.is_empty() && !model.to_ascii_lowercase().starts_with(&vendor.to_ascii_lowercase()) {
                        format!("{} {}", vendor, model)
                    } else {
                        model.clone()
                    }
                } else if !vendor.is_empty() {
                    vendor.clone()
                } else {
                    format!("External Drive ({})", name)
                };

                // Detect partitions from sysfs
                let mut part_infos = Vec::new();
                if let Ok(sub_entries) = std::fs::read_dir(&block_path) {
                    let mut p_nums = Vec::new();
                    for sub in sub_entries.flatten() {
                        let sub_name = sub.file_name().to_string_lossy().to_string();
                        if sub_name.starts_with(&name) && sub_name.len() > name.len() {
                            if let Ok(part_num) = sub_name[name.len()..].trim_start_matches('p').parse::<u32>() {
                                p_nums.push(part_num);
                            }
                        }
                    }
                    p_nums.sort_unstable();
                    for pnum in p_nums {
                        part_infos.push(usb_x_core::PartitionInfo {
                            number: pnum,
                            mount_point: None,
                            size_bytes: 0,
                            file_system: None,
                            label: None,
                        });
                    }
                }

                // Check for system risk (root filesystem or boot partition)
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

                let bus_type = if is_usb_bus {
                    BusType::Usb
                } else {
                    BusType::SdCard
                };

                let risk = if is_root_or_boot {
                    SystemDiskRisk::SystemBootDrive
                } else {
                    SystemDiskRisk::SafeRemovable
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
                    bus_type,
                    system_risk: risk,
                    partitions: part_infos,
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
        let need_open = match &self.active_handle {
            Some((ref path, _)) => path != id.as_str(),
            None => true,
        };

        if need_open {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(id.as_str())
                .map_err(|e| UsbXError::Io {
                    operation: "open".into(),
                    message: format!("Failed to open device {}: {}", id.as_str(), e),
                })?;
            self.active_handle = Some((id.as_str().to_string(), file));
        }

        let file = &mut self.active_handle.as_mut().unwrap().1;

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

        // Synchronize data to physical flash media every 8 MiB so kernel buffers
        // stay in step with flash writes without causing excessive FTL erase stalls.
        self.bytes_since_sync += data.len() as u64;
        if self.bytes_since_sync >= 8 * 1024 * 1024 {
            let _ = file.sync_data();
            self.bytes_since_sync = 0;
        }

        Ok(data.len())
    }

    fn read_sectors(&mut self, id: &DeviceId, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let need_open = match &self.active_handle {
            Some((ref path, _)) => path != id.as_str(),
            None => true,
        };

        if need_open {
            let file = OpenOptions::new()
                .read(true)
                .open(id.as_str())
                .map_err(|e| UsbXError::Io {
                    operation: "open".into(),
                    message: format!("Failed to open device {}: {}", id.as_str(), e),
                })?;
            self.active_handle = Some((id.as_str().to_string(), file));
        }

        let file = &mut self.active_handle.as_mut().unwrap().1;

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
        if let Some((ref path, ref mut file)) = self.active_handle {
            if path == id.as_str() {
                let _ = file.sync_all();
            }
        }
        self.bytes_since_sync = 0;
        self.active_handle = None;

        // Synchronize kernel buffers to disk controllers
        let _ = system_cmd("sync").status();

        // Attempt device-level flush if accessible; ignore EBUSY if partition was recently unmounted
        match OpenOptions::new().write(true).open(id.as_str()) {
            Ok(file) => {
                let _ = file.sync_all();
            }
            Err(_) => {
                // System sync already committed data; non-fatal if block device busy
            }
        }

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
        let p = PathBuf::from(&appimage);
        if p.exists() {
            p
        } else if let Ok(cur) = std::env::current_exe() {
            cur
        } else {
            return false;
        }
    } else if let Ok(cur) = std::env::current_exe() {
        cur
    } else {
        return false;
    };

    let exe_str = exe.to_string_lossy().to_string();
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
    let xauth = std::env::var("XAUTHORITY").unwrap_or_else(|_| {
        if let Ok(home) = std::env::var("HOME") {
            let p = Path::new(&home).join(".Xauthority");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
        String::new()
    });
    let current_user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default();
    let initial_dark = is_system_dark_mode();
    let dbus_addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_default();

    // Authorize local root to connect to the X11 / Xwayland display server.
    // Without this, Xwayland rejects elevated root processes with "No protocol specified".
    let _ = std::process::Command::new("xhost")
        .arg("+si:localuser:root")
        .output();

    // In-place replacement via execvp so process is NOT orphaned ("dead parent" error in pkexec)
    use std::ffi::CString;
    let pkexec_path = find_system_binary("pkexec");
    if Path::new(&pkexec_path).exists() {
        if let (Ok(c_pk), Ok(c_env), Ok(c_disp), Ok(c_backend), Ok(c_dark), Ok(c_exe)) = (
            CString::new(pkexec_path.clone()),
            CString::new("env"),
            CString::new(format!("DISPLAY={}", display)),
            // Wayland compositors (like GNOME Mutter) explicitly isolate users and reject root
            // connections by design. Enforcing WINIT_UNIX_BACKEND=x11 allows the elevated window
            // to render reliably via Xwayland / X11 without crashing on WaylandError(NoCompositor).
            CString::new("WINIT_UNIX_BACKEND=x11"),
            CString::new(format!("PIGEON_SYSTEM_DARK={}", if initial_dark { "1" } else { "0" })),
            CString::new(exe_str.clone()),
        ) {
            let c_xauth = if !xauth.is_empty() {
                CString::new(format!("XAUTHORITY={}", xauth)).ok()
            } else {
                None
            };
            let c_appimage = std::env::var("APPIMAGE")
                .ok()
                .and_then(|ai| CString::new(format!("APPIMAGE={}", ai)).ok());
            let c_sudo_user = if !current_user.is_empty() {
                CString::new(format!("SUDO_USER={}", current_user)).ok()
            } else {
                None
            };
            let c_dbus = if !dbus_addr.is_empty() {
                CString::new(format!("DBUS_SESSION_BUS_ADDRESS={}", dbus_addr)).ok()
            } else {
                None
            };

            let c_cli_args: Vec<CString> = std::env::args()
                .skip(1)
                .filter_map(|arg| CString::new(arg).ok())
                .collect();

            let mut args: Vec<*const libc::c_char> = vec![
                c_pk.as_ptr(),
                c_env.as_ptr(),
                c_disp.as_ptr(),
                c_backend.as_ptr(),
                c_dark.as_ptr(),
            ];
            if let Some(ref xa) = c_xauth {
                args.push(xa.as_ptr());
            }
            if let Some(ref ai) = c_appimage {
                args.push(ai.as_ptr());
            }
            if let Some(ref su) = c_sudo_user {
                args.push(su.as_ptr());
            }
            if let Some(ref db) = c_dbus {
                args.push(db.as_ptr());
            }
            args.push(c_exe.as_ptr());
            for arg in &c_cli_args {
                args.push(arg.as_ptr());
            }
            args.push(std::ptr::null());

            unsafe {
                libc::execvp(c_pk.as_ptr(), args.as_ptr());
            }
        }
    }

    // Fallback: try sudo via execvp
    let sudo_path = find_system_binary("sudo");
    if Path::new(&sudo_path).exists() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        std::env::remove_var("WAYLAND_DISPLAY");
        if !current_user.is_empty() {
            std::env::set_var("SUDO_USER", &current_user);
        }
        let c_cli_args: Vec<CString> = std::env::args()
            .skip(1)
            .filter_map(|arg| CString::new(arg).ok())
            .collect();
        if let (Ok(c_sudo), Ok(c_flag), Ok(c_exe)) = (
            CString::new(sudo_path),
            CString::new("-E"),
            CString::new(exe_str),
        ) {
            let mut args: Vec<*const libc::c_char> = vec![
                c_sudo.as_ptr(),
                c_flag.as_ptr(),
                c_exe.as_ptr(),
            ];
            for arg in &c_cli_args {
                args.push(arg.as_ptr());
            }
            args.push(std::ptr::null());
            unsafe {
                libc::execvp(c_sudo.as_ptr(), args.as_ptr());
            }
        }
    }

    false
}

fn resolve_desktop_user() -> Option<(String, String)> {
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        let clean = sudo_user.trim().to_string();
        if !clean.is_empty() && clean != "root" {
            let uid = std::process::Command::new("id")
                .args(&["-u", &clean])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())?;
            return Some((clean, uid));
        }
    }
    if let Ok(pk_uid) = std::env::var("PKEXEC_UID") {
        let clean_uid = pk_uid.trim().to_string();
        if !clean_uid.is_empty() && clean_uid != "0" {
            let user = std::process::Command::new("id")
                .args(&["-nu", &clean_uid])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())?;
            return Some((user, clean_uid));
        }
    }
    None
}

fn run_desktop_theme_cmd(bin: &str, args: &[&str]) -> Option<String> {
    let bin_path = find_system_binary(bin);
    if !Path::new(&bin_path).exists() {
        return None;
    }

    // When running as root via pkexec or sudo, query the desktop user's session D-Bus
    if is_elevated_admin() {
        if let Some((user, uid)) = resolve_desktop_user() {
            let dbus_env = format!("DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/{}/bus", uid);
            let runtime_env = format!("XDG_RUNTIME_DIR=/run/user/{}", uid);
            let mut cmd = system_cmd("runuser");
            cmd.args(&["-u", &user, "--", "env", &dbus_env, &runtime_env, &bin_path])
                .args(args);
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    return Some(String::from_utf8_lossy(&output.stdout).to_ascii_lowercase());
                }
            }
        }
    }

    system_cmd(&bin_path)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase())
}

pub(crate) fn parse_portal_color_scheme(out: &str) -> Option<bool> {
    // org.freedesktop.portal.Settings returns e.g. "(<<uint32 1>>,)"
    let lower = out.to_ascii_lowercase();
    if lower.contains("uint32 1") {
        Some(true)
    } else if lower.contains("uint32 2") {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn parse_gnome_color_scheme(out: &str) -> Option<bool> {
    let lower = out.to_ascii_lowercase();
    if lower.contains("prefer-dark") {
        Some(true)
    } else if lower.contains("prefer-light") || lower.contains("default") {
        Some(false)
    } else {
        None
    }
}

pub fn is_system_dark_mode() -> bool {
    // 1. FreeDesktop XDG Portal (works across GNOME, KDE Plasma 5/6, COSMIC, XFCE, Hyprland, Sway)
    let portal_out = run_desktop_theme_cmd(
        "gdbus",
        &[
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.Read",
            "org.freedesktop.appearance",
            "color-scheme",
        ],
    );
    if let Some(ref s) = portal_out {
        if let Some(pref) = parse_portal_color_scheme(s) {
            return pref;
        }
    }

    // 2. GNOME GSettings color-scheme ('prefer-dark' vs 'default' / 'prefer-light')
    // Note: Mutter/GTK4 titlebars strictly follow GNOME color-scheme ('default' = Light mode on GNOME 42+).
    if let Some(ref s) = run_desktop_theme_cmd("gsettings", &["get", "org.gnome.desktop.interface", "color-scheme"]) {
        if let Some(pref) = parse_gnome_color_scheme(s) {
            return pref;
        }
    }

    // 3. GNOME GTK theme fallback (older GNOME versions without color-scheme)
    let gtk_theme = run_desktop_theme_cmd("gsettings", &["get", "org.gnome.desktop.interface", "gtk-theme"]);
    if let Some(ref s) = gtk_theme {
        return s.contains("dark");
    }

    // 4. XFCE via xfconf-query
    if let Some(s) = run_desktop_theme_cmd("xfconf-query", &["-c", "xsettings", "-p", "/Net/ThemeName"]) {
        return s.contains("dark");
    }

    // 5. KDE Plasma theme
    for kcmd in &["kreadconfig6", "kreadconfig5"] {
        if let Some(s) = run_desktop_theme_cmd(kcmd, &["--group", "General", "--key", "ColorScheme"]) {
            return s.contains("dark");
        }
    }

    // If Portal replied with uint32 0 (no preference / light) and none of the desktop commands found dark, use Light
    if portal_out.is_some() {
        return false;
    }

    // 6. Check GTK config files in active desktop user's home directory
    let user_home = resolve_desktop_user()
        .map(|(u, _)| format!("/home/{}", u))
        .or_else(|| std::env::var("HOME").ok());
    if let Some(home) = user_home {
        for sub in &[".config/gtk-4.0/settings.ini", ".config/gtk-3.0/settings.ini"] {
            if let Ok(ini) = std::fs::read_to_string(Path::new(&home).join(sub)) {
                let lower = ini.to_ascii_lowercase();
                if lower.contains("gtk-application-prefer-dark-theme=1")
                    || lower.contains("gtk-application-prefer-dark-theme=true")
                    || lower.contains("dark")
                {
                    return true;
                }
            }
        }
    }

    // 7. Startup fallback env vars (only when live DE session queries were unavailable)
    if let Ok(val) = std::env::var("PIGEON_SYSTEM_DARK") {
        if val == "1" {
            return true;
        } else if val == "0" {
            return false;
        }
    }
    for var in &["GTK_THEME", "COLOR_SCHEME"] {
        if let Ok(val) = std::env::var(var) {
            if val.to_ascii_lowercase().contains("dark") {
                return true;
            }
        }
    }

    // 8. Default: Kali Linux default is dark mode
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
    create_partition_on_disk_ex(dev_node, is_gpt, is_fat32, false, 0)
}

pub fn create_partition_on_disk_ex(
    dev_node: &str,
    is_gpt: bool,
    is_fat32: bool,
    old_bios_fixes: bool,
    total_size_bytes: u64,
) -> Result<String> {
    // 1. Forcefully unmount all existing partitions on this device
    let _ = unmount_device_all(dev_node);

    // 2. Wipe old filesystem/partition signatures so parted doesn't prompt or abort
    let _ = system_cmd("wipefs").arg("-a").arg(dev_node).status();

    // 3. Try parted first
    let label = if is_gpt { "gpt" } else { "msdos" };
    let fs_type = if is_fat32 { "fat32" } else { "ntfs" };
    let add_dummy_bios_part = old_bios_fixes && !is_gpt && total_size_bytes > 64 * 1024 * 1024;
    let total_mb = (total_size_bytes / (1024 * 1024)).max(64);
    let end_arg = if add_dummy_bios_part {
        format!("{}MiB", total_mb.saturating_sub(2))
    } else {
        "100%".to_string()
    };

    let mut success = false;
    let parted_status = system_cmd("parted")
        .arg("-s")
        .arg(dev_node)
        .arg("mklabel")
        .arg(label)
        .status();

    if let Ok(st) = parted_status {
        if st.success() {
            let mut parted_cmd = system_cmd("parted");
            parted_cmd.args(&["-s", dev_node, "mkpart", "primary", fs_type, "1MiB", &end_arg]);
            if add_dummy_bios_part {
                parted_cmd.args(&["mkpart", "primary", "fat16", &end_arg, "100%"]);
            }
            let part_status = parted_cmd.status();

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

    let part_dev = get_partition_device(dev_node, 1);
    for _ in 0..20 {
        if Path::new(&part_dev).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(part_dev)
}

/// Create Rufus-compatible Dual-Partition Windows Boot Layout (`NTFS + UEFI:NTFS`):
/// - Partition 1 (`Main Data Partition`): NTFS data partition holding the Windows setup payload ONCE
/// - Partition 2 (`UEFI:NTFS`): 1 MiB FAT12 partition holding Rufus's `uefi-ntfs.img` UEFI NTFS driver/chainloader
///   - GPT: Microsoft Basic Data GUID (`msftdata on`) + no-drive-letter attr (NOT ESP, preventing Windows dual-ESP setup errors)
///   - MBR: Partition type `0xef` (EFI) with Partition 1 marked `boot` active (`0x07`)
pub fn create_windows_dual_partition_on_disk(
    dev_node: &str,
    is_gpt: bool,
    total_size_bytes: u64,
    _uefi_ntfs_mb: u64,
) -> Result<(String, String)> {
    let _ = unmount_device_all(dev_node);
    let _ = system_cmd("wipefs").arg("-a").arg(dev_node).status();

    let total_mb = (total_size_bytes / (1024 * 1024)).max(2048);
    // Reserve 1 MiB at end for GPT backup header/alignment and 1 MiB (2048 sectors) for `uefi-ntfs.img`
    let uefi_start_mb = total_mb.saturating_sub(2).max(512);
    let uefi_end_mb = uefi_start_mb + 1;
    let uefi_start_arg = format!("{}MiB", uefi_start_mb);
    let uefi_end_arg = format!("{}MiB", uefi_end_mb);

    let mut success = false;
    let parted_res = if is_gpt {
        system_cmd("parted")
            .args(&[
                "-s",
                dev_node,
                "mklabel",
                "gpt",
                "mkpart",
                "Main Data Partition",
                "ntfs",
                "1MiB",
                &uefi_start_arg,
                "set",
                "1",
                "msftdata",
                "on",
                "mkpart",
                "UEFI:NTFS",
                "fat16",
                &uefi_start_arg,
                &uefi_end_arg,
                "set",
                "2",
                "msftdata",
                "on",
            ])
            .status()
    } else {
        system_cmd("parted")
            .args(&[
                "-s",
                dev_node,
                "mklabel",
                "msdos",
                "mkpart",
                "primary",
                "ntfs",
                "1MiB",
                &uefi_start_arg,
                "set",
                "1",
                "boot",
                "on",
                "mkpart",
                "primary",
                "fat16",
                &uefi_start_arg,
                &uefi_end_arg,
            ])
            .status()
    };

    if let Ok(st) = parted_res {
        if st.success() {
            if is_gpt {
                // Prevent automatic Windows drive letter assignment on UEFI:NTFS partition (bit 63)
                let _ = system_cmd("sfdisk")
                    .args(&["--part-attrs", dev_node, "2", "GUID:63"])
                    .status();
            } else {
                // Set MBR Partition 2 type to 0xef (matching Rufus XP_UEFI_NTFS)
                let _ = system_cmd("sfdisk")
                    .args(&["--part-type", dev_node, "2", "ef"])
                    .status();
            }
            success = true;
        }
    }

    if !success {
        let data_sectors = uefi_start_mb.saturating_sub(1) * 2048;
        let uefi_start_sector = uefi_start_mb * 2048;
        let sfdisk_input = if is_gpt {
            format!(
                "label: gpt\nstart=2048, size={}, type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7, name=\"Main Data Partition\"\nstart={}, size=2048, type=EBD0A0A2-B9E5-4433-87C0-68B6B72699C7, attrs=\"GUID:63\", name=\"UEFI:NTFS\"\n",
                data_sectors, uefi_start_sector
            )
        } else {
            format!(
                "label: dos\nstart=2048, size={}, type=7, bootable\nstart={}, size=2048, type=ef\n",
                data_sectors, uefi_start_sector
            )
        };

        if let Ok(mut child) = system_cmd("sfdisk")
            .arg(dev_node)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(sfdisk_input.as_bytes());
            }
            if let Ok(st) = child.wait() {
                success = st.success();
            }
        }
    }

    if !success {
        return Err(UsbXError::Io {
            operation: "create_windows_dual_partition".into(),
            message: format!("Failed to partition {} for Windows UEFI:NTFS layout", dev_node),
        });
    }

    let _ = system_cmd("partprobe").arg(dev_node).status();
    let _ = system_cmd("blockdev").arg("--rereadpt").arg(dev_node).status();
    let _ = system_cmd("udevadm").arg("settle").status();

    let part1 = get_partition_device(dev_node, 1);
    let part2 = get_partition_device(dev_node, 2);

    for _ in 0..25 {
        if Path::new(&part1).exists() && Path::new(&part2).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    if !Path::new(&part1).exists() || !Path::new(&part2).exists() {
        return Err(UsbXError::Io {
            operation: "wait_for_partitions".into(),
            message: format!("Created partitions {} and {} did not appear in /dev", part1, part2),
        });
    }

    Ok((part1, part2))
}

/// Write Rufus `uefi-ntfs.img` directly onto Partition 2 (1 MiB raw FAT12 partition).
pub fn write_uefi_ntfs_partition(part2_dev: &str, uefi_ntfs_img: &[u8]) -> Result<()> {
    let _ = unmount_device_all(part2_dev);
    let _ = system_cmd("udevadm").arg("settle").status();

    let mut file = OpenOptions::new()
        .write(true)
        .open(part2_dev)
        .map_err(|e| UsbXError::Io {
            operation: "open_uefi_ntfs_partition".into(),
            message: format!("{}: {}", part2_dev, e),
        })?;

    file.write_all(uefi_ntfs_img).map_err(|e| UsbXError::Io {
        operation: "write_uefi_ntfs_partition".into(),
        message: format!("{}: {}", part2_dev, e),
    })?;

    file.sync_all().map_err(|e| UsbXError::Io {
        operation: "sync_uefi_ntfs_partition".into(),
        message: format!("{}: {}", part2_dev, e),
    })?;
    drop(file);

    let _ = system_cmd("blockdev").arg("--flushbufs").arg(part2_dev).status();
    Ok(())
}

/// Partition disk into Partition 1 (Boot/ISO) + Partition 2 (ext4 Persistence `casper-rw` / `persistence`)
pub fn create_partition_with_persistence_on_disk(
    dev_node: &str,
    is_gpt: bool,
    is_fat32: bool,
    total_size_bytes: u64,
    persist_mb: u64,
) -> Result<(String, String)> {
    let _ = unmount_device_all(dev_node);
    let _ = system_cmd("wipefs").arg("-a").arg(dev_node).status();

    let total_mb = (total_size_bytes / (1024 * 1024)).max(2048);
    let persist = persist_mb.clamp(256, total_mb.saturating_sub(1536).max(256));
    let split_mb = total_mb.saturating_sub(persist).max(1024);
    let split_arg = format!("{}MiB", split_mb);
    let p1_fs = if is_fat32 { "fat32" } else { "ntfs" };

    let mut success = false;
    let parted_res = if is_gpt {
        system_cmd("parted")
            .args(&[
                "-s",
                dev_node,
                "mklabel",
                "gpt",
                "mkpart",
                "LIVE_USB",
                p1_fs,
                "1MiB",
                &split_arg,
                "set",
                "1",
                "msftdata",
                "on",
                "mkpart",
                "persistence",
                "ext4",
                &split_arg,
                "100%",
            ])
            .status()
    } else {
        system_cmd("parted")
            .args(&[
                "-s",
                dev_node,
                "mklabel",
                "msdos",
                "mkpart",
                "primary",
                p1_fs,
                "1MiB",
                &split_arg,
                "set",
                "1",
                "boot",
                "on",
                "mkpart",
                "primary",
                "ext4",
                &split_arg,
                "100%",
            ])
            .status()
    };

    if let Ok(st) = parted_res {
        if st.success() {
            success = true;
        }
    }

    if !success {
        return Err(UsbXError::Io {
            operation: "create_persistence_partition".into(),
            message: format!("Failed to partition {} with persistence partition", dev_node),
        });
    }

    let _ = system_cmd("partprobe").arg(dev_node).status();
    let _ = system_cmd("blockdev").arg("--rereadpt").arg(dev_node).status();
    let _ = system_cmd("udevadm").arg("settle").status();

    let part1 = get_partition_device(dev_node, 1);
    let part2 = get_partition_device(dev_node, 2);

    for _ in 0..25 {
        if Path::new(&part1).exists() && Path::new(&part2).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    if !Path::new(&part1).exists() || !Path::new(&part2).exists() {
        return Err(UsbXError::Io {
            operation: "wait_for_persistence_partitions".into(),
            message: format!("Partitions {} and {} did not appear in /dev", part1, part2),
        });
    }

    Ok((part1, part2))
}

/// Format Partition 2 as ext4 (`casper-rw` or `persistence`) and write `/persistence.conf`
pub fn setup_linux_persistence_partition(part2: &str, label: &str) -> Result<()> {
    let _ = unmount_device_all(part2);
    let _ = system_cmd("udevadm").arg("settle").status();

    let clean_lbl = if label.is_empty() { "casper-rw" } else { label };
    let st = system_cmd("mkfs.ext4")
        .args(&["-F", "-L", clean_lbl, part2])
        .status();

    if !st.map(|s| s.success()).unwrap_or(false) {
        return Err(UsbXError::Io {
            operation: "format_persistence_ext4".into(),
            message: format!("mkfs.ext4 failed on {}", part2),
        });
    }

    let guard = mount_volume(part2)?;
    let _ = std::fs::write(guard.mount_point.join("persistence.conf"), "/ union\n");
    let _ = system_cmd("sync").arg("-f").arg(&guard.mount_point).status();
    drop(guard);
    Ok(())
}

/// Preallocate contiguous blocks for a target file on Linux to avoid NTFS/FAT32 fragmentation.
pub fn preallocate_file(file: &File, size: u64) {
    if size >= 64 * 1024 {
        use std::os::unix::io::AsRawFd;
        unsafe {
            let _ = libc::posix_fallocate(file.as_raw_fd(), 0, size as libc::off_t);
        }
    }
}

/// Hint the kernel to optimize read-ahead for sequential ISO file reading.
pub fn advise_sequential_read(file: &File) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let _ = libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL);
    }
}

/// Kick off non-blocking kernel writeback for a written byte range.
pub fn trigger_async_writeback(file: &File, offset: u64, len: u64) {
    use std::os::unix::io::AsRawFd;
    if len > 0 {
        unsafe {
            let _ = libc::sync_file_range(
                file.as_raw_fd(),
                offset as libc::off64_t,
                len as libc::off64_t,
                libc::SYNC_FILE_RANGE_WRITE,
            );
        }
    }
}

/// Flush written dirty pages synchronously and drop page cache for finished regions.
pub fn sync_and_drop_range(file: &File, offset: u64, len: u64) {
    use std::os::unix::io::AsRawFd;
    if len > 0 {
        unsafe {
            let fd = file.as_raw_fd();
            let _ = libc::sync_file_range(
                fd,
                offset as libc::off64_t,
                len as libc::off64_t,
                libc::SYNC_FILE_RANGE_WAIT_BEFORE
                    | libc::SYNC_FILE_RANGE_WRITE
                    | libc::SYNC_FILE_RANGE_WAIT_AFTER,
            );
            let _ = libc::posix_fadvise(fd, offset as libc::off64_t, len as libc::off64_t, libc::POSIX_FADV_DONTNEED);
        }
    }
}

/// Install Legacy BIOS GRUB chainloader (`ntldr /bootmgr`) onto MBR so Legacy BIOS / CSM boots Windows Setup
pub fn install_legacy_bios_windows_grub(
    dev_node: &str,
    esp_mount_point: &Path,
    win_part_dev: Option<&str>,
    bios_id: Option<u8>,
) -> Result<()> {
    let grub_bin = find_system_binary("grub-install");
    if !Path::new(&grub_bin).exists() || !Path::new("/usr/lib/grub/i386-pc").exists() {
        return Ok(());
    }

    let boot_dir = esp_mount_point.join("boot");
    let grub_dir = boot_dir.join("grub");
    let _ = std::fs::create_dir_all(&grub_dir);

    let extra_drivemap = match bios_id {
        Some(0x81) => "\n    insmod drivemap\n    drivemap -s (hd0) (hd1)",
        Some(0x82) => "\n    insmod drivemap\n    drivemap -s (hd0) (hd2)",
        _ => "",
    };

    let part_uuid = win_part_dev.and_then(|p| {
        system_cmd("blkid")
            .args(&["-o", "value", "-s", "UUID", p])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    });

    let search_block = if let Some(uuid) = part_uuid {
        format!(
            "    insmod search_fs_uuid\n    search --no-floppy --fs-uuid --set=root {uuid}\n    ntldr ($root)/bootmgr"
        )
    } else {
        "    if search --no-floppy --file --set=root /sources/install.wim ; then\n        ntldr ($root)/bootmgr\n    elif search --no-floppy --file --set=root /sources/install.esd ; then\n        ntldr ($root)/bootmgr\n    else\n        search --no-floppy --file --set=root /bootmgr\n        ntldr ($root)/bootmgr\n    fi".to_string()
    };

    let grub_cfg = format!(
        r#"set timeout=0
set default=0

menuentry "Windows Setup (Legacy BIOS)" {{
    insmod part_msdos
    insmod part_gpt
    insmod ntfs
    insmod fat
    insmod search_fs_file
    insmod ntldr{extra_drivemap}
{search_block}
}}
"#
    );
    let _ = std::fs::write(grub_dir.join("grub.cfg"), grub_cfg);

    let status = system_cmd("grub-install")
        .args(&[
            "--target=i386-pc",
            &format!("--boot-directory={}", boot_dir.display()),
            "--force",
            dev_node,
        ])
        .status();

    match status {
        Ok(st) if st.success() => Ok(()),
        _ => Err(UsbXError::Io {
            operation: "install_legacy_bios_grub".into(),
            message: format!("grub-install i386-pc could not complete on {}", dev_node),
        }),
    }
}

pub fn format_partition_volume(part_dev: &str, fs_str: &str, label: &str) -> Result<()> {
    format_partition_volume_with_options(part_dev, fs_str, label, None, true)
}

pub fn format_partition_volume_with_options(
    part_dev: &str,
    fs_str: &str,
    label: &str,
    cluster_size_bytes: Option<u32>,
    quick_format: bool,
) -> Result<()> {
    let _ = system_cmd("udevadm").arg("settle").status();
    let _ = unmount_device_all(part_dev);

    let fs_upper = fs_str.to_ascii_uppercase();
    let clean_label = if label.is_empty() {
        "USB_BOOT".to_string()
    } else if fs_upper == "FAT32" || fs_upper == "VFAT" {
        let fat_clean: String = label
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .take(11)
            .collect();
        if fat_clean.is_empty() {
            "USB_BOOT".to_string()
        } else {
            fat_clean.to_uppercase()
        }
    } else {
        label.chars().take(32).collect()
    };

    let cluster_arg = cluster_size_bytes.map(|b| b.to_string());
    let fat_spc_arg = cluster_size_bytes.map(|b| (b / 512).clamp(1, 128).to_string());

    let run_mkfs = || -> std::io::Result<std::process::ExitStatus> {
        match fs_upper.as_str() {
            "NTFS" => {
                let mut cmd = system_cmd("mkfs.ntfs");
                cmd.arg("-F");
                if quick_format {
                    cmd.arg("-f");
                }
                if let Some(ref c) = cluster_arg {
                    cmd.arg("-c").arg(c);
                }
                cmd.arg("-L").arg(&clean_label).arg(part_dev);
                cmd.status().or_else(|_| {
                    let mut fallback = system_cmd("mkfs.ntfs");
                    if quick_format {
                        fallback.arg("-Q");
                    }
                    fallback.arg("-L").arg(&clean_label).arg(part_dev).status()
                })
            }
            "EXFAT" => {
                let mut cmd = system_cmd("mkfs.exfat");
                if let Some(ref c) = cluster_arg {
                    cmd.arg("-c").arg(c);
                }
                cmd.arg("-n").arg(&clean_label).arg(part_dev).status()
            }
            _ => {
                let mut cmd = system_cmd("mkfs.vfat");
                cmd.arg("-I").arg("-F").arg("32");
                if !quick_format {
                    cmd.arg("-c");
                }
                if let Some(ref spc) = fat_spc_arg {
                    cmd.arg("-s").arg(spc);
                }
                cmd.arg("-n").arg(&clean_label).arg(part_dev).status()
            }
        }
    };

    let mut status = run_mkfs();
    if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
        let _ = unmount_device_all(part_dev);
        let _ = system_cmd("udevadm").arg("settle").status();
        std::thread::sleep(std::time::Duration::from_millis(600));
        status = run_mkfs();
    }

    match status {
        Ok(st) if st.success() => Ok(()),
        _ => Err(UsbXError::Io {
            operation: "format_partition".into(),
            message: format!("Failed to format partition {} as {}", part_dev, fs_str),
        }),
    }
}

pub fn mount_volume(part_dev: &str) -> Result<LinuxMountGuard> {
    mount_volume_fs(part_dev, None)
}

pub fn mount_volume_fs(part_dev: &str, fs_hint: Option<&str>) -> Result<LinuxMountGuard> {
    let dev_tag = Path::new(part_dev)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "part".to_string());
    let temp_dir = std::env::temp_dir().join(format!("usb_x_target_{}_{}", std::process::id(), dev_tag));
    let _ = std::fs::create_dir_all(&temp_dir);

    for _ in 0..15 {
        if Path::new(part_dev).exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    // Unmount any desktop automount (e.g. udisks2 in /run/media) before binding our target mount
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == part_dev {
            let _ = system_cmd("umount").arg("-l").arg(parts[1]).status();
        }
    }

    let prefer_ntfs3 = fs_hint.map(|s| s.eq_ignore_ascii_case("NTFS")).unwrap_or(false);
    let mut status = if prefer_ntfs3 {
        // Kernel ntfs3 with prealloc is dramatically faster and avoids fragmentation vs FUSE ntfs-3g
        let s = system_cmd("mount")
            .args(&["-t", "ntfs3", "-o", "prealloc", part_dev])
            .arg(&temp_dir)
            .status();
        if s.as_ref().map(|x| x.success()).unwrap_or(false) {
            s
        } else {
            system_cmd("mount")
                .args(&["-t", "ntfs3", part_dev])
                .arg(&temp_dir)
                .status()
        }
    } else {
        system_cmd("mount").arg(part_dev).arg(&temp_dir).status()
    };

    if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
        status = system_cmd("mount")
            .args(&["-t", "ntfs3", "-o", "prealloc", part_dev])
            .arg(&temp_dir)
            .status()
            .or_else(|_| system_cmd("mount").args(&["-t", "ntfs-3g", "-o", "big_writes", part_dev]).arg(&temp_dir).status())
            .or_else(|_| system_cmd("mount").arg(part_dev).arg(&temp_dir).status());
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

    // 1. Try explicit UDF loop mount first (Windows 10/11 ISOs are UDF bridge discs with >4GB files)
    let udf_status = system_cmd("mount")
        .arg("-t")
        .arg("udf")
        .arg("-o")
        .arg("loop,ro")
        .arg(iso_path)
        .arg(&temp_dir)
        .status();

    if let Ok(st) = udf_status {
        if st.success() {
            return Ok(LinuxMountGuard::new(temp_dir));
        }
    }

    // 2. Try native auto-detected kernel loop mount (Linux ISO9660 / Hybrid ISOs)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_devices_only_and_name_size_summary() {
        let backend = LinuxStorageBackend::new();
        let devs = backend.enumerate_devices().unwrap();
        for d in &devs {
            assert!(
                matches!(d.bus_type, BusType::Usb | BusType::SdCard),
                "Only external USB/SD devices should be enumerated, got {:?} ({})",
                d.bus_type,
                d.device_path
            );
            assert!(
                !d.summary().contains("/dev/"),
                "Device summary '{}' must show device name and size only",
                d.summary()
            );
            assert!(d.summary().ends_with(']'));
        }
    }

    #[test]
    fn test_desktop_color_scheme_parsing_dark_and_light() {
        assert_eq!(parse_portal_color_scheme("(<<uint32 1>>,)"), Some(true));
        assert_eq!(parse_portal_color_scheme("(<<uint32 2>>,)"), Some(false));
        assert_eq!(parse_portal_color_scheme("(<<uint32 0>>,)"), None);

        assert_eq!(parse_gnome_color_scheme("'prefer-dark'\n"), Some(true));
        assert_eq!(parse_gnome_color_scheme("'default'\n"), Some(false));
        assert_eq!(parse_gnome_color_scheme("'prefer-light'\n"), Some(false));
    }
}
