use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};
use heq_core::ui::palette::Rgba;

pub fn col(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x11, 0x0F);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(0x1E, 0x1A, 0x16);
pub const PANEL_HOVER: Color32 = Color32::from_rgb(0x2A, 0x24, 0x1E);
pub const TEXT: Color32 = Color32::from_rgb(0xE6, 0xDC, 0xCE);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8A, 0x7C, 0x6B);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(0x0C, 0x0A, 0x08);
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(0x5A, 0x3C, 0x18);
    visuals.selection.stroke = Stroke::new(1.0, col(heq_core::ui::palette::ACCENT));
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x2A, 0x24, 0x1E));

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
    w.inactive.bg_fill = PANEL_LIGHT;
    w.inactive.weak_bg_fill = PANEL_LIGHT;
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.bg_fill = PANEL_HOVER;
    w.hovered.weak_bg_fill = PANEL_HOVER;
    w.active.bg_fill = PANEL_HOVER;
    w.active.weak_bg_fill = PANEL_HOVER;

    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = CornerRadius::same(3);
    }

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 22.0;
    });
}
