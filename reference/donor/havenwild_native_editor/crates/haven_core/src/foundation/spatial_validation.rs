use super::*;

pub(super) fn validate_footprint_bounds(
    map: &TavernMap,
    anchor_x: i32,
    anchor_y: i32,
    footprint: ObjectFootprint,
    issues: &mut Vec<PlacementIssue>,
) {
    let visual = (
        anchor_x + footprint.visual_offset_x,
        anchor_y + footprint.visual_offset_y,
        footprint.visual_w,
        footprint.visual_h,
    );
    validate_rect_bounds(map, visual, "visual footprint", issues);

    let collision = (
        anchor_x + footprint.collision_offset_x,
        anchor_y + footprint.collision_offset_y,
        footprint.collision_w,
        footprint.collision_h,
    );
    validate_rect_bounds(map, collision, "collision footprint", issues);
    if footprint.blocks_movement {
        let (x, y, w, h) = collision;
        for cell_y in y..y + h.max(0) {
            for cell_x in x..x + w.max(0) {
                if TavernMap::idx(cell_x, cell_y).is_some() && !map.get(cell_x, cell_y).walkable() {
                    issues.push(PlacementIssue::new(
                        cell_x,
                        cell_y,
                        "collision footprint on blocked tile",
                    ));
                }
            }
        }
    }

    let interaction = (
        anchor_x + footprint.interaction_offset_x,
        anchor_y + footprint.interaction_offset_y,
        footprint.interaction_w,
        footprint.interaction_h,
    );
    validate_rect_bounds(map, interaction, "interaction footprint", issues);
}

pub(super) fn validate_rect_bounds(
    _map: &TavernMap,
    rect: (i32, i32, i32, i32),
    label: &str,
    issues: &mut Vec<PlacementIssue>,
) {
    let (x, y, w, h) = rect;
    if w < 0 || h < 0 {
        issues.push(PlacementIssue::new(x, y, format!("negative {label}")));
        return;
    }
    if w == 0 || h == 0 {
        return;
    }
    for cell_y in y..y + h {
        for cell_x in x..x + w {
            if TavernMap::idx(cell_x, cell_y).is_none() {
                issues.push(PlacementIssue::new(
                    cell_x,
                    cell_y,
                    format!("{label} outside scene"),
                ));
            }
        }
    }
}

pub(super) fn spatial_footprints_overlap(
    a: ObjectFootprint,
    ax: i32,
    ay: i32,
    b: ObjectFootprint,
    bx: i32,
    by: i32,
) -> bool {
    let a_visual = (
        ax + a.visual_offset_x,
        ay + a.visual_offset_y,
        a.visual_w,
        a.visual_h,
    );
    let a_collision = (
        ax + a.collision_offset_x,
        ay + a.collision_offset_y,
        a.collision_w,
        a.collision_h,
    );
    let a_interaction = (
        ax + a.interaction_offset_x,
        ay + a.interaction_offset_y,
        a.interaction_w,
        a.interaction_h,
    );
    let b_visual = (
        bx + b.visual_offset_x,
        by + b.visual_offset_y,
        b.visual_w,
        b.visual_h,
    );
    let b_collision = (
        bx + b.collision_offset_x,
        by + b.collision_offset_y,
        b.collision_w,
        b.collision_h,
    );
    let b_interaction = (
        bx + b.interaction_offset_x,
        by + b.interaction_offset_y,
        b.interaction_w,
        b.interaction_h,
    );
    rects_overlap(a_collision, b_collision)
        || rects_overlap(a_interaction, b_interaction)
        || rects_overlap(a_visual, b_visual)
}

pub(super) fn objects_overlap(a: PlacedObject, b: PlacedObject) -> bool {
    rects_overlap(a.collision_rect(), b.collision_rect())
        || rects_overlap(a.interaction_rect(), b.interaction_rect())
        || rects_overlap(a.visual_rect(), b.visual_rect())
}

pub(super) fn rects_overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    aw > 0
        && ah > 0
        && bw > 0
        && bh > 0
        && ax < bx + bw
        && ax + aw > bx
        && ay < by + bh
        && ay + ah > by
}
