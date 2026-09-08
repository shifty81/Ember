use super::pixel_color_panel::*;
use super::pixel_layer_panel::*;
use super::pixel_studio::*;
use super::pixel_studio_layout::*;
use super::render_helpers::*;
use super::sprite_canvas_authority::*;
use super::sprite_workspace::*;
use super::*;
use haven_assets::asset_intake::repo_root_dir;
use haven_pixel::{publish_working_copy, record_recent_pixel_document, PixelDocument, PixelTool};

impl EditorApp {
    pub(crate) fn pixel_canvas_rect(&self) -> Rect {
        let host = self.canvas_host_rect();
        Rect::new(
            host.x + 32.0,
            host.y + 76.0,
            host.w - 40.0,
            host.h - 84.0 - SPRITE_BOTTOM_DOCK_HEIGHT,
        )
    }

    pub(crate) fn draw_pixel_canvas(&mut self, host: Rect) {
        let canvas = self.pixel_canvas_rect();
        if self.pixel_studio.needs_frame {
            self.pixel_studio.frame_document(canvas);
        }
        draw_pixel_toolbar(self, canvas);
        draw_rectangle(
            canvas.x,
            canvas.y,
            canvas.w,
            canvas.h,
            Color::new(0.07, 0.08, 0.10, 1.0),
        );
        let Some(document) = self.pixel_studio.document.as_ref() else {
            draw_wrapped(
                "Choose New Asset to create a tile, tilesheet, sprite sheet, animation sheet, UI texture, object sprite, character layer, or free canvas. Existing project assets open from the library at left.",
                canvas.x + 28.0,
                canvas.y + 46.0,
                canvas.w - 56.0,
                18.0,
                TEXT,
            );
            return;
        };
        let Some(transform) = self.pixel_studio.canvas_transform(canvas) else {
            return;
        };
        let image_rect = transform.image;
        if let Some(clipped) = intersect_rect(image_rect, canvas) {
            draw_checkerboard(clipped, 28.0);
        }
        if let (Some(texture), Some(clipped)) = (
            self.pixel_studio.texture.as_ref(),
            intersect_rect(image_rect, canvas),
        ) {
            let zoom = self.pixel_studio.zoom();
            let source = Rect::new(
                (clipped.x - image_rect.x) / zoom,
                (clipped.y - image_rect.y) / zoom,
                clipped.w / zoom,
                clipped.h / zoom,
            );
            draw_texture_ex(
                texture,
                clipped.x,
                clipped.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(clipped.w, clipped.h)),
                    source: Some(source),
                    ..Default::default()
                },
            );
        }
        if let Some(border) = intersect_rect(image_rect, canvas) {
            draw_rectangle_lines(border.x, border.y, border.w, border.h, 1.0, TEXT);
        }
        let pixel_grid_style = SpriteOverlayKind::PixelGrid.style();
        if self.pixel_studio.show_pixel_grid && transform.zoom >= pixel_grid_style.minimum_zoom {
            draw_pixel_grid(document, transform);
        }
        if self.pixel_studio.show_atlas_grid {
            draw_atlas_grid(document, transform);
        }
        draw_pixel_selection(document, transform);

        let status = format!(
            "{}x{}  |  Zoom {:.0}%  |  {}",
            document.width(),
            document.height(),
            self.pixel_studio.zoom() * 100.0,
            self.pixel_studio.tool.label()
        );
        let status_w = measure_editor_text(&status, None, 12, 1.0).width + 18.0;
        let status_rect = Rect::new(
            canvas.x + 8.0,
            canvas.y + canvas.h - 28.0,
            status_w.min(canvas.w - 16.0),
            20.0,
        );
        draw_badge(status_rect, &status, true);

        let visible_pixels = transform.visible_pixel_bounds();
        draw_canvas_rulers(canvas, visible_pixels, 1.0, "Pixel");
        draw_pixel_animation_context_overlay(self, document, image_rect, canvas);
        draw_asset_preview(self, canvas);
        if let (Some(start), Some(current)) =
            (self.pixel_studio.drag_start, self.pixel_studio.drag_current)
        {
            if matches!(
                self.pixel_studio.tool,
                PixelTool::Selection | PixelTool::Line | PixelTool::Rectangle
            ) {
                let selection = haven_pixel::PixelSelection::from_points(
                    start.0, start.1, current.0, current.1,
                );
                draw_selection_outline(selection, transform, WARN);
            }
        }
        let bottom = Rect::new(
            canvas.x,
            host.y + host.h - SPRITE_BOTTOM_DOCK_HEIGHT,
            canvas.w,
            SPRITE_BOTTOM_DOCK_HEIGHT,
        );
        draw_sprite_bottom_dock(
            bottom,
            "Palette",
            "Left = foreground | Right = background",
            &PALETTE,
            self.pixel_studio.selected_color,
        );
        let options = Rect::new(bottom.x + bottom.w - 330.0, bottom.y + 36.0, 320.0, 28.0);
        draw_sprite_tool_options(
            options,
            self.pixel_studio.tool.label(),
            self.pixel_studio.secondary_tool.label(),
            self.pixel_studio.brush_opacity,
        );
    }

    pub(crate) fn draw_pixel_inspector(&self, rect: Rect) {
        let Some(document) = self.pixel_studio.document.as_ref() else {
            draw_editor_text("No pixel document open", rect.x, rect.y + 22.0, 18.0, MUTED);
            return;
        };
        draw_scissored_text(
            &format!(
                "{}{}",
                document.metadata.display_name,
                if document.dirty { " *" } else { "" }
            ),
            rect.x,
            rect.y + 20.0,
            rect.w,
            22.0,
            TEXT,
        );
        draw_editor_widget(
            pixel_layer_tab_rect(rect),
            "Layers",
            self.pixel_studio.inspector_tab == PixelInspectorTab::Layers,
        );
        draw_editor_widget(
            pixel_asset_tab_rect(rect),
            "Grid",
            self.pixel_studio.inspector_tab == PixelInspectorTab::Asset,
        );
        draw_editor_widget(
            pixel_animation_tab_rect(rect),
            "Anim",
            self.pixel_studio.inspector_tab == PixelInspectorTab::Animation,
        );
        draw_editor_widget(
            pixel_color_tab_rect(rect),
            "Color",
            self.pixel_studio.inspector_tab == PixelInspectorTab::Color,
        );
        match self.pixel_studio.inspector_tab {
            PixelInspectorTab::Layers => self.draw_pixel_layer_inspector(rect),
            PixelInspectorTab::Asset => self.draw_pixel_asset_inspector(rect),
            PixelInspectorTab::Animation => self.draw_pixel_animation_inspector(rect),
            PixelInspectorTab::Color => draw_pixel_color_panel(self, rect),
        }
    }

    fn draw_pixel_animation_inspector(&self, rect: Rect) {
        let Some(document) = self.pixel_studio.document.as_ref() else {
            return;
        };
        let Some(context) = self.pixel_studio.animation_context.as_ref() else {
            draw_editor_text("Animation Bridge", rect.x, rect.y + 96.0, 18.0, TEXT);
            draw_wrapped(
                "Open a frame from Animation Studio using Edit Selected Frame in Pixel Studio. The bridge preserves clip/frame selection and returns to the same timeline position.",
                rect.x,
                rect.y + 124.0,
                rect.w,
                15.0,
                MUTED,
            );
            return;
        };
        draw_editor_text("Animation Frame Context", rect.x, rect.y + 96.0, 18.0, TEXT);
        for (index, line) in [
            format!("Animation: {}", context.animation_display_name),
            format!("Clip: {} ({})", context.clip_label, context.direction_label),
            format!("Frame: {}", context.frame_index + 1),
            format!(
                "Source: {},{} {}x{}",
                context.frame_source.x,
                context.frame_source.y,
                context.frame_source.width,
                context.frame_source.height
            ),
            format!(
                "Sockets: {} | Pivot: {},{}",
                context.sockets.len(),
                document.metadata.pivot[0] - context.frame_source.x as i32,
                document.metadata.pivot[1] - context.frame_source.y as i32
            ),
        ]
        .into_iter()
        .enumerate()
        {
            draw_scissored_text(
                &line,
                rect.x,
                rect.y + 124.0 + index as f32 * 20.0,
                rect.w,
                14.0,
                TEXT,
            );
        }
        draw_editor_widget(
            pixel_animation_onion_rect(rect),
            if context.onion_skin {
                "Onion Skin: ON"
            } else {
                "Onion Skin: OFF"
            },
            context.onion_skin,
        );
        draw_editor_widget(pixel_animation_focus_rect(rect), "Focus Frame", false);
        draw_editor_widget(
            pixel_animation_save_return_rect(rect),
            "Save Pixels & Return to Animation",
            false,
        );
        draw_editor_widget(
            pixel_animation_cancel_return_rect(rect),
            "Return Without Saving Pixels",
            false,
        );
        draw_wrapped(
            "The active frame is outlined in green. Previous and next frames are composited over it as blue/red onion skins. Pivot, shadow, and sockets remain animation metadata and are shown as overlays.",
            rect.x,
            rect.y + 354.0,
            rect.w,
            14.0,
            MUTED,
        );
        draw_wrapped(
            &format!("Original source: {}", context.source_path_before_edit),
            rect.x,
            rect.y + 450.0,
            rect.w,
            13.0,
            MUTED,
        );
    }

    fn draw_pixel_asset_inspector(&self, rect: Rect) {
        let document = self.pixel_studio.document.as_ref().unwrap();
        let metadata = &document.metadata;
        for (index, line) in [
            format!("Image: {} x {}", document.width(), document.height()),
            format!(
                "Type: {} | Preview: {}",
                metadata.asset_kind.label(),
                metadata.preview_mode.label()
            ),
            format!(
                "Zoom: {:.3}x | Tool: {}",
                self.pixel_studio.zoom(),
                self.pixel_studio.tool.label()
            ),
            format!("License: {}", metadata.license.status),
            format!("Active layer: {}", document.active_layer().metadata.name),
            format!(
                "Grid {}x{} | origin {},{} | spacing {},{}",
                metadata.grid.cell_width,
                metadata.grid.cell_height,
                metadata.grid.offset_x,
                metadata.grid.offset_y,
                metadata.grid.spacing_x,
                metadata.grid.spacing_y
            ),
        ]
        .into_iter()
        .enumerate()
        {
            draw_scissored_text(
                &line,
                rect.x,
                rect.y + 82.0 + index as f32 * 20.0,
                rect.w,
                15.0,
                TEXT,
            );
        }

        draw_editor_text("Frame Grid", rect.x, rect.y + 210.0, 18.0, TEXT);
        draw_editor_widget(
            pixel_grid_toggle_rect(rect),
            if self.pixel_studio.show_atlas_grid {
                "Grid Visible"
            } else {
                "Grid Hidden"
            },
            self.pixel_studio.show_atlas_grid,
        );
        draw_editor_widget(
            pixel_realign_rect(rect),
            if self.pixel_studio.grid_realign_enabled {
                "Grid Realign: ON"
            } else if self.pixel_studio.grid_realign_armed {
                "Confirm Realign"
            } else {
                "Grid Realign: OFF"
            },
            self.pixel_studio.grid_realign_enabled,
        );
        draw_value_stepper(
            rect,
            rect.y + 270.0,
            "Cell W",
            metadata.grid.cell_width as i32,
            0,
        );
        draw_value_stepper(
            rect,
            rect.y + 304.0,
            "Cell H",
            metadata.grid.cell_height as i32,
            1,
        );
        draw_value_stepper(rect, rect.y + 338.0, "Offset X", metadata.grid.offset_x, 2);
        draw_value_stepper(rect, rect.y + 372.0, "Offset Y", metadata.grid.offset_y, 3);
        draw_value_stepper(
            rect,
            rect.y + 406.0,
            "Spacing X",
            metadata.grid.spacing_x as i32,
            4,
        );
        draw_value_stepper(
            rect,
            rect.y + 440.0,
            "Spacing Y",
            metadata.grid.spacing_y as i32,
            5,
        );

        let selection = metadata.selection;
        draw_editor_text(
            &format!("{} Selection", self.pixel_studio.selection_mode.label()),
            rect.x,
            rect.y + 480.0,
            18.0,
            TEXT,
        );
        for (index, line) in [
            format!("X {}  Y {}", selection.x, selection.y),
            format!("Size {} x {}", selection.width, selection.height),
            format!("Pivot {}, {}", metadata.pivot[0], metadata.pivot[1]),
        ]
        .into_iter()
        .enumerate()
        {
            draw_scissored_text(
                &line,
                rect.x,
                rect.y + 504.0 + index as f32 * 20.0,
                rect.w,
                15.0,
                TEXT,
            );
        }
        draw_editor_widget(pixel_flip_h_rect(rect), "Flip H", false);
        draw_editor_widget(pixel_flip_v_rect(rect), "Flip V", false);
        draw_editor_widget(pixel_pivot_bottom_rect(rect), "Pivot Bottom", false);
        let exact_source_context = self.pixel_studio.world_asset_context.is_some()
            && self
                .pixel_studio
                .document
                .as_ref()
                .and_then(|document| document.metadata.source_region)
                .is_some();
        draw_editor_widget(
            pixel_fit_visual_rect(rect),
            if exact_source_context { "Reset Source" } else { "Fit Visual" },
            false,
        );

        draw_editor_text("Runtime Draft Target", rect.x, rect.y + 646.0, 18.0, TEXT);
        draw_editor_widget(
            pixel_target_kind_rect(rect),
            self.pixel_studio.target_kind.label(),
            false,
        );
        draw_editor_widget(pixel_target_prev_rect(rect), "<", false);
        draw_editor_widget(pixel_target_next_rect(rect), ">", false);
        draw_scissored_text(
            self.pixel_studio.target_label(),
            rect.x + 124.0,
            rect.y + 682.0,
            rect.w - 162.0,
            14.0,
            TEXT,
        );
        draw_editor_widget(pixel_save_rect(rect), "Save Working Copy", false);
        draw_editor_widget(pixel_publish_rect(rect), "Publish Slice Draft", false);

        draw_editor_text("Palette", rect.x, rect.y + 760.0, 18.0, TEXT);
        for (index, color) in PALETTE.into_iter().enumerate() {
            let swatch = pixel_swatch_rect(rect, index);
            let display = Color::from_rgba(color[0], color[1], color[2], color[3]);
            draw_rectangle(swatch.x, swatch.y, swatch.w, swatch.h, display);
            draw_rectangle_lines(
                swatch.x,
                swatch.y,
                swatch.w,
                swatch.h,
                if color == self.pixel_studio.selected_color {
                    3.0
                } else {
                    1.0
                },
                if color == self.pixel_studio.selected_color {
                    TEXT
                } else {
                    PANEL_EDGE
                },
            );
        }
        let color = self.pixel_studio.selected_color;
        draw_scissored_text(
            &format!(
                "RGBA {}, {}, {}, {} | Shift+click cell | Alt+click pivot",
                color[0], color[1], color[2], color[3]
            ),
            rect.x,
            rect.y + rect.h - 8.0,
            rect.w,
            13.0,
            MUTED,
        );
    }

    pub(crate) fn save_pixel_document(&mut self) {
        let Some(document) = self.pixel_studio.document.as_mut() else {
            self.status_message = "No pixel document is open".to_string();
            return;
        };
        let result = document.save(repo_root_dir());
        let output_path = document.metadata.output_path.clone();
        let display_name = document.metadata.display_name.clone();
        self.status_message = match result {
            Ok(()) => {
                let _ = record_recent_pixel_document(repo_root_dir(), &output_path, &display_name);
                let _ = self.pixel_studio.refresh_library();
                if self.pixel_studio.world_asset_context.is_some() {
                    format!(
                        "Saved derived working copy {output_path}. Runtime binding is unchanged until Publish Slice Draft -> approve -> Bake + Reload."
                    )
                } else {
                    format!("Saved {output_path}, layered package, and metadata sidecar")
                }
            }
            Err(error) => error,
        };
    }

    pub(crate) fn publish_pixel_document(&mut self) {
        let target_kind = self.pixel_studio.target_kind;
        let target_code = self.pixel_studio.target_code().to_string();
        let Some(document) = self.pixel_studio.document.as_mut() else {
            self.status_message = "No pixel document is open".to_string();
            return;
        };
        self.status_message = match publish_working_copy(
            document,
            repo_root_dir(),
            target_kind,
            &target_code,
        ) {
            Ok(result) => format!(
                "Published draft {}. Review/approve it in Scene Editor > Intake, then Bake + Reload.",
                result.intake_stable_id
            ),
            Err(error) => format!("Pixel publish failed: {error}"),
        };
    }
}

