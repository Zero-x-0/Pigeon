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

fn format_file_time(meta: &std::fs::Metadata) -> String {
    if let Ok(time) = meta.modified() {
        if let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) {
            let secs = duration.as_secs();
            let days = secs / 86400;
            let time_in_day = secs % 86400;
            let hours = time_in_day / 3600;
            let minutes = (time_in_day % 3600) / 60;

            let mut y = 1970;
            let mut d = days;
            loop {
                let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
                let days_in_year = if leap { 366 } else { 365 };
                if d < days_in_year {
                    break;
                }
                d -= days_in_year;
                y += 1;
            }
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            let mut m = 1;
            for md in month_days {
                if d < md {
                    break;
                }
                d -= md;
                m += 1;
            }
            let day = d + 1;
            return format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, day, hours, minutes);
        }
    }
    String::new()
}

fn get_path_breadcrumbs(path: &std::path::Path) -> Vec<(String, PathBuf)> {
    let mut crumbs = Vec::new();
    let mut current = Some(path);
    while let Some(p) = current {
        let name = match p.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => {
                let s = p.to_string_lossy().to_string();
                if s.is_empty() {
                    "/".to_string()
                } else {
                    s
                }
            }
        };
        if !name.is_empty() {
            crumbs.push((name, p.to_path_buf()));
        }
        current = p.parent();
    }
    crumbs.reverse();
    crumbs
}

#[cfg(windows)]
fn get_system_drives() -> Vec<(String, PathBuf)> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let root = format!("{}:\\", letter as char);
        let path = PathBuf::from(&root);
        if path.exists() {
            let label = if (letter as char) == 'C' {
                format!("🗄 System ({}:)", letter as char)
            } else {
                format!("💾 Drive ({}:)", letter as char)
            };
            drives.push((label, path));
        }
    }
    drives
}

