use super::render_helpers::*;
use super::*;

impl EditorApp {
    pub(crate) fn update(&mut self) {
        self.poll_development_client();
        self.update_asset_palette_drag();

        // Active Pixel Studio modals own input before canvas navigation.
        // The canvas navigation path reports the pointer as consumed while the
        // modal is visible, so routing this later made the dialog unclickable.
        if self.viewport_mode == EditorViewportMode::PixelStudio
            && self.pixel_studio.new_dialog.is_some()
        {
            if is_mouse_button_pressed(MouseButton::Left) {
                let mouse = vec2(mouse_position().0, mouse_position().1);
                self.handle_new_pixel_dialog_click(mouse);
            }
            self.handle_new_pixel_dialog_input();
            return;
        }
        if self.update_workspace_resize_input() {
            return;
        }

        // UI pointer ownership is gesture-scoped, not frame-scoped. Without this
        // latch, a context-menu press can be consumed on its first frame and the
        // still-held button can fall through to Scene Map painting on the next.
        if self.primary_pointer_owned_by_ui {
            if is_mouse_button_released(MouseButton::Left) {
                self.primary_pointer_owned_by_ui = false;
            }
            return;
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            if self.open_scene_asset_context_menu() {
                return;
            }
            if self.open_world_canvas_context_menu() {
                return;
            }
            self.world_canvas_context_menu = None;
        }
        if self.text_focus != EditorTextFocus::None {
            if is_mouse_button_pressed(MouseButton::Left) {
                let consumed = self.handle_primary_click();
                if !consumed {
                    self.text_focus = EditorTextFocus::None;
                }
            }
            self.handle_text_input();
            return;
        }
        if self.handle_workspace_shell_shortcuts() {
            return;
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();

            // Popups are top-most input owners. Whether the click chooses an
            // action or merely dismisses the popup, it must never reach canvas
            // authoring beneath it.
            if self.scene_asset_context_menu.is_some() {
                let _ = self.handle_scene_asset_context_click(mx, my);
                self.primary_pointer_owned_by_ui = true;
                return;
            }
            if self.world_canvas_context_menu.is_some() {
                let _ = self.handle_world_canvas_context_click(mx, my);
                self.primary_pointer_owned_by_ui = true;
                return;
            }

            if self.handle_workspace_chrome_click(mx, my) {
                self.primary_pointer_owned_by_ui = true;
                return;
            }
        }

        let canvas_pointer_consumed = self.update_canvas_navigation();
        let pointer_consumed =
            if is_mouse_button_pressed(MouseButton::Left) && !canvas_pointer_consumed {
                self.handle_primary_click()
            } else {
                false
            };
        if is_key_pressed(KeyCode::Tab) && self.pixel_studio.new_dialog.is_none() {
            let _ = self.command_bus.commit_gesture();
            self.last_painted_cell = None;
            self.viewport_mode = match self.viewport_mode {
                EditorViewportMode::RegionGraph => EditorViewportMode::SceneRectangles,
                EditorViewportMode::SceneRectangles => EditorViewportMode::SceneBank,
                EditorViewportMode::SceneBank => EditorViewportMode::SceneMap,
                EditorViewportMode::SceneMap => EditorViewportMode::PixelStudio,
                EditorViewportMode::PixelStudio => EditorViewportMode::AnimationStudio,
                EditorViewportMode::AnimationStudio => EditorViewportMode::CharacterStudio,
                EditorViewportMode::CharacterStudio => EditorViewportMode::RegionGraph,
            };
            self.status_message = format!("Viewport mode: {}", self.viewport_mode.label());
            self.command_bus.record_event(self.app_command(
                EditorCommandKind::SceneMutation,
                self.status_message.clone(),
            ));
        }
        let control_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if (control_down && is_key_pressed(KeyCode::S)) || is_key_pressed(KeyCode::F5) {
            self.save_all_editor_documents();
        }
        if control_down && is_key_pressed(KeyCode::L) {
            self.reload_all_editor_documents();
        }

        match self.viewport_mode {
            EditorViewportMode::RegionGraph => {
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Right) {
                    self.cycle_landmass_selection(1);
                }
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Left) {
                    self.cycle_landmass_selection(-1);
                }
                if is_key_pressed(KeyCode::Enter) {
                    self.viewport_mode = EditorViewportMode::SceneRectangles;
                    self.frame_selected_landmass();
                }
            }
            EditorViewportMode::SceneRectangles => {
                let shift_down =
                    is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                // Storage-partition manipulation is diagnostic-only. It cannot
                // steal ordinary world-authoring keys from the global canvas.
                if self.world_show_partitions && control_down && shift_down {
                    if is_key_pressed(KeyCode::Left) {
                        self.move_selected_scene_cell(-1, 0);
                    }
                    if is_key_pressed(KeyCode::Right) {
                        self.move_selected_scene_cell(1, 0);
                    }
                    if is_key_pressed(KeyCode::Up) {
                        self.move_selected_scene_cell(0, -1);
                    }
                    if is_key_pressed(KeyCode::Down) {
                        self.move_selected_scene_cell(0, 1);
                    }
                    if is_key_pressed(KeyCode::Backspace) {
                        self.clear_selected_rectangle_assignment();
                    }
                }
                if is_key_pressed(KeyCode::Enter) {
                    self.open_assigned_rectangle_scene();
                }
                self.update_world_editor_input(pointer_consumed, canvas_pointer_consumed);
            }
            EditorViewportMode::SceneBank => {
                if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::Right) {
                    self.cycle_scene_bank_selection(1);
                }
                if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Left) {
                    self.cycle_scene_bank_selection(-1);
                }
                if is_key_pressed(KeyCode::Enter) {
                    self.open_selected_scene_bank_scene();
                }
            }
            EditorViewportMode::SceneMap => {
                let shift_down =
                    is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
                let mut building_input_consumed = false;

                if is_key_pressed(KeyCode::PageUp) {
                    building_input_consumed |= self.cycle_building_preview_level(1);
                }
                if is_key_pressed(KeyCode::PageDown) {
                    building_input_consumed |= self.cycle_building_preview_level(-1);
                }
                if is_key_pressed(KeyCode::Home) {
                    building_input_consumed |= self.toggle_building_preview_cutaway();
                }
                if control_down && shift_down && is_key_pressed(KeyCode::B) {
                    building_input_consumed |= self.delete_building_instance_at_scene_cursor();
                } else if control_down && !alt_down && is_key_pressed(KeyCode::B) {
                    building_input_consumed |= self.place_building_instance_at_scene_cursor();
                }
                if control_down && alt_down {
                    if is_key_pressed(KeyCode::Right) {
                        building_input_consumed |= self.move_building_instance_at_scene_cursor(1, 0);
                    }
                    if is_key_pressed(KeyCode::Left) {
                        building_input_consumed |= self.move_building_instance_at_scene_cursor(-1, 0);
                    }
                    if is_key_pressed(KeyCode::Down) {
                        building_input_consumed |= self.move_building_instance_at_scene_cursor(0, 1);
                    }
                    if is_key_pressed(KeyCode::Up) {
                        building_input_consumed |= self.move_building_instance_at_scene_cursor(0, -1);
                    }
                }
                if !building_input_consumed {
                    self.update_scene_map_input(pointer_consumed, canvas_pointer_consumed);
                }
            }
            EditorViewportMode::PixelStudio => {
                self.update_pixel_studio_input();
            }
            EditorViewportMode::AnimationStudio => {
                self.update_animation_studio_input();
            }
            EditorViewportMode::CharacterStudio => {
                self.update_character_studio_input();
            }
        }
    }

    pub(crate) fn handle_primary_click(&mut self) -> bool {
        let (mx, my) = mouse_position();

        if self.viewport_mode == EditorViewportMode::PixelStudio
            && self.pixel_studio.new_dialog.is_some()
        {
            return self.handle_pixel_studio_click(mx, my);
        }

        if self.handle_editor_menu_click(mx, my) {
            return true;
        }
        if self.handle_workspace_chrome_click(mx, my) {
            return true;
        }

        if self.scene_asset_context_menu.is_some() && self.handle_scene_asset_context_click(mx, my)
        {
            return true;
        }

        if self.world_canvas_context_menu.is_some()
            && self.handle_world_canvas_context_click(mx, my)
        {
            return true;
        }

        // The seven workspace buttons are deliberately persistent. The editor
        // should never require users to discover the Tab shortcut first.
        for (index, mode) in [
            EditorViewportMode::RegionGraph,
            EditorViewportMode::SceneRectangles,
            EditorViewportMode::SceneBank,
            EditorViewportMode::SceneMap,
            EditorViewportMode::PixelStudio,
            EditorViewportMode::AnimationStudio,
            EditorViewportMode::CharacterStudio,
        ]
        .into_iter()
        .enumerate()
        {
            let button = workspace_tab_rect(index);
            if button.contains(vec2(mx, my)) {
                let _ = self.command_bus.commit_gesture();
                self.last_painted_cell = None;
                self.text_focus = EditorTextFocus::None;
                self.scene_name_edit = None;
                self.scene_delete_armed = None;
                self.world_canvas_context_menu = None;
                self.viewport_mode = mode;
                if mode == EditorViewportMode::RegionGraph
                    && self.selected_region_node_index().is_none()
                {
                    self.select_region_node_index(1);
                }
                self.status_message = format!("Opened {} workspace", mode.label());
                return true;
            }
        }

        let list_rect = self.shell_layout().list_content;
        if self.workspace_shell.left_panel_visible
            && self.viewport_mode == EditorViewportMode::SceneMap
            && list_rect.contains(vec2(mx, my))
        {
            return self.handle_scene_outliner_click(mx, my, list_rect);
        }
        if self.workspace_shell.left_panel_visible
            && self.viewport_mode == EditorViewportMode::SceneBank
            && list_rect.contains(vec2(mx, my))
        {
            return self.handle_scene_bank_list_click(mx, my, list_rect);
        }
        if self.workspace_shell.left_panel_visible
            && matches!(
                self.viewport_mode,
                EditorViewportMode::RegionGraph | EditorViewportMode::SceneRectangles
            )
            && list_rect.contains(vec2(mx, my))
        {
            return self.handle_landmass_list_click(mx, my, list_rect);
        }
        if self.viewport_mode == EditorViewportMode::PixelStudio {
            return self.handle_pixel_studio_click(mx, my);
        }
        if self.viewport_mode == EditorViewportMode::AnimationStudio {
            return self.handle_animation_studio_click(mx, my);
        }
        if self.viewport_mode == EditorViewportMode::CharacterStudio {
            return self.handle_character_studio_click(mx, my);
        }

        self.text_focus = EditorTextFocus::None;
        if self.handle_canvas_toolbar_click(mx, my) {
            return true;
        }
        if self.workspace_shell.right_panel_visible
            && self.viewport_mode == EditorViewportMode::SceneMap
        {
            let inspector_rect = self.inspector_content_rect();
            if self.handle_scene_dock_click(mx, my, inspector_rect) {
                return true;
            }
        }
        if self.workspace_shell.right_panel_visible
            && self.viewport_mode == EditorViewportMode::SceneBank
        {
            if self.handle_scene_bank_inspector_click(mx, my) {
                return true;
            }
            if self.handle_scene_bank_canvas_click(mx, my) {
                return true;
            }
        }
        if self.viewport_mode == EditorViewportMode::SceneRectangles
            && self.workspace_shell.right_panel_visible
            && self.handle_scene_rectangle_inspector_click(mx, my)
        {
            return true;
        }
        // Global canvas authoring is routed after chrome/inspector input so
        // paint drags, marquees, and cross-partition gestures own the press.
        if self.viewport_mode == EditorViewportMode::RegionGraph {
            if self.workspace_shell.right_panel_visible
                && self.handle_world_routes_inspector_click(mx, my)
            {
                return true;
            }
            if self.handle_world_routes_click(mx, my) {
                return true;
            }
        }
        false
    }

    pub(crate) fn handle_scene_rectangle_inspector_click(&mut self, mx: f32, my: f32) -> bool {
        if self.viewport_mode != EditorViewportMode::SceneRectangles {
            return false;
        }
        let rect = self.inspector_content_rect();
        self.handle_world_authoring_inspector_click(mx, my, rect)
    }
}
