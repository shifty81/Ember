use super::render_helpers::*;
use super::*;

impl EditorApp {
    pub(crate) fn app_command(
        &self,
        kind: EditorCommandKind,
        description: String,
    ) -> EditorCommand {
        EditorCommand::new(
            kind,
            EditorCommandSource::MainEditor,
            self.model.project.project_id.clone(),
            None,
            None,
            Vec::new(),
            description,
        )
    }

    pub(crate) fn assign_selected_scene_to_rectangle(&mut self) {
        let Some(rectangle_id) = self
            .scene_rectangles
            .as_ref()
            .and_then(|manifest| manifest.scene_rectangles.get(self.selected_rectangle))
            .map(|rectangle| rectangle.scene_id.clone())
        else {
            return;
        };
        let scene = SceneId::ALL[self.selected_scene_cycle];
        self.scene_assignments.assign_scene_to_rectangle(
            scene.code(),
            &rectangle_id,
            scene_role_catalog()[self.selected_role_cycle],
            scene_ownership_catalog()[self.selected_ownership_cycle],
        );
        self.status_message = format!(
            "Assigned {} to {} as {} / {}",
            scene.label(),
            rectangle_id,
            scene_role_catalog()[self.selected_role_cycle],
            scene_ownership_catalog()[self.selected_ownership_cycle]
        );
        self.command_bus.record_event(self.app_command(
            EditorCommandKind::SceneMutation,
            self.status_message.clone(),
        ));
    }

    pub(crate) fn clear_selected_rectangle_assignment(&mut self) {
        let Some(rectangle_id) = self
            .scene_rectangles
            .as_ref()
            .and_then(|manifest| manifest.scene_rectangles.get(self.selected_rectangle))
            .map(|rectangle| rectangle.scene_id.clone())
        else {
            return;
        };
        self.scene_assignments.clear_rectangle(&rectangle_id);
        self.status_message = format!("Cleared assignment for {}", rectangle_id);
        self.command_bus.record_event(self.app_command(
            EditorCommandKind::SceneMutation,
            self.status_message.clone(),
        ));
    }

    pub(crate) fn sync_assignment_cycles_to_selected_rectangle(&mut self) {
        let Some(assignment) = self
            .scene_rectangles
            .as_ref()
            .and_then(|manifest| manifest.scene_rectangles.get(self.selected_rectangle))
            .and_then(|rectangle| {
                self.scene_assignments
                    .assignment_for_rectangle(&rectangle.scene_id)
            })
        else {
            return;
        };
        if let Some(index) = scene_role_catalog()
            .iter()
            .position(|candidate| *candidate == assignment.role.as_str())
        {
            self.selected_role_cycle = index;
        }
        if let Some(index) = scene_ownership_catalog()
            .iter()
            .position(|candidate| *candidate == assignment.ownership.as_str())
        {
            self.selected_ownership_cycle = index;
        }
    }

    pub(crate) fn selected_tile_kind(&self) -> TileKind {
        TileKind::ALL[self.selected_tile]
    }

    pub(crate) fn selected_object_kind(&self) -> ObjectKind {
        OBJECT_BRUSHES[self.selected_object]
    }

    pub(crate) fn selected_zone_kind(&self) -> ZoneKind {
        ZONE_BRUSHES[self.selected_zone]
    }

    pub(crate) fn selected_transition_target_scene(&self) -> ProjectSceneId {
        self.model
            .world
            .scenes
            .get(self.selected_transition_target)
            .map(|scene| scene.id.clone())
            .or_else(|| self.active_scene_id())
            .unwrap_or_else(|| ProjectSceneId::from(SceneId::Farmstead))
    }

