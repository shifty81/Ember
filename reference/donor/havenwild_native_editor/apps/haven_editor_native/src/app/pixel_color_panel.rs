use super::pixel_studio::PALETTE;
use super::render_helpers::*;
use super::*;
use std::f32::consts::TAU;

pub(crate) fn pixel_color_tab_rect(rect: Rect) -> Rect {
    let width = (rect.w - 18.0) / 4.0;
    Rect::new(rect.x + (width + 6.0) * 3.0, rect.y + 34.0, width, 30.0)
}

pub(crate) fn color_wheel_center(rect: Rect) -> Vec2 {
    vec2(rect.x + rect.w * 0.43, rect.y + 165.0)
}

pub(crate) fn color_wheel_outer_radius(rect: Rect) -> f32 {
    (rect.w * 0.29).clamp(54.0, 82.0)
}

pub(crate) fn color_value_rect(rect: Rect) -> Rect {
    let radius = color_wheel_outer_radius(rect);
    let center = color_wheel_center(rect);
    Rect::new(
        center.x + radius + 14.0,
        center.y - radius,
        22.0,
        radius * 2.0,
    )
}

pub(crate) fn color_foreground_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 270.0, 72.0, 52.0)
}

pub(crate) fn color_background_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 82.0, rect.y + 282.0, 72.0, 52.0)
}

pub(crate) fn color_swap_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 166.0, rect.y + 274.0, 54.0, 28.0)
}

pub(crate) fn color_reset_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 226.0, rect.y + 274.0, 60.0, 28.0)
}

pub(crate) fn color_channel_minus_rect(rect: Rect, channel: usize) -> Rect {
    Rect::new(
        rect.x + 116.0,
        rect.y + 368.0 + channel as f32 * 32.0,
        30.0,
        26.0,
    )
}

pub(crate) fn color_channel_plus_rect(rect: Rect, channel: usize) -> Rect {
    Rect::new(
        rect.x + 222.0,
        rect.y + 368.0 + channel as f32 * 32.0,
        30.0,
        26.0,
    )
}

pub(crate) fn color_palette_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + (index % 8) as f32 * 34.0,
        rect.y + 540.0 + (index / 8) as f32 * 34.0,
        28.0,
        28.0,
    )
}

pub(crate) fn draw_pixel_color_panel(app: &EditorApp, rect: Rect) {
    let fg = app.pixel_studio.selected_color;
    let bg = app.pixel_studio.background_color;
    let (hue, saturation, value) = rgba_to_hsv(fg);
    let center = color_wheel_center(rect);
    let outer = color_wheel_outer_radius(rect);
    let inner = outer * 0.70;

    draw_editor_text("Color", rect.x, rect.y + 92.0, 18.0, TEXT);

    let segments = 72;
    for segment in 0..segments {
        let a0 = segment as f32 / segments as f32 * TAU;
        let a1 = (segment + 1) as f32 / segments as f32 * TAU;
        let color = hsv_to_color(segment as f32 / segments as f32, 1.0, 1.0, 1.0);
        draw_triangle(
            center + vec2(a0.cos(), a0.sin()) * inner,
            center + vec2(a0.cos(), a0.sin()) * outer,
            center + vec2(a1.cos(), a1.sin()) * outer,
            color,
        );
        draw_triangle(
            center + vec2(a0.cos(), a0.sin()) * inner,
            center + vec2(a1.cos(), a1.sin()) * outer,
            center + vec2(a1.cos(), a1.sin()) * inner,
            color,
        );
    }

    // Saturation disc for the active hue. It is deliberately stepped so the
    // editor stays dependency-free and pixel-art friendly.
    for ring in (1..=18).rev() {
        let s = ring as f32 / 18.0;
        draw_circle(
            center.x,
            center.y,
            inner * s,
            hsv_to_color(hue, s, value, 1.0),
        );
    }
    draw_circle(center.x, center.y, 2.0, hsv_to_color(hue, 0.0, value, 1.0));

    let hue_angle = hue * TAU;
    let hue_marker = center + vec2(hue_angle.cos(), hue_angle.sin()) * ((inner + outer) * 0.5);
    draw_circle_lines(hue_marker.x, hue_marker.y, 5.0, 2.0, WHITE);
    let sat_marker = center + vec2(1.0, 0.0) * inner * saturation;
    draw_circle_lines(sat_marker.x, sat_marker.y, 5.0, 2.0, WHITE);

    let value_rect = color_value_rect(rect);
    let steps = 32;
    for step in 0..steps {
        let t0 = step as f32 / steps as f32;
        let t1 = (step + 1) as f32 / steps as f32;
        draw_rectangle(
            value_rect.x,
            value_rect.y + t0 * value_rect.h,
            value_rect.w,
            (t1 - t0) * value_rect.h + 1.0,
            hsv_to_color(hue, saturation, 1.0 - t0, 1.0),
        );
    }
    draw_rectangle_lines(
        value_rect.x,
        value_rect.y,
        value_rect.w,
        value_rect.h,
        1.0,
        PANEL_EDGE,
    );
    let value_y = value_rect.y + (1.0 - value) * value_rect.h;
    draw_line(
        value_rect.x - 3.0,
        value_y,
        value_rect.x + value_rect.w + 3.0,
        value_y,
        2.0,
        WHITE,
    );

    draw_color_swatch(color_background_rect(rect), bg, false, "BG");
    draw_color_swatch(color_foreground_rect(rect), fg, true, "FG");
    draw_editor_widget(color_swap_rect(rect), "Swap", false);
    draw_editor_widget(color_reset_rect(rect), "Reset", false);

    draw_editor_text(
        &format!("#{:02X}{:02X}{:02X}{:02X}", fg[0], fg[1], fg[2], fg[3]),
        rect.x,
        rect.y + 356.0,
        18.0,
        TEXT,
    );
    for (channel, label) in ["R", "G", "B", "A"].into_iter().enumerate() {
        draw_editor_text(
            label,
            rect.x,
            rect.y + 388.0 + channel as f32 * 32.0,
            15.0,
            MUTED,
        );
        draw_editor_widget(color_channel_minus_rect(rect, channel), "-", false);
        draw_scissored_text(
            &fg[channel].to_string(),
            rect.x + 158.0,
            rect.y + 388.0 + channel as f32 * 32.0,
            52.0,
            15.0,
            TEXT,
        );
        draw_editor_widget(color_channel_plus_rect(rect, channel), "+", false);
    }

    draw_editor_text("Project Palette", rect.x, rect.y + 526.0, 18.0, TEXT);
    for (index, color) in PALETTE.into_iter().enumerate() {
        let swatch = color_palette_rect(rect, index);
        draw_rectangle(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            Color::from_rgba(color[0], color[1], color[2], color[3]),
        );
        draw_rectangle_lines(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            if color == fg || color == bg { 3.0 } else { 1.0 },
            if color == fg {
                TEXT
            } else if color == bg {
                ACCENT
            } else {
                PANEL_EDGE
            },
        );
    }
    draw_wrapped(
        "Left click assigns foreground. Right click assigns background. X swaps colors; D resets them.",
        rect.x,
        rect.y + 620.0,
        rect.w,
        14.0,
        MUTED,
    );
}

