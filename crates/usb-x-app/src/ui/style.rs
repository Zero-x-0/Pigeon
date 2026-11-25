use egui::{Color32, Context, CornerRadius, Pos2, Rect, RichText, Stroke, Ui, Vec2};

pub const COLOR_GREEN_PROGRESS: Color32 = Color32::from_rgb(0, 166, 81);
pub const COLOR_ACCENT_BLUE: Color32 = Color32::from_rgb(0, 120, 215);
pub const COLOR_DANGER_RED: Color32 = Color32::from_rgb(200, 35, 35);

pub fn get_system_accent() -> Color32 {
    if let Some((r, g, b)) = usb_x_platform::get_windows_accent_color() {
        Color32::from_rgb(r, g, b)
    } else {
        COLOR_ACCENT_BLUE
    }
}

pub fn icon_blue(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(90, 165, 235)
    } else {
        Color32::from_rgb(35, 110, 180)
    }
}

pub fn panel_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(32, 32, 32)
    } else {
        Color32::from_rgb(240, 240, 240)
    }
}

pub fn footer_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(26, 26, 26)
    } else {
        Color32::from_rgb(236, 236, 236)
    }
}

pub fn trough_bg(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(46, 46, 46)
    } else {
        Color32::from_rgb(230, 230, 230)
    }
}

pub fn border_color(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(72, 72, 72)
    } else {
        Color32::from_rgb(188, 188, 188)
    }
}

pub fn text_primary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(240, 240, 240)
    } else {
        Color32::from_rgb(20, 20, 20)
    }
}

pub fn text_secondary(is_dark: bool) -> Color32 {
    if is_dark {
        Color32::from_rgb(170, 170, 170)
    } else {
        Color32::from_rgb(65, 65, 65)
    }
}

pub fn build_dark_visuals(accent: Color32) -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(32, 32, 32);
    visuals.window_fill = Color32::from_rgb(32, 32, 32);
    visuals.extreme_bg_color = Color32::from_rgb(24, 24, 24);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(44, 44, 44);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(44, 44, 44);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 70));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(240, 240, 240));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(2);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 55);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(55, 55, 55);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(2);

    visuals.widgets.active.bg_fill = Color32::from_rgb(40, 40, 40);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(40, 40, 40);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(2);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(36, 36, 36);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(36, 36, 36);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(58, 58, 58));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(135, 135, 135));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(2);

    visuals.widgets.open.bg_fill = Color32::from_rgb(44, 44, 44);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(44, 44, 44);
    visuals.widgets.open.bg_stroke = Stroke::new(1.2_f32, accent);
    visuals.widgets.open.corner_radius = CornerRadius::same(2);

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals
}

pub fn build_light_visuals(accent: Color32) -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::from_rgb(240, 240, 240);
    visuals.window_fill = Color32::from_rgb(240, 240, 240);
    visuals.extreme_bg_color = Color32::WHITE;

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(253, 253, 253);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(253, 253, 253);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(192, 192, 192));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 20, 20));
    visuals.widgets.inactive.corner_radius = CornerRadius::same(2);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(2);

    visuals.widgets.active.bg_fill = Color32::from_rgb(204, 228, 247);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(20, 20, 20));
    visuals.widgets.active.corner_radius = CornerRadius::same(2);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(238, 238, 238);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(238, 238, 238);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(205, 205, 205));
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(130, 130, 130));
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(2);

    visuals.widgets.open.bg_fill = Color32::WHITE;
    visuals.widgets.open.weak_bg_fill = Color32::WHITE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.2_f32, accent);
    visuals.widgets.open.corner_radius = CornerRadius::same(2);

    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals
}

pub fn apply_theme(ctx: &Context, is_dark: bool, accent: Color32) {
    let dark_vis = build_dark_visuals(accent);
    let light_vis = build_light_visuals(accent);

    ctx.set_visuals_of(egui::Theme::Dark, dark_vis);
    ctx.set_visuals_of(egui::Theme::Light, light_vis);
    ctx.set_theme(if is_dark { egui::Theme::Dark } else { egui::Theme::Light });

    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 3.5);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.combo_height = 220.0;
        s.spacing.scroll.bar_width = 7.0;
        s.spacing.scroll.floating = true;
    });
}

pub fn apply_theme_default(ctx: &Context, is_dark: bool) {
    let accent = get_system_accent();
    apply_theme(ctx, is_dark, accent);
}

