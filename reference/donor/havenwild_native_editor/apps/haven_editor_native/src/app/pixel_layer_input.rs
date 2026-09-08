use super::pixel_layer_panel::*;
use super::*;
use haven_assets::asset_intake::repo_root_dir;

impl EditorApp {
    pub(crate) fn handle_pixel_layer_rename_input(&mut self) -> bool {
        let Some(buffer) = self.pixel_studio.layer_rename_buffer.as_mut() else {
            return false;
        };
        while let Some(character) = get_char_pressed() {
            if !character.is_control() && buffer.chars().count() < 48 {
                buffer.push(character);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            buffer.pop();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.pixel_studio.layer_rename_buffer = None;
            self.status_message = "Layer rename cancelled".to_string();
            return true;
        }
        if is_key_pressed(KeyCode::Enter) {
            let name = self
                .pixel_studio
                .layer_rename_buffer
                .take()
                .unwrap_or_default();
            if let Some(document) = self.pixel_studio.document.as_mut() {
                if document.rename_active_layer(name) {
                    self.status_message = "Layer renamed".to_string();
                }
            }
            return true;
        }
        true
    }

    pub(crate) fn handle_pixel_layer_panel_click(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if self.pixel_studio.document.is_none() {
            return false;
        }
        if pixel_layer_add_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                let number = document.layer_count() + 1;
                document.add_layer(format!("Layer {number}"));
            }
            self.pixel_studio.layer_offset = 0;
            self.pixel_studio.refresh_texture();
            self.status_message = "Added pixel layer".to_string();
            return true;
        }
        if pixel_layer_duplicate_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.duplicate_active_layer();
            }
            self.pixel_studio.layer_offset = 0;
            self.pixel_studio.refresh_texture();
            self.status_message = "Duplicated active layer".to_string();
            return true;
        }
        if pixel_layer_delete_rect(rect).contains(mouse) {
            let deleted = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.delete_active_layer());
            if deleted {
                self.pixel_studio.refresh_texture();
            }
            self.status_message = if deleted {
                "Deleted active layer".to_string()
            } else {
                "A pixel document must keep at least one layer".to_string()
            };
            return true;
        }
        if pixel_layer_up_rect(rect).contains(mouse) {
            let moved = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.move_active_layer(1));
            if moved {
                self.pixel_studio.refresh_texture();
                self.status_message = "Moved layer up".to_string();
            }
            return true;
        }
        if pixel_layer_down_rect(rect).contains(mouse) {
            let moved = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.move_active_layer(-1));
            if moved {
                self.pixel_studio.refresh_texture();
                self.status_message = "Moved layer down".to_string();
            }
            return true;
        }
        if pixel_layer_merge_rect(rect).contains(mouse) {
            let merged = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.merge_active_down());
            if merged {
                self.pixel_studio.refresh_texture();
            }
            self.status_message = if merged {
                "Merged active layer down".to_string()
            } else {
                "The bottom layer cannot merge down".to_string()
            };
            return true;
        }
        for row in 0..8 {
            let index = self.pixel_studio.document.as_ref().and_then(|document| {
                pixel_layer_index_for_row(
                    document.layer_count(),
                    self.pixel_studio.layer_offset,
                    row,
                )
            });
            let Some(index) = index else {
                break;
            };
            if pixel_layer_visibility_rect(rect, row).contains(mouse) {
                if let Some(document) = self.pixel_studio.document.as_mut() {
                    document.select_layer(index);
                    document.toggle_active_layer_visibility();
                }
                self.pixel_studio.refresh_texture();
                return true;
            }
            if pixel_layer_lock_rect(rect, row).contains(mouse) {
                if let Some(document) = self.pixel_studio.document.as_mut() {
                    document.select_layer(index);
                    document.toggle_active_layer_lock();
                }
                return true;
            }
            if pixel_layer_row_rect(rect, row).contains(mouse) {
                let name = self.pixel_studio.document.as_mut().and_then(|document| {
                    document
                        .select_layer(index)
                        .then(|| document.active_layer().metadata.name.clone())
                });
                self.pixel_studio.layer_rename_buffer = None;
                if let Some(name) = name {
                    self.status_message = format!("Selected layer {name}");
                }
                return true;
            }
        }
        if pixel_layer_visibility_toggle_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.toggle_active_layer_visibility();
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_layer_lock_toggle_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.toggle_active_layer_lock();
            }
            return true;
        }
        if pixel_layer_opacity_minus_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.adjust_active_layer_opacity(-16);
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_layer_opacity_plus_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.adjust_active_layer_opacity(16);
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_layer_blend_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.cycle_active_layer_blend_mode();
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_layer_rename_rect(rect).contains(mouse)
            || pixel_layer_rename_field_rect(rect).contains(mouse)
        {
            self.pixel_studio.layer_rename_buffer = self
                .pixel_studio
                .document
                .as_ref()
                .map(|document| document.active_layer().metadata.name.clone());
            self.status_message = "Type a layer name, then press Enter".to_string();
            return true;
        }
        if pixel_layer_save_rect(rect).contains(mouse) {
            self.save_pixel_document();
            return true;
        }
        if pixel_layer_autosave_rect(rect).contains(mouse) {
            let result = self
                .pixel_studio
                .document
                .as_mut()
                .map(|document| document.autosave(repo_root_dir()));
            self.status_message = match result {
                Some(Ok(path)) => {
                    let message = format!("Autosaved recovery to {}", path.display());
                    self.pixel_studio.autosave_status = message.clone();
                    message
                }
                Some(Err(error)) => format!("Autosave failed: {error}"),
                None => "No pixel document is open".to_string(),
            };
            return true;
        }
        let animation_bridge_active = self.pixel_studio.animation_context.is_some();
        if animation_bridge_active
            && (pixel_layer_crop_rect(rect).contains(mouse)
                || pixel_layer_trim_rect(rect).contains(mouse)
                || pixel_layer_resize_w_minus_rect(rect).contains(mouse)
                || pixel_layer_resize_w_plus_rect(rect).contains(mouse)
                || pixel_layer_resize_h_minus_rect(rect).contains(mouse)
                || pixel_layer_resize_h_plus_rect(rect).contains(mouse)
                || pixel_layer_resize_apply_rect(rect).contains(mouse))
        {
            self.status_message =
                "Canvas crop/trim/resize is disabled while editing an animation frame; return to a normal Pixel Studio document for sheet geometry changes"
                    .to_string();
            return true;
        }
        if pixel_layer_crop_rect(rect).contains(mouse) {
            let cropped = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.crop_to_selection());
            if cropped {
                if let Some((width, height)) = self
                    .pixel_studio
                    .document
                    .as_ref()
                    .map(|document| (document.width(), document.height()))
                {
                    self.pixel_studio.resize_width = width;
                    self.pixel_studio.resize_height = height;
                }
                self.pixel_studio.refresh_texture();
                self.pixel_studio.needs_frame = true;
            }
            self.status_message = if cropped {
                "Cropped document to the selected rectangle".to_string()
            } else {
                "Crop selection is empty or already covers the full canvas".to_string()
            };
            return true;
        }
        if pixel_layer_trim_rect(rect).contains(mouse) {
            let trimmed = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.trim_transparent_padding());
            if trimmed {
                if let Some((width, height)) = self
                    .pixel_studio
                    .document
                    .as_ref()
                    .map(|document| (document.width(), document.height()))
                {
                    self.pixel_studio.resize_width = width;
                    self.pixel_studio.resize_height = height;
                }
                self.pixel_studio.refresh_texture();
                self.pixel_studio.needs_frame = true;
            }
            self.status_message = if trimmed {
                "Trimmed transparent canvas padding".to_string()
            } else {
                "No transparent padding could be trimmed".to_string()
            };
            return true;
        }
        if pixel_layer_resize_w_minus_rect(rect).contains(mouse) {
            self.pixel_studio.resize_width =
                self.pixel_studio.resize_width.saturating_sub(1).max(1);
            return true;
        }
        if pixel_layer_resize_w_plus_rect(rect).contains(mouse) {
            self.pixel_studio.resize_width = (self.pixel_studio.resize_width + 1).min(8192);
            return true;
        }
        if pixel_layer_resize_h_minus_rect(rect).contains(mouse) {
            self.pixel_studio.resize_height =
                self.pixel_studio.resize_height.saturating_sub(1).max(1);
            return true;
        }
        if pixel_layer_resize_h_plus_rect(rect).contains(mouse) {
            self.pixel_studio.resize_height = (self.pixel_studio.resize_height + 1).min(8192);
            return true;
        }
        if pixel_layer_resize_apply_rect(rect).contains(mouse) {
            let width = self.pixel_studio.resize_width;
            let height = self.pixel_studio.resize_height;
            let resized = self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.resize_canvas(width, height));
            if resized {
                self.pixel_studio.refresh_texture();
                self.pixel_studio.needs_frame = true;
                self.status_message = format!(
                    "Resized pixel canvas to {} x {}",
                    self.pixel_studio.resize_width, self.pixel_studio.resize_height
                );
            }
            return true;
        }
        true
    }
}
