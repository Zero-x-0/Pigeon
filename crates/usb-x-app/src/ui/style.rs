use egui::{Color32, CornerRadius, Context, RichText, Stroke, Ui};

pub const COLOR_GREEN_PROGRESS: Color32 = Color32::from_rgb(0, 166, 81);
pub const COLOR_ACCENT_BLUE: Color32 = Color32::from_rgb(0, 120, 215);
pub const COLOR_DANGER_RED: Color32 = Color32::from_rgb(216, 59, 1);
pub const COLOR_SUCCESS_GREEN: Color32 = Color32::from_rgb(16, 124, 65);

pub fn get_system_accent() -> Color32 {
    if let Some((r, g, b)) = usb_x_platform::get_windows_accent_color() {
        Color32::from_rgb(r, g, b)
    } else {
        COLOR_ACCENT_BLUE
    }
}

pub fn panel_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(32, 32, 32)
    } else {
        Color32::from_rgb(243, 243, 243)
    }
}

#[allow(dead_code)]
pub fn footer_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(24, 24, 24)
    } else {
        Color32::from_rgb(238, 238, 238)
    }
}

pub fn trough_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(44, 44, 44)
    } else {
        Color32::from_rgb(228, 228, 228)
    }
}

#[allow(dead_code)]
pub fn border_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(55, 55, 55)
    } else {
        Color32::from_rgb(205, 205, 205)
    }
}

pub fn text_primary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(245, 245, 245)
    } else {
        Color32::from_rgb(28, 28, 28)
    }
}

pub fn text_secondary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(175, 175, 175)
    } else {
        Color32::from_rgb(85, 85, 85)
    }
}

pub fn build_dark_visuals(accent: Color32) -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(32, 32, 32);
    visuals.window_fill = Color32::from_rgb(32, 32, 32);
    visuals.extreme_bg_color = Color32::from_rgb(26, 26, 26);

    // Inactive widgets
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(42, 42, 42);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(42, 42, 42);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(60, 60, 60));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(240, 240, 240));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(3);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 52, 52);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 52, 52);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(3);

    // Active / Pressed widgets
    visuals.widgets.active.bg_fill = Color32::from_rgb(36, 36, 36);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(36, 36, 36);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5f32, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(3);

    // Disabled / Noninteractive widgets
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(34, 34, 34);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(34, 34, 34);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(50, 50, 50));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(120, 120, 120));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(3);

    // Open widgets
    visuals.widgets.open.bg_fill = Color32::from_rgb(42, 42, 42);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(42, 42, 42);
    visuals.widgets.open.bg_stroke = Stroke::new(1.5f32, accent);
    visuals.widgets.open.corner_radius = CornerRadius::same(3);

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0f32, Color32::WHITE);

    visuals
}

pub fn build_light_visuals(accent: Color32) -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::from_rgb(243, 243, 243);
    visuals.window_fill = Color32::from_rgb(243, 243, 243);
    visuals.extreme_bg_color = Color32::from_rgb(238, 238, 238);

    // Inactive widgets
    visuals.widgets.inactive.bg_fill = Color32::WHITE;
    visuals.widgets.inactive.weak_bg_fill = Color32::WHITE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(205, 205, 205));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(28, 28, 28));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(3);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(250, 250, 250);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(250, 250, 250);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0f32, accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0f32, Color32::BLACK);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(3);

    // Active / Pressed widgets (Dark foreground for light backgrounds so strong() is crisp black)
    visuals.widgets.active.bg_fill = Color32::from_rgb(230, 230, 230);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(230, 230, 230);
    visuals.widgets.active.bg_stroke = Stroke::new(1.5f32, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(20, 20, 20));
    visuals.widgets.active.corner_radius = CornerRadius::same(3);

    // Disabled / Noninteractive widgets (Visible gray foreground for disabled buttons)
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(246, 246, 246);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(246, 246, 246);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, Color32::from_rgb(215, 215, 215));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0f32, Color32::from_rgb(130, 130, 130));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(3);

    // Open widgets
    visuals.widgets.open.bg_fill = Color32::WHITE;
    visuals.widgets.open.weak_bg_fill = Color32::WHITE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.5f32, accent);
    visuals.widgets.open.corner_radius = CornerRadius::same(3);

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0f32, Color32::WHITE);

    visuals
}

pub fn apply_theme(ctx: &Context, is_dark: bool, accent: Color32) {
    let dark_vis = build_dark_visuals(accent);
    let light_vis = build_light_visuals(accent);

    ctx.set_visuals_of(egui::Theme::Dark, dark_vis);
    ctx.set_visuals_of(egui::Theme::Light, light_vis);
    ctx.set_theme(if is_dark { egui::Theme::Dark } else { egui::Theme::Light });

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 3.5);
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.scroll.bar_width = 6.0;
        s.spacing.scroll.floating = true;
        s.spacing.scroll.bar_inner_margin = 2.0;
        s.spacing.scroll.bar_outer_margin = 2.0;
    });
}

pub fn apply_theme_default(ctx: &Context, is_dark: bool) {
    let accent = get_system_accent();
    apply_theme(ctx, is_dark, accent);
}

pub fn render_section_header(ui: &mut Ui, title: &str, is_dark: bool) {
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let title_color = if is_dark {
            Color32::from_rgb(96, 175, 255)
        } else {
            Color32::from_rgb(0, 51, 153)
        };
        let label_resp = ui.label(RichText::new(title).strong().size(13.5).color(title_color));
        let available_w = ui.available_width();
        if available_w > 10.0 {
            let line_color = if is_dark {
                Color32::from_rgb(60, 60, 60)
            } else {
                Color32::from_rgb(205, 205, 205)
            };
            let line_y = label_resp.rect.center().y;
            let p1 = egui::pos2(label_resp.rect.max.x + 8.0, line_y);
            let p2 = egui::pos2(label_resp.rect.max.x + available_w, line_y);
            ui.painter().line_segment([p1, p2], Stroke::new(1.0f32, line_color));
        }
    });
    ui.add_space(2.0);
}

pub fn field_label(text: &str, is_dark: bool) -> RichText {
    RichText::new(text).size(11.5).color(text_secondary(is_dark))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_visuals_high_contrast() {
        let ctx = Context::default();
        apply_theme_default(&ctx, false); // light mode
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let resp = ui.selectable_label(true, "Standard Windows installation");
                let vis = ui.style().interact_selectable(&resp, true);
                assert_eq!(vis.fg_stroke.color, Color32::WHITE, "Selected text color must be crisp white");
            });
        });
    }

    #[test]
    fn test_theme_switching_safety() {
        let ctx = Context::default();
        let accent = Color32::from_rgb(0, 120, 215);

        // Switch back and forth repeatedly to verify stability
        for is_dark in [true, false, true, false, true] {
            apply_theme(&ctx, is_dark, accent);
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.label("Theme test");
                });
            });
            assert_eq!(ctx.theme(), if is_dark { egui::Theme::Dark } else { egui::Theme::Light });
        }
    }
}

