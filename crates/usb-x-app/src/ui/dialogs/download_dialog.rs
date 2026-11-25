use egui::{Color32, Context, RichText, Stroke, Vec2};
use crate::state::AppState;
use crate::ui::style::{panel_bg, text_primary, text_secondary, COLOR_ACCENT_BLUE};

pub fn show_download_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_download_dialog {
        return;
    }

    let is_dark = state.is_dark_mode_active();
    let catalog = usb_x_catalog::get_official_windows_catalog();

    let mut versions: Vec<&str> = Vec::new();
    for entry in &catalog {
        if !versions.contains(&entry.version) {
            versions.push(entry.version);
        }
    }
    if !versions.iter().any(|&v| v == state.dl_version) {
        if let Some(&first) = versions.first() {
            state.dl_version = first.to_string();
        }
    }

    let ver_matches: Vec<&usb_x_catalog::WindowsIsoEntry> = catalog
        .iter()
        .filter(|e| e.version == state.dl_version)
        .collect();

    let mut releases: Vec<&str> = Vec::new();
    for entry in &ver_matches {
        if !releases.contains(&entry.release) {
            releases.push(entry.release);
        }
    }
    if !releases.iter().any(|&r| r == state.dl_release) {
        if let Some(&first) = releases.first() {
            state.dl_release = first.to_string();
        }
    }

    let rel_matches: Vec<&usb_x_catalog::WindowsIsoEntry> = ver_matches
        .iter()
        .copied()
        .filter(|e| e.release == state.dl_release)
        .collect();

    let mut editions: Vec<&str> = Vec::new();
    for entry in &rel_matches {
        if !editions.contains(&entry.edition) {
            editions.push(entry.edition);
        }
    }
    if !editions.iter().any(|&ed| ed == state.dl_edition) {
        if let Some(&first) = editions.first() {
            state.dl_edition = first.to_string();
        }
    }

    let ed_matches: Vec<&usb_x_catalog::WindowsIsoEntry> = rel_matches
        .iter()
        .copied()
        .filter(|e| e.edition == state.dl_edition)
        .collect();

    let mut languages: Vec<&str> = Vec::new();
    for entry in &ed_matches {
        if !languages.contains(&entry.language) {
            languages.push(entry.language);
        }
    }
    if !languages.iter().any(|&l| l == state.dl_language) {
        if let Some(&first) = languages.first() {
            state.dl_language = first.to_string();
        }
    }

    let lang_matches: Vec<&usb_x_catalog::WindowsIsoEntry> = ed_matches
        .iter()
        .copied()
        .filter(|e| e.language == state.dl_language)
        .collect();

    let mut arches: Vec<&str> = Vec::new();
    for entry in &lang_matches {
        if !arches.contains(&entry.architecture) {
            arches.push(entry.architecture);
        }
    }
    if !arches.iter().any(|&a| a == state.dl_arch) {
        if let Some(&first) = arches.first() {
            state.dl_arch = first.to_string();
        }
    }

    let selected_entry = lang_matches
        .iter()
        .copied()
        .find(|e| e.architecture == state.dl_arch)
        .or_else(|| ver_matches.first().copied())
        .cloned();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("usb_x_download_iso_viewport"),
        egui::ViewportBuilder::default()
            .with_title("Download ISO Image")
            .with_icon(crate::icon::get_app_icon())
            .with_inner_size([355.0, 395.0])
            .with_min_inner_size([355.0, 395.0])
            .with_max_inner_size([355.0, 395.0])
            .with_maximize_button(false)
            .with_resizable(false),
        |ctx, _class| {
            if ctx.input(|i| i.viewport().close_requested()) && !state.dl_is_downloading {
                state.show_download_dialog = false;
            }

            let bg_color = panel_bg(is_dark);

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(bg_color)
                        .inner_margin(egui::Margin::symmetric(16, 14)),
                )
                .show(ctx, |ui| {
                    let field_w = ui.available_width() - 2.0;

                    ui.label(RichText::new("Version").size(13.5).color(text_primary(is_dark)));
                    egui::ComboBox::from_id_salt("dl_version_combo")
                        .width(field_w)
                        .selected_text(&state.dl_version)
                        .show_ui(ui, |ui| {
                            for v in &versions {
                                ui.selectable_value(&mut state.dl_version, (*v).to_string(), *v);
                            }
                        });
                    ui.add_space(6.0);

                    ui.label(RichText::new("Release").size(13.5).color(text_primary(is_dark)));
                    egui::ComboBox::from_id_salt("dl_release_combo")
                        .width(field_w)
                        .selected_text(&state.dl_release)
                        .show_ui(ui, |ui| {
                            for r in &releases {
                                ui.selectable_value(&mut state.dl_release, (*r).to_string(), *r);
                            }
                        });
                    ui.add_space(6.0);

                    ui.label(RichText::new("Edition").size(13.5).color(text_primary(is_dark)));
                    egui::ComboBox::from_id_salt("dl_edition_combo")
                        .width(field_w)
                        .selected_text(&state.dl_edition)
                        .show_ui(ui, |ui| {
                            for e in &editions {
                                ui.selectable_value(&mut state.dl_edition, (*e).to_string(), *e);
                            }
                        });
                    ui.add_space(6.0);

                    ui.label(RichText::new("Language").size(13.5).color(text_primary(is_dark)));
                    egui::ComboBox::from_id_salt("dl_language_combo")
                        .width(field_w)
                        .selected_text(&state.dl_language)
                        .show_ui(ui, |ui| {
                            for l in &languages {
                                ui.selectable_value(&mut state.dl_language, (*l).to_string(), *l);
                            }
                        });
                    ui.add_space(6.0);

                    ui.label(RichText::new("Architecture").size(13.5).color(text_primary(is_dark)));
                    egui::ComboBox::from_id_salt("dl_arch_combo")
                        .width(field_w)
                        .selected_text(&state.dl_arch)
                        .show_ui(ui, |ui| {
                            for a in &arches {
                                ui.selectable_value(&mut state.dl_arch, (*a).to_string(), *a);
                            }
                        });
                    ui.add_space(10.0);

                    ui.checkbox(&mut state.dl_use_browser, "Download using a browser");
                    ui.add_space(8.0);

                    if state.dl_is_downloading {
                        ui.add(
                            egui::ProgressBar::new(state.dl_progress_fraction)
                                .text(&state.dl_progress_text)
                                .animate(true),
                        );
                        ui.add_space(6.0);
                    } else if !state.dl_progress_text.is_empty() && state.dl_progress_text.contains("failed") {
                        ui.label(
                            RichText::new(&state.dl_progress_text)
                                .size(11.0)
                                .color(Color32::from_rgb(200, 50, 50)),
                        );
                    } else if let Some(ref entry) = selected_entry {
                        let mode_hint = if entry.direct_iso_download && !state.dl_use_browser {
                            format!("Direct download (~{:.1} GB) to ~/Downloads", entry.approx_size_gb)
                        } else {
                            format!("Opens official portal (~{:.1} GB)", entry.approx_size_gb)
                        };
                        ui.label(RichText::new(mode_hint).size(10.5).color(text_secondary(is_dark)));
                        ui.add_space(4.0);
                    }

                    // Bottom side-by-side equal buttons: [ Download ] [ Back ] matching Image 2
                    let btn_w = ((ui.available_width() - 12.0) / 2.0).max(110.0);
                    ui.horizontal(|ui| {
                        let dl_btn = egui::Button::new(
                            RichText::new("Download").size(13.0).color(text_primary(is_dark)),
                        )
                        .min_size(Vec2::new(btn_w, 28.0))
                        .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_BLUE));

                        if ui.add_enabled(!state.dl_is_downloading, dl_btn).clicked() {
                            if let Some(ref entry) = selected_entry {
                                state.start_iso_download(entry);
                            }
                        }

                        ui.add_space(10.0);

                        let back_btn = egui::Button::new(
                            RichText::new("Back").size(13.0).color(text_primary(is_dark)),
                        )
                        .min_size(Vec2::new(btn_w, 28.0));

                        if ui.add_enabled(!state.dl_is_downloading, back_btn).clicked() {
                            state.show_download_dialog = false;
                        }
                    });
                });
        },
    );
}
