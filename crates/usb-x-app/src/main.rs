#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod icon;
mod state;
mod ui;

use app::UsbXApp;
use eframe::egui;

fn main() -> eframe::Result {
    #[cfg(windows)]
    {
        if !usb_x_platform::is_elevated_admin() {
            // Automatically prompt user for Administrator privileges via Windows UAC
            if usb_x_platform::relaunch_as_admin() {
                // Elevated process spawned successfully; terminate unelevated instance
                return Ok(());
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([475.0, 595.0])
            .with_min_inner_size([440.0, 520.0])
            .with_title("USB X 0.1.0")
            .with_icon(icon::get_app_icon())
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "USB X",
        native_options,
        Box::new(|cc| {
            // Auto-detect system dark/light mode across Linux (GNOME/XFCE/KDE) and Windows
            let is_dark = cc.egui_ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or_else(usb_x_platform::is_system_dark_mode);
            ui::style::apply_theme_default(&cc.egui_ctx, is_dark);
            Ok(Box::new(UsbXApp::new(cc)))
        }),
    )
}
