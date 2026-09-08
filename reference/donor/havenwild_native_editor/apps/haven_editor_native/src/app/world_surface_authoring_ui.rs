use super::render_helpers::*;
use super::*;

const LAYER_TOP: f32 = 218.0;
const TOOL_TOP: f32 = 278.0;
const BRUSH_TOP: f32 = 398.0;
const ACTION_TOP: f32 = 506.0;

fn layer_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 4.0;
    let width = ((rect.w - gap * 3.0) / 4.0).max(54.0);
    Rect::new(
        rect.x + index as f32 * (width + gap),
        rect.y + LAYER_TOP,
        width,
        30.0,
    )
}

fn tool_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 5.0;
    let width = ((rect.w - gap * 2.0) / 3.0).max(78.0);
    let row = index / 3;
    let column = index % 3;
    Rect::new(
        rect.x + column as f32 * (width + gap),
        rect.y + TOOL_TOP + row as f32 * 34.0,
        width,
        29.0,
    )
}

fn value_button_rect(rect: Rect, next: bool) -> Rect {
    let width = 58.0;
    Rect::new(
        if next {
            rect.x + rect.w - width
        } else {
            rect.x
        },
        rect.y + BRUSH_TOP,
        width,
        30.0,
    )
}

fn brush_button_rect(rect: Rect, increase: bool) -> Rect {
    Rect::new(
        rect.x + if increase { 68.0 } else { 0.0 },
        rect.y + BRUSH_TOP + 38.0,
        62.0,
        28.0,
    )
}

fn action_button_rect(rect: Rect, index: usize) -> Rect {
    let gap = 5.0;
    let width = ((rect.w - gap) / 2.0).max(92.0);
    Rect::new(
        rect.x + (index % 2) as f32 * (width + gap),
        rect.y + ACTION_TOP + (index / 2) as f32 * 36.0,
        width,
        30.0,
    )
}

