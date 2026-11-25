use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use usb_x_core::device::DeviceInfo;
use usb_x_core::event::{LogLevel, OperationEvent};
use usb_x_core::plan::{ImageInfo, OperationKind, OperationPlan};
use usb_x_core::types::{
    BootSelectionMode, ClusterSize, CompressedFormat, FileSystem, ImageWriteMode, PartitionScheme,
    TargetSystem, ThemePreference,
};
use usb_x_image::hash::ImageChecksums;
use usb_x_platform::{create_default_backend, StorageBackend};

pub struct AppState {
    pub backend: Box<dyn StorageBackend>,
    pub devices: Vec<DeviceInfo>,
    pub selected_device_idx: usize,

    pub boot_mode: BootSelectionMode,
    pub image_path: String,
    pub image_info: Option<ImageInfo>,

    pub partition_scheme: PartitionScheme,
    pub target_system: TargetSystem,
    pub file_system: FileSystem,
    pub cluster_size: ClusterSize,
    pub volume_label: String,

    pub quick_format: bool,
    pub extended_label: bool,
    pub check_bad_blocks: bool,
    pub verify_after_write: bool,

    pub show_advanced_drive_properties: bool,
    pub show_advanced_format_options: bool,
    pub list_usb_hard_drives: bool,
    pub old_bios_fixes: bool,
    pub use_rufus_mbr: bool,
    pub rufus_mbr_bios_id: u8,
    pub bad_blocks_passes: u32,

    pub linux_persistence_mb: u64,
    pub persist_unit_is_gb: bool,
    pub write_mode: ImageWriteMode,

    pub backup_output_path: String,
    pub is_admin: bool,

    // Execution state
    pub is_running: bool,
    pub progress_fraction: f32,
    pub completed_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub current_stage_text: String,
    pub current_detail_text: String,
    pub elapsed_display: String,
    pub start_time: Option<std::time::Instant>,
    pub log_messages: Vec<String>,

    pub event_sender: Sender<OperationEvent>,
    pub event_receiver: Receiver<OperationEvent>,
    pub cancel_flag: Arc<AtomicBool>,

    // Dialogs & Controls
    pub select_split_is_download: bool,
    pub show_confirm_dialog: bool,
    pub show_hash_dialog: bool,
    pub show_log_window: bool,
    pub show_download_dialog: bool,
    pub show_about_dialog: bool,
    pub show_settings_dialog: bool,
    pub show_file_browser_dialog: bool,
    pub file_browser_current_dir: std::path::PathBuf,
    pub file_browser_selected_file: Option<std::path::PathBuf>,
    pub file_browser_error: Option<String>,
    pub file_browser_filter_all: bool,
    pub file_browser_path_input: String,
    pub file_browser_editing_path: bool,
    pub file_browser_search: String,
    pub file_browser_history: Vec<std::path::PathBuf>,
    pub file_browser_history_idx: usize,

    // Download ISO Image dialog state
    pub dl_version: String,
    pub dl_release: String,
    pub dl_edition: String,
    pub dl_language: String,
    pub dl_arch: String,
    pub dl_use_browser: bool,
    pub dl_is_downloading: bool,
    pub dl_progress_fraction: f32,
    pub dl_progress_text: String,
    pub dl_receiver: Option<Receiver<(f32, String, Option<String>, Option<String>)>>,
    pub backup_path_receiver: Option<Receiver<String>>,

    pub calculated_checksums: Option<ImageChecksums>,
    pub is_calculating_hash: bool,
    pub include_sha512: bool,
    pub hash_progress_fraction: f32,
    pub hash_progress_text: String,
    pub hash_receiver: Option<Receiver<(f32, String, Option<ImageChecksums>)>>,
    pub typed_confirm_input: String,

    pub theme_preference: ThemePreference,
    pub last_theme_check: std::time::Instant,
    pub theme_probe_receiver: Option<Receiver<(bool, egui::Color32)>>,
    pub last_device_poll: std::time::Instant,
    pub cached_is_dark: bool,
    pub cached_accent: egui::Color32,
    pub status_bar_hint: Option<String>,
    pub app_language: String,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let backend = create_default_backend();
        let devices = backend.enumerate_devices().unwrap_or_default();

        let initial_is_dark = usb_x_platform::is_system_dark_mode();
        let initial_accent = crate::ui::style::get_system_accent();
        let os_ver = usb_x_platform::get_windows_version_string();

