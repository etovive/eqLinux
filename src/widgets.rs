//! Widgets dibujados a medida que imitan los controles esqueuomórficos de eqMac.

use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::config::SpectrumStyle;
use crate::dsp::{ExpertBand, FilterType};
use crate::theme::{self, KNOB, PANEL_LIGHT, SCREEN_BG, SCREEN_TEXT, TEXT, TEXT_DIM};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Close,
    Minus,
    Power,
    Gear,
    Help,
    Eye,
    EyeOff,
    Trash,
    Plus,
    Reset,
    Record,
    Triangle,
}

pub fn knob(
    ui: &mut Ui,
    value: &mut f64,
    min: f64,
    max: f64,
    size: f32,
    reset_value: f64,
    enabled: bool,
) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());

    if enabled {
        if response.dragged() {
            let delta = -response.drag_delta().y as f64;
            let range = max - min;
            *value = (*value + delta * range / 180.0).clamp(min, max);
            response.mark_changed();
        }
        if response.double_clicked() {
            *value = reset_value;
            response.mark_changed();
        }
        if response.hovered() {
            let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y) as f64;
            if scroll != 0.0 {
                *value = (*value + scroll * (max - min) / 600.0).clamp(min, max);
                response.mark_changed();
            }
        }
    }

    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.5;

    painter.circle_filled(
        center + Vec2::new(0.0, 2.0),
        radius,
        Color32::from_black_alpha(90),
    );
    painter.circle_filled(center, radius, Color32::from_rgb(30, 30, 32));
    painter.circle_stroke(
        center,
        radius - 0.5,
        Stroke::new(1.0, Color32::from_rgb(70, 70, 74)),
    );
    painter.circle_filled(center, radius * 0.82, KNOB);
    painter.circle_stroke(
        center,
        radius * 0.82,
        Stroke::new(1.0, Color32::from_rgb(20, 20, 22)),
    );

    let start = 135f32.to_radians();
    let end = 405f32.to_radians();
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0) as f32;
    let angle = start + (end - start) * t;

    let track_radius = radius * 0.9;
    let segments = 56;
    for i in 0..segments {
        let a0 = start + (end - start) * (i as f32 / segments as f32);
        let a1 = start + (end - start) * ((i + 1) as f32 / segments as f32);
        let color = if a1 <= angle + 0.001 {
            theme::accent()
        } else {
            Color32::from_rgb(82, 82, 86)
        };
        let p0 = center + Vec2::new(a0.cos(), a0.sin()) * track_radius;
        let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
        painter.line_segment([p0, p1], Stroke::new(2.0, color));
    }

    let pointer_start = center + Vec2::new(angle.cos(), angle.sin()) * (radius * 0.2);
    let pointer_end = center + Vec2::new(angle.cos(), angle.sin()) * (radius * 0.62);
    painter.line_segment(
        [pointer_start, pointer_end],
        Stroke::new(2.5, theme::accent()),
    );

    if !enabled {
        painter.circle_filled(center, radius, Color32::from_black_alpha(105));
    }

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    response
}

pub fn value_screen(ui: &mut Ui, text: &str, enabled: bool, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 17.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, SCREEN_BG);
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_rgb(52, 62, 56)),
        StrokeKind::Inside,
    );
    let color = if enabled {
        SCREEN_TEXT
    } else {
        Color32::from_rgb(48, 76, 58)
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(10.0),
        color,
    );
    response
}

pub fn toggle(ui: &mut Ui, on: &mut bool, enabled: bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(28.0, 16.0), Sense::click());
    if enabled && response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    let painter = ui.painter();
    let track = if *on {
        theme::accent()
    } else {
        Color32::from_rgb(76, 76, 80)
    };
    let track = if enabled {
        track
    } else {
        track.gamma_multiply(0.55)
    };
    painter.rect_filled(rect, 8.0, track);
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, Color32::from_rgb(24, 24, 26)),
        StrokeKind::Inside,
    );

    let knob_x = if *on {
        rect.right() - 8.0
    } else {
        rect.left() + 8.0
    };
    painter.circle_filled(
        egui::pos2(knob_x, rect.center().y),
        6.0,
        if enabled {
            Color32::from_rgb(232, 232, 234)
        } else {
            Color32::from_rgb(160, 160, 162)
        },
    );
    painter.circle_stroke(
        egui::pos2(knob_x, rect.center().y),
        6.0,
        Stroke::new(1.0, Color32::from_rgb(40, 40, 42)),
    );

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn check_box(ui: &mut Ui, checked: &mut bool, enabled: bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::click());
    if enabled && response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }

    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, SCREEN_BG);
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(
            1.0,
            if *checked {
                theme::accent()
            } else {
                Color32::from_rgb(78, 78, 82)
            },
        ),
        StrokeKind::Inside,
    );

    if *checked {
        let check = Stroke::new(1.6, theme::accent_bright());
        painter.line_segment(
            [
                rect.left_top() + Vec2::new(3.0, 6.0),
                rect.center() + Vec2::new(-1.0, 2.5),
            ],
            check,
        );
        painter.line_segment(
            [
                rect.center() + Vec2::new(-1.0, 2.5),
                rect.right_bottom() - Vec2::new(3.0, 4.0),
            ],
            check,
        );
    }

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn labeled_checkbox(ui: &mut Ui, checked: &mut bool, label: &str, enabled: bool) -> Response {
    let mut changed = false;
    let mut response = ui
        .horizontal(|ui| {
            let response = check_box(ui, checked, enabled);
            changed |= response.changed();
            let label_response = ui.add(
                egui::Label::new(egui::RichText::new(label).size(10.0).color(TEXT))
                    .sense(Sense::click()),
            );
            if enabled && label_response.clicked() {
                *checked = !*checked;
                changed = true;
            }
            response
        })
        .inner;

    if changed {
        response.mark_changed();
    }
    response
}

