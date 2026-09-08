use haven_core::{GameWorld, ProjectSceneId, SceneMap, MAP_H, MAP_W};
use haven_editor::{
    visible_grid_bounds_in_world, CanvasPoint as AuthoringCanvasPoint,
    CanvasRect as AuthoringCanvasRect, GridRect,
};
use haven_world::scene_rectangles::{
    SceneRectangleAssignmentsFile, SceneRectangleManifest, SceneRectangleSpec,
};
use macroquad::prelude::*;

use super::canvas_camera::CanvasCameraState;
use super::canvas_view::{draw_canvas_rulers, draw_infinite_grid};
use super::editor_text::draw_editor_text;
use super::render_helpers::{scene_tile_color, zone_preview_color};
use super::{WorldEditTool, WorldLayerMode, MUTED, PANEL_EDGE, TEXT, WARN};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WorldSurfaceViewOptions {
    pub selected_index: usize,
    pub selected_landmass_id: i32,
    pub show_partitions: bool,
    pub show_objects: bool,
    pub show_zones: bool,
    pub show_structural_levels: bool,
    pub cursor: Option<(i32, i32)>,
    pub selection: Option<GridRect>,
    pub drag_preview: Option<GridRect>,
    pub edit_tool: WorldEditTool,
    pub layer_mode: WorldLayerMode,
    pub brush_radius: i32,
}

const WORLD_SURFACE_PARTITION_W: f32 = MAP_W as f32;
const WORLD_SURFACE_PARTITION_H: f32 = MAP_H as f32;

