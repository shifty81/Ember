use super::*;

impl EditorApp {
    pub(crate) fn world_surface_value(&self) -> Option<WorldSurfaceValue> {
        match self.world_layer_mode {
            WorldLayerMode::Terrain => Some(WorldSurfaceValue::Terrain(self.selected_tile_kind())),
            WorldLayerMode::Zones => Some(WorldSurfaceValue::Zone(self.selected_zone_kind())),
            WorldLayerMode::StructuralLevels => Some(WorldSurfaceValue::StructuralLevel(
                self.world_structural_level.min(MAX_STRUCTURAL_LEVEL),
            )),
            WorldLayerMode::Objects => None,
        }
    }

    fn world_selected_or_cursor_cells(&self) -> Vec<GridPos> {
        self.world_selection
            .unwrap_or_else(|| {
                GridRect::single(GridPos {
                    x: self.world_cursor_x,
                    y: self.world_cursor_y,
                })
            })
            .cells()
    }

    pub(crate) fn apply_world_structural_level_to_selection(&mut self, level: u8, label: &str) {
        if self.world_layer_mode != WorldLayerMode::StructuralLevels {
            self.status_message =
                "Switch to Levels & Cliffs before applying a platform level".to_string();
            return;
        }
        let Some(manifest) = self.scene_rectangles.clone() else {
            return;
        };
        let cells = self.world_selected_or_cursor_cells();
        let result = paint_world_surface_cells(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            &manifest,
            &self.scene_assignments,
            self.selected_landmass_id,
            cells,
            WorldSurfaceValue::StructuralLevel(level.min(MAX_STRUCTURAL_LEVEL)),
            label.to_string(),
        );
        self.finish_world_result(result);
    }

    pub(crate) fn adjust_world_structural_selection(&mut self, delta: i8) {
        if self.world_layer_mode != WorldLayerMode::StructuralLevels {
            self.status_message =
                "Switch to Levels & Cliffs before raising or lowering a platform".to_string();
            return;
        }
        let Some(manifest) = self.scene_rectangles.clone() else {
            return;
        };
        let label = if delta > 0 {
            "Raise selected platform one level"
        } else {
            "Lower selected platform one level"
        };
        let cells = self.world_selected_or_cursor_cells();
        let result = adjust_world_structural_levels(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            &manifest,
            &self.scene_assignments,
            self.selected_landmass_id,
            cells,
            delta,
            label,
        );
        self.finish_world_result(result);
    }
}
