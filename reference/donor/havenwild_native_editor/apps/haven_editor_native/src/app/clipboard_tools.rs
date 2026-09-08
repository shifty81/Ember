use super::*;

impl EditorApp {
    pub(crate) fn paste_clipboard_at_cursor(&mut self) {
        let Some(clipboard) = self.scene_clipboard.clone() else {
            self.status_message = "Clipboard is empty".to_string();
            return;
        };
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        self.scene_layer_mode = SceneLayerMode::from_authoring_layer(clipboard.layer);
        if self.active_layer_state().locked {
            self.status_message = format!("{} layer is locked", self.scene_layer_mode.label());
            return;
        }
        let result = paste_scene_clipboard(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id.clone(),
            &clipboard,
            GridPos {
                x: self.scene_cursor_x,
                y: self.scene_cursor_y,
            },
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

    pub(crate) fn duplicate_current_selection(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        let Some(bounds) = self.selection.bounds else {
            self.status_message = "Nothing selected to duplicate".to_string();
            return;
        };
        let clipboard = match copy_scene_selection(
            scene,
            self.scene_authoring_layer(),
            &self.selection.items,
            bounds,
        ) {
            Ok(clipboard) => clipboard,
            Err(error) => {
                self.status_message = error;
                return;
            }
        };
        self.scene_clipboard = Some(clipboard);
        self.scene_cursor_x = bounds.min.x + 1;
        self.scene_cursor_y = bounds.min.y + 1;
        self.paste_clipboard_at_cursor();
    }

    pub(crate) fn delete_current_selection(&mut self) {
        if self.active_layer_state().locked {
            self.status_message = format!("{} layer is locked", self.scene_layer_mode.label());
            return;
        }
        let Some(scene_id) = self.active_scene_id() else {
            return;
        };
        if self.selection.is_empty() {
            self.status_message = "Nothing selected to delete".to_string();
            return;
        }
        let tile = self.selected_tile_kind();
        let result = delete_scene_selection(
            &mut self.model.world,
            &mut self.command_bus,
            &self.model.project.project_id,
            EditorCommandSource::MainEditor,
            scene_id,
            &self.selection.items,
            tile,
        );
        self.finish_bulk_result(result);
        self.selection.clear_items();
    }

    pub(crate) fn frame_current_selection(&mut self) {
        let Some(bounds) = self.selection.bounds else {
            self.status_message = "Nothing selected to frame".to_string();
            return;
        };
        self.scene_canvas.frame_rect(
            self.scene_canvas_viewport_rect(),
            self.scene_canvas_bounds(),
            Rect::new(
                bounds.min.x as f32,
                bounds.min.y as f32,
                bounds.width() as f32,
                bounds.height() as f32,
            ),
        );
        self.status_message = "Framed current selection".to_string();
    }

    pub(crate) fn scene_drag_preview_rect(&self) -> Option<GridRect> {
        let drag = self.scene_drag?;
        match drag.kind {
            SceneCanvasDragKind::MoveSelection => self.selection.bounds.map(|bounds| {
                bounds.translated(GridPos {
                    x: drag.current.x - drag.start.x,
                    y: drag.current.y - drag.start.y,
                })
            }),
            SceneCanvasDragKind::Marquee | SceneCanvasDragKind::Rectangle => Some(drag.rect()),
        }
    }
}
