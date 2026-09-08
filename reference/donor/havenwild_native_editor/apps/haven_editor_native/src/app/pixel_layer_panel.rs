use super::render_helpers::*;
use super::sprite_workspace::*;
use super::*;

const LAYER_ROW_HEIGHT: f32 = 34.0;
const VISIBLE_LAYER_ROWS: usize = 8;

impl EditorApp {
    pub(crate) fn draw_pixel_layer_inspector(&self, rect: Rect) {
        let Some(document) = self.pixel_studio.document.as_ref() else {
            return;
        };
        draw_scissored_text(
            &format!(
                "{} layers | Active: {}",
                document.layer_count(),
                document.active_layer().metadata.name
            ),
            rect.x,
            rect.y + 82.0,
            rect.w,
            15.0,
            TEXT,
        );

        for (button, label) in [
            (pixel_layer_add_rect(rect), "+ Layer"),
            (pixel_layer_duplicate_rect(rect), "Duplicate"),
            (pixel_layer_delete_rect(rect), "Delete"),
            (pixel_layer_up_rect(rect), "Up"),
            (pixel_layer_down_rect(rect), "Down"),
            (pixel_layer_merge_rect(rect), "Merge Down"),
        ] {
            draw_editor_widget(button, label, false);
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + 152.0, rect.w, 24.0),
            "Layer Stack",
            Some("V = visible  L = locked"),
        );
        for row in 0..VISIBLE_LAYER_ROWS {
            let Some(index) = pixel_layer_index_for_row(
                document.layer_count(),
                self.pixel_studio.layer_offset,
                row,
            ) else {
                break;
            };
            let layer = &document.layers()[index];
            let row_rect = pixel_layer_row_rect(rect, row);
            let active = index == document.active_layer_index();
            draw_sprite_layer_row(
                row_rect,
                &layer.metadata.name,
                layer.metadata.blend_mode.label(),
                active,
                layer.metadata.visible,
                layer.metadata.locked,
            );
            draw_editor_widget(
                pixel_layer_visibility_rect(rect, row),
                if layer.metadata.visible { "V" } else { "-" },
                layer.metadata.visible,
            );
            draw_editor_widget(
                pixel_layer_lock_rect(rect, row),
                if layer.metadata.locked { "L" } else { "-" },
                layer.metadata.locked,
            );
        }

        let layer = document.active_layer();
        draw_section_header(
            Rect::new(rect.x, rect.y + 450.0, rect.w, 24.0),
            "Selected Layer",
            Some(layer.metadata.blend_mode.label()),
        );
        draw_editor_widget(
            pixel_layer_visibility_toggle_rect(rect),
            if layer.metadata.visible {
                "Visible"
            } else {
                "Hidden"
            },
            layer.metadata.visible,
        );
        draw_editor_widget(
            pixel_layer_lock_toggle_rect(rect),
            if layer.metadata.locked {
                "Locked"
            } else {
                "Unlocked"
            },
            layer.metadata.locked,
        );
        draw_editor_widget(pixel_layer_opacity_minus_rect(rect), "-", false);
        draw_editor_widget(pixel_layer_opacity_plus_rect(rect), "+", false);
        draw_scissored_text(
            &format!("Opacity {}%", layer.metadata.opacity as u32 * 100 / 255),
            rect.x + 46.0,
            rect.y + 538.0,
            118.0,
            14.0,
            TEXT,
        );
        draw_editor_widget(
            pixel_layer_blend_rect(rect),
            layer.metadata.blend_mode.label(),
            false,
        );
        draw_editor_widget(pixel_layer_rename_rect(rect), "Rename", false);
        let rename_text = self
            .pixel_studio
            .layer_rename_buffer
            .as_deref()
            .unwrap_or(&layer.metadata.name);
        draw_rectangle(
            pixel_layer_rename_field_rect(rect).x,
            pixel_layer_rename_field_rect(rect).y,
            pixel_layer_rename_field_rect(rect).w,
            pixel_layer_rename_field_rect(rect).h,
            CONTROL_BG,
        );
        draw_rectangle_lines(
            pixel_layer_rename_field_rect(rect).x,
            pixel_layer_rename_field_rect(rect).y,
            pixel_layer_rename_field_rect(rect).w,
            pixel_layer_rename_field_rect(rect).h,
            if self.pixel_studio.layer_rename_buffer.is_some() {
                2.0
            } else {
                1.0
            },
            if self.pixel_studio.layer_rename_buffer.is_some() {
                ACCENT
            } else {
                PANEL_EDGE
            },
        );
        draw_scissored_text(
            rename_text,
            pixel_layer_rename_field_rect(rect).x + 6.0,
            pixel_layer_rename_field_rect(rect).y + 20.0,
            pixel_layer_rename_field_rect(rect).w - 12.0,
            14.0,
            TEXT,
        );