pub fn icon_button(ui: &mut Ui, icon: Icon, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    let painter = ui.painter();
    let center = rect.center();

    let fill = if response.hovered() && enabled {
        Color32::from_rgb(72, 72, 76)
    } else {
        Color32::from_rgb(56, 56, 59)
    };
    painter.circle_filled(center, 8.5, fill);
    painter.circle_stroke(center, 8.5, Stroke::new(1.0, Color32::from_rgb(28, 28, 30)));

    let color = if enabled {
        Color32::from_rgb(205, 205, 208)
    } else {
        Color32::from_rgb(110, 110, 114)
    };
    draw_icon(painter, center, icon, 4.2, color);

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn header_button(ui: &mut Ui, icon: Icon, enabled: bool, diameter: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::click());
    let painter = ui.painter();
    let center = rect.center();
    let fill = if response.hovered() && enabled {
        Color32::from_rgb(158, 158, 163)
    } else if enabled {
        Color32::from_rgb(142, 142, 147)
    } else {
        Color32::from_rgb(96, 96, 100)
    };
    painter.circle_filled(center, diameter * 0.5, fill);
    painter.circle_stroke(
        center,
        diameter * 0.5,
        Stroke::new(1.0, Color32::from_rgb(46, 46, 46)),
    );
    let color = if enabled {
        Color32::from_rgb(46, 46, 46)
    } else {
        Color32::from_rgb(120, 120, 124)
    };
    draw_icon(painter, center, icon, diameter * 0.24, color);

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn square_button(ui: &mut Ui, icon: Icon, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    let painter = ui.painter();
    let fill = if response.hovered() && enabled {
        Color32::from_rgb(74, 74, 78)
    } else {
        Color32::from_rgb(56, 56, 59)
    };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0, Color32::from_rgb(24, 24, 26)),
        StrokeKind::Inside,
    );
    let color = if enabled {
        Color32::from_rgb(205, 205, 208)
    } else {
        Color32::from_rgb(110, 110, 114)
    };
    draw_icon(painter, rect.center(), icon, 4.6, color);

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn reset_triangle(ui: &mut Ui, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::click());
    let color = if enabled {
        theme::accent()
    } else {
        theme::accent().gamma_multiply(0.5)
    };
    draw_icon(ui.painter(), rect.center(), Icon::Triangle, 4.0, color);

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn draw_icon(painter: &egui::Painter, center: Pos2, icon: Icon, size: f32, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    match icon {
        Icon::Close => {
            painter.line_segment(
                [
                    center + Vec2::new(-size, -size),
                    center + Vec2::new(size, size),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-size, size),
                    center + Vec2::new(size, -size),
                ],
                stroke,
            );
        }
        Icon::Minus => {
            painter.line_segment(
                [
                    center + Vec2::new(-size, 0.0),
                    center + Vec2::new(size, 0.0),
                ],
                stroke,
            );
        }
        Icon::Power => {
            let points = arc_points(center, size, 0.6, 5.68, 20);
            painter.add(egui::Shape::line(points, stroke));
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -size * 1.1),
                    center + Vec2::new(0.0, -size * 0.1),
                ],
                stroke,
            );
        }
        Icon::Gear => {
            painter.circle_stroke(center, size * 0.55, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let p0 = center + Vec2::new(a.cos(), a.sin()) * (size * 0.65);
                let p1 = center + Vec2::new(a.cos(), a.sin()) * (size * 1.0);
                painter.line_segment([p0, p1], stroke);
            }
        }
        Icon::Help => {
            painter.text(
                center,
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(size * 2.2),
                color,
            );
        }
        Icon::Eye | Icon::EyeOff => {
            let top = arc_points(center, size * 1.15, 3.34, 6.08, 16);
            let bottom = arc_points(center, size * 1.15, 0.2, 2.94, 16);
            painter.add(egui::Shape::line(top, stroke));
            painter.add(egui::Shape::line(bottom, stroke));
            painter.circle_filled(center, size * 0.32, color);
            if icon == Icon::EyeOff {
                painter.line_segment(
                    [
                        center + Vec2::new(-size * 1.2, size * 1.1),
                        center + Vec2::new(size * 1.2, -size * 1.1),
                    ],
                    stroke,
                );
            }
        }
        Icon::Trash => {
            let body = Rect::from_center_size(
                center + Vec2::new(0.0, size * 0.15),
                Vec2::new(size * 1.25, size * 1.55),
            );
            painter.rect_stroke(body, 1.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.9, -size * 0.75),
                    center + Vec2::new(size * 0.9, -size * 0.75),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.35, -size * 1.05),
                    center + Vec2::new(size * 0.35, -size * 1.05),
                ],
                stroke,
            );
        }
        Icon::Plus => {
            painter.line_segment(
                [
                    center + Vec2::new(-size, 0.0),
                    center + Vec2::new(size, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -size),
                    center + Vec2::new(0.0, size),
                ],
                stroke,
            );
        }
        Icon::Reset => {
            let points = arc_points(center, size, 0.5, 5.0, 18);
            painter.add(egui::Shape::line(points, stroke));
            let tip = center + Vec2::new(size * 0.55, -size * 0.85);
            painter.line_segment([tip, tip + Vec2::new(-size * 0.35, size * 0.1)], stroke);
            painter.line_segment([tip, tip + Vec2::new(size * 0.35, size * 0.1)], stroke);
        }
        Icon::Record => {
            painter.circle_stroke(center, size, stroke);
            painter.circle_filled(center, size * 0.55, color);
        }
        Icon::Triangle => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + Vec2::new(0.0, -size),
                    center + Vec2::new(size, size * 0.8),
                    center + Vec2::new(-size, size * 0.8),
                ],
                color,
                Stroke::NONE,
            ));
        }
    }
}

