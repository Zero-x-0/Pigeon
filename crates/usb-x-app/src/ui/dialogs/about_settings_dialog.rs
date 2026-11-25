use egui::{Context, RichText, Stroke, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_ACCENT_BLUE};
use usb_x_core::types::ThemePreference;

pub fn show_about_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_about_dialog {
        return;
    }
    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_about_viewport"),
        egui::ViewportBuilder::default()
            .with_title("About Pigeon")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([400.0, 260.0])
            .with_maximize_button(false)
            .with_resizable(false),
        |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_about_dialog = false;
            }
            let bg = panel_bg(is_dark);
            egui::TopBottomPanel::bottom("about_bottom")
                .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(14, 10)))
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = egui::Button::new(RichText::new("OK").size(12.0).color(text_primary(is_dark)))
                            .min_size(Vec2::new(78.0_f32, 24.0_f32))
                            .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_BLUE));
                        if ui.add(ok).clicked() {
                            state.show_about_dialog = false;
                        }
                    });
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(16, 14)))
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(format!("Pigeon {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::ARCH))
                            .strong()
                            .size(16.0)
                            .color(text_primary(is_dark)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Fast, lightweight bootable USB & disk imaging utility.")
                            .size(12.0)
                            .color(text_secondary(is_dark)),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("License: GNU GPL v3.0 or later").size(11.5));
                    ui.label(RichText::new(format!("Host OS: {}", usb_x_platform::get_windows_version_string())).size(11.5));
                    ui.label(RichText::new(format!("Privileges: {}", if state.is_admin { "Elevated (Root/Admin)" } else { "Standard User" })).size(11.5));
                    ui.add_space(8.0);
                    if let Some(dev) = state.selected_device() {
                        ui.label(RichText::new("Selected Device:").strong().size(12.0));
                        ui.label(RichText::new(format!("  {} ({})", dev.summary(), dev.device_path)).size(11.5));
                        ui.label(RichText::new(format!("  Capacity: {} bytes", dev.size_bytes)).size(11.5));
                    }
                });
        },
    );
}

pub fn show_settings_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_settings_dialog {
        return;
    }
    let is_dark = state.is_dark_mode_active();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_settings_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Settings")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([370.0, 235.0])
            .with_maximize_button(false)
            .with_resizable(false),
        |ctx, _| {
            if ctx.input(|i| i.viewport().close_requested()) {
                state.show_settings_dialog = false;
            }
            let bg = panel_bg(is_dark);
            egui::TopBottomPanel::bottom("settings_bottom")
                .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(14, 10)))
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = egui::Button::new(RichText::new("Close").size(12.0).color(text_primary(is_dark)))
                            .min_size(Vec2::new(78.0_f32, 24.0_f32))
                            .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_BLUE));
                        if ui.add(ok).clicked() {
                            state.show_settings_dialog = false;
                        }
                    });
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(bg).inner_margin(egui::Margin::symmetric(16, 14)))
                .show(ctx, |ui| {
                    ui.label(RichText::new("Application Settings").strong().size(13.5).color(text_primary(is_dark)));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Theme:").size(12.0));
                        egui::ComboBox::from_id_salt("app_theme_combo")
                            .width(190.0)
                            .selected_text(state.theme_preference.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.theme_preference, ThemePreference::System, ThemePreference::System.label());
                                ui.selectable_value(&mut state.theme_preference, ThemePreference::Light, ThemePreference::Light.label());
                                ui.selectable_value(&mut state.theme_preference, ThemePreference::Dark, ThemePreference::Dark.label());
                            });
                    });
                    ui.add_space(6.0);
                    ui.checkbox(&mut state.verify_after_write, "Verify written data (DD mode SHA-256 verification)");
                    ui.add_space(8.0);
                    if ui.button("Refresh Connected USB Devices (F5)").clicked() {
                        state.refresh_devices();
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Shortcuts: F5 Refresh · Alt+H SHA-512 · Alt+F USB HDDs · Ctrl+Alt+D Theme")
                            .size(10.5)
                            .color(text_secondary(is_dark)),
                    );
                });
        },
    );
}