        draw_section_header(
            Rect::new(rect.x, rect.y + 602.0, rect.w, 24.0),
            "Document",
            Some("Save / Resize"),
        );
        draw_editor_widget(pixel_layer_save_rect(rect), "Save", false);
        draw_editor_widget(pixel_layer_autosave_rect(rect), "Autosave", false);
        draw_editor_widget(pixel_layer_crop_rect(rect), "Crop Selection", false);
        draw_editor_widget(pixel_layer_trim_rect(rect), "Trim Transparent", false);
        draw_editor_widget(pixel_layer_resize_w_minus_rect(rect), "W-", false);
        draw_editor_widget(pixel_layer_resize_w_plus_rect(rect), "W+", false);
        draw_editor_widget(pixel_layer_resize_h_minus_rect(rect), "H-", false);
        draw_editor_widget(pixel_layer_resize_h_plus_rect(rect), "H+", false);
        draw_scissored_text(
            &format!(
                "Resize {} x {}",
                self.pixel_studio.resize_width, self.pixel_studio.resize_height
            ),
            rect.x + 4.0,
            rect.y + 733.0,
            170.0,
            14.0,
            TEXT,
        );
        draw_editor_widget(pixel_layer_resize_apply_rect(rect), "Apply", false);
        draw_scissored_text(
            &self.pixel_studio.autosave_status,
            rect.x,
            rect.y + rect.h - 8.0,
            rect.w,
            12.0,
            if document.recovered_from_autosave {
                WARN
            } else {
                MUTED
            },
        );
    }
}

pub(crate) fn pixel_layer_index_for_row(
    layer_count: usize,
    offset: usize,
    row: usize,
) -> Option<usize> {
    let reverse_index = offset.checked_add(row)?;
    let one_based_index = reverse_index.checked_add(1)?;
    layer_count.checked_sub(one_based_index)
}