/// Classic Rufus section header with bold title and solid right-extending horizontal rule
pub fn render_section_header(ui: &mut Ui, title: &str, is_dark: bool) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let title_color = if is_dark {
            Color32::from_rgb(245, 245, 245)
        } else {
            Color32::from_rgb(15, 15, 15)
        };
        let label_resp = ui.label(RichText::new(title).size(16.0).color(title_color));
        let available_w = ui.available_width();
        if available_w > 10.0 {
            let line_color = if is_dark {
                Color32::from_rgb(95, 95, 95)
            } else {
                Color32::from_rgb(35, 35, 35)
            };
            let line_y = label_resp.rect.center().y + 1.0;
            let p1 = Pos2::new(label_resp.rect.max.x + 8.0, line_y);
            let p2 = Pos2::new(label_resp.rect.max.x + available_w, line_y);
            ui.painter().line_segment([p1, p2], Stroke::new(1.2_f32, line_color));
        }
    });
    ui.add_space(3.0);
}

pub fn field_label(text: &str, is_dark: bool) -> RichText {
    RichText::new(text).size(12.0).color(text_primary(is_dark))
}

/// Crisp vector split-button dropdown arrow without depending on Unicode fonts
pub fn split_arrow_button(ui: &mut Ui, is_dark: bool, enabled: bool) -> egui::Response {
    let size = Vec2::new(20.0_f32, 22.0_f32);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );

    let vis = ui.style().interact(&response);
    let bg = if enabled {
        vis.bg_fill
    } else {
        ui.style().visuals.widgets.noninteractive.bg_fill
    };
    let stroke = if enabled {
        vis.bg_stroke
    } else {
        ui.style().visuals.widgets.noninteractive.bg_stroke
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(2),
        bg,
        stroke,
        egui::StrokeKind::Inside,
    );

    let fg = if !enabled {
        text_secondary(is_dark)
    } else {
        text_primary(is_dark)
    };
    let c = rect.center();
    let pts = vec![
        Pos2::new(c.x - 3.5, c.y - 1.5),
        Pos2::new(c.x + 3.5, c.y - 1.5),
        Pos2::new(c.x, c.y + 2.5),
    ];
    ui.painter()
        .add(egui::Shape::convex_polygon(pts, fg, Stroke::NONE));

    response
}

/// Crisp vector chevron + label toggle matching Rufus's Show/Hide expanders
pub fn expander_button(ui: &mut Ui, expanded: bool, text: &str, is_dark: bool) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (chev_rect, chev_resp) = ui.allocate_exact_size(Vec2::new(14.0_f32, 16.0_f32), egui::Sense::click());
        let lbl_resp = ui.add(
            egui::Button::new(RichText::new(text).size(11.5).color(text_primary(is_dark)))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .frame(false),
        );
        let hovered = chev_resp.hovered() || lbl_resp.hovered();
        let fg = if hovered {
            COLOR_ACCENT_BLUE
        } else {
            text_primary(is_dark)
        };
        let c = chev_rect.center();
        let st = Stroke::new(1.4_f32, fg);
        if expanded {
            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y + 2.0), Pos2::new(c.x, c.y - 2.0)], st);
            ui.painter().line_segment([Pos2::new(c.x, c.y - 2.0), Pos2::new(c.x + 4.0, c.y + 2.0)], st);
        } else {
            ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y - 2.0), Pos2::new(c.x, c.y + 2.0)], st);
            ui.painter().line_segment([Pos2::new(c.x, c.y + 2.0), Pos2::new(c.x + 4.0, c.y - 2.0)], st);
        }
        chev_resp.union(lbl_resp)
    })
    .inner
}

#[derive(Clone, Copy)]
pub enum ToolbarIcon {
    SaveDisk,
    CheckCircle,
    Globe,
    InfoCircle,
    SettingsSliders,
    LogTable,
}

