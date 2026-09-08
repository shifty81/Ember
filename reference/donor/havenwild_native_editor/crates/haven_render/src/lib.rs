pub mod structural_cliff_visual;
pub use structural_cliff_visual::*;

use haven_assets::asset_registry::AtlasRect;
use haven_authoring::InspectorReport;
use haven_core::{PlacedObject, PlacedStamp, SceneMap, SceneReference, UiAnchor, UiPanelId, TILE_SIZE};
use macroquad::prelude::{
    draw_circle, draw_rectangle, draw_rectangle_lines, draw_text, load_texture, mouse_position,
    vec2, Color, FilterMode, Rect, Texture2D, Vec2,
};

pub const ARCHITECTURE_STATUS: &str =
    "Active render crate for shared runtime/editor render helpers; larger 2.5D rendering ownership will continue moving here.";

pub use haven_core::{MAP_H, MAP_W, TILE_SIZE as WORLD_TILE_SIZE};

pub fn atlas_rect(rect: AtlasRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.w, rect.h)
}

/// Canonical bottom-center world foot/root for placeable objects.
///
/// The anchor is derived from collision geometry first and interaction geometry
/// second so runtime rendering, native-editor rendering, selection and depth
/// sorting agree on the same physical root. Tall LPC art must never be aligned
/// from its image top-left or the object's anchor-cell top-left.
pub fn object_foot_world(object: PlacedObject) -> Vec2 {
    let (collision_x, collision_y, collision_w, collision_h) = object.collision_rect();
    if collision_w > 0 && collision_h > 0 {
        return vec2(
            (collision_x as f32 + collision_w as f32 * 0.5) * TILE_SIZE,
            (collision_y + collision_h) as f32 * TILE_SIZE,
        );
    }

    let (interaction_x, interaction_y, interaction_w, interaction_h) = object.interaction_rect();
    vec2(
        (interaction_x as f32 + interaction_w.max(1) as f32 * 0.5) * TILE_SIZE,
        (interaction_y + interaction_h.max(1)) as f32 * TILE_SIZE,
    )
}

/// Same canonical foot/root in tile coordinates for editor-space rendering.
pub fn object_foot_tiles(object: PlacedObject) -> Vec2 {
    object_foot_world(object) / TILE_SIZE
}

/// Canonical bottom-center world foot/root for multi-cell authored stamps.
/// This mirrors object anchoring so editor overlays and depth diagnostics use
/// the same collision-first semantic root for every placeable family.
pub fn stamp_foot_world(stamp: &PlacedStamp) -> Vec2 {
    let (collision_x, collision_y, collision_w, collision_h) = stamp.collision_rect();
    if collision_w > 0 && collision_h > 0 {
        return vec2(
            (collision_x as f32 + collision_w as f32 * 0.5) * TILE_SIZE,
            (collision_y + collision_h) as f32 * TILE_SIZE,
        );
    }

    let (interaction_x, interaction_y, interaction_w, interaction_h) = stamp.interaction_rect();
    vec2(
        (interaction_x as f32 + interaction_w.max(1) as f32 * 0.5) * TILE_SIZE,
        (interaction_y + interaction_h.max(1)) as f32 * TILE_SIZE,
    )
}

pub fn stamp_foot_tiles(stamp: &PlacedStamp) -> Vec2 {
    stamp_foot_world(stamp) / TILE_SIZE
}

pub fn object_origin(object: PlacedObject) -> Vec2 {
    let (x, y, _, _) = object.visual_rect();
    vec2(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE)
}

pub fn object_sort_y(object: PlacedObject) -> f32 {
    object.sort_y()
}

pub fn player_sort_y(player: Vec2) -> f32 {
    player.y + 20.0
}

pub fn sky_color(night_amount: f32) -> Color {
    let night = night_amount;
    Color::new(
        0.18 * (1.0 - night) + 0.012 * night,
        0.28 * (1.0 - night) + 0.018 * night,
        0.28 * (1.0 - night) + 0.050 * night,
        1.0,
    )
}