        let mut app = Self {
            backend,
            devices: devices.clone(),
            selected_device_idx: 0,

            boot_mode: BootSelectionMode::DiskImage,
            image_path: String::new(),
            image_info: None,

            partition_scheme: PartitionScheme::Gpt,
            target_system: TargetSystem::Uefi,
            file_system: FileSystem::Fat32,
            cluster_size: ClusterSize::Default,
            volume_label: "USB_BOOT".to_string(),

            quick_format: true,
            extended_label: true,
            check_bad_blocks: false,
            verify_after_write: true,

            show_advanced_drive_properties: false,
            show_advanced_format_options: false,
            list_usb_hard_drives: false,
            old_bios_fixes: false,
            use_rufus_mbr: false,
            rufus_mbr_bios_id: 0x80,
            bad_blocks_passes: 1,

            linux_persistence_mb: 0,
            persist_unit_is_gb: true,
            write_mode: ImageWriteMode::IsoImageMode,

            backup_output_path: "backup_drive.img".to_string(),
            is_admin: usb_x_platform::is_elevated_admin(),

            is_running: false,
            progress_fraction: 0.0,
            completed_bytes: 0,
            total_bytes: None,
            speed_bps: None,
            eta_seconds: None,
            current_stage_text: "READY".to_string(),
            current_detail_text: String::new(),
            elapsed_display: "00:00:00".to_string(),
            start_time: None,
            log_messages: Vec::new(),

            event_sender: tx,
            event_receiver: rx,
            cancel_flag: Arc::new(AtomicBool::new(false)),

            select_split_is_download: false,
            show_confirm_dialog: false,
            show_hash_dialog: false,
            show_log_window: false,
            show_download_dialog: false,
            show_about_dialog: false,
            show_settings_dialog: false,
            show_file_browser_dialog: false,
            file_browser_current_dir: std::path::PathBuf::from("/"),
            file_browser_selected_file: None,
            file_browser_error: None,
            file_browser_filter_all: false,
            file_browser_path_input: String::new(),
            file_browser_editing_path: false,
            file_browser_search: String::new(),
            file_browser_history: Vec::new(),
            file_browser_history_idx: 0,

            dl_version: "Windows 11".to_string(),
            dl_release: "24H2 (Build 26100.1742 - 2024.10)".to_string(),
            dl_edition: "Windows 11 Home/Pro/Edu".to_string(),
            dl_language: "English International".to_string(),
            dl_arch: "x64".to_string(),
            dl_use_browser: false,
            dl_is_downloading: false,
            dl_progress_fraction: 0.0,
            dl_progress_text: String::new(),
            dl_receiver: None,
            backup_path_receiver: None,

            calculated_checksums: None,
            is_calculating_hash: false,
            include_sha512: false,
            hash_progress_fraction: 0.0,
            hash_progress_text: String::new(),
            hash_receiver: None,
            typed_confirm_input: String::new(),

            theme_preference: ThemePreference::System,
            last_theme_check: std::time::Instant::now(),
            theme_probe_receiver: None,
            last_device_poll: std::time::Instant::now(),
            cached_is_dark: initial_is_dark,
            cached_accent: initial_accent,
            status_bar_hint: None,
            app_language: "English (US)".to_string(),
        };

        app.log_messages.push(format!(
            "Pigeon v{} ({})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::ARCH
        ));
        app.log_messages.push(format!("OS: {}", os_ver));
        app.log_messages.push(format!("Privileges: {}", if app.is_admin { "Root/Administrator" } else { "Standard User" }));

