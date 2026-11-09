use egui::{Color32, CornerRadius, Rect, RichText, Stroke, StrokeKind, Ui, Vec2};
use crate::state::AppState;
use crate::ui::style::{
    get_system_accent, render_section_header, trough_bg,
    COLOR_DANGER_RED, COLOR_GREEN_PROGRESS,
};

pub fn render_status_section(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.is_dark_mode_active();
    let accent = get_system_accent();
    render_section_header(ui, "Status", is_dark);

    // 1. Rufus-Style Green Progress Bar
    let desired_width = ui.available_width();
    let bar_height = 25.0;
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(desired_width, bar_height), egui::Sense::hover());

    let painter = ui.painter();
    // Background Trough
    painter.rect_filled(rect, CornerRadius::same(3), trough_bg(is_dark));
    let trough_stroke_color = if is_dark { Color32::from_rgb(60, 60, 60) } else { Color32::from_rgb(185, 185, 185) };
    painter.rect_stroke(rect, CornerRadius::same(3), Stroke::new(1.0f32, trough_stroke_color), StrokeKind::Inside);

    // Filled Green Region
    let fraction = state.progress_fraction.clamp(0.0, 1.0);
    if fraction > 0.0 || state.is_running {
        let fill_width = rect.width() * fraction;
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, bar_height));
        painter.rect_filled(fill_rect, CornerRadius::same(3), COLOR_GREEN_PROGRESS);
    }

    // Centered Progress Text: strictly floating number with 1 decimal place (e.g. "Writing: 3.8%")
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
            let s = state.current_stage_text.trim();
            if s.starts_with("Formatting") {
                "Formatting partition...".to_string()
            } else if s.starts_with("Partitioning") || s.starts_with("Creating") {
                "Partitioning drive...".to_string()
            } else if s.starts_with("Mounting") {
                "Mounting ISO...".to_string()
            } else if s.starts_with("Flushing") || s.starts_with("Finalizing") {
                "Finalizing buffers...".to_string()
            } else if s.eq_ignore_ascii_case("writing") || s == "READY" {
                "Writing: 0.0%".to_string()
            } else if s.len() > 32 {
                format!("{}...", &s[..29])
            } else {
                s.to_string()
            }
        } else {
            "Writing: 0.0%".to_string()
        }
    } else {
        state.current_stage_text.clone()
    };

    let display_text = if raw_text.len() > 36 {
        format!("{}...", &raw_text[..33])
    } else {
        raw_text
    };

    let text_color = if fraction > 0.4 || (state.is_running && fraction > 0.15) {
        Color32::WHITE
    } else if is_dark {
        Color32::from_rgb(230, 230, 230)
    } else {
        Color32::from_rgb(30, 30, 30)
    };

    let text_pos = rect.center();
    painter.with_clip_rect(rect).text(
        text_pos,
        egui::Align2::CENTER_CENTER,
        display_text,
        egui::FontId::proportional(12.0),
        text_color,
    );

    ui.add_space(6.0);

    // 2. Action Toolbar: Icons on Left, START / (CLOSE or CANCEL) on Right
    ui.horizontal(|ui| {
        let log_btn_text = if state.show_log_window { "Hide Log" } else { "Log" };
        if ui.button(RichText::new(log_btn_text).size(11.5)).on_hover_text("Toggle Detailed Event Log Window").clicked() {
            state.show_log_window = !state.show_log_window;
        }

        let hash_btn_text = if state.show_hash_dialog { "Hide Checksums" } else { "Checksums" };
        if ui.button(RichText::new(hash_btn_text).size(11.5)).on_hover_text("View Image SHA-256, MD5, and CRC-32 Checksums").clicked() {
            state.show_hash_dialog = !state.show_hash_dialog;
        }

        // Space to push buttons to right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Close / Cancel Button:
            // When flashing is running -> CANCEL button (aborts write, red outline/text)
            // When drive is ready / completed -> CLOSE button (closes application, matching Rufus)
            if state.is_running {
                let cancel_btn = egui::Button::new(
                    RichText::new("CANCEL")
                        .strong()
                        .size(11.5)
                        .color(COLOR_DANGER_RED),
                )
                .min_size(Vec2::new(96.0, 26.0))
                .stroke(Stroke::new(1.5f32, COLOR_DANGER_RED));

                if ui.add(cancel_btn).on_hover_text("Abort ongoing write operation").clicked() {
                    state.cancel_operation();
                }
            } else {
                let close_btn = egui::Button::new(
                    RichText::new("CLOSE")
                        .strong()
                        .size(11.5)
                        .color(crate::ui::style::text_primary(is_dark)),
                )
                .min_size(Vec2::new(96.0, 26.0));

                if ui.add(close_btn).on_hover_text("Close Pigeon").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            ui.add_space(6.0);

            // START Button (Universal ASCII, guaranteed visible text in all themes & states)
            let can_start = !state.is_running && state.selected_device().map(|d| !d.system_risk.is_destructive_blocked()).unwrap_or(false);
            let start_btn = if can_start {
                let text_clr = if is_dark { Color32::from_rgb(100, 185, 255) } else { Color32::from_rgb(0, 51, 153) };
                egui::Button::new(
                    RichText::new("START")
                        .strong()
                        .size(11.5)
                        .color(text_clr),
                )
                .min_size(Vec2::new(96.0, 26.0))
                .stroke(Stroke::new(1.5f32, accent))
            } else {
                let disabled_clr = if is_dark { Color32::from_rgb(110, 110, 110) } else { Color32::from_rgb(140, 140, 140) };
                egui::Button::new(
                    RichText::new("START")
                        .strong()
                        .size(11.5)
                        .color(disabled_clr),
                )
                .min_size(Vec2::new(96.0, 26.0))
            };

            if ui.add_enabled(can_start, start_btn).clicked() {
                if state.is_windows_image() {
                    state.show_win11_tweaks_dialog = true;
                } else {
                    state.typed_confirm_input.clear();
                    state.show_confirm_dialog = true;
                }
            }
        });
    });
}