fn draw_pixel_toolbar(app: &EditorApp, canvas: Rect) {
    let toolbar = Rect::new(canvas.x, canvas.y - 44.0, canvas.w, SPRITE_TOOLBAR_HEIGHT);
    draw_sprite_toolbar_background(toolbar);
    for (index, tool) in PixelTool::ALL.into_iter().enumerate() {
        draw_editor_widget(
            pixel_tool_rect(toolbar, index),
            tool.label(),
            app.pixel_studio.tool == tool,
        );
    }
    let tools_end = pixel_tool_rect(toolbar, PixelTool::ALL.len().saturating_sub(1)).x
        + pixel_tool_rect(toolbar, PixelTool::ALL.len().saturating_sub(1)).w;
    draw_sprite_toolbar_separator(tools_end + 3.0, toolbar);
    draw_editor_widget(pixel_zoom_out_rect(toolbar), "-", false);
    draw_editor_widget(pixel_zoom_in_rect(toolbar), "+", false);
    draw_editor_widget(pixel_frame_rect(toolbar), "Fit", false);
    draw_editor_widget(
        pixel_selection_mode_rect(toolbar),
        &format!("Select: {}", app.pixel_studio.selection_mode.label()),
        app.pixel_studio.tool == PixelTool::Selection,
    );
    draw_sprite_toolbar_separator(pixel_selection_mode_rect(toolbar).x - 5.0, toolbar);
    draw_editor_widget(
        pixel_atlas_grid_rect(toolbar),
        "Frame Grid",
        app.pixel_studio.show_atlas_grid,
    );
    draw_editor_widget(
        pixel_pixel_grid_rect(toolbar),
        "Pixel Grid",
        app.pixel_studio.show_pixel_grid,
    );
}

