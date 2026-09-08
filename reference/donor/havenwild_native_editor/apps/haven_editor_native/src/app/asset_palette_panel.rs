use super::render_helpers::*;
use super::*;
use haven_assets::asset_palette::{AssetPaletteEntry, AssetPaletteKind, AssetPaletteTreeGroup};

const ASSET_PAGE_SIZE: usize = 4;
const ASSET_TREE_ROW_HEIGHT: f32 = 22.0;

#[derive(Clone, Debug)]
pub(crate) struct AssetPaletteDrag {
    pub stable_id: String,
    pub kind: AssetPaletteKind,
}

impl EditorApp {
    pub(crate) fn draw_asset_palette(&self, rect: Rect) {
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Asset Palette",
            Some(self.asset_category.label()),
        );
        let search = asset_search_rect(rect);
        draw_editor_widget_tone(
            search,
            if self.asset_filter.is_empty() {
                "Search assets..."
            } else {
                self.asset_filter.as_str()
            },
            self.text_focus == EditorTextFocus::AssetFilter,
            WidgetTone::Quiet,
        );
        draw_editor_widget_tone(
            asset_search_clear_rect(rect),
            "×",
            false,
            if self.asset_filter.is_empty() {
                WidgetTone::Disabled
            } else {
                WidgetTone::Quiet
            },
        );

        for (index, row) in asset_tree_rows(self.asset_category).into_iter().enumerate() {
            let row_rect = asset_tree_row_rect(rect, index);
            match row {
                AssetTreeRow::Group(group, expanded) => {
                    draw_list_row(
                        row_rect,
                        &format!("{} {}", if expanded { "▾" } else { "▸" }, group.label()),
                        Some("Folder"),
                        false,
                    );
                }
                AssetTreeRow::Category(category, depth) => {
                    let label = format!("{}{}", "  ".repeat(depth), category.label());
                    draw_list_row(row_rect, &label, None, category == self.asset_category);
                }
            }
        }
        draw_editor_widget(
            asset_favorites_rect(rect),
            if self.asset_favorites_only {
                "★ Favorites"
            } else {
                "☆ Favorites"
            },
            self.asset_favorites_only,
        );
        draw_editor_widget(asset_recent_rect(rect), "Recent", self.asset_recent_only);

        let entries = self.filtered_asset_entries();
        let start = self.asset_list_offset.min(entries.len().saturating_sub(1));
        for slot in 0..ASSET_PAGE_SIZE {
            let Some(entry) = entries.get(start + slot).copied() else {
                break;
            };
            let card = asset_card_rect(rect, slot);
            let active = self.asset_entry_is_selected(entry);
            draw_list_row(card, "", None, active);
            let thumbnail = asset_thumbnail_rect(card);
            draw_rectangle(
                thumbnail.x,
                thumbnail.y,
                thumbnail.w,
                thumbnail.h,
                Color::new(0.08, 0.09, 0.09, 1.0),
            );
            if !self
                .editor_textures
                .draw_palette_thumbnail(entry, thumbnail)
            {
                draw_rectangle_lines(
                    thumbnail.x + 2.0,
                    thumbnail.y + 2.0,
                    thumbnail.w - 4.0,
                    thumbnail.h - 4.0,
                    1.0,
                    WARN,
                );
                draw_editor_text("!", thumbnail.x + 11.0, thumbnail.y + 24.0, 18.0, WARN);
            }
            draw_scissored_text(
                &entry.label,
                card.x + 48.0,
                card.y + 18.0,
                (card.w - 78.0).max(1.0),
                14.0,
                if entry.runtime_ready() { TEXT } else { WARN },
            );
            draw_scissored_text(
                if entry.runtime_ready() {
                    entry.category.label()
                } else {
                    "Binding needed"
                },
                card.x + 48.0,
                card.y + 35.0,
                (card.w - 78.0).max(1.0),
                11.0,
                if entry.runtime_ready() { MUTED } else { WARN },
            );
            draw_editor_text(
                if self.asset_palette_state.is_favorite(&entry.stable_id) {
                    "★"
                } else {
                    "☆"
                },
                card.x + card.w - 22.0,
                card.y + 26.0,
                16.0,
                if self.asset_palette_state.is_favorite(&entry.stable_id) {
                    GOOD
                } else {
                    MUTED
                },
            );
        }

