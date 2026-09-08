use haven_editor::{
    AuthoringCanvasTransform, CanvasPoint as AuthoringCanvasPoint,
    CanvasRect as AuthoringCanvasRect, GridPos, GridRect, AUTHORING_CANVAS_MAX_ZOOM, AUTHORING_CANVAS_MIN_ZOOM,
};
use macroquad::prelude::*;

pub const CANVAS_MIN_ZOOM: f32 = AUTHORING_CANVAS_MIN_ZOOM;
pub const CANVAS_MAX_ZOOM: f32 = AUTHORING_CANVAS_MAX_ZOOM;
const CANVAS_ZOOM_STEP: f32 = 1.18;
const CANVAS_ZOOM_RESPONSE: f32 = 18.0;
const WHEEL_DEAD_ZONE: f32 = 0.05;

#[derive(Clone, Copy, Debug)]
pub struct CanvasCameraState {
    pub zoom: f32,
    zoom_target: f32,
    pub pan: Vec2,
    dragging: bool,
    drag_last_screen: Vec2,
}

impl Default for CanvasCameraState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            zoom_target: 1.0,
            pan: Vec2::ZERO,
            dragging: false,
            drag_last_screen: Vec2::ZERO,
        }
    }
}

impl CanvasCameraState {
    pub fn reset(&mut self) {
        self.set_zoom_immediate(1.0);
        self.pan = Vec2::ZERO;
        self.dragging = false;
    }

    pub fn zoom_percent(&self) -> f32 {
        self.zoom * 100.0
    }

    pub fn zoom_out_step(&mut self) {
        self.set_zoom_target(self.zoom_target / 1.50);
    }

    pub fn zoom_in_step(&mut self) {
        self.set_zoom_target(self.zoom_target * 1.50);
    }

    pub fn actual_size(&mut self) {
        self.set_zoom_immediate(1.0);
    }

    pub fn frame_rect(&mut self, viewport: Rect, content_bounds: Rect, target: Rect) {
        let viewport_aspect = (viewport.w / viewport.h.max(1.0)).max(0.01);
        let bounds_aspect = (content_bounds.w / content_bounds.h.max(1.0)).max(0.01);
        let margin = 1.12;
        let (fit_w, fit_h) = if viewport_aspect >= bounds_aspect {
            let height = content_bounds.h * margin;
            (height * viewport_aspect, height)
        } else {
            let width = content_bounds.w * margin;
            (width, width / viewport_aspect)
        };
        let target_w = target.w.max(1.0) * 1.35;
        let target_h = target.h.max(1.0) * 1.35;
        let framed_zoom = (fit_w / target_w)
            .min(fit_h / target_h)
            .clamp(CANVAS_MIN_ZOOM, CANVAS_MAX_ZOOM);
        self.set_zoom_immediate(framed_zoom);
        let content_center = vec2(
            content_bounds.x + content_bounds.w * 0.5,
            content_bounds.y + content_bounds.h * 0.5,
        );
        let target_center = vec2(target.x + target.w * 0.5, target.y + target.h * 0.5);
        self.pan = target_center - content_center;
        self.dragging = false;
    }

    pub fn visible_world_rect(&self, viewport: Rect, content_bounds: Rect) -> Rect {
        let visible = self.authoring_transform(viewport, content_bounds).visible_world_rect();
        Rect::new(visible.x, visible.y, visible.w, visible.h)
    }

    pub fn camera(&self, viewport: Rect, content_bounds: Rect) -> Camera2D {
        let display_rect = self.visible_world_rect(viewport, content_bounds);
        let mut camera = Camera2D {
            target: vec2(
                display_rect.x + display_rect.w * 0.5,
                display_rect.y + display_rect.h * 0.5,
            ),
            // Havenwild scene data and the runtime both use the same positive-Y
            // camera convention. Do not use the display-rectangle helper here:
            // its negative Y zoom mirrors the native editor against the game.
            zoom: editor_camera_zoom(display_rect),
            ..Default::default()
        };
        camera.viewport = Some((
            viewport.x.round() as i32,
            (screen_height() - viewport.y - viewport.h).round() as i32,
            viewport.w.max(1.0).round() as i32,
            viewport.h.max(1.0).round() as i32,
        ));
        camera
    }

    pub fn screen_to_world(
        &self,
        viewport: Rect,
        content_bounds: Rect,
        screen_point: Vec2,
    ) -> Vec2 {
        let point = self
            .authoring_transform(viewport, content_bounds)
            .screen_to_world(AuthoringCanvasPoint::new(screen_point.x, screen_point.y));
        vec2(point.x, point.y)
    }

    #[allow(dead_code)]
    pub fn world_to_screen(
        &self,
        viewport: Rect,
        content_bounds: Rect,
        world_point: Vec2,
    ) -> Vec2 {
        let point = self
            .authoring_transform(viewport, content_bounds)
            .world_to_screen(AuthoringCanvasPoint::new(world_point.x, world_point.y));
        vec2(point.x, point.y)
    }

    pub fn screen_to_grid_cell(
        &self,
        viewport: Rect,
        content_bounds: Rect,
        screen_point: Vec2,
        cell_size: f32,
    ) -> GridPos {
        self.authoring_transform(viewport, content_bounds).screen_to_grid_cell(
            AuthoringCanvasPoint::new(screen_point.x, screen_point.y),
            cell_size,
        )
    }