fn draw_pixel_animation_context_overlay(
    app: &EditorApp,
    document: &PixelDocument,
    image: Rect,
    canvas: Rect,
) {
    let Some(context) = app.pixel_studio.animation_context.as_ref() else {
        return;
    };
    let Some(texture) = app.pixel_studio.texture.as_ref() else {
        return;
    };
    let zoom = app.pixel_studio.zoom();
    let current = Rect::new(
        image.x + context.frame_source.x as f32 * zoom,
        image.y + context.frame_source.y as f32 * zoom,
        context.frame_source.width as f32 * zoom,
        context.frame_source.height as f32 * zoom,
    );
    if let Some(destination) = intersect_rect(current, canvas) {
        if context.onion_skin {
            for (source, tint) in [
                (context.previous_source, Color::new(0.20, 0.60, 1.0, 0.24)),
                (context.next_source, Color::new(1.0, 0.35, 0.30, 0.18)),
            ] {
                if let Some(source) = source {
                    let source_rect = Rect::new(
                        source.x as f32 + (destination.x - current.x) / zoom,
                        source.y as f32 + (destination.y - current.y) / zoom,
                        destination.w / zoom,
                        destination.h / zoom,
                    );
                    draw_texture_ex(
                        texture,
                        destination.x,
                        destination.y,
                        tint,
                        DrawTextureParams {
                            dest_size: Some(vec2(destination.w, destination.h)),
                            source: Some(source_rect),
                            ..Default::default()
                        },
                    );
                }
            }
        }
        draw_rectangle_lines(
            destination.x,
            destination.y,
            destination.w,
            destination.h,
            3.0,
            GOOD,
        );
    }
    let pivot_x = image.x + document.metadata.pivot[0] as f32 * zoom;
    let pivot_y = image.y + document.metadata.pivot[1] as f32 * zoom;
    if canvas.contains(vec2(pivot_x, pivot_y)) {
        draw_line(pivot_x - 8.0, pivot_y, pivot_x + 8.0, pivot_y, 2.0, WARN);
        draw_line(pivot_x, pivot_y - 8.0, pivot_x, pivot_y + 8.0, 2.0, WARN);
    }
    let shadow_x = pivot_x + context.shadow_offset[0] as f32 * zoom;
    let shadow_y = pivot_y + context.shadow_offset[1] as f32 * zoom;
    if canvas.contains(vec2(shadow_x, shadow_y)) {
        draw_ellipse_lines(
            shadow_x,
            shadow_y,
            12.0,
            5.0,
            0.0,
            1.0,
            Color::new(0.45, 0.50, 0.58, 0.95),
        );
    }
    for socket in &context.sockets {
        let x = image.x + (context.frame_source.x as i32 + socket.position[0]) as f32 * zoom;
        let y = image.y + (context.frame_source.y as i32 + socket.position[1]) as f32 * zoom;
        if canvas.contains(vec2(x, y)) {
            draw_circle(x, y, 5.0, ACCENT);
            draw_circle_lines(x, y, 7.0, 1.0, TEXT);
            draw_editor_text(socket.kind.label(), x + 9.0, y + 4.0, 11.0, TEXT);
        }
    }
}