pub(crate) fn draw_scene_rectangle_map(
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    world: &GameWorld,
    viewport: Rect,
    canvas: &CanvasCameraState,
    options: WorldSurfaceViewOptions,
) {
    draw_rectangle(
        viewport.x,
        viewport.y,
        viewport.w,
        viewport.h,
        Color::new(0.10, 0.13, 0.15, 1.0),
    );
    let Some(bounds) = world_scene_grid_bounds_for_landmass(manifest, options.selected_landmass_id)
    else {
        draw_editor_text(
            "No global world-surface data available",
            viewport.x + 16.0,
            viewport.y + 36.0,
            22.0,
            WARN,
        );
        return;
    };

    let camera = canvas.camera(viewport, bounds);
    set_camera(&camera);
    let visible = canvas.visible_world_rect(viewport, bounds);
    let pixels_per_tile = viewport.w / visible.w.max(1.0);
    draw_rectangle(
        visible.x,
        visible.y,
        visible.w,
        visible.h,
        Color::new(0.08, 0.15, 0.18, 1.0),
    );

    for (index, rectangle) in manifest.scene_rectangles.iter().enumerate() {
        if !rectangle_is_overworld_surface(rectangle)
            || rectangle.landmass_id != options.selected_landmass_id
        {
            continue;
        }
        let target = world_scene_grid_rect(manifest, rectangle);
        if !rects_intersect(target, visible) {
            continue;
        }
        let assigned_scene = assignments
            .assignment_for_rectangle(&rectangle.scene_id)
            .and_then(|assignment| {
                world.scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))
            });
        if let Some(scene) = assigned_scene {
            draw_scene_surface_into_rect(scene, target, visible, pixels_per_tile);
            if options.show_zones {
                draw_scene_zone_overlay(scene, target, visible, pixels_per_tile);
            }
            if options.show_structural_levels {
                draw_scene_structural_level_overlay(scene, target, visible, pixels_per_tile);
            }
            if options.show_objects {
                draw_scene_object_overlay(scene, target, visible, pixels_per_tile);
            }
        } else {
            draw_rectangle(
                target.x,
                target.y,
                target.w,
                target.h,
                Color::new(0.16, 0.25, 0.26, 0.94),
            );
            draw_editor_text(
                "generation pending",
                target.x + 5.0,
                target.y + 17.0,
                12.0,
                WARN,
            );
        }

        if options.show_partitions {
            let selected = index == options.selected_index;
            draw_rectangle_lines(
                target.x,
                target.y,
                target.w,
                target.h,
                if selected { 1.8 } else { 0.75 },
                if selected { TEXT } else { PANEL_EDGE },
            );
            draw_rectangle(
                target.x,
                target.y,
                32.0_f32.min(target.w),
                4.0,
                Color::new(0.02, 0.03, 0.04, 0.72),
            );
            draw_editor_text(
                &format!(
                    "{},{}",
                    rectangle.grid_x.unwrap_or(0),
                    rectangle.grid_y.unwrap_or(0)
                ),
                target.x + 1.5,
                target.y + 3.2,
                3.5,
                TEXT,
            );
        }
    }

    if options.show_structural_levels && pixels_per_tile >= 2.5 {
        draw_structural_edge_diagnostics(
            manifest,
            assignments,
            world,
            options.selected_landmass_id,
            visible,
            canvas.zoom,
        );
    }
    if pixels_per_tile >= 6.0 {
        draw_infinite_grid(visible, 1.0, 8, canvas.zoom);
    }
    for (rect, color) in [
        (options.selection, Color::new(0.30, 0.76, 1.0, 0.82)),
        (options.drag_preview, WARN),
    ] {
        if let Some(rect) = rect {
            let width = rect.width().max(1) as f32;
            let height = rect.height().max(1) as f32;
            draw_rectangle(
                rect.min.x as f32,
                rect.min.y as f32,
                width,
                height,
                Color::new(color.r, color.g, color.b, 0.10),
            );
            draw_rectangle_lines(
                rect.min.x as f32,
                rect.min.y as f32,
                width,
                height,
                (0.14 / canvas.zoom.max(0.20)).min(0.20),
                color,
            );
        }
    }
    if let Some((cursor_x, cursor_y)) = options.cursor {
        if matches!(
            options.edit_tool,
            WorldEditTool::Paint | WorldEditTool::Erase
        ) {
            let radius = options.brush_radius.max(0);
            let brush = Rect::new(
                (cursor_x - radius) as f32,
                (cursor_y - radius) as f32,
                (radius * 2 + 1) as f32,
                (radius * 2 + 1) as f32,
            );
            draw_rectangle(
                brush.x,
                brush.y,
                brush.w,
                brush.h,
                Color::new(1.0, 0.78, 0.24, 0.06),
            );
            draw_rectangle_lines(
                brush.x,
                brush.y,
                brush.w,
                brush.h,
                (0.10 / canvas.zoom.max(0.20)).min(0.16),
                WARN,
            );
        }
        let cursor = Rect::new(cursor_x as f32, cursor_y as f32, 1.0, 1.0);
        if rects_intersect(cursor, visible) {
            draw_rectangle(
                cursor.x,
                cursor.y,
                cursor.w,
                cursor.h,
                Color::new(1.0, 0.78, 0.24, 0.18),
            );
            draw_rectangle_lines(
                cursor.x,
                cursor.y,
                cursor.w,
                cursor.h,
                (0.12 / canvas.zoom.max(0.20)).min(0.18),
                WARN,
            );
        }
    }
    set_default_camera();
    draw_canvas_rulers(viewport, visible, 1.0, "Tile");

    draw_rectangle_lines(
        viewport.x, viewport.y, viewport.w, viewport.h, 1.0, PANEL_EDGE,
    );
    draw_rectangle(
        viewport.x + 8.0,
        viewport.y + 8.0,
        430.0,
        44.0,
        Color::new(0.04, 0.05, 0.05, 0.78),
    );
    draw_editor_text(
        &format!(
            "Alderreach | {} layer | {} tool | brush {}x{}",
            options.layer_mode.label(),
            options.edit_tool.label(),
            options.brush_radius * 2 + 1,
            options.brush_radius * 2 + 1
        ),
        viewport.x + 16.0,
        viewport.y + 25.0,
        17.0,
        TEXT,
    );
    draw_editor_text(
        "Continuous global transactions; storage partitions are diagnostics only",
        viewport.x + 16.0,
        viewport.y + 44.0,
        13.0,
        MUTED,
    );
}

fn draw_structural_edge_diagnostics(
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    world: &GameWorld,
    landmass_id: i32,
    visible: Rect,
    zoom: f32,
) {
    let partition_index = manifest
        .scene_rectangles
        .iter()
        .filter(|entry| rectangle_is_overworld_surface(entry) && entry.landmass_id == landmass_id)
        .filter_map(|rectangle| {
            let assignment = assignments.assignment_for_rectangle(&rectangle.scene_id)?;
            let scene = world.scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))?;
            Some(((rectangle.grid_x?, rectangle.grid_y?), scene))
        })
        .collect::<std::collections::HashMap<_, _>>();

    let min_x = visible.x.floor() as i32;
    let min_y = visible.y.floor() as i32;
    let max_x = (visible.x + visible.w).ceil() as i32;
    let max_y = (visible.y + visible.h).ceil() as i32;
    let line_width = (0.12 / zoom.max(0.20)).min(0.18);
    let color = Color::new(1.0, 0.48, 0.20, 0.88);
    for y in min_y..max_y {
        for x in min_x..max_x {
            let Some(level) = indexed_global_surface_sample(&partition_index, x, y) else {
                continue;
            };
            if let Some(east_level) = indexed_global_surface_sample(&partition_index, x + 1, y) {
                if east_level != level {
                    draw_line(
                        (x + 1) as f32,
                        y as f32,
                        (x + 1) as f32,
                        (y + 1) as f32,
                        line_width,
                        color,
                    );
                }
            }
            if let Some(south_level) = indexed_global_surface_sample(&partition_index, x, y + 1) {
                if south_level != level {
                    draw_line(
                        x as f32,
                        (y + 1) as f32,
                        (x + 1) as f32,
                        (y + 1) as f32,
                        line_width,
                        color,
                    );
                }
            }
        }
    }
}

