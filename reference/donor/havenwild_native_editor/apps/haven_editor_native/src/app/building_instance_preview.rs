use super::*;
use haven_assets::{
    asset_intake::repo_root_dir,
    building_instance::{
        BuildingInstanceDefinition, BuildingInstanceOrigin, BuildingInstanceViewState,
        BuildingPlacementSpace, BUILDING_INSTANCE_SCHEMA,
    },
    building_recipe::BuildingRecipePieceKind,
};

impl EditorApp {
    fn resolved_building_instances_for_scene(&self, scene: &SceneMap) -> Vec<BuildingInstanceDefinition> {
        if scene.kind == haven_core::SceneKind::Exterior {
            let manifest = haven_world::continuous_surface::ContinuousSurfaceManifest::for_world(&self.model.world);
            let origin = manifest
                .chunk_for_scene(&scene.id)
                .map(|chunk| [chunk.x * MAP_W as i32, chunk.y * MAP_H as i32]);
            self.building_instance_registry.resolved_for_scene(
                scene.id.code(),
                manifest.pcg_region.as_deref(),
                origin,
                [MAP_W as i32, MAP_H as i32],
                &self.building_recipe_registry,
            )
        } else {
            self.building_instance_registry.resolved_for_scene(
                scene.id.code(),
                None,
                None,
                [MAP_W as i32, MAP_H as i32],
                &self.building_recipe_registry,
            )
        }
    }

    pub(crate) fn draw_building_instance_previews(&self, scene: &SceneMap) {
        let mut pieces = Vec::new();
        for instance in self.resolved_building_instances_for_scene(scene) {
            let Some(recipe) = self.building_recipe_registry.entry(&instance.recipe_id) else {
                continue;
            };
            let state = self
                .building_preview_views
                .get(&instance.id)
                .copied()
                .unwrap_or_else(|| {
                    let mut state = BuildingInstanceViewState::for_definition(&instance);
                    state.inside = true;
                    state
                });
            let building_bottom =
                (instance.anchor_tile[1] + recipe.footprint[1] as i32) as f32;
            for piece in self
                .building_instance_registry
                .visible_pieces_for_instance(&instance, recipe, state)
            {
                let sort_y = if piece.piece.kind == BuildingRecipePieceKind::Roof {
                    building_bottom
                } else {
                    piece.world_tile[1] as f32 + 1.0
                };
                pieces.push((sort_y, piece));
            }
        }
        pieces.sort_by(|left, right| left.0.total_cmp(&right.0));
        for (_, piece) in pieces {
            let opacity = if piece.piece.kind == BuildingRecipePieceKind::Roof {
                0.92
            } else {
                1.0
            };
            let _ = self.editor_textures.draw_published_asset_at_tile(
                &piece.piece.asset_id,
                piece.piece.state.as_deref(),
                piece.world_tile,
                opacity,
            );
        }
    }