fn draw_asset_preview(app: &EditorApp, canvas: Rect) {
    let Some(document) = app.pixel_studio.document.as_ref() else {
        return;
    };
    let role = PixelAssetRole::from_document(document);
    if role.supports_repeat_preview() && app.pixel_studio.show_repeat_preview {
        draw_repeat_preview(app, canvas);
    } else if matches!(role, PixelAssetRole::Character | PixelAssetRole::Object) {
        draw_frame_preview(app, canvas, role);
    }
}

fn draw_frame_preview(app: &EditorApp, canvas: Rect, role: PixelAssetRole) {
    let (Some(document), Some(texture)) = (
        app.pixel_studio.document.as_ref(),
        app.pixel_studio.texture.as_ref(),
    ) else {
        return;
    };
    let selection = document.metadata.selection;
    if selection.is_empty() {
        return;
    }
    let panel = Rect::new(canvas.x + canvas.w - 190.0, canvas.y + 12.0, 178.0, 198.0);
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::new(0.06, 0.07, 0.09, 0.94),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, PANEL_EDGE);
    let title = if matches!(role, PixelAssetRole::Character) {
        "Character Frame"
    } else {
        "Object in World Grid"
    };
    draw_editor_text(title, panel.x + 10.0, panel.y + 20.0, 15.0, TEXT);
    let preview = Rect::new(
        panel.x + 10.0,
        panel.y + 30.0,
        panel.w - 20.0,
        panel.w - 20.0,
    );
    draw_checkerboard(preview, 10.0);
    let scale =
        (preview.w / selection.width.max(1) as f32).min(preview.h / selection.height.max(1) as f32);
    let dest = vec2(
        selection.width as f32 * scale,
        selection.height as f32 * scale,
    );
    draw_texture_ex(
        texture,
        preview.x + (preview.w - dest.x) * 0.5,
        preview.y + (preview.h - dest.y) * 0.5,
        WHITE,
        DrawTextureParams {
            dest_size: Some(dest),
            source: Some(Rect::new(
                selection.x as f32,
                selection.y as f32,
                selection.width as f32,
                selection.height as f32,
            )),
            ..Default::default()
        },
    );
    draw_rectangle_lines(preview.x, preview.y, preview.w, preview.h, 1.0, TEXT);
}