fn indexed_global_surface_sample(
    partition_index: &std::collections::HashMap<(i32, i32), &SceneMap>,
    global_x: i32,
    global_y: i32,
) -> Option<u8> {
    let partition_w = MAP_W as i32;
    let partition_h = MAP_H as i32;
    let partition = (
        global_x.div_euclid(partition_w),
        global_y.div_euclid(partition_h),
    );
    let local_x = global_x.rem_euclid(partition_w);
    let local_y = global_y.rem_euclid(partition_h);
    let scene = partition_index.get(&partition)?;
    Some(
        scene
            .map
            .get_structural_level(local_x, local_y)
            .unwrap_or(0),
    )
}

pub(crate) fn world_scene_grid_rect(
    _manifest: &SceneRectangleManifest,
    rectangle: &SceneRectangleSpec,
) -> Rect {
    Rect::new(
        rectangle.grid_x.unwrap_or(0) as f32 * WORLD_SURFACE_PARTITION_W,
        rectangle.grid_y.unwrap_or(0) as f32 * WORLD_SURFACE_PARTITION_H,
        WORLD_SURFACE_PARTITION_W,
        WORLD_SURFACE_PARTITION_H,
    )
}

pub(crate) fn world_scene_grid_bounds_for_landmass(
    manifest: &SceneRectangleManifest,
    landmass_id: i32,
) -> Option<Rect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for rectangle in manifest
        .scene_rectangles
        .iter()
        .filter(|entry| rectangle_is_overworld_surface(entry) && entry.landmass_id == landmass_id)
    {
        let rect = world_scene_grid_rect(manifest, rectangle);
        min_x = min_x.min(rect.x);
        min_y = min_y.min(rect.y);
        max_x = max_x.max(rect.x + rect.w);
        max_y = max_y.max(rect.y + rect.h);
    }
    if !min_x.is_finite() || !min_y.is_finite() {
        return None;
    }
    Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

pub(crate) fn rectangle_is_overworld_surface(rectangle: &SceneRectangleSpec) -> bool {
    rectangle.grid_x.is_some()
        && rectangle.grid_y.is_some()
        && !rectangle.kind.starts_with("special_")
}

fn rects_intersect(left: Rect, right: Rect) -> bool {
    left.x < right.x + right.w
        && left.x + left.w > right.x
        && left.y < right.y + right.h
        && left.y + left.h > right.y
}

fn world_preview_step(pixels_per_tile: f32) -> i32 {
    if pixels_per_tile >= 5.0 {
        1
    } else if pixels_per_tile >= 2.5 {
        2
    } else if pixels_per_tile >= 1.2 {
        4
    } else if pixels_per_tile >= 0.6 {
        8
    } else {
        12
    }
}

fn scene_visible_local_bounds(target: Rect, visible: Rect) -> (i32, i32, i32, i32) {
    let Some(bounds) = visible_grid_bounds_in_world(
        AuthoringCanvasRect::new(visible.x, visible.y, visible.w, visible.h),
        AuthoringCanvasPoint::new(target.x, target.y),
        1.0,
        MAP_W as i32,
        MAP_H as i32,
    ) else {
        return (0, 0, 0, 0);
    };
    (
        bounds.min.x,
        bounds.min.y,
        bounds.max.x + 1,
        bounds.max.y + 1,
    )
}

fn draw_scene_surface_into_rect(
    scene: &SceneMap,
    target: Rect,
    visible: Rect,
    pixels_per_tile: f32,
) {
    let step = world_preview_step(pixels_per_tile);
    let (min_x, min_y, max_x, max_y) = scene_visible_local_bounds(target, visible);
    if min_x >= max_x || min_y >= max_y {
        return;
    }
    let mut y = min_y;
    while y < max_y {
        let block_h = step.min(max_y - y);
        let mut run_start = min_x;
        let mut run_tile = scene.map.get(min_x, y);
        let mut x = min_x + step;
        while x < max_x {
            let next_tile = scene.map.get(x, y);
            if next_tile != run_tile {
                draw_rectangle(
                    target.x + run_start as f32,
                    target.y + y as f32,
                    (x - run_start) as f32 + 0.02,
                    block_h as f32 + 0.02,
                    scene_tile_color(run_tile),
                );
                run_start = x;
                run_tile = next_tile;
            }
            x += step;
        }
        draw_rectangle(
            target.x + run_start as f32,
            target.y + y as f32,
            (max_x - run_start) as f32 + 0.02,
            block_h as f32 + 0.02,
            scene_tile_color(run_tile),
        );
        y += step;
    }
}

