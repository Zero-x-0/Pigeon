use egui::{Color32, Context, RichText, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_ACCENT_BLUE};

pub fn show_win11_tweaks_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_win11_tweaks_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_win11_tweaks_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Windows User Experience")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([480.0, 315.0])
            .with_min_inner_size([420.0, 270.0])
            .with_resizable(false),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_win11_tweaks_dialog = false;
            }

            let bg_color = panel_bg(is_dark);

            // Bottom action panel
            egui::TopBottomPanel::bottom("win_tweaks_bottom_toolbar")
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin { left: 16, right: 16, top: 8, bottom: 12 }),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Tweaks embedded in autounattend.xml")
                                .size(10.5)
                                .color(text_secondary(is_dark)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let ok_btn = egui::Button::new(
                                RichText::new("OK").strong().size(12.0).color(Color32::WHITE),
                            )
                            .min_size(Vec2::new(76.0, 26.0))
                            .fill(COLOR_ACCENT_BLUE);

                            if ui.add(ok_btn).clicked() {
                                state.show_win11_tweaks_dialog = false;
                                state.typed_confirm_input.clear();
                                state.show_confirm_dialog = true;
                            }

                            ui.add_space(8.0);

                            let cancel_btn = egui::Button::new(
                                RichText::new("Cancel").size(12.0).color(text_primary(is_dark)),
                            )
                            .min_size(Vec2::new(76.0, 26.0));

                            if ui.add(cancel_btn).clicked() {
                                state.show_win11_tweaks_dialog = false;
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
                        RichText::new("Customize Windows installation media:")
                            .strong()
                            .size(13.0)
                            .color(text_primary(is_dark)),
                    );
                    ui.add_space(10.0);

                    ui.checkbox(
                        &mut state.win11_tweaks.bypass_tpm_secure_boot_ram,
                        "Remove requirement for 4GB+ RAM, Secure Boot and TPM 2.0",
                    );
                    ui.add_space(5.0);

                    ui.checkbox(
                        &mut state.win11_tweaks.bypass_online_account,
                        "Remove requirement for an online Microsoft account (BypassNRO)",
                    );
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut state.win11_tweaks.create_local_account,
                            "Create a local account with username:",
                        );
                        ui.add_enabled(
                            state.win11_tweaks.create_local_account,
                            egui::TextEdit::singleline(&mut state.win11_tweaks.local_username).desired_width(120.0),
                        );
                    });
                    ui.add_space(5.0);

                    ui.checkbox(
                        &mut state.win11_tweaks.disable_telemetry_privacy,
                        "Disable data collection (Skip privacy questions)",
                    );
                    ui.add_space(5.0);

                    ui.checkbox(
                        &mut state.win11_tweaks.disable_bitlocker_device_encryption,
                        "Disable BitLocker automatic device encryption",
                    );
                });
        },
    );
}
