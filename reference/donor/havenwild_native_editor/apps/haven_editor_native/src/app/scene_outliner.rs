use super::render_helpers::*;
use super::*;

const SCENE_VISIBLE_ROWS: usize = 5;
const OBJECT_MAX_VISIBLE_ROWS: usize = 8;

#[derive(Clone, Copy)]
enum OutlinerEntry {
    Object(usize),
    Stamp(usize),
}

impl EditorApp {
    pub(crate) fn draw_scene_outliner(&self, rect: Rect) {
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Scenes",
            Some("world documents"),
        );
        for (index, label) in ["New", "Duplicate", "Rename", self.scene_delete_label()]
            .into_iter()
            .enumerate()
        {
            draw_editor_widget(scene_action_rect(rect, index), label, false);
        }

        let edit_rect = scene_name_input_rect(rect);
        if let Some(edit) = &self.scene_name_edit {
            draw_editor_widget(
                edit_rect,
                &edit.buffer,
                self.text_focus == EditorTextFocus::SceneName,
            );
            draw_editor_text(
                match edit.mode {
                    SceneNameEditMode::Create => "New scene name — Enter confirms",
                    SceneNameEditMode::Rename => "Rename scene — Enter confirms",
                },
                rect.x,
                edit_rect.y + edit_rect.h + 15.0,
                13.0,
                GOOD,
            );
        } else {
            let selected = self
                .model
                .world
                .scenes
                .get(self.selected_scene)
                .map(|scene| format!("{} ({})", scene.name, scene.id))
                .unwrap_or_else(|| "No scene selected".to_string());
            draw_editor_widget(edit_rect, &selected, false);
        }

        let scene_count = self.model.world.scenes.len();
        let start = self.scene_list_offset.min(scene_count.saturating_sub(1));
        for slot in 0..SCENE_VISIBLE_ROWS {
            let index = start + slot;
            let Some(scene) = self.model.world.scenes.get(index) else {
                break;
            };
            let row = scene_row_rect(rect, slot);
            draw_list_row(
                row,
                &scene.name,
                Some(&format!(
                    "{}  •  {} objects  •  {} stamps",
                    scene.kind.code(),
                    scene.map.objects.len(),
                    scene.map.stamps.len()
                )),
                index == self.selected_scene,
            );
        }
        draw_editor_widget(scene_page_prev_rect(rect), "Prev", false);
        draw_editor_widget(scene_page_next_rect(rect), "Next", false);
        draw_editor_text(
            &format!(
                "{} scene{}",
                scene_count,
                if scene_count == 1 { "" } else { "s" }
            ),
            rect.x + 78.0,
            scene_page_prev_rect(rect).y + 19.0,
            14.0,
            MUTED,
        );

        draw_section_header(
            Rect::new(rect.x, scene_page_prev_rect(rect).y + 28.0, rect.w, 24.0),
            "Objects & Stamps",
            Some("selected scene"),
        );
        let search = object_filter_rect(rect);
        let filter_label = if self.object_filter.is_empty() {
            "Search objects or stamps..."
        } else {
            self.object_filter.as_str()
        };
        draw_editor_widget(
            search,
            filter_label,
            self.text_focus == EditorTextFocus::ObjectFilter,
        );

