use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct CanvasCamera {
    pub center: Vec2,
    pub zoom: f32,
    pub viewport: Vec2,
}
impl CanvasCamera {
    pub fn new(viewport: Vec2) -> Self {
        Self {
            center: Vec2 { x: 0.0, y: 0.0 },
            zoom: 1.0,
            viewport,
        }
    }
    pub fn pan(&mut self, delta_screen: Vec2) {
        self.center.x -= delta_screen.x / self.zoom;
        self.center.y -= delta_screen.y / self.zoom;
    }
    pub fn zoom_around(&mut self, screen: Vec2, factor: f32) {
        let before = self.screen_to_world(screen);
        self.zoom = (self.zoom * factor).clamp(0.05, 64.0);
        let after = self.screen_to_world(screen);
        self.center.x += before.x - after.x;
        self.center.y += before.y - after.y;
    }
    pub fn world_to_screen(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: (p.x - self.center.x) * self.zoom + self.viewport.x * 0.5,
            y: (p.y - self.center.y) * self.zoom + self.viewport.y * 0.5,
        }
    }
    pub fn screen_to_world(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: (p.x - self.viewport.x * 0.5) / self.zoom + self.center.x,
            y: (p.y - self.viewport.y * 0.5) / self.zoom + self.center.y,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        let c = CanvasCamera::new(Vec2 { x: 800.0, y: 600.0 });
        let p = Vec2 { x: 12.0, y: -3.0 };
        let q = c.screen_to_world(c.world_to_screen(p));
        assert!((p.x - q.x).abs() < 0.001);
        assert!((p.y - q.y).abs() < 0.001);
    }
}
