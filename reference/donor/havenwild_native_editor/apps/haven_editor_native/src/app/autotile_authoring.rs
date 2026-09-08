use super::render_helpers::{cycle_index, draw_editor_widget, draw_scissored_text};
use super::*;
use haven_world::autotile::{E, N, NE, NW, S, SE, SW, W};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AutotilePreset {
    pub label: &'static str,
    pub mask: u8,
}

pub(crate) const AUTOTILE_PRESETS: [AutotilePreset; 16] = [
    AutotilePreset {
        label: "Isolated",
        mask: 0,
    },
    AutotilePreset {
        label: "End N",
        mask: N,
    },
    AutotilePreset {
        label: "End E",
        mask: E,
    },
    AutotilePreset {
        label: "End S",
        mask: S,
    },
    AutotilePreset {
        label: "End W",
        mask: W,
    },
    AutotilePreset {
        label: "Vertical",
        mask: N | S,
    },
    AutotilePreset {
        label: "Horizontal",
        mask: E | W,
    },
    AutotilePreset {
        label: "Corner NE",
        mask: N | E | NE,
    },
    AutotilePreset {
        label: "Corner SE",
        mask: E | S | SE,
    },
    AutotilePreset {
        label: "Corner SW",
        mask: S | W | SW,
    },
    AutotilePreset {
        label: "Corner NW",
        mask: W | N | NW,
    },
    AutotilePreset {
        label: "Tee NES",
        mask: N | E | S | NE | SE,
    },
    AutotilePreset {
        label: "Tee ESW",
        mask: E | S | W | SE | SW,
    },
    AutotilePreset {
        label: "Tee SWN",
        mask: S | W | N | SW | NW,
    },
    AutotilePreset {
        label: "Tee WNE",
        mask: W | N | E | NW | NE,
    },
    AutotilePreset {
        label: "Cross",
        mask: N | E | S | W | NE | SE | SW | NW,
    },
];

pub(crate) fn autotile_toolbar_rect(host: Rect) -> Rect {
    Rect::new(host.x, host.y + 38.0, host.w, 34.0)
}

fn preview_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 8.0, rect.y + 3.0, 92.0, 28.0)
}

fn dirty_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 106.0, rect.y + 3.0, 78.0, 28.0)
}

fn previous_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 194.0, rect.y + 3.0, 30.0, 28.0)
}

fn preset_label_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 228.0, rect.y + 3.0, 124.0, 28.0)
}

fn next_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 356.0, rect.y + 3.0, 30.0, 28.0)
}

fn apply_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 396.0, rect.y + 3.0, 92.0, 28.0)
}

fn clear_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 494.0, rect.y + 3.0, 88.0, 28.0)
}

fn grid_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 590.0, rect.y + 3.0, 68.0, 28.0)
}

fn zones_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 664.0, rect.y + 3.0, 68.0, 28.0)
}

fn links_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 738.0, rect.y + 3.0, 68.0, 28.0)
}

fn clean_button(rect: Rect) -> Rect {
    Rect::new(rect.x + 812.0, rect.y + 3.0, 72.0, 28.0)
}

impl EditorApp {
    pub(crate) fn selected_autotile_preset(&self) -> AutotilePreset {
        AUTOTILE_PRESETS[self.selected_autotile_preset % AUTOTILE_PRESETS.len()]
    }

    pub(crate) fn cycle_autotile_preset(&mut self, delta: i32) {
        self.selected_autotile_preset =
            cycle_index(self.selected_autotile_preset, AUTOTILE_PRESETS.len(), delta);
        let preset = self.selected_autotile_preset();
        self.status_message = format!(
            "Manual autotile preset: {} (mask {:02x})",
            preset.label, preset.mask
        );
    }

    pub(crate) fn draw_autotile_toolbar(&self, host: Rect) {
        let rect = autotile_toolbar_rect(host);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.08, 0.10, 0.10, 1.0),
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
        draw_editor_widget(
            preview_button(rect),
            if self.autotile_preview_enabled {
                "Auto: On"
            } else {
                "Auto: Off"
            },
            self.autotile_preview_enabled,
        );
        draw_editor_widget(
            dirty_button(rect),
            if self.autotile_dirty_overlay {
                "Dirty: On"
            } else {
                "Dirty: Off"
            },
            self.autotile_dirty_overlay,
        );
        draw_editor_widget(previous_button(rect), "<", false);
        draw_editor_widget(
            preset_label_rect(rect),
            self.selected_autotile_preset().label,
            true,
        );
        draw_editor_widget(next_button(rect), ">", false);
        draw_editor_widget(apply_button(rect), "Set Override", false);
        draw_editor_widget(clear_button(rect), "Auto Cell", false);
        draw_editor_widget(
            grid_button(rect),
            if self.scene_show_grid {
                "Grid On"
            } else {
                "Grid Off"
            },
            self.scene_show_grid,
        );
        let zones_visible = self.layer_state(SceneLayerMode::Zones).visible;
        draw_editor_widget(
            zones_button(rect),
            if zones_visible {
                "Zones On"
            } else {
                "Zones Off"
            },
            zones_visible,
        );
        let links_visible = self.layer_state(SceneLayerMode::Transitions).visible;
        draw_editor_widget(
            links_button(rect),
            if links_visible {
                "Links On"
            } else {
                "Links Off"
            },
            links_visible,
        );
        draw_editor_widget(clean_button(rect), "Clean", false);

