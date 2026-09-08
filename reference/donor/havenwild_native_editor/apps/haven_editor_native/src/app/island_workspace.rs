use std::collections::BTreeMap;

use super::render_helpers::{draw_editor_widget, draw_list_row, draw_section_header, graph_point};
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct LandmassSummary {
    pub id: i32,
    pub name: String,
    pub rectangle_indices: Vec<usize>,
    pub preview_bounds: Rect,
}

pub(crate) fn landmass_summaries(manifest: &SceneRectangleManifest) -> Vec<LandmassSummary> {
    let mut grouped: BTreeMap<i32, LandmassSummary> = BTreeMap::new();
    for (index, rectangle) in manifest.scene_rectangles.iter().enumerate() {
        if !rectangle_is_overworld_surface(rectangle) {
            continue;
        }
        let source = rectangle.world_rect_preview_px;
        let source_rect = Rect::new(
            source[0] as f32,
            source[1] as f32,
            source[2] as f32,
            source[3] as f32,
        );
        let entry = grouped
            .entry(rectangle.landmass_id)
            .or_insert_with(|| LandmassSummary {
                id: rectangle.landmass_id,
                name: rectangle.landmass_name.clone(),
                rectangle_indices: Vec::new(),
                preview_bounds: source_rect,
            });
        entry.rectangle_indices.push(index);
        entry.preview_bounds = union_rect(entry.preview_bounds, source_rect);
    }
    grouped.into_values().collect()
}

fn union_rect(left: Rect, right: Rect) -> Rect {
    let min_x = left.x.min(right.x);
    let min_y = left.y.min(right.y);
    let max_x = (left.x + left.w).max(right.x + right.w);
    let max_y = (left.y + left.h).max(right.y + right.h);
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn preview_bounds(manifest: &SceneRectangleManifest) -> Option<Rect> {
    landmass_summaries(manifest)
        .into_iter()
        .map(|summary| summary.preview_bounds)
        .reduce(union_rect)
}

fn harbor_node_id_for_landmass(landmass_id: i32) -> RegionNodeId {
    if landmass_id == 0 {
        RegionNodeId::new("future_harbor")
    } else {
        RegionNodeId::new(format!("island_harbor_{landmass_id}"))
    }
}

fn map_preview_rect(source: Rect, source_bounds: Rect, target: Rect) -> Rect {
    let scale_x = target.w / source_bounds.w.max(1.0);
    let scale_y = target.h / source_bounds.h.max(1.0);
    Rect::new(
        target.x + (source.x - source_bounds.x) * scale_x,
        target.y + (source.y - source_bounds.y) * scale_y,
        source.w * scale_x,
        source.h * scale_y,
    )
}

fn assigned_scene<'a>(
    assignments: &SceneRectangleAssignmentsFile,
    world: &'a haven_core::GameWorld,
    rectangle_id: &str,
) -> Option<&'a SceneMap> {
    let assignment = assignments.assignment_for_rectangle(rectangle_id)?;
    world.scene_by_id(&ProjectSceneId::new(assignment.scene_code.as_str()))
}

