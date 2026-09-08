use std::fs::create_dir_all;
use std::path::Path;

use haven_assets::asset_intake::repo_root_dir;
use image::{Rgba, RgbaImage};

use super::render_helpers::{draw_editor_widget, draw_editor_widget_tone, WidgetTone};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorMenuKind {
    File,
    Edit,
    View,
    Help,
}

impl EditorMenuKind {
    fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::View => "View",
            Self::Help => "Help",
        }
    }

    fn items(self) -> &'static [&'static str] {
        match self {
            Self::File => &[
                "Save All",
                "Reload Saved",
                "Regenerate Current Seed",
                "Reroll Archipelago Seed",
                "Export Island PNGs",
                "Play Development World",
                "Play From Here",
                "Stop Development Client",
            ],
            Self::Edit => &["Undo", "Redo"],
            Self::View => &[
                "World Routes",
                "World Editor",
                "Scene Bank",
                "Scene Editor",
                "Pixel Studio",
                "Animation Studio",
                "Character Studio",
                "Toggle Project Panel",
                "Toggle Inspector",
                "Toggle Bottom Panels",
                "Open Validation Panel",
                "Reset Workspace Layout",
            ],
            Self::Help => &["Editor Shortcuts", "World Workflow"],
        }
    }
}

pub(crate) fn menu_item_rect(index: usize) -> Rect {
    Rect::new(8.0 + index as f32 * 48.0, 7.0, 44.0, 30.0)
}

pub(crate) fn save_all_button_rect(width: f32) -> Rect {
    Rect::new(width - 108.0, 7.0, 98.0, 30.0)
}

fn development_play_rect(width: f32) -> Rect { Rect::new(width - 456.0, 7.0, 86.0, 30.0) }
fn development_here_rect(width: f32) -> Rect { Rect::new(width - 366.0, 7.0, 112.0, 30.0) }
fn development_stop_rect(width: f32) -> Rect { Rect::new(width - 250.0, 7.0, 72.0, 30.0) }
fn development_restart_rect(width: f32) -> Rect { Rect::new(width - 174.0, 7.0, 62.0, 30.0) }

fn dropdown_rect(kind: EditorMenuKind) -> Rect {
    let index = match kind {
        EditorMenuKind::File => 0,
        EditorMenuKind::Edit => 1,
        EditorMenuKind::View => 2,
        EditorMenuKind::Help => 3,
    };
    let origin = menu_item_rect(index);
    Rect::new(
        origin.x,
        39.0,
        210.0,
        kind.items().len() as f32 * 30.0 + 8.0,
    )
}

impl EditorApp {
    pub(crate) fn draw_editor_menus(&self, width: f32) {
        for (index, kind) in [
            EditorMenuKind::File,
            EditorMenuKind::Edit,
            EditorMenuKind::View,
            EditorMenuKind::Help,
        ]
        .into_iter()
        .enumerate()
        {
            draw_editor_widget(
                menu_item_rect(index),
                kind.label(),
                self.open_menu == Some(kind),
            );
        }
        let dev_running = self.development_client.is_some();
        draw_editor_widget_tone(development_play_rect(width), "Play", dev_running, WidgetTone::Primary);
        draw_editor_widget_tone(development_here_rect(width), "Play From Here", false, WidgetTone::Quiet);
        draw_editor_widget_tone(development_stop_rect(width), "Stop", false, WidgetTone::Quiet);
        draw_editor_widget_tone(development_restart_rect(width), "Restart", false, WidgetTone::Quiet);
        draw_editor_widget_tone(
            save_all_button_rect(width),
            "Save All",
            false,
            WidgetTone::Primary,
        );

        let Some(kind) = self.open_menu else {
            return;
        };
        let dropdown = dropdown_rect(kind);
        draw_rectangle(
            dropdown.x,
            dropdown.y,
            dropdown.w,
            dropdown.h,
            Color::new(0.035, 0.045, 0.058, 0.99),
        );
        draw_rectangle_lines(
            dropdown.x, dropdown.y, dropdown.w, dropdown.h, 1.0, PANEL_EDGE,
        );
        for (index, label) in kind.items().iter().enumerate() {
            let row = Rect::new(
                dropdown.x + 4.0,
                dropdown.y + 4.0 + index as f32 * 30.0,
                dropdown.w - 8.0,
                28.0,
            );
            draw_editor_widget_tone(row, label, false, WidgetTone::Quiet);
        }
    }

