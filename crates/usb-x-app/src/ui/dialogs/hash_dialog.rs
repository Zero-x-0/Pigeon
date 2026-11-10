use egui::{Color32, Context, CornerRadius, RichText, Stroke, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_DANGER_RED, COLOR_SUCCESS_GREEN};

pub fn show_hash_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_hash_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_hash_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Image Checksums & Verification")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([720.0, 310.0])
            .with_min_inner_size([680.0, 270.0])
            .with_resizable(false),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_hash_dialog = false;
            }

            let bg_color = panel_bg(is_dark);

            // Bottom toolbar
            egui::TopBottomPanel::bottom("hash_window_bottom_toolbar")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 16, right: 16, top: 6, bottom: 10 }),
                )
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_clr = text_primary(is_dark);
                        let close_btn = egui::Button::new(RichText::new("Close").size(12.0).color(btn_clr))
                            .min_size(Vec2::new(76.0, 24.0));
                        if ui.add(close_btn).clicked() {
                            state.show_hash_dialog = false;
                        }
                    });
                });

            // Main body
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 16, right: 16, top: 12, bottom: 4 }),
                )
                .show(ctx, |ui| {
                    if state.image_path.trim().is_empty() {
                        ui.label(
                            RichText::new("No Disk Image Selected")
                                .size(13.0)
                                .strong()
                                .color(text_primary(is_dark)),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Select an ISO or disk image file to compute and verify SHA-256, MD5, and CRC-32 checksums.")
                                .size(11.5)
                                .color(text_secondary(is_dark)),
                        );
                        ui.add_space(10.0);

                        let browse_btn = egui::Button::new(
                            RichText::new("Select Image File...")
                                .strong()
                                .size(12.0)
                                .color(text_primary(is_dark)),
                        )
                        .min_size(Vec2::new(150.0, 26.0));

                        if ui.add(browse_btn).clicked() {
                            state.trigger_file_dialog();
                        }
                    } else {
                        let filename = std::path::Path::new(&state.image_path)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| state.image_path.clone());

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Image:").strong().size(12.0).color(text_primary(is_dark)));
                            ui.label(
                                RichText::new(filename)
                                    .size(12.0)
                                    .strong()
                                    .color(if is_dark { Color32::from_rgb(100, 185, 255) } else { Color32::from_rgb(0, 51, 153) }),
                            )
                            .on_hover_text(&state.image_path);
                        });
                        ui.add_space(8.0);

                        if state.is_calculating_hash {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(RichText::new("Computing SHA-256, MD5, and CRC-32 in background...").strong());
                            });
                            ui.add_space(6.0);
                            let progress_bar = egui::ProgressBar::new(state.hash_progress_fraction)
                                .text(&state.hash_progress_text)
                                .animate(true);
                            ui.add(progress_bar);
                        } else if let Some(sums) = &state.calculated_checksums {
                            let card_bg = if is_dark { Color32::from_rgb(26, 26, 26) } else { Color32::WHITE };
                            let card_border = if is_dark { Color32::from_rgb(50, 50, 50) } else { Color32::from_rgb(215, 215, 215) };

                            egui::Frame::new()
                                .fill(card_bg)
                                .stroke(Stroke::new(1.0f32, card_border))
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(egui::Margin::symmetric(12, 10))
                                .show(ui, |ui| {
                                    egui::Grid::new("checksum_table_grid")
                                        .num_columns(3)
                                        .spacing(Vec2::new(14.0, 7.0))
                                        .show(ui, |ui| {
                                            // 1. MD5
                                            ui.label(RichText::new("MD5:").monospace().strong().size(11.5).color(text_secondary(is_dark)));
                                            ui.add(egui::Label::new(RichText::new(&sums.md5).monospace().size(11.5).color(text_primary(is_dark))).truncate());
                                            let copy_md5 = egui::Button::new(RichText::new("Copy").size(11.0)).min_size(Vec2::new(52.0, 22.0));
                                            if ui.add(copy_md5).on_hover_text("Copy MD5 checksum to clipboard").clicked() {
                                                if let Ok(mut clip) = arboard::Clipboard::new() {
                                                    let _ = clip.set_text(&sums.md5);
                                                }
                                            }
                                            ui.end_row();

                                            // 2. SHA-256
                                            ui.label(RichText::new("SHA-256:").monospace().strong().size(11.5).color(text_secondary(is_dark)));
                                            ui.add(egui::Label::new(RichText::new(&sums.sha256).monospace().size(11.5).color(text_primary(is_dark))).truncate());
                                            let copy_sha = egui::Button::new(RichText::new("Copy").size(11.0)).min_size(Vec2::new(52.0, 22.0));
                                            if ui.add(copy_sha).on_hover_text("Copy SHA-256 checksum to clipboard").clicked() {
                                                if let Ok(mut clip) = arboard::Clipboard::new() {
                                                    let _ = clip.set_text(&sums.sha256);
                                                }
                                            }
                                            ui.end_row();

                                            // 3. CRC-32
                                            ui.label(RichText::new("CRC-32:").monospace().strong().size(11.5).color(text_secondary(is_dark)));
                                            ui.add(egui::Label::new(RichText::new(&sums.crc32).monospace().size(11.5).color(text_primary(is_dark))).truncate());
                                            let copy_crc = egui::Button::new(RichText::new("Copy").size(11.0)).min_size(Vec2::new(52.0, 22.0));
                                            if ui.add(copy_crc).on_hover_text("Copy CRC-32 checksum to clipboard").clicked() {
                                                if let Ok(mut clip) = arboard::Clipboard::new() {
                                                    let _ = clip.set_text(&sums.crc32);
                                                }
                                            }
                                            ui.end_row();
                                        });
                                });

                            ui.add_space(8.0);
                            ui.label(RichText::new("Paste expected checksum to verify:").strong().size(11.5).color(text_primary(is_dark)));
                            ui.add_space(2.0);
                            ui.add(
                                egui::TextEdit::singleline(&mut state.expected_hash_input)
                                    .desired_width(ui.available_width())
                                    .hint_text("Paste MD5, SHA-256, or CRC-32 here..."),
                            );

                            let raw_input = state.expected_hash_input.trim();
                            if !raw_input.is_empty() {
                                ui.add_space(6.0);
                                let sanitized = raw_input
                                    .trim_matches('"')
                                    .trim_matches('\'')
                                    .replace(' ', "")
                                    .replace('-', "")
                                    .replace(':', "")
                                    .to_lowercase();

                                if sanitized == sums.sha256.to_lowercase() {
                                    ui.label(
                                        RichText::new("✓ MATCH (SHA-256): Verified authentic image checksum.")
                                            .color(COLOR_SUCCESS_GREEN)
                                            .strong()
                                            .size(12.0),
                                    );
                                } else if sanitized == sums.md5.to_lowercase() {
                                    ui.label(
                                        RichText::new("✓ MATCH (MD5): Verified authentic image checksum.")
                                            .color(COLOR_SUCCESS_GREEN)
                                            .strong()
                                            .size(12.0),
                                        );
                                } else if sanitized == sums.crc32.to_lowercase() {
                                    ui.label(
                                        RichText::new("✓ MATCH (CRC-32): Verified authentic image checksum.")
                                            .color(COLOR_SUCCESS_GREEN)
                                            .strong()
                                            .size(12.0),
                                        );
                                } else {
                                    ui.label(
                                        RichText::new("⚠ MISMATCH: Checksum does not match any computed digest.")
                                            .color(COLOR_DANGER_RED)
                                            .strong()
                                            .size(12.0),
                                    );
                                }
                            }
                        } else {
                            if !state.is_calculating_hash {
                                state.start_hash_calculation();
                            }
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(RichText::new("Computing SHA-256, MD5, and CRC-32 in background...").strong());
                            });
                            ui.add_space(6.0);
                            let progress_bar = egui::ProgressBar::new(state.hash_progress_fraction)
                                .text(&state.hash_progress_text)
                                .animate(true);
                            ui.add(progress_bar);
                        }
                    }
                });
        },
    );
}