        if let Some((idx, _)) = app.visible_devices().first() {
            app.selected_device_idx = *idx;
        }
        let dev_lines: Vec<String> = app
            .visible_devices()
            .into_iter()
            .map(|(_, dev)| format!("  {} ({})", dev.summary(), dev.device_path))
            .collect();
        app.log_messages.push(format!(
            "Found {} device{}",
            dev_lines.len(),
            if dev_lines.len() == 1 { "" } else { "s" }
        ));
        app.log_messages.extend(dev_lines);
        app
    }

    pub fn refresh_theme_if_needed(&mut self, egui_hint: Option<bool>) {
        // Consume any completed background theme probe first
        if let Some(ref rx) = self.theme_probe_receiver {
            if let Ok((dark, accent)) = rx.try_recv() {
                if self.theme_preference == ThemePreference::System && egui_hint.is_none() {
                    self.cached_is_dark = dark;
                    self.cached_accent = accent;
                }
                self.theme_probe_receiver = None;
                self.last_theme_check = std::time::Instant::now();
            }
        }

        match self.theme_preference {
            ThemePreference::Light => self.cached_is_dark = false,
            ThemePreference::Dark => self.cached_is_dark = true,
            ThemePreference::System => {
                if let Some(hint) = egui_hint {
                    self.cached_is_dark = hint;
                } else if self.theme_probe_receiver.is_none()
                    && self.last_theme_check.elapsed() >= std::time::Duration::from_millis(650)
                {
                    let (tx, rx) = channel();
                    self.theme_probe_receiver = Some(rx);
                    self.last_theme_check = std::time::Instant::now();
                    std::thread::spawn(move || {
                        let dark = usb_x_platform::is_system_dark_mode();
                        let accent = crate::ui::style::get_system_accent();
                        let _ = tx.send((dark, accent));
                    });
                }
            }
        }
    }

    pub fn toggle_dark_light_shortcut(&mut self) {
        let next_dark = !self.cached_is_dark;
        self.theme_preference = if next_dark { ThemePreference::Dark } else { ThemePreference::Light };
        self.cached_is_dark = next_dark;
        self.log(
            LogLevel::Info,
            format!("Switched UI theme to {}", self.theme_preference.label()),
        );
    }

    pub fn is_dark(&self) -> bool {
        self.cached_is_dark
    }

    pub fn is_dark_mode_active(&self) -> bool {
        self.cached_is_dark
    }

    pub fn is_windows_image(&self) -> bool {
        if self.boot_mode != BootSelectionMode::DiskImage {
            return false;
        }
        if let Some(ref info) = self.image_info {
            info.is_windows()
        } else {
            false
        }
    }

    pub fn is_linux_image(&self) -> bool {
        if self.boot_mode != BootSelectionMode::DiskImage {
            return false;
        }
        self.image_info
            .as_ref()
            .map(|i| i.is_linux() || (i.is_isohybrid && !i.is_windows() && !i.is_raw_disk_image()))
            .unwrap_or(false)
    }

    pub fn is_dd_only_mode(&self) -> bool {
        if self.boot_mode != BootSelectionMode::DiskImage {
            return false;
        }
        let is_raw = self.image_info.as_ref().map(|i| i.is_raw_disk_image()).unwrap_or(false);
        is_raw || self.write_mode == ImageWriteMode::DdImageMode
    }

    pub fn visible_devices(&self) -> Vec<(usize, &DeviceInfo)> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                if d.system_risk.is_destructive_blocked() {
                    return false;
                }
                let is_external = matches!(
                    d.bus_type,
                    usb_x_core::types::BusType::Usb | usb_x_core::types::BusType::SdCard
                );
                if !is_external {
                    return false;
                }
                d.removable || (self.list_usb_hard_drives && d.bus_type == usb_x_core::types::BusType::Usb)
            })
            .collect()
    }

    pub fn refresh_devices(&mut self) {
        if self.is_running {
            return;
        }
        if let Ok(devs) = self.backend.enumerate_devices() {
            let summaries: Vec<String> = devs
                .iter()
                .map(|d| format!("  • {}: {}", d.device_path, d.summary()))
                .collect();
            self.devices = devs;
            if let Some((first_idx, _)) = self.visible_devices().first() {
                if !self.visible_devices().iter().any(|(idx, _)| *idx == self.selected_device_idx) {
                    self.selected_device_idx = *first_idx;
                }
            }
            self.log(
                LogLevel::Info,
                format!("Refreshed devices: {} visible", self.visible_devices().len()),
            );
            for msg in summaries {
                self.log(LogLevel::Info, msg);
            }
        }
    }

    pub fn poll_hotplug_if_needed(&mut self) {
        if self.is_running || self.last_device_poll.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_device_poll = std::time::Instant::now();
        if let Ok(devs) = self.backend.enumerate_devices() {
            let changed = devs.len() != self.devices.len()
                || devs
                    .iter()
                    .zip(self.devices.iter())
                    .any(|(a, b)| a.id != b.id || a.size_bytes != b.size_bytes);
            if changed {
                let prev_id = self.selected_device().map(|d| d.id.clone());
                self.devices = devs;
                if let Some(pid) = prev_id {
                    if let Some((new_idx, _)) = self.visible_devices().into_iter().find(|(_, d)| d.id == pid) {
                        self.selected_device_idx = new_idx;
                        return;
                    }
                }
                if let Some((first_idx, _)) = self.visible_devices().first() {
                    self.selected_device_idx = *first_idx;
                }
            }
        }
    }

    pub fn selected_device(&self) -> Option<&DeviceInfo> {
        let visible = self.visible_devices();
        visible
            .iter()
            .find(|(idx, _)| *idx == self.selected_device_idx)
            .or_else(|| visible.first())
            .map(|(_, d)| *d)
    }

    pub fn log(&mut self, level: LogLevel, msg: String) {
        let entry = match level {
            LogLevel::Error => format!("ERROR: {}", msg),
            LogLevel::Warn => format!("WARNING: {}", msg),
            _ => msg,
        };
        if self.log_messages.len() >= 10000 {
            self.log_messages.drain(..2000);
        }
        self.log_messages.push(entry);
    }

    pub fn update_events(&mut self) {
        self.poll_hotplug_if_needed();

        if self.is_running {
            if let Some(st) = self.start_time {
                let secs = st.elapsed().as_secs();
                self.elapsed_display = format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);
            }
        }

        // Check async backup save path selection
        let mut chosen_backup_path = None;
        if let Some(ref rx) = self.backup_path_receiver {
            if let Ok(path) = rx.try_recv() {
                chosen_backup_path = Some(path);
            }
        }
        if let Some(dest) = chosen_backup_path {
            self.backup_path_receiver = None;
            if !dest.is_empty() {
                self.backup_output_path = dest;
                let prev_mode = self.boot_mode;
                self.boot_mode = BootSelectionMode::BackupDrive;
                self.start_operation();
                self.boot_mode = prev_mode;
            }
        }

        // Check async ISO download progress
        let mut dl_done_path: Option<String> = None;
        let mut dl_err: Option<String> = None;
        if let Some(ref rx) = self.dl_receiver {
            while let Ok((frac, text, done_path, err_msg)) = rx.try_recv() {
                self.dl_progress_fraction = frac;
                self.dl_progress_text = text;
                if let Some(p) = done_path {
                    dl_done_path = Some(p);
                }
                if let Some(e) = err_msg {
                    dl_err = Some(e);
                }
            }
        }
        if let Some(path) = dl_done_path {
            self.dl_is_downloading = false;
            self.dl_receiver = None;
            self.show_download_dialog = false;
            self.select_split_is_download = false;
            self.log(LogLevel::Info, format!("Downloaded ISO image: {}", path));
            self.load_image_file(path);
        } else if let Some(err) = dl_err {
            self.dl_is_downloading = false;
            self.dl_receiver = None;
            self.dl_progress_text = format!("Download failed: {}", err);
            self.log(LogLevel::Error, format!("ISO download error: {}", err));
        }

        let mut completed_sums = None;
        if let Some(ref rx) = self.hash_receiver {
            while let Ok((frac, text, sums)) = rx.try_recv() {
                self.hash_progress_fraction = frac;
                self.hash_progress_text = text;
                if let Some(s) = sums {
                    completed_sums = Some(s);
                }
            }
        }
        if let Some(s) = completed_sums {
            self.calculated_checksums = Some(s.clone());
            self.is_calculating_hash = false;
            self.log(LogLevel::Info, "Image checksums computed:".to_string());
            self.log(LogLevel::Info, format!("  MD5:    {}", s.md5));
            self.log(LogLevel::Info, format!("  SHA1:   {}", s.sha1));
            self.log(LogLevel::Info, format!("  SHA256: {}", s.sha256));
            if let Some(ref s512) = s.sha512 {
                self.log(LogLevel::Info, format!("  SHA512: {}", s512));
            }
            self.log(LogLevel::Info, format!("  CRC32:  {}", s.crc32));
        }

        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                OperationEvent::StageChanged { message, .. } => {
                    self.current_stage_text = message.clone();
                    self.current_detail_text = message.clone();
                    self.log(LogLevel::Info, message);
                }
                OperationEvent::Progress {
                    completed_bytes,
                    total_bytes,
                    bytes_per_second,
                    eta_seconds,
                } => {
                    self.completed_bytes = completed_bytes;
                    self.total_bytes = total_bytes;
                    self.speed_bps = bytes_per_second;
                    self.eta_seconds = eta_seconds;

                    if let Some(total) = total_bytes {
                        if total > 0 {
                            self.progress_fraction = (completed_bytes as f32 / total as f32).clamp(0.0, 1.0);
                        }
                    }
                    if let Some(spd) = bytes_per_second {
                        let eta_part = match eta_seconds {
                            Some(eta) if eta > 0 => format!(" (ETA {:02}:{:02})", eta / 60, eta % 60),
                            _ => String::new(),
                        };
                        self.current_detail_text = if let Some(tot) = total_bytes {
                            format!(
                                "{:.1} MB / {:.1} MB ({:.1} MB/s){}",
                                completed_bytes as f64 / 1_048_576.0,
                                tot as f64 / 1_048_576.0,
                                spd as f64 / 1_048_576.0,
                                eta_part
                            )
                        } else {
                            format!(
                                "{:.1} MB written ({:.1} MB/s)",
                                completed_bytes as f64 / 1_048_576.0,
                                spd as f64 / 1_048_576.0
                            )
                        };
                    }
                }
                OperationEvent::Log { level, message, .. } => {
                    self.log(level, message);
                }
                OperationEvent::Warning { message } => {
                    self.log(LogLevel::Warn, message);
                }
                OperationEvent::Finished { result } => {
                    self.is_running = false;
                    self.progress_fraction = 1.0;
                    match result {
                        usb_x_core::event::OperationResult::Success { message } => {
                            self.current_stage_text = "READY".to_string();
                            self.current_detail_text = message.clone();
                            self.log(LogLevel::Info, format!("SUCCESS: {}", message));
                            self.refresh_devices();
                        }
                        usb_x_core::event::OperationResult::Cancelled => {
                            self.current_stage_text = "CANCELLED".to_string();
                            self.current_detail_text = "Operation cancelled by user".to_string();
                            self.log(LogLevel::Warn, "Operation was cancelled by user.".to_string());
                        }
                        usb_x_core::event::OperationResult::Failed { error } => {
                            self.current_stage_text = "FAILED".to_string();
                            self.current_detail_text = format!("Error: {}", error);
                            self.log(LogLevel::Error, format!("ERROR: {}", error));
                        }
                    }
                }
            }
        }
    }

    pub fn start_operation(&mut self) {
        let device = match self.selected_device() {
            Some(d) => d.clone(),
            None => {
                self.log(LogLevel::Error, "No device selected!".to_string());
                return;
            }
        };

        if device.system_risk.is_destructive_blocked() {
            self.log(LogLevel::Error, "Cannot perform operation on protected system disk!".to_string());
            return;
        }

        let is_virtual = device.device_path.starts_with("vdisk:") || device.bus_type == usb_x_core::types::BusType::VirtualFile;
        if !self.is_admin && !is_virtual {
            self.log(LogLevel::Error, "Root/Administrator privileges required to write physical drives.".to_string());
            self.current_stage_text = "FAILED: Elevation required".to_string();
            return;
        }

        let bad_blocks_opt = if self.check_bad_blocks {
            Some(self.bad_blocks_passes)
        } else {
            None
        };
        let mbr_bios_opt = if self.use_rufus_mbr && self.partition_scheme == PartitionScheme::Mbr {
            Some(self.rufus_mbr_bios_id)
        } else {
            None
        };

        let plan_kind = match self.boot_mode {
            BootSelectionMode::DiskImage => {
                let img = match &self.image_info {
                    Some(info) => info.clone(),
                    None => {
                        self.log(LogLevel::Error, "Please select an image file first!".to_string());
                        return;
                    }
                };
                OperationKind::WriteImage {
                    image: img,
                    partition_scheme: self.partition_scheme,
                    target_system: self.target_system,
                    file_system: self.file_system,
                    cluster_size: self.cluster_size,
                    volume_label: self.volume_label.clone(),
                    quick_format: self.quick_format,
                    extended_label: self.extended_label,
                    verify_after_write: self.verify_after_write,
                    check_bad_blocks_passes: bad_blocks_opt,
                    linux_persistence_mb: if self.linux_persistence_mb > 0 && self.is_linux_image() {
                        Some(self.linux_persistence_mb)
                    } else {
                        None
                    },
                    write_mode: self.write_mode,
                    old_bios_fixes: self.old_bios_fixes,
                    rufus_mbr_bios_id: mbr_bios_opt,
                }
            }
            BootSelectionMode::FormatOnly => OperationKind::FormatOnly {
                partition_scheme: self.partition_scheme,
                file_system: self.file_system,
                cluster_size: self.cluster_size,
                volume_label: self.volume_label.clone(),
                quick_format: self.quick_format,
                extended_label: self.extended_label,
                check_bad_blocks_passes: bad_blocks_opt,
                old_bios_fixes: self.old_bios_fixes,
                rufus_mbr_bios_id: mbr_bios_opt,
            },
            BootSelectionMode::BackupDrive => OperationKind::BackupDrive {
                output_file_path: self.backup_output_path.clone(),
            },
        };

        let plan = OperationPlan {
            target_device_id: device.id.clone(),
            target_device_summary: device.summary(),
            target_size_bytes: device.size_bytes,
            mode: self.boot_mode,
            kind: plan_kind,
        };

        self.is_running = true;
        self.start_time = Some(std::time::Instant::now());
        self.elapsed_display = "00:00:00".to_string();
        self.progress_fraction = 0.0;
        self.completed_bytes = 0;
        self.total_bytes = None;
        self.cancel_flag.store(false, Ordering::Relaxed);
        let cancel_clone = self.cancel_flag.clone();
        let tx = self.event_sender.clone();
        let backend = create_default_backend();

        std::thread::spawn(move || {
            usb_x_engine::execute_operation_pipeline(backend, plan, device, tx, cancel_clone);
        });
    }

    pub fn start_hash_calculation(&mut self) {
        self.start_hash_calculation_ex(self.include_sha512);
    }

    pub fn start_hash_calculation_ex(&mut self, include_sha512: bool) {
        if self.is_calculating_hash || self.image_path.is_empty() {
            return;
        }
        self.include_sha512 = include_sha512;
        self.is_calculating_hash = true;
        self.hash_progress_fraction = 0.0;
        self.hash_progress_text = "Reading image data...".to_string();
        self.calculated_checksums = None;

        let (tx, rx) = channel();
        self.hash_receiver = Some(rx);

        let path = self.image_path.clone();
        std::thread::spawn(move || {
            let res = usb_x_image::hash::calculate_checksums_with_sha512(&path, include_sha512, |cur, total| {
                let frac = if total > 0 { (cur as f32 / total as f32).clamp(0.0, 1.0) } else { 0.0 };
                let mb_cur = cur as f64 / 1_048_576.0;
                let mb_tot = total as f64 / 1_048_576.0;
                let text = format!("{:.0}% ({:.1} MB / {:.1} MB)", frac * 100.0, mb_cur, mb_tot);
                let _ = tx.send((frac, text, None));
            });

            if let Ok(sums) = res {
                let _ = tx.send((1.0, "Complete".to_string(), Some(sums)));
            } else {
                let _ = tx.send((0.0, "Failed to compute checksums".to_string(), None));
            }
        });
    }

    pub fn trigger_backup_drive(&mut self) {
        if self.is_running || self.selected_device().is_none() {
            return;
        }
        let (tx, rx) = channel();
        self.backup_path_receiver = Some(rx);
        std::thread::spawn(move || {
            if let Some(dest) = usb_x_platform::save_file_dialog(
                "Save Drive Image",
                "usb_drive_backup.img",
                "Disk Images (*.img)",
                "*.img",
            ) {
                let _ = tx.send(dest);
            }
        });
    }

    pub fn start_iso_download(&mut self, entry: &usb_x_catalog::WindowsIsoEntry) {
        if self.dl_is_downloading {
            return;
        }

        if self.dl_use_browser || !entry.direct_iso_download {
            let _ = webbrowser::open(entry.official_url);
            self.log(LogLevel::Info, format!("Opened official download page: {}", entry.official_url));
            self.show_download_dialog = false;
            return;
        }

        let url = entry.official_url.to_string();
        let filename = entry.suggested_filename.to_string();

        let home_dir = std::env::var("SUDO_USER")
            .ok()
            .filter(|u| !u.is_empty() && u != "root")
            .map(|u| std::path::PathBuf::from(format!("/home/{}", u)))
            .filter(|p| p.is_dir())
            .or_else(|| std::env::var("HOME").ok().map(std::path::PathBuf::from))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));

        let downloads_dir = if home_dir.join("Downloads").is_dir() {
            home_dir.join("Downloads")
        } else {
            home_dir
        };
        let dest_path = downloads_dir.join(&filename);
        let dest_str = dest_path.to_string_lossy().to_string();
        let approx_bytes = (entry.approx_size_gb as f64 * 1024.0 * 1024.0 * 1024.0) as u64;

        self.dl_is_downloading = true;
        self.dl_progress_fraction = 0.0;
        self.dl_progress_text = format!("Connecting to download {}...", filename);

        let (tx, rx) = channel();
        self.dl_receiver = Some(rx);

        std::thread::spawn(move || {
            let expected_bytes = std::process::Command::new("curl")
                .args(&["-sI", "-L", "--connect-timeout", "8", &url])
                .output()
                .ok()
                .and_then(|out| {
                    let hdr = String::from_utf8_lossy(&out.stdout);
                    hdr.lines()
                        .filter(|l| l.to_ascii_lowercase().starts_with("content-length:"))
                        .filter_map(|l| l.split(':').nth(1)?.trim().parse::<u64>().ok())
                        .last()
                })
                .filter(|&sz| sz > 0)
                .unwrap_or(approx_bytes.max(1));

            let child_res = std::process::Command::new("curl")
                .args(&["-L", "-f", "-s", "-S", "--connect-timeout", "15", "-o", &dest_str, &url])
                .stderr(std::process::Stdio::piped())
                .spawn();

            let mut child = match child_res {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send((0.0, "Failed".to_string(), None, Some(e.to_string())));
                    return;
                }
            };

            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        if status.success() {
                            let _ = tx.send((1.0, "Download complete".to_string(), Some(dest_str), None));
                        } else {
                            let mut err_buf = String::new();
                            if let Some(mut se) = child.stderr.take() {
                                use std::io::Read;
                                let _ = se.read_to_string(&mut err_buf);
                            }
                            let msg = if err_buf.trim().is_empty() {
                                "HTTP download failed".to_string()
                            } else {
                                err_buf.trim().to_string()
                            };
                            let _ = std::fs::remove_file(&dest_str);
                            let _ = tx.send((0.0, "Failed".to_string(), None, Some(msg)));
                        }
                        break;
                    }
                    Ok(None) => {
                        let cur_bytes = std::fs::metadata(&dest_str).map(|m| m.len()).unwrap_or(0);
                        let frac = (cur_bytes as f32 / expected_bytes as f32).clamp(0.02, 0.99);
                        let text = format!(
                            "{:.1}% ({:.1} MB / {:.1} MB)",
                            frac * 100.0,
                            cur_bytes as f64 / 1_048_576.0,
                            expected_bytes as f64 / 1_048_576.0
                        );
                        let _ = tx.send((frac, text, None, None));
                        std::thread::sleep(std::time::Duration::from_millis(250));
                    }
                    Err(e) => {
                        let _ = tx.send((0.0, "Failed".to_string(), None, Some(e.to_string())));
                        break;
                    }
                }
            }
        });
    }

    pub fn cancel_operation(&mut self) {
        if self.is_running {
            self.cancel_flag.store(true, Ordering::Relaxed);
            self.log(LogLevel::Warn, "Cancelling operation...".to_string());
        }
    }

    pub fn load_image_file(&mut self, chosen: String) {
        if chosen.is_empty() {
            return;
        }
        self.boot_mode = BootSelectionMode::DiskImage;
        self.image_path = chosen.clone();
        self.calculated_checksums = None;
        self.include_sha512 = false;
        if let Ok(info) = usb_x_image::probe_image(&self.image_path) {
            if let Some(ref lbl) = info.volume_label {
                if !lbl.trim().is_empty() {
                    self.volume_label = lbl.trim().to_string();
                }
            }

            if info.is_raw_disk_image() {
                self.write_mode = ImageWriteMode::DdImageMode;
            } else if info.is_windows() {
                self.write_mode = ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
                self.file_system = FileSystem::Ntfs;
            } else if info.is_linux() || info.is_isohybrid {
                self.write_mode = ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
                self.file_system = FileSystem::Fat32;
            } else {
                self.write_mode = ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
            }

            self.image_info = Some(info);
        } else if let Ok(meta) = std::fs::metadata(&chosen) {
            let is_comp = CompressedFormat::from_path(&chosen).is_some();
            self.write_mode = if is_comp {
                ImageWriteMode::DdImageMode
            } else {
                ImageWriteMode::IsoImageMode
            };
            self.image_info = Some(ImageInfo {
                path: chosen.clone(),
                file_size: meta.len(),
                volume_label: std::path::Path::new(&chosen)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string()),
                detected_type: usb_x_core::types::ImageDetectedType::GenericDiskImage,
                architecture: Some("x64".to_string()),
                has_efi_boot: true,
                has_bios_boot: true,
                is_wim_split_needed: false,
                persistence_supported: false,
                is_isohybrid: false,
                has_grub: false,
            });
        }
    }

    pub fn trigger_file_dialog(&mut self) {
        match usb_x_platform::open_file_dialog_ex(
            "Select Bootable ISO or Disk Image",
            "Disk Images (*.iso;*.img;*.bin;*.vhd;*.raw;*.gz;*.xz;*.bz2;*.zst)",
            "*.iso;*.img;*.bin;*.vhd;*.raw;*.gz;*.xz;*.bz2;*.zst",
        ) {
            usb_x_platform::DialogResult::Selected(path) => {
                self.load_image_file(path);
            }
            usb_x_platform::DialogResult::Cancelled => {}
            usb_x_platform::DialogResult::Unsupported => {
                self.open_file_browser();
            }
        }
    }

    pub fn set_file_browser_dir(&mut self, dir: std::path::PathBuf) {
        if dir.is_dir() {
            self.file_browser_current_dir = dir.clone();
            self.file_browser_path_input = dir.to_string_lossy().to_string();
            self.file_browser_selected_file = None;
            self.file_browser_error = None;

            if self.file_browser_history.is_empty() {
                self.file_browser_history.push(dir);
                self.file_browser_history_idx = 0;
            } else if self.file_browser_history[self.file_browser_history_idx] != dir {
                self.file_browser_history.truncate(self.file_browser_history_idx + 1);
                self.file_browser_history.push(dir);
                self.file_browser_history_idx = self.file_browser_history.len() - 1;
            }
        }
    }

    pub fn file_browser_can_go_back(&self) -> bool {
        self.file_browser_history_idx > 0
    }

    pub fn file_browser_go_back(&mut self) {
        if self.file_browser_can_go_back() {
            self.file_browser_history_idx -= 1;
            let dir = self.file_browser_history[self.file_browser_history_idx].clone();
            self.file_browser_current_dir = dir.clone();
            self.file_browser_path_input = dir.to_string_lossy().to_string();
            self.file_browser_selected_file = None;
            self.file_browser_error = None;
        }
    }

    pub fn file_browser_can_go_forward(&self) -> bool {
        self.file_browser_history_idx + 1 < self.file_browser_history.len()
    }

    pub fn file_browser_go_forward(&mut self) {
        if self.file_browser_can_go_forward() {
            self.file_browser_history_idx += 1;
            let dir = self.file_browser_history[self.file_browser_history_idx].clone();
            self.file_browser_current_dir = dir.clone();
            self.file_browser_path_input = dir.to_string_lossy().to_string();
            self.file_browser_selected_file = None;
            self.file_browser_error = None;
        }
    }

    pub fn file_browser_go_up(&mut self) {
        if let Some(parent) = self.file_browser_current_dir.parent() {
            let p = parent.to_path_buf();
            self.set_file_browser_dir(p);
        }
    }

    pub fn open_file_browser(&mut self) {
        self.show_file_browser_dialog = true;
        self.file_browser_error = None;
        self.file_browser_selected_file = None;
        self.file_browser_search.clear();
        self.file_browser_editing_path = false;

        if !self.image_path.is_empty() {
            let p = std::path::Path::new(&self.image_path);
            if let Some(parent) = p.parent() {
                if parent.is_dir() {
                    self.set_file_browser_dir(parent.to_path_buf());
                    return;
                }
            }
        }

        #[cfg(windows)]
        let home = std::env::var("USERPROFILE").ok();
        #[cfg(not(windows))]
        let home = std::env::var("SUDO_USER")
            .ok()
            .filter(|u| !u.is_empty() && u != "root")
            .map(|u| format!("/home/{}", u))
            .filter(|p| std::path::Path::new(p).is_dir())
            .or_else(|| std::env::var("HOME").ok());

        if let Some(ref h) = home {
            for sub in &["Downloads", "Desktop", ""] {
                let candidate = if sub.is_empty() {
                    std::path::PathBuf::from(h)
                } else {
                    std::path::Path::new(h).join(sub)
                };
                if candidate.is_dir() {
                    self.set_file_browser_dir(candidate);
                    return;
                }
            }
        }

        let cur = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        self.set_file_browser_dir(cur);
    }
}
