use super::*;

impl SceneLayerMode {
    pub(crate) fn index(self) -> usize {
        match self {
            SceneLayerMode::Terrain => 0,
            SceneLayerMode::Objects => 1,
            SceneLayerMode::Zones => 2,
            SceneLayerMode::Transitions => 3,
        }
    }

    pub(crate) fn from_authoring_layer(layer: SceneAuthoringLayer) -> Self {
        match layer {
            SceneAuthoringLayer::Terrain => SceneLayerMode::Terrain,
            SceneAuthoringLayer::Objects => SceneLayerMode::Objects,
            SceneAuthoringLayer::Zones => SceneLayerMode::Zones,
            SceneAuthoringLayer::Transitions => SceneLayerMode::Transitions,
        }
    }
}

impl EditorApp {
    pub(crate) fn active_layer_state(&self) -> SceneLayerState {
        self.scene_layers[self.scene_layer_mode.index()]
    }

    pub(crate) fn layer_state(&self, mode: SceneLayerMode) -> SceneLayerState {
        self.scene_layers[mode.index()]
    }

    pub(crate) fn toggle_layer_visibility(&mut self, mode: SceneLayerMode) {
        let state = &mut self.scene_layers[mode.index()];
        state.visible = !state.visible;
        self.status_message = format!(
            "{} layer {}",
            mode.label(),
            if state.visible { "visible" } else { "hidden" }
        );
    }

    pub(crate) fn toggle_layer_lock(&mut self, mode: SceneLayerMode) {
        let state = &mut self.scene_layers[mode.index()];
        state.locked = !state.locked;
        self.status_message = format!(
            "{} layer {}",
            mode.label(),
            if state.locked { "locked" } else { "unlocked" }
        );
    }

    pub(crate) fn adjust_active_layer_opacity(&mut self, delta: f32) {
        let mode = self.scene_layer_mode;
        let index = mode.index();
        self.scene_layers[index].opacity =
            (self.scene_layers[index].opacity + delta).clamp(0.1, 1.0);
        self.status_message = format!(
            "{} layer opacity {:.0}%",
            mode.label(),
            self.scene_layers[index].opacity * 100.0
        );
    }

    pub(crate) fn update_scene_map_input(
        &mut self,
        pointer_consumed: bool,
        canvas_pointer_consumed: bool,
    ) {
        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let left_down = is_mouse_button_down(MouseButton::Left);
        let left_released = is_mouse_button_released(MouseButton::Left);
        let control_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

        if !pointer_consumed && !canvas_pointer_consumed {
            if left_pressed {
                if let Some((x, y)) = self.scene_cell_at_mouse() {
                    self.scene_cursor_x = x;
                    self.scene_cursor_y = y;
                    self.begin_scene_canvas_action(GridPos { x, y }, shift_down);
                }
            } else if left_down {
                if let Some((x, y)) = self.scene_cell_at_mouse() {
                    self.scene_cursor_x = x;
                    self.scene_cursor_y = y;
                    self.update_scene_canvas_action(GridPos { x, y });
                }
            }
        }

        if left_released {
            self.finish_scene_canvas_action();
        }

        if is_key_pressed(KeyCode::Escape) {
            if self.command_bus.gesture_is_open() {
                match self.command_bus.cancel_gesture(&mut self.model.world) {
                    Ok(()) => self.status_message = "Cancelled active edit gesture".to_string(),
                    Err(error) => self.status_message = format!("Gesture cancel failed: {error}"),
                }
            }
            self.scene_drag = None;
            self.last_painted_cell = None;
        }

        // Right-click is reserved for the scene asset context menu.
        // Erasing remains an explicit tool action so source editing is discoverable.

        self.handle_scene_keyboard_shortcuts(control_down, shift_down);
    }