fn draw_color_checkerboard(rect: Rect, cell: f32) {
    let columns = (rect.w / cell).ceil() as i32;
    let rows = (rect.h / cell).ceil() as i32;
    for y in 0..rows {
        for x in 0..columns {
            let color = if (x + y) % 2 == 0 {
                editor_theme::colors::CHECKER_LIGHT
            } else {
                editor_theme::colors::CHECKER_DARK
            };
            draw_rectangle(
                rect.x + x as f32 * cell,
                rect.y + y as f32 * cell,
                cell.min(rect.x + rect.w - (rect.x + x as f32 * cell)),
                cell.min(rect.y + rect.h - (rect.y + y as f32 * cell)),
                color,
            );
        }
    }
}

fn draw_color_swatch(rect: Rect, color: [u8; 4], active: bool, label: &str) {
    draw_color_checkerboard(rect, 8.0);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::from_rgba(color[0], color[1], color[2], color[3]),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active { 3.0 } else { 1.0 },
        if active { ACCENT } else { PANEL_EDGE },
    );
    draw_editor_text(label, rect.x + 5.0, rect.y + rect.h - 6.0, 14.0, TEXT);
}

pub(crate) fn update_color_from_pointer(app: &mut EditorApp, mouse: Vec2, rect: Rect) -> bool {
    let center = color_wheel_center(rect);
    let outer = color_wheel_outer_radius(rect);
    let inner = outer * 0.70;
    let delta = mouse - center;
    let distance = delta.length();
    let current = app.pixel_studio.selected_color;
    let (mut hue, mut saturation, mut value) = rgba_to_hsv(current);
    if distance >= inner && distance <= outer {
        hue = delta.y.atan2(delta.x).rem_euclid(TAU) / TAU;
    } else if distance < inner {
        saturation = (distance / inner).clamp(0.0, 1.0);
    } else if color_value_rect(rect).contains(mouse) {
        let value_rect = color_value_rect(rect);
        value = 1.0 - ((mouse.y - value_rect.y) / value_rect.h).clamp(0.0, 1.0);
    } else {
        return false;
    }
    let alpha = current[3];
    let color = hsv_to_rgba(hue, saturation, value, alpha);
    app.pixel_studio.selected_color = color;
    true
}

pub(crate) fn rgba_to_hsv(color: [u8; 4]) -> (f32, f32, f32) {
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        (((b - r) / delta) + 2.0) / 6.0
    } else {
        (((r - g) / delta) + 4.0) / 6.0
    };
    let saturation = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, saturation, max)
}

fn hsv_to_color(h: f32, s: f32, v: f32, a: f32) -> Color {
    let rgba = hsv_to_rgba(h, s, v, (a * 255.0).round() as u8);
    Color::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn hsv_to_rgba(h: f32, s: f32, v: f32, alpha: u8) -> [u8; 4] {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let i = h6.floor() as i32;
    let f = h6 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
        alpha,
    ]
}
