use crate::state::AppState;
use crate::ui::*;
use eframe::egui;

pub struct UsbXApp {
    state: AppState,
    last_applied_theme: Option<(bool, egui::Color32)>,
    last_window_height: f32,
}

impl UsbXApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "linux")]
        let is_dark = usb_x_platform::is_system_dark_mode();
        #[cfg(not(target_os = "linux"))]
        let is_dark = cc
            .egui_ctx
            .system_theme()
            .map(|t| t == egui::Theme::Dark)
            .unwrap_or_else(usb_x_platform::is_system_dark_mode);
        let accent = crate::ui::style::get_system_accent();
        crate::ui::style::apply_theme(&cc.egui_ctx, is_dark, accent);

        let mut state = AppState::new();
        state.cached_is_dark = is_dark;
        state.cached_accent = accent;

        Self {
            state,
            last_applied_theme: Some((is_dark, accent)),
            last_window_height: 515.0,
        }
    }
}

fn is_supported_image_extension(lower_path: &str) -> bool {
    const EXTS: &[&str] = &[".iso", ".img", ".bin", ".raw", ".vhd", ".dd", ".gz", ".xz", ".bz2", ".zst"];
    EXTS.iter().any(|ext| lower_path.ends_with(ext))
}

impl eframe::App for UsbXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.update_events();
        if self.state.is_running || self.state.dl_is_downloading || self.state.is_calculating_hash {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        } else if self.state.theme_probe_receiver.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(550));
        }

        // Rufus shortcuts:
        // - Alt+H: Compute & display SHA-512 checksum
        // - Alt+F: Toggle listing USB hard drives
        // - Ctrl+Alt+D: Toggle Light / Dark theme
        // - F5: Refresh USB devices
        let (alt_h, alt_f, ctrl_alt_d, f5) = ctx.input(|i| {
            (
                i.modifiers.alt && !i.modifiers.ctrl && i.key_pressed(egui::Key::H),
                i.modifiers.alt && !i.modifiers.ctrl && i.key_pressed(egui::Key::F),
                i.modifiers.ctrl && i.modifiers.alt && i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::F5),
            )
        });

        if alt_h && !self.state.image_path.is_empty() {
            if !self.state.include_sha512 {
                self.state.start_hash_calculation_ex(true);
            }
            self.state.show_hash_dialog = true;
        }
        if alt_f && !self.state.is_running {
            self.state.list_usb_hard_drives = !self.state.list_usb_hard_drives;
            self.state.refresh_devices();
        }
        if ctrl_alt_d {
            self.state.toggle_dark_light_shortcut();
        }
        if f5 {
            self.state.refresh_devices();
        }

        // On Linux X11/Xwayland, winit system_theme() can stay frozen at startup theme;
        // always use our live XDG Portal / gsettings / KDE probe instead.
        #[cfg(target_os = "linux")]
        let egui_hint = None;
        #[cfg(not(target_os = "linux"))]
        let egui_hint = ctx.system_theme().map(|t| t == egui::Theme::Dark);
        self.state.refresh_theme_if_needed(egui_hint);
        let is_dark = self.state.is_dark();
        let accent = self.state.cached_accent;

        if self.last_applied_theme != Some((is_dark, accent)) {
            crate::ui::style::apply_theme(ctx, is_dark, accent);
            self.last_applied_theme = Some((is_dark, accent));
            ctx.request_repaint();
        }

        if !self.state.is_running {
            ctx.input(|i| {
                if let Some(file) = i.raw.dropped_files.first() {
                    if let Some(ref path) = file.path {
                        let path_str = path.to_string_lossy().to_string();
                        if is_supported_image_extension(&path_str.to_ascii_lowercase()) {
                            self.state.load_image_file(path_str);
                        }
                    }
                }
            });
        }

        // Bottom Status Bar ("1 device found" | "00:00:00")
        let footer_fill = crate::ui::style::footer_bg(is_dark);
        let border_clr = crate::ui::style::border_color(is_dark);
        let footer_resp = egui::TopBottomPanel::bottom("rufus_bottom_status_bar")
            .exact_height(24.0)
            .frame(
                egui::Frame::NONE
                    .fill(footer_fill)
                    .stroke(egui::Stroke::new(1.0_f32, border_clr))
                    .inner_margin(egui::Margin::symmetric(10, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let left_text = if self.state.is_running && !self.state.current_detail_text.is_empty() {
                        self.state.current_detail_text.clone()
                    } else if let Some(ref hint) = self.state.status_bar_hint {
                        hint.clone()
                    } else {
                        let cnt = self.state.visible_devices().len();
                        if cnt == 1 {
                            "1 device found".to_string()
                        } else {
                            format!("{} devices found", cnt)
                        }
                    };

                    ui.label(
                        egui::RichText::new(left_text)
                            .size(11.5)
                            .color(crate::ui::style::text_primary(is_dark)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let timer_clr = if self.state.is_running {
                            crate::ui::style::text_primary(is_dark)
                        } else {
                            crate::ui::style::text_secondary(is_dark)
                        };
                        ui.label(
                            egui::RichText::new(&self.state.elapsed_display)
                                .size(11.5)
                                .color(timer_clr),
                        );
                        ui.add_space(6.0);
                        ui.separator();
                    });
                });
            });
        let footer_h = footer_resp.response.rect.height();

        let panel_frame = egui::Frame::NONE
            .fill(crate::ui::style::panel_bg(is_dark))
            .inner_margin(egui::Margin::symmetric(12, 8));

        let mut content_h = 0.0_f32;
        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            let content_resp = ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);

                ui.add_enabled_ui(!self.state.is_running, |ui| {
                    render_drive_properties(ui, &mut self.state);
                    ui.add_space(4.0);
                    render_format_options(ui, &mut self.state);
                });

                ui.add_space(4.0);
                render_status_section(ui, &mut self.state);
            });
            content_h = content_resp.response.rect.height();
        });

        let max_screen_h = ctx
            .input(|i| i.viewport().monitor_size)
            .map(|sz| (sz.y - 80.0).max(520.0))
            .unwrap_or(960.0);
        let needed_h = (content_h + 16.0 + footer_h + 4.0)
            .ceil()
            .clamp(400.0, max_screen_h);

        if (needed_h - self.last_window_height).abs() >= 2.0 {
            let target_size = egui::vec2(430.0, needed_h);
            if needed_h > self.last_window_height {
                ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(target_size));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(target_size));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(target_size));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target_size));
                ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(target_size));
            }
            self.last_window_height = needed_h;
            ctx.request_repaint();
        }

        // Secondary Dialogs / Viewports
        show_confirm_dialog(ctx, &mut self.state);
        show_hash_dialog(ctx, &mut self.state);
        show_log_dialog(ctx, &mut self.state);
        show_download_dialog(ctx, &mut self.state);
        show_about_dialog(ctx, &mut self.state);
        show_settings_dialog(ctx, &mut self.state);
        show_file_browser_dialog(ctx, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure_main_content_height(state: &mut AppState) -> f32 {
        let ctx = egui::Context::default();
        let mut h = 0.0_f32;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(430.0, 900.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(12, 8)))
                    .show(ctx, |ui| {
                        let resp = ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
                            render_drive_properties(ui, state);
                            ui.add_space(4.0);
                            render_format_options(ui, state);
                            ui.add_space(4.0);
                            render_status_section(ui, state);
                        });
                        h = resp.response.rect.height();
                    });
            },
        );
        h
    }

    #[test]
    fn test_window_height_expands_for_advanced_options() {
        let mut state = AppState::new();
        state.show_advanced_drive_properties = false;
        state.show_advanced_format_options = false;
        let h_collapsed = measure_main_content_height(&mut state);

        state.show_advanced_drive_properties = true;
        let h_drive_adv = measure_main_content_height(&mut state);
        assert!(h_drive_adv > h_collapsed + 40.0);

        state.show_advanced_format_options = true;
        let h_both_adv = measure_main_content_height(&mut state);
        assert!(h_both_adv > h_drive_adv + 40.0);

        state.show_advanced_drive_properties = false;
        state.show_advanced_format_options = false;
        let h_recollapsed = measure_main_content_height(&mut state);
        assert!((h_recollapsed - h_collapsed).abs() < 1.0);
    }
}
