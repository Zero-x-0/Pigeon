use crate::state::AppState;
use crate::ui::*;
use eframe::egui;

pub struct UsbXApp {
    state: AppState,
    last_applied_theme: Option<(bool, egui::Color32)>,
}

impl UsbXApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let is_dark = cc.egui_ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or_else(usb_x_platform::is_system_dark_mode);
        let accent = crate::ui::style::get_system_accent();
        crate::ui::style::apply_theme(&cc.egui_ctx, is_dark, accent);

        let mut state = AppState::new();
        state.cached_is_dark = is_dark;
        state.cached_accent = accent;

        Self {
            state,
            last_applied_theme: Some((is_dark, accent)),
        }
    }
}

impl eframe::App for UsbXApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background worker event updates
        self.state.update_events();
        if self.state.is_running {
            ctx.request_repaint();
        }

        // Throttle theme registry checks and apply theme cleanly only when changed
        let egui_hint = ctx.system_theme().map(|t| t == egui::Theme::Dark);
        self.state.refresh_theme_if_needed(egui_hint);
        let is_dark = self.state.is_dark();
        let accent = self.state.cached_accent;

        if self.last_applied_theme != Some((is_dark, accent)) {
            crate::ui::style::apply_theme(ctx, is_dark, accent);
            self.last_applied_theme = Some((is_dark, accent));
            ctx.request_repaint();
        }

        // Support drag-and-drop of ISO and disk images onto the window
        if !self.state.is_running {
            ctx.input(|i| {
                if let Some(file) = i.raw.dropped_files.first() {
                    if let Some(ref path) = file.path {
                        let path_str = path.to_string_lossy().to_string();
                        let lower = path_str.to_ascii_lowercase();
                        if lower.ends_with(".iso")
                            || lower.ends_with(".img")
                            || lower.ends_with(".bin")
                            || lower.ends_with(".raw")
                            || lower.ends_with(".vhd")
                        {
                            self.state.load_image_file(path_str);
                        }
                    }
                }
            });
        }

        let panel_frame = egui::Frame::NONE
            .fill(crate::ui::style::panel_bg(is_dark))
            .inner_margin(egui::Margin::symmetric(10, 8));

        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 3.5);

                    // When writing is running, lock all options to avoid accidental errors
                    ui.add_enabled_ui(!self.state.is_running, |ui| {
                        // Section 1: Drive Properties
                        render_drive_properties(ui, &mut self.state);

                        ui.add_space(4.0);

                        // Section 2: Format Options
                        render_format_options(ui, &mut self.state);
                    });

                    ui.add_space(4.0);

                    // Section 3: Status, Green Progress Bar, Toolbar & Bottom Status Bar
                    render_status_section(ui, &mut self.state);
                });
        });

        // Secondary Free Windows (Immediate Desktop Viewports)
        show_confirm_dialog(ctx, &mut self.state);
        show_win11_tweaks_dialog(ctx, &mut self.state);
        show_hash_dialog(ctx, &mut self.state);
        show_log_dialog(ctx, &mut self.state);
        show_file_browser_dialog(ctx, &mut self.state);
    }
}
