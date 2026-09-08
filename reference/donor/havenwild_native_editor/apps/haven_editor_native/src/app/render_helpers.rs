use super::*;
use haven_core::GameWorld;

pub(crate) fn cycle_index(current: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    if delta >= 0 {
        (current + delta as usize) % len
    } else {
        (current + len - ((-delta) as usize % len)) % len
    }
}

pub(crate) use super::ui_shell::workspace_tab_rect;

pub(crate) fn draw_top_bar(
    w: f32,
    mode: EditorViewportMode,
    active_title: &str,
    active_dirty: bool,
) {
    draw_rectangle(
        0.0,
        0.0,
        w,
        super::ui_shell::TOP_BAR_H,
        editor_theme::colors::TOP_BAR_BG,
    );
    draw_rectangle(
        0.0,
        super::ui_shell::MENU_BAR_H,
        w,
        super::ui_shell::DOCUMENT_BAR_H,
        editor_theme::colors::PANEL_HEADER,
    );
    draw_line(
        0.0,
        super::ui_shell::MENU_BAR_H,
        w,
        super::ui_shell::MENU_BAR_H,
        1.0,
        editor_theme::colors::BORDER_SUBTLE,
    );
    draw_line(
        0.0,
        super::ui_shell::TOP_BAR_H - 1.0,
        w,
        super::ui_shell::TOP_BAR_H - 1.0,
        1.0,
        editor_theme::colors::BORDER_STRONG,
    );

    draw_editor_text(
        "HAVENWILD NATIVE EDITOR",
        220.0,
        25.0,
        16.0,
        editor_theme::colors::TEXT_PRIMARY,
    );

    for (index, candidate) in [
        EditorViewportMode::RegionGraph,
        EditorViewportMode::SceneRectangles,
        EditorViewportMode::SceneBank,
        EditorViewportMode::SceneMap,
        EditorViewportMode::PixelStudio,
        EditorViewportMode::AnimationStudio,
        EditorViewportMode::CharacterStudio,
    ]
    .into_iter()
    .enumerate()
    {
        let rect = workspace_tab_rect(index);
        let label = if candidate == mode && active_dirty {
            format!("{} *", candidate.label())
        } else {
            candidate.label().to_string()
        };
        draw_tab_widget(rect, &label, candidate == mode);
    }

    let last = workspace_tab_rect(6);
    let title_x = last.x + last.w + 14.0;
    let available = (w - title_x - 12.0).max(0.0);
    if available > 100.0 {
        draw_scissored_text(
            active_title,
            title_x,
            super::ui_shell::MENU_BAR_H + 23.0,
            available,
            12.0,
            editor_theme::colors::TEXT_SECONDARY,
        );
    }
}

pub(crate) fn draw_scissored_text(text: &str, x: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let mut shown = text.to_string();
    while !shown.is_empty() && measure_editor_text(&shown, None, size as u16, 1.0).width > max_w {
        shown.pop();
    }
    if shown.len() < text.len() && shown.len() > 3 {
        shown.truncate(shown.len() - 3);
        shown.push_str("...");
    }
    draw_editor_text(&shown, x, y, size, color);
}

pub(crate) use super::scene_render_helpers::{
    draw_scene_grid_overlay, draw_scene_tilemap, scene_tile_color, zone_preview_color,
    SceneTilemapDraw,
};

pub(crate) fn draw_panel(rect: Rect, title: &str) {
    draw_rectangle(
        rect.x + 3.0,
        rect.y + 4.0,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.18),
    );
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL_BG);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        super::ui_shell::PANEL_HEADER_H,
        editor_theme::colors::PANEL_HEADER,
    );
    draw_rectangle(
        rect.x,
        rect.y + super::ui_shell::PANEL_HEADER_H - 1.0,
        rect.w,
        1.0,
        editor_theme::colors::BORDER_STRONG,
    );
    draw_editor_text(title, rect.x + 11.0, rect.y + 20.0, 15.0, TEXT);
}

pub(crate) fn draw_section_header(rect: Rect, label: &str, detail: Option<&str>) {
    draw_editor_text(label, rect.x, rect.y + 16.0, 15.0, TEXT);
    if let Some(detail) = detail {
        let width = measure_editor_text(detail, None, 11, 1.0).width;
        draw_editor_text(detail, rect.x + rect.w - width, rect.y + 15.0, 11.0, MUTED);
    }
    draw_line(
        rect.x,
        rect.y + 23.0,
        rect.x + rect.w,
        rect.y + 23.0,
        1.0,
        Color::new(0.15, 0.19, 0.24, 1.0),
    );
}

