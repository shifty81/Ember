use super::workspace_shell::EditorWorkspaceShellState;
use super::*;

pub(crate) const MENU_BAR_H: f32 = super::editor_theme::metrics::MENU_BAR_H;
pub(crate) const DOCUMENT_BAR_H: f32 = super::editor_theme::metrics::DOCUMENT_BAR_H;
pub(crate) const TOP_BAR_H: f32 = super::editor_theme::metrics::TOP_BAR_H;
pub(crate) const OUTER_GAP: f32 = super::editor_theme::metrics::OUTER_GAP;
pub(crate) const PANEL_HEADER_H: f32 = super::editor_theme::metrics::PANEL_HEADER_H;
pub(crate) const STATUS_BAR_H: f32 = super::editor_theme::metrics::STATUS_BAR_H;
pub(crate) const BOTTOM_TAB_H: f32 = 30.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorShellLayout {
    pub left_panel: Rect,
    pub center_panel: Rect,
    pub right_panel: Rect,
    pub bottom_dock: Rect,
    pub bottom_tab_bar: Rect,
    pub status_bar: Rect,
    pub list_content: Rect,
    pub inspector_content: Rect,
    pub bottom_content: Rect,
    pub left_splitter: Rect,
    pub right_splitter: Rect,
    pub bottom_splitter: Rect,
}

impl EditorShellLayout {
    pub(crate) fn calculate(width: f32, height: f32, state: &EditorWorkspaceShellState) -> Self {
        // Never pretend the window is larger than the actual drawable area.
        // The old 960x640 floor caused panels to be laid out off-screen when
        // Windows/DPI scaling or a resized window reported a smaller surface.
        let width = width.max(1.0);
        let height = height.max(1.0);
        let body_y = TOP_BAR_H + OUTER_GAP;
        let status_bar = Rect::new(
            OUTER_GAP,
            height - OUTER_GAP - STATUS_BAR_H,
            width - OUTER_GAP * 2.0,
            STATUS_BAR_H,
        );
        let body_bottom = status_bar.y - OUTER_GAP;
        let body_h = (body_bottom - body_y).max(1.0);

        let requested_left = if state.left_panel_visible {
            state.left_panel_width.clamp(220.0, 360.0)
        } else {
            0.0
        };
        let requested_right = if state.right_panel_visible {
            state.right_panel_width.clamp(300.0, 480.0)
        } else {
            0.0
        };
        let left_gap = if state.left_panel_visible { OUTER_GAP } else { 0.0 };
        let right_gap = if state.right_panel_visible { OUTER_GAP } else { 0.0 };
        let side_budget = (width
            - OUTER_GAP * 2.0
            - left_gap
            - right_gap
            - super::editor_theme::metrics::MIN_CANVAS_W)
            .max(0.0);
        let requested_side_total = requested_left + requested_right;
        let compression = if requested_side_total > side_budget && requested_side_total > 0.0 {
            (side_budget / requested_side_total).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let left_w = requested_left * compression;
        let right_w = requested_right * compression;

        let left_panel = if state.left_panel_visible {
            Rect::new(OUTER_GAP, body_y, left_w, body_h)
        } else {
            Rect::new(OUTER_GAP, body_y, 0.0, body_h)
        };
        let right_panel = if state.right_panel_visible {
            Rect::new(width - OUTER_GAP - right_w, body_y, right_w, body_h)
        } else {
            Rect::new(width - OUTER_GAP, body_y, 0.0, body_h)
        };
        let center_x = OUTER_GAP + left_w + left_gap;
        let center_right = width - OUTER_GAP - right_w - right_gap;
        let center_panel = Rect::new(
            center_x,
            body_y,
            (center_right - center_x).max(1.0),
            body_h,
        );

        let bottom_dock = if state.bottom_dock_open {
            let max_dock_h = center_panel.h.min(320.0).max(1.0);
            let min_dock_h = super::editor_theme::metrics::MIN_BOTTOM_DOCK_H.min(max_dock_h);
            let dock_h = state.bottom_dock_height.clamp(min_dock_h, max_dock_h);
            Rect::new(
                center_panel.x,
                center_panel.y + center_panel.h - dock_h,
                center_panel.w,
                dock_h,
            )
        } else {
            Rect::new(
                center_panel.x,
                center_panel.y + center_panel.h,
                center_panel.w,
                0.0,
            )
        };
        let bottom_tab_bar = Rect::new(
            bottom_dock.x,
            bottom_dock.y,
            bottom_dock.w,
            if state.bottom_dock_open {
                BOTTOM_TAB_H
            } else {
                0.0
            },
        );
        let bottom_content = Rect::new(
            bottom_dock.x + super::editor_theme::metrics::PANEL_PAD,
            bottom_dock.y + BOTTOM_TAB_H + super::editor_theme::metrics::PANEL_PAD,
            (bottom_dock.w - super::editor_theme::metrics::PANEL_PAD * 2.0).max(1.0),
            (bottom_dock.h - BOTTOM_TAB_H - super::editor_theme::metrics::PANEL_PAD * 2.0).max(1.0),
        );

        let left_splitter = if state.left_panel_visible {
            Rect::new(left_panel.x + left_panel.w, body_y, OUTER_GAP, body_h)
        } else {
            Rect::new(center_panel.x, body_y, 0.0, body_h)
        };
        let right_splitter = if state.right_panel_visible {
            Rect::new(right_panel.x - OUTER_GAP, body_y, OUTER_GAP, body_h)
        } else {
            Rect::new(center_panel.x + center_panel.w, body_y, 0.0, body_h)
        };
        let bottom_splitter = if state.bottom_dock_open {
            Rect::new(bottom_dock.x, bottom_dock.y - 3.0, bottom_dock.w, 6.0)
        } else {
            Rect::new(bottom_dock.x, bottom_dock.y, bottom_dock.w, 0.0)
        };

        Self {
            left_panel,
            center_panel,
            right_panel,
            bottom_dock,
            bottom_tab_bar,
            status_bar,
            list_content: inset_panel_content(left_panel),
            inspector_content: inset_panel_content(right_panel),
            bottom_content,
            left_splitter,
            right_splitter,
            bottom_splitter,
        }
    }
}

pub(crate) fn inset_panel_content(rect: Rect) -> Rect {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return Rect::new(rect.x, rect.y, 0.0, 0.0);
    }
    Rect::new(
        rect.x + super::editor_theme::metrics::PANEL_PAD,
        rect.y + PANEL_HEADER_H + super::editor_theme::metrics::PANEL_PAD,
        (rect.w - super::editor_theme::metrics::PANEL_PAD * 2.0).max(1.0),
        (rect.h - PANEL_HEADER_H - super::editor_theme::metrics::PANEL_PAD * 2.0).max(1.0),
    )
}

