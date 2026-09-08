use super::*;

pub(super) fn scene_dock_tab_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + index as f32 * (rect.w / 5.0),
        rect.y,
        rect.w / 5.0 - 4.0,
        30.0,
    )
}

pub(super) fn footprint_target_rect(rect: Rect, index: usize) -> Rect {
    let widths = [80.0, 88.0, 104.0];
    let x = rect.x + widths[..index].iter().sum::<f32>() + index as f32 * 5.0;
    Rect::new(x, rect.y + 154.0, widths[index], 28.0)
}

pub(super) fn property_row_y(rect: Rect, row: usize) -> f32 {
    rect.y + 202.0 + row as f32 * 42.0
}

pub(super) fn property_minus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 82.0,
        property_row_y(rect, row),
        34.0,
        28.0,
    )
}

pub(super) fn property_plus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 40.0,
        property_row_y(rect, row),
        34.0,
        28.0,
    )
}

pub(super) fn draw_property_row(rect: Rect, row: usize, label: &str, value: i32) {
    let y = property_row_y(rect, row);
    draw_editor_text(
        &format!("{}: {}", label, value),
        rect.x,
        y + 20.0,
        16.0,
        TEXT,
    );
    draw_editor_widget(property_minus_rect(rect, row), "-", false);
    draw_editor_widget(property_plus_rect(rect, row), "+", false);
}

pub(super) fn behavior_button_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(rect.x, rect.y + 394.0 + index as f32 * 34.0, rect.w, 28.0)
}

pub(super) fn object_action_rect(rect: Rect, index: usize) -> Rect {
    let col = index % 2;
    let row = index / 2;
    let width = (rect.w - 6.0) * 0.5;
    Rect::new(
        rect.x + col as f32 * (width + 6.0),
        rect.y + 484.0 + row as f32 * 34.0,
        width,
        28.0,
    )
}

pub(super) fn footprint_values(object: PlacedObject, target: FootprintEditTarget) -> (i32, i32, i32, i32) {
    match target {
        FootprintEditTarget::Visual => (
            object.footprint.visual_offset_x,
            object.footprint.visual_offset_y,
            object.footprint.visual_w,
            object.footprint.visual_h,
        ),
        FootprintEditTarget::Collision => (
            object.footprint.collision_offset_x,
            object.footprint.collision_offset_y,
            object.footprint.collision_w,
            object.footprint.collision_h,
        ),
        FootprintEditTarget::Interaction => (
            object.footprint.interaction_offset_x,
            object.footprint.interaction_offset_y,
            object.footprint.interaction_w,
            object.footprint.interaction_h,
        ),
    }
}

pub(super) fn draw_footprint_rect((x, y, w, h): (i32, i32, i32, i32), color: Color) {
    if w <= 0 || h <= 0 {
        return;
    }
    draw_rectangle(
        x as f32,
        y as f32,
        w as f32,
        h as f32,
        Color::new(color.r, color.g, color.b, 0.08),
    );
    draw_rectangle_lines(x as f32, y as f32, w as f32, h as f32, 0.10, color);
}
