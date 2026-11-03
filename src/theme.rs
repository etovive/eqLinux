//! Lenguaje visual portado de eqMac: degradado grafito, acento verde y
//! pantallas de valores con estilo LCD.

use std::sync::atomic::{AtomicU32, Ordering};

use eframe::egui::{self, Color32, Stroke};

static ACCENT_VALUE: AtomicU32 = AtomicU32::new(0x004f8d71);

pub fn set_accent(rgb: (u8, u8, u8)) {
    ACCENT_VALUE.store(
        ((rgb.0 as u32) << 16) | ((rgb.1 as u32) << 8) | rgb.2 as u32,
        Ordering::Relaxed,
    );
}

pub fn accent() -> Color32 {
    let value = ACCENT_VALUE.load(Ordering::Relaxed);
    Color32::from_rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

pub fn accent_bright() -> Color32 {
    let color = accent();
    Color32::from_rgb(
        color.r().saturating_add(36),
        color.g().saturating_add(50),
        color.b().saturating_add(40),
    )
}
pub const WARNING: Color32 = Color32::from_rgb(0xeb, 0x3f, 0x42);
pub const TEXT: Color32 = Color32::from_rgb(0xc9, 0xcd, 0xd0);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x93);
pub const BG_TOP: Color32 = Color32::from_rgb(90, 91, 95);
pub const BG_BOTTOM: Color32 = Color32::from_rgb(44, 44, 46);
pub const HEADER_BG: Color32 = Color32::from_rgb(62, 62, 62);
pub const PANEL: Color32 = Color32::from_rgb(49, 49, 51);
pub const PANEL_DARK: Color32 = Color32::from_rgb(37, 37, 39);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(58, 58, 61);
pub const SCREEN_BG: Color32 = Color32::from_rgb(16, 21, 18);
pub const SCREEN_TEXT: Color32 = Color32::from_rgb(126, 226, 166);
pub const KNOB: Color32 = Color32::from_rgb(42, 42, 45);

pub fn configure(ctx: &egui::Context) {
    let accent = accent();
    let accent_bright = accent_bright();
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_BOTTOM;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = SCREEN_BG;
    visuals.faint_bg_color = PANEL_DARK;
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = Stroke::new(1.0, accent_bright);
    visuals.hyperlink_color = accent_bright;
    visuals.override_text_color = Some(TEXT);

    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, PANEL_DARK);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.inactive.bg_fill = PANEL_LIGHT;
    visuals.widgets.inactive.weak_bg_fill = PANEL_LIGHT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(32, 32, 34));
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 74);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(70, 70, 74);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, accent);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);

    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.weak_bg_fill = accent;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, accent_bright);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);

    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(24, 24, 26));
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 30,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.slider_width = 110.0;
        style.spacing.interact_size.y = 18.0;
    });
}

pub fn paint_background(ui: &egui::Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();
    let rows = 96;
    for i in 0..rows {
        let t0 = i as f32 / rows as f32;
        let t1 = (i + 1) as f32 / rows as f32;
        let color = lerp_color(BG_TOP, BG_BOTTOM, t0);
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1 + 1.0;
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(rect.left(), y0), egui::pos2(rect.right(), y1)),
            0.0,
            color,
        );
    }

    // Viñeta muy sutil, como el ruido y el degradado del fondo original.
    let edge = Color32::from_black_alpha(28);
    painter.rect_stroke(
        rect.shrink(0.5),
        0.0,
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

pub fn small_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(10.0).color(TEXT_DIM));
}

pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(TEXT).strong());
}