        if entries.is_empty() {
            let empty = Rect::new(rect.x, asset_favorites_rect(rect).y + 38.0, rect.w, 116.0);
            draw_rectangle(
                empty.x,
                empty.y,
                empty.w,
                empty.h,
                Color::new(0.05, 0.06, 0.075, 1.0),
            );
            draw_rectangle_lines(empty.x, empty.y, empty.w, empty.h, 1.0, PANEL_EDGE);
            draw_editor_text(
                "No palette matches",
                empty.x + 14.0,
                empty.y + 34.0,
                18.0,
                TEXT,
            );
            draw_wrapped(
                "Clear search, favorites, or recent filters to restore available tiles, objects, and stamps.",
                empty.x + 14.0,
                empty.y + 58.0,
                empty.w - 28.0,
                14.0,
                MUTED,
            );
        }

        draw_editor_widget(asset_prev_rect(rect), "Prev", false);
        draw_editor_widget(asset_next_rect(rect), "Next", false);
        draw_editor_text(
            &format!(
                "{} result{} | page {} of {}",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" },
                if entries.is_empty() {
                    0
                } else {
                    start / ASSET_PAGE_SIZE + 1
                },
                entries.len().div_ceil(ASSET_PAGE_SIZE).max(1)
            ),
            rect.x,
            asset_prev_rect(rect).y + 49.0,
            15.0,
            MUTED,
        );
        draw_wrapped(
            "Filter by name or semantic role. Click to arm Paint or Place; drag a row onto the snapped canvas. Warning rows remain searchable until their runtime binding exists.",
            rect.x,
            asset_prev_rect(rect).y + 70.0,
            rect.w,
            14.0,
            TEXT,
        );
    }

    pub(crate) fn handle_asset_palette_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }
        if asset_search_rect(rect).contains(mouse) {
            self.text_focus = EditorTextFocus::AssetFilter;
            return true;
        }
        if asset_search_clear_rect(rect).contains(mouse) {
            if !self.asset_filter.is_empty() {
                self.asset_filter.clear();
                self.asset_list_offset = 0;
                self.status_message = "Scene Asset search cleared".to_string();
            }
            return true;
        }
        for (index, row) in asset_tree_rows(self.asset_category).into_iter().enumerate() {
            if !asset_tree_row_rect(rect, index).contains(mouse) {
                continue;
            }
            let category = match row {
                AssetTreeRow::Group(group, _) => group.first_category(),
                AssetTreeRow::Category(category, _) => category,
            };
            self.asset_category = category;
            self.asset_list_offset = 0;
            self.asset_recent_only = false;
            self.status_message = format!("Asset folder: {}", category.label());
            return true;
        }
        if asset_favorites_rect(rect).contains(mouse) {
            self.asset_favorites_only = !self.asset_favorites_only;
            self.asset_list_offset = 0;
            self.status_message = if self.asset_favorites_only {
                "Showing favorite assets".to_string()
            } else {
                "Showing all matching assets".to_string()
            };
            return true;
        }
        if asset_recent_rect(rect).contains(mouse) {
            self.asset_recent_only = !self.asset_recent_only;
            self.asset_list_offset = 0;
            self.status_message = if self.asset_recent_only {
                "Showing recently used assets".to_string()
            } else {
                "Recent asset filter cleared".to_string()
            };
            return true;
        }
        if asset_prev_rect(rect).contains(mouse) {
            self.asset_list_offset = self.asset_list_offset.saturating_sub(ASSET_PAGE_SIZE);
            return true;
        }
        if asset_next_rect(rect).contains(mouse) {
            let count = self.filtered_asset_entries().len();
            self.asset_list_offset = (self.asset_list_offset + ASSET_PAGE_SIZE)
                .min(count.saturating_sub(ASSET_PAGE_SIZE));
            return true;
        }

        let entries: Vec<(String, AssetPaletteKind, String, bool, Option<String>)> = self
            .filtered_asset_entries()
            .into_iter()
            .map(|entry| {
                (
                    entry.stable_id.clone(),
                    entry.kind,
                    entry.provenance.label().to_string(),
                    entry.runtime_ready(),
                    entry.warning.clone(),
                )
            })
            .collect();
        let start = self.asset_list_offset.min(entries.len().saturating_sub(1));
        for slot in 0..ASSET_PAGE_SIZE {
            let Some((stable_id, kind, provenance, runtime_ready, warning)) =
                entries.get(start + slot).cloned()
            else {
                break;
            };
            let card = asset_card_rect(rect, slot);
            if asset_favorite_star_rect(card).contains(mouse) {
                let favorite = self.asset_palette_state.toggle_favorite(&stable_id);
                let _ = self.asset_palette_state.save_default();
                self.status_message = format!(
                    "{} {}",
                    if favorite {
                        "Favorited"
                    } else {
                        "Removed favorite"
                    },
                    stable_id
                );
                return true;
            }
            if card.contains(mouse) {
                self.select_palette_asset(stable_id.clone(), kind);
                self.asset_drag = Some(AssetPaletteDrag { stable_id, kind });
                self.status_message = warning.unwrap_or_else(|| {
                    format!(
                        "Selected palette asset ({provenance}, {}) — drag to canvas or click a cell",
                        if runtime_ready { "runtime ready" } else { "binding warning" }
                    )
                });
                return true;
            }
        }
        true
    }

    pub(crate) fn update_asset_palette_drag(&mut self) {
        if !is_mouse_button_released(MouseButton::Left) {
            return;
        }
        let Some(drag) = self.asset_drag.take() else {
            return;
        };
        if self.viewport_mode != EditorViewportMode::SceneMap {
            return;
        }
        let Some((x, y)) = self.scene_cell_at_mouse() else {
            return;
        };
        self.scene_cursor_x = x;
        self.scene_cursor_y = y;
        self.select_palette_asset(drag.stable_id, drag.kind);
        self.apply_scene_edit_tool();
    }

    pub(crate) fn draw_asset_drag_preview(&self) {
        let Some(drag) = &self.asset_drag else {
            return;
        };
        if !is_mouse_button_down(MouseButton::Left) {
            return;
        }
        let Some(entry) = self.asset_catalog.entry(&drag.stable_id) else {
            return;
        };
        let (mx, my) = mouse_position();
        let rect = Rect::new(mx + 12.0, my + 12.0, 52.0, 52.0);
        draw_rectangle(
            rect.x - 3.0,
            rect.y - 3.0,
            rect.w + 6.0,
            rect.h + 25.0,
            PANEL_BG,
        );
        draw_rectangle_lines(
            rect.x - 3.0,
            rect.y - 3.0,
            rect.w + 6.0,
            rect.h + 25.0,
            1.0,
            PANEL_EDGE,
        );
        if !self.editor_textures.draw_palette_thumbnail(entry, rect) {
            draw_editor_text("!", rect.x + 18.0, rect.y + 34.0, 28.0, WARN);
        }
        draw_scissored_text(&entry.label, rect.x, rect.y + 70.0, rect.w, 13.0, TEXT);
    }

    fn filtered_asset_entries(&self) -> Vec<&AssetPaletteEntry> {
        let mut entries = self.asset_catalog.filtered(
            self.asset_category,
            &self.asset_filter,
            self.asset_favorites_only,
            &self.asset_palette_state,
        );
        if self.asset_recent_only {
            entries.retain(|entry| self.asset_palette_state.recent().contains(&entry.stable_id));
            entries.sort_by_key(|entry| {
                self.asset_palette_state
                    .recent()
                    .iter()
                    .position(|stable_id| stable_id == &entry.stable_id)
                    .unwrap_or(usize::MAX)
            });
        }
        entries
    }

    fn select_palette_asset(&mut self, stable_id: String, kind: AssetPaletteKind) {
        match kind {
            AssetPaletteKind::Tile(tile) => {
                self.selected_stamp_id = None;
                if let Some(index) = TileKind::ALL
                    .iter()
                    .position(|candidate| *candidate == tile)
                {
                    self.selected_tile = index;
                }
                self.scene_layer_mode = SceneLayerMode::Terrain;
                self.scene_edit_tool = SceneEditTool::Paint;
            }
            AssetPaletteKind::Object(object) => {
                self.selected_stamp_id = None;
                if let Some(index) = OBJECT_BRUSHES
                    .iter()
                    .position(|candidate| *candidate == object)
                {
                    self.selected_object = index;
                }
                self.scene_layer_mode = SceneLayerMode::Objects;
                self.scene_edit_tool = SceneEditTool::Place;
            }
            AssetPaletteKind::Stamp => {
                self.selected_stamp_id = stable_id.strip_prefix("stamp/").map(str::to_string);
                self.scene_layer_mode = SceneLayerMode::Objects;
                self.scene_edit_tool = SceneEditTool::Place;
            }
        }
        self.asset_palette_state.mark_recent(&stable_id);
        let _ = self.asset_palette_state.save_default();
    }

    fn asset_entry_is_selected(&self, entry: &AssetPaletteEntry) -> bool {
        match entry.kind {
            AssetPaletteKind::Tile(tile) => {
                self.scene_layer_mode == SceneLayerMode::Terrain
                    && self.selected_tile_kind() == tile
            }
            AssetPaletteKind::Object(object) => {
                self.scene_layer_mode == SceneLayerMode::Objects
                    && self.selected_stamp_id.is_none()
                    && self.selected_object_kind() == object
            }
            AssetPaletteKind::Stamp => {
                self.scene_layer_mode == SceneLayerMode::Objects
                    && self
                        .selected_stamp_id
                        .as_deref()
                        .is_some_and(|id| entry.stable_id == format!("stamp/{id}"))
            }
        }
    }
}