fn arc_points(center: Pos2, radius: f32, start: f32, end: f32, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|i| {
            let a = start + (end - start) * (i as f32 / steps as f32);
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect()
}

pub fn fader(
    ui: &mut Ui,
    value: &mut f64,
    min: f64,
    max: f64,
    height: f32,
    enabled: bool,
) -> Response {
    let width = 24.0;
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, height), Sense::click_and_drag());

    if enabled {
        if response.dragged() {
            let delta = -response.drag_delta().y as f64;
            *value = (*value + delta * (max - min) / (height as f64 * 0.78)).clamp(min, max);
            response.mark_changed();
        }
        if response.double_clicked() {
            *value = 0.0;
            response.mark_changed();
        }
    }

    let painter = ui.painter();
    let cx = rect.center().x;
    let top = rect.top() + 8.0;
    let bottom = rect.bottom() - 8.0;

    let groove = Rect::from_min_max(egui::pos2(cx - 3.0, top), egui::pos2(cx + 3.0, bottom));
    painter.rect_filled(groove, 3.0, Color32::from_rgb(22, 22, 24));
    painter.rect_stroke(
        groove,
        3.0,
        Stroke::new(1.0, Color32::from_rgb(58, 58, 62)),
        StrokeKind::Inside,
    );

    for i in 0..=4 {
        let y = top + (bottom - top) * (i as f32 / 4.0);
        painter.line_segment(
            [egui::pos2(cx + 8.0, y), egui::pos2(cx + 11.0, y)],
            Stroke::new(1.0, Color32::from_rgb(88, 88, 92)),
        );
    }

    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0) as f32;
    let thumb_y = bottom - (bottom - top) * t;
    let thumb = Rect::from_center_size(egui::pos2(cx, thumb_y), Vec2::new(18.0, 8.0));
    let thumb_fill = if enabled {
        Color32::from_rgb(126, 126, 130)
    } else {
        Color32::from_rgb(86, 86, 90)
    };
    painter.rect_filled(thumb, 2.0, thumb_fill);
    painter.rect_stroke(
        thumb,
        2.0,
        Stroke::new(1.0, Color32::from_rgb(28, 28, 30)),
        StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(thumb.left() + 3.0, thumb.center().y),
            egui::pos2(thumb.right() - 3.0, thumb.center().y),
        ],
        Stroke::new(1.0, Color32::from_rgb(196, 196, 200)),
    );

    let center_y = top + (bottom - top) * 0.5;
    painter.line_segment(
        [
            egui::pos2(cx - 7.0, center_y),
            egui::pos2(cx + 7.0, center_y),
        ],
        Stroke::new(1.0, Color32::from_rgb(150, 150, 154)),
    );

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    response
}