/// Draw crisp vector icons matching the Rufus UI without relying on system emoji fonts
pub fn icon_button(ui: &mut Ui, icon: ToolbarIcon, is_dark: bool, enabled: bool) -> egui::Response {
    let size = Vec2::new(22.0_f32, 20.0_f32);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled { egui::Sense::click() } else { egui::Sense::hover() },
    );

    let tint = if !enabled {
        Color32::from_rgb(155, 155, 155)
    } else if response.hovered() {
        COLOR_ACCENT_BLUE
    } else {
        icon_blue(is_dark)
    };

    if response.hovered() && enabled {
        let hover_bg = if is_dark {
            Color32::from_rgb(50, 65, 82)
        } else {
            Color32::from_rgb(225, 238, 250)
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(2),
            hover_bg,
            Stroke::new(1.0_f32, COLOR_ACCENT_BLUE),
            egui::StrokeKind::Inside,
        );
    }

    let p = ui.painter();
    let c = rect.center();
    let st = Stroke::new(1.3_f32, tint);

    match icon {
        ToolbarIcon::SaveDisk => {
            let r = Rect::from_center_size(c, Vec2::new(13.0_f32, 13.0_f32));
            p.rect_stroke(r, CornerRadius::same(1), st, egui::StrokeKind::Inside);
            p.rect_stroke(
                Rect::from_min_max(Pos2::new(r.left() + 3.0, r.top()), Pos2::new(r.right() - 3.0, r.top() + 5.0)),
                CornerRadius::ZERO,
                Stroke::new(1.1_f32, tint),
                egui::StrokeKind::Inside,
            );
            p.rect_stroke(
                Rect::from_min_max(Pos2::new(r.left() + 2.5, r.bottom() - 5.5), Pos2::new(r.right() - 2.5, r.bottom())),
                CornerRadius::ZERO,
                Stroke::new(1.1_f32, tint),
                egui::StrokeKind::Inside,
            );
        }
        ToolbarIcon::CheckCircle => {
            let radius = 7.5_f32;
            p.circle_stroke(c, radius, st);
            p.line_segment(
                [Pos2::new(c.x - 3.8, c.y + 0.3), Pos2::new(c.x - 1.0, c.y + 3.2)],
                Stroke::new(1.5_f32, tint),
            );
            p.line_segment(
                [Pos2::new(c.x - 1.0, c.y + 3.2), Pos2::new(c.x + 4.0, c.y - 2.8)],
                Stroke::new(1.5_f32, tint),
            );
        }
        ToolbarIcon::Globe => {
            let radius = 7.5_f32;
            p.circle_stroke(c, radius, st);
            p.line_segment([Pos2::new(c.x - radius, c.y), Pos2::new(c.x + radius, c.y)], Stroke::new(1.0_f32, tint));
            p.line_segment([Pos2::new(c.x, c.y - radius), Pos2::new(c.x, c.y + radius)], Stroke::new(1.0_f32, tint));
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(7.0_f32, 14.5_f32)),
                CornerRadius::same(4),
                Stroke::new(1.0_f32, tint),
                egui::StrokeKind::Inside,
            );
        }
        ToolbarIcon::InfoCircle => {
            let radius = 7.5_f32;
            p.circle_stroke(c, radius, st);
            p.circle_filled(Pos2::new(c.x, c.y - 3.5), 1.1_f32, tint);
            p.line_segment(
                [Pos2::new(c.x, c.y - 1.0), Pos2::new(c.x, c.y + 4.3)],
                Stroke::new(1.6_f32, tint),
            );
        }
        ToolbarIcon::SettingsSliders => {
            for (row_idx, knob_dx) in [(-4.0_f32, -2.5_f32), (0.0_f32, 2.5_f32), (4.0_f32, -1.5_f32)] {
                let y = c.y + row_idx;
                p.line_segment([Pos2::new(c.x - 7.0, y), Pos2::new(c.x + 7.0, y)], Stroke::new(1.2_f32, tint));
                p.rect_filled(
                    Rect::from_center_size(Pos2::new(c.x + knob_dx, y), Vec2::new(2.5_f32, 4.0_f32)),
                    CornerRadius::ZERO,
                    tint,
                );
            }
        }
        ToolbarIcon::LogTable => {
            let r = Rect::from_center_size(c, Vec2::new(14.0_f32, 12.0_f32));
            p.rect_stroke(r, CornerRadius::ZERO, st, egui::StrokeKind::Inside);
            p.line_segment([Pos2::new(r.left(), r.top() + 3.5), Pos2::new(r.right(), r.top() + 3.5)], Stroke::new(1.2_f32, tint));
            p.line_segment([Pos2::new(r.left(), r.top() + 7.5), Pos2::new(r.right(), r.top() + 7.5)], Stroke::new(1.0_f32, tint));
            p.line_segment([Pos2::new(r.left() + 4.5, r.top() + 3.5), Pos2::new(r.left() + 4.5, r.bottom())], Stroke::new(1.0_f32, tint));
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_visuals_high_contrast() {
        let ctx = Context::default();
        apply_theme_default(&ctx, false);
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