pub(crate) fn pixel_layer_tabs_y(rect: Rect) -> f32 {
    rect.y + 34.0
}
pub(crate) fn pixel_layer_tab_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        pixel_layer_tabs_y(rect),
        (rect.w - 18.0) / 4.0,
        30.0,
    )
}
pub(crate) fn pixel_asset_tab_rect(rect: Rect) -> Rect {
    let width = (rect.w - 18.0) / 4.0;
    Rect::new(rect.x + width + 6.0, pixel_layer_tabs_y(rect), width, 30.0)
}
pub(crate) fn pixel_animation_tab_rect(rect: Rect) -> Rect {
    let width = (rect.w - 18.0) / 4.0;
    Rect::new(
        rect.x + (width + 6.0) * 2.0,
        pixel_layer_tabs_y(rect),
        width,
        30.0,
    )
}
pub(crate) fn pixel_animation_onion_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 196.0, 136.0, 30.0)
}
pub(crate) fn pixel_animation_focus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 196.0, 142.0, 30.0)
}
pub(crate) fn pixel_animation_save_return_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 252.0, 286.0, 34.0)
}
pub(crate) fn pixel_animation_cancel_return_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 292.0, 286.0, 34.0)
}
pub(crate) fn pixel_layer_add_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 96.0, 88.0, 28.0)
}
pub(crate) fn pixel_layer_duplicate_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 94.0, rect.y + 96.0, 96.0, 28.0)
}
pub(crate) fn pixel_layer_delete_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 196.0, rect.y + 96.0, 90.0, 28.0)
}
pub(crate) fn pixel_layer_up_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 130.0, 56.0, 28.0)
}
pub(crate) fn pixel_layer_down_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 62.0, rect.y + 130.0, 66.0, 28.0)
}
pub(crate) fn pixel_layer_merge_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 134.0, rect.y + 130.0, 152.0, 28.0)
}
pub(crate) fn pixel_layer_row_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x,
        rect.y + 184.0 + row as f32 * LAYER_ROW_HEIGHT,
        rect.w,
        LAYER_ROW_HEIGHT - 4.0,
    )
}
pub(crate) fn pixel_layer_visibility_rect(rect: Rect, row: usize) -> Rect {
    let row_rect = pixel_layer_row_rect(rect, row);
    Rect::new(row_rect.x + 3.0, row_rect.y + 3.0, 28.0, 24.0)
}
pub(crate) fn pixel_layer_lock_rect(rect: Rect, row: usize) -> Rect {
    let row_rect = pixel_layer_row_rect(rect, row);
    Rect::new(row_rect.x + 35.0, row_rect.y + 3.0, 28.0, 24.0)
}
pub(crate) fn pixel_layer_visibility_toggle_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 486.0, 136.0, 30.0)
}
pub(crate) fn pixel_layer_lock_toggle_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 486.0, 142.0, 30.0)
}
pub(crate) fn pixel_layer_opacity_minus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 522.0, 38.0, 30.0)
}
pub(crate) fn pixel_layer_opacity_plus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 128.0, rect.y + 522.0, 38.0, 30.0)
}
pub(crate) fn pixel_layer_blend_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 174.0, rect.y + 522.0, 112.0, 30.0)
}
pub(crate) fn pixel_layer_rename_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 558.0, 80.0, 30.0)
}
pub(crate) fn pixel_layer_rename_field_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 86.0, rect.y + 558.0, rect.w - 86.0, 30.0)
}
pub(crate) fn pixel_layer_save_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 638.0, 136.0, 30.0)
}
pub(crate) fn pixel_layer_autosave_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 638.0, 142.0, 30.0)
}
pub(crate) fn pixel_layer_crop_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 674.0, 136.0, 30.0)
}
pub(crate) fn pixel_layer_trim_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 674.0, 142.0, 30.0)
}
pub(crate) fn pixel_layer_resize_w_minus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 710.0, 40.0, 28.0)
}
pub(crate) fn pixel_layer_resize_w_plus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 46.0, rect.y + 710.0, 40.0, 28.0)
}
pub(crate) fn pixel_layer_resize_h_minus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 92.0, rect.y + 710.0, 40.0, 28.0)
}
pub(crate) fn pixel_layer_resize_h_plus_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 138.0, rect.y + 710.0, 40.0, 28.0)
}
pub(crate) fn pixel_layer_resize_apply_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 190.0, rect.y + 710.0, 96.0, 28.0)
}

#[cfg(test)]
mod tests {
    use super::pixel_layer_index_for_row;

    #[test]
    fn empty_layer_stack_has_no_visible_rows() {
        assert_eq!(pixel_layer_index_for_row(0, 0, 0), None);
    }

    #[test]
    fn rows_map_from_topmost_layer_down() {
        assert_eq!(pixel_layer_index_for_row(3, 0, 0), Some(2));
        assert_eq!(pixel_layer_index_for_row(3, 0, 2), Some(0));
    }

    #[test]
    fn rows_outside_layer_stack_are_ignored() {
        assert_eq!(pixel_layer_index_for_row(3, 0, 3), None);
        assert_eq!(pixel_layer_index_for_row(3, 4, 0), None);
    }

    #[test]
    fn offset_addition_overflow_is_ignored() {
        assert_eq!(pixel_layer_index_for_row(3, usize::MAX, 0), None);
        assert_eq!(pixel_layer_index_for_row(3, usize::MAX, 1), None);
    }
}