        let report = self.active_autotile_report();
        draw_scissored_text(
            &format!(
                "resolved {} | overrides {} | last dirty {}",
                report.resolved_cells, report.manual_override_cells, report.recalculated_cells
            ),
            rect.x + 894.0,
            rect.y + 22.0,
            (rect.w - 902.0).max(1.0),
            14.0,
            MUTED,
        );
    }

    pub(crate) fn handle_autotile_toolbar_click(&mut self, mx: f32, my: f32) -> bool {
        if self.viewport_mode != EditorViewportMode::SceneMap {
            return false;
        }
        let rect = autotile_toolbar_rect(self.canvas_host_rect());
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }
        if preview_button(rect).contains(mouse) {
            self.autotile_preview_enabled = !self.autotile_preview_enabled;
            self.status_message = format!(
                "Live autotile preview {}",
                if self.autotile_preview_enabled {
                    "enabled"
                } else {
                    "hidden"
                }
            );
        } else if dirty_button(rect).contains(mouse) {
            self.autotile_dirty_overlay = !self.autotile_dirty_overlay;
            self.status_message = format!(
                "Dirty-neighbor overlay {}",
                if self.autotile_dirty_overlay {
                    "enabled"
                } else {
                    "hidden"
                }
            );
        } else if previous_button(rect).contains(mouse) {
            self.cycle_autotile_preset(-1);
        } else if next_button(rect).contains(mouse) {
            self.cycle_autotile_preset(1);
        } else if apply_button(rect).contains(mouse) {
            self.apply_autotile_override_at_cursor();
        } else if clear_button(rect).contains(mouse) {
            self.clear_autotile_override_at_cursor();
        } else if grid_button(rect).contains(mouse) {
            self.scene_show_grid = !self.scene_show_grid;
            self.status_message = format!(
                "Scene grid {}",
                if self.scene_show_grid {
                    "shown"
                } else {
                    "hidden"
                }
            );
        } else if zones_button(rect).contains(mouse) {
            self.toggle_layer_visibility(SceneLayerMode::Zones);
        } else if links_button(rect).contains(mouse) {
            self.toggle_layer_visibility(SceneLayerMode::Transitions);
        } else if clean_button(rect).contains(mouse) {
            self.scene_show_grid = false;
            self.autotile_dirty_overlay = false;
            self.scene_layers[SceneLayerMode::Zones.index()].visible = false;
            self.scene_layers[SceneLayerMode::Transitions.index()].visible = false;
            self.status_message =
                "Clean authoring view: grid, zones, links, and dirty overlays hidden".to_string();
        }
        true
    }

    pub(crate) fn apply_autotile_override_at_cursor(&mut self) {
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        if self.scene_layer_mode != SceneLayerMode::Terrain {
            self.status_message = "Autotile overrides are edited on the Terrain layer".to_string();
            return;
        }
        let preset = self.selected_autotile_preset();
        match set_scene_autotile_override(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            self.scene_cursor_x,
            self.scene_cursor_y,
            preset.mask,
        ) {
            Ok(outcome) => self.status_message = outcome.message,
            Err(error) => self.status_message = error,
        }
    }

    pub(crate) fn clear_autotile_override_at_cursor(&mut self) {
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        match clear_scene_autotile_override(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            self.scene_cursor_x,
            self.scene_cursor_y,
        ) {
            Ok(outcome) => self.status_message = outcome.message,
            Err(error) => self.status_message = error,
        }
    }

    pub(crate) fn active_autotile_report(&self) -> AutotileSyncReport {
        self.active_scene_id()
            .and_then(|scene_id| self.autotile_caches.get(&scene_id))
            .map(LiveAutotileCache::last_report)
            .unwrap_or_default()
    }
}

impl EditorApp {
    /// Exact Terrain Standard v1 tuple resolution under the scene cursor.
    pub(crate) fn terrain_tuple_resolution_at_cursor(
        &self,
    ) -> Result<haven_world::TerrainTupleResolution, String> {
        let scene = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .ok_or_else(|| "No scene loaded".to_string())?;
        let resolver = haven_world::embedded_terrain_tuple_resolver().map_err(str::to_string)?;
        Ok(haven_world::resolve_semantic_tuple_at(
            resolver,
            &scene.map,
            self.scene_cursor_x,
            self.scene_cursor_y,
        ))
    }
}
