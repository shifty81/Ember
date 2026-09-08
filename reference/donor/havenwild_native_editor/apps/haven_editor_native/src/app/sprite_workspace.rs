use super::render_helpers::*;
use super::*;

pub(crate) const SPRITE_TOOLBAR_HEIGHT: f32 = 36.0;
pub(crate) const SPRITE_BOTTOM_DOCK_HEIGHT: f32 = 78.0;
pub(crate) const SPRITE_SWATCH_SIZE: f32 = 24.0;
pub(crate) const SPRITE_SWATCH_GAP: f32 = 5.0;

pub(crate) fn draw_sprite_toolbar_background(rect: Rect) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        editor_theme::colors::PANEL_RAISED,
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
}

pub(crate) fn draw_sprite_toolbar_separator(x: f32, rect: Rect) {
    draw_line(x, rect.y + 6.0, x, rect.y + rect.h - 6.0, 1.0, PANEL_EDGE);
}

pub(crate) fn draw_sprite_panel(rect: Rect, title: &str, context: Option<&str>) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL_BG);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        30.0,
        editor_theme::colors::PANEL_RAISED,
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
    draw_editor_text(title, rect.x + 10.0, rect.y + 21.0, 15.0, TEXT);
    if let Some(context) = context {
        let measured = measure_editor_text(context, None, 12, 1.0).width;
        draw_scissored_text(
            context,
            (rect.x + rect.w - measured - 10.0).max(rect.x + 92.0),
            rect.y + 20.0,
            (rect.w - 104.0).max(24.0),
            12.0,
            MUTED,
        );
    }
    draw_line(
        rect.x,
        rect.y + 30.0,
        rect.x + rect.w,
        rect.y + 30.0,
        1.0,
        PANEL_EDGE,
    );
}

pub(crate) fn draw_sprite_bottom_dock(
    rect: Rect,
    active_tab: &str,
    secondary_tab: &str,
    colors: &[[u8; 4]],
    selected: [u8; 4],
) {
    draw_sprite_panel(rect, active_tab, Some(secondary_tab));
    let y = rect.y + 38.0;
    let available = (rect.w - 20.0).max(0.0);
    let swatch = SPRITE_SWATCH_SIZE;
    let gap = SPRITE_SWATCH_GAP;
    let visible = ((available + gap) / (swatch + gap)).floor().max(0.0) as usize;
    for (index, rgba) in colors.iter().take(visible).enumerate() {
        let swatch_rect = sprite_bottom_swatch_rect(rect, index);
        let x = swatch_rect.x;
        let color = Color::new(
            rgba[0] as f32 / 255.0,
            rgba[1] as f32 / 255.0,
            rgba[2] as f32 / 255.0,
            rgba[3] as f32 / 255.0,
        );
        if rgba[3] < 255 {
            draw_sprite_checkerboard(Rect::new(x, y, swatch, swatch), 6.0);
        }
        draw_rectangle(x, y, swatch, swatch, color);
        let is_selected = *rgba == selected;
        draw_rectangle_lines(
            x - if is_selected { 2.0 } else { 0.0 },
            y - if is_selected { 2.0 } else { 0.0 },
            swatch + if is_selected { 4.0 } else { 0.0 },
            swatch + if is_selected { 4.0 } else { 0.0 },
            if is_selected { 2.0 } else { 1.0 },
            if is_selected { ACCENT } else { PANEL_EDGE },
        );
    }
}

fn draw_sprite_checkerboard(rect: Rect, cell: f32) {
    let cols = (rect.w / cell).ceil() as usize;
    let rows = (rect.h / cell).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            let x = rect.x + col as f32 * cell;
            let y = rect.y + row as f32 * cell;
            let w = cell.min(rect.x + rect.w - x);
            let h = cell.min(rect.y + rect.h - y);
            let color = if (row + col) % 2 == 0 {
                editor_theme::colors::CHECKER_LIGHT
            } else {
                editor_theme::colors::CHECKER_DARK
            };
            draw_rectangle(x, y, w, h, color);
        }
    }
}

pub(crate) fn sprite_bottom_swatch_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + 10.0 + index as f32 * (SPRITE_SWATCH_SIZE + SPRITE_SWATCH_GAP),
        rect.y + 38.0,
        SPRITE_SWATCH_SIZE,
        SPRITE_SWATCH_SIZE,
    )
}

pub(crate) fn draw_sprite_tool_options(
    rect: Rect,
    primary_tool: &str,
    secondary_tool: &str,
    opacity: u8,
) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        editor_theme::colors::PANEL_RAISED,
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
    draw_scissored_text(
        &format!("L: {primary_tool}"),
        rect.x + 8.0,
        rect.y + 17.0,
        rect.w * 0.32,
        12.0,
        TEXT,
    );
    draw_scissored_text(
        &format!("R: {secondary_tool}"),
        rect.x + rect.w * 0.34,
        rect.y + 17.0,
        rect.w * 0.32,
        12.0,
        TEXT,
    );
    draw_scissored_text(
        &format!("Opacity: {}%", opacity as u32 * 100 / 255),
        rect.x + rect.w * 0.68,
        rect.y + 17.0,
        rect.w * 0.30 - 8.0,
        12.0,
        MUTED,
    );
}

pub(crate) fn draw_sprite_layer_row(
    row_rect: Rect,
    name: &str,
    blend_mode: &str,
    active: bool,
    visible: bool,
    locked: bool,
) {
    draw_list_row(row_rect, "", None, active);
    let state = if !visible {
        "Hidden"
    } else if locked {
        "Locked"
    } else {
        "Visible"
    };
    draw_scissored_text(
        name,
        row_rect.x + 76.0,
        row_rect.y + 20.0,
        row_rect.w - 158.0,
        14.0,
        TEXT,
    );
    draw_scissored_text(
        state,
        row_rect.x + row_rect.w - 150.0,
        row_rect.y + 20.0,
        68.0,
        10.0,
        MUTED,
    );
    draw_scissored_text(
        blend_mode,
        row_rect.x + row_rect.w - 76.0,
        row_rect.y + 20.0,
        68.0,
        10.0,
        MUTED,
    );
}