fn draw_repeat_preview(app: &EditorApp, canvas: Rect) {
    let (Some(document), Some(texture)) = (
        app.pixel_studio.document.as_ref(),
        app.pixel_studio.texture.as_ref(),
    ) else {
        return;
    };
    let mut selection = document.metadata.selection;
    if selection.is_empty() {
        selection.width = document.metadata.grid.cell_width.max(1);
        selection.height = document.metadata.grid.cell_height.max(1);
    }
    selection.width = selection
        .width
        .min(document.width().saturating_sub(selection.x));
    selection.height = selection
        .height
        .min(document.height().saturating_sub(selection.y));
    if selection.is_empty() {
        return;
    }
    let panel = Rect::new(canvas.x + canvas.w - 190.0, canvas.y + 12.0, 178.0, 198.0);
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::new(0.06, 0.07, 0.09, 0.94),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, PANEL_EDGE);
    draw_editor_text(
        "3x3 Repeat Preview",
        panel.x + 10.0,
        panel.y + 20.0,
        15.0,
        TEXT,
    );
    let preview = Rect::new(
        panel.x + 10.0,
        panel.y + 30.0,
        panel.w - 20.0,
        panel.w - 20.0,
    );
    draw_checkerboard(preview, 10.0);
    let cell = (preview.w / 3.0).min(preview.h / 3.0);
    let source = Rect::new(
        selection.x as f32,
        selection.y as f32,
        selection.width as f32,
        selection.height as f32,
    );
    for row in 0..3 {
        for column in 0..3 {
            draw_texture_ex(
                texture,
                preview.x + column as f32 * cell,
                preview.y + row as f32 * cell,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(cell, cell)),
                    source: Some(source),
                    ..Default::default()
                },
            );
        }
    }
    draw_rectangle_lines(preview.x, preview.y, preview.w, preview.h, 1.0, TEXT);
}