    pub(crate) fn handle_editor_menu_click(&mut self, mx: f32, my: f32) -> bool {
        let point = vec2(mx, my);
        let width = screen_width();
        if development_play_rect(width).contains(point) { self.open_menu=None; self.play_development_world(false); return true; }
        if development_here_rect(width).contains(point) { self.open_menu=None; self.play_development_world(true); return true; }
        if development_stop_rect(width).contains(point) { self.open_menu=None; self.stop_development_client(); return true; }
        if development_restart_rect(width).contains(point) {
            self.open_menu=None;
            self.stop_development_client();
            self.play_development_world(false);
            return true;
        }
        if save_all_button_rect(width).contains(point) {
            self.open_menu = None;
            self.save_all_editor_documents();
            return true;
        }
        for (index, kind) in [
            EditorMenuKind::File,
            EditorMenuKind::Edit,
            EditorMenuKind::View,
            EditorMenuKind::Help,
        ]
        .into_iter()
        .enumerate()
        {
            if menu_item_rect(index).contains(point) {
                self.open_menu = (self.open_menu != Some(kind)).then_some(kind);
                return true;
            }
        }
        let Some(kind) = self.open_menu else {
            return false;
        };
        let dropdown = dropdown_rect(kind);
        if !dropdown.contains(point) {
            self.open_menu = None;
            return true;
        }
        let index = ((my - dropdown.y - 4.0) / 30.0).floor().max(0.0) as usize;
        self.open_menu = None;
        self.run_menu_action(kind, index);
        true
    }

    fn run_menu_action(&mut self, kind: EditorMenuKind, index: usize) {
        match (kind, index) {
            (EditorMenuKind::File, 0) => self.save_all_editor_documents(),
            (EditorMenuKind::File, 1) => self.reload_all_editor_documents(),
            (EditorMenuKind::File, 2) => self.regenerate_current_archipelago_seed(),
            (EditorMenuKind::File, 3) => self.reroll_structural_archipelago(),
            (EditorMenuKind::File, 4) => self.export_island_preview_pngs_with_status(),
            (EditorMenuKind::File, 5) => self.play_development_world(false),
            (EditorMenuKind::File, 6) => self.play_development_world(true),
            (EditorMenuKind::File, 7) => self.stop_development_client(),
            (EditorMenuKind::Edit, 0) => self.undo_world_edit(),
            (EditorMenuKind::Edit, 1) => self.redo_world_edit(),
            (EditorMenuKind::View, 0) => self.viewport_mode = EditorViewportMode::RegionGraph,
            (EditorMenuKind::View, 1) => self.viewport_mode = EditorViewportMode::SceneRectangles,
            (EditorMenuKind::View, 2) => self.viewport_mode = EditorViewportMode::SceneBank,
            (EditorMenuKind::View, 3) => self.viewport_mode = EditorViewportMode::SceneMap,
            (EditorMenuKind::View, 4) => self.viewport_mode = EditorViewportMode::PixelStudio,
            (EditorMenuKind::View, 5) => self.viewport_mode = EditorViewportMode::AnimationStudio,
            (EditorMenuKind::View, 6) => self.viewport_mode = EditorViewportMode::CharacterStudio,
            (EditorMenuKind::View, 7) => self.toggle_left_workspace_panel(),
            (EditorMenuKind::View, 8) => self.toggle_right_workspace_panel(),
            (EditorMenuKind::View, 9) => self.toggle_bottom_workspace_panel(),
            (EditorMenuKind::View, 10) => self.open_validation_bottom_panel(),
            (EditorMenuKind::View, 11) => self.reset_workspace_layout(),
            (EditorMenuKind::Help, 0) => {
                self.status_message = "Ctrl+S or F5 Save All | Ctrl+Z/Y Undo/Redo | Tab changes document | Ctrl+J bottom panels | Ctrl+Shift+L project panel | Ctrl+Shift+I inspector | wheel zoom | middle-drag or Space+drag pan | Sprite/Pixel Studio: [/] zoom, F frame, recovery autosave | Animation Studio: Space play, arrows frames, A add frame, P pivot, H shadow, K socket, E edit frame pixels".to_string();
            }
            (EditorMenuKind::Help, 1) => {
                self.status_message = "The native editor uses one persistent workspace shell: project/assets at left, tabbed authoring documents in the center, context inspector at right, fixed status bar, and overlay bottom panels for console, validation, imports, build readiness, and tasks. World Editor is the continuous global-tile Alderreach authoring lane; Scene Bank remains for interiors, caves, ruins, and instances; selected animation frames round-trip into Sprite/Pixel Studio without overwriting imported sources".to_string();
            }
            _ => {}
        }
    }