        let entries = self.filtered_outliner_entries();
        let object_start = self.object_list_offset.min(entries.len().saturating_sub(1));
        for slot in 0..object_visible_rows(rect) {
            let Some(entry) = entries.get(object_start + slot).copied() else {
                break;
            };
            let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
                break;
            };
            let (label, active) = match entry {
                OutlinerEntry::Object(index) => {
                    let Some(object) = scene.map.objects.get(index) else {
                        continue;
                    };
                    (
                        format!("{}  {},{}", object.kind.label(), object.x, object.y),
                        self.selected_object_id() == Some(object.id),
                    )
                }
                OutlinerEntry::Stamp(index) => {
                    let Some(stamp) = scene.map.stamps.get(index) else {
                        continue;
                    };
                    let label = self
                        .stamp_registry
                        .entry(&stamp.stamp_key)
                        .map_or(stamp.stamp_key.as_str(), |definition| {
                            definition.label.as_str()
                        });
                    (
                        format!("STAMP {}  {},{}", label, stamp.x, stamp.y),
                        self.selected_stamp_instance_id() == Some(stamp.id),
                    )
                }
            };
            let row = object_row_rect(rect, slot);
            draw_list_row(row, &label, None, active);
        }
        draw_editor_widget(object_page_prev_rect(rect), "Prev", false);
        draw_editor_widget(object_page_next_rect(rect), "Next", false);
        draw_editor_text(
            &format!("{} / {}", entries.len(), self.active_outliner_count()),
            rect.x + 80.0,
            object_page_prev_rect(rect).y + 19.0,
            14.0,
            MUTED,
        );
    }

    pub(crate) fn handle_scene_outliner_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }

        for action in 0..4 {
            if scene_action_rect(rect, action).contains(mouse) {
                self.text_focus = EditorTextFocus::None;
                match action {
                    0 => self.begin_scene_name_edit(SceneNameEditMode::Create),
                    1 => self.duplicate_selected_scene(),
                    2 => self.begin_scene_name_edit(SceneNameEditMode::Rename),
                    3 => self.delete_selected_scene(),
                    _ => {}
                }
                return true;
            }
        }
        if scene_name_input_rect(rect).contains(mouse) {
            if self.scene_name_edit.is_some() {
                self.text_focus = EditorTextFocus::SceneName;
            }
            return true;
        }
        for slot in 0..SCENE_VISIBLE_ROWS {
            if !scene_row_rect(rect, slot).contains(mouse) {
                continue;
            }
            let index = self.scene_list_offset + slot;
            if index < self.model.world.scenes.len() {
                self.select_scene_index(index);
                self.ensure_scene_visible();
                self.object_list_offset = 0;
                self.scene_delete_armed = None;
                self.status_message =
                    format!("Opened scene {}", self.model.world.scenes[index].name);
            }
            return true;
        }
        if scene_page_prev_rect(rect).contains(mouse) {
            self.scene_list_offset = self.scene_list_offset.saturating_sub(SCENE_VISIBLE_ROWS);
            return true;
        }
        if scene_page_next_rect(rect).contains(mouse) {
            let max = self
                .model
                .world
                .scenes
                .len()
                .saturating_sub(SCENE_VISIBLE_ROWS);
            self.scene_list_offset = (self.scene_list_offset + SCENE_VISIBLE_ROWS).min(max);
            return true;
        }
        if object_filter_rect(rect).contains(mouse) {
            self.text_focus = EditorTextFocus::ObjectFilter;
            return true;
        }

        let entries = self.filtered_outliner_entries();
        let object_start = self.object_list_offset.min(entries.len().saturating_sub(1));
        for slot in 0..object_visible_rows(rect) {
            if !object_row_rect(rect, slot).contains(mouse) {
                continue;
            }
            let Some(entry) = entries.get(object_start + slot).copied() else {
                return true;
            };
            let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
                return true;
            };
            let scene_id = scene.id.clone();
            match entry {
                OutlinerEntry::Object(index) => {
                    let Some(object) = scene.map.objects.get(index).copied() else {
                        return true;
                    };
                    let item = SelectionItem::Object(object.id);
                    let bounds = selection_bounds_for_items(scene, std::slice::from_ref(&item));
                    self.selection.replace_many(scene_id, vec![item], bounds);
                    self.scene_cursor_x = object.x;
                    self.scene_cursor_y = object.y;
                    self.status_message =
                        format!("Selected {} ({})", object.kind.label(), object.id);
                }
                OutlinerEntry::Stamp(index) => {
                    let Some(stamp) = scene.map.stamps.get(index) else {
                        return true;
                    };
                    let item = SelectionItem::Stamp(stamp.id);
                    let bounds = selection_bounds_for_items(scene, std::slice::from_ref(&item));
                    self.selection.replace_many(scene_id, vec![item], bounds);
                    self.scene_cursor_x = stamp.x;
                    self.scene_cursor_y = stamp.y;
                    self.status_message =
                        format!("Selected stamp {} ({})", stamp.stamp_key, stamp.id);
                }
            }
            self.scene_layer_mode = SceneLayerMode::Objects;
            self.scene_edit_tool = SceneEditTool::Select;
            self.scene_dock_tab = SceneDockTab::Inspector;
            return true;
        }
        if object_page_prev_rect(rect).contains(mouse) {
            self.object_list_offset = self
                .object_list_offset
                .saturating_sub(object_visible_rows(rect));
            return true;
        }
        if object_page_next_rect(rect).contains(mouse) {
            let visible = object_visible_rows(rect);
            let max = entries.len().saturating_sub(visible);
            self.object_list_offset = (self.object_list_offset + visible).min(max);
            return true;
        }
        true
    }

    pub(crate) fn handle_text_input(&mut self) {
        match self.text_focus {
            EditorTextFocus::None => (),
            EditorTextFocus::SceneName => {
                if is_key_pressed(KeyCode::Escape) {
                    self.scene_name_edit = None;
                    self.text_focus = EditorTextFocus::None;
                    self.status_message = "Scene name edit cancelled".to_string();
                    return;
                }
                if is_key_pressed(KeyCode::Enter) {
                    self.confirm_scene_name_edit();
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    if let Some(edit) = self.scene_name_edit.as_mut() {
                        edit.buffer.pop();
                    }
                }
                while let Some(character) = get_char_pressed() {
                    if let Some(edit) = self.scene_name_edit.as_mut() {
                        if edit.buffer.len() < 48
                            && (character.is_ascii_alphanumeric()
                                || matches!(character, ' ' | '_' | '-'))
                        {
                            edit.buffer.push(character);
                        }
                    }
                }
            }
            EditorTextFocus::ObjectFilter => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.text_focus = EditorTextFocus::None;
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.object_filter.pop();
                    self.object_list_offset = 0;
                }
                while let Some(character) = get_char_pressed() {
                    if self.object_filter.len() < 40
                        && (character.is_ascii_alphanumeric()
                            || matches!(character, ' ' | '_' | '-'))
                    {
                        self.object_filter.push(character);
                        self.object_list_offset = 0;
                    }
                }
            }
            EditorTextFocus::AssetFilter => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.text_focus = EditorTextFocus::None;
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.asset_filter.pop();
                    self.asset_list_offset = 0;
                }
                while let Some(character) = get_char_pressed() {
                    if self.asset_filter.len() < 48
                        && (character.is_ascii_alphanumeric()
                            || matches!(character, ' ' | '_' | '-' | '/'))
                    {
                        self.asset_filter.push(character);
                        self.asset_list_offset = 0;
                    }
                }
            }
            EditorTextFocus::AssetLibraryFilter => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.text_focus = EditorTextFocus::None;
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.asset_library_filter.pop();
                    self.asset_library_list_offset = 0;
                }
                while let Some(character) = get_char_pressed() {
                    if self.asset_library_filter.len() < 64
                        && (character.is_ascii_alphanumeric()
                            || matches!(character, ' ' | '_' | '-' | '/' | '.'))
                    {
                        self.asset_library_filter.push(character);
                        self.asset_library_list_offset = 0;
                    }
                }
            }
            EditorTextFocus::PixelLibraryFilter => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.text_focus = EditorTextFocus::None;
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.pixel_studio.library_filter.pop();
                    self.pixel_studio.library_offset = 0;
                }
                while let Some(character) = get_char_pressed() {
                    if self.pixel_studio.library_filter.len() < 64
                        && (character.is_ascii_alphanumeric()
                            || matches!(character, ' ' | '_' | '-' | '/' | '.'))
                    {
                        self.pixel_studio.library_filter.push(character);
                        self.pixel_studio.library_offset = 0;
                    }
                }
            }
            EditorTextFocus::CharacterAssetFilter => {
                if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) {
                    self.text_focus = EditorTextFocus::None;
                    return;
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.character_studio.query.pop();
                    self.character_studio.rebuild_filter();
                }
                while let Some(character) = get_char_pressed() {
                    if self.character_studio.query.len() < 72
                        && (character.is_ascii_alphanumeric()
                            || matches!(character, ' ' | '_' | '-' | '/' | '.'))
                    {
                        self.character_studio.query.push(character);
                        self.character_studio.rebuild_filter();
                    }
                }
            }
        }
    }

    fn begin_scene_name_edit(&mut self, mode: SceneNameEditMode) {
        let buffer = match mode {
            SceneNameEditMode::Create => "New Scene".to_string(),
            SceneNameEditMode::Rename => self
                .model
                .world
                .scenes
                .get(self.selected_scene)
                .map(|scene| scene.name.clone())
                .unwrap_or_else(|| "Scene".to_string()),
        };
        self.scene_name_edit = Some(SceneNameEditState { mode, buffer });
        self.text_focus = EditorTextFocus::SceneName;
        self.scene_delete_armed = None;
    }

    fn confirm_scene_name_edit(&mut self) {
        let Some(edit) = self.scene_name_edit.take() else {
            self.text_focus = EditorTextFocus::None;
            return;
        };
        let display_name = edit.buffer.trim().to_string();
        if display_name.is_empty() {
            self.scene_name_edit = Some(edit);
            self.status_message = "Scene name cannot be empty".to_string();
            return;
        }
        let requested_id = ProjectSceneId::new(display_name.clone());
        match edit.mode {
            SceneNameEditMode::Create => {
                let scene_id = self.unique_scene_id(requested_id);
                let (kind, biome) = self
                    .model
                    .world
                    .scenes
                    .get(self.selected_scene)
                    .map(|scene| (scene.kind, scene.biome))
                    .unwrap_or((SceneKind::Exterior, SceneBiome::Temperate));
                let scene = SceneMap::blank(scene_id.clone(), display_name, kind, biome);
                let result = create_project_scene(
                    &mut self.model.world,
                    &mut self.command_bus,
                    &self.model.project.project_id,
                    EditorCommandSource::MainEditor,
                    scene,
                );
                match result {
                    Ok(outcome) => {
                        self.selected_scene =
                            self.model.world.scenes.position(&scene_id).unwrap_or(0);
                        self.selection.clear();
                        self.scene_canvas = CanvasCameraState::default();
                        self.ensure_scene_visible();
                        self.status_message = outcome.message;
                    }
                    Err(error) => self.status_message = error,
                }
            }
            SceneNameEditMode::Rename => {
                let Some(current) = self.active_scene_id() else {
                    self.text_focus = EditorTextFocus::None;
                    return;
                };
                let replacement = if requested_id == current {
                    current.clone()
                } else if self.model.world.scene_by_id(&requested_id).is_some() {
                    self.status_message = format!("Scene '{}' already exists", requested_id);
                    self.scene_name_edit = Some(SceneNameEditState {
                        mode: edit.mode,
                        buffer: edit.buffer,
                    });
                    return;
                } else {
                    requested_id
                };
                let result = rename_project_scene(
                    &mut self.model.world,
                    &mut self.command_bus,
                    &self.model.project.project_id,
                    EditorCommandSource::MainEditor,
                    &current,
                    replacement.clone(),
                    display_name,
                );
                match result {
                    Ok(outcome) => {
                        if let Some(camera) = self.scene_canvas_states.remove(&current) {
                            self.scene_canvas_states.insert(replacement.clone(), camera);
                        }
                        self.selected_scene =
                            self.model.world.scenes.position(&replacement).unwrap_or(0);
                        self.selection.clear();
                        self.status_message = outcome.message;
                    }
                    Err(error) => self.status_message = error,
                }
            }
        }
        self.text_focus = EditorTextFocus::None;
        self.scene_delete_armed = None;
    }

    fn duplicate_selected_scene(&mut self) {
        let Some(source) = self.model.world.scenes.get(self.selected_scene).cloned() else {
            self.status_message = "No scene selected".to_string();
            return;
        };
        let replacement =
            self.unique_scene_id(ProjectSceneId::new(format!("{}_copy", source.id.code())));
        let display_name = format!("{} Copy", source.name);
        match duplicate_project_scene(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            &source.id,
            replacement.clone(),
            display_name,
        ) {
            Ok(outcome) => {
                self.selected_scene = self.model.world.scenes.position(&replacement).unwrap_or(0);
                self.scene_canvas = CanvasCameraState::default();
                self.selection.clear();
                self.ensure_scene_visible();
                self.status_message = outcome.message;
            }
            Err(error) => self.status_message = error,
        }
    }

    fn delete_selected_scene(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        let scene_id = scene.id.clone();
        if self.scene_delete_armed.as_ref() != Some(&scene_id) {
            self.scene_delete_armed = Some(scene_id.clone());
            self.status_message = format!("Press Delete Scene again to remove {}", scene.name);
            return;
        }
        match delete_project_scene(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            &scene_id,
        ) {
            Ok(outcome) => {
                self.scene_canvas_states.remove(&scene_id);
                self.selected_scene = self
                    .selected_scene
                    .min(self.model.world.scenes.len().saturating_sub(1));
                self.selection.clear();
                self.ensure_scene_visible();
                self.status_message = outcome.message;
            }
            Err(error) => self.status_message = error,
        }
        self.scene_delete_armed = None;
    }

    fn unique_scene_id(&self, requested: ProjectSceneId) -> ProjectSceneId {
        if self.model.world.scene_by_id(&requested).is_none() {
            return requested;
        }
        for suffix in 2..10_000 {
            let candidate = ProjectSceneId::new(format!("{}_{}", requested.code(), suffix));
            if self.model.world.scene_by_id(&candidate).is_none() {
                return candidate;
            }
        }
        ProjectSceneId::new(format!("{}_copy", requested.code()))
    }

    fn scene_delete_label(&self) -> &'static str {
        let selected = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .map(|scene| &scene.id);
        if selected.is_some() && selected == self.scene_delete_armed.as_ref() {
            "Confirm"
        } else {
            "Delete"
        }
    }

    fn filtered_outliner_entries(&self) -> Vec<OutlinerEntry> {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return Vec::new();
        };
        let filter = self.object_filter.trim().to_ascii_lowercase();
        let mut entries = scene
            .map
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| {
                filter.is_empty()
                    || object.kind.label().to_ascii_lowercase().contains(&filter)
                    || object.id.code().contains(&filter)
            })
            .map(|(index, _)| OutlinerEntry::Object(index))
            .collect::<Vec<_>>();
        entries.extend(
            scene
                .map
                .stamps
                .iter()
                .enumerate()
                .filter(|(_, stamp)| {
                    let label = self
                        .stamp_registry
                        .entry(&stamp.stamp_key)
                        .map_or("", |definition| definition.label.as_str());
                    filter.is_empty()
                        || stamp.stamp_key.contains(&filter)
                        || label.to_ascii_lowercase().contains(&filter)
                        || stamp.id.code().contains(&filter)
                })
                .map(|(index, _)| OutlinerEntry::Stamp(index)),
        );
        entries
    }

    fn active_outliner_count(&self) -> usize {
        self.model
            .world
            .scenes
            .get(self.selected_scene)
            .map_or(0, |scene| scene.map.objects.len() + scene.map.stamps.len())
    }

    pub(crate) fn ensure_scene_visible(&mut self) {
        if self.selected_scene < self.scene_list_offset {
            self.scene_list_offset = self.selected_scene;
        } else if self.selected_scene >= self.scene_list_offset + SCENE_VISIBLE_ROWS {
            self.scene_list_offset = self.selected_scene.saturating_sub(SCENE_VISIBLE_ROWS - 1);
        }
    }
}