    pub fn visible_grid_bounds(
        &self,
        viewport: Rect,
        content_bounds: Rect,
        grid_origin: Vec2,
        cell_size: f32,
        width: i32,
        height: i32,
    ) -> Option<GridRect> {
        self.authoring_transform(viewport, content_bounds).visible_grid_bounds(
            AuthoringCanvasPoint::new(grid_origin.x, grid_origin.y),
            cell_size,
            width,
            height,
        )
    }

    pub fn handle_navigation(
        &mut self,
        viewport: Rect,
        content_bounds: Rect,
        primary_button_pans: bool,
    ) -> bool {
        let (mouse_x, mouse_y) = mouse_position();
        let mouse = vec2(mouse_x, mouse_y);
        let pointer_inside = viewport.contains(mouse);
        let raw_wheel = if pointer_inside { mouse_wheel().1 } else { 0.0 };
        let wheel = normalize_wheel_delta(raw_wheel);
        let primary_pan = primary_button_pans && is_mouse_button_down(MouseButton::Left);
        let modified_primary_pan =
            is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left);
        let middle_pan = is_mouse_button_down(MouseButton::Middle);
        let wants_pan = primary_pan || modified_primary_pan || middle_pan;

        if wants_pan && pointer_inside && !self.dragging {
            self.dragging = true;
            self.drag_last_screen = mouse;
        }

        if self.dragging {
            if wants_pan {
                let previous_world =
                    self.screen_to_world(viewport, content_bounds, self.drag_last_screen);
                let current_world = self.screen_to_world(viewport, content_bounds, mouse);
                self.pan += previous_world - current_world;
                self.drag_last_screen = mouse;
            } else {
                self.dragging = false;
            }
        }

        if wheel.abs() > f32::EPSILON {
            self.set_zoom_target(self.zoom_target * CANVAS_ZOOM_STEP.powf(wheel));
        }

        if (self.zoom_target - self.zoom).abs() > 0.0001 {
            let anchor = if pointer_inside {
                mouse
            } else {
                vec2(viewport.x + viewport.w * 0.5, viewport.y + viewport.h * 0.5)
            };
            let before = self.screen_to_world(viewport, content_bounds, anchor);
            let t = 1.0 - (-CANVAS_ZOOM_RESPONSE * get_frame_time().clamp(0.0, 0.1)).exp();
            self.zoom += (self.zoom_target - self.zoom) * t;
            if (self.zoom_target - self.zoom).abs() <= 0.0001 {
                self.zoom = self.zoom_target;
            }
            let after = self.screen_to_world(viewport, content_bounds, anchor);
            self.pan += before - after;
        }

        self.dragging || (wants_pan && pointer_inside) || wheel.abs() > f32::EPSILON
    }

    fn set_zoom_target(&mut self, zoom: f32) {
        self.zoom_target = zoom.clamp(CANVAS_MIN_ZOOM, CANVAS_MAX_ZOOM);
    }

    fn set_zoom_immediate(&mut self, zoom: f32) {
        let zoom = zoom.clamp(CANVAS_MIN_ZOOM, CANVAS_MAX_ZOOM);
        self.zoom = zoom;
        self.zoom_target = zoom;
    }

    fn authoring_transform(
        &self,
        viewport: Rect,
        content_bounds: Rect,
    ) -> AuthoringCanvasTransform {
        AuthoringCanvasTransform::new(
            AuthoringCanvasRect::new(viewport.x, viewport.y, viewport.w, viewport.h),
            AuthoringCanvasRect::new(
                content_bounds.x,
                content_bounds.y,
                content_bounds.w,
                content_bounds.h,
            ),
            self.zoom,
            AuthoringCanvasPoint::new(self.pan.x, self.pan.y),
        )
    }
}

fn editor_camera_zoom(display_rect: Rect) -> Vec2 {
    vec2(2.0 / display_rect.w.max(1.0), 2.0 / display_rect.h.max(1.0))
}

fn normalize_wheel_delta(raw: f32) -> f32 {
    if !raw.is_finite() || raw.abs() < WHEEL_DEAD_ZONE {
        0.0
    } else {
        raw.clamp(-1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{editor_camera_zoom, normalize_wheel_delta};
    use macroquad::prelude::Rect;

    #[test]
    fn wheel_normalization_preserves_trackpad_fraction() {
        assert!((normalize_wheel_delta(0.25) - 0.25).abs() < f32::EPSILON);
        assert!((normalize_wheel_delta(-0.4) + 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn wheel_normalization_clamps_notches_and_ignores_noise() {
        assert_eq!(normalize_wheel_delta(120.0), 1.0);
        assert_eq!(normalize_wheel_delta(-120.0), -1.0);
        assert_eq!(normalize_wheel_delta(0.02), 0.0);
    }

    #[test]
    fn editor_camera_matches_runtime_positive_y_orientation() {
        let zoom = editor_camera_zoom(Rect::new(0.0, 0.0, 96.0, 64.0));
        assert!(zoom.x > 0.0);
        assert!(zoom.y > 0.0);
    }
}
