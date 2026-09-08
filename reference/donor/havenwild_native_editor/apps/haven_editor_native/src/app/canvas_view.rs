use haven_core::{SceneMap, MAP_H, MAP_W};
use macroquad::prelude::*;

use super::editor_text::{draw_editor_text, measure_editor_text};
use super::render_helpers::{draw_editor_widget, draw_scissored_text, scene_tile_color};
use super::world_surface_editor::WorldSurfaceViewOptions;
use super::{WorldEditTool, MUTED, PANEL_EDGE, TEXT};

pub(crate) const CANVAS_RULER_THICKNESS: f32 = 24.0;

pub(crate) fn draw_canvas_rulers(viewport: Rect, visible: Rect, major_step: f32, unit_label: &str) {
    if viewport.w <= 1.0 || viewport.h <= 1.0 || visible.w <= 0.0 || visible.h <= 0.0 {
        return;
    }
    let top = Rect::new(
        viewport.x,
        viewport.y - CANVAS_RULER_THICKNESS,
        viewport.w,
        CANVAS_RULER_THICKNESS,
    );
    let left = Rect::new(
        viewport.x - CANVAS_RULER_THICKNESS,
        viewport.y,
        CANVAS_RULER_THICKNESS,
        viewport.h,
    );
    draw_rectangle(
        top.x,
        top.y,
        top.w,
        top.h,
        Color::new(0.055, 0.075, 0.085, 1.0),
    );
    draw_rectangle(
        left.x,
        left.y,
        left.w,
        left.h,
        Color::new(0.055, 0.075, 0.085, 1.0),
    );
    draw_rectangle_lines(top.x, top.y, top.w, top.h, 1.0, PANEL_EDGE);
    draw_rectangle_lines(left.x, left.y, left.w, left.h, 1.0, PANEL_EDGE);

    let step = adaptive_ruler_step(major_step.max(0.0001), visible.w / viewport.w.max(1.0));
    let start_x = (visible.x / step).floor() as i32;
    let end_x = ((visible.x + visible.w) / step).ceil() as i32;
    for index in start_x..=end_x {
        let world_x = index as f32 * step;
        let screen_x = viewport.x + (world_x - visible.x) / visible.w * viewport.w;
        if screen_x < viewport.x - 1.0 || screen_x > viewport.x + viewport.w + 1.0 {
            continue;
        }
        draw_line(screen_x, top.y + 14.0, screen_x, top.y + top.h, 1.0, MUTED);
        draw_editor_text(
            &format_ruler_value(world_x),
            screen_x + 3.0,
            top.y + 12.0,
            11.0,
            MUTED,
        );
    }
    let start_y = (visible.y / step).floor() as i32;
    let end_y = ((visible.y + visible.h) / step).ceil() as i32;
    for index in start_y..=end_y {
        let world_y = index as f32 * step;
        let screen_y = viewport.y + (world_y - visible.y) / visible.h * viewport.h;
        if screen_y < viewport.y - 1.0 || screen_y > viewport.y + viewport.h + 1.0 {
            continue;
        }
        draw_line(
            left.x + 14.0,
            screen_y,
            left.x + left.w,
            screen_y,
            1.0,
            MUTED,
        );
        draw_editor_text(
            &format_ruler_value(world_y),
            left.x + 2.0,
            screen_y - 3.0,
            10.0,
            MUTED,
        );
    }

    let mouse = vec2(mouse_position().0, mouse_position().1);
    if viewport.contains(mouse) {
        let world_x = visible.x + (mouse.x - viewport.x) / viewport.w * visible.w;
        let world_y = visible.y + (mouse.y - viewport.y) / viewport.h * visible.h;
        draw_line(
            mouse.x,
            viewport.y,
            mouse.x,
            viewport.y + viewport.h,
            1.0,
            Color::new(0.96, 0.62, 0.30, 0.58),
        );
        draw_line(
            viewport.x,
            mouse.y,
            viewport.x + viewport.w,
            mouse.y,
            1.0,
            Color::new(0.96, 0.62, 0.30, 0.58),
        );
        let readout = format!(
            "{} {}, {}",
            unit_label,
            format_ruler_value(world_x),
            format_ruler_value(world_y)
        );
        let width = measure_editor_text(&readout, None, 13, 1.0).width + 12.0;
        let x = (mouse.x + 12.0).min(viewport.x + viewport.w - width - 4.0);
        let y = (mouse.y - 10.0).max(viewport.y + 18.0);
        draw_rectangle(x, y - 16.0, width, 20.0, Color::new(0.02, 0.03, 0.04, 0.90));
        draw_editor_text(&readout, x + 6.0, y, 13.0, TEXT);
    }
}

