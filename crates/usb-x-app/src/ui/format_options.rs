use egui::{Color32, RichText, Ui};
use crate::state::AppState;
use crate::ui::style::{field_label, render_section_header, text_secondary};
use usb_x_core::types::{ClusterSize, FileSystem};

pub fn render_format_options(ui: &mut Ui, state: &mut AppState) {
    let is_dark = state.is_dark_mode_active();
    render_section_header(ui, "Format Options", is_dark);

    // 1. Volume label
    ui.label(field_label("Volume label", is_dark));
    ui.add(
        egui::TextEdit::singleline(&mut state.volume_label)
            .desired_width(ui.available_width() - 4.0),
    );

    ui.add_space(3.0);

    // 2. File system & Cluster size (Two columns)
    ui.columns(2, |cols| {
        // Left: File system
        cols[0].label(field_label("File system", is_dark));
        egui::ComboBox::from_id_salt("file_system_combo")
            .width(cols[0].available_width() - 4.0)
            .selected_text(state.file_system.label())
            .show_ui(&mut cols[0], |ui| {
                ui.selectable_value(&mut state.file_system, FileSystem::Ntfs, "NTFS");
                ui.selectable_value(&mut state.file_system, FileSystem::Fat32, "FAT32 (Default)");
                ui.selectable_value(&mut state.file_system, FileSystem::ExFat, "exFAT");
                ui.selectable_value(&mut state.file_system, FileSystem::Udf, "UDF");
            });

        // Right: Cluster size
        cols[1].label(field_label("Cluster size", is_dark));
        egui::ComboBox::from_id_salt("cluster_size_combo")
            .width(cols[1].available_width() - 4.0)
            .selected_text(state.cluster_size.label())
            .show_ui(&mut cols[1], |ui| {
                ui.selectable_value(&mut state.cluster_size, ClusterSize::Default, "4096 bytes (Default)");
                ui.selectable_value(&mut state.cluster_size, ClusterSize::Bytes8192, "8192 bytes");
                ui.selectable_value(&mut state.cluster_size, ClusterSize::Bytes16384, "16 kilobytes");
                ui.selectable_value(&mut state.cluster_size, ClusterSize::Bytes32768, "32 kilobytes");
                ui.selectable_value(&mut state.cluster_size, ClusterSize::Bytes65536, "64 kilobytes");
            });
    });

    ui.add_space(3.0);

    // 3. Collapsible: Advanced format options
    let arrow = if state.show_advanced_format_options { "[-] Hide advanced format options" } else { "[+] Show advanced format options" };
    let adv_btn = egui::Button::new(RichText::new(arrow).size(11.0).color(text_secondary(is_dark)))
        .fill(Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE);
    if ui.add(adv_btn).clicked() {
        state.show_advanced_format_options = !state.show_advanced_format_options;
    }

    if state.show_advanced_format_options {
        ui.indent("advanced_format_opts", |ui| {
            ui.checkbox(&mut state.quick_format, "Quick format");
            ui.checkbox(&mut state.extended_label, "Create extended label and icon files");

            ui.horizontal(|ui| {
                ui.checkbox(&mut state.check_bad_blocks, "Check device for bad blocks");
                ui.add_enabled_ui(state.check_bad_blocks, |ui| {
                    let pass_label = format!("{} pass{}", state.bad_blocks_passes, if state.bad_blocks_passes > 1 { "es" } else { "" });
                    egui::ComboBox::from_id_salt("bad_blocks_passes_combo")
                        .width(90.0)
                        .selected_text(pass_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.bad_blocks_passes, 1, "1 pass");
                            ui.selectable_value(&mut state.bad_blocks_passes, 2, "2 passes");
                            ui.selectable_value(&mut state.bad_blocks_passes, 4, "4 passes");
                        });
                });
            });
        });
    }
}