fn asset_search_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 32.0, (rect.w - 30.0).max(40.0), 24.0)
}

fn asset_search_clear_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 24.0, rect.y + 32.0, 24.0, 24.0)
}

#[derive(Clone, Copy, Debug)]
enum AssetTreeRow {
    Group(AssetPaletteTreeGroup, bool),
    Category(AssetPaletteCategory, usize),
}

fn asset_tree_rows(active: AssetPaletteCategory) -> Vec<AssetTreeRow> {
    let active_group = active
        .tree_group()
        .unwrap_or(AssetPaletteTreeGroup::Terrain);
    let mut rows = vec![AssetTreeRow::Category(AssetPaletteCategory::All, 0)];
    for group in AssetPaletteTreeGroup::ALL {
        let expanded = group == active_group;
        rows.push(AssetTreeRow::Group(group, expanded));
        if expanded {
            rows.extend(
                AssetPaletteCategory::children(group)
                    .iter()
                    .copied()
                    .map(|category| AssetTreeRow::Category(category, 1)),
            );
        }
    }
    rows.push(AssetTreeRow::Category(AssetPaletteCategory::Stamps, 0));
    rows
}

fn asset_tree_row_rect(rect: Rect, index: usize) -> Rect {
    Rect::new(
        rect.x,
        rect.y + 62.0 + index as f32 * ASSET_TREE_ROW_HEIGHT,
        rect.w,
        ASSET_TREE_ROW_HEIGHT - 2.0,
    )
}

