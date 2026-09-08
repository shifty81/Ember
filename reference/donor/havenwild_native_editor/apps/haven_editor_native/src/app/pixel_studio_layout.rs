use super::render_helpers::*;
use super::sprite_canvas_authority::*;
use super::*;

pub(crate) fn draw_selection_outline(
    selection: haven_pixel::PixelSelection,
    transform: SpriteCanvasTransform,
    color: Color,
) {
    if selection.is_empty() {
        return;
    }
    let rect = transform.pixel_rect_to_screen(Rect::new(
        selection.x as f32,
        selection.y as f32,
        selection.width as f32,
        selection.height as f32,
    ));
    if let Some(clipped) = intersect_rect(rect, transform.canvas) {
        let width = SpriteOverlayKind::Selection.style().line_width;
        draw_rectangle_lines(clipped.x, clipped.y, clipped.w, clipped.h, width, color);
    }
}

pub(crate) fn intersect_rect(left: Rect, right: Rect) -> Option<Rect> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.w).min(right.x + right.w);
    let bottom_edge = (left.y + left.h).min(right.y + right.h);
    (right_edge > x && bottom_edge > y).then_some(Rect::new(x, y, right_edge - x, bottom_edge - y))
}

pub(crate) fn draw_value_stepper(rect: Rect, y: f32, label: &str, value: i32, row: usize) {
    draw_editor_text(label, rect.x, y + 20.0, 15.0, TEXT);
    draw_editor_widget(pixel_value_minus_rect(rect, row), "-", false);
    draw_editor_widget(pixel_value_plus_rect(rect, row), "+", false);
    draw_scissored_text(
        &value.to_string(),
        rect.x + 178.0,
        y + 20.0,
        48.0,
        15.0,
        TEXT,
    );
}

pub(crate) fn pixel_refresh_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 34.0, 104.0, 30.0)
}
pub(crate) fn pixel_new_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 112.0, rect.y + 34.0, 104.0, 30.0)
}
pub(crate) fn pixel_library_search_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 70.0, (rect.w - 30.0).max(40.0), 24.0)
}

pub(crate) fn pixel_library_clear_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 24.0, rect.y + 70.0, 24.0, 24.0)
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PixelLibraryVisualRow {
    Group { rect: Rect, label: &'static str },
    Entry { rect: Rect, index: usize },
}

pub(crate) fn pixel_library_visual_rows(
    rect: Rect,
    state: &PixelStudioState,
) -> Vec<PixelLibraryVisualRow> {
    let indices = state.filtered_library_indices();
    let start = state.library_offset.min(indices.len().saturating_sub(1));
    let mut rows = Vec::new();
    let mut y = rect.y + 102.0;
    let bottom = rect.y + rect.h - 4.0;
    let mut previous_group: Option<&'static str> = None;
    for &index in indices.iter().skip(start) {
        let Some(entry) = state.library.get(index) else {
            continue;
        };
        let group = if entry.recent_rank.is_some() {
            "Recent"
        } else {
            entry.source.label()
        };
        if previous_group != Some(group) {
            if y + 20.0 > bottom {
                break;
            }
            rows.push(PixelLibraryVisualRow::Group {
                rect: Rect::new(rect.x, y, rect.w, 20.0),
                label: group,
            });
            y += 23.0;
            previous_group = Some(group);
        }
        if y + 38.0 > bottom {
            break;
        }
        rows.push(PixelLibraryVisualRow::Entry {
            rect: Rect::new(rect.x, y, rect.w, 36.0),
            index,
        });
        y += 39.0;
    }
    rows
}
pub(crate) fn pixel_tool_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(rect.x + index as f32 * 66.0, rect.y, 62.0, 30.0)
}
pub(crate) fn pixel_zoom_out_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 416.0, rect.y, 34.0, 30.0)
}
pub(crate) fn pixel_zoom_in_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 378.0, rect.y, 34.0, 30.0)
}
pub(crate) fn pixel_frame_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 340.0, rect.y, 58.0, 30.0)
}
pub(crate) fn pixel_selection_mode_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 278.0, rect.y, 92.0, 30.0)
}
pub(crate) fn pixel_pixel_grid_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 92.0, rect.y, 88.0, 30.0)
}
pub(crate) fn pixel_atlas_grid_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 182.0, rect.y, 86.0, 30.0)
}
pub(crate) fn pixel_grid_toggle_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 232.0, 136.0, 30.0)
}
pub(crate) fn pixel_realign_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 232.0, 142.0, 30.0)
}
pub(crate) fn pixel_value_minus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + 132.0,
        rect.y + 270.0 + row as f32 * 34.0,
        34.0,
        28.0,
    )
}
pub(crate) fn pixel_value_plus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + 232.0,
        rect.y + 270.0 + row as f32 * 34.0,
        34.0,
        28.0,
    )
}
pub(crate) fn pixel_flip_h_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 570.0, 136.0, 30.0)
}
pub(crate) fn pixel_flip_v_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 570.0, 142.0, 30.0)
}
pub(crate) fn pixel_pivot_bottom_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 606.0, 136.0, 30.0)
}
pub(crate) fn pixel_fit_visual_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 606.0, 142.0, 30.0)
}
pub(crate) fn pixel_target_kind_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 674.0, 78.0, 30.0)
}
pub(crate) fn pixel_target_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 84.0, rect.y + 674.0, 34.0, 30.0)
}
pub(crate) fn pixel_target_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 34.0, rect.y + 674.0, 34.0, 30.0)
}
pub(crate) fn pixel_save_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 714.0, 136.0, 32.0)
}
pub(crate) fn pixel_publish_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 144.0, rect.y + 714.0, 142.0, 32.0)
}
pub(crate) fn pixel_swatch_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x + (index % 8) as f32 * 34.0,
        rect.y + 786.0 + (index / 8) as f32 * 34.0,
        28.0,
        28.0,
    )
}
