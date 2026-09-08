use super::*;

impl EditorApp {
    pub(crate) fn select_world_cell(&mut self, global: GridPos) -> bool {
        let Some(manifest) = &self.scene_rectangles else {
            return false;
        };
        for (index, rectangle) in manifest.scene_rectangles.iter().enumerate().rev() {
            if !rectangle_is_overworld_surface(rectangle)
                || rectangle.landmass_id != self.selected_landmass_id
            {
                continue;
            }
            let partition_rect = world_scene_grid_rect(manifest, rectangle);
            if !partition_rect.contains(vec2(global.x as f32 + 0.5, global.y as f32 + 0.5)) {
                continue;
            }
            let rectangle_id = rectangle.scene_id.clone();
            let local_x = global.x - partition_rect.x.floor() as i32;
            let local_y = global.y - partition_rect.y.floor() as i32;
            self.selected_rectangle = index;
            self.world_cursor_x = global.x;
            self.world_cursor_y = global.y;
            self.sync_assignment_cycles_to_selected_rectangle();
            let selected_scene_id = self
                .scene_assignments
                .assignment_for_rectangle(&rectangle_id)
                .and_then(|assignment| {
                    self.model
                        .world
                        .scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))
                        .map(|scene| scene.id.clone())
                });
            if let Some(scene_id) = selected_scene_id {
                self.selection.replace(
                    scene_id,
                    SelectionItem::Tile(GridPos {
                        x: local_x,
                        y: local_y,
                    }),
                );
            } else {
                self.selection.clear();
            }
            self.status_message = format!(
                "Global tile {}, {} | partition {} | local {}, {}",
                global.x, global.y, rectangle_id, local_x, local_y
            );
            return true;
        }
        false
    }

    pub(crate) fn handle_scene_toolrail_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }

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
            if scene_tool_button_rect(rect, index).contains(mouse) {
                self.set_scene_edit_tool(tool);
                return true;
            }
        }

        for (index, layer) in [
            SceneLayerMode::Terrain,
            SceneLayerMode::Objects,
            SceneLayerMode::Zones,
            SceneLayerMode::Transitions,
        ]
        .into_iter()
        .enumerate()
        {
            if scene_layer_visibility_button_rect(rect, index).contains(mouse) {
                self.toggle_layer_visibility(layer);
                return true;
            }
            if scene_layer_lock_button_rect(rect, index).contains(mouse) {
                self.toggle_layer_lock(layer);
                return true;
            }
            if scene_layer_button_rect(rect, index).contains(mouse) {
                self.set_scene_layer_mode(layer);
                return true;
            }
        }
        if scene_layer_opacity_down_rect(rect).contains(mouse) {
            self.adjust_active_layer_opacity(-0.1);
            return true;
        }
        if scene_layer_opacity_up_rect(rect).contains(mouse) {
            self.adjust_active_layer_opacity(0.1);
            return true;
        }
        if self.scene_layer_mode == SceneLayerMode::Terrain {
            for (index, mode) in TerrainPaintMode::ALL.into_iter().enumerate() {
                if scene_terrain_paint_mode_button_rect(rect, index).contains(mouse) {
                    self.terrain_paint_mode = mode;
                    self.status_message =
                        format!("Terrain paint mode: {} — {}", mode.label(), mode.help());
                    return true;
                }
            }
        }

        let count = match self.scene_layer_mode {
            SceneLayerMode::Terrain => TileKind::LPC_MAPPED_EDITOR_TERRAIN.len(),
            SceneLayerMode::Objects => OBJECT_BRUSHES.len(),
            SceneLayerMode::Zones => ZONE_BRUSHES.len(),
            SceneLayerMode::Transitions => self.model.world.scenes.len(),
        };
        let selected_index = match self.scene_layer_mode {
            SceneLayerMode::Terrain => TileKind::LPC_MAPPED_EDITOR_TERRAIN
                .iter()
                .position(|tile| *tile == self.selected_tile_kind())
                .unwrap_or(0),
            SceneLayerMode::Objects => self.selected_object,
            SceneLayerMode::Zones => self.selected_zone,
            SceneLayerMode::Transitions => self.selected_transition_target,
        };
        let page_start = scene_content_page_start(rect, count, selected_index);
        let capacity = scene_content_capacity(rect);
        for slot in 0..capacity {
            let index = page_start + slot;
            if index >= count {
                break;
            }
            if scene_content_button_rect(rect, slot).contains(mouse) {
                match self.scene_layer_mode {
                    SceneLayerMode::Terrain => {
                        let tile = TileKind::LPC_MAPPED_EDITOR_TERRAIN[index];
                        if let Some(all_index) = TileKind::ALL
                            .iter()
                            .position(|candidate| *candidate == tile)
                        {
                            self.selected_tile = all_index;
                        }
                        self.set_scene_edit_tool(SceneEditTool::Paint);
                        self.status_message = format!(
                            "Semantic terrain brush: {} [{}]. Click or drag on the snapped tile grid.",
                            self.selected_tile_kind().label(),
                            haven_world::terrain_standard_code(self.selected_tile_kind())
                        );
                    }
                    SceneLayerMode::Objects => {
                        self.selected_stamp_id = None;
                        self.selected_object = index;
                        self.set_scene_edit_tool(SceneEditTool::Place);
                        self.status_message = format!(
                            "Place tool: {}. Click a grid cell to place it.",
                            self.selected_object_kind().label()
                        );
                    }
                    SceneLayerMode::Zones => {
                        self.selected_zone = index;
                        self.set_scene_edit_tool(SceneEditTool::Paint);
                        self.status_message = format!(
                            "Zone brush: {}. Click or drag on the snapped tile grid.",
                            self.selected_zone_kind().label()
                        );
                    }
                    SceneLayerMode::Transitions => {
                        self.selected_transition_target = index;
                        self.set_scene_edit_tool(SceneEditTool::Place);
                        self.status_message = format!(
                            "Transition target: {}. Click a grid cell to place it.",
                            self.selected_transition_target_scene().label()
                        );
                    }
                }
                return true;
            }
        }

        if scene_apply_button_rect(rect).contains(mouse) {
            self.apply_scene_edit_tool();
            return true;
        }
        if scene_erase_button_rect(rect).contains(mouse) {
            self.set_scene_edit_tool(SceneEditTool::Erase);
            self.apply_scene_edit_tool();
            return true;
        }
        true
    }

    pub(crate) fn main_viewport_rect(&self) -> Rect {
        self.shell_layout().center_panel
    }

    pub(crate) fn inspector_content_rect(&self) -> Rect {
        self.shell_layout().inspector_content
    }

    pub(crate) fn canvas_host_rect(&self) -> Rect {
        let rect = self.main_viewport_rect();
        Rect::new(rect.x + 16.0, rect.y + 42.0, rect.w - 32.0, rect.h - 58.0)
    }

    pub(crate) fn scene_canvas_viewport_rect(&self) -> Rect {
        let host = self.canvas_host_rect();
        Rect::new(
            host.x + 30.0,
            host.y + 98.0,
            (host.w - 30.0).max(1.0),
            (host.h - 98.0).max(1.0),
        )
    }

    pub(crate) fn world_canvas_viewport_rect(&self) -> Rect {
        let host = self.canvas_host_rect();
        Rect::new(
            host.x + 30.0,
            host.y + 62.0,
            (host.w - 30.0).max(1.0),
            (host.h - 62.0).max(1.0),
        )
    }

    pub(crate) fn scene_canvas_bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, MAP_W as f32, MAP_H as f32)
    }

    pub(crate) fn world_canvas_bounds(&self) -> Option<Rect> {
        let manifest = self.scene_rectangles.as_ref()?;
        world_scene_grid_bounds_for_landmass(manifest, self.selected_landmass_id)
    }

    pub(crate) fn update_canvas_navigation(&mut self) -> bool {
        let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if control && is_key_pressed(KeyCode::Equal) {
            match self.viewport_mode {
                EditorViewportMode::SceneMap => self.scene_canvas.zoom_in_step(),
                EditorViewportMode::SceneRectangles => self.world_canvas.zoom_in_step(),
                EditorViewportMode::SceneBank => self.scene_bank_canvas.zoom_in_step(),
                EditorViewportMode::PixelStudio => self.pixel_studio.zoom_in(),
                _ => {}
            }
        }
        if control && is_key_pressed(KeyCode::Minus) {
            match self.viewport_mode {
                EditorViewportMode::SceneMap => self.scene_canvas.zoom_out_step(),
                EditorViewportMode::SceneRectangles => self.world_canvas.zoom_out_step(),
                EditorViewportMode::SceneBank => self.scene_bank_canvas.zoom_out_step(),
                EditorViewportMode::PixelStudio => self.pixel_studio.zoom_out(),
                _ => {}
            }
        }
        if control && is_key_pressed(KeyCode::Key0) {
            match self.viewport_mode {
                EditorViewportMode::SceneMap => self.scene_canvas.actual_size(),
                EditorViewportMode::SceneRectangles => self.world_canvas.actual_size(),
                EditorViewportMode::SceneBank => self.scene_bank_canvas.actual_size(),
                EditorViewportMode::PixelStudio => {
                    self.pixel_studio.frame_document(self.pixel_canvas_rect())
                }
                _ => {}
            }
        }
        if is_key_pressed(KeyCode::F)
            && !is_key_down(KeyCode::LeftShift)
            && !is_key_down(KeyCode::RightShift)
        {
            match self.viewport_mode {
                EditorViewportMode::SceneMap => {
                    self.scene_canvas.reset();
                    self.status_message = "Framed active scene canvas".to_string();
                }
                EditorViewportMode::SceneRectangles => {
                    self.world_canvas.reset();
                    self.status_message = "Framed global world canvas".to_string();
                }
                EditorViewportMode::SceneBank => {
                    self.scene_bank_canvas.reset();
                    self.status_message = "Framed Scene Bank library".to_string();
                }
                EditorViewportMode::RegionGraph => {}
                EditorViewportMode::PixelStudio => {
                    self.pixel_studio.frame_document(self.pixel_canvas_rect());
                    self.status_message = "Framed active pixel document".to_string();
                }
                EditorViewportMode::AnimationStudio => {}
                EditorViewportMode::CharacterStudio => {}
            }
        }

        let pointer = vec2(mouse_position().0, mouse_position().1);
        let shell_layout = self.shell_layout();
        if self.workspace_shell.bottom_dock_open && shell_layout.bottom_dock.contains(pointer) {
            return true;
        }

        match self.viewport_mode {
            EditorViewportMode::SceneMap => {
                let viewport = self.scene_canvas_viewport_rect();
                let bounds = self.scene_canvas_bounds();
                let consumed = self.scene_canvas.handle_navigation(
                    viewport,
                    bounds,
                    self.scene_edit_tool == SceneEditTool::Pan,
                );
                if consumed && mouse_wheel().1.abs() > f32::EPSILON {
                    self.status_message =
                        format!("Scene canvas zoom {:.0}%", self.scene_canvas.zoom_percent());
                }
                consumed
            }
            EditorViewportMode::SceneRectangles => {
                let Some(bounds) = self.world_canvas_bounds() else {
                    return false;
                };
                let viewport = self.world_canvas_viewport_rect();
                let consumed = self.world_canvas.handle_navigation(
                    viewport,
                    bounds,
                    self.world_canvas_pan_tool,
                );
                if consumed && mouse_wheel().1.abs() > f32::EPSILON {
                    self.status_message =
                        format!("World canvas zoom {:.0}%", self.world_canvas.zoom_percent());
                }
                consumed
            }
            EditorViewportMode::SceneBank => {
                let viewport = self.scene_bank_viewport_rect();
                let bounds = scene_bank_workspace::scene_bank_bounds(&self.model.world);
                let consumed = self.scene_bank_canvas.handle_navigation(
                    viewport,
                    bounds,
                    self.scene_bank_pan_tool,
                );
                if consumed && mouse_wheel().1.abs() > f32::EPSILON {
                    self.status_message = format!(
                        "Scene Bank zoom {:.0}%",
                        self.scene_bank_canvas.zoom_percent()
                    );
                }
                consumed
            }
            EditorViewportMode::RegionGraph => false,
            EditorViewportMode::PixelStudio => self.update_pixel_studio_navigation(),
            EditorViewportMode::AnimationStudio => false,
            EditorViewportMode::CharacterStudio => false,
        }
    }

    pub(crate) fn handle_canvas_toolbar_click(&mut self, mx: f32, my: f32) -> bool {
        if !matches!(
            self.viewport_mode,
            EditorViewportMode::SceneMap
                | EditorViewportMode::SceneRectangles
                | EditorViewportMode::SceneBank
        ) {
            return false;
        }
        let host = self.canvas_host_rect();
        if self.handle_autotile_toolbar_click(mx, my) {
            return true;
        }
        if !Rect::new(host.x, host.y, host.w, 36.0).contains(vec2(mx, my)) {
            return false;
        }

        if self.viewport_mode == EditorViewportMode::SceneMap {
            let buttons = [
                (canvas_toolbar_button_rect(host, 0, 82.0), 0),
                (canvas_toolbar_button_rect(host, 1, 82.0), 1),
                (canvas_toolbar_button_rect(host, 2, 42.0), 2),
                (canvas_toolbar_button_rect(host, 3, 42.0), 3),
            ];
            for (button, action) in buttons {
                if button.contains(vec2(mx, my)) {
                    match action {
                        0 => {
                            self.scene_canvas.reset();
                            self.status_message = "Framed active scene canvas".to_string();
                        }
                        1 => self.frame_current_selection(),
                        2 => {
                            self.scene_canvas.zoom_out_step();
                            self.status_message = format!(
                                "Scene canvas zoom {:.0}%",
                                self.scene_canvas.zoom_percent()
                            );
                        }
                        3 => {
                            self.scene_canvas.zoom_in_step();
                            self.status_message = format!(
                                "Scene canvas zoom {:.0}%",
                                self.scene_canvas.zoom_percent()
                            );
                        }
                        _ => {}
                    }
                    return true;
                }
            }
        } else if self.viewport_mode == EditorViewportMode::SceneRectangles {
            let buttons = [
                (canvas_toolbar_button_rect(host, 0, 70.0), 0),
                (canvas_toolbar_button_rect(host, 1, 70.0), 1),
                (canvas_toolbar_button_rect(host, 2, 82.0), 2),
                (canvas_toolbar_button_rect(host, 3, 42.0), 3),
                (canvas_toolbar_button_rect(host, 4, 42.0), 4),
                (canvas_toolbar_button_rect(host, 5, 82.0), 5),
                (canvas_toolbar_button_rect(host, 6, 76.0), 6),
                (canvas_toolbar_button_rect(host, 7, 70.0), 7),
                (canvas_toolbar_button_rect(host, 8, 86.0), 8),
            ];
            for (button, action) in buttons {
                if !button.contains(vec2(mx, my)) {
                    continue;
                }
                match action {
                    0 => self.set_world_edit_tool(WorldEditTool::Select),
                    1 => self.set_world_edit_tool(WorldEditTool::Pan),
                    2 => self.world_canvas.reset(),
                    3 => self.world_canvas.zoom_out_step(),
                    4 => self.world_canvas.zoom_in_step(),
                    5 => self.world_show_partitions = !self.world_show_partitions,
                    6 => self.world_show_objects = !self.world_show_objects,
                    7 => self.world_show_zones = !self.world_show_zones,
                    8 => self.world_show_structural_levels = !self.world_show_structural_levels,
                    _ => {}
                }
                self.status_message = format!(
                    "World editor {} | zoom {:.0}% | partitions {} | objects {} | zones {} | structural levels {}",
                    self.world_edit_tool.label(),
                    self.world_canvas.zoom_percent(),
                    if self.world_show_partitions {
                        "on"
                    } else {
                        "off"
                    },
                    if self.world_show_objects {
                        "on"
                    } else {
                        "off"
                    },
                    if self.world_show_zones { "on" } else { "off" },
                    if self.world_show_structural_levels {
                        "on"
                    } else {
                        "off"
                    },
                );
                return true;
            }
        } else {
            let buttons = [
                (canvas_toolbar_button_rect(host, 0, 70.0), 0),
                (canvas_toolbar_button_rect(host, 1, 70.0), 1),
                (canvas_toolbar_button_rect(host, 2, 82.0), 2),
                (canvas_toolbar_button_rect(host, 3, 42.0), 3),
                (canvas_toolbar_button_rect(host, 4, 42.0), 4),
            ];
            for (button, action) in buttons {
                if !button.contains(vec2(mx, my)) {
                    continue;
                }
                match action {
                    0 => self.scene_bank_pan_tool = false,
                    1 => self.scene_bank_pan_tool = true,
                    2 => self.scene_bank_canvas.reset(),
                    3 => self.scene_bank_canvas.zoom_out_step(),
                    4 => self.scene_bank_canvas.zoom_in_step(),
                    _ => {}
                }
                self.status_message = format!(
                    "Scene Bank {} | zoom {:.0}%",
                    if self.scene_bank_pan_tool {
                        "Pan"
                    } else {
                        "Select"
                    },
                    self.scene_bank_canvas.zoom_percent()
                );
                return true;
            }
        }
        true
    }
}
