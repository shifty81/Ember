use super::render_helpers::*;
use super::*;

impl EditorApp {
    pub(crate) fn draw(&mut self) {
        set_default_camera();
        gl_use_default_material();
        clear_background(editor_theme::colors::WINDOW_BG);

        let w = screen_width();
        let validation = self.model.validation_report();
        let layout = self.shell_layout();
        let graph_rect = layout.center_panel;
        let active_title = self.active_document_title();

        draw_top_bar(
            w,
            self.viewport_mode,
            &active_title,
            self.active_document_dirty(),
        );
        if self.workspace_shell.left_panel_visible {
            draw_panel(
                layout.left_panel,
                match self.viewport_mode {
                    EditorViewportMode::RegionGraph => "Project / Islands & Harbors",
                    EditorViewportMode::SceneRectangles => "Project / Alderreach & Islands",
                    EditorViewportMode::SceneBank => "Project / Off-World Scenes",
                    EditorViewportMode::SceneMap => "Project / Scene Documents",
                    EditorViewportMode::PixelStudio => "Assets / Pixel Documents",
                    EditorViewportMode::AnimationStudio => "Assets / Animation Sources",
                    EditorViewportMode::CharacterStudio => "Assets / Universal LPC",
                },
            );
        }
        if self.workspace_shell.right_panel_visible {
            draw_panel(layout.right_panel, "Inspector / Properties");
        }
        draw_panel(
            graph_rect,
            match self.viewport_mode {
                EditorViewportMode::RegionGraph => "World Routes / Harbor Network",
                EditorViewportMode::SceneRectangles => "Alderreach Global World Editor",
                EditorViewportMode::SceneBank => "Scene Bank Library",
                EditorViewportMode::SceneMap => "Playable Scene Editor",
                EditorViewportMode::PixelStudio => "Pixel Studio Canvas",
                EditorViewportMode::AnimationStudio => "Animation Timeline",
                EditorViewportMode::CharacterStudio => "Character Assembly / NPC Generation",
            },
        );

        // Reassert the native UI render state before entering a workspace.
        // Macroquad 0.4.14 does not expose QuadGl::flush; material and camera
        // ownership must therefore be normalized explicitly at each boundary.
        set_default_camera();
        gl_use_default_material();

        let list_rect = layout.list_content;
        match self.viewport_mode {
            EditorViewportMode::RegionGraph => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_landmass_list(list_rect);
                }
                self.draw_world_routes_workspace(graph_rect);
            }
            EditorViewportMode::SceneRectangles => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_landmass_list(list_rect);
                }
                self.draw_scene_rectangles(graph_rect);
            }
            EditorViewportMode::SceneBank => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_scene_bank_list(list_rect);
                }
                self.draw_scene_bank_workspace();
            }
            EditorViewportMode::SceneMap => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_scene_outliner(list_rect);
                }
                self.draw_scene_map(graph_rect);
            }
            EditorViewportMode::PixelStudio => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_pixel_library(list_rect);
                }
                self.draw_pixel_canvas(graph_rect);
            }
            EditorViewportMode::AnimationStudio => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_animation_library(list_rect);
                }
                self.draw_animation_workspace(graph_rect);
            }
            EditorViewportMode::CharacterStudio => {
                if self.workspace_shell.left_panel_visible {
                    self.draw_character_catalog(list_rect);
                }
                self.draw_character_studio_workspace(graph_rect);
            }
        }

        // Workspaces may use custom cameras/materials. Restore the shared UI
        // state before inspector and text rendering so tab switches cannot
        // leak dark/black text state into the surrounding editor chrome.
        set_default_camera();
        gl_use_default_material();
        if self.workspace_shell.right_panel_visible {
            let inspector_rect = layout.inspector_content;
            match self.viewport_mode {
                EditorViewportMode::RegionGraph => self.draw_world_routes_inspector(inspector_rect),
                EditorViewportMode::SceneMap => self.draw_scene_dock(inspector_rect),
                EditorViewportMode::SceneBank => self.draw_scene_bank_inspector(inspector_rect),
                EditorViewportMode::PixelStudio => self.draw_pixel_inspector(inspector_rect),
                EditorViewportMode::AnimationStudio => {
                    self.draw_animation_inspector(inspector_rect)
                }
                EditorViewportMode::CharacterStudio => {
                    self.draw_character_studio_inspector(inspector_rect)
                }
                _ => self.draw_inspector(inspector_rect),
            }
        }

        self.draw_workspace_bottom_dock(&layout, &validation);
        self.draw_workspace_splitters(&layout);
        self.draw_workspace_status_bar(&layout, &validation);

        set_default_camera();
        gl_use_default_material();
        if let Some(menu) = self.world_canvas_context_menu {
            world_canvas_context::draw_world_canvas_context_menu(menu);
        }
        if let Some(menu) = self.scene_asset_context_menu {
            scene_asset_context::draw(menu);
        }
        self.draw_editor_menus(w);
        self.draw_asset_drag_preview();
        if let Some(dialog) = self.pixel_studio.new_dialog.as_ref() {
            pixel_new_document::draw_new_pixel_dialog(dialog);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn draw_node_list(&self, rect: Rect) {
        let mut y = rect.y;
        draw_editor_text("Click a node to select it", rect.x, y, 18.0, MUTED);
        y += 28.0;
        for node in &self.model.region_graph.nodes {
            let active = self.selection.primary_region_node_id() == Some(&node.id);
            let row_h = 42.0;
            let bg = if active { ACCENT } else { CONTROL_BG };
            draw_rectangle(rect.x, y - 18.0, rect.w, row_h, bg);
            draw_rectangle_lines(rect.x, y - 18.0, rect.w, row_h, 1.0, PANEL_EDGE);
            draw_editor_text(&node.label, rect.x + 10.0, y + 2.0, 20.0, TEXT);
            let scene = node
                .scene_id
                .as_ref()
                .map(ProjectSceneId::label)
                .unwrap_or_else(|| "future scene".to_string());
            draw_editor_text(
                &format!("{} | {}", node.kind.code(), scene),
                rect.x + 10.0,
                y + 20.0,
                14.0,
                MUTED,
            );
            y += row_h + 8.0;
        }
    }

    pub(crate) fn draw_inspector(&self, rect: Rect) {
        let mut y = rect.y;
        match self.viewport_mode {
            EditorViewportMode::RegionGraph => {
                let Some(node) = self.selected_region_node() else {
                    draw_editor_text("No region node selected", rect.x, y, 20.0, MUTED);
                    return;
                };
                draw_editor_text(&node.label, rect.x, y, 28.0, TEXT);
                y += 34.0;
                let scene = node
                    .scene_id
                    .as_ref()
                    .map(ProjectSceneId::label)
                    .unwrap_or_else(|| "future scene".to_string());
                for line in [
                    format!("Node ID: {}", node.id),
                    format!("Kind: {}", node.kind.code()),
                    format!("Scene: {scene}"),
                    format!("Biome: {}", node.biome.label()),
                    format!("Position: {:.2}, {:.2}", node.position.x, node.position.y),
                ] {
                    draw_editor_text(&line, rect.x, y, 18.0, TEXT);
                    y += 24.0;
                }
                y += 8.0;
                draw_editor_text("Region Actions", rect.x, y, 18.0, TEXT);
                draw_editor_widget(
                    Rect::new(rect.x, 238.0, 140.0, 32.0),
                    "Add Scene Node",
                    false,
                );
                draw_editor_widget(
                    Rect::new(rect.x + 146.0, 238.0, 140.0, 32.0),
                    "Remove Node",
                    false,
                );
                y = 292.0;
            }
            EditorViewportMode::SceneRectangles => {
                self.draw_world_surface_inspector(rect);
                return;
            }
            EditorViewportMode::SceneBank => {
                self.draw_scene_bank_inspector(rect);
                return;
            }
            EditorViewportMode::SceneMap => {
                let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
                    draw_editor_text("No scene loaded", rect.x, y, 22.0, WARN);
                    return;
                };
                draw_editor_text(&scene.name, rect.x, y, 28.0, TEXT);
                y += 34.0;
                for line in [
                    format!("Scene ID: {}", scene.id.code()),
                    format!("Kind: {}", scene.kind.code()),
                    format!("Biome: {}", scene.biome.label()),
                    format!("Spawn: {}, {}", scene.spawn_x, scene.spawn_y),
                    format!("Cursor: {}, {}", self.scene_cursor_x, self.scene_cursor_y),
                    format!("Layer: {}", self.scene_layer_mode.label()),
                    format!("Tool: {}", self.scene_edit_tool.label()),
                    format!("Brush tile: {}", self.selected_tile_kind().label()),
                    format!("Brush object: {}", self.selected_object_kind().label()),
                    format!(
                        "Brush stamp: {}",
                        self.selected_stamp_id
                            .as_deref()
                            .and_then(|stamp_id| self.stamp_registry.entry(stamp_id))
                            .map(|definition| definition.label.as_str())
                            .unwrap_or("None")
                    ),
                    format!(
                        "Selected object: {}",
                        self.selected_scene_object_label(scene)
                    ),
                    format!("Brush zone: {}", self.selected_zone_kind().label()),
                    format!(
                        "Transition target: {}",
                        self.selected_transition_target_scene().label()
                    ),
                    format!(
                        "Cursor tile: {}",
                        scene
                            .map
                            .get(self.scene_cursor_x, self.scene_cursor_y)
                            .label()
                    ),
                    format!(
                        "Cursor zone: {}",
                        scene
                            .zone_at(self.scene_cursor_x, self.scene_cursor_y)
                            .label()
                    ),
                    format!(
                        "Cursor object: {}",
                        scene
                            .map
                            .object_at(self.scene_cursor_x, self.scene_cursor_y)
                            .map(|index| scene.map.objects[index].kind.label())
                            .unwrap_or("None")
                    ),
                    format!(
                        "Cursor stamp: {}",
                        scene
                            .map
                            .stamp_at(self.scene_cursor_x, self.scene_cursor_y)
                            .and_then(|index| scene.map.stamps.get(index))
                            .map(|stamp| stamp.stamp_key.as_str())
                            .unwrap_or("None")
                    ),
                    format!(
                        "Cursor transition: {}",
                        scene
                            .transition_at(self.scene_cursor_x, self.scene_cursor_y)
                            .map(|transition| transition.label.as_str())
                            .unwrap_or("None")
                    ),
                    format!("Objects: {}", scene.map.objects.len()),
                    format!("Stamps: {}", scene.map.stamps.len()),
                    format!("Transitions: {}", scene.transitions.len()),
                ] {
                    draw_editor_text(&line, rect.x, y, 18.0, TEXT);
                    y += 24.0;
                }
                y += 8.0;
                draw_editor_text("Map Preview", rect.x, y, 22.0, TEXT);
                y += 24.0;
                draw_wrapped(
                    "This view uses a clipped permanent canvas with a visible 1x1 grid. Select supports marquee and drag-move. Rectangle, Fill, Replace, Pick, Place, Erase, and Pan are explicit tools. Wheel zooms around the pointer; middle-drag or Space+drag pans. Ctrl+C/X/V/D handles clipboard and duplication, Delete removes selection, F frames the scene, and Shift+F frames selection. Every bulk action is one typed undo transaction.",
                    rect.x,
                    y,
                    rect.w,
                    16.0,
                    MUTED,
                );
                y += 58.0;
            }
            EditorViewportMode::PixelStudio => {
                draw_editor_text("Pixel Studio", rect.x, y, 24.0, TEXT);
                y += 30.0;
                draw_wrapped(
                    "Pixel Studio uses a dedicated image, grid, slice, palette, license, and publishing inspector.",
                    rect.x,
                    y,
                    rect.w,
                    16.0,
                    MUTED,
                );
                y += 58.0;
            }
            EditorViewportMode::AnimationStudio => {
                draw_editor_text("Animation Studio", rect.x, y, 24.0, TEXT);
                y += 30.0;
                draw_wrapped(
                    "Animation Studio uses dedicated source-sheet, clip, frame, event, socket, timeline, and runtime-publishing tools.",
                    rect.x,
                    y,
                    rect.w,
                    16.0,
                    MUTED,
                );
                y += 58.0;
            }
            EditorViewportMode::CharacterStudio => {
                draw_editor_text("Character Studio", rect.x, y, 24.0, TEXT);
                y += 30.0;
                draw_wrapped(
                    "Character Studio consumes the mounted Universal LPC authority for player, NPC, population, compatibility, and license-aware source selection.",
                    rect.x,
                    y,
                    rect.w,
                    16.0,
                    MUTED,
                );
                y += 58.0;
            }
        }

        y += 16.0;
        draw_editor_text("Model Summary", rect.x, y, 22.0, TEXT);
        y += 30.0;
        for line in [
            format!("World scenes: {}", self.model.world_scene_count()),
            format!("Region nodes: {}", self.model.region_graph.nodes.len()),
            format!("Region links: {}", self.model.region_graph.links.len()),
            format!("Linked scenes: {}", self.model.linked_region_scene_count()),
        ] {
            draw_editor_text(&line, rect.x, y, 18.0, TEXT);
            y += 24.0;
        }

        y += 16.0;
        draw_editor_text("Project Shell", rect.x, y, 22.0, TEXT);
        y += 30.0;
        for line in self.model.project_summary() {
            draw_editor_text(&line, rect.x, y, 18.0, TEXT);
            y += 24.0;
        }

        y += 10.0;
        draw_editor_text("System Ownership", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for system in editor_system_audit().into_iter().take(5) {
            let color = system_status_color(system.status);
            draw_editor_text(
                &format!("{}: {}", system.name, system.status.label()),
                rect.x,
                y,
                17.0,
                color,
            );
            y += 22.0;
        }

        y += 10.0;
        draw_editor_text("Editor Workspaces", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for workspace in editor_workspace_titles().into_iter().take(5) {
            draw_editor_text(workspace, rect.x, y, 16.0, MUTED);
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Scene Rectangle Contract", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for line in scene_rectangle_contract_summary() {
            draw_editor_text(&line, rect.x, y, 16.0, MUTED);
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Generated Assets", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for line in generated_asset_registry_summary() {
            draw_editor_text(&line, rect.x, y, 16.0, MUTED);
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Animation Contract", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for line in animation_contract_summary() {
            draw_editor_text(&line, rect.x, y, 16.0, MUTED);
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Validation Registry", rect.x, y, 22.0, TEXT);
        y += 28.0;
        for entry in validation_registry().into_iter().take(4) {
            draw_editor_text(
                &format!("{}: {}", entry.title, entry.status.label()),
                rect.x,
                y,
                16.0,
                MUTED,
            );
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Command History", rect.x, y, 22.0, TEXT);
        y += 24.0;
        draw_editor_text(
            &format!(
                "Undo {} (typed {}) | Redo {} | active ops {}",
                self.command_bus.undo_len(),
                self.command_bus.typed_undo_len(),
                self.command_bus.redo_len(),
                self.command_bus.active_operation_count()
            ),
            rect.x,
            y,
            15.0,
            MUTED,
        );
        y += 22.0;
        for command in self.command_bus.recent_commands().iter().rev().take(4) {
            draw_editor_text(command.label(), rect.x, y, 16.0, MUTED);
            y += 20.0;
        }

        y += 10.0;
        draw_editor_text("Scope", rect.x, y, 22.0, TEXT);
        y += 28.0;
        draw_wrapped(
            haven_editor::editor_world_scope_note(),
            rect.x,
            y,
            rect.w,
            16.0,
            MUTED,
        );
    }
}

include!("draw_scene_views.rs");
