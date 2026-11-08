use egui::{Color32, CornerRadius, RichText, Stroke, Ui, Vec2};
use crate::state::AppState;
use crate::ui::style::{field_label, render_section_header, text_secondary, COLOR_DANGER_RED};
use usb_x_core::types::{BootSelectionMode, PartitionScheme, TargetSystem};

pub fn render_drive_properties(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.is_dark_mode_active();
    render_section_header(ui, "Drive Properties", is_dark);

    if !state.is_admin {
        let (banner_bg, banner_stroke, banner_text) = if is_dark {
            (Color32::from_rgb(50, 36, 10), Color32::from_rgb(170, 115, 0), Color32::from_rgb(255, 205, 100))
        } else {
            (Color32::from_rgb(255, 244, 206), Color32::from_rgb(255, 185, 0), Color32::from_rgb(130, 65, 0))
        };

        egui::Frame::new()
            .fill(banner_bg)
            .stroke(Stroke::new(1.0f32, banner_stroke))
            .corner_radius(CornerRadius::same(3))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🛡 Admin rights needed to write to flash drives.")
                            .color(banner_text)
                            .size(11.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Restart as Root / Admin ↗").strong().size(11.0)).clicked() {
                            if usb_x_platform::relaunch_as_admin() {
                                std::process::exit(0);
                            }
                        }
                    });
                });
            });
        ui.add_space(2.0);
    }

    // 1. Device
    ui.horizontal(|ui| {
        ui.label(field_label("Device", is_dark));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(!state.is_running, egui::Button::new("⟳")).on_hover_text("Scan for plugged in USB drives").clicked() {
                state.refresh_devices();
            }
        });
    });

    let selected_summary = state.selected_device()
        .map(|d| d.summary())
        .unwrap_or_else(|| "No removable devices detected".to_string());

    egui::ComboBox::from_id_salt("device_picker_combo")
        .width(ui.available_width() - 4.0)
        .selected_text(selected_summary)
        .show_ui(ui, |ui| {
            for (idx, dev) in state.devices.iter().enumerate() {
                let is_system = dev.system_risk.is_destructive_blocked();
                let text = if is_system {
                    format!("🔒 {} [PROTECTED SYSTEM DISK]", dev.summary())
                } else {
                    dev.summary()
                };
                ui.selectable_value(&mut state.selected_device_idx, idx, text);
            }
        });

    if let Some(dev) = state.selected_device() {
        if dev.system_risk.is_destructive_blocked() {
            ui.label(
                RichText::new("⚠ PROTECTED HOST DISK: Contains the running operating system. Destructive actions disabled.")
                    .color(COLOR_DANGER_RED)
                    .small()
                    .strong(),
            );
        }
    }

    ui.add_space(4.0);

    // 2. Boot selection
    ui.label(field_label("Boot selection", is_dark));
    ui.horizontal(|ui| {
        let boot_display_text = match state.boot_mode {
            BootSelectionMode::DiskImage => {
                if !state.image_path.is_empty() {
                    std::path::Path::new(&state.image_path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| state.image_path.clone())
                } else {
                    "Disk or ISO image (Please select)".to_string()
                }
            }
            _ => state.boot_mode.label().to_string(),
        };

        let combo_width = (ui.available_width() - 86.0).max(140.0);
        egui::ComboBox::from_id_salt("boot_selection_mode_combo")
            .width(combo_width)
            .selected_text(boot_display_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::DiskImage, "Disk or ISO image");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::FreeDos, "FreeDOS");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::MsDosUserFiles, "MS-DOS");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::FormatOnly, "Non-bootable");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::BackupDrive, "Backup Drive (.img)");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::RestoreDrive, "Restore Drive (.img)");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::FlashTest, "Test Bad Blocks / Fake Flash");
            });

        let btn_text_clr = if state.is_running {
            crate::ui::style::text_secondary(is_dark)
        } else {
            crate::ui::style::text_primary(is_dark)
        };

        // SELECT button - Clean standard ASCII matching Rufus
        let select_btn = egui::Button::new(RichText::new("SELECT").strong().size(11.5).color(btn_text_clr))
            .min_size(Vec2::new(76.0, 22.0));
        if ui.add_enabled(!state.is_running, select_btn).clicked() {
            if usb_x_platform::has_native_file_dialog() {
                if let Some(chosen) = usb_x_platform::open_file_dialog(
                    "Select Bootable ISO or Disk Image",
                    "Disk Images (*.iso;*.img;*.bin;*.vhd;*.raw)",
                    "*.iso;*.img;*.bin;*.vhd;*.raw",
                ) {
                    state.load_image_file(chosen);
                }
            } else {
                state.open_file_browser();
            }
        }
    });

    ui.add_space(4.0);

    // 3. Image option: Automatically tailored for Windows (Standard/WTG) or Linux (GRUB/UEFI boot)
    let is_isohybrid = state.image_info.as_ref().map(|i| i.is_isohybrid || i.has_grub).unwrap_or(false);
    let is_linux = state.image_info.as_ref().map(|i| {
        matches!(i.detected_type, usb_x_core::types::ImageDetectedType::LinuxHybridIso { .. })
        || is_isohybrid
        || i.path.to_ascii_lowercase().contains("ubuntu")
        || i.path.to_ascii_lowercase().contains("linux")
        || i.path.to_ascii_lowercase().contains("mint")
        || i.path.to_ascii_lowercase().contains("debian")
    }).unwrap_or(false);

    ui.label(field_label("Image option", is_dark));
    if is_linux || is_isohybrid {
        let opt_label = match state.write_mode {
            usb_x_core::types::ImageWriteMode::DdImageMode => "Write in DD Image mode (Raw disk copy)",
            _ => "Write in ISO Image mode (Recommended - Bootable USB & Full Drive)",
        };
        egui::ComboBox::from_id_salt("linux_image_option_combo")
            .width(ui.available_width() - 4.0)
            .selected_text(opt_label)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut state.write_mode,
                    usb_x_core::types::ImageWriteMode::IsoImageMode,
                    "Write in ISO Image mode (Recommended - Bootable USB & Full Drive)",
                );
                ui.selectable_value(
                    &mut state.write_mode,
                    usb_x_core::types::ImageWriteMode::DdImageMode,
                    "Write in DD Image mode (Raw disk copy)",
                );
            });

        let is_arch = state.image_info.as_ref().map(|i| {
            i.path.to_ascii_lowercase().contains("arch")
                || i.volume_label.as_deref().unwrap_or("").to_ascii_lowercase().contains("arch")
        }).unwrap_or(false);

        if is_arch {
            ui.add_space(2.0);
            let arch_color = if is_dark { Color32::from_rgb(255, 175, 60) } else { Color32::from_rgb(180, 100, 0) };
            ui.label(
                RichText::new("ℹ Arch Linux requires 'Secure Boot: Disabled' in PC BIOS.")
                    .size(11.0)
                    .color(arch_color)
            );
        }

        ui.add_space(3.0);
    } else {
        let image_option_text = if state.is_windows_to_go {
            "Windows To Go"
        } else {
            "Standard Windows installation"
        };

        egui::ComboBox::from_id_salt("image_option_combo")
            .width(ui.available_width() - 4.0)
            .selected_text(image_option_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.is_windows_to_go, false, "Standard Windows installation");
                ui.selectable_value(&mut state.is_windows_to_go, true, "Windows To Go");
            });

        ui.add_space(3.0);
    }

    // 4. Partition scheme & Target system (Two Columns)
    // Enforce mutual locking to prevent firmware & boot mismatch
    match state.partition_scheme {
        PartitionScheme::Gpt => state.target_system = TargetSystem::Uefi,
        PartitionScheme::Mbr | PartitionScheme::Auto => state.target_system = TargetSystem::Bios,
    }

    ui.columns(2, |cols| {
        // Left: Partition scheme
        cols[0].label(field_label("Partition scheme", is_dark));
        let part_text = match state.partition_scheme {
            PartitionScheme::Gpt => "GPT",
            PartitionScheme::Mbr => "MBR",
            PartitionScheme::Auto => "GPT",
        };
        egui::ComboBox::from_id_salt("part_scheme_combo")
            .width(cols[0].available_width() - 4.0)
            .selected_text(part_text)
            .show_ui(&mut cols[0], |ui| {
                if ui.selectable_value(&mut state.partition_scheme, PartitionScheme::Gpt, "GPT").clicked() {
                    state.target_system = TargetSystem::Uefi;
                }
                if ui.selectable_value(&mut state.partition_scheme, PartitionScheme::Mbr, "MBR").clicked() {
                    state.target_system = TargetSystem::Bios;
                }
            });

        // Right: Target system
        cols[1].horizontal(|ui| {
            ui.label(field_label("Target system", is_dark));
            ui.label(RichText::new("?").strong().size(11.0).color(text_secondary(is_dark))).on_hover_text(
                "UEFI (non CSM): Modern UEFI boot (required for GPT).\nBIOS (or UEFI-CSM): Legacy BIOS boot (requires MBR)."
            );
        });

        match state.partition_scheme {
            PartitionScheme::Gpt => {
                state.target_system = TargetSystem::Uefi;
                cols[1].add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("target_system_combo_gpt_locked")
                        .width(ui.available_width() - 4.0)
                        .selected_text("UEFI (non CSM)")
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.target_system, TargetSystem::Uefi, "UEFI (non CSM)");
                        });
                }).response.on_disabled_hover_text("GPT partition scheme strictly requires UEFI (non CSM)");
            }
            PartitionScheme::Mbr | PartitionScheme::Auto => {
                state.target_system = TargetSystem::Bios;
                cols[1].add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("target_system_combo_mbr_locked")
                        .width(ui.available_width() - 4.0)
                        .selected_text("BIOS (or UEFI-CSM)")
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.target_system, TargetSystem::Bios, "BIOS (or UEFI-CSM)");
                        });
                }).response.on_disabled_hover_text("MBR partition scheme locked to BIOS (or UEFI-CSM) to prevent firmware mismatch");
            }
        }
    });

    ui.add_space(3.0);

    // 5. Collapsible: Advanced drive properties
    let arrow = if state.show_advanced_drive_properties { "[-] Hide advanced drive properties" } else { "[+] Show advanced drive properties" };
    let adv_btn = egui::Button::new(RichText::new(arrow).size(11.0).color(text_secondary(is_dark)))
        .fill(Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE);
    if ui.add(adv_btn).clicked() {
        state.show_advanced_drive_properties = !state.show_advanced_drive_properties;
    }

    if state.show_advanced_drive_properties {
        ui.indent("advanced_drive_props", |ui| {
            ui.checkbox(&mut state.list_usb_hard_drives, "List USB Hard Drives");
            ui.checkbox(&mut state.old_bios_fixes, "Add fixes for old BIOSes (extra partition, align, etc.)");
            ui.checkbox(&mut state.use_rufus_mbr, "Use Rufus MBR with BIOS ID");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Context;

    #[test]
    fn test_gpt_forces_uefi_target() {
        let mut state = AppState::new();
        state.partition_scheme = PartitionScheme::Gpt;
        state.target_system = TargetSystem::Bios; // invalid combination attempted

        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_drive_properties(ui, &mut state);
            });
        });

        assert_eq!(state.target_system, TargetSystem::Uefi, "GPT partition scheme must strictly enforce UEFI target system");
    }

    #[test]
    fn test_mbr_forces_bios_target() {
        let mut state = AppState::new();
        state.partition_scheme = PartitionScheme::Mbr;
        state.target_system = TargetSystem::Uefi; // invalid combination attempted

        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_drive_properties(ui, &mut state);
            });
        });

        assert_eq!(state.target_system, TargetSystem::Bios, "MBR partition scheme must strictly enforce BIOS target system");
    }
}
