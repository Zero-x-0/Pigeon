use egui::{Color32, CornerRadius, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};
use crate::state::AppState;
use crate::ui::style::{
    get_system_accent, icon_button, render_section_header, text_primary, text_secondary, trough_bg,
    ToolbarIcon, COLOR_DANGER_RED, COLOR_GREEN_PROGRESS,
};
use usb_x_core::types::BootSelectionMode;

pub fn render_status_section(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.is_dark_mode_active();
    let accent = get_system_accent();
    render_section_header(ui, "Status", is_dark);

    // 1. Rufus-Style Full-Width Status / Progress Bar
    let desired_width = ui.available_width();
    let bar_height = 22.0;
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(desired_width, bar_height), egui::Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(1), trough_bg(is_dark));
    let trough_stroke_color = if is_dark { Color32::from_rgb(70, 70, 70) } else { Color32::from_rgb(185, 185, 185) };
    painter.rect_stroke(rect, CornerRadius::same(1), Stroke::new(1.0_f32, trough_stroke_color), StrokeKind::Inside);

    let fraction = state.progress_fraction.clamp(0.0, 1.0);
    if fraction > 0.0 || state.is_running {
        let fill_width = rect.width() * fraction;
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, bar_height));
        painter.rect_filled(fill_rect, CornerRadius::same(1), COLOR_GREEN_PROGRESS);
    }

    let raw_text = if state.is_running {
        if state.progress_fraction > 0.0 {
            let base_stage = if let Some((prefix, _)) = state.current_stage_text.split_once(':') {
                prefix.trim()
            } else {
                state.current_stage_text.trim()
            };
            let stage = if base_stage.is_empty() || base_stage == "READY" || base_stage.eq_ignore_ascii_case("writing") {
                "Writing"
            } else {
                base_stage
            };
            format!("{}: {:.1}%", stage, state.progress_fraction * 100.0)
        } else if !state.current_stage_text.is_empty() {
            state.current_stage_text.trim().to_string()
        } else {
            "Writing: 0.0%".to_string()
        }
    } else {
        state.current_stage_text.clone()
    };

    let display_text = if raw_text.len() > 48 {
        format!("{}...", &raw_text[..45])
    } else {
        raw_text
    };

    let text_color = if fraction > 0.45 || (state.is_running && fraction > 0.15) {
        Color32::WHITE
    } else if is_dark {
        Color32::from_rgb(235, 235, 235)
    } else {
        Color32::from_rgb(20, 20, 20)
    };

    painter.with_clip_rect(rect).text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        display_text,
        egui::FontId::proportional(11.5),
        text_color,
    );

    ui.add_space(7.0);

    // 2. Action Toolbar: 4 Icon Buttons on Left matching Rufus, [ START ] [ CLOSE / CANCEL ] on Right
    state.status_bar_hint = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // 1) Globe: Change application language popup
        let lang_popup_id = ui.make_persistent_id("rufus_language_popup");
        let globe_resp = icon_button(ui, ToolbarIcon::Globe, is_dark, true)
            .on_hover_text("Change the application language");
        if globe_resp.hovered() {
            state.status_bar_hint = Some("Change the application language".to_string());
        }
        if globe_resp.clicked() {
            ui.memory_mut(|m| m.toggle_popup(lang_popup_id));
        }
        egui::popup_below_widget(ui, lang_popup_id, &globe_resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
            ui.set_min_width(145.0);
            for lang in ["English (US)", "English (UK)", "System Locale"] {
                if ui.selectable_label(state.app_language == lang, lang).clicked() {
                    state.app_language = lang.to_string();
                }
            }
        });

        // 2) Info (i): About Pigeon & Drive Information
        let info_resp = icon_button(ui, ToolbarIcon::InfoCircle, is_dark, true)
            .on_hover_text("About Pigeon");
        if info_resp.hovered() {
            state.status_bar_hint = Some("About Pigeon".to_string());
        }
        if info_resp.clicked() {
            state.show_about_dialog = true;
        }

        // 3) Sliders: Settings
        let cfg_resp = icon_button(ui, ToolbarIcon::SettingsSliders, is_dark, true)
            .on_hover_text("Settings & Preferences");
        if cfg_resp.hovered() {
            state.status_bar_hint = Some("Show application settings".to_string());
        }
        if cfg_resp.clicked() {
            state.show_settings_dialog = !state.show_settings_dialog;
        }

        // 4) Log Table: Show the log (exact tooltip from Image 1)
        let log_resp = icon_button(ui, ToolbarIcon::LogTable, is_dark, true)
            .on_hover_text("Show the log");
        if log_resp.hovered() {
            state.status_bar_hint = Some("Show the log".to_string());
        }
        if log_resp.clicked() {
            state.show_log_window = !state.show_log_window;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.is_running {
                let cancel_btn = egui::Button::new(
                    RichText::new("CANCEL")
                        .size(11.5)
                        .color(COLOR_DANGER_RED),
                )
                .min_size(Vec2::new(88.0_f32, 24.0_f32))
                .stroke(Stroke::new(1.0_f32, COLOR_DANGER_RED));

                if ui.add(cancel_btn).on_hover_text("Abort ongoing operation").clicked() {
                    state.cancel_operation();
                }
            } else {
                let close_btn = egui::Button::new(
                    RichText::new("CLOSE")
                        .size(11.5)
                        .color(text_primary(is_dark)),
                )
                .min_size(Vec2::new(88.0_f32, 24.0_f32));

                if ui.add(close_btn).on_hover_text("Close Pigeon").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            ui.add_space(6.0);

            let has_valid_source = state.boot_mode != BootSelectionMode::DiskImage || !state.image_path.is_empty();
            let can_start = !state.is_running
                && has_valid_source
                && state
                    .selected_device()
                    .map(|d| !d.system_risk.is_destructive_blocked())
                    .unwrap_or(false);

            let start_btn = if can_start {
                egui::Button::new(
                    RichText::new("START")
                        .size(11.5)
                        .color(text_primary(is_dark)),
                )
                .min_size(Vec2::new(88.0_f32, 24.0_f32))
                .stroke(Stroke::new(1.2_f32, accent))
            } else {
                egui::Button::new(
                    RichText::new("START")
                        .size(11.5)
                        .color(text_secondary(is_dark)),
                )
                .min_size(Vec2::new(88.0_f32, 24.0_f32))
            };

            if ui.add_enabled(can_start, start_btn).clicked() {
                state.typed_confirm_input.clear();
                state.show_confirm_dialog = true;
            }
        });
    });
}