#[cfg(not(windows))]
fn get_system_drives() -> Vec<(String, PathBuf)> {
    let mut drives = Vec::new();
    drives.push(("🗄 Root ( / )".to_string(), PathBuf::from("/")));

    for mount_root in &["/media", "/mnt", "/run/media"] {
        let p = std::path::Path::new(mount_root);
        if p.is_dir() {
            drives.push((format!("💾 {}", mount_root), p.to_path_buf()));
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let sub = entry.path();
                    if sub.is_dir() {
                        let name = sub.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            drives.push((format!("  💾 {}", name), sub.clone()));
                            if let Ok(user_entries) = std::fs::read_dir(&sub) {
                                for ue in user_entries.flatten() {
                                    let usb_p = ue.path();
                                    if usb_p.is_dir() {
                                        let usb_name = usb_p.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        if !usb_name.starts_with('.') {
                                            drives.push((format!("    🔌 {}", usb_name), usb_p));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    drives
}

#[cfg(not(windows))]
fn get_linux_user_homes() -> Vec<(String, PathBuf)> {
    let mut homes = Vec::new();
    let home_dir = std::path::Path::new("/home");
    if let Ok(entries) = std::fs::read_dir(home_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let user = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                if !user.starts_with('.') {
                    homes.push((format!("👤 {}", user), p));
                }
            }
        }
    }
    homes
}

pub fn show_file_browser_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_file_browser_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();

    // Show as a real, independent, free-floating desktop window (Viewport)
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("pigeon_file_browser_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Select Bootable ISO or Disk Image")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([820.0, 540.0])
            .with_min_inner_size([580.0, 380.0])
            .with_resizable(true),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)) {
                state.show_file_browser_dialog = false;
            }

            let bg_color = crate::ui::style::panel_bg(is_dark);
            let mut close_dialog = false;
            let mut file_to_load: Option<String> = None;
            let mut navigate_to_path: Option<PathBuf> = None;

            // 1. Top Panel: Back, Forward, Up, Refresh, Breadcrumbs / Address bar, Search
            egui::TopBottomPanel::top("file_browser_top_panel")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 8 }),
                )
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    ui.horizontal(|ui| {
                        let can_back = state.file_browser_can_go_back();
                        if ui.add_enabled(can_back, egui::Button::new(RichText::new("⬅").size(13.0))).on_hover_text("Back").clicked() {
                            state.file_browser_go_back();
                        }

                        let can_forward = state.file_browser_can_go_forward();
                        if ui.add_enabled(can_forward, egui::Button::new(RichText::new("➡").size(13.0))).on_hover_text("Forward").clicked() {
                            state.file_browser_go_forward();
                        }

                        let can_up = state.file_browser_current_dir.parent().is_some();
                        if ui.add_enabled(can_up, egui::Button::new(RichText::new("⬆ Up").strong().size(12.0))).on_hover_text("Go up one folder").clicked() {
                            state.file_browser_go_up();
                        }

                        if ui.button(RichText::new("🔄").size(12.0)).on_hover_text("Refresh").clicked() {
                            state.file_browser_error = None;
                        }

                        // Toggle between Breadcrumbs and Direct Text Edit
                        let edit_icon = if state.file_browser_editing_path { "✕ Cancel" } else { "✏ Edit" };
                        if ui.button(RichText::new(edit_icon).size(11.0)).clicked() {
                            state.file_browser_editing_path = !state.file_browser_editing_path;
                            if state.file_browser_editing_path {
                                state.file_browser_path_input = state.file_browser_current_dir.to_string_lossy().to_string();
                            }
                        }

                        // If editing path: Show direct text box and Go button
                        if state.file_browser_editing_path {
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut state.file_browser_path_input)
                                    .desired_width(ui.available_width() - 170.0)
                                    .hint_text("Type path and press Enter..."),
                            );
                            let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let go_clicked = ui.button(RichText::new("Go ➡").strong().size(11.5)).clicked();

                            if enter_pressed || go_clicked {
                                let target = PathBuf::from(&state.file_browser_path_input);
                                if target.is_dir() {
                                    navigate_to_path = Some(target);
                                    state.file_browser_editing_path = false;
                                } else if target.is_file() {
                                    file_to_load = Some(target.to_string_lossy().to_string());
                                    close_dialog = true;
                                } else {
                                    state.file_browser_error = Some(format!("Path not found: {}", state.file_browser_path_input));
                                }
                            }
                        } else {
                            // Breadcrumb buttons
                            let crumbs = get_path_breadcrumbs(&state.file_browser_current_dir);
                            egui::ScrollArea::horizontal()
                                .max_width(ui.available_width() - 170.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        for (idx, (name, path)) in crumbs.iter().enumerate() {
                                            if idx > 0 {
                                                ui.label(RichText::new("/").color(text_secondary(is_dark)));
                                            }
                                            if ui.button(RichText::new(name).size(11.5)).clicked() {
                                                navigate_to_path = Some(path.clone());
                                            }
                                        }
                                    });
                                });
                        }

                        // Search Box on Right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !state.file_browser_search.is_empty() {
                                if ui.button(RichText::new("✕").size(11.0)).on_hover_text("Clear search").clicked() {
                                    state.file_browser_search.clear();
                                }
                            }
                            ui.add(
                                egui::TextEdit::singleline(&mut state.file_browser_search)
                                    .desired_width(130.0)
                                    .hint_text("🔍 Filter..."),
                            );
                        });
                    });

                    if let Some(ref err) = state.file_browser_error {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("⚠ {}", err)).color(Color32::from_rgb(230, 80, 80)).size(11.5));
                        });
                    }
                });

            // 2. Bottom Panel: Selected file info, Filter combo, Open and Cancel buttons
            egui::TopBottomPanel::bottom("file_browser_bottom_panel")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 12, right: 12, top: 8, bottom: 10 }),
                )
                .show(ctx, |ui| {
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
                                .width(200.0)
                                .selected_text(filter_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut state.file_browser_filter_all, false, "Disk Images (*.iso, *.img, *.raw)");
                                    ui.selectable_value(&mut state.file_browser_filter_all, true, "All Files (*.*)");
                                });
                        });
                    });
                });

            // 3. Left Panel: Places, Linux Users, System Drives & Media
            egui::SidePanel::left("file_browser_side_panel")
                .default_width(170.0)
                .width_range(140.0..=240.0)
                .frame(
                    egui::Frame::NONE
                        .fill(if is_dark { Color32::from_rgb(26, 26, 30) } else { Color32::from_rgb(240, 242, 245) })
                        .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 }),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label(field_label("Places", is_dark));

                            #[cfg(windows)]
                            let home = std::env::var("USERPROFILE").ok();
                            #[cfg(not(windows))]
                            let home = {
                                if let Ok(sudo_user) = std::env::var("SUDO_USER") {
                                    if !sudo_user.is_empty() && sudo_user != "root" {
                                        let user_home = PathBuf::from(format!("/home/{}", sudo_user));
                                        if user_home.is_dir() {
                                            Some(user_home.to_string_lossy().to_string())
                                        } else {
                                            std::env::var("HOME").ok()
                                        }
                                    } else {
                                        std::env::var("HOME").ok()
                                    }
                                } else {
                                    std::env::var("HOME").ok()
                                }
                            };

                            if let Some(ref h) = home {
                                let h_path = PathBuf::from(h);
                                if ui.button("🏠 Home").clicked() {
                                    navigate_to_path = Some(h_path.clone());
                                }
                                let dt = h_path.join("Desktop");
                                if dt.is_dir() && ui.button("🖥 Desktop").clicked() {
                                    navigate_to_path = Some(dt);
                                }
                                let dl = h_path.join("Downloads");
                                if dl.is_dir() && ui.button("📥 Downloads").clicked() {
                                    navigate_to_path = Some(dl);
                                }
                                let doc = h_path.join("Documents");
                                if doc.is_dir() && ui.button("📄 Documents").clicked() {
                                    navigate_to_path = Some(doc);
                                }
                            }

                            #[cfg(not(windows))]
                            {
                                let user_homes = get_linux_user_homes();
                                if !user_homes.is_empty() {
                                    ui.add_space(4.0);
                                    ui.label(field_label("Users", is_dark));
                                    for (label, p) in user_homes {
                                        if ui.button(&label).clicked() {
                                            navigate_to_path = Some(p);
                                        }
                                    }
                                }
                            }

                            ui.add_space(6.0);
                            ui.label(field_label("Drives & Media", is_dark));

                            for (label, drive_path) in get_system_drives() {
                                if ui.button(&label).clicked() {
                                    navigate_to_path = Some(drive_path);
                                }
                            }
                        });
                });

            // 4. Central Panel: Folder and file table
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 10, bottom: 10 }),
                )
                .show(ctx, |ui| {
                    let mut pick_file: Option<PathBuf> = None;

                    // Table Header
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Name").strong().size(11.5).color(text_secondary(is_dark)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("Date Modified").strong().size(11.5).color(text_secondary(is_dark)));
                            ui.add_space(40.0);
                            ui.label(RichText::new("Size").strong().size(11.5).color(text_secondary(is_dark)));
                            ui.add_space(20.0);
                        });
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            match std::fs::read_dir(&state.file_browser_current_dir) {
                                Ok(entries) => {
                                    let mut dirs = Vec::new();
                                    let mut files = Vec::new();

                                    let search_term = state.file_browser_search.to_lowercase();

                                    for entry in entries.flatten() {
                                        let path = entry.path();
                                        let fname = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                                        if fname.starts_with('.') && fname != ".." {
                                            continue; // Hide hidden dot files/folders
                                        }

                                        if !search_term.is_empty() && !fname.to_lowercase().contains(&search_term) {
                                            continue;
                                        }

                                        if path.is_dir() {
                                            dirs.push((fname, path));
                                        } else if path.is_file() {
                                            files.push((fname, path));
                                        }
                                    }

                                    dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
                                    files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

                                    if dirs.is_empty() && files.is_empty() {
                                        ui.add_space(20.0);
                                        ui.vertical_centered(|ui| {
                                            ui.label(RichText::new("This folder is empty").size(12.0).color(text_secondary(is_dark)));
                                        });
                                    }

                                    // 1. Folders
                                    for (fname, d_path) in dirs {
                                        ui.horizontal(|ui| {
                                            let label = format!("📁 {}", fname);
                                            let resp = ui.selectable_label(false, RichText::new(label).strong().size(12.0));
                                            if resp.double_clicked() || (resp.clicked() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                                navigate_to_path = Some(d_path);
                                            }
                                        });
                                    }

                                    // 2. Files
                                    for (fname, f_path) in files {
                                        let lower = fname.to_ascii_lowercase();
                                        let is_disk_image = lower.ends_with(".iso")
                                            || lower.ends_with(".img")
                                            || lower.ends_with(".bin")
                                            || lower.ends_with(".raw")
                                            || lower.ends_with(".vhd");

                                        if !state.file_browser_filter_all && !is_disk_image {
                                            continue;
                                        }

                                        let is_selected = state.file_browser_selected_file.as_ref() == Some(&f_path);
                                        let meta = std::fs::metadata(&f_path).ok();
                                        let size_str = meta.as_ref().map(|m| format_file_size(m.len())).unwrap_or_default();
                                        let date_str = meta.as_ref().map(|m| format_file_time(m)).unwrap_or_default();

                                        let icon = if is_disk_image { "💽" } else { "📄" };
                                        let text_clr = if is_disk_image {
                                            if is_dark { Color32::from_rgb(110, 205, 255) } else { Color32::from_rgb(0, 102, 204) }
                                        } else {
                                            text_primary(is_dark)
                                        };

                                        ui.horizontal(|ui| {
                                            let resp = ui.selectable_label(
                                                is_selected,
                                                RichText::new(format!("{} {}", icon, fname))
                                                    .strong()
                                                    .size(12.0)
                                                    .color(text_clr),
                                            );

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.label(RichText::new(date_str).size(11.0).color(text_secondary(is_dark)));
                                                ui.add_space(20.0);
                                                ui.label(RichText::new(size_str).size(11.0).color(text_secondary(is_dark)));
                                            });

                                            if resp.clicked() {
                                                state.file_browser_selected_file = Some(f_path.clone());
                                            }
                                            if resp.double_clicked() {
                                                pick_file = Some(f_path);
                                            }
                                        });
                                    }
                                }
                                Err(err) => {
                                    ui.add_space(20.0);
                                    ui.label(
                                        RichText::new(format!("Error reading directory: {}", err))
                                            .color(Color32::from_rgb(230, 60, 60))
                                            .size(12.0),
                                    );
                                    if ui.button("⬆ Go to Parent Folder").clicked() {
                                        state.file_browser_go_up();
                                    }
                                }
                            }
                        });

                    if let Some(chosen_file) = pick_file {
                        file_to_load = Some(chosen_file.to_string_lossy().to_string());
                        close_dialog = true;
                    }
                });

            if let Some(target_dir) = navigate_to_path {
                state.set_file_browser_dir(target_dir);
            }
            if let Some(path) = file_to_load {
                state.load_image_file(path);
                state.show_file_browser_dialog = false;
            }
            if close_dialog {
                state.show_file_browser_dialog = false;
            }
        },
    );
}