fn draw_scene_zone_overlay(scene: &SceneMap, target: Rect, visible: Rect, pixels_per_tile: f32) {
    let step = world_preview_step(pixels_per_tile).max(2);
    let (min_x, min_y, max_x, max_y) = scene_visible_local_bounds(target, visible);
    let mut y = min_y;
    while y < max_y {
        let mut x = min_x;
        while x < max_x {
            if let Some(mut color) = zone_preview_color(scene.zone_at(x, y)) {
                color.a = color.a.max(0.22);
                draw_rectangle(
                    target.x + x as f32,
                    target.y + y as f32,
                    step.min(max_x - x) as f32,
                    step.min(max_y - y) as f32,
                    color,
                );
            }
            x += step;
        }
        y += step;
    }
}

fn draw_scene_structural_level_overlay(
    scene: &SceneMap,
    target: Rect,
    visible: Rect,
    pixels_per_tile: f32,
) {
    let step = world_preview_step(pixels_per_tile).max(2);
    let (min_x, min_y, max_x, max_y) = scene_visible_local_bounds(target, visible);
    let mut y = min_y;
    while y < max_y {
        let mut x = min_x;
        while x < max_x {
            let level = scene.map.get_structural_level(x, y).unwrap_or(0);
            if level > 0 {
                let alpha = (0.10 + level as f32 * 0.09).min(0.36);
                draw_rectangle(
                    target.x + x as f32,
                    target.y + y as f32,
                    step.min(max_x - x) as f32,
                    step.min(max_y - y) as f32,
                    Color::new(0.78, 0.42, 0.18, alpha),
                );
            }
            x += step;
        }
        y += step;
    }
}

fn draw_scene_object_overlay(scene: &SceneMap, target: Rect, visible: Rect, pixels_per_tile: f32) {
    let line = if pixels_per_tile >= 4.0 { 0.12 } else { 0.28 };
    for stamp in &scene.map.stamps {
        let (x, y, w, h) = stamp.visual_rect();
        let rect = Rect::new(target.x + x as f32, target.y + y as f32, w as f32, h as f32);
        if rects_intersect(rect, visible) {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w.max(1.0),
                rect.h.max(1.0),
                Color::new(0.26, 0.72, 0.94, 0.18),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w.max(1.0),
                rect.h.max(1.0),
                line,
                Color::new(0.42, 0.86, 1.0, 0.86),
            );
        }
    }
    for object in &scene.map.objects {
        let (x, y, w, h) = object.visual_rect();
        let rect = Rect::new(target.x + x as f32, target.y + y as f32, w as f32, h as f32);
        if rects_intersect(rect, visible) {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w.max(1.0),
                rect.h.max(1.0),
                Color::new(1.0, 0.78, 0.24, 0.16),
            );
            draw_rectangle_lines(
                rect.x,
                rect.y,
                rect.w.max(1.0),
                rect.h.max(1.0),
                line,
                Color::new(1.0, 0.86, 0.36, 0.90),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_intersection_rejects_offscreen_regions() {
        assert!(rects_intersect(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(5.0, 5.0, 10.0, 10.0),
        ));
        assert!(!rects_intersect(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(20.0, 20.0, 10.0, 10.0),
        ));
    }

    #[test]
    fn world_surface_partitions_use_true_tile_dimensions() {
        assert_eq!(WORLD_SURFACE_PARTITION_W, MAP_W as f32);
        assert_eq!(WORLD_SURFACE_PARTITION_H, MAP_H as f32);
        assert_ne!(WORLD_SURFACE_PARTITION_W, WORLD_SURFACE_PARTITION_H);
    }

    #[test]
    fn world_preview_lod_reaches_native_tiles_when_zoomed_in() {
        assert_eq!(world_preview_step(6.0), 1);
        assert_eq!(world_preview_step(3.0), 2);
        assert_eq!(world_preview_step(1.5), 4);
        assert_eq!(world_preview_step(0.8), 8);
        assert_eq!(world_preview_step(0.2), 12);
    }
}