pub(crate) fn draw_list_row(rect: Rect, primary: &str, secondary: Option<&str>, selected: bool) {
    let mouse = vec2(mouse_position().0, mouse_position().1);
    let hovered = rect.contains(mouse);
    let bg = if selected {
        editor_theme::colors::SELECTION_FILL
    } else if hovered {
        Color::new(0.105, 0.125, 0.155, 1.0)
    } else {
        Color::new(0.065, 0.077, 0.094, 1.0)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if selected || hovered {
            Color::new(0.31, 0.48, 0.65, 1.0)
        } else {
            PANEL_EDGE
        },
    );
    if selected {
        draw_rectangle(
            rect.x,
            rect.y,
            3.0,
            rect.h,
            editor_theme::colors::SELECTION_OUTLINE,
        );
    }
    draw_scissored_text(
        primary,
        rect.x + 10.0,
        rect.y
            + if secondary.is_some() {
                18.0
            } else {
                rect.h * 0.62
            },
        (rect.w - 20.0).max(1.0),
        15.0,
        TEXT,
    );
    if let Some(secondary) = secondary {
        draw_scissored_text(
            secondary,
            rect.x + 10.0,
            rect.y + 35.0,
            (rect.w - 20.0).max(1.0),
            12.0,
            MUTED,
        );
    }
}

