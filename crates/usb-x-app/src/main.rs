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

    #[cfg(target_os = "linux")]
    {
        // Prompt for root access and authentication first on Linux
        if !usb_x_platform::is_elevated_admin() {
            let args: Vec<String> = std::env::args().collect();
            let skip_elevation = args.iter().any(|a| a == "--no-elevation" || a == "--unelevated");
            if !skip_elevation {
                let _ = usb_x_platform::relaunch_as_admin();
            }
        }

        // When running as root on Linux (e.g., elevated via pkexec or sudo), native Wayland
        // compositors (like GNOME Mutter) reject root client connections by design, causing
        // winit to panic with WaylandError(Connection(NoCompositor)).
        // Enforce X11 / Xwayland backend for elevated root sessions.
        if usb_x_platform::is_elevated_admin() {
            if std::env::var("WINIT_UNIX_BACKEND").is_err() {
                std::env::set_var("WINIT_UNIX_BACKEND", "x11");
            }
            std::env::remove_var("WAYLAND_DISPLAY");
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([430.0, 515.0])
            .with_min_inner_size([430.0, 400.0])
            .with_max_inner_size([430.0, 960.0])
            .with_maximize_button(false)
            .with_icon(icon::get_app_icon())
            .with_title("Pigeon 1.11.26")
            .with_resizable(false),
        ..Default::default()
    };

    let res = eframe::run_native(
        "Pigeon",
        native_options.clone(),
        Box::new(|cc| {
            // On Linux, always query the active desktop environment theme directly (works for both root/pkexec and user sessions)
            #[cfg(target_os = "linux")]
            let is_dark = usb_x_platform::is_system_dark_mode();
            #[cfg(not(target_os = "linux"))]
            let is_dark = cc
                .egui_ctx
                .system_theme()
                .map(|t| t == egui::Theme::Dark)
                .unwrap_or_else(usb_x_platform::is_system_dark_mode);
            ui::style::apply_theme_default(&cc.egui_ctx, is_dark);
            Ok(Box::new(UsbXApp::new(cc)))
        }),
    );

    #[cfg(target_os = "linux")]
    if let Err(ref e) = res {
        eprintln!("Pigeon startup error on Linux: {:?}", e);
        // If Wayland failed, fallback to X11 once
        if std::env::var("WINIT_UNIX_BACKEND").as_deref() != Ok("x11") {
            std::env::set_var("WINIT_UNIX_BACKEND", "x11");
            std::env::remove_var("WAYLAND_DISPLAY");
            let _ = std::process::Command::new("xhost").arg("+si:localuser:root").output();
            return eframe::run_native(
                "Pigeon",
                native_options,
                Box::new(|cc| {
                    let is_dark = usb_x_platform::is_system_dark_mode();
                    ui::style::apply_theme_default(&cc.egui_ctx, is_dark);
                    Ok(Box::new(UsbXApp::new(cc)))
                }),
            );
        }
    }

    res
}
