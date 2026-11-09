use egui::{Color32, Context, RichText, Vec2};
use crate::state::AppState;
use crate::ui::style::{field_label, text_primary, text_secondary, COLOR_ACCENT_BLUE};
use std::path::PathBuf;

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn show_file_browser_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_file_browser_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();
    let mut close_dialog = false;
    let mut file_to_load: Option<String> = None;

    egui::Window::new("Select Bootable ISO or Disk Image")
        .open(&mut state.show_file_browser_dialog)
        .default_size(Vec2::new(680.0, 460.0))
        .min_size(Vec2::new(540.0, 360.0))
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            // 1. Top Bar: Navigation, Up button, Refresh, Current Directory
            ui.horizontal(|ui| {
                let can_go_up = state.file_browser_current_dir.parent().is_some();
                if ui.add_enabled(can_go_up, egui::Button::new(RichText::new("⬆ Up").strong().size(12.0))).clicked() {
                    if let Some(p) = state.file_browser_current_dir.parent() {
                        state.file_browser_current_dir = p.to_path_buf();
                        state.file_browser_selected_file = None;
                        state.file_browser_error = None;
                    }
                }

                if ui.button(RichText::new("🔄 Refresh").size(12.0)).clicked() {
                    state.file_browser_error = None;
                }

                ui.label(RichText::new("Path:").strong().size(12.0).color(text_secondary(is_dark)));
                let cur_path_str = state.file_browser_current_dir.to_string_lossy().to_string();
                ui.add(
                    egui::Label::new(
                        RichText::new(cur_path_str)
                            .monospace()
                            .size(11.5)
                            .color(text_primary(is_dark)),
                    )
                    .truncate(),
                );
            });

            ui.separator();

            // 2. Main content: Left shortcuts panel + Right file table
            ui.columns(2, |columns| {
                // Column 0: Left Shortcuts (Quick Access)
                let left_ui = &mut columns[0];
                left_ui.set_max_width(140.0);
                left_ui.vertical(|ui| {
                    ui.label(field_label("Quick Access", is_dark));

                    #[cfg(windows)]
                    let home = std::env::var("USERPROFILE").ok();
                    #[cfg(not(windows))]
                    let home = std::env::var("HOME").ok();

                    if let Some(ref h) = home {
                        let h_path = PathBuf::from(h);
                        if ui.button("🏠 Home").clicked() {
                            state.file_browser_current_dir = h_path.clone();
                            state.file_browser_selected_file = None;
                        }
                        let dt = h_path.join("Desktop");
                        if dt.is_dir() && ui.button("🖥 Desktop").clicked() {
                            state.file_browser_current_dir = dt;
                            state.file_browser_selected_file = None;
                        }
                        let dl = h_path.join("Downloads");
                        if dl.is_dir() && ui.button("📥 Downloads").clicked() {
                            state.file_browser_current_dir = dl;
                            state.file_browser_selected_file = None;
                        }
                        let doc = h_path.join("Documents");
                        if doc.is_dir() && ui.button("📄 Documents").clicked() {
                            state.file_browser_current_dir = doc;
                            state.file_browser_selected_file = None;
                        }
                    }

                    ui.separator();
                    ui.label(field_label("System", is_dark));

                    #[cfg(windows)]
                    {
                        if ui.button("🗄 System (C:)").clicked() {
                            state.file_browser_current_dir = PathBuf::from("C:\\");
                            state.file_browser_selected_file = None;
                        }
                    }

                    #[cfg(not(windows))]
                    {
                        if ui.button("🗄 Root ( / )").clicked() {
                            state.file_browser_current_dir = PathBuf::from("/");
                            state.file_browser_selected_file = None;
                        }
                        if std::path::Path::new("/media").is_dir() && ui.button("💾 /media").clicked() {
                            state.file_browser_current_dir = PathBuf::from("/media");
                            state.file_browser_selected_file = None;
                        }
                        if std::path::Path::new("/mnt").is_dir() && ui.button("💾 /mnt").clicked() {
                            state.file_browser_current_dir = PathBuf::from("/mnt");
                            state.file_browser_selected_file = None;
                        }
                        if std::path::Path::new("/run/media").is_dir() && ui.button("💾 /run/media").clicked() {
                            state.file_browser_current_dir = PathBuf::from("/run/media");
                            state.file_browser_selected_file = None;
                        }
                    }
                });

                // Column 1: Right File & Folder browser
                let right_ui = &mut columns[1];
                right_ui.vertical(|ui| {
                    let mut navigate_to: Option<PathBuf> = None;
                    let mut pick_file: Option<PathBuf> = None;

                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            match std::fs::read_dir(&state.file_browser_current_dir) {
                                Ok(entries) => {
                                    let mut dirs = Vec::new();
                                    let mut files = Vec::new();

                                    for entry in entries.flatten() {
                                        let path = entry.path();
                                        if path.is_dir() {
                                            dirs.push(path);
                                        } else if path.is_file() {
                                            files.push(path);
                                        }
                                    }

                                    // Sort alphabetically (case-insensitive)
                                    dirs.sort_by(|a, b| {
                                        a.file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_lowercase()
                                            .cmp(&b.file_name().unwrap_or_default().to_string_lossy().to_lowercase())
                                    });
                                    files.sort_by(|a, b| {
                                        a.file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_lowercase()
                                            .cmp(&b.file_name().unwrap_or_default().to_string_lossy().to_lowercase())
                                    });

                                    // 1. Folders
                                    for d in dirs {
                                        let name = d.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        if name.starts_with('.') && name != ".." {
                                            continue; // Hide hidden dot folders
                                        }
                                        let label = format!("📁 {}", name);
                                        let resp = ui.selectable_label(false, RichText::new(label).size(12.0));
                                        if resp.double_clicked() {
                                            navigate_to = Some(d);
                                            break;
                                        }
                                    }

                                    // 2. Files
                                    for f in files {
                                        let name = f.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        if name.starts_with('.') {
                                            continue; // Hide hidden dot files
                                        }

                                        let lower = name.to_ascii_lowercase();
                                        let is_disk_image = lower.ends_with(".iso")
                                            || lower.ends_with(".img")
                                            || lower.ends_with(".bin")
                                            || lower.ends_with(".raw")
                                            || lower.ends_with(".vhd");

                                        if !state.file_browser_filter_all && !is_disk_image {
                                            continue;
                                        }

                                        let is_selected = state.file_browser_selected_file.as_ref() == Some(&f);
                                        let size_str = std::fs::metadata(&f)
                                            .map(|m| format_file_size(m.len()))
                                            .unwrap_or_default();

                                        let icon = if is_disk_image { "💽" } else { "📄" };
                                        let text_clr = if is_disk_image {
                                            if is_dark { Color32::from_rgb(100, 200, 255) } else { Color32::from_rgb(0, 102, 204) }
                                        } else {
                                            text_primary(is_dark)
                                        };

                                        ui.horizontal(|ui| {
                                            let resp = ui.selectable_label(
                                                is_selected,
                                                RichText::new(format!("{} {}", icon, name))
                                                    .strong()
                                                    .size(12.0)
                                                    .color(text_clr),
                                            );

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(size_str).size(11.0).color(text_secondary(is_dark)));
                                            });

                                            if resp.clicked() {
                                                state.file_browser_selected_file = Some(f.clone());
                                            }
                                            if resp.double_clicked() {
                                                pick_file = Some(f);
                                            }
                                        });
                                    }
                                }
                                Err(err) => {
                                    ui.label(
                                        RichText::new(format!("Error opening directory: {}", err))
                                            .color(Color32::from_rgb(230, 60, 60))
                                            .size(12.0),
                                    );
                                }
                            }
                        });

                    if let Some(target_dir) = navigate_to {
                        state.file_browser_current_dir = target_dir;
                        state.file_browser_selected_file = None;
                    }
                    if let Some(chosen_file) = pick_file {
                        file_to_load = Some(chosen_file.to_string_lossy().to_string());
                        close_dialog = true;
                    }
                });
            });

            ui.separator();

            // 3. Bottom Bar: Selected file label, Filter selector, [Open] and [Cancel]
            ui.horizontal(|ui| {
                let sel_label = state
                    .file_browser_selected_file
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "No file selected".to_string());

                ui.label(RichText::new("File:").strong().size(12.0));
                ui.label(RichText::new(sel_label).strong().size(12.0).color(text_primary(is_dark)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let cancel_btn = egui::Button::new(RichText::new("Cancel").size(12.0))
                        .min_size(Vec2::new(76.0, 24.0));
                    if ui.add(cancel_btn).clicked() {
                        close_dialog = true;
                    }

                    let has_selection = state.file_browser_selected_file.is_some();
                    let open_btn = egui::Button::new(
                        RichText::new("Open")
                            .strong()
                            .size(12.0)
                            .color(Color32::WHITE),
                    )
                    .fill(COLOR_ACCENT_BLUE)
                    .min_size(Vec2::new(76.0, 24.0));

                    if ui.add_enabled(has_selection, open_btn).clicked() {
                        if let Some(ref sel) = state.file_browser_selected_file {
                            file_to_load = Some(sel.to_string_lossy().to_string());
                            close_dialog = true;
                        }
                    }

                    // Filter dropdown
                    let filter_text = if state.file_browser_filter_all {
                        "All Files (*.*)"
                    } else {
                        "Disk Images (*.iso, *.img, *.raw)"
                    };
                    egui::ComboBox::from_id_salt("file_browser_filter_combo")
                        .width(180.0)
                        .selected_text(filter_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.file_browser_filter_all, false, "Disk Images (*.iso, *.img, *.raw)");
                            ui.selectable_value(&mut state.file_browser_filter_all, true, "All Files (*.*)");
                        });
                });
            });
        });

    if let Some(path) = file_to_load {
        state.load_image_file(path);
    }
    if close_dialog {
        state.show_file_browser_dialog = false;
    }
}