pub(crate) fn draw_badge(rect: Rect, label: &str, active: bool) {
    let bg = if active {
        Color::new(0.13, 0.31, 0.43, 1.0)
    } else {
        Color::new(0.09, 0.105, 0.13, 1.0)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
    let width = measure_editor_text(label, None, 11, 1.0).width;
    draw_editor_text(
        label,
        rect.x + (rect.w - width) * 0.5,
        rect.y + rect.h - 6.0,
        11.0,
        if active { TEXT } else { MUTED },
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WidgetTone {
    Standard,
    Primary,
    Quiet,
    Destructive,
    Disabled,
}

pub(crate) fn draw_editor_widget(rect: Rect, label: &str, active: bool) {
    draw_editor_widget_tone(
        rect,
        label,
        active,
        if active {
            WidgetTone::Primary
        } else {
            WidgetTone::Standard
        },
    );
}

pub(crate) fn draw_editor_widget_tone(rect: Rect, label: &str, active: bool, tone: WidgetTone) {
    let mouse = vec2(mouse_position().0, mouse_position().1);
    let enabled = tone != WidgetTone::Disabled;
    let hovered = enabled && rect.contains(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let (base, hover, pressed_color, active_color, text_color) = match tone {
        WidgetTone::Standard => (
            CONTROL_BG,
            editor_theme::colors::CONTROL_HOVER,
            editor_theme::colors::ACCENT_PRESSED,
            ACCENT,
            TEXT,
        ),
        WidgetTone::Primary => (
            editor_theme::colors::ACCENT_PRESSED,
            editor_theme::colors::ACCENT_HOVER,
            editor_theme::colors::ACCENT_HOVER,
            ACCENT,
            TEXT,
        ),
        WidgetTone::Quiet => (
            Color::new(0.055, 0.066, 0.082, 1.0),
            Color::new(0.10, 0.12, 0.15, 1.0),
            editor_theme::colors::CONTROL_HOVER,
            Color::new(0.11, 0.20, 0.28, 1.0),
            MUTED,
        ),
        WidgetTone::Destructive => (
            Color::new(0.24, 0.085, 0.085, 1.0),
            Color::new(0.34, 0.10, 0.10, 1.0),
            Color::new(0.46, 0.12, 0.12, 1.0),
            Color::new(0.40, 0.10, 0.10, 1.0),
            TEXT,
        ),
        WidgetTone::Disabled => (
            Color::new(0.045, 0.052, 0.064, 1.0),
            Color::new(0.045, 0.052, 0.064, 1.0),
            Color::new(0.045, 0.052, 0.064, 1.0),
            Color::new(0.045, 0.052, 0.064, 1.0),
            Color::new(0.34, 0.37, 0.42, 1.0),
        ),
    };
    let bg = if pressed {
        pressed_color
    } else if active {
        active_color
    } else if hovered {
        hover
    } else {
        base
    };
    let edge = if hovered || active {
        Color::new(0.34, 0.48, 0.62, 1.0)
    } else {
        PANEL_EDGE
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, edge);
    if active {
        draw_rectangle(
            rect.x,
            rect.y,
            3.0,
            rect.h,
            editor_theme::colors::SELECTION_OUTLINE,
        );
    }
    draw_scissored_text(
        label,
        rect.x + 8.0,
        rect.y + rect.h - 8.0,
        (rect.w - 14.0).max(1.0),
        13.0,
        text_color,
    );
}

pub(crate) fn draw_tab_widget(rect: Rect, label: &str, selected: bool) {
    draw_editor_widget_tone(
        rect,
        label,
        selected,
        if selected {
            WidgetTone::Primary
        } else {
            WidgetTone::Quiet
        },
    );
    if selected {
        draw_rectangle(
            rect.x,
            rect.y + rect.h - 2.0,
            rect.w,
            2.0,
            Color::new(0.55, 0.78, 1.0, 1.0),
        );
    }
}

pub(crate) fn draw_text_field(rect: Rect, label: &str, value: &str, focused: bool) {
    draw_editor_text(label, rect.x, rect.y - 6.0, 14.0, MUTED);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, CONTROL_BG);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if focused { 2.0 } else { 1.0 },
        if focused { ACCENT } else { PANEL_EDGE },
    );
    draw_scissored_text(
        value,
        rect.x + 8.0,
        rect.y + 20.0,
        rect.w - 16.0,
        16.0,
        TEXT,
    );
    if focused && ((get_time() * 2.0) as i64 % 2 == 0) {
        let width = measure_editor_text(value, None, 16, 1.0)
            .width
            .min(rect.w - 18.0);
        draw_line(
            rect.x + 9.0 + width,
            rect.y + 7.0,
            rect.x + 9.0 + width,
            rect.y + rect.h - 7.0,
            1.0,
            TEXT,
        );
    }
}

#[allow(dead_code)]
pub(crate) fn draw_region_graph(
    graph: &IslandRegionGraph,
    world: &GameWorld,
    rect: Rect,
    selected_node: Option<&RegionNodeId>,
) {
    let inner = Rect::new(rect.x + 16.0, rect.y + 42.0, rect.w - 32.0, rect.h - 58.0);
    draw_island_landmass(graph, inner);

    for link in &graph.links {
        let Some(from) = graph.node(&link.from) else {
            continue;
        };
        let Some(to) = graph.node(&link.to) else {
            continue;
        };
        let a = graph_point(inner, from.position.x, from.position.y);
        let b = graph_point(inner, to.position.x, to.position.y);
        let color = link_color(link.kind);
        draw_line(
            a.x,
            a.y,
            b.x,
            b.y,
            if matches!(
                link.kind,
                RegionLinkKind::FutureRoute | RegionLinkKind::SeaRoute
            ) {
                2.0
            } else {
                4.0
            },
            color,
        );
    }

    for node in &graph.nodes {
        let p = graph_point(inner, node.position.x, node.position.y);
        let is_selected = selected_node == Some(&node.id);
        if let Some(scene) = node
            .scene_id
            .as_ref()
            .and_then(|scene_id| world.scene(scene_id))
        {
            draw_scene_world_thumbnail(scene, p, is_selected);
        } else {
            let radius = if is_selected { 12.0 } else { 9.0 };
            draw_circle(p.x, p.y, radius, node_color(node.kind));
            draw_circle_lines(p.x, p.y, radius, 2.0, TEXT);
        }
        let label_w = node.label.len() as f32 * 6.5 + 12.0;
        draw_rectangle(
            p.x - label_w / 2.0,
            p.y + 15.0,
            label_w,
            17.0,
            Color::new(0.08, 0.06, 0.04, 0.75),
        );
        draw_editor_text(
            &node.label,
            p.x - label_w / 2.0 + 6.0,
            p.y + 28.0,
            14.0,
            TEXT,
        );
    }
}

pub(crate) fn draw_scene_world_thumbnail(scene: &SceneMap, center: Vec2, selected: bool) {
    const SCALE: f32 = 1.5;
    let width = MAP_W as f32 * SCALE;
    let height = MAP_H as f32 * SCALE;
    let origin_x = center.x - width * 0.5;
    let origin_y = center.y - height * 0.5;
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            draw_rectangle(
                origin_x + x as f32 * SCALE,
                origin_y + y as f32 * SCALE,
                SCALE + 0.2,
                SCALE + 0.2,
                scene_tile_color(scene.map.get(x, y)),
            );
        }
    }
    draw_rectangle_lines(
        origin_x,
        origin_y,
        width,
        height,
        if selected { 3.0 } else { 1.0 },
        if selected { TEXT } else { PANEL_EDGE },
    );
}

