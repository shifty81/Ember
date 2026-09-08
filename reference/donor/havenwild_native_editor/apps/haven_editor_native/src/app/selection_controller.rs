use super::*;

impl EditorApp {
    pub(crate) fn active_scene_id(&self) -> Option<ProjectSceneId> {
        self.model
            .world
            .scenes
            .get(self.selected_scene)
            .map(|scene| scene.id.clone())
    }

    pub(crate) fn scene_authoring_layer(&self) -> SceneAuthoringLayer {
        match self.scene_layer_mode {
            SceneLayerMode::Terrain => SceneAuthoringLayer::Terrain,
            SceneLayerMode::Objects => SceneAuthoringLayer::Objects,
            SceneLayerMode::Zones => SceneAuthoringLayer::Zones,
            SceneLayerMode::Transitions => SceneAuthoringLayer::Transitions,
        }
    }

    pub(crate) fn scene_hit_at(&self, x: i32, y: i32) -> Option<haven_editor::CanvasHit> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        hit_test_scene_cell(scene, GridPos { x, y }, self.scene_authoring_layer())
    }

    pub(crate) fn scene_hit_at_cursor(&self) -> Option<haven_editor::CanvasHit> {
        self.scene_hit_at(self.scene_cursor_x, self.scene_cursor_y)
    }

    pub(crate) fn select_scene_hit(&mut self, hit: haven_editor::CanvasHit) {
        let item = hit.selection_item();
        if matches!(item, SelectionItem::Object(_) | SelectionItem::Stamp(_)) {
            self.scene_dock_tab = SceneDockTab::Inspector;
        }
        self.selection
            .replace_with_bounds(hit.scene_id, item, hit.bounds);
    }

    pub(crate) fn selected_object_id(&self) -> Option<haven_editor::ObjectId> {
        self.selection.primary_object_id()
    }

    pub(crate) fn selected_stamp_instance_id(&self) -> Option<haven_editor::StampInstanceId> {
        self.selection.primary_stamp_id()
    }

    pub(crate) fn selected_transition_id(&self) -> Option<haven_editor::TransitionId> {
        self.selection.primary_transition_id()
    }

    pub(crate) fn selected_region_node_index(&self) -> Option<usize> {
        let selected = self.selection.primary_region_node_id()?;
        self.model
            .region_graph
            .nodes
            .iter()
            .position(|node| &node.id == selected)
    }

    pub(crate) fn selected_region_node(&self) -> Option<&RegionNode> {
        let index = self.selected_region_node_index()?;
        self.model.region_graph.nodes.get(index)
    }

    pub(crate) fn select_region_node_index(&mut self, index: usize) {
        let index = index.min(self.model.region_graph.nodes.len().saturating_sub(1));
        if let Some(node) = self.model.region_graph.nodes.get(index) {
            self.selection.set_region_node(node.id.clone());
        } else {
            self.selection.clear();
        }
    }

    pub(crate) fn select_scene_index(&mut self, index: usize) {
        let _ = self.command_bus.commit_gesture();
        if let Some(scene) = self.model.world.scenes.get(self.selected_scene) {
            self.scene_canvas_states
                .insert(scene.id.clone(), self.scene_canvas);
        }
        self.selected_scene = index.min(self.model.world.scenes.len().saturating_sub(1));
        self.object_list_offset = 0;
        self.scene_delete_armed = None;
        self.ensure_scene_visible();
        self.scene_canvas = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .and_then(|scene| self.scene_canvas_states.get(&scene.id).copied())
            .unwrap_or_default();
        self.scene_drag = None;
        self.selection.clear();
        self.clamp_editor_selection();
    }

    pub(crate) fn refresh_scene_selection_bounds(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            self.selection.clear();
            return;
        };
        self.selection.bounds = selection_bounds_for_items(scene, &self.selection.items);
        if self.selection.items.is_empty() {
            self.selection.clear_items();
        }
    }

    pub(crate) fn clamp_editor_selection(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            self.selection.clear();
            return;
        };
        if self
            .selection
            .scene_id
            .as_ref()
            .is_some_and(|id| id != &scene.id)
        {
            self.selection.clear();
            return;
        }
        self.selection.items.retain(|item| match item {
            SelectionItem::Object(id) => scene.map.object(*id).is_some(),
            SelectionItem::Stamp(id) => scene.map.stamp(*id).is_some(),
            SelectionItem::Transition(id) => scene.transition(*id).is_some(),
            SelectionItem::Tile(cell) | SelectionItem::ZoneCell { cell, .. } => {
                cell.x >= 0 && cell.y >= 0 && cell.x < MAP_W as i32 && cell.y < MAP_H as i32
            }
            SelectionItem::Scene(id) => self.model.world.scene_by_id(id).is_some(),
            SelectionItem::RegionNode(id) => self.model.region_graph.node(id).is_some(),
        });
        if self.selection.items.is_empty() {
            self.selection.clear_items();
        } else if self
            .selection
            .primary
            .as_ref()
            .is_none_or(|primary| !self.selection.items.contains(primary))
        {
            self.selection.primary = self.selection.items.first().cloned();
        }
        self.selection.bounds = selection_bounds_for_items(scene, &self.selection.items);
    }
}