    pub(crate) fn save_all_editor_documents(&mut self) {
        let root = repo_root_dir();
        let project_path = root.join(STARTER_PROJECT_FILE_PATH);
        let scene_manifest_path = root.join(SCENE_RECTANGLE_MANIFEST_PATH);
        let scene_assignments_path = root.join(SCENE_RECTANGLE_ASSIGNMENTS_PATH);
        let harbor_route_path = root.join(HARBOR_ROUTE_CATALOG_PATH);
        let project_result = self.model.project.save_to_path(&project_path.to_string_lossy());
        let manifest_result = self
            .scene_rectangles
            .as_ref()
            .ok_or_else(|| "scene rectangle manifest is unavailable".to_string())
            .and_then(|manifest| manifest.save_to_path(&scene_manifest_path.to_string_lossy()));
        let assignment_result = self
            .scene_assignments
            .save_to_path(&scene_assignments_path.to_string_lossy());
        let route_result = self.harbor_routes.save_to_path(&harbor_route_path.to_string_lossy());
        let editor_world_path = development_session::editor_world_path();
        let world_result = save_world_to_path(&editor_world_path.to_string_lossy(), &self.model.world);
        let preview_result = self.export_island_preview_pngs();
        let pixel_result = self
            .pixel_studio
            .document
            .as_mut()
            .map_or(Ok(()), |document| document.save(repo_root_dir()));
        let animation_result = self
            .animation_studio
            .document
            .as_mut()
            .map_or(Ok(()), |document| document.save(repo_root_dir()));
        match (
            project_result,
            manifest_result,
            assignment_result,
            route_result,
            world_result,
            preview_result,
            pixel_result,
            animation_result,
        ) {
            (Ok(()), Ok(()), Ok(()), Ok(()), Ok(()), Ok(count), Ok(()), Ok(())) => {
                self.saved_undo_depth = self.command_bus.undo_len();
                let _ = self.workspace_shell.save_default();
                self.status_message = format!(
                    "Saved project, island layout, assignments, harbor routes, editable world, open art/animation documents, and {count} assembled island PNG previews"
                );
                self.command_bus.record_event(
                    self.app_command(EditorCommandKind::SaveWorld, self.status_message.clone()),
                );
            }
            (Err(error), _, _, _, _, _, _, _)
            | (_, Err(error), _, _, _, _, _, _)
            | (_, _, Err(error), _, _, _, _, _)
            | (_, _, _, Err(error), _, _, _, _)
            | (_, _, _, _, Err(error), _, _, _)
            | (_, _, _, _, _, Err(error), _, _)
            | (_, _, _, _, _, _, Err(error), _)
            | (_, _, _, _, _, _, _, Err(error)) => {
                self.status_message = format!("Save failed: {error}")
            }
        }
    }

