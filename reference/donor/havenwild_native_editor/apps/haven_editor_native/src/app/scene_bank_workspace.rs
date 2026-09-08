use super::render_helpers::{draw_badge, draw_editor_widget, draw_list_row, draw_section_header};
use super::*;

const BANK_CARD_W: f32 = 248.0;
const BANK_CARD_H: f32 = 176.0;
const BANK_CARD_GAP: f32 = 28.0;
const BANK_COLUMNS: usize = 3;

pub(crate) fn scene_bank_indices(world: &haven_core::GameWorld) -> Vec<usize> {
    world
        .scenes
        .iter()
        .enumerate()
        .filter_map(|(index, scene)| (scene.kind != SceneKind::Exterior).then_some(index))
        .collect()
}

pub(crate) fn scene_bank_card_rect(position: usize) -> Rect {
    let column = position % BANK_COLUMNS;
    let row = position / BANK_COLUMNS;
    Rect::new(
        column as f32 * (BANK_CARD_W + BANK_CARD_GAP),
        row as f32 * (BANK_CARD_H + BANK_CARD_GAP),
        BANK_CARD_W,
        BANK_CARD_H,
    )
}

pub(crate) fn scene_bank_bounds(world: &haven_core::GameWorld) -> Rect {
    let count = scene_bank_indices(world).len().max(1);
    let rows = count.div_ceil(BANK_COLUMNS);
    Rect::new(
        -BANK_CARD_GAP,
        -BANK_CARD_GAP,
        BANK_COLUMNS as f32 * BANK_CARD_W + (BANK_COLUMNS + 1) as f32 * BANK_CARD_GAP,
        rows as f32 * BANK_CARD_H + (rows + 1) as f32 * BANK_CARD_GAP,
    )
}

impl EditorApp {
    pub(crate) fn scene_bank_viewport_rect(&self) -> Rect {
        let host = self.canvas_host_rect();
        Rect::new(host.x, host.y + 38.0, host.w, (host.h - 38.0).max(1.0))
    }

    pub(crate) fn draw_scene_bank_list(&self, rect: Rect) {
        let indices = scene_bank_indices(&self.model.world);
        let mut y = rect.y;
        draw_section_header(
            Rect::new(rect.x, y, rect.w, 24.0),
            "Scene Bank",
            Some("interiors & special scenes"),
        );
        y += 34.0;
        for scene_index in indices.into_iter().take(14) {
            let Some(scene) = self.model.world.scenes.get(scene_index) else {
                continue;
            };
            let active = scene_index == self.selected_scene;
            let row_h = 44.0;
            draw_list_row(
                Rect::new(rect.x, y - 18.0, rect.w, row_h),
                &scene.name,
                Some(&format!("{}  •  {}", scene.kind.code(), scene.id.code())),
                active,
            );
            y += row_h + 6.0;
        }
        if scene_bank_indices(&self.model.world).is_empty() {
            draw_editor_text("No off-world scenes exist yet", rect.x, y, 17.0, WARN);
        }
    }

    pub(crate) fn draw_scene_bank_workspace(&self) {
        let host = self.canvas_host_rect();
        let viewport = self.scene_bank_viewport_rect();
        let bounds = scene_bank_bounds(&self.model.world);
        draw_canvas_toolbar(
            host,
            CanvasToolbarKind::SceneBank,
            self.scene_bank_canvas.zoom_percent(),
            self.scene_bank_pan_tool,
            None,
        );
        draw_rectangle(
            viewport.x,
            viewport.y,
            viewport.w,
            viewport.h,
            Color::new(0.075, 0.085, 0.095, 1.0),
        );

        let camera = self.scene_bank_canvas.camera(viewport, bounds);
        set_camera(&camera);
        let visible = self.scene_bank_canvas.visible_world_rect(viewport, bounds);
        draw_infinite_grid(visible, 32.0, 4, self.scene_bank_canvas.zoom);
        for (position, scene_index) in scene_bank_indices(&self.model.world)
            .into_iter()
            .enumerate()
        {
            let Some(scene) = self.model.world.scenes.get(scene_index) else {
                continue;
            };
            let card = scene_bank_card_rect(position);
            let selected = scene_index == self.selected_scene;
            draw_rectangle(
                card.x,
                card.y,
                card.w,
                card.h,
                Color::new(0.085, 0.075, 0.065, 0.98),
            );
            let preview = Rect::new(card.x + 8.0, card.y + 34.0, card.w - 16.0, card.h - 62.0);
            draw_scene_into_rect(scene, preview);
            draw_rectangle(
                card.x,
                card.y,
                card.w,
                28.0,
                Color::new(0.035, 0.040, 0.048, 0.94),
            );
            draw_editor_text(&scene.name, card.x + 8.0, card.y + 20.0, 17.0, TEXT);
            draw_badge(
                Rect::new(card.x + 8.0, card.y + card.h - 25.0, 72.0, 18.0),
                scene.kind.code(),
                selected,
            );
            draw_editor_text(
                &format!("{} transitions", scene.transitions.len()),
                card.x + 88.0,
                card.y + card.h - 10.0,
                12.0,
                MUTED,
            );
            draw_rectangle_lines(
                card.x,
                card.y,
                card.w,
                card.h,
                if selected { 3.0 } else { 1.0 },
                if selected { GOOD } else { PANEL_EDGE },
            );
        }
        set_default_camera();
        draw_rectangle_lines(
            viewport.x, viewport.y, viewport.w, viewport.h, 1.0, PANEL_EDGE,
        );
        draw_editor_text(
            "Off-world scenes remain independent documents and connect through transitions",
            viewport.x + 12.0,
            viewport.y + 24.0,
            16.0,
            TEXT,
        );
    }