    pub(crate) fn selected_scene_object_label(&self, scene: &SceneMap) -> String {
        if let Some(stamp) = self
            .selected_stamp_instance_id()
            .and_then(|id| scene.map.stamp(id))
        {
            return format!(
                "{} at {}, {} ({})",
                stamp.stamp_key, stamp.x, stamp.y, stamp.id
            );
        }
        self.selected_object_id()
            .and_then(|id| scene.map.object(id))
            .map(|object| {
                format!(
                    "{} at {}, {} ({})",
                    object.kind.label(),
                    object.x,
                    object.y,
                    object.id
                )
            })
            .unwrap_or_else(|| "None".to_string())
    }

    pub(crate) fn cycle_active_scene_brush(&mut self, delta: i32) {
        match self.scene_layer_mode {
            SceneLayerMode::Terrain => {
                let palette = TileKind::LPC_MAPPED_EDITOR_TERRAIN;
                let current = palette
                    .iter()
                    .position(|tile| *tile == self.selected_tile_kind())
                    .unwrap_or(0);
                let next = cycle_index(current, palette.len(), delta);
                if let Some(index) = TileKind::ALL.iter().position(|tile| *tile == palette[next]) {
                    self.selected_tile = index;
                }
            }
            SceneLayerMode::Objects => {
                self.selected_stamp_id = None;
                self.selected_object =
                    cycle_index(self.selected_object, OBJECT_BRUSHES.len(), delta);
            }
            SceneLayerMode::Zones => {
                self.selected_zone = cycle_index(self.selected_zone, ZONE_BRUSHES.len(), delta);
            }
            SceneLayerMode::Transitions => {
                self.selected_transition_target = cycle_index(
                    self.selected_transition_target,
                    self.model.world.scenes.len(),
                    delta,
                );
            }
        }
    }

    pub(crate) fn primary_tool_for_active_layer(&self) -> SceneEditTool {
        match self.scene_layer_mode {
            SceneLayerMode::Terrain | SceneLayerMode::Zones => SceneEditTool::Paint,
            SceneLayerMode::Objects | SceneLayerMode::Transitions => SceneEditTool::Place,
        }
    }

    pub(crate) fn set_scene_edit_tool(&mut self, tool: SceneEditTool) {
        let _ = self.command_bus.commit_gesture();
        self.scene_edit_tool = tool;
        self.status_message = format!(
            "{} tool active on {} layer",
            tool.label(),
            self.scene_layer_mode.label()
        );
    }

    pub(crate) fn set_scene_layer_mode(&mut self, layer_mode: SceneLayerMode) {
        let _ = self.command_bus.commit_gesture();
        self.scene_layer_mode = layer_mode;
        self.selection.clear_items();
        self.scene_edit_tool = self.primary_tool_for_active_layer();
        self.status_message = format!(
            "Scene layer: {}. {} tool activated automatically.",
            layer_mode.label(),
            self.scene_edit_tool.label()
        );
        self.command_bus.record_event(self.app_command(
            EditorCommandKind::SceneMutation,
            self.status_message.clone(),
        ));
    }

