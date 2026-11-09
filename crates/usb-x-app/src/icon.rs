use eframe::egui;

const ICON_BYTES: &[u8] = include_bytes!("icon_rgba_64.bin");

pub fn get_app_icon() -> egui::IconData {
    egui::IconData {
        rgba: ICON_BYTES.to_vec(),
        width: 64,
        height: 64,
    }
}