fn draw_checkerboard(rect: Rect, cell: f32) {
    let columns = (rect.w / cell).ceil() as i32;
    let rows = (rect.h / cell).ceil() as i32;
    for y in 0..rows {
        for x in 0..columns {
            let color = if (x + y) % 2 == 0 {
                Color::new(0.16, 0.17, 0.19, 1.0)
            } else {
                Color::new(0.11, 0.12, 0.14, 1.0)
            };
            draw_rectangle(
                rect.x + x as f32 * cell,
                rect.y + y as f32 * cell,
                cell,
                cell,
                color,
            );
        }
    }
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
}

fn draw_pixel_grid(_document: &PixelDocument, transform: SpriteCanvasTransform) {
    let style = SpriteOverlayKind::PixelGrid.style();
    let visible = transform.visible_pixel_bounds();
    let start_x = visible.x as i32;
    let end_x = (visible.x + visible.w).ceil() as i32;
    let start_y = visible.y as i32;
    let end_y = (visible.y + visible.h).ceil() as i32;
    let Some(clipped) = transform.clipped_image() else {
        return;
    };
    for x in start_x..=end_x {
        let sx = transform.pixel_to_screen(vec2(x as f32, 0.0)).x;
        draw_line(
            sx,
            clipped.y,
            sx,
            clipped.y + clipped.h,
            style.line_width,
            style.color,
        );
    }
    for y in start_y..=end_y {
        let sy = transform.pixel_to_screen(vec2(0.0, y as f32)).y;
        draw_line(
            clipped.x,
            sy,
            clipped.x + clipped.w,
            sy,
            style.line_width,
            style.color,
        );
    }
}