    fn first_building_in_current_scene(&self) -> Option<BuildingInstanceDefinition> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        self.resolved_building_instances_for_scene(scene).into_iter().next()
    }

    fn building_at_scene_cursor(&self) -> Option<BuildingInstanceDefinition> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        let tile = [self.scene_cursor_x, self.scene_cursor_y];
        self.resolved_building_instances_for_scene(scene)
            .into_iter()
            .find(|instance| {
                self.building_recipe_registry
                    .entry(&instance.recipe_id)
                    .is_some_and(|recipe| instance.footprint_contains_world_tile(recipe, tile))
            })
    }

    pub(crate) fn place_building_instance_at_scene_cursor(&mut self) -> bool {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene).cloned() else {
            return false;
        };
        let Some(recipe) = self
            .building_recipe_registry
            .entries()
            .iter()
            .find(|recipe| !recipe.classification.diagnostic_only)
            .or_else(|| self.building_recipe_registry.entries().first())
            .cloned()
        else {
            self.status_message = "No BuildingRecipe is available for placement".to_string();
            return true;
        };
        let mut definition = BuildingInstanceDefinition {
            schema: BUILDING_INSTANCE_SCHEMA.to_string(),
            id: String::new(),
            recipe_id: recipe.id.clone(),
            scene_id: scene.id.code().to_string(),
            anchor_tile: [self.scene_cursor_x, self.scene_cursor_y],
            initial_level: recipe.default_level,
            diagnostic_only: false,
            origin: BuildingInstanceOrigin::Authored,
            placement_space: BuildingPlacementSpace::SceneLocal,
            surface_region_id: None,
            global_anchor_tile: None,
        };
        if scene.kind == haven_core::SceneKind::Exterior {
            let manifest = haven_world::continuous_surface::ContinuousSurfaceManifest::for_world(&self.model.world);
            if let Some(chunk) = manifest.chunk_for_scene(&scene.id) {
                definition.placement_space = BuildingPlacementSpace::ContinuousSurface;
                definition.surface_region_id = manifest.pcg_region.clone();
                definition.global_anchor_tile = Some([
                    chunk.x * MAP_W as i32 + self.scene_cursor_x,
                    chunk.y * MAP_H as i32 + self.scene_cursor_y,
                ]);
            }
        }
        match self
            .building_instance_registry
            .place_authored_instance(definition, &self.building_recipe_registry)
        {
            Ok(id) => {
                let mut view = self
                    .building_instance_registry
                    .entry(&id)
                    .map(BuildingInstanceViewState::for_definition)
                    .unwrap_or(BuildingInstanceViewState {
                        active_level: recipe.default_level,
                        inside: true,
                    });
                view.inside = true;
                self.building_preview_views.insert(id.clone(), view);
                self.status_message = match self
                    .building_instance_registry
                    .save_authored_to_project_root(repo_root_dir())
                {
                    Ok(()) => format!(
                        "Placed and saved BuildingInstance {id} using {} at {}, {}.",
                        recipe.id, self.scene_cursor_x, self.scene_cursor_y
                    ),
                    Err(error) => format!(
                        "Placed BuildingInstance {id}, but source persistence failed: {error}"
                    ),
                };
                self.command_bus.record_event(self.app_command(
                    EditorCommandKind::SceneMutation,
                    self.status_message.clone(),
                ));
            }
            Err(error) => self.status_message = format!("Building placement failed: {error}"),
        }
        true
    }

    pub(crate) fn move_building_instance_at_scene_cursor(&mut self, dx: i32, dy: i32) -> bool {
        let Some(resolved) = self.building_at_scene_cursor() else {
            self.status_message = "No BuildingInstance footprint under the scene cursor".to_string();
            return true;
        };
        let Some(authoritative) = self.building_instance_registry.entry(&resolved.id).cloned() else {
            return false;
        };
        let local_anchor = [
            authoritative.anchor_tile[0] + dx,
            authoritative.anchor_tile[1] + dy,
        ];
        let global_anchor = authoritative
            .global_anchor_tile
            .map(|anchor| [anchor[0] + dx, anchor[1] + dy]);
        match self.building_instance_registry.move_authored_instance(
            &resolved.id,
            local_anchor,
            global_anchor,
            &self.building_recipe_registry,
        ) {
            Ok(()) => {
                self.scene_cursor_x = (self.scene_cursor_x + dx).clamp(0, MAP_W as i32 - 1);
                self.scene_cursor_y = (self.scene_cursor_y + dy).clamp(0, MAP_H as i32 - 1);
                self.status_message = match self
                    .building_instance_registry
                    .save_authored_to_project_root(repo_root_dir())
                {
                    Ok(()) => format!(
                        "Moved and saved BuildingInstance {} by {}, {}. Stable id preserved.",
                        resolved.id, dx, dy
                    ),
                    Err(error) => format!(
                        "Moved BuildingInstance {}, but source persistence failed: {error}",
                        resolved.id
                    ),
                };
                self.command_bus.record_event(self.app_command(
                    EditorCommandKind::SceneMutation,
                    self.status_message.clone(),
                ));
            }
            Err(error) => self.status_message = format!("Building move failed: {error}"),
        }
        true
    }

    pub(crate) fn delete_building_instance_at_scene_cursor(&mut self) -> bool {
        let Some(instance) = self.building_at_scene_cursor() else {
            self.status_message = "No BuildingInstance footprint under the scene cursor".to_string();
            return true;
        };
        match self
            .building_instance_registry
            .delete_authored_instance(&instance.id, &self.building_recipe_registry)
        {
            Ok(()) => {
                self.building_preview_views.remove(&instance.id);
                self.status_message = match self
                    .building_instance_registry
                    .save_authored_to_project_root(repo_root_dir())
                {
                    Ok(()) => format!(
                        "Deleted and saved authored BuildingInstance {}.",
                        instance.id
                    ),
                    Err(error) => format!(
                        "Deleted BuildingInstance {}, but source persistence failed: {error}",
                        instance.id
                    ),
                };
                self.command_bus.record_event(self.app_command(
                    EditorCommandKind::SceneMutation,
                    self.status_message.clone(),
                ));
            }
            Err(error) => self.status_message = format!("Building delete failed: {error}"),
        }
        true
    }

    pub(crate) fn cycle_building_preview_level(&mut self, delta: i32) -> bool {
        let Some(instance) = self.first_building_in_current_scene() else {
            return false;
        };
        let Some(recipe) = self.building_recipe_registry.entry(&instance.recipe_id) else {
            return false;
        };
        let mut levels = recipe.levels.iter().map(|level| level.level).collect::<Vec<_>>();
        levels.sort_unstable();
        levels.dedup();
        if levels.is_empty() {
            return false;
        }
        let state = self
            .building_preview_views
            .entry(instance.id.clone())
            .or_insert_with(|| {
                let mut state = BuildingInstanceViewState::for_definition(&instance);
                state.inside = true;
                state
            });
        let current = levels
            .iter()
            .position(|level| *level == state.active_level)
            .unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, levels.len() as i32 - 1) as usize;
        state.active_level = levels[next];
        state.inside = true;
        self.status_message = format!(
            "Building preview {}: structural level {} (PageUp/PageDown)",
            instance.id, state.active_level
        );
        true
    }

    pub(crate) fn toggle_building_preview_cutaway(&mut self) -> bool {
        let Some(instance) = self.first_building_in_current_scene() else {
            return false;
        };
        let state = self
            .building_preview_views
            .entry(instance.id.clone())
            .or_insert_with(|| BuildingInstanceViewState::for_definition(&instance));
        state.inside = !state.inside;
        self.status_message = format!(
            "Building preview {}: {} (Home toggles cutaway)",
            instance.id,
            if state.inside {
                format!("cutaway level {}", state.active_level)
            } else {
                "exterior roof view".to_string()
            }
        );
        true
    }

    pub(crate) fn building_preview_label(&self, scene: &SceneMap) -> Option<String> {
        let instance = self.resolved_building_instances_for_scene(scene).into_iter().next()?;
        let state = self
            .building_preview_views
            .get(&instance.id)
            .copied()
            .unwrap_or_else(|| BuildingInstanceViewState::for_definition(&instance));
        Some(if state.inside {
            format!(
                "Building cutaway L{} | PgUp/PgDn floor | Home exterior | Ctrl+B place | Ctrl+Alt+Arrows move | Ctrl+Shift+B delete",
                state.active_level
            )
        } else {
            "Building exterior/roof | Home cutaway | PgUp/PgDn floor | Ctrl+B place".to_string()
        })
    }
}