impl EditorApp {
    pub(crate) fn draw_world_surface_inspector(&self, rect: Rect) {
        let Some(manifest) = self.scene_rectangles.as_ref() else {
            draw_editor_text(
                "No world-surface manifest loaded",
                rect.x,
                rect.y,
                22.0,
                WARN,
            );
            return;
        };
        let cursor = GridPos {
            x: self.world_cursor_x,
            y: self.world_cursor_y,
        };
        let address = resolve_world_surface_cell(
            manifest,
            &self.scene_assignments,
            &self.model.world,
            self.selected_landmass_id,
            cursor,
        )
        .ok();
        let scene = address
            .as_ref()
            .and_then(|address| self.model.world.scene_by_id(&address.scene_id));
        let terrain = address
            .as_ref()
            .zip(scene)
            .map(|(address, scene)| scene.map.get(address.local.x, address.local.y).label())
            .unwrap_or("Unavailable");
        let structural_level = address
            .as_ref()
            .zip(scene)
            .and_then(|(address, scene)| {
                scene
                    .map
                    .get_structural_level(address.local.x, address.local.y)
            })
            .map(|level| format!("Level {level}"))
            .unwrap_or_else(|| "Auto / legacy".to_string());
        let zone = address
            .as_ref()
            .zip(scene)
            .map(|(address, scene)| scene.zone_at(address.local.x, address.local.y).label())
            .unwrap_or("None");
        let partition = address
            .as_ref()
            .map(|address| address.rectangle_id.as_str())
            .unwrap_or("outside assigned surface");

        draw_editor_text("Alderreach World Authoring", rect.x, rect.y, 24.0, TEXT);
        draw_editor_text(
            &format!("Global {}, {}", cursor.x, cursor.y),
            rect.x,
            rect.y + 30.0,
            18.0,
            TEXT,
        );
        for (index, line) in [
            format!("Partition: {partition}"),
            format!("Terrain: {terrain}"),
            format!("Structural: {structural_level}"),
            format!("Zone: {zone}"),
            format!("Tool: {}", self.world_edit_tool.label()),
            format!("Layer: {}", self.world_layer_mode.label()),
            format!(
                "Selection: {}",
                self.world_selection
                    .map(|selection| format!("{}x{}", selection.width(), selection.height()))
                    .unwrap_or_else(|| "none".to_string())
            ),
        ]
        .into_iter()
        .enumerate()
        {
            draw_scissored_text(
                &line,
                rect.x,
                rect.y + 56.0 + index as f32 * 20.0,
                rect.w,
                15.0,
                if index < 4 { MUTED } else { TEXT },
            );
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + LAYER_TOP - 30.0, rect.w, 24.0),
            "Authoring Layer",
            Some(self.world_layer_mode.label()),
        );
        for (index, layer) in WorldLayerMode::ALL.into_iter().enumerate() {
            draw_editor_widget(
                layer_button_rect(rect, index),
                layer.label(),
                self.world_layer_mode == layer,
            );
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + TOOL_TOP - 30.0, rect.w, 24.0),
            "Global Tool",
            Some(self.world_edit_tool.label()),
        );
        for (index, tool) in WorldEditTool::ALL.into_iter().enumerate() {
            draw_editor_widget(
                tool_button_rect(rect, index),
                tool.label(),
                self.world_edit_tool == tool,
            );
        }

        let active_brush_label = self.world_active_brush_label();
        draw_section_header(
            Rect::new(rect.x, rect.y + BRUSH_TOP - 30.0, rect.w, 24.0),
            if self.world_layer_mode == WorldLayerMode::StructuralLevels {
                "Platform Level"
            } else {
                "Active Brush"
            },
            Some(&active_brush_label),
        );
        draw_editor_widget(value_button_rect(rect, false), "Prev", false);
        draw_editor_widget(value_button_rect(rect, true), "Next", false);
        draw_scissored_text(
            &active_brush_label,
            rect.x + 64.0,
            rect.y + BRUSH_TOP + 21.0,
            (rect.w - 128.0).max(1.0),
            15.0,
            TEXT,
        );
        if self.world_layer_mode == WorldLayerMode::StructuralLevels {
            draw_editor_widget(brush_button_rect(rect, false), "Apply Sel", false);
            draw_editor_widget(brush_button_rect(rect, true), "Carve L0", false);
            draw_editor_text(
                "Rectangles and selections only",
                rect.x + 142.0,
                rect.y + BRUSH_TOP + 58.0,
                15.0,
                MUTED,
            );
        } else {
            draw_editor_widget(brush_button_rect(rect, false), "Brush -", false);
            draw_editor_widget(brush_button_rect(rect, true), "Brush +", false);
            draw_editor_text(
                &format!(
                    "{}x{}",
                    self.world_brush_radius * 2 + 1,
                    self.world_brush_radius * 2 + 1
                ),
                rect.x + 142.0,
                rect.y + BRUSH_TOP + 58.0,
                16.0,
                MUTED,
            );
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + ACTION_TOP - 30.0, rect.w, 24.0),
            "Transaction Actions",
            Some("Undo / clipboard / framing"),
        );
        let transaction_actions: &[&str] =
            if self.world_layer_mode == WorldLayerMode::StructuralLevels {
                &[
                    "Undo",
                    "Redo",
                    "Copy",
                    "Paste",
                    "Frame",
                    "Open Partition",
                    "Raise Level",
                    "Lower Level",
                ]
            } else {
                &["Undo", "Redo", "Copy", "Paste", "Frame", "Open Partition"]
            };
        for (index, label) in transaction_actions.iter().copied().enumerate() {
            draw_editor_widget(action_button_rect(rect, index), label, false);
        }

        let help_y = rect.y
            + ACTION_TOP
            + if self.world_layer_mode == WorldLayerMode::StructuralLevels {
                144.0
            } else {
                108.0
            };
        draw_wrapped(
            "1-9 tools • F1-F4 layers • Ctrl+C/V copy/paste • Ctrl+Z/Y undo/redo. Structural levels are discrete Level 0-2 regions: select or rectangle a platform, then Apply, Raise, Lower, or Carve. Freeform height painting is disabled.",
            rect.x,
            help_y,
            rect.w,
            14.0,
            MUTED,
        );
        if self.world_show_partitions {
            draw_wrapped(
                "Partition diagnostics are visible. Ctrl+Shift+Arrow moves the selected diagnostic partition; Ctrl+Shift+Backspace clears its override.",
                rect.x,
                help_y + 64.0,
                rect.w,
                13.0,
                WARN,
            );
        }
    }

    pub(crate) fn handle_world_authoring_inspector_click(
        &mut self,
        mx: f32,
        my: f32,
        rect: Rect,
    ) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }
        for (index, layer) in WorldLayerMode::ALL.into_iter().enumerate() {
            if layer_button_rect(rect, index).contains(mouse) {
                self.set_world_layer_mode(layer);
                return true;
            }
        }
        for (index, tool) in WorldEditTool::ALL.into_iter().enumerate() {
            if tool_button_rect(rect, index).contains(mouse) {
                self.set_world_edit_tool(tool);
                return true;
            }
        }
        if value_button_rect(rect, false).contains(mouse) {
            self.cycle_world_brush(-1);
            return true;
        }
        if value_button_rect(rect, true).contains(mouse) {
            self.cycle_world_brush(1);
            return true;
        }
        if brush_button_rect(rect, false).contains(mouse) {
            if self.world_layer_mode == WorldLayerMode::StructuralLevels {
                self.apply_world_structural_level_to_selection(
                    self.world_structural_level,
                    "Apply active level to selected platform",
                );
            } else {
                self.adjust_world_brush_radius(-1);
            }
            return true;
        }
        if brush_button_rect(rect, true).contains(mouse) {
            if self.world_layer_mode == WorldLayerMode::StructuralLevels {
                self.apply_world_structural_level_to_selection(
                    0,
                    "Carve selected platform to Level 0",
                );
            } else {
                self.adjust_world_brush_radius(1);
            }
            return true;
        }
        let action_count = if self.world_layer_mode == WorldLayerMode::StructuralLevels {
            8
        } else {
            6
        };
        for index in 0..action_count {
            if !action_button_rect(rect, index).contains(mouse) {
                continue;
            }
            match index {
                0 => self.undo_world_edit(),
                1 => self.redo_world_edit(),
                2 => self.copy_world_selection(),
                3 => self.paste_world_selection(),
                4 => {
                    self.world_canvas.reset();
                    self.status_message = "Framed Alderreach world surface".to_string();
                }
                5 => self.open_assigned_rectangle_scene(),
                6 => self.adjust_world_structural_selection(1),
                7 => self.adjust_world_structural_selection(-1),
                _ => {}
            }
            return true;
        }
        true
    }

    pub(crate) fn world_active_brush_label(&self) -> String {
        match self.world_layer_mode {
            WorldLayerMode::Terrain => self.selected_tile_kind().label().to_string(),
            WorldLayerMode::Objects => self
                .selected_stamp_id
                .as_deref()
                .and_then(|id| self.stamp_registry.entry(id))
                .map(|entry| entry.label.clone())
                .or_else(|| {
                    self.selected_placeable_id
                        .as_deref()
                        .and_then(|id| self.placeable_registry.entry(id))
                        .map(|entry| entry.label.clone())
                })
                .unwrap_or_else(|| self.selected_object_kind().label().to_string()),
            WorldLayerMode::Zones => self.selected_zone_kind().label().to_string(),
            WorldLayerMode::StructuralLevels => {
                format!("Level {}", self.world_structural_level)
            }
        }
    }

    fn cycle_world_brush(&mut self, direction: i32) {
        match self.world_layer_mode {
            WorldLayerMode::Terrain => {
                let palette = TileKind::LPC_MAPPED_EDITOR_TERRAIN;
                let current = palette
                    .iter()
                    .position(|candidate| *candidate == self.selected_tile_kind())
                    .unwrap_or(0);
                let next = wrap_index(current, palette.len(), direction);
                if let Some(index) = TileKind::ALL
                    .iter()
                    .position(|candidate| *candidate == palette[next])
                {
                    self.selected_tile = index;
                }
            }
            WorldLayerMode::Objects => {
                self.selected_stamp_id = None;
                self.selected_placeable_id = None;
                self.selected_object =
                    wrap_index(self.selected_object, OBJECT_BRUSHES.len(), direction);
            }
            WorldLayerMode::Zones => {
                self.selected_zone = wrap_index(self.selected_zone, ZONE_BRUSHES.len(), direction);
            }
            WorldLayerMode::StructuralLevels => {
                if direction < 0 {
                    self.world_structural_level = self.world_structural_level.saturating_sub(1);
                } else {
                    self.world_structural_level = self
                        .world_structural_level
                        .saturating_add(1)
                        .min(MAX_STRUCTURAL_LEVEL);
                }
            }
        }
        self.status_message = format!("World brush: {}", self.world_active_brush_label());
    }

    fn adjust_world_brush_radius(&mut self, direction: i32) {
        self.world_brush_radius = (self.world_brush_radius + direction).clamp(0, 8);
        self.status_message = format!(
            "World brush {}x{}",
            self.world_brush_radius * 2 + 1,
            self.world_brush_radius * 2 + 1
        );
    }
}

fn wrap_index(current: usize, len: usize, direction: i32) -> usize {
    if len == 0 {
        return 0;
    }
    if direction < 0 {
        (current + len - 1) % len
    } else {
        (current + 1) % len
    }
}
