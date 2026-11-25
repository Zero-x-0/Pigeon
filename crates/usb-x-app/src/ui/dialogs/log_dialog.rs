use egui::{Color32, Context, CornerRadius, RichText, ScrollArea, Stroke, Vec2};
use crate::state::AppState;

pub fn show_log_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_log_window {
        return;
    }

    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_log_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Log")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([500.0, 560.0])
            .with_min_inner_size([360.0, 240.0])
            .with_resizable(true),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_log_window = false;
            }

            let bg_color = crate::ui::style::panel_bg(is_dark);

            // 1. Bottom Toolbar Panel: Guaranteed proper margins on bottom and right
            egui::TopBottomPanel::bottom("log_window_bottom_toolbar")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 8, bottom: 12 }),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let count_str = format!("{} lines logged", state.log_messages.len());
                        ui.label(
                            RichText::new(count_str)
                                .size(11.0)
                                .color(if is_dark {
                                    Color32::from_rgb(130, 130, 130)
                                } else {
                                    Color32::from_rgb(110, 110, 110)
                                }),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_clr = crate::ui::style::text_primary(is_dark);

                            // Close button
                            let close_btn = egui::Button::new(RichText::new("Close").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(72.0, 26.0));
                            if ui.add(close_btn).clicked() {
                                state.show_log_window = false;
                            }

                            ui.add_space(6.0);

                            // Save button (Asynchronous to prevent freezing UI thread and GNOME 'Not Responding' modal)
                            let save_btn = egui::Button::new(RichText::new("Save").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(72.0, 26.0));
                            if ui.add(save_btn).on_hover_text("Save log contents to a file").clicked() {
                                let messages = state.log_messages.clone();
                                std::thread::spawn(move || {
                                    if let Some(dest_path) = usb_x_platform::save_file_dialog(
                                        "Save Log File",
                                        "pigeon.log",
                                        "Log Files (*.log)",
                                        "*.log",
                                    ) {
                                        let text = messages.join("\n");
                                        let _ = std::fs::write(&dest_path, text);
                                    } else if let Ok(home) = std::env::var("HOME") {
                                        let fallback = std::path::Path::new(&home).join("pigeon.log");
                                        let text = messages.join("\n");
                                        let _ = std::fs::write(&fallback, text);
                                    }
                                });
                            }

                            ui.add_space(6.0);

                            // Copy button
                            let copy_btn = egui::Button::new(RichText::new("Copy").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(72.0, 26.0));
                            if ui.add(copy_btn).on_hover_text("Copy log contents to clipboard").clicked() {
                                let text = state.log_messages.join("\n");
                                std::thread::spawn(move || {
                                    if let Ok(mut cb) = arboard::Clipboard::new() {
                                        let _ = cb.set_text(text);
                                    }
                                });
                            }

                            ui.add_space(6.0);

                            // Clear button
                            let clear_btn = egui::Button::new(RichText::new("Clear").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(72.0, 26.0));
                            if ui.add(clear_btn).on_hover_text("Clear all log messages").clicked() {
                                state.log_messages.clear();
                            }
                        });
                    });
                });

            // 2. Main Terminal Console Panel: Zero-wrapping virtualized painter rendering
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 12, bottom: 4 }),
                )
                .show(ctx, |ui| {
                    let console_frame = egui::Frame::new()
                        .fill(Color32::from_rgb(18, 18, 18))
                        .stroke(Stroke::new(1.0f32, Color32::from_rgb(50, 50, 50)))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(10, 8));

                    console_frame.show(ui, |ui| {
                        let font_id = egui::FontId::monospace(11.5);
                        let row_height = 16.0;
                        let char_w = ui.fonts(|f| f.glyph_width(&font_id, 'M')).max(6.8);
                        let max_chars = state.log_messages.iter().map(|s| s.len()).max().unwrap_or(60);
                        let avail_w = ui.available_width().max(1.0);
                        let row_width = avail_w.max(max_chars as f32 * char_w + 16.0);

                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        ScrollArea::both()
                            .id_salt("pigeon_log_scroll")
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show_rows(ui, row_height, state.log_messages.len(), |ui, row_range| {
                                for idx in row_range {
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::new(row_width, row_height),
                                        egui::Sense::hover(),
                                    );
                                    if let Some(line) = state.log_messages.get(idx) {
                                        let text_color = if line.starts_with("ERROR:") {
                                            Color32::from_rgb(245, 115, 115)
                                        } else if line.starts_with("WARNING:") {
                                            Color32::from_rgb(245, 198, 92)
                                        } else {
                                            Color32::from_rgb(220, 220, 220)
                                        };
                                        ui.painter().text(
                                            egui::pos2(rect.left() + 2.0, rect.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            line,
                                            font_id.clone(),
                                            text_color,
                                        );
                                    }
                                }
                            });
                    });
                });
        },
    );
}
