use super::pixel_studio_layout::*;
use super::render_helpers::*;
use super::*;

impl EditorApp {
    pub(crate) fn draw_pixel_library(&self, rect: Rect) {
        let filtered = self.pixel_studio.filtered_library_indices();
        let summary = format!("{} / {}", filtered.len(), self.pixel_studio.library.len());
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Pixel Assets",
            Some(&summary),
        );
        draw_editor_widget(pixel_refresh_rect(rect), "Rescan", false);
        draw_editor_widget(pixel_new_rect(rect), "New Asset", false);

        let search = pixel_library_search_rect(rect);
        draw_editor_widget_tone(
            search,
            if self.pixel_studio.library_filter.is_empty() {
                "Search assets..."
            } else {
                self.pixel_studio.library_filter.as_str()
            },
            self.text_focus == EditorTextFocus::PixelLibraryFilter,
            WidgetTone::Quiet,
        );
        draw_editor_widget_tone(
            pixel_library_clear_rect(rect),
            "×",
            false,
            if self.pixel_studio.library_filter.is_empty() {
                WidgetTone::Disabled
            } else {
                WidgetTone::Quiet
            },
        );

        if self.pixel_studio.library.is_empty() {
            let message = if self.pixel_studio.library_loaded {
                "No project PNG files were found. Raw external LPC sheets stay out of the eager library; use the LPC slice catalog workflow for them."
            } else {
                "No project-owned images are available yet. Choose New Asset or use Rescan after adding files."
            };
            draw_wrapped(message, rect.x, rect.y + 108.0, rect.w, 15.0, MUTED);
            return;
        }
        if filtered.is_empty() {
            draw_wrapped(
                "No Pixel Assets match this search. Clear the compact filter to restore the grouped project library.",
                rect.x,
                rect.y + 108.0,
                rect.w,
                14.0,
                MUTED,
            );
            return;
        }

        for row in pixel_library_visual_rows(rect, &self.pixel_studio) {
            match row {
                PixelLibraryVisualRow::Group { rect, label } => {
                    draw_editor_text(label, rect.x + 2.0, rect.y + 15.0, 12.0, MUTED);
                    let label_width = measure_editor_text(label, None, 12, 1.0).width;
                    draw_line(
                        rect.x + label_width + 10.0,
                        rect.y + 11.0,
                        rect.x + rect.w,
                        rect.y + 11.0,
                        1.0,
                        PANEL_EDGE,
                    );
                }
                PixelLibraryVisualRow::Entry { rect: row, index } => {
                    let Some(entry) = self.pixel_studio.library.get(index) else {
                        continue;
                    };
                    let secondary = if entry.recent_rank.is_some() {
                        format!("Recent • {}", entry.relative_path)
                    } else {
                        entry.relative_path.clone()
                    };
                    draw_list_row(
                        row,
                        &entry.display_name,
                        Some(&secondary),
                        index == self.pixel_studio.selected_entry,
                    );
                }
            }
        }
    }

    pub(crate) fn update_pixel_library_navigation(&mut self, rect: Rect, mouse: Vec2) -> bool {
        if !rect.contains(mouse) {
            return false;
        }
        let wheel = mouse_wheel().1;
        if wheel.abs() <= 0.05 {
            return false;
        }
        let filtered_count = self.pixel_studio.filtered_library_indices().len();
        let visible = pixel_library_visual_rows(rect, &self.pixel_studio)
            .into_iter()
            .filter(|row| matches!(row, PixelLibraryVisualRow::Entry { .. }))
            .count()
            .max(1);
        let maximum = filtered_count.saturating_sub(visible);
        if wheel > 0.0 {
            self.pixel_studio.library_offset = self.pixel_studio.library_offset.saturating_sub(1);
        } else {
            self.pixel_studio.library_offset = (self.pixel_studio.library_offset + 1).min(maximum);
        }
        true
    }

    pub(crate) fn handle_pixel_library_click(&mut self, mouse: Vec2, rect: Rect) -> bool {
        if pixel_refresh_rect(rect).contains(mouse) {
            self.status_message = match self.pixel_studio.refresh_library() {
                Ok(count) => format!("Rescanned Pixel Studio library: {count} PNG files"),
                Err(error) => error,
            };
            return true;
        }
        if pixel_new_rect(rect).contains(mouse) {
            self.status_message = self.pixel_studio.create_blank();
            return true;
        }
        if pixel_library_search_rect(rect).contains(mouse) {
            self.text_focus = EditorTextFocus::PixelLibraryFilter;
            return true;
        }
        if pixel_library_clear_rect(rect).contains(mouse) {
            if !self.pixel_studio.library_filter.is_empty() {
                self.pixel_studio.library_filter.clear();
                self.pixel_studio.library_offset = 0;
                self.status_message = "Pixel Asset search cleared".to_string();
            }
            return true;
        }
        for row in pixel_library_visual_rows(rect, &self.pixel_studio) {
            let PixelLibraryVisualRow::Entry {
                rect: row_rect,
                index,
            } = row
            else {
                continue;
            };
            if row_rect.contains(mouse) {
                self.pixel_studio.selected_entry = index;
                self.open_selected_pixel_library_entry();
                return true;
            }
        }
        true
    }

    pub(crate) fn open_selected_pixel_library_entry(&mut self) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.pixel_studio.load_selected()
        }));
        self.status_message = match result {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => format!("Pixel Studio could not open asset: {error}"),
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown asset-load panic");
                format!(
                    "Pixel Studio blocked an asset-load crash: {detail}. See logs/haven_editor_native_crash.log"
                )
            }
        };
    }

    pub(crate) fn cycle_pixel_library(&mut self, delta: i32) {
        let indices = self.pixel_studio.filtered_library_indices();
        if indices.is_empty() {
            return;
        }
        let current = indices
            .iter()
            .position(|index| *index == self.pixel_studio.selected_entry)
            .unwrap_or(0);
        let next = cycle_index(current, indices.len(), delta);
        self.pixel_studio.selected_entry = indices[next];
        if next < self.pixel_studio.library_offset {
            self.pixel_studio.library_offset = next;
        } else {
            let list = self.shell_layout().list_content;
            let visible_entries = pixel_library_visual_rows(list, &self.pixel_studio)
                .into_iter()
                .filter(|row| matches!(row, PixelLibraryVisualRow::Entry { .. }))
                .count()
                .max(1);
            if next >= self.pixel_studio.library_offset + visible_entries {
                self.pixel_studio.library_offset = next + 1 - visible_entries;
            }
        }
    }
}