    pub(crate) fn apply_scene_edit_tool(&mut self) {
        let Some(scene_id) = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .map(|scene| scene.id.clone())
        else {
            return;
        };

        if self.scene_edit_tool == SceneEditTool::Pan {
            self.status_message =
                "Pan tool is active; drag the canvas instead of editing a cell".to_string();
            return;
        }
        if self.scene_edit_tool == SceneEditTool::Rectangle {
            let rect = self.selection.bounds.unwrap_or_else(|| {
                GridRect::single(GridPos {
                    x: self.scene_cursor_x,
                    y: self.scene_cursor_y,
                })
            });
            self.apply_rectangle_tool(rect);
            return;
        }
        if self.scene_edit_tool == SceneEditTool::Fill {
            self.apply_fill_tool();
            return;
        }
        if self.scene_edit_tool == SceneEditTool::Replace {
            self.apply_replace_tool();
            return;
        }
        if self.scene_edit_tool == SceneEditTool::Eyedropper {
            self.pick_scene_brush();
            return;
        }
        if self.scene_edit_tool == SceneEditTool::Select {
            if let Some(hit) = self.scene_hit_at_cursor() {
                let message = match &hit.item {
                    SelectionItem::Tile(cell) => {
                        let tile = self
                            .model
                            .world
                            .scene_by_id(&hit.scene_id)
                            .map(|scene| scene.map.get(cell.x, cell.y).label())
                            .unwrap_or("Unknown");
                        format!("Selected {} tile at {}, {}", tile, cell.x, cell.y)
                    }
                    SelectionItem::Object(id) => self
                        .model
                        .world
                        .scene_by_id(&hit.scene_id)
                        .and_then(|scene| scene.map.object(*id))
                        .map(|object| {
                            format!(
                                "Selected {} at {}, {} ({})",
                                object.kind.label(),
                                object.x,
                                object.y,
                                id
                            )
                        })
                        .unwrap_or_else(|| format!("Selected object {}", id)),
                    SelectionItem::Stamp(id) => self
                        .model
                        .world
                        .scene_by_id(&hit.scene_id)
                        .and_then(|scene| scene.map.stamp(*id))
                        .map(|stamp| {
                            format!(
                                "Selected stamp {} at {}, {} ({})",
                                stamp.stamp_key, stamp.x, stamp.y, id
                            )
                        })
                        .unwrap_or_else(|| format!("Selected stamp {}", id)),
                    SelectionItem::ZoneCell { cell, .. } => self
                        .model
                        .world
                        .scene_by_id(&hit.scene_id)
                        .map(|scene| {
                            format!(
                                "Selected {} zone at {}, {}",
                                scene.zone_at(cell.x, cell.y).label(),
                                cell.x,
                                cell.y
                            )
                        })
                        .unwrap_or_else(|| "Selected zone cell".to_string()),
                    SelectionItem::Transition(id) => self
                        .model
                        .world
                        .scene_by_id(&hit.scene_id)
                        .and_then(|scene| scene.transition(*id))
                        .map(|transition| {
                            format!("Selected transition {} ({})", transition.label, id)
                        })
                        .unwrap_or_else(|| format!("Selected transition {}", id)),
                    SelectionItem::Scene(id) => format!("Selected scene {}", id.label()),
                    SelectionItem::RegionNode(id) => format!("Selected region node {}", id),
                };
                self.select_scene_hit(hit);
                self.status_message = message;
                return;
            }

            if self.scene_layer_mode == SceneLayerMode::Objects {
                if let Some(stamp_id) = self.selected_stamp_instance_id() {
                    match move_scene_stamp(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id.clone(),
                        stamp_id,
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                    ) {
                        Ok(outcome) => {
                            self.status_message = outcome.message;
                            if let Some(hit) = self.scene_hit_at_cursor() {
                                self.select_scene_hit(hit);
                            }
                        }
                        Err(message) => self.status_message = message,
                    }
                    return;
                }
                if let Some(object_id) = self.selected_object_id() {
                    match move_scene_object(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id.clone(),
                        object_id,
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                    ) {
                        Ok(outcome) => {
                            self.status_message = outcome.message;
                            if self.development_client.is_some() {
                                if let Ok(descriptor) = development_session::DevelopmentWorldDescriptor::load() {
                                    if let Err(error) = development_session::publish_live_object_move(
                                        &descriptor, &scene_id, object_id, self.scene_cursor_x, self.scene_cursor_y,
                                    ) {
                                        self.status_message = format!("{} | live sync failed: {}", self.status_message, error);
                                    }
                                }
                            }
                            if let Some(hit) = self.scene_hit_at_cursor() {
                                self.select_scene_hit(hit);
                            }
                        }
                        Err(message) => self.status_message = message,
                    }
                    return;
                }
            }

            self.selection.clear_items();
            self.status_message = format!(
                "No {} content at {}, {}",
                self.scene_layer_mode.label().to_ascii_lowercase(),
                self.scene_cursor_x,
                self.scene_cursor_y
            );
            return;
        }

        let selected_tile = self.selected_tile_kind();
        let selected_object = self.selected_object_kind();
        let selected_stamp = self
            .selected_stamp_id
            .as_deref()
            .and_then(|id| self.stamp_registry.entry(id))
            .cloned();
        let selected_zone = self.selected_zone_kind();
        let selected_transition_target = self.selected_transition_target_scene();
        let selected_transition_spawn = self
            .model
            .world
            .scene(selected_transition_target.clone())
            .map(|scene| (scene.spawn_x, scene.spawn_y))
            .unwrap_or((0, 0));
        let result = match (self.scene_layer_mode, self.scene_edit_tool) {
            (SceneLayerMode::Terrain, SceneEditTool::Paint) => paint_scene_tile_with_mode(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
                selected_tile,
                self.terrain_paint_mode,
            ),
            (SceneLayerMode::Zones, SceneEditTool::Paint) => paint_scene_zone(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
                selected_zone,
            ),
            (SceneLayerMode::Objects, SceneEditTool::Place) => {
                if let Some(definition) = selected_stamp.as_ref() {
                    place_scene_stamp(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id,
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                        definition,
                    )
                } else if let Some(definition) = self
                    .selected_placeable_id
                    .as_deref()
                    .and_then(|id| self.placeable_registry.entry(id))
                    .cloned()
                {
                    place_scene_pack_asset(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id,
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                        definition.compatibility_kind(),
                        definition.footprint,
                        definition.persistent_ref(),
                        definition
                            .states
                            .get(self.selected_placeable_preview_state)
                            .or_else(|| definition.states.first())
                            .map(String::as_str),
                        &definition.label,
                    )
                } else {
                    let result = place_scene_object_with_footprint(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id.clone(),
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                        selected_object,
                        self.placeable_registry
                            .footprint_for_legacy_object(selected_object),
                    );
                    if result.is_ok() && self.development_client.is_some() {
                        if let (Ok(descriptor), Some(object)) = (
                            development_session::DevelopmentWorldDescriptor::load(),
                            self.model.world.scene_by_id(&scene_id)
                                .and_then(|scene| scene.map.object_id_at(self.scene_cursor_x, self.scene_cursor_y))
                                .and_then(|id| self.model.world.scene_by_id(&scene_id).and_then(|scene| scene.map.object(id)).copied()),
                        ) {
                            if let Err(error) = development_session::publish_live_object_spawn(&descriptor, &scene_id, object) {
                                self.status_message = format!("Live spawn sync failed: {error}");
                            }
                        }
                    }
                    result
                }
            }
            (SceneLayerMode::Transitions, SceneEditTool::Place) => create_scene_transition(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
                2,
                2,
                selected_transition_target.clone(),
                selected_transition_spawn.0,
                selected_transition_spawn.1,
                format!("To {}", selected_transition_target.label()),
            ),
            (SceneLayerMode::Objects, SceneEditTool::Erase) => {
                let stamp_here = self
                    .model
                    .world
                    .scene_by_id(&scene_id)
                    .and_then(|scene| {
                        scene
                            .map
                            .stamp_id_at(self.scene_cursor_x, self.scene_cursor_y)
                    })
                    .is_some();
                if stamp_here {
                    erase_scene_stamp(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id,
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                    )
                } else {
                    let object_id = self.model.world.scene_by_id(&scene_id)
                        .and_then(|scene| scene.map.object_id_at(self.scene_cursor_x, self.scene_cursor_y));
                    let result = erase_scene_object(
                        &mut self.model.world,
                        &mut self.command_bus,
                        &self.model.project.project_id,
                        EditorCommandSource::MainEditor,
                        scene_id.clone(),
                        self.scene_cursor_x,
                        self.scene_cursor_y,
                    );
                    if result.is_ok() && self.development_client.is_some() {
                        if let (Some(object_id), Ok(descriptor)) = (object_id, development_session::DevelopmentWorldDescriptor::load()) {
                            if let Err(error) = development_session::publish_live_object_delete(&descriptor, &scene_id, object_id) {
                                self.status_message = format!("Live delete sync failed: {error}");
                            }
                        }
                    }
                    result
                }
            }
            (SceneLayerMode::Transitions, SceneEditTool::Erase) => erase_scene_transition(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
            ),
            (SceneLayerMode::Zones, SceneEditTool::Erase) => paint_scene_zone(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
                ZoneKind::None,
            ),
            (SceneLayerMode::Terrain, SceneEditTool::Erase) => erase_scene_cell(
                &mut self.model.world,
                &mut self.command_bus,
                &self.model.project.project_id,
                EditorCommandSource::MainEditor,
                scene_id,
                self.scene_cursor_x,
                self.scene_cursor_y,
                selected_tile,
            ),
            (SceneLayerMode::Terrain | SceneLayerMode::Zones, SceneEditTool::Place) => {
                Err("Place is for objects and transitions; choose Paint for this layer".to_string())
            }
            (SceneLayerMode::Objects | SceneLayerMode::Transitions, SceneEditTool::Paint) => {
                Err("Paint is for terrain and zones; choose Place for this layer".to_string())
            }
            (
                _,
                SceneEditTool::Select
                | SceneEditTool::Rectangle
                | SceneEditTool::Fill
                | SceneEditTool::Replace
                | SceneEditTool::Eyedropper
                | SceneEditTool::Pan,
            ) => unreachable!(),
        };
        match result {
            Ok(outcome) => {
                self.status_message = outcome.message;
                if self.scene_edit_tool == SceneEditTool::Erase {
                    self.selection.clear_items();
                } else if let Some(hit) = self.scene_hit_at_cursor() {
                    self.select_scene_hit(hit);
                }
                self.clamp_editor_selection();
            }
            Err(error) => {
                self.status_message = error;
            }
        }
    }

