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
                                .min_size(Vec2::new(76.0, 26.0));
                            if ui.add(close_btn).clicked() {
                                state.show_log_window = false;
                            }

                            ui.add_space(8.0);

                            // Save button
                            let save_btn = egui::Button::new(RichText::new("Save").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(76.0, 26.0));
                            if ui.add(save_btn).on_hover_text("Save log contents to a file").clicked() {
                                if let Some(dest_path) = usb_x_platform::save_file_dialog(
                                    "Save Log File",
                                    "pigeon.log",
                                    "Log Files (*.log)",
                                    "*.log",
                                ) {
                                    let text = state.log_messages.join("\r\n");
                                    let _ = std::fs::write(&dest_path, text);
                                }
                            }

                            ui.add_space(8.0);

                            // Clear button
                            let clear_btn = egui::Button::new(RichText::new("Clear").size(12.0).color(btn_clr))
                                .min_size(Vec2::new(76.0, 26.0));
                            if ui.add(clear_btn).on_hover_text("Clear all log messages").clicked() {
                                state.log_messages.clear();
                            }
                        });
                    });
                });

            // 2. Main Terminal Console Panel: Fills all remaining space above the toolbar
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 12, bottom: 4 }),
                )
                .show(ctx, |ui| {
                    let console_frame = egui::Frame::new()
                        .fill(Color32::from_rgb(18, 18, 18))
                        .stroke(Stroke::new(1.0f32, Color32::from_rgb(45, 45, 45)))
                        .corner_radius(CornerRadius::same(2))
                        .inner_margin(egui::Margin::symmetric(10, 8));

                    console_frame.show(ui, |ui| {
                        ScrollArea::both()
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);
                                for line in &state.log_messages {
                                    ui.label(
                                        RichText::new(line)
                                            .monospace()
                                            .size(11.5)
                                            .color(Color32::from_rgb(220, 220, 220)),
                                    );
                                }
                            });
                    });
                });
        },
    );
}
