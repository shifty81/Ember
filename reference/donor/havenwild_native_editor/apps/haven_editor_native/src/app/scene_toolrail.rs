use super::render_helpers::*;
use super::*;

impl EditorApp {
    pub(crate) fn draw_scene_toolrail(&self, rect: Rect) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        draw_editor_text("Scene Editor", rect.x, rect.y + 20.0, 23.0, TEXT);
        draw_editor_text(&scene.name, rect.x, rect.y + 40.0, 14.0, MUTED);

        draw_section_header(
            Rect::new(rect.x, rect.y + 52.0, rect.w, 24.0),
            "Tools",
            Some("1–9"),
        );
        for (index, tool) in [
            SceneEditTool::Select,
            SceneEditTool::Paint,
            SceneEditTool::Rectangle,
            SceneEditTool::Fill,
            SceneEditTool::Replace,
            SceneEditTool::Eyedropper,
            SceneEditTool::Place,
            SceneEditTool::Erase,
            SceneEditTool::Pan,
        ]
        .into_iter()
        .enumerate()
        {
            draw_editor_widget(
                scene_tool_button_rect(rect, index),
                tool.label(),
                tool == self.scene_edit_tool,
            );
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + 166.0, rect.w, 24.0),
            "Layers",
            Some("F1–F4"),
        );
        for (index, layer) in [
            SceneLayerMode::Terrain,
            SceneLayerMode::Objects,
            SceneLayerMode::Zones,
            SceneLayerMode::Transitions,
        ]
        .into_iter()
        .enumerate()
        {
            let state = self.layer_state(layer);
            draw_editor_widget(
                scene_layer_button_rect(rect, index),
                layer.label(),
                layer == self.scene_layer_mode,
            );
            draw_editor_widget(
                scene_layer_visibility_button_rect(rect, index),
                if state.visible { "V" } else { "H" },
                state.visible,
            );
            draw_editor_widget(
                scene_layer_lock_button_rect(rect, index),
                if state.locked { "L" } else { "U" },
                state.locked,
            );
        }
        let active_layer = self.active_layer_state();
        draw_editor_text(
            &format!("Opacity {:.0}%", active_layer.opacity * 100.0),
            rect.x,
            rect.y + 330.0,
            15.0,
            MUTED,
        );
        draw_editor_widget(scene_layer_opacity_down_rect(rect), "-", false);
        draw_editor_widget(scene_layer_opacity_up_rect(rect), "+", false);

        if self.scene_layer_mode == SceneLayerMode::Terrain {
            draw_section_header(
                Rect::new(rect.x, rect.y + 338.0, rect.w, 24.0),
                "Paint Mode",
                Some("Exact is the safe default"),
            );
            for (index, mode) in TerrainPaintMode::ALL.into_iter().enumerate() {
                draw_editor_widget(
                    scene_terrain_paint_mode_button_rect(rect, index),
                    mode.label(),
                    mode == self.terrain_paint_mode,
                );
            }
        }
        let (content_title, content_detail) = if self.scene_layer_mode == SceneLayerMode::Terrain {
            (
                "Terrain Standard v1",
                haven_world::terrain_standard_code(self.selected_tile_kind()),
            )
        } else {
            ("Content", "Q / E cycle")
        };
        draw_section_header(
            Rect::new(rect.x, rect.y + 398.0, rect.w, 24.0),
            content_title,
            Some(content_detail),
        );
        let labels: Vec<(&str, bool)> = match self.scene_layer_mode {
            SceneLayerMode::Terrain => TileKind::LPC_MAPPED_EDITOR_TERRAIN
                .iter()
                .map(|tile| (tile.label(), *tile == self.selected_tile_kind()))
                .collect(),
            SceneLayerMode::Objects => OBJECT_BRUSHES
                .iter()
                .enumerate()
                .map(|(index, object)| (object.label(), index == self.selected_object))
                .collect(),
            SceneLayerMode::Zones => ZONE_BRUSHES
                .iter()
                .enumerate()
                .map(|(index, zone)| (zone.label(), index == self.selected_zone))
                .collect(),
            SceneLayerMode::Transitions => self
                .model
                .world
                .scenes
                .iter()
                .enumerate()
                .map(|(index, target)| {
                    (
                        target.name.as_str(),
                        index == self.selected_transition_target,
                    )
                })
                .collect(),
        };
        let selected_index = labels.iter().position(|(_, active)| *active).unwrap_or(0);
        let content_count = labels.len();
        let page_start = scene_content_page_start(rect, content_count, selected_index);
        let capacity = scene_content_capacity(rect);
        for (slot, (label, active)) in labels
            .into_iter()
            .skip(page_start)
            .take(capacity)
            .enumerate()
        {
            draw_editor_widget(scene_content_button_rect(rect, slot), label, active);
        }

        let action_y = scene_apply_button_rect(rect).y;
        if content_count > capacity {
            let page_end = (page_start + capacity).min(content_count);
            draw_scissored_text(
                &format!(
                    "Showing {}-{} of {} | Q/E cycles all",
                    page_start + 1,
                    page_end,
                    content_count
                ),
                rect.x,
                action_y - 34.0,
                rect.w,
                13.0,
                MUTED,
            );
        }
        draw_editor_text(
            &format!(
                "{} | {}{} | cell {}, {}",
                self.scene_edit_tool.label(),
                self.scene_layer_mode.label(),
                if self.scene_layer_mode == SceneLayerMode::Terrain {
                    format!(" / {}", self.terrain_paint_mode.label())
                } else {
                    String::new()
                },
                self.scene_cursor_x,
                self.scene_cursor_y
            ),
            rect.x,
            action_y - 14.0,
            16.0,
            MUTED,
        );
        draw_editor_widget_tone(
            scene_apply_button_rect(rect),
            "Apply Semantic",
            true,
            WidgetTone::Primary,
        );
        draw_editor_widget(scene_erase_button_rect(rect), "Erase Cell", false);
        draw_editor_text(
            "1-9 tools | F1-F4 layers | Q/E brush",
            rect.x,
            action_y + 52.0,
            15.0,
            MUTED,
        );
        draw_editor_text(
            "Ctrl+C/X/V/D | P preview | M preset | O override",
            rect.x,
            action_y + 72.0,
            15.0,
            GOOD,
        );
    }
}