    pub(crate) fn resize_transition_under_cursor(&mut self, delta_w: i32, delta_h: i32) {
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        let transition_id = self
            .scene_hit_at_cursor()
            .and_then(|hit| match hit.item {
                SelectionItem::Transition(id) => Some(id),
                _ => None,
            })
            .or_else(|| self.selected_transition_id());
        let Some(transition_id) = transition_id else {
            self.status_message =
                "No transition selected; activate Select and click a transition first".to_string();
            return;
        };
        match resize_scene_transition(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            transition_id,
            delta_w,
            delta_h,
        ) {
            Ok(outcome) => {
                self.status_message = outcome.message;
                if let Some(hit) = self.scene_hit_at_cursor() {
                    self.select_scene_hit(hit);
                }
            }
            Err(error) => {
                self.status_message = error;
            }
        }
    }

    pub(crate) fn undo_world_edit(&mut self) {
        let _ = self.command_bus.commit_gesture();
        match self.command_bus.undo_world(&mut self.model.world) {
            Ok(step) => {
                self.selected_scene = self
                    .selected_scene
                    .min(self.model.world.scenes.len().saturating_sub(1));
                self.clamp_editor_selection();
                self.status_message = if step.typed_transaction {
                    format!(
                        "Undid {} ({} operation{})",
                        step.label,
                        step.operation_count,
                        if step.operation_count == 1 { "" } else { "s" }
                    )
                } else {
                    format!("Undid {} from snapshot fallback", step.label)
                };
            }
            Err(error) => {
                self.status_message = error;
            }
        }
    }

    pub(crate) fn redo_world_edit(&mut self) {
        let _ = self.command_bus.commit_gesture();
        match self.command_bus.redo_world(&mut self.model.world) {
            Ok(step) => {
                self.selected_scene = self
                    .selected_scene
                    .min(self.model.world.scenes.len().saturating_sub(1));
                self.clamp_editor_selection();
                self.status_message = if step.typed_transaction {
                    format!(
                        "Redid {} ({} operation{})",
                        step.label,
                        step.operation_count,
                        if step.operation_count == 1 { "" } else { "s" }
                    )
                } else {
                    format!("Redid {} from snapshot fallback", step.label)
                };
            }
            Err(error) => {
                self.status_message = error;
            }
        }
    }
}