pub(crate) fn workspace_tab_rect(index: usize) -> Rect {
    const WIDTHS: [f32; 7] = [104.0, 112.0, 102.0, 108.0, 108.0, 122.0, 120.0];
    const GAP: f32 = 5.0;
    const MIN_TAB_W: f32 = 64.0;
    let available = (screen_width() - 16.0 - GAP * (WIDTHS.len() - 1) as f32).max(1.0);
    let natural_total: f32 = WIDTHS.iter().sum();
    let scale = (available / natural_total).min(1.0);
    let scaled = WIDTHS.map(|width| (width * scale).max(MIN_TAB_W));
    let scaled_total: f32 = scaled.iter().sum::<f32>() + GAP * (WIDTHS.len() - 1) as f32;
    let fit_scale = if scaled_total > screen_width() - 16.0 {
        ((screen_width() - 16.0 - GAP * (WIDTHS.len() - 1) as f32)
            / scaled.iter().sum::<f32>())
            .clamp(0.1, 1.0)
    } else {
        1.0
    };
    let mut x = 8.0;
    for width in scaled.iter().take(index) {
        x += *width * fit_scale + GAP;
    }
    Rect::new(
        x,
        MENU_BAR_H + 3.0,
        scaled[index.min(scaled.len() - 1)] * fit_scale,
        DOCUMENT_BAR_H - 6.0,
    )
}

pub(crate) fn bottom_tab_rect(layout: &EditorShellLayout, index: usize) -> Rect {
    let width = 92.0;
    Rect::new(
        layout.bottom_tab_bar.x + 6.0 + index as f32 * (width + 4.0),
        layout.bottom_tab_bar.y + 3.0,
        width,
        BOTTOM_TAB_H - 6.0,
    )
}

pub(crate) fn bottom_dock_toggle_rect(layout: &EditorShellLayout) -> Rect {
    Rect::new(
        layout.status_bar.x + layout.status_bar.w - 118.0,
        layout.status_bar.y + 3.0,
        110.0,
        layout.status_bar.h - 6.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_dock_overlays_without_resizing_center_canvas() {
        let closed = EditorWorkspaceShellState::default();
        let mut open = closed.clone();
        open.bottom_dock_open = true;
        let closed_layout = EditorShellLayout::calculate(1600.0, 900.0, &closed);
        let open_layout = EditorShellLayout::calculate(1600.0, 900.0, &open);
        assert_eq!(closed_layout.center_panel.x, open_layout.center_panel.x);
        assert_eq!(closed_layout.center_panel.y, open_layout.center_panel.y);
        assert_eq!(closed_layout.center_panel.w, open_layout.center_panel.w);
        assert_eq!(closed_layout.center_panel.h, open_layout.center_panel.h);
        assert!(open_layout.bottom_dock.h > 0.0);
        assert_eq!(closed_layout.bottom_dock.h, 0.0);
    }

    #[test]
    fn hidden_side_panels_expand_the_authoring_canvas() {
        let visible = EditorWorkspaceShellState::default();
        let mut hidden = visible.clone();
        hidden.left_panel_visible = false;
        hidden.right_panel_visible = false;
        let visible_layout = EditorShellLayout::calculate(1600.0, 900.0, &visible);
        let hidden_layout = EditorShellLayout::calculate(1600.0, 900.0, &hidden);
        assert!(hidden_layout.center_panel.w > visible_layout.center_panel.w);
        assert_eq!(hidden_layout.left_splitter.w, 0.0);
        assert_eq!(hidden_layout.right_splitter.w, 0.0);
    }

    #[test]
    fn visible_panels_and_bottom_dock_expose_resize_splitters() {
        let state = EditorWorkspaceShellState {
            bottom_dock_open: true,
            ..EditorWorkspaceShellState::default()
        };
        let layout = EditorShellLayout::calculate(1600.0, 900.0, &state);
        assert!(layout.left_splitter.w > 0.0);
        assert!(layout.right_splitter.w > 0.0);
        assert!(layout.bottom_splitter.h > 0.0);
    }
}
