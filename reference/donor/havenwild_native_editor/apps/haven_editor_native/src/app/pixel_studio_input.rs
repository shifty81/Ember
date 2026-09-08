use super::pixel_color_panel::*;
use super::pixel_layer_panel::*;
use super::pixel_new_document::*;
use super::pixel_studio::*;
use super::pixel_studio_layout::*;
use super::render_helpers::cycle_index;
use super::sprite_workspace::*;
use super::*;
use haven_assets::asset_intake::{repo_root_dir, AssetIntakeTargetKind};
use haven_pixel::{PixelDocumentKind, PixelSelection, PixelTool};

impl EditorApp {
    pub(crate) fn update_pixel_studio_navigation(&mut self) -> bool {
        if self.viewport_mode != EditorViewportMode::PixelStudio {
            return false;
        }
        if self.pixel_studio.new_dialog.is_some() {
            return true;
        }
        let canvas = self.pixel_canvas_rect();
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let library_rect = self.shell_layout().list_content;
        if self.update_pixel_library_navigation(library_rect, mouse) {
            return true;
        }
        let inspector = self.inspector_content_rect();
        if self.pixel_studio.inspector_tab == PixelInspectorTab::Layers && inspector.contains(mouse)
        {
            let wheel = mouse_wheel().1;
            if wheel.abs() > 0.05 {
                let maximum = self
                    .pixel_studio
                    .document
                    .as_ref()
                    .map(|document| document.layer_count().saturating_sub(8))
                    .unwrap_or_default();
                if wheel > 0.0 {
                    self.pixel_studio.layer_offset =
                        self.pixel_studio.layer_offset.saturating_sub(1);
                } else {
                    self.pixel_studio.layer_offset =
                        (self.pixel_studio.layer_offset + 1).min(maximum);
                }
                return true;
            }
        }
        if is_key_pressed(KeyCode::X) {
            std::mem::swap(
                &mut self.pixel_studio.selected_color,
                &mut self.pixel_studio.background_color,
            );
            self.status_message = "Swapped foreground and background colors".to_string();
            return true;
        }
        if is_key_pressed(KeyCode::D) {
            self.pixel_studio.selected_color = [0, 0, 0, 255];
            self.pixel_studio.background_color = [255, 255, 255, 255];
            self.status_message = "Reset foreground/background colors".to_string();
            return true;
        }
        if is_key_pressed(KeyCode::F) {
            self.pixel_studio.frame_document(canvas);
            self.status_message = "Framed active pixel document".to_string();
            return true;
        }
        if canvas.contains(mouse) {
            let wheel = mouse_wheel().1;
            if wheel.abs() > 0.05 {
                let before = self.pixel_studio.screen_to_pixel(canvas, mouse);
                if wheel > 0.0 {
                    self.pixel_studio.zoom_in();
                } else {
                    self.pixel_studio.zoom_out();
                }
                if let (Some(pixel), Some(image)) = (before, self.pixel_studio.image_rect(canvas)) {
                    let after_screen = vec2(
                        image.x + (pixel.0 as f32 + 0.5) * self.pixel_studio.zoom(),
                        image.y + (pixel.1 as f32 + 0.5) * self.pixel_studio.zoom(),
                    );
                    self.pixel_studio.pan += mouse - after_screen;
                }
                self.status_message = format!("Pixel canvas zoom {:.3}x", self.pixel_studio.zoom());
                return true;
            }
            let pan_down = is_mouse_button_down(MouseButton::Middle)
                || (is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left));
            if pan_down {
                let previous = self.pixel_studio.pan_drag.unwrap_or(mouse);
                self.pixel_studio.pan += mouse - previous;
                self.pixel_studio.pan_drag = Some(mouse);
                return true;
            }
        }
        if !(is_mouse_button_down(MouseButton::Middle)
            || is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left))
        {
            self.pixel_studio.pan_drag = None;
        }
        false
    }

    pub(crate) fn update_pixel_studio_input(&mut self) {
        if self.viewport_mode != EditorViewportMode::PixelStudio {
            return;
        }
        if !self.pixel_studio.library_loaded {
            self.status_message = match self.pixel_studio.refresh_library() {
                Ok(count) => format!("Pixel Studio indexed {count} project assets"),
                Err(error) => format!("Pixel Studio library scan failed: {error}"),
            };
        }
        if self.handle_new_pixel_dialog_input() {
            return;
        }
        if self.handle_pixel_layer_rename_input() {
            return;
        }
        if let Some(Err(error)) = self.pixel_studio.update_autosave() {
            self.pixel_studio.autosave_status = format!("Autosave failed: {error}");
            self.status_message = self.pixel_studio.autosave_status.clone();
        }
        let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if control && is_key_pressed(KeyCode::N) {
            self.pixel_studio.open_new_dialog();
            self.status_message = "New Pixel Studio asset".to_string();
            return;
        }
        if control && is_key_pressed(KeyCode::O) {
            self.status_message = match self.pixel_studio.refresh_library() {
                Ok(count) if count > 0 => {
                    format!("Choose an asset from the Pixel Assets list ({count} available)")
                }
                Ok(_) => "No editable project assets were found".to_string(),
                Err(error) => format!("Pixel Studio open scan failed: {error}"),
            };
            return;
        }
        if control && is_key_pressed(KeyCode::S) {
            self.save_pixel_document();
            return;
        }
        if control && is_key_pressed(KeyCode::Enter) {
            if self.pixel_studio.world_asset_context.is_some() {
                self.save_world_asset_pixels_and_return();
                return;
            }
            if self.pixel_studio.animation_context.is_some() {
                self.save_animation_pixels_and_return();
                return;
            }
        }
        if control
            && is_key_pressed(KeyCode::Z)
            && self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.undo())
        {
            self.pixel_studio.refresh_texture();
            self.status_message = "Pixel edit undone".to_string();
        }
        if control
            && (is_key_pressed(KeyCode::Y)
                || (is_key_down(KeyCode::LeftShift) && is_key_pressed(KeyCode::Z)))
            && self
                .pixel_studio
                .document
                .as_mut()
                .is_some_and(|document| document.redo())
        {
            self.pixel_studio.refresh_texture();
            self.status_message = "Pixel edit redone".to_string();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.pixel_studio.grid_realign_armed = false;
            self.pixel_studio.grid_realign_enabled = false;
            self.pixel_studio.drag_start = None;
            self.pixel_studio.drag_current = None;
            self.pixel_studio.stroke_started = false;
            self.status_message = "Cancelled Pixel Studio gesture / grid realignment".to_string();
        }
        if is_key_pressed(KeyCode::LeftBracket) {
            self.pixel_studio.zoom_out();
        }
        if is_key_pressed(KeyCode::RightBracket) {
            self.pixel_studio.zoom_in();
        }
        if is_key_pressed(KeyCode::Up) {
            self.cycle_pixel_library(-1);
        }
        if is_key_pressed(KeyCode::Down) {
            self.cycle_pixel_library(1);
        }
        if is_key_pressed(KeyCode::Enter) && self.pixel_studio.document.is_none() {
            self.open_selected_pixel_library_entry();
        }

        let canvas = self.pixel_canvas_rect();
        let mouse = vec2(mouse_position().0, mouse_position().1);
        let current = self.pixel_studio.screen_to_pixel(canvas, mouse);
        if (is_mouse_button_down(MouseButton::Left) || is_mouse_button_down(MouseButton::Right))
            && self.pixel_studio.stroke_started
            && !is_key_down(KeyCode::Space)
        {
            if let Some(pixel) = current {
                if self.pixel_studio.grid_realign_enabled {
                    if let (Some(start), Some(origin), Some(document)) = (
                        self.pixel_studio.drag_start,
                        self.pixel_studio.grid_drag_origin,
                        self.pixel_studio.document.as_mut(),
                    ) {
                        document.metadata.grid.offset_x =
                            origin.0 + pixel.0 as i32 - start.0 as i32;
                        document.metadata.grid.offset_y =
                            origin.1 + pixel.1 as i32 - start.1 as i32;
                        document.dirty = true;
                    }
                } else {
                    let active_tool = if is_mouse_button_down(MouseButton::Right) {
                        self.pixel_studio.secondary_tool
                    } else {
                        self.pixel_studio.tool
                    };
                    match active_tool {
                        PixelTool::Pencil | PixelTool::Eraser => {
                            let previous = self.pixel_studio.drag_current.unwrap_or(pixel);
                            if previous != pixel {
                                self.apply_pixel_brush_segment(previous, pixel);
                                self.pixel_studio.drag_current = Some(pixel);
                            }
                        }
                        PixelTool::Selection | PixelTool::Line | PixelTool::Rectangle => {
                            self.pixel_studio.drag_current = Some(pixel);
                        }
                        _ => {}
                    }
                }
            }
        }
        if (is_mouse_button_released(MouseButton::Left)
            || is_mouse_button_released(MouseButton::Right))
            && self.pixel_studio.stroke_started
        {
            self.finish_pixel_gesture(current);
        }
    }

    pub(crate) fn handle_pixel_studio_click(&mut self, mx: f32, my: f32) -> bool {
        if self.viewport_mode != EditorViewportMode::PixelStudio {
            return false;
        }
        let mouse = vec2(mx, my);
        if self.pixel_studio.new_dialog.is_some() {
            return self.handle_new_pixel_dialog_click(mouse);
        }
        let list = self.shell_layout().list_content;
        if list.contains(mouse) {
            return self.handle_pixel_library_click(mouse, list);
        }

        let canvas = self.pixel_canvas_rect();
        let toolbar = Rect::new(canvas.x, canvas.y - 44.0, canvas.w, 36.0);
        if toolbar.contains(mouse) {
            for (index, tool) in PixelTool::ALL.into_iter().enumerate() {
                if pixel_tool_rect(toolbar, index).contains(mouse) {
                    if is_mouse_button_down(MouseButton::Right) {
                        self.pixel_studio.secondary_tool = tool;
                        self.status_message = format!("Secondary pixel tool: {}", tool.label());
                    } else {
                        self.pixel_studio.tool = tool;
                        self.status_message = format!("Primary pixel tool: {}", tool.label());
                    }
                    return true;
                }
            }
            if pixel_zoom_out_rect(toolbar).contains(mouse) {
                self.pixel_studio.zoom_out();
                return true;
            }
            if pixel_zoom_in_rect(toolbar).contains(mouse) {
                self.pixel_studio.zoom_in();
                return true;
            }
            if pixel_frame_rect(toolbar).contains(mouse) {
                self.pixel_studio.frame_document(canvas);
                return true;
            }
            if pixel_selection_mode_rect(toolbar).contains(mouse) {
                self.pixel_studio.selection_mode = self.pixel_studio.selection_mode.cycle();
                self.pixel_studio.tool = PixelTool::Selection;
                self.status_message = format!(
                    "Selection mode: {}",
                    self.pixel_studio.selection_mode.label()
                );
                return true;
            }
            if pixel_atlas_grid_rect(toolbar).contains(mouse) {
                self.pixel_studio.show_atlas_grid = !self.pixel_studio.show_atlas_grid;
                self.status_message = format!(
                    "Frame grid {}",
                    if self.pixel_studio.show_atlas_grid {
                        "visible"
                    } else {
                        "hidden"
                    }
                );
                return true;
            }
            if pixel_pixel_grid_rect(toolbar).contains(mouse) {
                self.pixel_studio.show_pixel_grid = !self.pixel_studio.show_pixel_grid;
                return true;
            }
        }

        let inspector = self.inspector_content_rect();
        if inspector.contains(mouse) && self.handle_pixel_inspector_click(mouse, inspector) {
            return true;
        }

        let bottom = Rect::new(
            canvas.x,
            canvas.y + canvas.h - SPRITE_BOTTOM_DOCK_HEIGHT,
            canvas.w,
            SPRITE_BOTTOM_DOCK_HEIGHT,
        );
        if bottom.contains(mouse) {
            for (index, color) in super::pixel_studio::PALETTE.into_iter().enumerate() {
                if sprite_bottom_swatch_rect(bottom, index).contains(mouse) {
                    if is_mouse_button_down(MouseButton::Right) {
                        self.pixel_studio.background_color = color;
                        self.status_message =
                            format!("Background color set from palette slot {}", index + 1);
                    } else {
                        self.pixel_studio.selected_color = color;
                        self.status_message =
                            format!("Foreground color set from palette slot {}", index + 1);
                    }
                    return true;
                }
            }
            return true;
        }

        if canvas.contains(mouse) {
            let Some(pixel) = self.pixel_studio.screen_to_pixel(canvas, mouse) else {
                return true;
            };
            let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
            if alt_down {
                if let Some(document) = self.pixel_studio.document.as_mut() {
                    document.begin_edit();
                    document.metadata.pivot = [pixel.0 as i32, pixel.1 as i32];
                    document.dirty = true;
                    self.status_message = format!("Pivot set to {}, {}", pixel.0, pixel.1);
                }
                return true;
            }
            let active_tool = if is_mouse_button_down(MouseButton::Right) {
                self.pixel_studio.secondary_tool
            } else {
                self.pixel_studio.tool
            };
            if active_tool == PixelTool::Selection
                && self.pixel_studio.selection_mode == PixelSelectionMode::Frame
            {
                if let Some(document) = self.pixel_studio.document.as_mut() {
                    let selection = grid_cell_selection(document, pixel);
                    document.begin_edit();
                    document.metadata.selection = selection;
                    document.dirty = true;
                    self.status_message = format!(
                        "Selected frame {}x{} at {}, {}",
                        selection.width, selection.height, selection.x, selection.y
                    );
                }
                return true;
            }
            if self.pixel_studio.grid_realign_enabled {
                if let Some(document) = self.pixel_studio.document.as_mut() {
                    document.begin_edit();
                }
                self.pixel_studio.drag_start = Some(pixel);
                self.pixel_studio.drag_current = Some(pixel);
                self.pixel_studio.grid_drag_origin =
                    self.pixel_studio.document.as_ref().map(|document| {
                        (
                            document.metadata.grid.offset_x,
                            document.metadata.grid.offset_y,
                        )
                    });
                self.pixel_studio.stroke_started = true;
                return true;
            }
            match active_tool {
                PixelTool::Pencil | PixelTool::Eraser => {
                    if let Some(document) = self.pixel_studio.document.as_mut() {
                        document.begin_edit();
                    }
                    self.pixel_studio.drag_start = Some(pixel);
                    self.pixel_studio.drag_current = Some(pixel);
                    self.pixel_studio.stroke_started = true;
                    self.apply_pixel_brush_segment(pixel, pixel);
                }
                PixelTool::Fill => {
                    let color = if is_mouse_button_down(MouseButton::Right) {
                        self.pixel_studio.background_color
                    } else {
                        self.pixel_studio.selected_color
                    };
                    if let Some(document) = self.pixel_studio.document.as_mut() {
                        document.begin_edit();
                        let changed = document.flood_fill(pixel.0, pixel.1, color);
                        self.status_message = format!("Filled {changed} pixels");
                    }
                    self.pixel_studio.refresh_texture();
                }
                PixelTool::Eyedropper => {
                    if let Some(document) = self.pixel_studio.document.as_ref() {
                        self.pixel_studio.selected_color = document.color_at(pixel.0, pixel.1);
                        self.status_message = format!("Picked color at {}, {}", pixel.0, pixel.1);
                    }
                }
                PixelTool::Selection | PixelTool::Line | PixelTool::Rectangle => {
                    if let Some(document) = self.pixel_studio.document.as_mut() {
                        document.begin_edit();
                    }
                    self.pixel_studio.drag_start = Some(pixel);
                    self.pixel_studio.drag_current = Some(pixel);
                    self.pixel_studio.stroke_started = true;
                }
            }
            return true;
        }
        false
    }

    fn handle_pixel_inspector_click(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if self.pixel_studio.document.is_none() {
            return false;
        }
        if pixel_layer_tab_rect(rect).contains(mouse) {
            self.pixel_studio.inspector_tab = PixelInspectorTab::Layers;
            return true;
        }
        if pixel_asset_tab_rect(rect).contains(mouse) {
            self.pixel_studio.inspector_tab = PixelInspectorTab::Asset;
            return true;
        }
        if pixel_animation_tab_rect(rect).contains(mouse) {
            self.pixel_studio.inspector_tab = PixelInspectorTab::Animation;
            return true;
        }
        if pixel_color_tab_rect(rect).contains(mouse) {
            self.pixel_studio.inspector_tab = PixelInspectorTab::Color;
            return true;
        }
        if self.pixel_studio.inspector_tab == PixelInspectorTab::Color {
            if update_color_from_pointer(self, mouse, rect) {
                return true;
            }
            if color_swap_rect(rect).contains(mouse) {
                std::mem::swap(
                    &mut self.pixel_studio.selected_color,
                    &mut self.pixel_studio.background_color,
                );
                return true;
            }
            if color_reset_rect(rect).contains(mouse) {
                self.pixel_studio.selected_color = [0, 0, 0, 255];
                self.pixel_studio.background_color = [255, 255, 255, 255];
                return true;
            }
            for channel in 0..4 {
                if color_channel_minus_rect(rect, channel).contains(mouse) {
                    self.pixel_studio.selected_color[channel] =
                        self.pixel_studio.selected_color[channel].saturating_sub(1);
                    return true;
                }
                if color_channel_plus_rect(rect, channel).contains(mouse) {
                    self.pixel_studio.selected_color[channel] =
                        self.pixel_studio.selected_color[channel].saturating_add(1);
                    return true;
                }
            }
            for (index, color) in super::pixel_studio::PALETTE.into_iter().enumerate() {
                if color_palette_rect(rect, index).contains(mouse) {
                    if is_mouse_button_down(MouseButton::Right) {
                        self.pixel_studio.background_color = color;
                    } else {
                        self.pixel_studio.selected_color = color;
                    }
                    return true;
                }
            }
            return true;
        }
        if self.pixel_studio.inspector_tab == PixelInspectorTab::Layers {
            return self.handle_pixel_layer_panel_click(mouse, rect);
        }
        if self.pixel_studio.inspector_tab == PixelInspectorTab::Animation {
            if pixel_animation_onion_rect(rect).contains(mouse) {
                if let Some(context) = self.pixel_studio.animation_context.as_mut() {
                    context.onion_skin = !context.onion_skin;
                    self.status_message = format!(
                        "Animation onion skin {}",
                        if context.onion_skin {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                }
                return true;
            }
            if pixel_animation_focus_rect(rect).contains(mouse) {
                let canvas = self.pixel_canvas_rect();
                self.pixel_studio.frame_selection(canvas);
                self.status_message = "Focused the active animation frame".to_string();
                return true;
            }
            if pixel_animation_save_return_rect(rect).contains(mouse) {
                self.save_animation_pixels_and_return();
                return true;
            }
            if pixel_animation_cancel_return_rect(rect).contains(mouse) {
                self.return_to_animation_without_pixel_save();
                return true;
            }
            return true;
        }
        if pixel_grid_toggle_rect(rect).contains(mouse) {
            self.pixel_studio.show_atlas_grid = !self.pixel_studio.show_atlas_grid;
            return true;
        }
        if pixel_realign_rect(rect).contains(mouse) {
            if self.pixel_studio.grid_realign_enabled {
                self.pixel_studio.grid_realign_enabled = false;
                self.pixel_studio.grid_realign_armed = false;
                self.status_message = "Atlas Grid Realignment Mode disabled".to_string();
            } else if self.pixel_studio.grid_realign_armed {
                self.pixel_studio.grid_realign_enabled = true;
                self.pixel_studio.grid_realign_armed = false;
                self.status_message = "Atlas Grid Realignment Mode enabled. Drag the logical grid; pixels will not move. Esc cancels.".to_string();
            } else {
                self.pixel_studio.grid_realign_armed = true;
                self.status_message = "Grid realignment is non-destructive metadata editing. Click Confirm Realign to enable.".to_string();
            }
            return true;
        }
        for row in 0..6 {
            if pixel_value_minus_rect(rect, row).contains(mouse) {
                self.adjust_pixel_grid(row, -1);
                return true;
            }
            if pixel_value_plus_rect(rect, row).contains(mouse) {
                self.adjust_pixel_grid(row, 1);
                return true;
            }
        }
        if pixel_flip_h_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.flip_selection_horizontal();
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_flip_v_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                document.flip_selection_vertical();
            }
            self.pixel_studio.refresh_texture();
            return true;
        }
        if pixel_pivot_bottom_rect(rect).contains(mouse) {
            if let Some(document) = self.pixel_studio.document.as_mut() {
                let selection = document.metadata.selection;
                document.begin_edit();
                document.metadata.pivot = [
                    selection.x as i32 + selection.width as i32 / 2,
                    selection.y as i32 + selection.height.saturating_sub(1) as i32,
                ];
                document.dirty = true;
                self.status_message = "Pivot moved to selected slice bottom-center".to_string();
            }
            return true;
        }
        if pixel_fit_visual_rect(rect).contains(mouse) {
            let reset_source = self.pixel_studio.world_asset_context.is_some()
                && self
                    .pixel_studio
                    .document
                    .as_ref()
                    .and_then(|document| document.metadata.source_region)
                    .is_some();
            if reset_source {
                let result = self
                    .pixel_studio
                    .document
                    .as_mut()
                    .expect("pixel document checked above")
                    .reset_to_source_region(repo_root_dir());
                self.status_message = match result {
                    Ok(()) => {
                        self.pixel_studio.refresh_texture();
                        self.pixel_studio.frame_document(self.pixel_canvas_rect());
                        "Reset derived working copy to the exact upstream source region".to_string()
                    }
                    Err(error) => format!("Reset to source failed: {error}"),
                };
            } else if let Some(document) = self.pixel_studio.document.as_mut() {
                let selection = document.metadata.selection;
                let tile_w = document.metadata.grid.cell_width.max(1);
                let tile_h = document.metadata.grid.cell_height.max(1);
                let width = selection.width.max(1).div_ceil(tile_w) as i32;
                let height = selection.height.max(1).div_ceil(tile_h) as i32;
                document.begin_edit();
                document.metadata.visual_footprint = [0, 1 - height, width.max(1), height.max(1)];
                document.dirty = true;
                self.status_message = format!(
                    "Visual footprint fitted to {}x{} atlas cells",
                    width.max(1),
                    height.max(1)
                );
            }
            return true;
        }
        if pixel_target_kind_rect(rect).contains(mouse) {
            self.pixel_studio.target_kind = match self.pixel_studio.target_kind {
                AssetIntakeTargetKind::Tile => AssetIntakeTargetKind::Object,
                AssetIntakeTargetKind::Object => AssetIntakeTargetKind::Tile,
            };
            self.pixel_studio.target_index = 0;
            return true;
        }
        if pixel_target_prev_rect(rect).contains(mouse) {
            self.cycle_pixel_target(-1);
            return true;
        }
        if pixel_target_next_rect(rect).contains(mouse) {
            self.cycle_pixel_target(1);
            return true;
        }
        if pixel_save_rect(rect).contains(mouse) {
            self.save_pixel_document();
            return true;
        }
        if pixel_publish_rect(rect).contains(mouse) {
            self.publish_pixel_document();
            return true;
        }
        for (index, color) in super::pixel_studio::PALETTE.into_iter().enumerate() {
            if pixel_swatch_rect(rect, index).contains(mouse) {
                self.pixel_studio.selected_color = color;
                return true;
            }
        }
        false
    }

    fn apply_pixel_brush_segment(&mut self, start: (u32, u32), end: (u32, u32)) {
        let active_tool = if is_mouse_button_down(MouseButton::Right) {
            self.pixel_studio.secondary_tool
        } else {
            self.pixel_studio.tool
        };
        let mut color = if active_tool == PixelTool::Eraser {
            [0, 0, 0, 0]
        } else if is_mouse_button_down(MouseButton::Right) {
            self.pixel_studio.background_color
        } else {
            self.pixel_studio.selected_color
        };
        if active_tool != PixelTool::Eraser {
            color[3] = ((color[3] as u16 * self.pixel_studio.brush_opacity as u16) / 255) as u8;
        }
        let changed = if let Some(document) = self.pixel_studio.document.as_mut() {
            if start == end {
                document.set_pixel(end.0, end.1, color)
            } else {
                document.draw_line(start, end, color);
                true
            }
        } else {
            false
        };
        if changed {
            // Keep the visible canvas synchronized while the mouse is held.
            self.pixel_studio.refresh_texture();
        }
    }

    fn finish_pixel_gesture(&mut self, current: Option<(u32, u32)>) {
        let start = self.pixel_studio.drag_start;
        let end = current.or(self.pixel_studio.drag_current);
        if !self.pixel_studio.grid_realign_enabled {
            if let (Some(start), Some(end), Some(document)) =
                (start, end, self.pixel_studio.document.as_mut())
            {
                match self.pixel_studio.tool {
                    PixelTool::Selection => {
                        document.metadata.selection =
                            PixelSelection::from_points(start.0, start.1, end.0, end.1);
                        document.dirty = true;
                    }
                    PixelTool::Line => {
                        document.draw_line(start, end, self.pixel_studio.selected_color)
                    }
                    PixelTool::Rectangle => document.draw_rectangle(
                        PixelSelection::from_points(start.0, start.1, end.0, end.1),
                        self.pixel_studio.selected_color,
                        false,
                    ),
                    _ => {}
                }
            }
        }
        self.pixel_studio.stroke_started = false;
        self.pixel_studio.drag_start = None;
        self.pixel_studio.drag_current = None;
        self.pixel_studio.grid_drag_origin = None;
        self.pixel_studio.refresh_texture();
    }

    fn cycle_pixel_target(&mut self, delta: i32) {
        let len = match self.pixel_studio.target_kind {
            AssetIntakeTargetKind::Tile => TileKind::ALL.len(),
            AssetIntakeTargetKind::Object => OBJECT_BRUSHES.len(),
        };
        self.pixel_studio.target_index = cycle_index(self.pixel_studio.target_index, len, delta);
    }

    fn adjust_pixel_grid(&mut self, row: usize, delta: i32) {
        let Some(document) = self.pixel_studio.document.as_mut() else {
            return;
        };
        document.begin_edit();
        match row {
            0 => {
                document.metadata.grid.cell_width =
                    (document.metadata.grid.cell_width as i32 + delta).clamp(1, 512) as u32
            }
            1 => {
                document.metadata.grid.cell_height =
                    (document.metadata.grid.cell_height as i32 + delta).clamp(1, 512) as u32
            }
            2 => {
                document.metadata.grid.offset_x =
                    (document.metadata.grid.offset_x + delta).clamp(-512, 512)
            }
            3 => {
                document.metadata.grid.offset_y =
                    (document.metadata.grid.offset_y + delta).clamp(-512, 512)
            }
            4 => {
                document.metadata.grid.spacing_x =
                    (document.metadata.grid.spacing_x as i32 + delta).clamp(0, 128) as u32
            }
            5 => {
                document.metadata.grid.spacing_y =
                    (document.metadata.grid.spacing_y as i32 + delta).clamp(0, 128) as u32
            }
            _ => return,
        }
        let selection = document.metadata.selection;
        document.metadata.selection = PixelSelection {
            x: selection.x.min(document.width().saturating_sub(1)),
            y: selection.y.min(document.height().saturating_sub(1)),
            width: selection.width.min(document.width()).max(1),
            height: selection.height.min(document.height()).max(1),
        };
        document.dirty = true;
    }
}

fn grid_cell_selection(document: &haven_pixel::PixelDocument, pixel: (u32, u32)) -> PixelSelection {
    let grid = document.metadata.grid;
    let stride_x = (grid.cell_width + grid.spacing_x).max(1) as i32;
    let stride_y = (grid.cell_height + grid.spacing_y).max(1) as i32;
    let x = grid.offset_x + (pixel.0 as i32 - grid.offset_x).div_euclid(stride_x) * stride_x;
    let y = grid.offset_y + (pixel.1 as i32 - grid.offset_y).div_euclid(stride_y) * stride_y;
    let x = x.clamp(0, document.width().saturating_sub(1) as i32) as u32;
    let y = y.clamp(0, document.height().saturating_sub(1) as i32) as u32;
    PixelSelection {
        x,
        y,
        width: grid.cell_width.min(document.width() - x).max(1),
        height: grid.cell_height.min(document.height() - y).max(1),
    }
}

include!("pixel_new_document_input.rs");