pub fn draw_object_fallback(object: PlacedObject, screen_origin: Vec2) {
    let x = screen_origin.x;
    let y = screen_origin.y;
    match object.kind {
        haven_core::ObjectKind::Table => draw_rectangle(
            x + 4.0,
            y + 7.0,
            24.0,
            18.0,
            Color::from_rgba(113, 69, 37, 255),
        ),
        haven_core::ObjectKind::Chair => draw_rectangle(
            x + 9.0,
            y + 8.0,
            14.0,
            17.0,
            Color::from_rgba(91, 55, 31, 255),
        ),
        haven_core::ObjectKind::Bar => {
            draw_rectangle(x, y + 8.0, 32.0, 18.0, Color::from_rgba(105, 62, 34, 255))
        }
        haven_core::ObjectKind::Keg => {
            draw_circle(x + 16.0, y + 16.0, 12.0, Color::from_rgba(126, 74, 39, 255))
        }
        haven_core::ObjectKind::Bed => draw_rectangle(
            x + 3.0,
            y + 7.0,
            26.0,
            19.0,
            Color::from_rgba(99, 127, 166, 255),
        ),
        haven_core::ObjectKind::Fireplace => draw_rectangle(
            x + 5.0,
            y + 4.0,
            22.0,
            24.0,
            Color::from_rgba(91, 76, 63, 255),
        ),
        haven_core::ObjectKind::GreenhouseMarker => draw_rectangle_lines(
            x + 5.0,
            y + 5.0,
            22.0,
            22.0,
            3.0,
            Color::from_rgba(155, 242, 139, 255),
        ),
        haven_core::ObjectKind::Tree => {
            draw_rectangle(
                x + 13.0,
                y + 14.0,
                7.0,
                18.0,
                Color::from_rgba(96, 61, 34, 255),
            );
            draw_circle(x + 16.0, y + 12.0, 15.0, Color::from_rgba(54, 122, 66, 255));
            draw_circle(x + 8.0, y + 18.0, 10.0, Color::from_rgba(44, 101, 55, 255));
        }
        haven_core::ObjectKind::OreNode => {
            draw_circle(x + 16.0, y + 18.0, 12.0, Color::from_rgba(82, 86, 92, 255));
            draw_circle(x + 13.0, y + 15.0, 3.0, Color::from_rgba(180, 126, 75, 255));
            draw_circle(
                x + 21.0,
                y + 21.0,
                2.0,
                Color::from_rgba(207, 214, 218, 255),
            );
        }
        haven_core::ObjectKind::Bush => {
            draw_circle(x + 11.0, y + 19.0, 8.0, Color::from_rgba(50, 111, 55, 255));
            draw_circle(x + 21.0, y + 18.0, 9.0, Color::from_rgba(61, 132, 62, 255));
            draw_circle(x + 18.0, y + 15.0, 2.0, Color::from_rgba(178, 67, 63, 255));
        }
        haven_core::ObjectKind::Boulder => {
            draw_circle(x + 16.0, y + 19.0, 12.0, Color::from_rgba(94, 97, 98, 255));
            draw_circle(
                x + 13.0,
                y + 15.0,
                6.0,
                Color::from_rgba(132, 133, 128, 255),
            );
        }
        haven_core::ObjectKind::Mushroom => {
            draw_rectangle(
                x + 14.0,
                y + 18.0,
                4.0,
                8.0,
                Color::from_rgba(211, 189, 151, 255),
            );
            draw_circle(x + 16.0, y + 16.0, 7.0, Color::from_rgba(165, 75, 61, 255));
        }
        haven_core::ObjectKind::Herb => {
            draw_circle(x + 12.0, y + 20.0, 6.0, Color::from_rgba(63, 132, 68, 255));
            draw_circle(x + 20.0, y + 20.0, 6.0, Color::from_rgba(78, 151, 75, 255));
        }
        haven_core::ObjectKind::Crate => {
            draw_rectangle(
                x + 5.0,
                y + 9.0,
                22.0,
                19.0,
                Color::from_rgba(129, 82, 44, 255),
            );
            draw_rectangle_lines(
                x + 5.0,
                y + 9.0,
                22.0,
                19.0,
                2.0,
                Color::from_rgba(75, 43, 26, 255),
            );
        }
        haven_core::ObjectKind::Barrel => {
            draw_circle(x + 16.0, y + 18.0, 11.0, Color::from_rgba(135, 83, 43, 255));
            draw_rectangle(
                x + 6.0,
                y + 15.0,
                20.0,
                3.0,
                Color::from_rgba(65, 60, 57, 255),
            );
        }
        haven_core::ObjectKind::Well => {
            draw_circle(
                x + 16.0,
                y + 20.0,
                11.0,
                Color::from_rgba(113, 112, 105, 255),
            );
            draw_circle(x + 16.0, y + 20.0, 6.0, Color::from_rgba(29, 69, 88, 255));
        }
        haven_core::ObjectKind::Scarecrow => {
            draw_rectangle(
                x + 14.0,
                y + 6.0,
                4.0,
                24.0,
                Color::from_rgba(99, 67, 39, 255),
            );
            draw_rectangle(
                x + 6.0,
                y + 12.0,
                20.0,
                4.0,
                Color::from_rgba(151, 101, 49, 255),
            );
            draw_circle(x + 16.0, y + 7.0, 5.0, Color::from_rgba(218, 179, 96, 255));
        }
        haven_core::ObjectKind::Fence => {
            draw_rectangle(
                x + 4.0,
                y + 13.0,
                24.0,
                5.0,
                Color::from_rgba(126, 82, 45, 255),
            );
            draw_rectangle(
                x + 8.0,
                y + 7.0,
                4.0,
                22.0,
                Color::from_rgba(102, 62, 36, 255),
            );
            draw_rectangle(
                x + 22.0,
                y + 7.0,
                4.0,
                22.0,
                Color::from_rgba(102, 62, 36, 255),
            );
        }
        haven_core::ObjectKind::Lamp => {
            draw_rectangle(
                x + 14.0,
                y + 8.0,
                4.0,
                22.0,
                Color::from_rgba(55, 59, 61, 255),
            );
            draw_circle(x + 16.0, y + 8.0, 6.0, Color::from_rgba(242, 205, 107, 255));
        }
        haven_core::ObjectKind::Bench => {
            draw_rectangle(
                x + 4.0,
                y + 13.0,
                24.0,
                7.0,
                Color::from_rgba(129, 82, 44, 255),
            );
            draw_rectangle(
                x + 7.0,
                y + 20.0,
                4.0,
                8.0,
                Color::from_rgba(83, 51, 31, 255),
            );
            draw_rectangle(
                x + 22.0,
                y + 20.0,
                4.0,
                8.0,
                Color::from_rgba(83, 51, 31, 255),
            );
        }
        haven_core::ObjectKind::Stump => {
            draw_circle(x + 16.0, y + 19.0, 10.0, Color::from_rgba(117, 73, 41, 255));
            draw_circle(x + 16.0, y + 16.0, 7.0, Color::from_rgba(171, 119, 65, 255));
        }
        haven_core::ObjectKind::Log => {
            draw_rectangle(
                x + 4.0,
                y + 14.0,
                24.0,
                11.0,
                Color::from_rgba(111, 69, 38, 255),
            );
            draw_circle(x + 5.0, y + 19.0, 6.0, Color::from_rgba(159, 111, 63, 255));
        }
        haven_core::ObjectKind::Sign => {
            draw_rectangle(
                x + 14.0,
                y + 14.0,
                4.0,
                16.0,
                Color::from_rgba(90, 55, 31, 255),
            );
            draw_rectangle(
                x + 5.0,
                y + 5.0,
                22.0,
                13.0,
                Color::from_rgba(142, 92, 48, 255),
            );
        }
        haven_core::ObjectKind::Door => {
            draw_rectangle(
                x + 9.0,
                y + 2.0,
                14.0,
                29.0,
                Color::from_rgba(89, 54, 31, 255),
            );
            draw_circle(x + 21.0, y + 17.0, 2.0, Color::from_rgba(230, 190, 95, 255));
        }
        haven_core::ObjectKind::Stairs => {
            for step in 0..4 {
                draw_rectangle(
                    x + 5.0 + step as f32 * 4.0,
                    y + 8.0 + step as f32 * 5.0,
                    20.0,
                    4.0,
                    Color::from_rgba(111, 83, 56, 255),
                );
            }
        }
        haven_core::ObjectKind::CaveEntrance => {
            draw_circle(x + 16.0, y + 17.0, 15.0, Color::from_rgba(27, 29, 34, 255));
            draw_rectangle(
                x + 4.0,
                y + 17.0,
                24.0,
                14.0,
                Color::from_rgba(27, 29, 34, 255),
            );
            draw_rectangle_lines(
                x + 3.0,
                y + 6.0,
                26.0,
                25.0,
                2.0,
                Color::from_rgba(83, 80, 74, 255),
            );
        }
    }
}