#[allow(clippy::too_many_arguments)]
pub fn flat_slider(
    ui: &mut Ui,
    value: &mut f64,
    min: f64,
    max: f64,
    width: f32,
    reset_value: f64,
    show_middle_notch: bool,
    enabled: bool,
) -> Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());

    if enabled {
        if response.dragged() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                *value = min + (max - min) * t;
                response.mark_changed();
            }
        }
        if response.double_clicked() {
            *value = reset_value;
            response.mark_changed();
        }
    }

    let painter = ui.painter();
    let cy = rect.center().y;
    let track = Rect::from_min_max(
        egui::pos2(rect.left() + 4.0, cy - 1.5),
        egui::pos2(rect.right() - 4.0, cy + 1.5),
    );
    painter.rect_filled(track, 2.0, Color32::from_rgb(24, 24, 26));

    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0) as f32;
    let thumb_x = track.left() + track.width() * t;

    let filled = Rect::from_min_max(track.min, egui::pos2(thumb_x, track.max.y));
    painter.rect_filled(filled, 2.0, theme::accent());

    let thumb = egui::pos2(thumb_x, cy);
    painter.circle_filled(thumb, 5.0, Color32::from_rgb(220, 220, 224));
    painter.circle_stroke(thumb, 5.0, Stroke::new(1.0, Color32::from_rgb(30, 30, 32)));

    if show_middle_notch {
        let center_x = track.left() + track.width() * 0.5;
        painter.line_segment(
            [
                egui::pos2(center_x, cy - 4.0),
                egui::pos2(center_x, cy + 4.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(120, 120, 124)),
        );
    }

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn tab(ui: &mut Ui, text: &str, active: bool, enabled: bool) -> Response {
    let width = (text.len() as f32 * 6.6 + 18.0).max(52.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 22.0), Sense::click());

    let painter = ui.painter();
    if active {
        let radius = egui::CornerRadius {
            nw: 5,
            ne: 5,
            sw: 0,
            se: 0,
        };
        painter.rect_filled(rect, radius, Color32::from_rgb(36, 36, 38));
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, Color32::from_rgb(24, 24, 26)),
            StrokeKind::Inside,
        );
    }

    let color = if !enabled {
        TEXT_DIM.gamma_multiply(0.5)
    } else if active {
        theme::accent_bright()
    } else {
        TEXT_DIM
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(11.0),
        color,
    );

    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 7.0), Sense::hover());
    let painter = ui.painter();
    let y = rect.center().y;
    painter.line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0, Color32::from_rgb(24, 24, 26)),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left(), y + 1.0),
            egui::pos2(rect.right(), y + 1.0),
        ],
        Stroke::new(1.0, Color32::from_rgb(72, 72, 76)),
    );
}

pub fn vertical_divider(ui: &mut Ui, height: f32, center_x: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(7.0, height), Sense::hover());
    let painter = ui.painter();
    painter.line_segment(
        [
            egui::pos2(center_x, rect.top()),
            egui::pos2(center_x, rect.bottom()),
        ],
        Stroke::new(1.0, Color32::from_rgb(24, 24, 26)),
    );
    painter.line_segment(
        [
            egui::pos2(center_x + 1.0, rect.top()),
            egui::pos2(center_x + 1.0, rect.bottom()),
        ],
        Stroke::new(1.0, Color32::from_rgb(72, 72, 76)),
    );
}

pub fn vent(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 18.0), Sense::hover());
    let painter = ui.painter();
    for i in 0..6 {
        let x = rect.left() + 4.0 + i as f32 * 5.0;
        let shade = if i % 2 == 0 {
            Color32::from_rgb(30, 30, 32)
        } else {
            Color32::from_rgb(70, 70, 74)
        };
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 2.0),
                egui::pos2(x, rect.bottom() - 2.0),
            ],
            Stroke::new(2.0, shade),
        );
    }
}

pub fn help_badge(ui: &mut Ui, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
    let painter = ui.painter();
    painter.circle_filled(rect.center(), 6.0, PANEL_LIGHT);
    painter.circle_stroke(
        rect.center(),
        6.0,
        Stroke::new(1.0, Color32::from_rgb(30, 30, 32)),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::proportional(9.0),
        TEXT_DIM,
    );
    response.on_hover_text(tooltip)
}