fn draw_atlas_grid(document: &PixelDocument, transform: SpriteCanvasTransform) {
    let grid = document.metadata.grid;
    let kind =
        if grid.offset_x == 0 && grid.offset_y == 0 && grid.spacing_x == 0 && grid.spacing_y == 0 {
            SpriteOverlayKind::FrameGrid
        } else {
            SpriteOverlayKind::AtlasEntry
        };
    let style = kind.style();
    let Some(clipped) = transform.clipped_image() else {
        return;
    };
    let visible = transform.visible_pixel_bounds();
    let stride_x = (grid.cell_width + grid.spacing_x).max(1) as i32;
    let stride_y = (grid.cell_height + grid.spacing_y).max(1) as i32;
    let visible_min_x = visible.x.floor() as i32;
    let visible_max_x = (visible.x + visible.w).ceil() as i32;
    let visible_min_y = visible.y.floor() as i32;
    let visible_max_y = (visible.y + visible.h).ceil() as i32;

    let first_column = (visible_min_x - grid.offset_x).div_euclid(stride_x).max(0);
    let last_column = (visible_max_x - grid.offset_x).div_euclid(stride_x) + 1;
    for column in first_column..=last_column {
        let x = grid.offset_x + column * stride_x;
        if x > document.width() as i32 {
            break;
        }
        let sx = transform.pixel_to_screen(vec2(x as f32, 0.0)).x;
        if sx >= transform.canvas.x && sx <= transform.canvas.x + transform.canvas.w {
            draw_line(
                sx,
                clipped.y,
                sx,
                clipped.y + clipped.h,
                style.line_width,
                style.color,
            );
        }
    }

    let first_row = (visible_min_y - grid.offset_y).div_euclid(stride_y).max(0);
    let last_row = (visible_max_y - grid.offset_y).div_euclid(stride_y) + 1;
    for row in first_row..=last_row {
        let y = grid.offset_y + row * stride_y;
        if y > document.height() as i32 {
            break;
        }
        let sy = transform.pixel_to_screen(vec2(0.0, y as f32)).y;
        if sy >= transform.canvas.y && sy <= transform.canvas.y + transform.canvas.h {
            draw_line(
                clipped.x,
                sy,
                clipped.x + clipped.w,
                sy,
                style.line_width,
                style.color,
            );
        }
    }
}

fn draw_pixel_selection(document: &PixelDocument, transform: SpriteCanvasTransform) {
    let selection = document.metadata.selection;
    if !selection.is_empty() {
        let style = SpriteOverlayKind::Selection.style();
        let rect = transform.pixel_rect_to_screen(Rect::new(
            selection.x as f32,
            selection.y as f32,
            selection.width as f32,
            selection.height as f32,
        ));
        if let Some(clipped) = intersect_rect(rect, transform.canvas) {
            draw_rectangle_lines(
                clipped.x,
                clipped.y,
                clipped.w,
                clipped.h,
                style.line_width,
                style.color,
            );
        }
    }
    let pivot = document.metadata.pivot;
    let point = transform.pixel_to_screen(vec2(pivot[0] as f32, pivot[1] as f32));
    let style = SpriteOverlayKind::Pivot.style();
    if transform.canvas.contains(point) {
        draw_line(
            point.x - 6.0,
            point.y,
            point.x + 6.0,
            point.y,
            style.line_width,
            style.color,
        );
        draw_line(
            point.x,
            point.y - 6.0,
            point.x,
            point.y + 6.0,
            style.line_width,
            style.color,
        );
    }
}