pub fn draw_panel(x: f32, y: f32, w: f32, h: f32, title: &str) {
    draw_rectangle(x, y, w, h, Color::from_rgba(9, 15, 18, 250));
    draw_rectangle(x, y, w, 34.0, Color::from_rgba(28, 48, 54, 255));
    draw_rectangle_lines(x, y, w, h, 2.0, Color::from_rgba(154, 190, 178, 255));
    draw_text(
        title,
        x + 14.0,
        y + 24.0,
        24.0,
        Color::from_rgba(255, 246, 208, 255),
    );
}

pub fn draw_editor_button(label: &str, x: f32, y: f32, w: f32, h: f32) {
    let (mx, my) = mouse_position();
    let hot = mx >= x && mx <= x + w && my >= y && my <= y + h;
    draw_rectangle(
        x,
        y,
        w,
        h,
        if hot {
            Color::from_rgba(76, 101, 78, 245)
        } else {
            Color::from_rgba(24, 39, 44, 255)
        },
    );
    draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(136, 158, 148, 255));
    draw_text(
        label,
        x + 8.0,
        y + 19.0,
        16.0,
        if hot {
            Color::from_rgba(255, 232, 144, 255)
        } else {
            Color::from_rgba(235, 243, 246, 255)
        },
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPanelRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl UiPanelRect {
    pub fn contains(self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }

    pub fn title_bar_contains(self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + 34.0
    }
}

pub fn resolve_panel_rect(
    anchor: UiAnchor,
    grid_x: i32,
    grid_y: i32,
    width: f32,
    height: f32,
    ui_grid: f32,
) -> UiPanelRect {
    let offset_x = grid_x as f32 * ui_grid;
    let offset_y = grid_y as f32 * ui_grid;
    let x = match anchor {
        UiAnchor::TopLeft | UiAnchor::BottomLeft => offset_x,
        UiAnchor::TopRight | UiAnchor::BottomRight => {
            macroquad::prelude::screen_width() - width - offset_x
        }
    };
    let y = match anchor {
        UiAnchor::TopLeft | UiAnchor::TopRight => offset_y,
        UiAnchor::BottomLeft | UiAnchor::BottomRight => {
            macroquad::prelude::screen_height() - height - offset_y
        }
    };
    UiPanelRect {
        x,
        y,
        w: width,
        h: height,
    }
}

pub fn snap_panel_to_grid(x: f32, y: f32, ui_grid: f32) -> Vec2 {
    vec2(
        (x / ui_grid).round() * ui_grid,
        (y / ui_grid).round() * ui_grid,
    )
}

pub fn panel_grid_from_screen(
    anchor: UiAnchor,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    ui_grid: f32,
) -> (i32, i32) {
    let grid_x = match anchor {
        UiAnchor::TopLeft | UiAnchor::BottomLeft => (x / ui_grid).round() as i32,
        UiAnchor::TopRight | UiAnchor::BottomRight => {
            ((macroquad::prelude::screen_width() - width - x) / ui_grid).round() as i32
        }
    };
    let grid_y = match anchor {
        UiAnchor::TopLeft | UiAnchor::TopRight => (y / ui_grid).round() as i32,
        UiAnchor::BottomLeft | UiAnchor::BottomRight => {
            ((macroquad::prelude::screen_height() - height - y) / ui_grid).round() as i32
        }
    };
    (grid_x.max(0), grid_y.max(0))
}

pub fn draw_inspector_overlay(rect: UiPanelRect, inspector: &InspectorReport) {
    draw_panel(rect.x, rect.y, rect.w, rect.h, &inspector.title);
    for (i, line) in inspector.lines.iter().enumerate() {
        draw_text(
            line,
            rect.x + 18.0,
            rect.y + 58.0 + i as f32 * 24.0,
            19.0,
            Color::from_rgba(220, 231, 236, 255),
        );
    }
}

pub fn draw_world_graph_overlay(
    rect: UiPanelRect,
    scenes: &[SceneMap],
    active_scene: &SceneReference,
) {
    draw_panel(rect.x, rect.y, rect.w, rect.h, "World Graph");
    for (i, scene) in scenes.iter().enumerate() {
        let sy = rect.y + 56.0 + i as f32 * 23.0;
        let color = if active_scene.project_id() == &scene.id {
            Color::from_rgba(255, 232, 144, 255)
        } else {
            Color::from_rgba(220, 231, 236, 255)
        };
        draw_text(
            &format!(
                "{} -> {} link(s), spawn {},{}",
                scene.name,
                scene.transitions.len(),
                scene.spawn_x,
                scene.spawn_y
            ),
            rect.x + 18.0,
            sy,
            17.0,
            color,
        );
    }
}

pub fn draw_validation_overlay(rect: UiPanelRect, validation_messages: &[String]) {
    draw_panel(rect.x, rect.y, rect.w, rect.h, "Validation");
    for (i, message) in validation_messages.iter().take(4).enumerate() {
        draw_text(
            message,
            rect.x + 18.0,
            rect.y + 56.0 + i as f32 * 20.0,
            16.0,
            Color::from_rgba(255, 207, 157, 255),
        );
    }
}

pub fn draw_layout_guides(panel_rects: &[(UiPanelId, UiPanelRect)]) {
    for (panel, rect) in panel_rects {
        draw_rectangle_lines(
            rect.x - 2.0,
            rect.y - 2.0,
            rect.w + 4.0,
            rect.h + 4.0,
            2.0,
            Color::from_rgba(255, 232, 144, 220),
        );
        draw_text(
            panel.label(),
            rect.x + 10.0,
            rect.y - 6.0,
            16.0,
            Color::from_rgba(255, 232, 144, 255),
        );
    }
}

pub async fn load_nearest_texture(path: &str) -> Option<Texture2D> {
    match load_texture(path).await {
        Ok(texture) => {
            texture.set_filter(FilterMode::Nearest);
            Some(texture)
        }
        Err(error) => {
            println!("Could not load {path}: {error}");
            None
        }
    }
}


#[cfg(test)]
mod shared_anchor_tests {
    use super::*;
    use haven_core::{ObjectFootprint, ObjectKind};

    #[test]
    fn tall_tree_uses_bottom_center_collision_root() {
        let tree = PlacedObject::new(ObjectKind::Tree, 10, 20);
        let foot = object_foot_tiles(tree);
        assert_eq!(foot.x, 10.5);
        assert_eq!(foot.y, 21.0);
        assert_eq!(object_sort_y(tree) / TILE_SIZE, foot.y);
    }

    #[test]
    fn multi_cell_stamp_uses_collision_bottom_center_root() {
        let footprint = ObjectFootprint {
            collision_w: 3,
            collision_h: 2,
            interaction_w: 3,
            interaction_h: 2,
            ..ObjectFootprint::single_tile()
        };
        let stamp = PlacedStamp::new("test_stamp", 4, 7, footprint);
        let foot = stamp_foot_tiles(&stamp);
        assert_eq!(foot.x, 5.5);
        assert_eq!(foot.y, 9.0);
        assert_eq!(stamp.sort_y() / TILE_SIZE, foot.y);
    }
}