pub fn spectrum(
    ui: &mut Ui,
    bands: &[f32; 32],
    enabled: bool,
    style: &mut SpectrumStyle,
    spanish: bool,
) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    let changed = response.clicked();
    if changed {
        *style = style.next();
    }
    response
        .on_hover_text(format!(
            "{} · {}",
            style.label(spanish),
            if spanish {
                "haz clic para cambiar el estilo"
            } else {
                "click to change style"
            }
        ))
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, Color32::from_rgb(14, 16, 15));
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_rgb(44, 52, 47)),
        StrokeKind::Inside,
    );

    let color_at = |position: f32, value: f32| {
        if enabled {
            iridescent_color(theme::accent(), position)
                .gamma_multiply(0.65 + value.clamp(0.0, 1.0) * 0.35)
        } else {
            Color32::from_rgb(40, 54, 46)
        }
    };
    let center_y = rect.center().y;
    let max_height = rect.height() * 0.42;
    let gap = 1.0;
    let bar_width =
        ((rect.width() - gap * (bands.len() as f32 + 1.0)) / bands.len() as f32).max(1.0);

    match *style {
        SpectrumStyle::Default => {
            for (i, value) in bands.iter().enumerate() {
                let value = value.clamp(0.0, 1.0);
                let h = (rect.height() - 4.0) * value;
                let x = rect.left() + gap + i as f32 * (bar_width + gap);
                let bar = Rect::from_min_max(
                    egui::pos2(x, rect.bottom() - 2.0 - h),
                    egui::pos2(x + bar_width, rect.bottom() - 2.0),
                );
                painter.rect_filled(
                    bar,
                    1.0,
                    color_at(i as f32 / (bands.len() - 1) as f32, value),
                );
            }
        }
        SpectrumStyle::Waveform => {
            let points: Vec<Pos2> = bands
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    let x = rect.left()
                        + 2.0
                        + i as f32 * (rect.width() - 4.0) / (bands.len() - 1) as f32;
                    let y = center_y - value.clamp(0.0, 1.0) * max_height;
                    egui::pos2(x, y)
                })
                .collect();
            for (i, pair) in points.windows(2).enumerate() {
                painter.line_segment(
                    [pair[0], pair[1]],
                    Stroke::new(
                        1.3,
                        color_at(
                            (i as f32 + 0.5) / (bands.len() - 1) as f32,
                            bands[i].max(bands[i + 1]),
                        ),
                    ),
                );
            }
        }
        SpectrumStyle::Mirrored | SpectrumStyle::Rounded => {
            let radius = if *style == SpectrumStyle::Rounded {
                bar_width.min(3.0)
            } else {
                0.5
            };
            for (i, value) in bands.iter().enumerate() {
                let value = value.clamp(0.0, 1.0);
                let h = max_height * value;
                let x = rect.left() + gap + i as f32 * (bar_width + gap);
                let bar = Rect::from_min_max(
                    egui::pos2(x, center_y - h),
                    egui::pos2(x + bar_width, center_y + h),
                );
                painter.rect_filled(
                    bar,
                    radius,
                    color_at(i as f32 / (bands.len() - 1) as f32, value),
                );
            }
        }
        SpectrumStyle::Filled => {
            let segment_width = (rect.width() - 4.0) / (bands.len() - 1) as f32;
            for (i, pair) in bands.windows(2).enumerate() {
                let x1 = rect.left() + 2.0 + i as f32 * segment_width;
                let x2 = x1 + segment_width;
                let h1 = pair[0].clamp(0.0, 1.0) * max_height;
                let h2 = pair[1].clamp(0.0, 1.0) * max_height;
                let fill = color_at(
                    (i as f32 + 0.5) / (bands.len() - 1) as f32,
                    pair[0].max(pair[1]),
                );
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        egui::pos2(x1, center_y - h1),
                        egui::pos2(x2, center_y - h2),
                        egui::pos2(x2, center_y + h2),
                        egui::pos2(x1, center_y + h1),
                    ],
                    fill,
                    Stroke::NONE,
                ));
            }
        }
        SpectrumStyle::Blocks | SpectrumStyle::Dots => {
            let step = if *style == SpectrumStyle::Blocks {
                3.0
            } else {
                4.0
            };
            let dot_radius = 1.35;
            for (i, value) in bands.iter().enumerate() {
                let value = value.clamp(0.0, 1.0);
                let x = rect.left() + gap + i as f32 * (bar_width + gap) + bar_width * 0.5;
                let levels = (value * max_height / step).ceil() as usize;
                for level in 0..levels {
                    let y_offset = level as f32 * step + step * 0.5;
                    let color = color_at(i as f32 / (bands.len() - 1) as f32, value);
                    if *style == SpectrumStyle::Blocks {
                        let block = Rect::from_center_size(
                            egui::pos2(x, center_y - y_offset),
                            Vec2::new(bar_width.max(1.0), step - 1.0),
                        );
                        let mirror = Rect::from_center_size(
                            egui::pos2(x, center_y + y_offset),
                            Vec2::new(bar_width.max(1.0), step - 1.0),
                        );
                        painter.rect_filled(block, 0.2, color);
                        painter.rect_filled(mirror, 0.2, color);
                    } else {
                        painter.circle_filled(
                            egui::pos2(x, center_y - y_offset),
                            dot_radius,
                            color,
                        );
                        painter.circle_filled(
                            egui::pos2(x, center_y + y_offset),
                            dot_radius,
                            color,
                        );
                    }
                }
            }
        }
    }
    changed
}

