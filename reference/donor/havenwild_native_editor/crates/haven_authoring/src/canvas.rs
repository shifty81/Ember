use serde::{Deserialize, Serialize};

use crate::{GridPos, GridRect};

pub const AUTHORING_CANVAS_MIN_ZOOM: f32 = 0.01;
pub const AUTHORING_CANVAS_MAX_ZOOM: f32 = 32.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanvasPoint {
    pub x: f32,
    pub y: f32,
}

impl CanvasPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanvasRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl CanvasRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(self, point: CanvasPoint) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }
}

/// Headless screen/world transform shared by Havenwild authoring frontends.
///
/// The native editor remains responsible for input smoothing and renderer camera
/// setup, but all hit-testing and visible-region math can use this exact mapping.
/// Positive world Y maps toward the bottom of the viewport, matching Havenwild's
/// runtime/tile-grid convention.

pub fn visible_grid_bounds_in_world(
    visible: CanvasRect,
    origin: CanvasPoint,
    cell_size: f32,
    width: i32,
    height: i32,
) -> Option<GridRect> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let cell_size = cell_size.max(f32::EPSILON);
    let min_x = ((visible.x - origin.x) / cell_size).floor() as i32;
    let min_y = ((visible.y - origin.y) / cell_size).floor() as i32;
    let max_x = ((visible.x + visible.w - origin.x) / cell_size).ceil() as i32 - 1;
    let max_y = ((visible.y + visible.h - origin.y) / cell_size).ceil() as i32 - 1;
    if max_x < 0 || max_y < 0 || min_x >= width || min_y >= height {
        return None;
    }
    let min = GridPos {
        x: min_x.clamp(0, width - 1),
        y: min_y.clamp(0, height - 1),
    };
    let max = GridPos {
        x: max_x.clamp(0, width - 1),
        y: max_y.clamp(0, height - 1),
    };
    (min.x <= max.x && min.y <= max.y).then_some(GridRect { min, max })
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthoringCanvasTransform {
    pub viewport: CanvasRect,
    pub content_bounds: CanvasRect,
    pub zoom: f32,
    pub pan: CanvasPoint,
}

impl AuthoringCanvasTransform {
    pub fn new(
        viewport: CanvasRect,
        content_bounds: CanvasRect,
        zoom: f32,
        pan: CanvasPoint,
    ) -> Self {
        Self {
            viewport,
            content_bounds,
            zoom: zoom.clamp(AUTHORING_CANVAS_MIN_ZOOM, AUTHORING_CANVAS_MAX_ZOOM),
            pan,
        }
    }

    pub fn visible_world_rect(self) -> CanvasRect {
        let viewport_aspect = (self.viewport.w / self.viewport.h.max(1.0)).max(0.01);
        let bounds_aspect = (self.content_bounds.w / self.content_bounds.h.max(1.0)).max(0.01);
        let margin = 1.12;
        let (fit_w, fit_h) = if viewport_aspect >= bounds_aspect {
            let height = self.content_bounds.h * margin;
            (height * viewport_aspect, height)
        } else {
            let width = self.content_bounds.w * margin;
            (width, width / viewport_aspect)
        };
        let visible_w = fit_w / self.zoom.max(AUTHORING_CANVAS_MIN_ZOOM);
        let visible_h = fit_h / self.zoom.max(AUTHORING_CANVAS_MIN_ZOOM);
        let center = CanvasPoint::new(
            self.content_bounds.x + self.content_bounds.w * 0.5 + self.pan.x,
            self.content_bounds.y + self.content_bounds.h * 0.5 + self.pan.y,
        );
        CanvasRect::new(
            center.x - visible_w * 0.5,
            center.y - visible_h * 0.5,
            visible_w,
            visible_h,
        )
    }

    pub fn screen_to_world(self, point: CanvasPoint) -> CanvasPoint {
        let visible = self.visible_world_rect();
        let nx = (point.x - self.viewport.x) / self.viewport.w.max(1.0);
        let ny = (point.y - self.viewport.y) / self.viewport.h.max(1.0);
        CanvasPoint::new(
            visible.x + nx * visible.w,
            visible.y + ny * visible.h,
        )
    }

    pub fn world_to_screen(self, point: CanvasPoint) -> CanvasPoint {
        let visible = self.visible_world_rect();
        let nx = (point.x - visible.x) / visible.w.max(f32::EPSILON);
        let ny = (point.y - visible.y) / visible.h.max(f32::EPSILON);
        CanvasPoint::new(
            self.viewport.x + nx * self.viewport.w,
            self.viewport.y + ny * self.viewport.h,
        )
    }

    pub fn screen_to_grid_cell(self, point: CanvasPoint, cell_size: f32) -> GridPos {
        let world = self.screen_to_world(point);
        let cell_size = cell_size.max(f32::EPSILON);
        GridPos {
            x: (world.x / cell_size).floor() as i32,
            y: (world.y / cell_size).floor() as i32,
        }
    }

    pub fn visible_grid_bounds(
        self,
        origin: CanvasPoint,
        cell_size: f32,
        width: i32,
        height: i32,
    ) -> Option<GridRect> {
        visible_grid_bounds_in_world(
            self.visible_world_rect(),
            origin,
            cell_size,
            width,
            height,
        )
    }

    pub fn pixels_per_world_unit(self) -> f32 {
        let visible = self.visible_world_rect();
        let x = self.viewport.w / visible.w.max(f32::EPSILON);
        let y = self.viewport.h / visible.h.max(f32::EPSILON);
        x.min(y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform() -> AuthoringCanvasTransform {
        AuthoringCanvasTransform::new(
            CanvasRect::new(100.0, 50.0, 800.0, 600.0),
            CanvasRect::new(0.0, 0.0, 96.0, 64.0),
            1.0,
            CanvasPoint::default(),
        )
    }

    #[test]
    fn positive_y_screen_mapping_matches_tile_world_convention() {
        let transform = transform();
        let top = transform.screen_to_world(CanvasPoint::new(500.0, 50.0));
        let bottom = transform.screen_to_world(CanvasPoint::new(500.0, 650.0));
        assert!(bottom.y > top.y);
    }

    #[test]
    fn screen_world_round_trip_is_stable() {
        let transform = transform();
        let point = CanvasPoint::new(417.0, 311.0);
        let round_trip = transform.world_to_screen(transform.screen_to_world(point));
        assert!((round_trip.x - point.x).abs() < 0.001);
        assert!((round_trip.y - point.y).abs() < 0.001);
    }

    #[test]
    fn visible_grid_bounds_are_clamped_to_document() {
        let transform = transform();
        let visible = transform
            .visible_grid_bounds(CanvasPoint::default(), 1.0, 96, 64)
            .expect("visible cells");
        assert!(visible.min.x >= 0 && visible.min.y >= 0);
        assert!(visible.max.x < 96 && visible.max.y < 64);
    }

    #[test]
    fn zoom_changes_visible_world_not_screen_mapping_orientation() {
        let near = AuthoringCanvasTransform::new(
            CanvasRect::new(0.0, 0.0, 800.0, 600.0),
            CanvasRect::new(0.0, 0.0, 96.0, 64.0),
            2.0,
            CanvasPoint::default(),
        );
        let far = transform();
        assert!(near.visible_world_rect().w < far.visible_world_rect().w);
        assert!(near.visible_world_rect().h < far.visible_world_rect().h);
    }
}
