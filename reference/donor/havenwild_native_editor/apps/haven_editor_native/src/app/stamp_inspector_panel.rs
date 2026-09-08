use super::*;

pub(crate) fn stamp_resize_rect(rect: Rect, index: usize) -> Rect {
    let width = (rect.w - 6.0) * 0.5;
    let column = index % 2;
    let row = index / 2;
    Rect::new(
        rect.x + column as f32 * (width + 6.0),
        rect.y + 294.0 + row as f32 * 34.0,
        width,
        28.0,
    )
}

pub(crate) fn stamp_reset_size_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 362.0, rect.w, 28.0)
}

pub(crate) fn stamp_action_rect(rect: Rect, index: usize) -> Rect {
    let width = (rect.w - 6.0) * 0.5;
    Rect::new(
        rect.x + index as f32 * (width + 6.0),
        rect.y + 462.0,
        width,
        30.0,
    )
}

pub(crate) fn set_rectangular_stamp_size(stamp: &mut PlacedStamp, width: i32, height: i32) {
    let width = width.max(1);
    let height = height.max(1);
    stamp.footprint.visual_offset_x = -(width / 2);
    stamp.footprint.visual_offset_y = -(height.saturating_sub(1));
    stamp.footprint.visual_w = width;
    stamp.footprint.visual_h = height;
    if stamp.footprint.collision_w > 0 || stamp.footprint.collision_h > 0 {
        stamp.footprint.collision_offset_x = -(width / 2);
        stamp.footprint.collision_offset_y = -(height.saturating_sub(1));
        stamp.footprint.collision_w = width;
        stamp.footprint.collision_h = height;
    }
    if stamp.footprint.interaction_w > 0 || stamp.footprint.interaction_h > 0 {
        stamp.footprint.interaction_offset_x = -(width / 2);
        stamp.footprint.interaction_offset_y = -(height.saturating_sub(1));
        stamp.footprint.interaction_w = width;
        stamp.footprint.interaction_h = height;
    }
}

pub(crate) fn stamp_footprint_label(stamp: &PlacedStamp) -> String {
    let (vx, vy, vw, vh) = stamp.visual_rect();
    let (cx, cy, cw, ch) = stamp.collision_rect();
    let (ix, iy, iw, ih) = stamp.interaction_rect();
    format!(
        "Visual {}x{} at {},{} | collision {}x{} at {},{} | interaction {}x{} at {},{} | blocks {}",
        vw,
        vh,
        vx,
        vy,
        cw,
        ch,
        cx,
        cy,
        iw,
        ih,
        ix,
        iy,
        if stamp.footprint.blocks_movement {
            "yes"
        } else {
            "no"
        }
    )
}