    pub(crate) fn play_development_world(&mut self, from_here: bool) {
        // Editor-owned development play must launch the authored state the
        // editor is showing, not an older on-disk scene.
        self.save_all_editor_documents();
        if self.status_message.starts_with("Save failed:") {
            return;
        }
        self.stop_development_client();
        let descriptor = match development_session::DevelopmentWorldDescriptor::load() {
            Ok(descriptor) => descriptor,
            Err(error) => {
                self.status_message = format!("Development world unavailable: {error}");
                return;
            }
        };
        let active_scene = self.active_scene_id().map(|scene| scene.to_string());
        let Some(active_scene) = active_scene else {
            self.status_message = "Play requires an open Scene Editor scene".to_string();
            return;
        };
        let selected_scene_spawn = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .map(|scene| [scene.spawn_x, scene.spawn_y])
            .unwrap_or([descriptor.spawn.x, descriptor.spawn.y]);
        let mut published_world = self.model.world.clone();
        if let Err(error) = published_world
            .set_active_scene(haven_core::ProjectSceneId::new(active_scene.clone()))
        {
            self.status_message = format!(
                "Development publish failed to activate editor scene {active_scene}: {error}"
            );
            return;
        }
        if let Some(parent) = descriptor.world_path().parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                self.status_message = format!("Development publish failed: {error}");
                return;
            }
        }
        let development_world_path = descriptor.world_path();
        let editor_sync = haven_core::format_world_sync(
            "EDITOR",
            &published_world,
            Some(&haven_core::ProjectSceneId::new(active_scene.clone())),
        );
        eprintln!("{editor_sync}");
        crate::append_editor_log(&editor_sync);
        if let Err(error) = haven_save::save_world_to_path(
            &development_world_path.to_string_lossy(),
            &published_world,
        ) {
            self.status_message = format!("Development publish failed: {error}");
            return;
        }
        let published_sync = haven_core::format_world_sync(
            "PUBLISHED",
            &published_world,
            Some(&haven_core::ProjectSceneId::new(active_scene.clone())),
        );
        eprintln!("{published_sync}");
        crate::append_editor_log(&published_sync);
        match haven_save::load_world_from_path(&development_world_path.to_string_lossy()) {
            Ok(roundtrip) => {
                let roundtrip_sync = haven_core::format_world_sync(
                    "ROUNDTRIP",
                    &roundtrip,
                    Some(&haven_core::ProjectSceneId::new(active_scene.clone())),
                );
                eprintln!("{roundtrip_sync}");
                crate::append_editor_log(&roundtrip_sync);
            }
            Err(error) => {
                self.status_message = format!("Development round-trip verification failed: {error}");
                return;
            }
        }
        // A development publish is an authored snapshot, not an old procedural
        // save. Publish current-generation metadata with it so startup never
        // runs PCG migration/population over the editor-authored scene.
        let metadata_path = development_world_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("world.json");
        let world_id = haven_save::WorldSaveId(descriptor.world_id.clone());
        let mut metadata = haven_save::WorldSaveMetadata::new(
            world_id,
            "Havenwild Editor Development World",
            1_337,
            published_world.scenes.len().max(1),
        );
        metadata.generated_exterior_scene_count = published_world
            .scenes
            .iter()
            .filter(|scene| scene.kind == SceneKind::Exterior)
            .count();
        metadata.starting_scene_code = active_scene.clone();
        if let Err(error) = haven_save::save_world_save_metadata(
            &metadata_path.to_string_lossy(),
            &metadata,
        ) {
            self.status_message = format!("Development metadata publish failed: {error}");
            return;
        }
        let scene = Some(active_scene.as_str());
        let spawn = if from_here {
            Some([self.scene_cursor_x, self.scene_cursor_y])
        } else {
            Some(selected_scene_spawn)
        };
        crate::append_editor_log(&format!(
            "Development Play launch: world={} scene={} spawn={},{} mode={}",
            descriptor.world_id,
            active_scene,
            spawn.map(|value| value[0]).unwrap_or(selected_scene_spawn[0]),
            spawn.map(|value| value[1]).unwrap_or(selected_scene_spawn[1]),
            if from_here { "from-here" } else { "scene-default" }
        ));
        match development_session::launch(&descriptor, scene, spawn) {
            Ok(child) => {
                self.development_client = Some(child);
                self.status_message = if from_here {
                    format!("Play From Here: {} at {}, {}", scene.unwrap_or("active scene"), self.scene_cursor_x, self.scene_cursor_y)
                } else {
                    format!("Playing editor scene {} in development world {}", active_scene, descriptor.world_id)
                };
            }
            Err(error) => self.status_message = format!("Development client launch failed: {error}"),
        }
    }

    pub(crate) fn poll_development_client(&mut self) {
        let Some(child) = self.development_client.as_mut() else { return; };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.development_client = None;
                let font_status = super::editor_text::initialize_editor_font();
                crate::append_editor_log(&format!(
                    "Development client exited ({status}); editor lifecycle recovery: {font_status}"
                ));
                self.status_message = format!("Development client exited ({status}); editor UI restored");
            }
            Ok(None) => {}
            Err(error) => {
                self.development_client = None;
                let font_status = super::editor_text::initialize_editor_font();
                crate::append_editor_log(&format!(
                    "Development client status failed: {error}; editor lifecycle recovery: {font_status}"
                ));
                self.status_message = format!("Development client status failed: {error}");
            }
        }
    }

    pub(crate) fn stop_development_client(&mut self) {
        let Some(mut child) = self.development_client.take() else {
            return;
        };
        self.status_message = match development_session::stop(&mut child) {
            Ok(()) => {
                let font_status = super::editor_text::initialize_editor_font();
                crate::append_editor_log(&format!(
                    "Development client stopped; editor lifecycle recovery: {font_status}"
                ));
                "Development client stopped; editor UI restored".to_string()
            }
            Err(error) => format!("Unable to stop development client: {error}"),
        };
    }

    pub(crate) fn reload_all_editor_documents(&mut self) {
        match (
            load_editor_project_file_from_path(STARTER_PROJECT_FILE_PATH),
            load_active_scene_rectangle_manifest(),
            load_active_scene_rectangle_assignments(),
            HarborRouteCatalog::load_from_path(HARBOR_ROUTE_CATALOG_PATH),
            load_world_from_path(&development_session::editor_world_path().to_string_lossy()),
        ) {
            (Ok(project), Ok(manifest), Ok(assignments), Ok(routes), Ok(world)) => {
                self.model.project = project;
                self.scene_rectangles = Some(manifest);
                self.scene_assignments = assignments;
                self.harbor_routes = routes;
                self.model.world = world;
                self.model.region_graph = haven_world::region_graph::starter_island_region_graph();
                self.restore_generated_harbor_routes();
                self.selected_scene = self
                    .selected_scene
                    .min(self.model.world.scenes.len().saturating_sub(1));
                self.selection.clear();
                self.command_bus.clear_history();
                self.saved_undo_depth = 0;
                self.status_message = format!(
                    "Reloaded project, assignments, and development world from {}",
                    development_session::path_label(&development_session::editor_world_path())
                );
            }
            (Err(error), _, _, _, _)
            | (_, Err(error), _, _, _)
            | (_, _, Err(error), _, _)
            | (_, _, _, Err(error), _)
            | (_, _, _, _, Err(error)) => {
                self.status_message = format!("Reload failed: {error}");
            }
        }
    }

    fn export_island_preview_pngs_with_status(&mut self) {
        self.status_message = match self.export_island_preview_pngs() {
            Ok(count) => format!("Exported {count} assembled island PNG previews"),
            Err(error) => format!("Island preview export failed: {error}"),
        };
    }

    fn export_island_preview_pngs(&self) -> Result<usize, String> {
        let Some(manifest) = &self.scene_rectangles else {
            return Err("scene rectangle manifest is unavailable".to_string());
        };
        let output_dir = Path::new("content/worldgen/island_previews");
        create_dir_all(output_dir).map_err(|error| error.to_string())?;
        let mut exported = 0usize;
        for summary in super::island_workspace::landmass_summaries(manifest) {
            let rectangles: Vec<_> = summary
                .rectangle_indices
                .iter()
                .map(|index| &manifest.scene_rectangles[*index])
                .collect();
            let min_x = rectangles
                .iter()
                .filter_map(|entry| entry.grid_x)
                .min()
                .unwrap_or(0);
            let max_x = rectangles
                .iter()
                .filter_map(|entry| entry.grid_x)
                .max()
                .unwrap_or(0);
            let min_y = rectangles
                .iter()
                .filter_map(|entry| entry.grid_y)
                .min()
                .unwrap_or(0);
            let max_y = rectangles
                .iter()
                .filter_map(|entry| entry.grid_y)
                .max()
                .unwrap_or(0);
            let width = (max_x - min_x + 1).max(1) as u32 * MAP_W as u32;
            let height = (max_y - min_y + 1).max(1) as u32 * MAP_H as u32;
            let mut image = RgbaImage::from_pixel(width, height, Rgba([9, 42, 70, 255]));
            for rectangle in rectangles {
                let Some(assignment) = self
                    .scene_assignments
                    .assignment_for_rectangle(&rectangle.scene_id)
                else {
                    continue;
                };
                let Some(scene) = self
                    .model
                    .world
                    .scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))
                else {
                    continue;
                };
                let origin_x = (rectangle.grid_x.unwrap_or(min_x) - min_x) as u32 * MAP_W as u32;
                let origin_y = (rectangle.grid_y.unwrap_or(min_y) - min_y) as u32 * MAP_H as u32;
                for y in 0..MAP_H as i32 {
                    for x in 0..MAP_W as i32 {
                        image.put_pixel(
                            origin_x + x as u32,
                            origin_y + y as u32,
                            tile_preview_color(scene.map.get(x, y)),
                        );
                    }
                }
            }
            let path = output_dir.join(format!("{}.png", slug(&summary.name)));
            image
                .save(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            exported += 1;
        }
        let surface_rectangles: Vec<_> = manifest
            .scene_rectangles
            .iter()
            .filter(|rectangle| rectangle_is_overworld_surface(rectangle))
            .collect();
        if !surface_rectangles.is_empty() {
            let min_x = surface_rectangles
                .iter()
                .map(|rectangle| rectangle.world_rect_preview_px[0])
                .min()
                .unwrap_or(0);
            let min_y = surface_rectangles
                .iter()
                .map(|rectangle| rectangle.world_rect_preview_px[1])
                .min()
                .unwrap_or(0);
            let max_x = surface_rectangles
                .iter()
                .map(|rectangle| {
                    rectangle.world_rect_preview_px[0] + rectangle.world_rect_preview_px[2]
                })
                .max()
                .unwrap_or(min_x + 1);
            let max_y = surface_rectangles
                .iter()
                .map(|rectangle| {
                    rectangle.world_rect_preview_px[1] + rectangle.world_rect_preview_px[3]
                })
                .max()
                .unwrap_or(min_y + 1);
            let width = (max_x - min_x).max(1) as u32;
            let height = (max_y - min_y).max(1) as u32;
            let mut archipelago = RgbaImage::from_pixel(width, height, Rgba([9, 42, 70, 255]));
            for link in self
                .model
                .region_graph
                .links
                .iter()
                .filter(|link| link.kind == RegionLinkKind::SeaRoute)
            {
                let Some(from) = self.model.region_graph.node(&link.from) else {
                    continue;
                };
                let Some(to) = self.model.region_graph.node(&link.to) else {
                    continue;
                };
                draw_preview_line(
                    &mut archipelago,
                    (from.position.x * (width.saturating_sub(1)) as f32) as i32,
                    (from.position.y * (height.saturating_sub(1)) as f32) as i32,
                    (to.position.x * (width.saturating_sub(1)) as f32) as i32,
                    (to.position.y * (height.saturating_sub(1)) as f32) as i32,
                    Rgba([72, 190, 235, 255]),
                );
            }
            for rectangle in surface_rectangles {
                let Some(assignment) = self
                    .scene_assignments
                    .assignment_for_rectangle(&rectangle.scene_id)
                else {
                    continue;
                };
                let Some(scene) = self
                    .model
                    .world
                    .scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))
                else {
                    continue;
                };
                let [preview_x, preview_y, preview_w, preview_h] = rectangle.world_rect_preview_px;
                for pixel_y in 0..preview_h.max(1) as u32 {
                    for pixel_x in 0..preview_w.max(1) as u32 {
                        let scene_x = (pixel_x * MAP_W as u32 / preview_w.max(1) as u32)
                            .min(MAP_W as u32 - 1) as i32;
                        let scene_y = (pixel_y * MAP_H as u32 / preview_h.max(1) as u32)
                            .min(MAP_H as u32 - 1) as i32;
                        let target_x = (preview_x - min_x) as u32 + pixel_x;
                        let target_y = (preview_y - min_y) as u32 + pixel_y;
                        if target_x < width && target_y < height {
                            archipelago.put_pixel(
                                target_x,
                                target_y,
                                tile_preview_color(scene.map.get(scene_x, scene_y)),
                            );
                        }
                    }
                }
            }
            let path = output_dir.join("archipelago.png");
            archipelago
                .save(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            exported += 1;
        }
        Ok(exported)
    }
}

