use macroquad::prelude::*;

/// Authoritative transform between a sprite document's pixel space and the native editor canvas.
/// Rendering, hit testing, rulers, selections, anchors, and overlays should use this same transform.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpriteCanvasTransform {
    pub canvas: Rect,
    pub image: Rect,
    pub zoom: f32,
    pub image_width: u32,
    pub image_height: u32,
}

impl SpriteCanvasTransform {
    pub(crate) fn new(
        canvas: Rect,
        image_width: u32,
        image_height: u32,
        zoom: f32,
        pan: Vec2,
    ) -> Self {
        let zoom = zoom.max(0.0001);
        let width = image_width as f32 * zoom;
        let height = image_height as f32 * zoom;
        let image = Rect::new(
            canvas.x + (canvas.w - width) * 0.5 + pan.x,
            canvas.y + (canvas.h - height) * 0.5 + pan.y,
            width,
            height,
        );
        Self {
            canvas,
            image,
            zoom,
            image_width,
            image_height,
        }
    }

    pub(crate) fn screen_to_pixel(self, point: Vec2) -> Option<(u32, u32)> {
        if !self.image.contains(point) {
            return None;
        }
        let x = ((point.x - self.image.x) / self.zoom).floor() as i32;
        let y = ((point.y - self.image.y) / self.zoom).floor() as i32;
        (x >= 0 && y >= 0 && x < self.image_width as i32 && y < self.image_height as i32)
            .then_some((x as u32, y as u32))
    }

    pub(crate) fn pixel_to_screen(self, pixel: Vec2) -> Vec2 {
        vec2(
            self.image.x + pixel.x * self.zoom,
            self.image.y + pixel.y * self.zoom,
        )
    }

    pub(crate) fn pixel_rect_to_screen(self, rect: Rect) -> Rect {
        let origin = self.pixel_to_screen(vec2(rect.x, rect.y));
        Rect::new(origin.x, origin.y, rect.w * self.zoom, rect.h * self.zoom)
    }

    pub(crate) fn visible_pixel_bounds(self) -> Rect {
        let left = ((self.canvas.x - self.image.x) / self.zoom)
            .floor()
            .max(0.0);
        let top = ((self.canvas.y - self.image.y) / self.zoom)
            .floor()
            .max(0.0);
        let right = ((self.canvas.x + self.canvas.w - self.image.x) / self.zoom)
            .ceil()
            .min(self.image_width as f32);
        let bottom = ((self.canvas.y + self.canvas.h - self.image.y) / self.zoom)
            .ceil()
            .min(self.image_height as f32);
        Rect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
    }

    pub(crate) fn clipped_image(self) -> Option<Rect> {
        intersect_rect(self.image, self.canvas)
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SpriteOverlayKind {
    PixelGrid,
    FrameGrid,
    AtlasEntry,
    Selection,
    Pivot,
    GroundAnchor,
    Socket,
    Collision,
    Interaction,
    WorldGrid,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SpriteOverlayStyle {
    pub color: Color,
    pub line_width: f32,
    pub minimum_zoom: f32,
}

impl SpriteOverlayKind {
    pub(crate) fn style(self) -> SpriteOverlayStyle {
        match self {
            Self::PixelGrid => SpriteOverlayStyle {
                color: Color::new(0.15, 0.18, 0.22, 0.65),
                line_width: 1.0,
                minimum_zoom: 6.0,
            },
            Self::FrameGrid => SpriteOverlayStyle {
                color: Color::new(0.95, 0.72, 0.25, 0.78),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::AtlasEntry => SpriteOverlayStyle {
                color: Color::new(0.95, 0.40, 0.25, 0.85),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::Selection => SpriteOverlayStyle {
                color: Color::new(0.38, 0.86, 0.50, 1.0),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::Pivot => SpriteOverlayStyle {
                color: Color::new(1.0, 0.70, 0.22, 1.0),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::GroundAnchor => SpriteOverlayStyle {
                color: Color::new(0.35, 0.75, 1.0, 1.0),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::Socket => SpriteOverlayStyle {
                color: Color::new(0.96, 0.50, 0.20, 1.0),
                line_width: 1.0,
                minimum_zoom: 0.0,
            },
            Self::Collision => SpriteOverlayStyle {
                color: Color::new(0.92, 0.24, 0.26, 0.90),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::Interaction => SpriteOverlayStyle {
                color: Color::new(0.72, 0.40, 0.92, 0.90),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
            Self::WorldGrid => SpriteOverlayStyle {
                color: Color::new(0.30, 0.68, 0.88, 0.70),
                line_width: 2.0,
                minimum_zoom: 0.0,
            },
        }
    }
}

fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.w).min(right.x + right.w);
    let bottom_edge = (left.y + left.h).min(right.y + right.h);
    (right_edge > x && bottom_edge > y).then_some(Rect::new(x, y, right_edge - x, bottom_edge - y))
}