fn adaptive_ruler_step(base: f32, world_per_pixel: f32) -> f32 {
    let target = (world_per_pixel * 90.0).max(base);
    let exponent = target.log10().floor();
    let magnitude = 10.0_f32.powf(exponent);
    let normalized = target / magnitude;
    let snapped = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    (snapped * magnitude).max(base)
}

fn format_ruler_value(value: f32) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.1}k", value / 1000.0)
    } else if value.fract().abs() < 0.01 {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.1}")
    }
}

pub(crate) fn scene_tool_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 6.0;
    let width = (rect.w - gap * 2.0) / 3.0;
    let column = index % 3;
    let row = index / 3;
    Rect::new(
        rect.x + column as f32 * (width + gap),
        rect.y + 68.0 + row as f32 * 34.0,
        width,
        28.0,
    )
}

pub(crate) fn scene_layer_button_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x,
        rect.y + 184.0 + index as f32 * 31.0,
        rect.w - 70.0,
        26.0,
    )
}

pub(crate) fn scene_layer_visibility_button_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 64.0,
        rect.y + 184.0 + index as f32 * 31.0,
        28.0,
        26.0,
    )
}

pub(crate) fn scene_layer_lock_button_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 32.0,
        rect.y + 184.0 + index as f32 * 31.0,
        32.0,
        26.0,
    )
}

pub(crate) fn scene_layer_opacity_down_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 70.0, rect.y + 312.0, 32.0, 24.0)
}

pub(crate) fn scene_layer_opacity_up_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 34.0, rect.y + 312.0, 34.0, 24.0)
}

pub(crate) fn scene_terrain_paint_mode_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 5.0;
    let width = (rect.w - gap * 2.0) / 3.0;
    Rect::new(
        rect.x + index as f32 * (width + gap),
        rect.y + 364.0,
        width,
        26.0,
    )
}

pub(crate) fn scene_content_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 5.0;
    let width = (rect.w - gap * 2.0) / 3.0;
    let column = index % 3;
    let row = index / 3;
    Rect::new(
        rect.x + column as f32 * (width + gap),
        rect.y + 426.0 + row as f32 * 29.0,
        width,
        25.0,
    )
}

pub(crate) fn scene_content_capacity(rect: Rect) -> usize {
    let content_top = rect.y + 426.0;
    let action_top = scene_apply_button_rect(rect).y - 38.0;
    let available = (action_top - content_top).max(29.0);
    let rows = (available / 29.0).floor().max(1.0) as usize;
    rows * 3
}

pub(crate) fn scene_content_page_start(rect: Rect, count: usize, selected: usize) -> usize {
    let capacity = scene_content_capacity(rect).max(1);
    let max_start = count.saturating_sub(1) / capacity * capacity;
    ((selected / capacity) * capacity).min(max_start)
}

pub(crate) fn scene_apply_button_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 102.0, (rect.w - 6.0) * 0.5, 32.0)
}

pub(crate) fn scene_erase_button_rect(rect: Rect) -> Rect {
    let apply = scene_apply_button_rect(rect);
    Rect::new(apply.x + apply.w + 6.0, apply.y, apply.w, apply.h)
}

