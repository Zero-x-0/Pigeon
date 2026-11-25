use egui::{Color32, CornerRadius, RichText, Stroke, Ui, Vec2};
use crate::state::AppState;
use crate::ui::style::{
    expander_button, field_label, icon_button, render_section_header, split_arrow_button,
    text_primary, text_secondary, ToolbarIcon, COLOR_DANGER_RED,
};
use usb_x_core::types::{BootSelectionMode, ImageWriteMode, PartitionScheme, TargetSystem};

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
            .stroke(Stroke::new(1.0_f32, banner_stroke))
            .corner_radius(CornerRadius::same(2))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Admin rights needed to write to flash drives.")
                            .color(banner_text)
                            .size(11.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Restart as Root").strong().size(11.0)).clicked() {
                            if usb_x_platform::relaunch_as_admin() {
                                std::process::exit(0);
                            } else {
                                state.log(usb_x_core::event::LogLevel::Warn, "Administrator elevation could not be initiated.".to_string());
                            }
                        }
                    });
                });
            });
        ui.add_space(3.0);
    }

    // 1. Device + Save Drive Image Icon
    ui.label(field_label("Device", is_dark));
    let visible = state.visible_devices();
    let visible_indices: Vec<(usize, String, bool)> = visible
        .iter()
        .map(|(idx, d)| (*idx, d.summary(), d.system_risk.is_destructive_blocked()))
        .collect();

    let selected_summary = state
        .selected_device()
        .map(|d| d.summary())
        .unwrap_or_else(|| "No removable devices detected".to_string());

    ui.horizontal(|ui| {
        let combo_width = (ui.available_width() - 28.0).max(160.0);
        egui::ComboBox::from_id_salt("device_picker_combo")
            .width(combo_width)
            .selected_text(selected_summary)
            .show_ui(ui, |ui| {
                if visible_indices.is_empty() {
                    ui.label(RichText::new("No removable USB devices found").color(text_secondary(is_dark)));
                } else {
                    for (idx, summary, is_system) in &visible_indices {
                        let text = if *is_system {
                            format!("{} [PROTECTED SYSTEM DISK]", summary)
                        } else {
                            summary.clone()
                        };
                        ui.selectable_value(&mut state.selected_device_idx, *idx, text);
                    }
                }
            });

        let can_backup = !state.is_running && state.selected_device().is_some();
        let save_resp = icon_button(ui, ToolbarIcon::SaveDisk, is_dark, can_backup)
            .on_hover_text("Save an image of the selected drive (.img)");
        if save_resp.hovered() {
            state.status_bar_hint = Some("Save an image of the selected drive".to_string());
        }
        if save_resp.clicked() && can_backup {
            state.trigger_backup_drive();
        }
    });

    if let Some(dev) = state.selected_device() {
        if dev.system_risk.is_destructive_blocked() {
            ui.label(
                RichText::new("PROTECTED HOST DISK: Contains running OS. Destructive actions disabled.")
                    .color(COLOR_DANGER_RED)
                    .small()
                    .strong(),
            );
        }
    }

    ui.add_space(4.0);

    // 2. Boot selection + Checksums (✓) + Split Button [ SELECT | ▼ ] / [ DOWNLOAD | ▼ ]
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

        let combo_width = (ui.available_width() - 122.0).max(120.0);
        egui::ComboBox::from_id_salt("boot_selection_mode_combo")
            .width(combo_width)
            .selected_text(boot_display_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::FormatOnly, "Non bootable");
                ui.selectable_value(&mut state.boot_mode, BootSelectionMode::DiskImage, "Disk or ISO image");
            });

        // Circular Checkmark Button: Compute Checksums (MD5/SHA1/SHA256/SHA512)
        let has_image = !state.image_path.is_empty() && state.boot_mode == BootSelectionMode::DiskImage;
        let chk_resp = icon_button(ui, ToolbarIcon::CheckCircle, is_dark, has_image)
            .on_hover_text("Compute image checksums (MD5, SHA1, SHA256, SHA512)");
        if chk_resp.hovered() {
            state.status_bar_hint = Some("Compute image checksums".to_string());
        }
        if chk_resp.clicked() && has_image {
            if state.calculated_checksums.is_none() && !state.is_calculating_hash {
                state.start_hash_calculation();
            }
            state.show_hash_dialog = true;
        }

        // Split button: [ SELECT | ▼ ] or [ DOWNLOAD | ▼ ]
        let btn_text_clr = if state.is_running {
            text_secondary(is_dark)
        } else {
            text_primary(is_dark)
        };
        let split_label = if state.select_split_is_download { "DOWNLOAD" } else { "SELECT" };

        ui.spacing_mut().item_spacing.x = 0.0;
        let main_btn = egui::Button::new(RichText::new(split_label).size(11.5).color(btn_text_clr))
            .min_size(Vec2::new(72.0_f32, 22.0_f32));
        if ui.add_enabled(!state.is_running, main_btn).clicked() {
            if state.select_split_is_download {
                state.show_download_dialog = true;
            } else {
                state.trigger_file_dialog();
            }
        }

        let arrow_btn_id = ui.make_persistent_id("select_download_split_menu");
        let arrow_resp = split_arrow_button(ui, is_dark, !state.is_running);
        if arrow_resp.clicked() && !state.is_running {
            ui.memory_mut(|m| m.toggle_popup(arrow_btn_id));
        }
        egui::popup_below_widget(ui, arrow_btn_id, &arrow_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(135.0);
            if ui.selectable_label(!state.select_split_is_download, "SELECT (Local ISO)").clicked() {
                state.select_split_is_download = false;
                state.trigger_file_dialog();
            }
            if ui.selectable_label(state.select_split_is_download, "DOWNLOAD (Online ISO)").clicked() {
                state.select_split_is_download = true;
                state.show_download_dialog = true;
            }
        });
    });

    ui.add_space(4.0);

    let is_linux = state.is_linux_image();
    let is_windows = state.is_windows_image();
    let is_raw = state.image_info.as_ref().map(|i| i.is_raw_disk_image()).unwrap_or(false);
    let dd_locked = state.is_dd_only_mode();

    // 3. Persistent partition size slider (for Linux ISOs in ISO mode, matching Image 1)
    if is_linux && state.boot_mode == BootSelectionMode::DiskImage && !dd_locked {
        ui.label(field_label("Persistent partition size", is_dark));
        let dev_max_mb = state
            .selected_device()
            .map(|d| (d.size_bytes / (1024 * 1024)).saturating_sub(2048).max(0))
            .unwrap_or(8192);
        let max_gb = (dev_max_mb / 1024).max(1);
        let max_val = if state.persist_unit_is_gb { max_gb } else { dev_max_mb.max(256) };

        let mut cur_val = if state.persist_unit_is_gb {
            state.linux_persistence_mb / 1024
        } else {
            state.linux_persistence_mb
        };

        ui.horizontal(|ui| {
            let slider_w = (ui.available_width() - 106.0).max(120.0);
            ui.spacing_mut().slider_width = slider_w;
            if ui.add(egui::Slider::new(&mut cur_val, 0..=max_val).show_value(false)).changed() {
                state.linux_persistence_mb = if state.persist_unit_is_gb {
                    cur_val * 1024
                } else {
                    cur_val
                };
            }

            let mut txt = cur_val.to_string();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut txt)
                        .desired_width(42.0)
                        .horizontal_align(egui::Align::RIGHT),
                )
                .changed()
            {
                if let Ok(parsed) = txt.trim().parse::<u64>() {
                    let clamped = parsed.min(max_val);
                    state.linux_persistence_mb = if state.persist_unit_is_gb {
                        clamped * 1024
                    } else {
                        clamped
                    };
                }
            }

            let unit_lbl = if state.persist_unit_is_gb { "GB" } else { "MB" };
            egui::ComboBox::from_id_salt("persist_unit_combo")
                .width(48.0)
                .selected_text(unit_lbl)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.persist_unit_is_gb, true, "GB");
                    ui.selectable_value(&mut state.persist_unit_is_gb, false, "MB");
                });
        });

        ui.add_space(4.0);
    }

    // 4. Image option (Windows standard, Linux ISO vs DD, or Raw DD image)
    if state.boot_mode == BootSelectionMode::DiskImage && !state.image_path.is_empty() {
        ui.label(field_label("Image option", is_dark));
        if is_windows {
            ui.add_enabled_ui(false, |ui| {
                let mut std_mode = 0u8;
                egui::ComboBox::from_id_salt("win_image_option_combo")
                    .width(ui.available_width() - 2.0)
                    .selected_text("Standard Windows installation")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut std_mode, 0, "Standard Windows installation");
                    });
            });
        } else if is_raw {
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt("raw_image_option_combo")
                    .width(ui.available_width() - 2.0)
                    .selected_text("Write in DD Image mode (Raw disk copy)")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.write_mode, ImageWriteMode::DdImageMode, "Write in DD Image mode (Raw disk copy)");
                    });
            });
        } else {
            let opt_label = state.write_mode.label();
            egui::ComboBox::from_id_salt("linux_image_option_combo")
                .width(ui.available_width() - 2.0)
                .selected_text(opt_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut state.write_mode,
                        ImageWriteMode::IsoImageMode,
                        ImageWriteMode::IsoImageMode.label(),
                    );
                    ui.selectable_value(
                        &mut state.write_mode,
                        ImageWriteMode::DdImageMode,
                        ImageWriteMode::DdImageMode.label(),
                    );
                });
        }
        ui.add_space(4.0);
    }

    // 5. Partition scheme & Target system (Two Columns matching Image 1, disabled in DD mode)
    match state.partition_scheme {
        PartitionScheme::Gpt => state.target_system = TargetSystem::Uefi,
        PartitionScheme::Mbr => state.target_system = TargetSystem::Bios,
    }

    ui.add_enabled_ui(!dd_locked, |ui| {
        ui.columns(2, |cols| {
            cols[0].label(field_label("Partition scheme", is_dark));
            let part_text = if dd_locked {
                "Image default".to_string()
            } else {
                state.partition_scheme.label().to_string()
            };
            egui::ComboBox::from_id_salt("part_scheme_combo")
                .width(cols[0].available_width() - 4.0)
                .selected_text(part_text)
                .show_ui(&mut cols[0], |ui| {
                    if ui.selectable_value(&mut state.partition_scheme, PartitionScheme::Mbr, "MBR").clicked() {
                        state.target_system = TargetSystem::Bios;
                    }
                    if ui.selectable_value(&mut state.partition_scheme, PartitionScheme::Gpt, "GPT").clicked() {
                        state.target_system = TargetSystem::Uefi;
                    }
                });

            cols[1].horizontal(|ui| {
                ui.label(field_label("Target system", is_dark));
                ui.label(RichText::new("?").size(11.0).color(text_secondary(is_dark))).on_hover_text(
                    "UEFI (non CSM): Modern UEFI boot (GPT).\nBIOS or UEFI: Legacy BIOS & UEFI boot (MBR).",
                );
            });

            cols[1].add_enabled_ui(false, |ui| {
                let target_lbl = if dd_locked {
                    "Image default"
                } else {
                    state.target_system.label()
                };
                let cur_target = state.target_system;
                egui::ComboBox::from_id_salt("target_system_combo_locked")
                    .width(ui.available_width() - 2.0)
                    .selected_text(target_lbl)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.target_system, cur_target, target_lbl);
                    });
            });
        });
    });

    ui.add_space(4.0);

    // 6. Collapsible: Advanced drive properties (matches Image 1)
    let adv_lbl = if state.show_advanced_drive_properties {
        "Hide advanced drive properties"
    } else {
        "Show advanced drive properties"
    };
    if expander_button(ui, state.show_advanced_drive_properties, adv_lbl, is_dark).clicked() {
        state.show_advanced_drive_properties = !state.show_advanced_drive_properties;
    }

    if state.show_advanced_drive_properties {
        ui.indent("advanced_drive_props", |ui| {
            if ui.checkbox(&mut state.list_usb_hard_drives, "List USB Hard Drives").changed() {
                state.refresh_devices();
            }
            let allow_mbr_fixes = !dd_locked && state.partition_scheme == PartitionScheme::Mbr;
            ui.add_enabled_ui(allow_mbr_fixes, |ui| {
                ui.checkbox(&mut state.old_bios_fixes, "Add fixes for old BIOSes (extra partition, align, etc.)");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.use_rufus_mbr, "Use Rufus MBR with BIOS ID");
                    ui.add_enabled_ui(state.use_rufus_mbr && allow_mbr_fixes, |ui| {
                        let bios_id_lbl = match state.rufus_mbr_bios_id {
                            0x80 => "0x80 (Default)",
                            0x81 => "0x81 (2nd disk)",
                            _ => "0x82 (3rd disk)",
                        };
                        egui::ComboBox::from_id_salt("rufus_mbr_bios_id_combo")
                            .width((ui.available_width() - 2.0).max(115.0))
                            .selected_text(bios_id_lbl)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.rufus_mbr_bios_id, 0x80, "0x80 (Default)");
                                ui.selectable_value(&mut state.rufus_mbr_bios_id, 0x81, "0x81 (2nd disk)");
                                ui.selectable_value(&mut state.rufus_mbr_bios_id, 0x82, "0x82 (3rd disk)");
                            });
                    });
                });
            });
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
        state.target_system = TargetSystem::Bios;

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
        state.target_system = TargetSystem::Uefi;

        let ctx = Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_drive_properties(ui, &mut state);
            });
        });

        assert_eq!(state.target_system, TargetSystem::Bios, "MBR partition scheme must strictly enforce BIOS target system");
    }
}
