use egui::{Color32, Context, Key, RichText, Stroke, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_ACCENT_BLUE};

pub fn show_hash_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_hash_dialog {
        return;
    }

    // Support <Alt>-<H> inside Checksums or main window to trigger SHA-512 computation
    if ctx.input(|i| i.modifiers.alt && i.key_pressed(Key::H)) && !state.include_sha512 {
        state.start_hash_calculation_ex(true);
    }

    let is_dark = state.is_dark_mode_active();
    let filename = if state.image_path.trim().is_empty() {
        "Checksums".to_string()
    } else {
        std::path::Path::new(&state.image_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Checksums".to_string())
    };

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_hash_viewport"),
        egui::ViewportBuilder::default()
            .with_title(filename)
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([445.0, 290.0])
            .with_min_inner_size([445.0, 290.0])
            .with_max_inner_size([445.0, 290.0])
            .with_maximize_button(false)
            .with_resizable(false),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_hash_dialog = false;
            }

            let bg_color = panel_bg(is_dark);
            let field_bg = if is_dark { Color32::from_rgb(28, 28, 28) } else { Color32::from_rgb(245, 245, 245) };
            let field_stroke = if is_dark { Color32::from_rgb(65, 65, 65) } else { Color32::from_rgb(205, 205, 205) };

            egui::TopBottomPanel::bottom("hash_window_bottom_toolbar")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 6, bottom: 12 }),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if !state.include_sha512 && !state.image_path.is_empty() && !state.is_calculating_hash {
                            if ui.small_button("Compute SHA512").clicked() {
                                state.start_hash_calculation_ex(true);
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok_btn = egui::Button::new(RichText::new("OK").size(12.0).color(text_primary(is_dark)))
                                .min_size(Vec2::new(80.0, 25.0))
                                .stroke(Stroke::new(1.2_f32, COLOR_ACCENT_BLUE));
                            if ui.add(ok_btn).clicked() {
                                state.show_hash_dialog = false;
                            }
                        });
                    });
                });

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 14, right: 14, top: 14, bottom: 4 }),
                )
                .show(ctx, |ui| {
                    if state.image_path.trim().is_empty() {
                        ui.label(RichText::new("No disk image selected.").size(12.5).color(text_primary(is_dark)));
                        return;
                    }

                    if state.is_calculating_hash {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Computing checksums...").size(12.0));
                        });
                        ui.add_space(6.0);
                        ui.add(
                            egui::ProgressBar::new(state.hash_progress_fraction)
                                .text(&state.hash_progress_text)
                                .animate(true),
                        );
                    } else if let Some(sums) = &state.calculated_checksums {
                        let label_w = 62.0;
                        let box_w = ui.available_width() - label_w - 10.0;

                        let sha256_wrapped = if sums.sha256.len() == 64 {
                            format!("{}\n{}", &sums.sha256[..32], &sums.sha256[32..])
                        } else {
                            sums.sha256.clone()
                        };

                        let sha512_display = if let Some(ref s512) = sums.sha512 {
                            s512.clone()
                        } else {
                            "Use <Alt>-<H> (in the main application\nwindow) to enable.".to_string()
                        };

                        let render_hash_row = |ui: &mut egui::Ui, label: &str, val: &str, lines: usize| {
                            ui.horizontal_top(|ui| {
                                ui.allocate_ui(Vec2::new(label_w, 20.0), |ui| {
                                    ui.label(RichText::new(label).size(12.0).color(text_primary(is_dark)));
                                });
                                let mut ro = val.to_string();
                                egui::Frame::new()
                                    .fill(field_bg)
                                    .stroke(Stroke::new(1.0_f32, field_stroke))
                                    .inner_margin(egui::Margin::symmetric(6, 4))
                                    .show(ui, |ui| {
                                        ui.set_width(box_w);
                                        if lines <= 1 {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut ro)
                                                    .font(egui::TextStyle::Monospace)
                                                    .frame(false)
                                                    .desired_width(box_w),
                                            );
                                        } else {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut ro)
                                                    .font(egui::TextStyle::Monospace)
                                                    .frame(false)
                                                    .desired_rows(lines)
                                                    .desired_width(box_w),
                                            );
                                        }
                                    });
                            });
                            ui.add_space(6.0);
                        };

                        render_hash_row(ui, "MD5:", &sums.md5, 1);
                        render_hash_row(ui, "SHA1:", &sums.sha1, 1);
                        render_hash_row(ui, "SHA256:", &sha256_wrapped, 2);
                        render_hash_row(ui, "SHA512:", &sha512_display, if sums.sha512.is_some() { 3 } else { 2 });
                    } else {
                        ui.label(RichText::new("Click Refresh or re-select ISO to compute hashes.").color(text_secondary(is_dark)));
                    }
                });
        },
    );
}