fn asset_tree_bottom(rect: Rect) -> f32 {
    // Reserve the maximum expanded tree height so switching semantic groups
    // never pushes the result list into neighboring controls.
    const MAX_TREE_ROWS: usize = 9;
    rect.y + 62.0 + MAX_TREE_ROWS as f32 * ASSET_TREE_ROW_HEIGHT
}

fn asset_favorites_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        asset_tree_bottom(rect) + 5.0,
        (rect.w - 5.0) * 0.5,
        28.0,
    )
}

fn asset_recent_rect(rect: Rect) -> Rect {
    let favorite = asset_favorites_rect(rect);
    Rect::new(
        favorite.x + favorite.w + 5.0,
        favorite.y,
        favorite.w,
        favorite.h,
    )
}

fn asset_card_rect(rect: Rect, slot: usize) -> Rect {
    Rect::new(
        rect.x,
        asset_favorites_rect(rect).y + 36.0 + slot as f32 * 48.0,
        rect.w,
        44.0,
    )
}

fn asset_thumbnail_rect(card: Rect) -> Rect {
    Rect::new(card.x + 5.0, card.y + 5.0, 34.0, 34.0)
}

fn asset_favorite_star_rect(card: Rect) -> Rect {
    Rect::new(card.x + card.w - 28.0, card.y, 28.0, 24.0)
}

fn asset_prev_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        (asset_favorites_rect(rect).y + 36.0 + ASSET_PAGE_SIZE as f32 * 48.0 + 4.0)
            .min(rect.y + rect.h - 82.0),
        (rect.w - 5.0) * 0.5,
        28.0,
    )
}

fn asset_next_rect(rect: Rect) -> Rect {
    let prev = asset_prev_rect(rect);
    Rect::new(prev.x + prev.w + 5.0, prev.y, prev.w, prev.h)
}