pub(crate) fn scene_action_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(rect.x + index as f32 * 56.0, rect.y + 24.0, 52.0, 28.0)
}

pub(crate) fn scene_name_input_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 60.0, rect.w, 30.0)
}

pub(crate) fn scene_row_rect(rect: Rect, slot: usize) -> Rect {
    Rect::new(rect.x, rect.y + 112.0 + slot as f32 * 42.0, rect.w, 38.0)
}

pub(crate) fn scene_page_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 326.0, 64.0, 26.0)
}

pub(crate) fn scene_page_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 64.0, rect.y + 326.0, 64.0, 26.0)
}

pub(crate) fn object_filter_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 382.0, rect.w, 28.0)
}

pub(crate) fn object_row_rect(rect: Rect, slot: usize) -> Rect {
    Rect::new(rect.x, rect.y + 416.0 + slot as f32 * 29.0, rect.w, 26.0)
}

fn object_visible_rows(rect: Rect) -> usize {
    (((rect.h - 454.0) / 29.0).floor() as usize).clamp(1, OBJECT_MAX_VISIBLE_ROWS)
}

pub(crate) fn object_page_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 30.0, 64.0, 26.0)
}

pub(crate) fn object_page_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 64.0, rect.y + rect.h - 30.0, 64.0, 26.0)
}