    fn begin_scene_canvas_action(&mut self, cell: GridPos, additive: bool) {
        match self.scene_edit_tool {
            SceneEditTool::Pan => {}
            SceneEditTool::Paint | SceneEditTool::Erase => {
                if self.active_layer_state().locked {
                    self.status_message =
                        format!("{} layer is locked", self.scene_layer_mode.label());
                    return;
                }
                self.command_bus.begin_gesture();
                self.apply_scene_edit_tool();
                self.last_painted_cell = Some((cell.x, cell.y));
            }
            SceneEditTool::Place => {
                if self.active_layer_state().locked {
                    self.status_message =
                        format!("{} layer is locked", self.scene_layer_mode.label());
                } else {
                    self.apply_scene_edit_tool();
                }
            }
            SceneEditTool::Fill => self.apply_fill_tool(),
            SceneEditTool::Replace => self.apply_replace_tool(),
            SceneEditTool::Eyedropper => self.pick_scene_brush(),
            SceneEditTool::Rectangle => {
                if self.active_layer_state().locked {
                    self.status_message =
                        format!("{} layer is locked", self.scene_layer_mode.label());
                } else if matches!(
                    self.scene_layer_mode,
                    SceneLayerMode::Terrain | SceneLayerMode::Zones
                ) {
                    self.scene_drag = Some(SceneCanvasDrag {
                        kind: SceneCanvasDragKind::Rectangle,
                        start: cell,
                        current: cell,
                        additive: false,
                    });
                } else {
                    self.status_message =
                        "Rectangle tool is available for terrain and zones".to_string();
                }
            }
            SceneEditTool::Select => self.begin_selection_action(cell, additive),
        }
    }

    fn update_scene_canvas_action(&mut self, cell: GridPos) {
        if let Some(drag) = self.scene_drag.as_mut() {
            drag.current = cell;
            return;
        }
        if matches!(
            self.scene_edit_tool,
            SceneEditTool::Paint | SceneEditTool::Erase
        ) && !self.active_layer_state().locked
            && self.last_painted_cell != Some((cell.x, cell.y))
        {
            self.apply_scene_edit_tool();
            self.last_painted_cell = Some((cell.x, cell.y));
        }
    }

    fn finish_scene_canvas_action(&mut self) {
        if let Some(drag) = self.scene_drag.take() {
            match drag.kind {
                SceneCanvasDragKind::Rectangle => self.apply_rectangle_tool(drag.rect()),
                SceneCanvasDragKind::Marquee => {
                    if drag.additive && drag.start == drag.current {
                        if let Some(hit) = self.scene_hit_at(drag.start.x, drag.start.y) {
                            let item = hit.selection_item();
                            self.selection.toggle(hit.scene_id, item, hit.bounds);
                            self.refresh_scene_selection_bounds();
                            self.status_message = format!(
                                "Toggled {} selection at {}, {}",
                                self.scene_layer_mode.label().to_ascii_lowercase(),
                                drag.start.x,
                                drag.start.y
                            );
                        }
                    } else {
                        self.select_scene_rectangle(drag.rect(), drag.additive);
                    }
                }
                SceneCanvasDragKind::MoveSelection => {
                    let delta = GridPos {
                        x: drag.current.x - drag.start.x,
                        y: drag.current.y - drag.start.y,
                    };
                    if delta.x != 0 || delta.y != 0 {
                        self.move_current_selection(delta);
                    }
                }
            }
        }
        if let Some(step) = self.command_bus.commit_gesture() {
            if step.operation_count > 1 {
                self.status_message = format!(
                    "Committed {} as one undo step ({} operations)",
                    step.label, step.operation_count
                );
            }
        }
        self.last_painted_cell = None;
    }

    fn begin_selection_action(&mut self, cell: GridPos, additive: bool) {
        let hit = self.scene_hit_at(cell.x, cell.y);
        if let Some(hit) = hit {
            let item = hit.selection_item();
            let can_drag_existing = !additive && self.selection.contains(&item);
            let can_drag_new_entity = !additive
                && matches!(
                    self.scene_layer_mode,
                    SceneLayerMode::Objects | SceneLayerMode::Transitions
                );
            if can_drag_existing || can_drag_new_entity {
                if can_drag_new_entity && !self.selection.contains(&item) {
                    self.select_scene_hit(hit);
                    self.refresh_scene_selection_bounds();
                }
                if !self.active_layer_state().locked && self.selection.contains(&item) {
                    self.scene_drag = Some(SceneCanvasDrag {
                        kind: SceneCanvasDragKind::MoveSelection,
                        start: cell,
                        current: cell,
                        additive,
                    });
                }
                return;
            }
        }
        self.scene_drag = Some(SceneCanvasDrag {
            kind: SceneCanvasDragKind::Marquee,
            start: cell,
            current: cell,
            additive,
        });
    }