pub(crate) fn canvas_toolbar_button_rect(host: Rect, index: usize, width: f32) -> Rect {
    Rect::new(
        host.x + 8.0 + index as f32 * 88.0,
        host.y + 4.0,
        width,
        28.0,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanvasToolbarKind {
    SceneMap,
    WorldScenes,
    SceneBank,
}

pub(crate) fn draw_canvas_toolbar(
    host: Rect,
    mode: CanvasToolbarKind,
    zoom_percent: f32,
    pan_active: bool,
    world_options: Option<WorldSurfaceViewOptions>,
) {
    draw_rectangle(
        host.x,
        host.y,
        host.w,
        36.0,
        Color::new(0.10, 0.09, 0.08, 1.0),
    );
    draw_rectangle_lines(host.x, host.y, host.w, 36.0, 1.0, PANEL_EDGE);
    match mode {
        CanvasToolbarKind::SceneMap => {
            draw_editor_widget(canvas_toolbar_button_rect(host, 0, 82.0), "Frame", false);
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 1, 82.0),
                "Frame Sel",
                false,
            );
            draw_editor_widget(canvas_toolbar_button_rect(host, 2, 42.0), "-", false);
            draw_editor_widget(canvas_toolbar_button_rect(host, 3, 42.0), "+", false);
            draw_editor_text(
                &format!("Zoom {:.0}%", zoom_percent),
                host.x + 366.0,
                host.y + 23.0,
                16.0,
                TEXT,
            );
            draw_scissored_text(
                "Infinite canvas | wheel zoom | middle-drag or Space+drag pan | F frame",
                host.x + 478.0,
                host.y + 23.0,
                (host.w - 488.0).max(1.0),
                14.0,
                MUTED,
            );
        }
        CanvasToolbarKind::WorldScenes => {
            let options = world_options.unwrap_or_default();
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 0, 70.0),
                "Select",
                options.edit_tool == WorldEditTool::Select,
            );
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 1, 70.0),
                "Pan",
                options.edit_tool == WorldEditTool::Pan,
            );
            draw_editor_widget(canvas_toolbar_button_rect(host, 2, 82.0), "Frame", false);
            draw_editor_widget(canvas_toolbar_button_rect(host, 3, 42.0), "-", false);
            draw_editor_widget(canvas_toolbar_button_rect(host, 4, 42.0), "+", false);
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 5, 82.0),
                "Partitions",
                options.show_partitions,
            );
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 6, 76.0),
                "Objects",
                options.show_objects,
            );
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 7, 70.0),
                "Zones",
                options.show_zones,
            );
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 8, 86.0),
                "Levels & Cliffs",
                options.show_structural_levels,
            );
            draw_editor_text(
                &format!("Zoom {:.0}%", zoom_percent),
                host.x + 805.0,
                host.y + 23.0,
                16.0,
                TEXT,
            );
            draw_scissored_text(
                &format!(
                    "{} / {} | 1-9 tools | F1-F4 layers | Enter opens selected partition",
                    options.layer_mode.label(),
                    options.edit_tool.label()
                ),
                host.x + 905.0,
                host.y + 23.0,
                (host.w - 915.0).max(1.0),
                14.0,
                MUTED,
            );
        }
        CanvasToolbarKind::SceneBank => {
            draw_editor_widget(
                canvas_toolbar_button_rect(host, 0, 70.0),
                "Select",
                !pan_active,
            );
            draw_editor_widget(canvas_toolbar_button_rect(host, 1, 70.0), "Pan", pan_active);
            draw_editor_widget(canvas_toolbar_button_rect(host, 2, 82.0), "Frame", false);
            draw_editor_widget(canvas_toolbar_button_rect(host, 3, 42.0), "-", false);
            draw_editor_widget(canvas_toolbar_button_rect(host, 4, 42.0), "+", false);
            draw_editor_text(
                &format!("Zoom {:.0}%", zoom_percent),
                host.x + 455.0,
                host.y + 23.0,
                16.0,
                TEXT,
            );
            draw_scissored_text(
                "Scene documents | Enter opens selected scene | wheel zoom | middle-drag or Space+drag pan",
                host.x + 560.0,
                host.y + 23.0,
                (host.w - 570.0).max(1.0),
                14.0,
                MUTED,
            );
        }
    }
}