fn iridescent_color(start: Color32, position: f32) -> Color32 {
    let stops = [
        start,
        Color32::from_rgb(50, 210, 195),
        Color32::from_rgb(145, 85, 220),
        Color32::from_rgb(255, 0, 170),
    ];
    let scaled = position.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let index = (scaled as usize).min(stops.len() - 2);
    theme::lerp_color(stops[index], stops[index + 1], scaled - index as f32)
}

#[cfg(test)]
mod tests {
    use super::iridescent_color;
    use eframe::egui::Color32;

    #[test]
    fn iridescent_gradient_uses_selected_accent_and_ends_in_fuchsia() {
        let start = Color32::from_rgb(72, 126, 176);
        assert_eq!(iridescent_color(start, 0.0), start);
        assert_eq!(iridescent_color(start, 1.0), Color32::from_rgb(255, 0, 170));
    }
}

pub fn band_color(frequency: f64) -> Color32 {
    // Map frequency logarithmically 20Hz..20kHz → hue 0°(red)..240°(blue)
    let t = ((frequency.max(20.0) / 20.0).log2() / (20_000.0_f64 / 20.0).log2()).clamp(0.0, 1.0);
    let hue = (1.0 - t) * 240.0_f64;
    let h = hue / 60.0;
    let i = h.floor() as i32;
    let f = h - h.floor();
    let (r, g, b): (f64, f64, f64) = match i % 6 {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    Color32::from_rgb((r * 220.0) as u8, (g * 220.0) as u8, (b * 220.0) as u8)
}

pub fn expert_graph(ui: &mut Ui, bands: &mut [ExpertBand], enabled: bool) -> bool {
    let width = ui.available_width();
    // Leave left/right margins for dB labels, bottom margin for freq labels
    let graph_height = 260.0_f32;
    let left_margin = 38.0_f32;
    let right_margin = 38.0_f32;
    let bottom_margin = 18.0_f32;
    let top_pad = 6.0_f32;
    let total_h = graph_height + bottom_margin + top_pad;

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, total_h), Sense::click_and_drag());
    let painter = ui.painter();

    // Background for full widget area
    painter.rect_filled(rect, 4.0, SCREEN_BG);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::from_rgb(44, 52, 47)),
        StrokeKind::Inside,
    );

    // The actual plot area (where curve lives)
    let plot = Rect::from_min_max(
        egui::pos2(rect.left() + left_margin, rect.top() + top_pad),
        egui::pos2(
            rect.right() - right_margin,
            rect.top() + top_pad + graph_height,
        ),
    );

    // ── coordinate helpers ───────────────────────────────────────────────
    let freq_at = |x: f32| {
        let t = ((x - plot.left()) / plot.width()).clamp(0.0, 1.0) as f64;
        20.0 * (20_000.0f64 / 20.0).powf(t)
    };
    let x_at = |frequency: f64| -> f32 {
        let t = (frequency.clamp(20.0, 20_000.0) / 20.0).log(20_000.0 / 20.0);
        plot.left() + plot.width() * t as f32
    };
    let y_at = |gain: f64| -> f32 {
        let t = ((gain.clamp(-30.0, 30.0) + 30.0) / 60.0) as f32;
        plot.bottom() - plot.height() * t
    };
    let gain_at = |y: f32| -> f64 {
        let t = ((plot.bottom() - y) / plot.height()).clamp(0.0, 1.0) as f64;
        t * 60.0 - 30.0
    };

    // ── grid – vertical (frequency) ──────────────────────────────────────
    let grid_dim = Stroke::new(1.0, Color32::from_rgb(30, 40, 34));

    let freq_grid: &[(f64, &str)] = &[
        (20.0, "20"),
        (50.0, "50"),
        (100.0, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1_000.0, "1k"),
        (2_000.0, "2k"),
        (5_000.0, "5k"),
        (10_000.0, "10k"),
        (20_000.0, "20k"),
    ];
    for &(freq, label) in freq_grid {
        let x = x_at(freq);
        painter.line_segment(
            [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
            grid_dim,
        );
        painter.text(
            egui::pos2(x, plot.bottom() + 3.0),
            Align2::CENTER_TOP,
            label,
            FontId::monospace(9.0),
            TEXT_DIM,
        );
    }

    // ── grid – horizontal (dB) ───────────────────────────────────────────
    let db_levels: &[(f64, bool)] = &[
        (-30.0, false),
        (-24.0, false),
        (-18.0, false),
        (-12.0, false),
        (-6.0, false),
        (0.0, true),
        (6.0, false),
        (12.0, false),
        (18.0, false),
        (24.0, false),
        (30.0, false),
    ];
    for &(db, is_zero) in db_levels {
        let y = y_at(db);
        let stroke = if is_zero {
            Stroke::new(1.2, Color32::from_rgb(65, 80, 68))
        } else {
            grid_dim
        };
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            stroke,
        );
        // Left dB label
        let label = if db > 0.0 {
            format!("+{db:.0}")
        } else {
            format!("{db:.0}")
        };
        painter.text(
            egui::pos2(plot.left() - 4.0, y),
            Align2::RIGHT_CENTER,
            &label,
            FontId::monospace(8.5),
            if is_zero { TEXT } else { TEXT_DIM },
        );
        // Right dB label (only main levels)
        if db % 6.0 == 0.0 {
            painter.text(
                egui::pos2(plot.right() + 4.0, y),
                Align2::LEFT_CENTER,
                &label,
                FontId::monospace(8.5),
                if is_zero { TEXT } else { TEXT_DIM },
            );
        }
    }

    // ── compute composite curve (181 points) ─────────────────────────────
    let mut composite: Vec<Pos2> = Vec::with_capacity(181);
    for i in 0..=180 {
        let x = plot.left() + plot.width() * (i as f32 / 180.0);
        let frequency = freq_at(x);
        let gain: f64 = bands
            .iter()
            .map(|b| band_response(b, frequency))
            .sum::<f64>()
            .clamp(-30.0, 30.0);
        composite.push(egui::pos2(x, y_at(gain)));
    }

    // ── per-band colored fills ───────────────────────────────────────────
    let zero_y = y_at(0.0);
    if enabled {
        // Sort bands by frequency so lower-freq fills go behind higher-freq
        let mut order: Vec<usize> = (0..bands.len()).filter(|&i| bands[i].enabled).collect();
        order.sort_by(|&a, &b| bands[a].frequency.partial_cmp(&bands[b].frequency).unwrap());

        for &bi in &order {
            let band = &bands[bi];
            let color = band_color(band.frequency).gamma_multiply(0.38);
            let mut band_pts: Vec<Pos2> = Vec::with_capacity(181);
            for i in 0..=180 {
                let x = plot.left() + plot.width() * (i as f32 / 180.0);
                let frequency = freq_at(x);
                let gain = band_response(band, frequency).clamp(-30.0, 30.0);
                band_pts.push(egui::pos2(x, y_at(gain)));
            }
            // Build fill polygon: curve forward + baseline backward
            let mut poly: Vec<Pos2> = band_pts.clone();
            poly.push(egui::pos2(band_pts.last().unwrap().x, zero_y));
            poly.push(egui::pos2(band_pts.first().unwrap().x, zero_y));
            painter.add(egui::Shape::convex_polygon(poly, color, Stroke::NONE));
        }
    }

    // ── composite white curve ────────────────────────────────────────────
    let curve_color = if enabled {
        Color32::from_rgb(240, 240, 240)
    } else {
        Color32::from_rgb(80, 90, 84)
    };
    // faint filled area under composite
    for pair in composite.windows(2) {
        painter.add(egui::Shape::convex_polygon(
            vec![
                pair[0],
                pair[1],
                egui::pos2(pair[1].x, zero_y),
                egui::pos2(pair[0].x, zero_y),
            ],
            curve_color.gamma_multiply(0.07),
            Stroke::NONE,
        ));
    }
    painter.add(egui::Shape::line(
        composite.clone(),
        Stroke::new(1.8, curve_color),
    ));

    // ── band handles ─────────────────────────────────────────────────────
    for (bi, band) in bands.iter().enumerate() {
        if !band.enabled {
            continue;
        }
        // Place handle at composite y for this band's frequency
        let handle_x = x_at(band.frequency);
        let composite_y = {
            let t = ((handle_x - plot.left()) / plot.width()).clamp(0.0, 1.0);
            let idx = ((t * 180.0) as usize).min(179);
            let t_frac = t * 180.0 - idx as f32;
            let y0 = composite[idx].y;
            let y1 = composite[(idx + 1).min(180)].y;
            y0 + (y1 - y0) * t_frac
        };
        let center = egui::pos2(handle_x, composite_y);
        let col = if enabled {
            band_color(band.frequency)
        } else {
            Color32::from_rgb(70, 80, 75)
        };
        painter.circle_filled(center, 7.0, col);
        painter.circle_stroke(
            center,
            7.0,
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 0, 0, 160)),
        );
        // Frequency label below handle
        let freq_label = if band.frequency >= 1000.0 {
            format!("{:.1}k", band.frequency / 1000.0)
        } else {
            format!("{:.0}Hz", band.frequency)
        };
        let label_y = center.y + 11.0;
        if label_y < plot.bottom() - 2.0 {
            painter.text(
                egui::pos2(center.x, label_y),
                Align2::CENTER_TOP,
                &freq_label,
                FontId::monospace(8.0),
                col,
            );
        }
        // Band index number inside handle
        let _ = bi; // not drawing index number — keep clean
    }

    // ── interaction: click / drag ─────────────────────────────────────────
    let active_id = response.id.with("active-band");
    let mut changed = false;

    // Helper: find nearest enabled band handle position on composite curve
    let handle_pos = |band: &ExpertBand| -> Pos2 {
        let hx = x_at(band.frequency);
        let t = ((hx - plot.left()) / plot.width()).clamp(0.0, 1.0);
        let idx = ((t * 180.0) as usize).min(179);
        let t_frac = t * 180.0 - idx as f32;
        let y0 = composite[idx].y;
        let y1 = composite[(idx + 1).min(180)].y;
        egui::pos2(hx, y0 + (y1 - y0) * t_frac)
    };

    if enabled && (response.clicked() || response.drag_started()) {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = bands
                .iter()
                .enumerate()
                .filter(|(_, band)| band.enabled)
                .map(|(index, band)| {
                    let center = handle_pos(band);
                    (index, center.distance(pointer))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((index, distance)) = nearest {
                if distance < 24.0 {
                    ui.data_mut(|data| data.insert_temp(active_id, index));
                }
            }
        }
    }

    if enabled && response.dragged() {
        let pointer = response.interact_pointer_pos();
        let active = ui.data_mut(|data| data.get_temp::<usize>(active_id));
        if let (Some(pointer), Some(index)) = (pointer, active) {
            if let Some(band) = bands.get_mut(index) {
                band.frequency = freq_at(pointer.x).clamp(20.0, 20_000.0);
                band.gain = gain_at(pointer.y).clamp(crate::dsp::MIN_GAIN, crate::dsp::MAX_GAIN);
                changed = true;
            }
        }
    }

    if response.drag_stopped() || response.clicked() {
        ui.data_mut(|data| data.remove::<usize>(active_id));
    }

    // ── scroll wheel on handle → change Q ────────────────────────────────
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > 0.1 {
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let nearest = bands
                    .iter()
                    .enumerate()
                    .filter(|(_, band)| band.enabled)
                    .map(|(index, band)| {
                        let center = handle_pos(band);
                        (index, center.distance(pointer))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((index, dist)) = nearest {
                    if dist < 24.0 {
                        if let Some(band) = bands.get_mut(index) {
                            band.q = (band.q + (scroll_delta * 0.04) as f64).clamp(0.1, 20.0);
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // ── plot border on top of everything ─────────────────────────────────
    painter.rect_stroke(
        plot,
        2.0,
        Stroke::new(1.0, Color32::from_rgb(50, 62, 54)),
        StrokeKind::Outside,
    );

    changed
}

fn band_response(band: &ExpertBand, frequency: f64) -> f64 {
    if !band.enabled {
        return 0.0;
    }
    let ratio = frequency / band.frequency.max(10.0);
    let octaves = ratio.log2();
    let spread = (1.0 / band.q.clamp(0.05, 30.0)).clamp(0.08, 5.0) * 0.75;
    match band.filter_type {
        FilterType::Peaking => band.gain * (-0.5 * (octaves / spread).powi(2)).exp(),
        FilterType::LowShelf => band.gain / (1.0 + (octaves / spread).exp()),
        FilterType::HighShelf => band.gain - band.gain / (1.0 + (octaves / spread).exp()),
        FilterType::Notch => -band.gain.abs() * (-0.5 * (octaves / spread).powi(2)).exp(),
        FilterType::LowPass | FilterType::HighPass | FilterType::BandPass => 0.0,
    }
}