fn draw_preview_line(
    image: &mut RgbaImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Rgba<u8>,
) {
    let delta_x = (x1 - x0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let delta_y = -(y1 - y0).abs();
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut error = delta_x + delta_y;
    loop {
        for offset_y in -1..=1 {
            for offset_x in -1..=1 {
                let x = x0 + offset_x;
                let y = y0 + offset_y;
                if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
                    image.put_pixel(x as u32, y as u32, color);
                }
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled = 2 * error;
        if doubled >= delta_y {
            error += delta_y;
            x0 += step_x;
        }
        if doubled <= delta_x {
            error += delta_x;
            y0 += step_y;
        }
    }
}

fn tile_preview_color(tile: TileKind) -> Rgba<u8> {
    let rgba = match tile.code() {
        "deep_water" => [10, 48, 82, 255],
        "shallow_water" | "water" => [31, 102, 135, 255],
        "sand" => [201, 180, 121, 255],
        "wet_sand" | "pebble_shore" => [156, 139, 103, 255],
        "grass" => [80, 137, 76, 255],
        "tall_grass" => [66, 122, 64, 255],
        "cliff" | "mountain_rock" => [91, 91, 88, 255],
        "road" | "dirt" => [125, 94, 63, 255],
        _ => [96, 136, 86, 255],
    };
    Rgba(rgba)
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('_') {
            output.push('_');
        }
    }
    output.trim_matches('_').to_string()
}