    pub(crate) fn draw_scene_bank_inspector(&self, rect: Rect) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            draw_editor_text("No scene selected", rect.x, rect.y, 20.0, MUTED);
            return;
        };
        if scene.kind == SceneKind::Exterior {
            draw_editor_text("Select a Scene Bank card", rect.x, rect.y, 20.0, MUTED);
            return;
        }
        let mut y = rect.y;
        draw_editor_text(&scene.name, rect.x, y + 22.0, 24.0, TEXT);
        draw_badge(
            Rect::new(rect.x, y + 32.0, 92.0, 20.0),
            scene.kind.code(),
            true,
        );
        y += 66.0;
        draw_section_header(Rect::new(rect.x, y, rect.w, 24.0), "Scene Details", None);
        y += 34.0;
        for line in [
            format!("ID: {}", scene.id.code()),
            format!("Kind: {}", scene.kind.code()),
            format!("Biome: {}", scene.biome.label()),
            format!("Objects: {}", scene.map.objects.len()),
            format!("Multi-tile stamps: {}", scene.map.stamps.len()),
            format!("Transitions: {}", scene.transitions.len()),
            format!("Document: {} x {} tiles", MAP_W, MAP_H),
        ] {
            draw_editor_text(&line, rect.x, y, 17.0, TEXT);
            y += 24.0;
        }
        y += 6.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 34.0),
            "Open in Scene Editor",
            false,
        );
        y += 52.0;
        draw_section_header(Rect::new(rect.x, y, rect.w, 24.0), "Scene Bank Rules", None);
        y += 34.0;
        for line in [
            "Not placed on overworld terrain",
            "Connected by authored transitions",
            "Editable with normal scene tools",
            "Saved inside the shared world document",
        ] {
            draw_editor_text(line, rect.x, y, 15.0, MUTED);
            y += 21.0;
        }
    }

    pub(crate) fn handle_scene_bank_canvas_click(&mut self, mx: f32, my: f32) -> bool {
        let viewport = self.scene_bank_viewport_rect();
        let point = vec2(mx, my);
        if !viewport.contains(point) {
            return false;
        }
        let bounds = scene_bank_bounds(&self.model.world);
        let world_point = self
            .scene_bank_canvas
            .screen_to_world(viewport, bounds, point);
        for (position, scene_index) in scene_bank_indices(&self.model.world)
            .into_iter()
            .enumerate()
        {
            if scene_bank_card_rect(position).contains(world_point) {
                let scene_name = self.model.world.scenes[scene_index].name.clone();
                self.select_scene_index(scene_index);
                self.status_message = format!("Selected Scene Bank entry {scene_name}");
                return true;
            }
        }
        true
    }

    pub(crate) fn handle_scene_bank_list_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let point = vec2(mx, my);
        if !rect.contains(point) {
            return false;
        }
        let relative = my - (rect.y + 12.0);
        if relative < 0.0 {
            return true;
        }
        let position = (relative / 50.0).floor() as usize;
        if let Some(scene_index) = scene_bank_indices(&self.model.world).get(position).copied() {
            let scene_name = self.model.world.scenes[scene_index].name.clone();
            self.select_scene_index(scene_index);
            self.status_message = format!("Selected Scene Bank entry {scene_name}");
        }
        true
    }

    pub(crate) fn handle_scene_bank_inspector_click(&mut self, mx: f32, my: f32) -> bool {
        let rect = self.inspector_content_rect();
        let button = Rect::new(rect.x, rect.y + 190.0, rect.w, 34.0);
        if button.contains(vec2(mx, my)) {
            self.open_selected_scene_bank_scene();
            return true;
        }
        false
    }

    pub(crate) fn cycle_scene_bank_selection(&mut self, delta: i32) {
        let indices = scene_bank_indices(&self.model.world);
        if indices.is_empty() {
            return;
        }
        let current = indices
            .iter()
            .position(|index| *index == self.selected_scene)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(indices.len() as i32) as usize;
        self.select_scene_index(indices[next]);
    }

    pub(crate) fn open_selected_scene_bank_scene(&mut self) {
        let Some(scene) = self.model.world.scenes.get(self.selected_scene) else {
            return;
        };
        if scene.kind == SceneKind::Exterior {
            self.status_message = "Select an interior, cave, dungeon, or special scene".to_string();
            return;
        }
        let name = scene.name.clone();
        self.viewport_mode = EditorViewportMode::SceneMap;
        self.status_message = format!("Opened {name} in Scene Editor");
    }
}