    fn handle_scene_keyboard_shortcuts(&mut self, control_down: bool, shift_down: bool) {
        let transition_resize_mode =
            self.scene_layer_mode == SceneLayerMode::Transitions && shift_down && !control_down;
        if is_key_pressed(KeyCode::PageDown) {
            let next =
                (self.selected_scene + 1).min(self.model.world.scenes.len().saturating_sub(1));
            self.select_scene_index(next);
        }
        if is_key_pressed(KeyCode::PageUp) {
            self.select_scene_index(self.selected_scene.saturating_sub(1));
        }
        if transition_resize_mode && is_key_pressed(KeyCode::Right) {
            self.resize_transition_under_cursor(1, 0);
        } else if is_key_pressed(KeyCode::Right) {
            self.scene_cursor_x = (self.scene_cursor_x + 1).min(MAP_W as i32 - 1);
        }
        if transition_resize_mode && is_key_pressed(KeyCode::Left) {
            self.resize_transition_under_cursor(-1, 0);
        } else if is_key_pressed(KeyCode::Left) {
            self.scene_cursor_x = self.scene_cursor_x.saturating_sub(1);
        }
        if transition_resize_mode && is_key_pressed(KeyCode::Down) {
            self.resize_transition_under_cursor(0, 1);
        } else if is_key_pressed(KeyCode::Down) {
            self.scene_cursor_y = (self.scene_cursor_y + 1).min(MAP_H as i32 - 1);
        }
        if transition_resize_mode && is_key_pressed(KeyCode::Up) {
            self.resize_transition_under_cursor(0, -1);
        } else if is_key_pressed(KeyCode::Up) {
            self.scene_cursor_y = self.scene_cursor_y.saturating_sub(1);
        }

        if is_key_pressed(KeyCode::E) {
            self.cycle_active_scene_brush(1);
        }
        if is_key_pressed(KeyCode::Q) {
            self.cycle_active_scene_brush(-1);
        }
        for (key, tool) in [
            (KeyCode::Key1, SceneEditTool::Select),
            (KeyCode::Key2, SceneEditTool::Paint),
            (KeyCode::Key3, SceneEditTool::Rectangle),
            (KeyCode::Key4, SceneEditTool::Fill),
            (KeyCode::Key5, SceneEditTool::Replace),
            (KeyCode::Key6, SceneEditTool::Eyedropper),
            (KeyCode::Key7, SceneEditTool::Place),
            (KeyCode::Key8, SceneEditTool::Erase),
            (KeyCode::Key9, SceneEditTool::Pan),
        ] {
            if is_key_pressed(key) {
                self.set_scene_edit_tool(tool);
            }
        }
        for (key, layer) in [
            (KeyCode::F1, SceneLayerMode::Terrain),
            (KeyCode::F2, SceneLayerMode::Objects),
            (KeyCode::F3, SceneLayerMode::Zones),
            (KeyCode::F4, SceneLayerMode::Transitions),
        ] {
            if is_key_pressed(key) {
                self.set_scene_layer_mode(layer);
            }
        }

        if control_down && is_key_pressed(KeyCode::C) {
            self.copy_selection_to_clipboard();
        }
        if control_down && is_key_pressed(KeyCode::X) {
            self.cut_current_selection();
        }
        if control_down && is_key_pressed(KeyCode::V) {
            self.paste_clipboard_at_cursor();
        }
        if control_down && is_key_pressed(KeyCode::D) {
            self.duplicate_current_selection();
        }
        if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
            self.delete_current_selection();
        }
        if shift_down && is_key_pressed(KeyCode::F) {
            self.frame_current_selection();
        }
        if is_key_pressed(KeyCode::P) && !control_down {
            self.autotile_preview_enabled = !self.autotile_preview_enabled;
            self.status_message = format!(
                "Live autotile preview {}",
                if self.autotile_preview_enabled {
                    "enabled"
                } else {
                    "hidden"
                }
            );
        }
        if is_key_pressed(KeyCode::M) && !control_down {
            self.cycle_autotile_preset(1);
        }
        if is_key_pressed(KeyCode::O) && !control_down {
            if shift_down {
                self.clear_autotile_override_at_cursor();
            } else {
                self.apply_autotile_override_at_cursor();
            }
        }
        if is_key_pressed(KeyCode::Enter) {
            self.apply_scene_edit_tool();
        }
        if control_down
            && (is_key_pressed(KeyCode::Y) || (shift_down && is_key_pressed(KeyCode::Z)))
        {
            self.redo_world_edit();
        } else if control_down && is_key_pressed(KeyCode::Z) {
            self.undo_world_edit();
        }
    }

    pub(crate) fn apply_rectangle_tool(&mut self, rect: GridRect) {
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        let layer = self.scene_authoring_layer();
        let tile = self.selected_tile_kind();
        let zone = self.selected_zone_kind();
        let result = paint_scene_rectangle(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            rect,
            layer,
            tile,
            zone,
        );
        self.finish_bulk_result(result);
    }

    pub(crate) fn apply_fill_tool(&mut self) {
        if self.active_layer_state().locked {
            self.status_message = format!("{} layer is locked", self.scene_layer_mode.label());
            return;
        }
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        let layer = self.scene_authoring_layer();
        let tile = self.selected_tile_kind();
        let zone = self.selected_zone_kind();
        let result = flood_fill_scene(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            GridPos {
                x: self.scene_cursor_x,
                y: self.scene_cursor_y,
            },
            layer,
            tile,
            zone,
        );
        self.finish_bulk_result(result);
    }

    pub(crate) fn apply_replace_tool(&mut self) {
        if self.active_layer_state().locked {
            self.status_message = format!("{} layer is locked", self.scene_layer_mode.label());
            return;
        }
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        let layer = self.scene_authoring_layer();
        let tile = self.selected_tile_kind();
        let zone = self.selected_zone_kind();
        let result = replace_scene_value(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            GridPos {
                x: self.scene_cursor_x,
                y: self.scene_cursor_y,
            },
            layer,
            tile,
            zone,
        );
        self.finish_bulk_result(result);
    }

    pub(crate) fn pick_scene_brush(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        match self.scene_layer_mode {
            SceneLayerMode::Terrain => {
                let tile = scene.map.get(self.scene_cursor_x, self.scene_cursor_y);
                if let Some(index) = TileKind::ALL
                    .iter()
                    .position(|candidate| *candidate == tile)
                {
                    self.selected_tile = index;
                    self.scene_edit_tool = SceneEditTool::Paint;
                    self.status_message = format!("Picked {} terrain brush", tile.label());
                }
            }
            SceneLayerMode::Zones => {
                let zone = scene.zone_at(self.scene_cursor_x, self.scene_cursor_y);
                if let Some(index) = ZONE_BRUSHES.iter().position(|candidate| *candidate == zone) {
                    self.selected_zone = index;
                    self.scene_edit_tool = SceneEditTool::Paint;
                    self.status_message = format!("Picked {} zone brush", zone.label());
                } else {
                    self.status_message = "The sampled cell has no paintable zone".to_string();
                }
            }
            SceneLayerMode::Objects => {
                if let Some(stamp_id) = scene
                    .map
                    .stamp_id_at(self.scene_cursor_x, self.scene_cursor_y)
                {
                    if let Some(stamp) = scene.map.stamp(stamp_id) {
                        self.selected_stamp_id = Some(stamp.stamp_key.clone());
                        self.scene_edit_tool = SceneEditTool::Place;
                        self.status_message = format!("Picked {} stamp brush", stamp.stamp_key);
                        return;
                    }
                }
                let Some(id) = scene
                    .map
                    .object_id_at(self.scene_cursor_x, self.scene_cursor_y)
                else {
                    self.status_message = "No object or stamp under the eyedropper".to_string();
                    return;
                };
                let Some(object) = scene.map.object(id) else {
                    return;
                };
                if let Some(index) = OBJECT_BRUSHES
                    .iter()
                    .position(|candidate| *candidate == object.kind)
                {
                    self.selected_stamp_id = None;
                    self.selected_object = index;
                    self.scene_edit_tool = SceneEditTool::Place;
                    self.status_message = format!("Picked {} object brush", object.kind.label());
                }
            }
            SceneLayerMode::Transitions => {
                let Some(transition) =
                    scene.transition_at(self.scene_cursor_x, self.scene_cursor_y)
                else {
                    self.status_message = "No transition under the eyedropper".to_string();
                    return;
                };
                if let Some(index) = self
                    .model
                    .world
                    .scenes
                    .iter()
                    .position(|candidate| &candidate.id == transition.target.project_id())
                {
                    self.selected_transition_target = index;
                    self.scene_edit_tool = SceneEditTool::Place;
                    self.status_message =
                        format!("Picked transition target {}", transition.target.label());
                } else {
                    self.status_message = format!(
                        "Transition target {} is not loaded in this project",
                        transition.target.label()
                    );
                }
            }
        }
    }

    pub(crate) fn select_scene_rectangle(&mut self, rect: GridRect, additive: bool) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        let items = selection_items_in_rect(scene, self.scene_authoring_layer(), rect);
        if additive {
            self.selection.add_many(scene.id.clone(), items, None);
        } else if items.is_empty() {
            self.selection.clear_items();
        } else {
            self.selection.replace_many(scene.id.clone(), items, None);
        }
        self.selection.bounds = selection_bounds_for_items(scene, &self.selection.items);
        self.status_message = format!(
            "Selected {} {} item{} in {}x{} marquee",
            self.selection.items.len(),
            self.scene_layer_mode.label().to_ascii_lowercase(),
            if self.selection.items.len() == 1 {
                ""
            } else {
                "s"
            },
            rect.width(),
            rect.height()
        );
    }

    pub(crate) fn move_current_selection(&mut self, delta: GridPos) {
        if self.active_layer_state().locked {
            self.status_message = format!("{} layer is locked", self.scene_layer_mode.label());
            return;
        }
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        let Some(bounds) = self.selection.bounds else {
            self.status_message = "Selection has no movable bounds".to_string();
            return;
        };
        let layer = self.scene_authoring_layer();
        let tile = self.selected_tile_kind();
        let result = move_scene_selection(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id.clone(),
            layer,
            &self.selection.items,
            bounds,
            delta,
            tile,
        );
        match result {
            Ok(outcome) => {
                self.status_message = outcome.edit.message;
                self.selection
                    .replace_many(scene_id, outcome.selected_items, outcome.bounds);
            }
            Err(error) => self.status_message = error,
        }
    }

    pub(crate) fn copy_selection_to_clipboard(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        let Some(bounds) = self.selection.bounds else {
            self.status_message = "Nothing selected to copy".to_string();
            return;
        };
        match copy_scene_selection(
            scene,
            self.scene_authoring_layer(),
            &self.selection.items,
            bounds,
        ) {
            Ok(clipboard) => {
                let count = clipboard.item_count();
                self.scene_clipboard = Some(clipboard);
                self.status_message =
                    format!("Copied {count} item{}", if count == 1 { "" } else { "s" });
            }
            Err(error) => self.status_message = error,
        }
    }

    pub(crate) fn cut_current_selection(&mut self) {
        let previous_clipboard = self.scene_clipboard.clone();
        self.scene_clipboard = None;
        self.copy_selection_to_clipboard();
        if self.scene_clipboard.is_some() {
            self.delete_current_selection();
        } else {
            self.scene_clipboard = previous_clipboard;
        }
    }
}
