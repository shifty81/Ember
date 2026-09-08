use super::animation_studio::*;
use super::animation_studio_render::*;
use super::*;
use haven_pixel::{AnimationDirection, AnimationEventKind, AnimationLoopMode, AnimationSocketKind};

impl EditorApp {
    pub(crate) fn update_animation_studio_input(&mut self) {
        if self.viewport_mode != EditorViewportMode::AnimationStudio {
            return;
        }
        self.animation_studio.advance_playback(get_frame_time());
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let library_rect = self.shell_layout().list_content;
        let wheel = mouse_wheel().1;
        if library_rect.contains(mouse) && wheel.abs() > 0.01 {
            let visible = ((library_rect.h - 92.0) / 44.0).floor().max(1.0) as usize;
            let max_offset = self.animation_studio.library.len().saturating_sub(visible);
            let steps = wheel.abs().ceil().clamp(1.0, 3.0) as usize;
            if wheel < 0.0 {
                self.animation_studio.library_offset =
                    (self.animation_studio.library_offset + steps).min(max_offset);
            } else {
                self.animation_studio.library_offset =
                    self.animation_studio.library_offset.saturating_sub(steps);
            }
        }
        if is_key_pressed(KeyCode::Space) {
            self.animation_studio.playing = !self.animation_studio.playing;
            self.animation_studio.playback_elapsed_ms = 0.0;
            self.status_message = if self.animation_studio.playing {
                "Animation playback started"
            } else {
                "Animation playback paused"
            }
            .to_string();
        }
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        if shift && is_key_pressed(KeyCode::Left) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_clip(-1);
            }
        } else if shift && is_key_pressed(KeyCode::Right) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_clip(1);
            }
        } else if is_key_pressed(KeyCode::Left) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_frame(-1);
            }
        } else if is_key_pressed(KeyCode::Right) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_frame(1);
            }
        }
        if (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace))
            && self
                .animation_studio
                .document
                .as_mut()
                .is_some_and(|document| document.delete_selected_frame())
        {
            self.status_message = "Deleted selected animation frame".to_string();
        }
        if is_key_pressed(KeyCode::A) {
            self.add_selected_animation_source_frame();
        }
        if is_key_pressed(KeyCode::P) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Pivot;
            self.status_message = "Click the clip preview to place the frame pivot".to_string();
        }
        if is_key_pressed(KeyCode::K) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Socket;
            self.status_message = format!(
                "Click the clip preview to place the {} socket",
                self.animation_studio.selected_socket_kind().label()
            );
        }
        if is_key_pressed(KeyCode::H) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Shadow;
            self.status_message = "Click the clip preview to place the shadow anchor".to_string();
        }
        if is_key_pressed(KeyCode::E) {
            self.open_selected_animation_frame_in_pixel_studio();
            return;
        }
        if is_key_pressed(KeyCode::Escape) {
            self.animation_studio.placement_mode = AnimationPlacementMode::None;
            self.animation_studio.playing = false;
        }
    }

    pub(crate) fn handle_animation_studio_click(&mut self, mx: f32, my: f32) -> bool {
        if self.viewport_mode != EditorViewportMode::AnimationStudio {
            return false;
        }
        let mouse = vec2(mx, my);
        let list = self.shell_layout().list_content;
        if list.contains(mouse) {
            if animation_refresh_rect(list).contains(mouse) {
                let count = self.animation_studio.refresh_library();
                self.status_message = format!("Rescanned {count} animation source sheets");
                return true;
            }
            if animation_open_rect(list).contains(mouse) {
                self.open_selected_animation_library_entry();
                return true;
            }
            let visible = ((list.h - 92.0) / 44.0).floor().max(1.0) as usize;
            for row in 0..visible {
                if animation_library_row_rect(list, row).contains(mouse) {
                    let index = self.animation_studio.library_offset + row;
                    if index < self.animation_studio.library.len() {
                        self.animation_studio.selected_entry = index;
                        self.open_selected_animation_library_entry();
                    }
                    return true;
                }
            }
            return true;
        }

        let host = self.canvas_host_rect();
        let toolbar = animation_toolbar_rect(host);
        if toolbar.contains(mouse) && self.handle_animation_toolbar_click(mouse, toolbar) {
            return true;
        }
        let source = animation_source_rect(host);
        if source.contains(mouse) && self.select_animation_source_cell(mouse, source) {
            return true;
        }
        let preview = animation_preview_rect(host);
        if preview.contains(mouse) && self.place_animation_point(mouse, preview) {
            return true;
        }
        let timeline = animation_timeline_rect(host);
        if timeline.contains(mouse) {
            let frame_count = self
                .animation_studio
                .document
                .as_ref()
                .and_then(|document| document.clip())
                .map_or(0, |clip| clip.frames.len());
            for index in 0..frame_count.min(12) {
                if animation_timeline_frame_rect(timeline, index).contains(mouse) {
                    if let Some(document) = self.animation_studio.document.as_mut() {
                        document.selected_frame = index;
                    }
                    self.animation_studio.playing = false;
                    self.status_message = format!("Selected timeline frame {}", index + 1);
                    return true;
                }
            }
            return true;
        }

        let inspector = self.inspector_content_rect();
        if inspector.contains(mouse) && self.handle_animation_inspector_click(mouse, inspector) {
            return true;
        }
        false
    }

    fn open_selected_animation_library_entry(&mut self) {
        self.status_message = match self.animation_studio.load_selected() {
            Ok(message) => message,
            Err(error) => format!("Animation source open failed: {error}"),
        };
    }

    fn handle_animation_toolbar_click(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if animation_play_rect(rect).contains(mouse) {
            self.animation_studio.playing = !self.animation_studio.playing;
            self.animation_studio.playback_elapsed_ms = 0.0;
            return true;
        }
        if animation_prev_frame_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_frame(-1);
            }
            self.animation_studio.playing = false;
            return true;
        }
        if animation_next_frame_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_frame(1);
            }
            self.animation_studio.playing = false;
            return true;
        }
        if animation_add_frame_rect(rect).contains(mouse) {
            self.add_selected_animation_source_frame();
            return true;
        }
        if animation_duplicate_frame_rect(rect).contains(mouse) {
            let changed = self
                .animation_studio
                .document
                .as_mut()
                .is_some_and(|document| document.duplicate_selected_frame());
            self.status_message = if changed {
                "Duplicated selected animation frame"
            } else {
                "No animation frame is selected"
            }
            .to_string();
            return true;
        }
        if animation_delete_frame_rect(rect).contains(mouse) {
            let changed = self
                .animation_studio
                .document
                .as_mut()
                .is_some_and(|document| document.delete_selected_frame());
            self.status_message = if changed {
                "Deleted selected animation frame"
            } else {
                "No animation frame is selected"
            }
            .to_string();
            return true;
        }
        if animation_move_left_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.move_selected_frame(-1);
            }
            return true;
        }
        if animation_move_right_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.move_selected_frame(1);
            }
            return true;
        }
        if animation_onion_rect(rect).contains(mouse) {
            self.animation_studio.onion_skin = !self.animation_studio.onion_skin;
            return true;
        }
        if animation_grid_rect(rect).contains(mouse) {
            self.animation_studio.show_source_grid = !self.animation_studio.show_source_grid;
            return true;
        }
        false
    }

    fn select_animation_source_cell(&mut self, mouse: Vec2, rect: Rect) -> bool {
        let Some(document) = self.animation_studio.document.as_ref() else {
            return true;
        };
        let content = Rect::new(rect.x + 8.0, rect.y + 32.0, rect.w - 16.0, rect.h - 40.0);
        let image_rect = fit_image_rect(
            content,
            document.metadata.image_width,
            document.metadata.image_height,
            1.0,
        );
        if !image_rect.contains(mouse) {
            return true;
        }
        let image_x = ((mouse.x - image_rect.x) / image_rect.w
            * document.metadata.image_width as f32)
            .floor() as i32;
        let image_y = ((mouse.y - image_rect.y) / image_rect.h
            * document.metadata.image_height as f32)
            .floor() as i32;
        let relative_x = image_x - document.metadata.grid_offset[0];
        let relative_y = image_y - document.metadata.grid_offset[1];
        if relative_x < 0 || relative_y < 0 {
            return true;
        }
        let column = relative_x as u32 / document.metadata.frame_width.max(1);
        let row = relative_y as u32 / document.metadata.frame_height.max(1);
        self.animation_studio.source_cell = [column, row];
        self.status_message = format!("Selected source cell {}, {}", column, row);
        true
    }

    fn place_animation_point(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if self.animation_studio.placement_mode == AnimationPlacementMode::None {
            return true;
        }
        let Some(document) = self.animation_studio.document.as_ref() else {
            return true;
        };
        let Some(frame) = document.frame() else {
            return true;
        };
        let content = Rect::new(rect.x + 8.0, rect.y + 32.0, rect.w - 16.0, rect.h - 40.0);
        let destination = fit_image_rect(content, frame.source.width, frame.source.height, 4.0);
        if !destination.contains(mouse) {
            return true;
        }
        let local_x = (((mouse.x - destination.x) / destination.w) * frame.source.width as f32)
            .floor()
            .clamp(0.0, frame.source.width.saturating_sub(1) as f32) as i32;
        let local_y = (((mouse.y - destination.y) / destination.h) * frame.source.height as f32)
            .floor()
            .clamp(0.0, frame.source.height.saturating_sub(1) as f32) as i32;
        let placement = self.animation_studio.placement_mode;
        let socket_kind = self.animation_studio.selected_socket_kind();
        if let Some(document) = self.animation_studio.document.as_mut() {
            if let Some(frame) = document.frame_mut() {
                match placement {
                    AnimationPlacementMode::Pivot => frame.pivot = [local_x, local_y],
                    AnimationPlacementMode::Shadow => {
                        frame.shadow_offset = [local_x - frame.pivot[0], local_y - frame.pivot[1]];
                    }
                    AnimationPlacementMode::Socket => {
                        frame.set_socket(socket_kind, [local_x, local_y]);
                    }
                    AnimationPlacementMode::None => {}
                }
                document.dirty = true;
            }
        }
        self.animation_studio.placement_mode = AnimationPlacementMode::None;
        self.status_message = match placement {
            AnimationPlacementMode::Pivot => format!("Placed frame pivot at {local_x}, {local_y}"),
            AnimationPlacementMode::Shadow => {
                format!("Placed shadow anchor at {local_x}, {local_y}")
            }
            AnimationPlacementMode::Socket => format!(
                "Placed {} socket at {local_x}, {local_y}",
                socket_kind.label()
            ),
            AnimationPlacementMode::None => String::new(),
        };
        true
    }

    fn handle_animation_inspector_click(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if animation_clip_prev_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_clip(-1);
            }
            return true;
        }
        if animation_clip_next_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.cycle_clip(1);
            }
            return true;
        }
        if animation_clip_add_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                document.add_clip();
                self.status_message = "Added a new animation clip".to_string();
            }
            return true;
        }
        if animation_clip_delete_rect(rect).contains(mouse) {
            let changed = self
                .animation_studio
                .document
                .as_mut()
                .is_some_and(|document| document.delete_selected_clip());
            self.status_message = if changed {
                "Deleted selected animation clip"
            } else {
                "At least one clip must remain"
            }
            .to_string();
            return true;
        }
        if animation_direction_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                if let Some(clip) = document.clip_mut() {
                    let current = AnimationDirection::ALL
                        .iter()
                        .position(|direction| *direction == clip.direction)
                        .unwrap_or(0);
                    clip.direction =
                        AnimationDirection::ALL[(current + 1) % AnimationDirection::ALL.len()];
                    document.dirty = true;
                }
            }
            return true;
        }
        if animation_loop_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                if let Some(clip) = document.clip_mut() {
                    let current = AnimationLoopMode::ALL
                        .iter()
                        .position(|mode| *mode == clip.loop_mode)
                        .unwrap_or(0);
                    clip.loop_mode =
                        AnimationLoopMode::ALL[(current + 1) % AnimationLoopMode::ALL.len()];
                    document.dirty = true;
                }
            }
            return true;
        }
        if animation_profile_rect(rect).contains(mouse) {
            let count = self
                .animation_studio
                .document
                .as_mut()
                .map_or(0, |document| document.apply_recommended_profile());
            self.status_message = format!("Applied recommended sheet profile: {count} clips");
            return true;
        }
        if animation_slice_row_rect(rect).contains(mouse) {
            let row = self.animation_studio.source_cell[1];
            let count = self
                .animation_studio
                .document
                .as_mut()
                .map_or(0, |document| document.auto_slice_selected_row(row));
            self.status_message = format!("Sliced source row {row} into {count} frames");
            return true;
        }
        if animation_frame_width_down_rect(rect).contains(mouse) {
            self.adjust_animation_source_frame_size(-32, 0);
            return true;
        }
        if animation_frame_width_up_rect(rect).contains(mouse) {
            self.adjust_animation_source_frame_size(32, 0);
            return true;
        }
        if animation_frame_height_down_rect(rect).contains(mouse) {
            self.adjust_animation_source_frame_size(0, -32);
            return true;
        }
        if animation_frame_height_up_rect(rect).contains(mouse) {
            self.adjust_animation_source_frame_size(0, 32);
            return true;
        }
        if animation_duration_down_rect(rect).contains(mouse) {
            self.adjust_animation_frame_duration(-25);
            return true;
        }
        if animation_duration_up_rect(rect).contains(mouse) {
            self.adjust_animation_frame_duration(25);
            return true;
        }
        if animation_pivot_mode_rect(rect).contains(mouse) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Pivot;
            self.status_message = "Click the preview to place the pivot".to_string();
            return true;
        }
        if animation_pivot_bottom_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                if let Some(frame) = document.frame_mut() {
                    frame.pivot = [
                        frame.source.width as i32 / 2,
                        frame.source.height.saturating_sub(4) as i32,
                    ];
                    document.dirty = true;
                    self.status_message = "Set bottom-center foot pivot".to_string();
                }
            }
            return true;
        }
        if animation_shadow_mode_rect(rect).contains(mouse) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Shadow;
            self.status_message = "Click the preview to place the shadow anchor".to_string();
            return true;
        }
        if animation_shadow_reset_rect(rect).contains(mouse) {
            if let Some(document) = self.animation_studio.document.as_mut() {
                if let Some(frame) = document.frame_mut() {
                    frame.shadow_offset = [0, 0];
                    document.dirty = true;
                    self.status_message = "Reset shadow to the frame pivot".to_string();
                }
            }
            return true;
        }
        if animation_socket_prev_rect(rect).contains(mouse) {
            self.animation_studio.socket_kind_index = self
                .animation_studio
                .socket_kind_index
                .checked_sub(1)
                .unwrap_or(AnimationSocketKind::ALL.len() - 1);
            return true;
        }
        if animation_socket_next_rect(rect).contains(mouse) {
            self.animation_studio.socket_kind_index =
                (self.animation_studio.socket_kind_index + 1) % AnimationSocketKind::ALL.len();
            return true;
        }
        if animation_socket_mode_rect(rect).contains(mouse) {
            self.animation_studio.placement_mode = AnimationPlacementMode::Socket;
            self.status_message = format!(
                "Click the preview to place the {} socket",
                self.animation_studio.selected_socket_kind().label()
            );
            return true;
        }
        if animation_event_prev_rect(rect).contains(mouse) {
            self.animation_studio.event_kind_index = self
                .animation_studio
                .event_kind_index
                .checked_sub(1)
                .unwrap_or(AnimationEventKind::ALL.len() - 1);
            return true;
        }
        if animation_event_next_rect(rect).contains(mouse) {
            self.animation_studio.event_kind_index =
                (self.animation_studio.event_kind_index + 1) % AnimationEventKind::ALL.len();
            return true;
        }
        if animation_event_toggle_rect(rect).contains(mouse) {
            let event = self.animation_studio.selected_event_kind();
            let enabled = self
                .animation_studio
                .document
                .as_mut()
                .and_then(|document| document.toggle_selected_event(event));
            self.status_message = match enabled {
                Some(true) => format!("Added {} event to selected frame", event.label()),
                Some(false) => format!("Removed {} event from selected frame", event.label()),
                None => "No animation frame is selected".to_string(),
            };
            return true;
        }
        if animation_edit_pixels_rect(rect).contains(mouse) {
            self.open_selected_animation_frame_in_pixel_studio();
            return true;
        }
        if animation_save_rect(rect).contains(mouse) {
            self.save_animation_document();
            return true;
        }
        if animation_publish_rect(rect).contains(mouse) {
            self.publish_animation_document();
            return true;
        }
        false
    }

    fn add_selected_animation_source_frame(&mut self) {
        let Some(source) = self.animation_studio.selected_source() else {
            self.status_message = "Selected source cell is outside the image".to_string();
            return;
        };
        if let Some(document) = self.animation_studio.document.as_mut() {
            document.add_frame(source);
            self.animation_studio.playing = false;
            self.status_message = format!(
                "Added source cell {}, {} to the selected clip",
                self.animation_studio.source_cell[0], self.animation_studio.source_cell[1]
            );
        }
    }

    fn adjust_animation_source_frame_size(&mut self, width_delta: i32, height_delta: i32) {
        let Some(document) = self.animation_studio.document.as_mut() else {
            return;
        };
        document.metadata.frame_width =
            (document.metadata.frame_width as i32 + width_delta).clamp(1, 1024) as u32;
        document.metadata.frame_height =
            (document.metadata.frame_height as i32 + height_delta).clamp(1, 1024) as u32;
        document.dirty = true;
        self.animation_studio.source_cell = [0, 0];
        self.animation_studio.playing = false;
        self.status_message = format!(
            "Animation source frame: {}x{}",
            document.metadata.frame_width, document.metadata.frame_height
        );
    }

    fn adjust_animation_frame_duration(&mut self, delta_ms: i32) {
        if let Some(document) = self.animation_studio.document.as_mut() {
            let duration = document.frame_mut().map(|frame| {
                frame.duration_ms = (frame.duration_ms as i32 + delta_ms).clamp(25, 5000) as u32;
                frame.duration_ms
            });
            if let Some(duration) = duration {
                document.dirty = true;
                self.status_message = format!("Frame duration: {duration} ms");
            }
        }
    }
}