pub(crate) fn draw_island_landmass(graph: &IslandRegionGraph, rect: Rect) {
    let center = graph_point(rect, graph.landmass.center.x, graph.landmass.center.y);
    let rx = graph.landmass.radius.x * rect.w;
    let ry = graph.landmass.radius.y * rect.h;
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.13, 0.31, 0.45, 1.0),
    );

    for y in (rect.y as i32..(rect.y + rect.h) as i32).step_by(4) {
        for x in (rect.x as i32..(rect.x + rect.w) as i32).step_by(4) {
            let fx = x as f32;
            let fy = y as f32;
            let nx = (fx - center.x) / rx.max(1.0);
            let ny = (fy - center.y) / ry.max(1.0);
            let wobble = (fx * 0.035).sin() * 0.08 + (fy * 0.041).cos() * 0.08;
            let d = nx * nx + ny * ny + wobble;
            if d <= 1.08 {
                let color = if d > 0.86 {
                    Color::new(0.75, 0.62, 0.33, 1.0)
                } else if d < 0.26 && fy < center.y {
                    Color::new(0.44, 0.45, 0.41, 1.0)
                } else if fx > center.x + rx * 0.28 && fy < center.y + ry * 0.2 {
                    Color::new(0.23, 0.48, 0.29, 1.0)
                } else {
                    Color::new(0.34, 0.61, 0.33, 1.0)
                };
                draw_rectangle(fx, fy, 4.0, 4.0, color);
            }
        }
    }
}

pub(crate) fn graph_point(rect: Rect, x: f32, y: f32) -> Vec2 {
    vec2(rect.x + x * rect.w, rect.y + y * rect.h)
}

pub(crate) fn node_color(kind: RegionNodeKind) -> Color {
    match kind {
        RegionNodeKind::Hub => Color::new(0.70, 0.42, 0.23, 1.0),
        RegionNodeKind::Connector => Color::new(0.55, 0.45, 0.32, 1.0),
        RegionNodeKind::Farm => Color::new(0.54, 0.36, 0.21, 1.0),
        RegionNodeKind::Forage => Color::new(0.18, 0.45, 0.26, 1.0),
        RegionNodeKind::CaveEntrance => Color::new(0.32, 0.32, 0.32, 1.0),
        RegionNodeKind::Cave => Color::new(0.18, 0.18, 0.20, 1.0),
        RegionNodeKind::FutureHarbor => Color::new(0.22, 0.49, 0.70, 1.0),
        RegionNodeKind::IslandHarbor => Color::new(0.18, 0.68, 0.82, 1.0),
    }
}

pub(crate) fn link_color(kind: RegionLinkKind) -> Color {
    match kind {
        RegionLinkKind::Road | RegionLinkKind::FieldPath => Color::new(0.55, 0.45, 0.32, 1.0),
        RegionLinkKind::Trail => Color::new(0.42, 0.50, 0.32, 1.0),
        RegionLinkKind::CaveRoute => Color::new(0.34, 0.34, 0.34, 1.0),
        RegionLinkKind::FutureRoute => Color::new(0.93, 0.85, 0.70, 0.55),
        RegionLinkKind::SeaRoute => Color::new(0.30, 0.78, 0.94, 0.85),
    }
}

pub(crate) fn system_status_color(status: EditorSystemStatus) -> Color {
    match status {
        EditorSystemStatus::Shared => GOOD,
        EditorSystemStatus::Partial => WARN,
        EditorSystemStatus::Missing => Color::new(0.90, 0.44, 0.36, 1.0),
    }
}

pub(crate) fn draw_validation_report(report: &EditorValidationReport, rect: Rect) {
    let mut y = rect.y;
    let clean = report.is_clean();
    draw_editor_text(
        if clean {
            "All Rust editor validation sections are clean."
        } else {
            "Validation issues found."
        },
        rect.x,
        y,
        19.0,
        if clean { GOOD } else { WARN },
    );
    y += 28.0;
    for section in &report.sections {
        let status = if section.is_clean() {
            "ok".to_string()
        } else {
            format!("{} issue(s)", section.messages.len())
        };
        draw_editor_text(
            &format!("{}: {}", section.title, status),
            rect.x,
            y,
            17.0,
            if section.is_clean() { GOOD } else { WARN },
        );
        y += 23.0;
    }
}

pub(crate) fn draw_wrapped(text: &str, x: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let mut line = String::new();
    let mut y = y;
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if measure_editor_text(&candidate, None, size as u16, 1.0).width > max_w && !line.is_empty()
        {
            draw_editor_text(&line, x, y, size, color);
            y += size + 5.0;
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        draw_editor_text(&line, x, y, size, color);
    }
}