pub(crate) fn draw_infinite_grid(visible: Rect, minor_step: f32, major_every: i32, zoom: f32) {
    if minor_step <= 0.0 || major_every <= 0 {
        return;
    }
    let minor_color = Color::new(0.23, 0.30, 0.33, 0.48);
    let major_color = Color::new(0.34, 0.43, 0.46, 0.78);
    let axis_color = Color::new(0.62, 0.50, 0.34, 0.82);
    let thin = ((0.055_f32).max(minor_step * 0.010) / zoom.max(0.20)).min(minor_step * 0.08);
    let thick = ((0.11_f32).max(minor_step * 0.020) / zoom.max(0.20)).min(minor_step * 0.12);
    let mut x = (visible.x / minor_step).floor() * minor_step;
    while x <= visible.x + visible.w {
        let grid_index = (x / minor_step).round() as i32;
        let major = grid_index.rem_euclid(major_every) == 0;
        let color = if grid_index == 0 {
            axis_color
        } else if major {
            major_color
        } else {
            minor_color
        };
        draw_line(
            x,
            visible.y,
            x,
            visible.y + visible.h,
            if major { thick } else { thin },
            color,
        );
        x += minor_step;
    }
    let mut y = (visible.y / minor_step).floor() * minor_step;
    while y <= visible.y + visible.h {
        let grid_index = (y / minor_step).round() as i32;
        let major = grid_index.rem_euclid(major_every) == 0;
        let color = if grid_index == 0 {
            axis_color
        } else if major {
            major_color
        } else {
            minor_color
        };
        draw_line(
            visible.x,
            y,
            visible.x + visible.w,
            y,
            if major { thick } else { thin },
            color,
        );
        y += minor_step;
    }
}

fn preview_grid_dimensions(rect: Rect) -> (i32, i32) {
    let columns = ((rect.w / 4.0).round() as i32)
        .clamp(8, 32)
        .min(MAP_W as i32);
    let rows = ((rect.h / 4.0).round() as i32)
        .clamp(8, 24)
        .min(MAP_H as i32);
    (columns, rows)
}

pub(crate) fn draw_scene_into_rect(scene: &SceneMap, rect: Rect) {
    // Scene cards previously issued MAP_W * MAP_H draw calls for every visible
    // card. With expanded 96x64 scenes this could exceed one hundred thousand
    // rectangles per frame and make Windows dim the editor as unresponsive.
    // Downsample the preview and merge horizontal runs while the real scene
    // editor continues to draw the complete map at native tile resolution.
    let (columns, rows) = preview_grid_dimensions(rect);
    let cell_w = rect.w / columns as f32;
    let cell_h = rect.h / rows as f32;

    for preview_y in 0..rows {
        let source_y = (((preview_y as f32 + 0.5) * MAP_H as f32 / rows as f32).floor() as i32)
            .clamp(0, MAP_H as i32 - 1);
        let mut run_start = 0;
        let first_source_x =
            ((0.5 * MAP_W as f32 / columns as f32).floor() as i32).clamp(0, MAP_W as i32 - 1);
        let mut run_tile = scene.map.get(first_source_x, source_y);

        for preview_x in 1..=columns {
            let next_tile = if preview_x < columns {
                let source_x = (((preview_x as f32 + 0.5) * MAP_W as f32 / columns as f32).floor()
                    as i32)
                    .clamp(0, MAP_W as i32 - 1);
                Some(scene.map.get(source_x, source_y))
            } else {
                None
            };

            if next_tile == Some(run_tile) {
                continue;
            }

            draw_rectangle(
                rect.x + run_start as f32 * cell_w,
                rect.y + preview_y as f32 * cell_h,
                (preview_x - run_start) as f32 * cell_w + 0.2,
                cell_h + 0.2,
                scene_tile_color(run_tile),
            );
            if let Some(tile) = next_tile {
                run_start = preview_x;
                run_tile = tile;
            }
        }
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn scene_preview_grid_is_bounded_for_large_scene_cards() {
        let (columns, rows) = preview_grid_dimensions(Rect::new(0.0, 0.0, 96.0, 96.0));
        assert!(columns <= 32);
        assert!(rows <= 24);
        assert!(columns * rows < MAP_W as i32 * MAP_H as i32);
    }
}