impl EditorApp {
    pub(crate) fn draw_landmass_list(&self, rect: Rect) {
        let Some(manifest) = &self.scene_rectangles else {
            draw_editor_text("No island manifest loaded", rect.x, rect.y, 18.0, WARN);
            return;
        };
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Islands",
            Some("Alderreach & archipelago"),
        );
        let mut y = rect.y + 32.0;
        for summary in landmass_summaries(manifest) {
            let active = summary.id == self.selected_landmass_id;
            let row = Rect::new(rect.x, y, rect.w, 46.0);
            let detail = format!("{} surface chunks", summary.rectangle_indices.len());
            draw_list_row(row, &summary.name, Some(&detail), active);
            y += 50.0;
        }
    }

    pub(crate) fn handle_landmass_list_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        if !rect.contains(vec2(mx, my)) {
            return false;
        }
        let summaries = self
            .scene_rectangles
            .as_ref()
            .map(landmass_summaries)
            .unwrap_or_default();
        let relative = my - (rect.y + 32.0);
        if relative >= 0.0 {
            let index = (relative / 50.0).floor() as usize;
            if let Some(landmass_id) = summaries.get(index).map(|summary| summary.id) {
                self.select_landmass(landmass_id);
            }
        }
        true
    }

    pub(crate) fn select_landmass(&mut self, landmass_id: i32) {
        self.selected_landmass_id = landmass_id;
        let first_rectangle = self.scene_rectangles.as_ref().and_then(|manifest| {
            manifest.scene_rectangles.iter().position(|rectangle| {
                rectangle.landmass_id == landmass_id && rectangle_is_overworld_surface(rectangle)
            })
        });
        if let Some(index) = first_rectangle {
            self.selected_rectangle = index;
            if let Some(rectangle) = self
                .scene_rectangles
                .as_ref()
                .and_then(|manifest| manifest.scene_rectangles.get(index))
            {
                self.world_cursor_x = rectangle.grid_x.unwrap_or(0) * MAP_W as i32;
                self.world_cursor_y = rectangle.grid_y.unwrap_or(0) * MAP_H as i32;
            }
            self.sync_assignment_cycles_to_selected_rectangle();
        }
        let harbor_node_id = harbor_node_id_for_landmass(landmass_id);
        if let Some(node_index) = self
            .model
            .region_graph
            .nodes
            .iter()
            .position(|node| node.id == harbor_node_id)
        {
            self.select_region_node_index(node_index);
        }
        self.status_message = self
            .selected_landmass_summary()
            .map(|summary| format!("Selected {} for island-level editing", summary.name))
            .unwrap_or_else(|| format!("Selected landmass {landmass_id}"));
    }

    pub(crate) fn cycle_landmass_selection(&mut self, delta: i32) {
        let summaries = self
            .scene_rectangles
            .as_ref()
            .map(landmass_summaries)
            .unwrap_or_default();
        if summaries.is_empty() {
            return;
        }
        let current = summaries
            .iter()
            .position(|summary| summary.id == self.selected_landmass_id)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(summaries.len() as i32) as usize;
        let landmass_id = summaries[next].id;
        self.select_landmass(landmass_id);
    }

    pub(crate) fn move_selected_scene_cell(&mut self, delta_x: i32, delta_y: i32) {
        let Some(manifest) = self.scene_rectangles.as_mut() else {
            self.status_message = "Island layout manifest is unavailable".to_string();
            return;
        };
        let Some(selected) = manifest.scene_rectangles.get(self.selected_rectangle) else {
            return;
        };
        if !rectangle_is_overworld_surface(selected) {
            self.status_message = "Only exterior surface chunks can be moved".to_string();
            return;
        }
        let landmass_id = selected.landmass_id;
        let current_x = selected.grid_x.unwrap_or_default();
        let current_y = selected.grid_y.unwrap_or_default();
        let target_x = current_x + delta_x;
        let target_y = current_y + delta_y;
        let occupied = manifest
            .scene_rectangles
            .iter()
            .enumerate()
            .any(|(index, rectangle)| {
                index != self.selected_rectangle
                    && rectangle.landmass_id == landmass_id
                    && rectangle.grid_x == Some(target_x)
                    && rectangle.grid_y == Some(target_y)
            });
        if occupied {
            self.status_message = format!(
                "Cannot move surface chunk to {target_x},{target_y}: that island grid position is occupied"
            );
            return;
        }
        let selected = &mut manifest.scene_rectangles[self.selected_rectangle];
        selected.grid_x = Some(target_x);
        selected.grid_y = Some(target_y);
        selected.world_rect_preview_px[0] += delta_x * selected.world_rect_preview_px[2];
        selected.world_rect_preview_px[1] += delta_y * selected.world_rect_preview_px[3];
        let scene_id = selected.scene_id.clone();
        let landmass_name = selected.landmass_name.clone();
        self.world_canvas.reset();
        self.status_message = format!(
            "Moved {scene_id} to {target_x},{target_y} on {landmass_name}; Save All persists the island assembly"
        );
        self.command_bus.record_event(self.app_command(
            EditorCommandKind::SceneMutation,
            self.status_message.clone(),
        ));
    }

    pub(crate) fn selected_landmass_summary(&self) -> Option<LandmassSummary> {
        landmass_summaries(self.scene_rectangles.as_ref()?)
            .into_iter()
            .find(|summary| summary.id == self.selected_landmass_id)
    }

    pub(crate) fn draw_world_routes_workspace(&self, rect: Rect) {
        let Some(manifest) = &self.scene_rectangles else {
            draw_editor_text(
                "No island manifest loaded",
                rect.x + 16.0,
                rect.y + 52.0,
                20.0,
                WARN,
            );
            return;
        };
        let inner = Rect::new(rect.x + 16.0, rect.y + 42.0, rect.w - 32.0, rect.h - 58.0);
        draw_rectangle(
            inner.x,
            inner.y,
            inner.w,
            inner.h,
            Color::new(0.055, 0.17, 0.28, 1.0),
        );
        let Some(source_bounds) = preview_bounds(manifest) else {
            return;
        };

        for link in self
            .model
            .region_graph
            .links
            .iter()
            .filter(|link| link.kind == RegionLinkKind::SeaRoute)
        {
            let Some(from) = self.model.region_graph.node(&link.from) else {
                continue;
            };
            let Some(to) = self.model.region_graph.node(&link.to) else {
                continue;
            };
            let a = graph_point(inner, from.position.x, from.position.y);
            let b = graph_point(inner, to.position.x, to.position.y);
            draw_line(a.x, a.y, b.x, b.y, 3.0, Color::new(0.35, 0.75, 0.92, 0.85));
        }

        for summary in landmass_summaries(manifest) {
            let destination_bounds = map_preview_rect(summary.preview_bounds, source_bounds, inner);
            for rectangle_index in &summary.rectangle_indices {
                let rectangle = &manifest.scene_rectangles[*rectangle_index];
                let source = rectangle.world_rect_preview_px;
                let target = map_preview_rect(
                    Rect::new(
                        source[0] as f32,
                        source[1] as f32,
                        source[2] as f32,
                        source[3] as f32,
                    ),
                    source_bounds,
                    inner,
                );
                if let Some(scene) = assigned_scene(
                    &self.scene_assignments,
                    &self.model.world,
                    &rectangle.scene_id,
                ) {
                    draw_scene_into_rect(scene, target);
                } else {
                    draw_rectangle(
                        target.x,
                        target.y,
                        target.w,
                        target.h,
                        Color::new(0.10, 0.23, 0.24, 0.92),
                    );
                }
                draw_rectangle_lines(target.x, target.y, target.w, target.h, 0.7, PANEL_EDGE);
            }
            let selected = summary.id == self.selected_landmass_id;
            draw_rectangle_lines(
                destination_bounds.x,
                destination_bounds.y,
                destination_bounds.w,
                destination_bounds.h,
                if selected { 3.0 } else { 1.2 },
                if selected { TEXT } else { PANEL_EDGE },
            );
            let label_w = measure_editor_text(&summary.name, None, 16, 1.0).width + 14.0;
            draw_rectangle(
                destination_bounds.x,
                destination_bounds.y - 22.0,
                label_w,
                21.0,
                Color::new(0.03, 0.04, 0.05, 0.88),
            );
            draw_editor_text(
                &summary.name,
                destination_bounds.x + 7.0,
                destination_bounds.y - 6.0,
                16.0,
                TEXT,
            );
        }

        for node in self.model.region_graph.nodes.iter().filter(|node| {
            matches!(
                node.kind,
                RegionNodeKind::FutureHarbor | RegionNodeKind::IslandHarbor
            )
        }) {
            let point = graph_point(inner, node.position.x, node.position.y);
            draw_circle(point.x, point.y, 6.0, WARN);
            draw_circle_lines(point.x, point.y, 8.0, 2.0, TEXT);
        }

        draw_rectangle_lines(inner.x, inner.y, inner.w, inner.h, 1.0, PANEL_EDGE);
        draw_editor_text(
            "Archipelago preview: assembled generated scenes with harbor-to-harbor sea routes",
            inner.x + 12.0,
            inner.y + 24.0,
            16.0,
            TEXT,
        );
    }

    pub(crate) fn handle_world_routes_click(&mut self, mx: f32, my: f32) -> bool {
        let viewport = self.main_viewport_rect();
        let inner = Rect::new(
            viewport.x + 16.0,
            viewport.y + 42.0,
            viewport.w - 32.0,
            viewport.h - 58.0,
        );
        if !inner.contains(vec2(mx, my)) {
            return false;
        }
        let Some((source_bounds, summaries)) =
            self.scene_rectangles.as_ref().and_then(|manifest| {
                preview_bounds(manifest).map(|bounds| (bounds, landmass_summaries(manifest)))
            })
        else {
            return true;
        };
        for summary in summaries {
            let target = map_preview_rect(summary.preview_bounds, source_bounds, inner);
            if target.contains(vec2(mx, my)) {
                self.select_landmass(summary.id);
                return true;
            }
        }
        true
    }

    pub(crate) fn apply_custom_harbor_routes(&mut self) {
        let routes = self.harbor_routes.routes.clone();
        for route in routes {
            let from = harbor_node_id_for_landmass(route.from_landmass_id);
            let to = harbor_node_id_for_landmass(route.to_landmass_id);
            let endpoints_exist = self.model.region_graph.node(&from).is_some()
                && self.model.region_graph.node(&to).is_some();
            if !endpoints_exist {
                continue;
            }
            let exists = self.model.region_graph.links.iter().any(|link| {
                link.kind == RegionLinkKind::SeaRoute
                    && ((link.from == from && link.to == to)
                        || (link.from == to && link.to == from))
            });
            if !exists {
                self.model.region_graph.links.push(RegionLink {
                    from,
                    to,
                    kind: RegionLinkKind::SeaRoute,
                });
            }
        }
    }

    pub(crate) fn begin_harbor_route(&mut self) {
        let harbor_id = harbor_node_id_for_landmass(self.selected_landmass_id);
        if self.model.region_graph.node(&harbor_id).is_none() {
            self.status_message =
                "Generate the selected island first so its harbor exists".to_string();
            return;
        }
        self.route_source_landmass_id = Some(self.selected_landmass_id);
        let name = self
            .selected_landmass_summary()
            .map(|summary| summary.name)
            .unwrap_or_else(|| format!("Island {}", self.selected_landmass_id));
        self.status_message =
            format!("Route start set to {name}. Select another island and choose Connect Route.");
    }

    pub(crate) fn connect_harbor_route_to_selected(&mut self) {
        let Some(source_id) = self.route_source_landmass_id else {
            self.status_message = "Choose Begin Route on the source island first".to_string();
            return;
        };
        let destination_id = self.selected_landmass_id;
        if source_id == destination_id {
            self.status_message = "Select a different destination island".to_string();
            return;
        }
        let from = harbor_node_id_for_landmass(source_id);
        let to = harbor_node_id_for_landmass(destination_id);
        if self.model.region_graph.node(&from).is_none()
            || self.model.region_graph.node(&to).is_none()
        {
            self.status_message =
                "Both islands must be generated before their harbors can be linked".to_string();
            return;
        }
        let added = self.harbor_routes.connect(source_id, destination_id);
        self.apply_custom_harbor_routes();
        self.route_source_landmass_id = None;
        self.status_message = if added {
            format!(
                "Connected harbor route between landmasses {source_id} and {destination_id}; Save All persists it"
            )
        } else {
            "That harbor route already exists".to_string()
        };
    }

    pub(crate) fn draw_world_routes_inspector(&self, rect: Rect) {
        let Some(summary) = self.selected_landmass_summary() else {
            draw_editor_text("No island selected", rect.x, rect.y, 20.0, MUTED);
            return;
        };
        let harbor_node_id = harbor_node_id_for_landmass(summary.id);
        let harbor_node = self
            .model
            .region_graph
            .nodes
            .iter()
            .find(|node| node.id == harbor_node_id);
        let route_start = self
            .route_source_landmass_id
            .and_then(|id| {
                self.scene_rectangles
                    .as_ref()
                    .and_then(|manifest| {
                        landmass_summaries(manifest)
                            .into_iter()
                            .find(|entry| entry.id == id)
                    })
                    .map(|entry| entry.name)
            })
            .unwrap_or_else(|| "not set".to_string());
        let mut y = rect.y;
        draw_editor_text(&summary.name, rect.x, y, 28.0, TEXT);
        y += 36.0;
        let generation = self
            .scene_rectangles
            .as_ref()
            .map(|manifest| &manifest.archipelago_generation);
        for line in [
            format!("Landmass ID: {}", summary.id),
            format!("Scene cells: {}", summary.rectangle_indices.len()),
            format!(
                "World seed: {}",
                generation.map(|settings| settings.seed).unwrap_or(1_337)
            ),
            format!(
                "Island spacing: {}px minimum",
                generation
                    .map(|settings| settings.minimum_island_gap_px)
                    .unwrap_or(120)
            ),
            format!(
                "Harbor: {}",
                harbor_node
                    .map(|node| node.label.as_str())
                    .unwrap_or("not generated")
            ),
            format!("Route start: {route_start}"),
            "Preview: assembled editable scene maps".to_string(),
        ] {
            draw_editor_text(&line, rect.x, y, 17.0, TEXT);
            y += 24.0;
        }
        y += 12.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Edit Island Layout",
            false,
        );
        y += 38.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Regenerate Current Seed",
            false,
        );
        y += 38.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Reroll Archipelago Seed",
            false,
        );
        y += 38.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Begin Route Here",
            false,
        );
        y += 38.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Connect Route to Selected",
            false,
        );
        y += 38.0;
        draw_editor_widget(
            Rect::new(rect.x, y, rect.w, 32.0),
            "Refresh Harbor Node",
            false,
        );
        y += 38.0;
        draw_editor_widget(Rect::new(rect.x, y, rect.w, 32.0), "Save All", false);
        y += 48.0;
        draw_editor_text("World Routes", rect.x, y, 20.0, TEXT);
        y += 27.0;
        for line in [
            "Select islands from the map or list",
            "Reroll creates a new collision-safe seed layout",
            "Open World Editor to inspect the global tile surface",
            "Save All writes seed, layout, routes, world, and PNGs",
        ] {
            draw_editor_text(line, rect.x, y, 15.0, MUTED);
            y += 21.0;
        }
    }

    pub(crate) fn handle_world_routes_inspector_click(&mut self, mx: f32, my: f32) -> bool {
        let rect = self.inspector_content_rect();
        let first_y = rect.y + 216.0;
        let edit = Rect::new(rect.x, first_y, rect.w, 32.0);
        let regenerate = Rect::new(rect.x, first_y + 38.0, rect.w, 32.0);
        let reroll = Rect::new(rect.x, first_y + 76.0, rect.w, 32.0);
        let begin_route = Rect::new(rect.x, first_y + 114.0, rect.w, 32.0);
        let connect_route = Rect::new(rect.x, first_y + 152.0, rect.w, 32.0);
        let refresh = Rect::new(rect.x, first_y + 190.0, rect.w, 32.0);
        let save = Rect::new(rect.x, first_y + 228.0, rect.w, 32.0);
        if edit.contains(vec2(mx, my)) {
            self.viewport_mode = EditorViewportMode::SceneRectangles;
            self.frame_selected_landmass();
            self.status_message = "Opened selected island in World Editor".to_string();
            return true;
        }
        if regenerate.contains(vec2(mx, my)) {
            self.regenerate_current_archipelago_seed();
            return true;
        }
        if reroll.contains(vec2(mx, my)) {
            self.reroll_structural_archipelago();
            return true;
        }
        if begin_route.contains(vec2(mx, my)) {
            self.begin_harbor_route();
            return true;
        }
        if connect_route.contains(vec2(mx, my)) {
            self.connect_harbor_route_to_selected();
            return true;
        }
        if refresh.contains(vec2(mx, my)) {
            self.refresh_selected_harbor_route();
            return true;
        }
        if save.contains(vec2(mx, my)) {
            self.save_all_editor_documents();
            return true;
        }
        false
    }
}
