use egui::{Color32, Context, CornerRadius, RichText, Stroke, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_DANGER_RED};
use usb_x_core::types::BootSelectionMode;

pub fn show_confirm_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_confirm_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_confirm_viewport"),
        egui::ViewportBuilder::default()
            .with_title("WARNING: Confirm Disk Erase")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([460.0, 320.0])
            .with_min_inner_size([400.0, 260.0])
            .with_resizable(false),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_confirm_dialog = false;
                state.typed_confirm_input.clear();
            }

            let bg_color = panel_bg(is_dark);

            let requires_typed_erase = matches!(
                state.boot_mode,
                BootSelectionMode::RestoreDrive | BootSelectionMode::FlashTest
            );

            // Bottom actions
            egui::TopBottomPanel::bottom("confirm_bottom_toolbar")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 16, right: 16, top: 8, bottom: 12 }),
                )
                .show(ctx, |ui| {
                    let can_proceed = if requires_typed_erase {
                        state.typed_confirm_input.trim() == "ERASE"
                    } else {
                        true
                    };

                    ui.horizontal(|ui| {
                        let cancel_btn = egui::Button::new(
                            RichText::new("Cancel").size(12.0).color(text_primary(is_dark)),
                        )
                        .min_size(Vec2::new(76.0, 26.0));

                        if ui.add(cancel_btn).clicked() {
                            state.show_confirm_dialog = false;
                            state.typed_confirm_input.clear();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok_btn = egui::Button::new(
                                RichText::new("OK - Erase and Start")
                                    .color(Color32::WHITE)
                                    .strong()
                                    .size(12.0),
                            )
                            .min_size(Vec2::new(140.0, 26.0))
                            .fill(COLOR_DANGER_RED);

                            if ui.add_enabled(can_proceed, ok_btn).clicked() {
                                state.show_confirm_dialog = false;
                                state.typed_confirm_input.clear();
                                state.start_operation();
                            }
                        });
                    });
                });

            // Main body
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 16, right: 16, top: 14, bottom: 4 }),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("WARNING: ALL DATA ON THE SELECTED DEVICE WILL BE DESTROYED!")
                            .color(COLOR_DANGER_RED)
                            .strong()
                            .size(12.5),
                    );
                    ui.add_space(8.0);

                    if let Some(dev) = state.selected_device() {
                        let card_bg = if is_dark { Color32::from_rgb(26, 26, 26) } else { Color32::WHITE };
                        let card_border = if is_dark { Color32::from_rgb(50, 50, 50) } else { Color32::from_rgb(215, 215, 215) };

                        egui::Frame::new()
                            .fill(card_bg)
                            .stroke(Stroke::new(1.0f32, card_border))
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.label(RichText::new(format!("Target: {}", dev.display_name)).strong().color(text_primary(is_dark)));
                                ui.label(RichText::new(format!("Model: {} ({})", dev.model, dev.vendor)).size(11.0).color(text_secondary(is_dark)));
                                ui.label(RichText::new(format!("Capacity: {} [{}]", dev.formatted_size(), dev.device_path)).size(11.0).color(text_secondary(is_dark)));
                            });
                    }

                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("• Partition Scheme: {}", state.partition_scheme.label())).size(11.5).color(text_primary(is_dark)));
                    ui.label(RichText::new(format!("• Target System:    {}", state.target_system.label())).size(11.5).color(text_primary(is_dark)));
                    ui.label(RichText::new(format!("• File System:      {}", state.file_system.label())).size(11.5).color(text_primary(is_dark)));

                    if requires_typed_erase {
                        ui.add_space(10.0);
                        ui.label(RichText::new("To confirm destructive overwrite, type ERASE:").strong().color(COLOR_DANGER_RED));
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut state.typed_confirm_input)
                                .desired_width(120.0),
                        );
                    }
                });
        },
    );
}
