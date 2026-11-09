use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use usb_x_core::device::DeviceInfo;
use usb_x_core::event::{LogLevel, OperationEvent};
use usb_x_core::plan::{ImageInfo, OperationKind, OperationPlan};
use usb_x_core::types::{
    BootSelectionMode, ClusterSize, FileSystem, FlashTestKind, PartitionScheme,
    TargetSystem, Windows11Tweaks,
};
use usb_x_image::hash::ImageChecksums;
use usb_x_platform::{create_default_backend, StorageBackend, VirtualDriveBackend};

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

    pub is_windows_to_go: bool,
    pub show_advanced_drive_properties: bool,
    pub show_advanced_format_options: bool,
    pub list_usb_hard_drives: bool,
    pub old_bios_fixes: bool,
    pub use_rufus_mbr: bool,
    pub bad_blocks_passes: u32,

    pub win11_tweaks: Windows11Tweaks,
    pub linux_persistence_mb: u64,
    pub write_mode: usb_x_core::types::ImageWriteMode,

    pub backup_output_path: String,
    pub restore_input_path: String,
    pub flash_test_kind: FlashTestKind,
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

    // Dialogs
    pub show_confirm_dialog: bool,
    pub show_hash_dialog: bool,
    pub show_win11_tweaks_dialog: bool,
    pub show_log_window: bool,
    pub show_file_browser_dialog: bool,
    pub file_browser_current_dir: std::path::PathBuf,
    pub file_browser_selected_file: Option<std::path::PathBuf>,
    pub file_browser_error: Option<String>,
    pub file_browser_filter_all: bool,

    pub calculated_checksums: Option<ImageChecksums>,
    pub is_calculating_hash: bool,
    pub hash_progress_fraction: f32,
    pub hash_progress_text: String,
    pub hash_receiver: Option<Receiver<(f32, String, Option<ImageChecksums>)>>,
    pub expected_hash_input: String,
    pub typed_confirm_input: String,

    pub last_theme_check: std::time::Instant,
    pub cached_is_dark: bool,
    pub cached_accent: egui::Color32,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let backend = create_default_backend();
        let devices = backend.enumerate_devices().unwrap_or_default();

        let initial_is_dark = usb_x_platform::is_system_dark_mode();
        let initial_accent = crate::ui::style::get_system_accent();

        let win_ver = usb_x_platform::get_windows_version_string();
        let mut initial_logs = vec![
            format!("Pigeon x64 v1.0"),
            format!("OS: {}", win_ver),
            format!("Storage backend: Native Low-Level Block I/O"),
            format!("Found {} block storage device(s)", devices.len()),
        ];
        for dev in &devices {
            initial_logs.push(format!("  • Disk {}: {}", dev.device_path, dev.summary()));
        }
        initial_logs.push("Ready.".to_string());

        let mut app = Self {
            backend,
            devices,
            selected_device_idx: 0,

            boot_mode: BootSelectionMode::DiskImage,
            image_path: String::new(),
            image_info: None,

            partition_scheme: PartitionScheme::Gpt,
            target_system: TargetSystem::Uefi,
            file_system: FileSystem::Fat32,
            cluster_size: ClusterSize::Default,
            volume_label: "USB_X".to_string(),

            quick_format: true,
            extended_label: true,
            check_bad_blocks: false,
            verify_after_write: true,

            is_windows_to_go: false,
            show_advanced_drive_properties: false,
            show_advanced_format_options: false,
            list_usb_hard_drives: false,
            old_bios_fixes: false,
            use_rufus_mbr: false,
            bad_blocks_passes: 1,

            win11_tweaks: Windows11Tweaks::default(),
            linux_persistence_mb: 4096,
            write_mode: usb_x_core::types::ImageWriteMode::IsoImageMode,

            backup_output_path: "D:\\backup_drive.img".to_string(),
            restore_input_path: String::new(),
            flash_test_kind: FlashTestKind::DestructiveFakeCapacityTest,
            is_admin: usb_x_platform::is_elevated_admin(),

            is_running: false,
            progress_fraction: 0.0,
            completed_bytes: 0,
            total_bytes: None,
            speed_bps: None,
            eta_seconds: None,
            current_stage_text: "READY".to_string(),
            current_detail_text: "Ready to write or format".to_string(),
            elapsed_display: "00:00:00".to_string(),
            start_time: None,
            log_messages: initial_logs,

            event_sender: tx,
            event_receiver: rx,
            cancel_flag: Arc::new(AtomicBool::new(false)),

            show_confirm_dialog: false,
            show_hash_dialog: false,
            show_win11_tweaks_dialog: false,
            show_log_window: false,
            show_file_browser_dialog: false,
            file_browser_current_dir: std::path::PathBuf::from("/"),
            file_browser_selected_file: None,
            file_browser_error: None,
            file_browser_filter_all: false,

            calculated_checksums: None,
            is_calculating_hash: false,
            hash_progress_fraction: 0.0,
            hash_progress_text: String::new(),
            hash_receiver: None,
            expected_hash_input: String::new(),
            typed_confirm_input: String::new(),

            last_theme_check: std::time::Instant::now(),
            cached_is_dark: initial_is_dark,
            cached_accent: initial_accent,
        };

        // If devices exist, auto-select first safe removable device
        if let Some(idx) = app.devices.iter().position(|d| d.removable && !d.system_risk.is_destructive_blocked()) {
            app.selected_device_idx = idx;
        }

        app.log(LogLevel::Info, format!("Found {} block storage device(s)", app.devices.len()));
        app
    }

    pub fn refresh_theme_if_needed(&mut self, egui_hint: Option<bool>) {
        if self.last_theme_check.elapsed() >= std::time::Duration::from_millis(500) {
            self.cached_is_dark = egui_hint.unwrap_or_else(usb_x_platform::is_system_dark_mode);
            self.cached_accent = crate::ui::style::get_system_accent();
            self.last_theme_check = std::time::Instant::now();
        }
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
            matches!(
                info.detected_type,
                usb_x_core::types::ImageDetectedType::Windows11Installer { .. }
                    | usb_x_core::types::ImageDetectedType::Windows10Installer { .. }
                    | usb_x_core::types::ImageDetectedType::WindowsLegacy { .. }
            ) || info.path.to_ascii_lowercase().contains("win11")
                || info.path.to_ascii_lowercase().contains("windows 11")
                || info.path.to_ascii_lowercase().contains("win10")
                || info.path.to_ascii_lowercase().contains("windows 10")
                || info.path.to_ascii_lowercase().contains("windows")
        } else {
            let lower = self.image_path.to_ascii_lowercase();
            !lower.is_empty() && (
                lower.contains("win11")
                    || lower.contains("windows 11")
                    || lower.contains("win10")
                    || lower.contains("windows 10")
                    || lower.contains("windows")
            )
        }
    }

    pub fn refresh_devices(&mut self) {
        if self.is_running {
            return;
        }
        if let Ok(devs) = self.backend.enumerate_devices() {
            let summaries: Vec<String> = devs.iter()
                .map(|d| format!("  • {}: {}", d.device_path, d.summary()))
                .collect();
            self.devices = devs;
            if self.selected_device_idx >= self.devices.len() {
                self.selected_device_idx = 0;
            }
            self.log(LogLevel::Info, format!("Refreshed devices: {} available", self.devices.len()));
            for msg in summaries {
                self.log(LogLevel::Info, msg);
            }
        }
    }

    #[allow(dead_code)]
    pub fn add_virtual_drive(&mut self, path_str: &str, size_bytes: u64) {
        match VirtualDriveBackend::open_or_create(path_str, size_bytes) {
            Ok(vdisk) => {
                if let Ok(devs) = vdisk.enumerate_devices() {
                    for d in devs {
                        self.log(LogLevel::Info, format!("Mounted virtual drive: {} ({} bytes)", d.display_name, d.size_bytes));
                        self.devices.push(d);
                    }
                    if !self.devices.is_empty() {
                        self.selected_device_idx = self.devices.len() - 1;
                    }
                }
            }
            Err(e) => {
                self.log(LogLevel::Error, format!("Failed to create/mount virtual drive: {}", e));
            }
        }
    }

    pub fn selected_device(&self) -> Option<&DeviceInfo> {
        self.devices.get(self.selected_device_idx)
    }

    pub fn log(&mut self, level: LogLevel, msg: String) {
        let entry = format!("[{}] {}  {}", "Now", level.as_str(), msg);
        self.log_messages.push(entry);
    }

    pub fn update_events(&mut self) {
        if self.is_running {
            if let Some(st) = self.start_time {
                let secs = st.elapsed().as_secs();
                let h = secs / 3600;
                let m = (secs % 3600) / 60;
                let s = secs % 60;
                self.elapsed_display = format!("{:02}:{:02}:{:02}", h, m, s);
            }
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
            self.log(LogLevel::Info, format!("  SHA256: {}", s.sha256));
            self.log(LogLevel::Info, format!("  CRC32:  {}", s.crc32));
        }

        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                OperationEvent::StageChanged { stage, message } => {
                    self.current_stage_text = message.clone();
                    self.current_detail_text = format!("[{}] {}", stage.label(), message);
                    self.log(LogLevel::Info, format!("[{}] {}", stage.label(), message));
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
                            self.current_stage_text = "Writing".to_string();
                        }
                    }
                    if let Some(spd) = bytes_per_second {
                        self.current_detail_text = format!(
                            "{:.1} MB / {:.1} MB ({:.1} MB/s)",
                            completed_bytes as f64 / 1_048_576.0,
                            total_bytes.unwrap_or(completed_bytes) as f64 / 1_048_576.0,
                            spd as f64 / 1_048_576.0
                        );
                    }
                }
                OperationEvent::Log { level, message, .. } => {
                    if message.contains(":\\") && message.contains('(') {
                        self.current_detail_text = message.clone();
                    }
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
                            self.current_detail_text = format!("Operation completed successfully: {}", message);
                            self.log(LogLevel::Info, format!("SUCCESS: {}", message));
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
            self.log(LogLevel::Error, "Cannot write to physical flash drive without Administrator privileges! Please click 'Restart as Admin' or run Pigeon as Administrator.".to_string());
            self.current_stage_text = "FAILED: Elevation required".to_string();
            self.current_detail_text = "Writing directly to physical flash drives requires Administrator privileges.".to_string();
            return;
        }

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
                    verify_after_write: self.verify_after_write,
                    win11_tweaks: Some(self.win11_tweaks.clone()),
                    linux_persistence_mb: if self.linux_persistence_mb > 0 {
                        Some(self.linux_persistence_mb)
                    } else {
                        None
                    },
                    write_mode: self.write_mode,
                }
            }
            BootSelectionMode::FreeDos => OperationKind::CreateFreeDosMedia {
                partition_scheme: self.partition_scheme,
                file_system: self.file_system,
            },
            BootSelectionMode::MsDosUserFiles => OperationKind::CreateMsDosMedia {
                files_folder: self.image_path.clone(),
                partition_scheme: self.partition_scheme,
                file_system: self.file_system,
            },
            BootSelectionMode::FormatOnly => OperationKind::FormatOnly {
                partition_scheme: self.partition_scheme,
                file_system: self.file_system,
                cluster_size: self.cluster_size,
                volume_label: self.volume_label.clone(),
                quick_format: self.quick_format,
                extended_label: self.extended_label,
            },
            BootSelectionMode::BackupDrive => OperationKind::BackupDrive {
                output_file_path: self.backup_output_path.clone(),
                compress_zstd: false,
            },
            BootSelectionMode::RestoreDrive => OperationKind::RestoreDrive {
                backup_image_path: self.restore_input_path.clone(),
                verify: self.verify_after_write,
            },
            BootSelectionMode::FlashTest => OperationKind::FlashTest {
                kind: self.flash_test_kind,
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
        if self.is_calculating_hash || self.image_path.is_empty() {
            return;
        }
        self.is_calculating_hash = true;
        self.hash_progress_fraction = 0.0;
        self.hash_progress_text = "Reading image data...".to_string();
        self.calculated_checksums = None;

        let (tx, rx) = channel();
        self.hash_receiver = Some(rx);

        let path = self.image_path.clone();
        std::thread::spawn(move || {
            let res = usb_x_image::hash::calculate_checksums(&path, |cur, total| {
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
        if let Ok(info) = usb_x_image::probe_image(&self.image_path) {
            if let Some(ref lbl) = info.volume_label {
                if !lbl.trim().is_empty() {
                    self.volume_label = lbl.trim().to_string();
                }
            }

            let is_windows = matches!(
                info.detected_type,
                usb_x_core::types::ImageDetectedType::Windows11Installer { .. }
                | usb_x_core::types::ImageDetectedType::Windows10Installer { .. }
                | usb_x_core::types::ImageDetectedType::WindowsLegacy { .. }
            );

            let is_linux = info.is_isohybrid || info.has_grub || matches!(
                info.detected_type,
                usb_x_core::types::ImageDetectedType::LinuxHybridIso { .. }
            );

            if is_windows {
                // Automatically adjust for Windows
                self.write_mode = usb_x_core::types::ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
                self.file_system = usb_x_core::types::FileSystem::Ntfs;

                // Pop up Windows User Experience tweaks when user loads Windows 10 or above
                let chosen_lower = chosen.to_ascii_lowercase();
                let is_win10_plus = matches!(
                    info.detected_type,
                    usb_x_core::types::ImageDetectedType::Windows11Installer { .. }
                    | usb_x_core::types::ImageDetectedType::Windows10Installer { .. }
                ) || chosen_lower.contains("win11")
                  || chosen_lower.contains("windows 11")
                  || chosen_lower.contains("win10")
                  || chosen_lower.contains("windows 10");
                if is_win10_plus {
                    self.show_win11_tweaks_dialog = true;
                }
            } else if is_linux {
                // Automatically adjust for Linux: ISO Image mode (formats full drive as FAT32, extracts boot files, no unallocated space!)
                self.write_mode = usb_x_core::types::ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
                self.file_system = usb_x_core::types::FileSystem::Fat32;
            } else {
                self.write_mode = usb_x_core::types::ImageWriteMode::IsoImageMode;
                self.partition_scheme = PartitionScheme::Gpt;
                self.target_system = TargetSystem::Uefi;
            }

            self.image_info = Some(info);
        } else if let Ok(meta) = std::fs::metadata(&chosen) {
            self.image_info = Some(usb_x_core::plan::ImageInfo {
                path: chosen.clone(),
                file_size: meta.len(),
                volume_label: std::path::Path::new(&chosen)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string()),
                detected_type: usb_x_core::types::ImageDetectedType::GenericDiskImage,
                architecture: Some("x64".to_string()),
                has_efi_boot: true,
                has_bios_boot: true,
                sha256: None,
                secure_boot_status: usb_x_core::types::SecureBootStatus::NotApplicable,
                is_wim_split_needed: false,
                persistence_supported: false,
                is_isohybrid: true,
                has_grub: false,
            });
        }
        // Automatically calculate checksums in background
        self.start_hash_calculation();
    }

    pub fn open_file_browser(&mut self) {
        self.show_file_browser_dialog = true;
        self.file_browser_error = None;
        self.file_browser_selected_file = None;

        if !self.image_path.is_empty() {
            let p = std::path::Path::new(&self.image_path);
            if let Some(parent) = p.parent() {
                if parent.is_dir() {
                    self.file_browser_current_dir = parent.to_path_buf();
                    return;
                }
            }
        }

        #[cfg(windows)]
        let home = std::env::var("USERPROFILE").ok();
        #[cfg(not(windows))]
        let home = std::env::var("HOME").ok();

        if let Some(ref h) = home {
            let downloads = std::path::Path::new(h).join("Downloads");
            if downloads.is_dir() {
                self.file_browser_current_dir = downloads;
                return;
            }
            let desktop = std::path::Path::new(h).join("Desktop");
            if desktop.is_dir() {
                self.file_browser_current_dir = desktop;
                return;
            }
            let home_path = std::path::Path::new(h);
            if home_path.is_dir() {
                self.file_browser_current_dir = home_path.to_path_buf();
                return;
            }
        }

        self.file_browser_current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    }
}

