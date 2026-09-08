use macroquad::prelude::*;

use super::editor_text::draw_editor_text;
use super::render_helpers::draw_editor_widget;
use super::{PANEL_BG, PANEL_EDGE, TEXT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldCanvasContextAction {
    OpenScene,
    AssignSelected,
    ClearAssignment,
    GenerateScene,
    GenerateIsland,
    GenerateAllIslands,
    FrameIsland,
    RefreshHarborRoute,
    MoveLeft,
    MoveUp,
    MoveDown,
    MoveRight,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WorldCanvasContextMenu {
    pub screen_position: Vec2,
    pub rectangle_index: usize,
}

pub(crate) fn context_menu_rect(menu: WorldCanvasContextMenu) -> Rect {
    let width = 224.0;
    let height = 8.0 + context_actions().len() as f32 * 30.0;
    let x = menu
        .screen_position
        .x
        .clamp(8.0, (screen_width() - width - 8.0).max(8.0));
    let y = menu
        .screen_position
        .y
        .clamp(48.0, (screen_height() - height - 8.0).max(48.0));
    Rect::new(x, y, width, height)
}

pub(crate) fn context_menu_action_at(
    menu: WorldCanvasContextMenu,
    point: Vec2,
) -> Option<WorldCanvasContextAction> {
    let rect = context_menu_rect(menu);
    if !rect.contains(point) {
        return None;
    }
    let index = ((point.y - rect.y - 4.0) / 30.0).floor() as usize;
    context_actions()
        .get(index)
        .copied()
        .map(|(_, action)| action)
}

pub(crate) fn draw_world_canvas_context_menu(menu: WorldCanvasContextMenu) {
    let rect = context_menu_rect(menu);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL_BG);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
    for (index, (label, _)) in context_actions().iter().enumerate() {
        let row = Rect::new(
            rect.x + 4.0,
            rect.y + 4.0 + index as f32 * 30.0,
            rect.w - 8.0,
            26.0,
        );
        draw_editor_widget(row, label, false);
    }
    draw_editor_text(
        "Global world-cell actions",
        rect.x + 8.0,
        rect.y - 7.0,
        13.0,
        TEXT,
    );
}

fn context_actions() -> &'static [(&'static str, WorldCanvasContextAction)] {
    &[
        (
            "Open partition in Scene Editor",
            WorldCanvasContextAction::OpenScene,
        ),
        (
            "Generate from selected template",
            WorldCanvasContextAction::AssignSelected,
        ),
        (
            "Clear assignment",
            WorldCanvasContextAction::ClearAssignment,
        ),
        (
            "Generate this scene",
            WorldCanvasContextAction::GenerateScene,
        ),
        (
            "Generate this island",
            WorldCanvasContextAction::GenerateIsland,
        ),
        (
            "Generate all islands",
            WorldCanvasContextAction::GenerateAllIslands,
        ),
        ("Frame this island", WorldCanvasContextAction::FrameIsland),
        (
            "Refresh harbor route",
            WorldCanvasContextAction::RefreshHarborRoute,
        ),
        (
            "Move surface chunk left",
            WorldCanvasContextAction::MoveLeft,
        ),
        ("Move surface chunk up", WorldCanvasContextAction::MoveUp),
        (
            "Move surface chunk down",
            WorldCanvasContextAction::MoveDown,
        ),
        (
            "Move surface chunk right",
            WorldCanvasContextAction::MoveRight,
        ),
    ]
}
