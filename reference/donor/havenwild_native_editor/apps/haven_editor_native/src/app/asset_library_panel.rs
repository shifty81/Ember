use super::render_helpers::*;
use super::*;
use haven_assets::asset_browser::{category_label, AssetBrowserFilter, AssetReadiness};

const LIBRARY_PAGE_SIZE: usize = 6;

impl EditorApp {
    pub(crate) fn draw_asset_library(&self, rect: Rect) {
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Content Library",
            Some(&self.asset_browser.summary()),
        );
        draw_editor_widget_tone(
            library_search_rect(rect),
            if self.asset_library_filter.is_empty() {
                "Search library..."
            } else {
                &self.asset_library_filter
            },
            self.text_focus == EditorTextFocus::AssetLibraryFilter,
            WidgetTone::Quiet,
        );
        draw_editor_widget_tone(
            library_search_clear_rect(rect),
            "×",
            false,
            if self.asset_library_filter.is_empty() {
                WidgetTone::Disabled
            } else {
                WidgetTone::Quiet
            },
        );

        let packs = self.asset_browser.packs.as_slice();
        let pack_label = if self.asset_library_pack_index == 0 {
            "Pack: All".to_string()
        } else {
            packs
                .get(self.asset_library_pack_index - 1)
                .map(|pack| format!("Pack: {}", pack.display_name))
                .unwrap_or_else(|| "Pack: All".to_string())
        };
        draw_editor_widget(library_pack_rect(rect), &pack_label, false);

        let categories: Vec<_> = self.asset_browser.categories().into_iter().collect();
        let category_label_text = if self.asset_library_category_index == 0 {
            "Category: All".to_string()
        } else {
            categories
                .get(self.asset_library_category_index - 1)
                .map(|category| format!("Category: {}", category_label(category)))
                .unwrap_or_else(|| "Category: All".to_string())
        };
        draw_editor_widget(library_category_rect(rect), &category_label_text, false);
        draw_editor_widget(
            library_production_rect(rect),
            if self.asset_library_production_only {
                "Production"
            } else {
                "All states"
            },
            self.asset_library_production_only,
        );
        draw_editor_widget(
            library_ready_rect(rect),
            if self.asset_library_runtime_ready_only {
                "Ready only"
            } else {
                "Any readiness"
            },
            self.asset_library_runtime_ready_only,
        );

        let filter = self.current_asset_library_filter();
        let entries = self.asset_browser.filtered(&filter);
        let start = self
            .asset_library_list_offset
            .min(entries.len().saturating_sub(1));
        for slot in 0..LIBRARY_PAGE_SIZE {
            let Some(entry) = entries.get(start + slot) else {
                break;
            };
            let card = library_card_rect(rect, slot);
            let selected = start + slot == self.asset_library_selected;
            let secondary = format!(
                "{} | {} | {}",
                entry.pack_display_name,
                category_label(&entry.stable_ref.category),
                entry.license_id
            );
            draw_list_row(card, &entry.semantic_id, Some(&secondary), selected);
            let readiness = readiness_label(&entry.readiness);
            let badge_w = measure_editor_text(readiness, None, 11, 1.0).width + 16.0;
            draw_badge(
                Rect::new(card.x + card.w - badge_w - 7.0, card.y + 6.0, badge_w, 18.0),
                readiness,
                matches!(entry.readiness, AssetReadiness::ProductionVerified),
            );
        }

        if entries.is_empty() {
            let empty = Rect::new(rect.x, rect.y + 161.0, rect.w, 118.0);
            draw_rectangle(
                empty.x,
                empty.y,
                empty.w,
                empty.h,
                Color::new(0.05, 0.06, 0.075, 1.0),
            );
            draw_rectangle_lines(empty.x, empty.y, empty.w, empty.h, 1.0, PANEL_EDGE);
            draw_editor_text(
                "No matching assets",
                empty.x + 14.0,
                empty.y + 34.0,
                18.0,
                TEXT,
            );
            draw_wrapped(
                "Clear one or more filters, rescan mounted packs, or review reference-only and readiness settings.",
                empty.x + 14.0,
                empty.y + 58.0,
                empty.w - 28.0,
                14.0,
                MUTED,
            );
        }

        draw_editor_widget(library_prev_rect(rect), "Prev", false);
        draw_editor_widget(library_next_rect(rect), "Next", false);
        let preview_state = self
            .selected_placeable_id
            .as_deref()
            .and_then(|id| self.placeable_registry.entry(id))
            .and_then(|definition| definition.states.get(self.selected_placeable_preview_state))
            .map(String::as_str)
            .unwrap_or("default");
        draw_editor_widget(library_state_prev_rect(rect), "State -", false);
        draw_editor_widget(
            library_state_next_rect(rect),
            &format!("State +: {preview_state}"),
            false,
        );
        draw_editor_text(
            &format!(
                "{} result{} | page {} of {}",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" },
                if entries.is_empty() {
                    0
                } else {
                    start / LIBRARY_PAGE_SIZE + 1
                },
                entries.len().div_ceil(LIBRARY_PAGE_SIZE).max(1),
            ),
            rect.x,
            library_prev_rect(rect).y + 47.0,
            14.0,
            MUTED,
        );
        draw_wrapped(
            "This browser inspects all discovered categories and providers. Reference-only packs stay visible but cannot enter production resolution until their license and readiness gates pass.",
            rect.x,
            library_prev_rect(rect).y + 66.0,
            rect.w,
            14.0,
            TEXT,
        );
    }

    pub(crate) fn handle_asset_library_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }
        if library_search_rect(rect).contains(mouse) {
            self.text_focus = EditorTextFocus::AssetLibraryFilter;
            return true;
        }
        if library_search_clear_rect(rect).contains(mouse) {
            if !self.asset_library_filter.is_empty() {
                self.asset_library_filter.clear();
                self.asset_library_list_offset = 0;
                self.asset_library_selected = 0;
                self.status_message = "Content Library search cleared".to_string();
            }
            return true;
        }
        if library_pack_rect(rect).contains(mouse) {
            self.asset_library_pack_index =
                (self.asset_library_pack_index + 1) % (self.asset_browser.packs.len() + 1).max(1);
            self.asset_library_list_offset = 0;
            self.asset_library_selected = 0;
            return true;
        }
        if library_category_rect(rect).contains(mouse) {
            let count = self.asset_browser.categories().len();
            self.asset_library_category_index =
                (self.asset_library_category_index + 1) % (count + 1).max(1);
            self.asset_library_list_offset = 0;
            self.asset_library_selected = 0;
            return true;
        }
        if library_production_rect(rect).contains(mouse) {
            self.asset_library_production_only = !self.asset_library_production_only;
            self.asset_library_list_offset = 0;
            return true;
        }
        if library_ready_rect(rect).contains(mouse) {
            self.asset_library_runtime_ready_only = !self.asset_library_runtime_ready_only;
            self.asset_library_list_offset = 0;
            return true;
        }
        if library_prev_rect(rect).contains(mouse) {
            self.asset_library_list_offset = self
                .asset_library_list_offset
                .saturating_sub(LIBRARY_PAGE_SIZE);
            return true;
        }
        if library_next_rect(rect).contains(mouse) {
            let count = self
                .asset_browser
                .filtered(&self.current_asset_library_filter())
                .len();
            self.asset_library_list_offset = (self.asset_library_list_offset + LIBRARY_PAGE_SIZE)
                .min(count.saturating_sub(LIBRARY_PAGE_SIZE));
            return true;
        }
        if library_state_prev_rect(rect).contains(mouse) {
            let state_count = self
                .selected_placeable_id
                .as_deref()
                .and_then(|id| self.placeable_registry.entry(id))
                .map(|definition| definition.states.len())
                .unwrap_or(0);
            if state_count > 0 {
                self.selected_placeable_preview_state =
                    (self.selected_placeable_preview_state + state_count - 1) % state_count;
            }
            return true;
        }
        if library_state_next_rect(rect).contains(mouse) {
            let state_count = self
                .selected_placeable_id
                .as_deref()
                .and_then(|id| self.placeable_registry.entry(id))
                .map(|definition| definition.states.len())
                .unwrap_or(0);
            if state_count > 0 {
                self.selected_placeable_preview_state =
                    (self.selected_placeable_preview_state + 1) % state_count;
            }
            return true;
        }
        let count = self
            .asset_browser
            .filtered(&self.current_asset_library_filter())
            .len();
        let start = self.asset_library_list_offset.min(count.saturating_sub(1));
        for slot in 0..LIBRARY_PAGE_SIZE {
            if start + slot >= count {
                break;
            }
            if library_card_rect(rect, slot).contains(mouse) {
                self.asset_library_selected = start + slot;
                let filter = self.current_asset_library_filter();
                if let Some(entry) = self.asset_browser.filtered(&filter).get(start + slot) {
                    if let Some(placeable_id) = &entry.placeable_stable_id {
                        self.selected_placeable_id = Some(placeable_id.clone());
                        self.selected_placeable_preview_state = 0;
                        self.scene_layer_mode = SceneLayerMode::Objects;
                        self.scene_edit_tool = SceneEditTool::Place;
                    }
                    self.status_message = format!(
                        "{} from {} | {} | {}{}",
                        entry.semantic_id,
                        entry.pack_display_name,
                        category_label(&entry.stable_ref.category),
                        readiness_label(&entry.readiness),
                        entry
                            .placeable_stable_id
                            .as_ref()
                            .map(|id| format!(" | selected placeable {id}"))
                            .unwrap_or_default()
                    );
                }
                return true;
            }
        }
        true
    }

    pub(crate) fn current_asset_library_filter(&self) -> AssetBrowserFilter {
        let categories: Vec<_> = self.asset_browser.categories().into_iter().collect();
        AssetBrowserFilter {
            query: self.asset_library_filter.clone(),
            pack_id: self
                .asset_library_pack_index
                .checked_sub(1)
                .and_then(|index| self.asset_browser.packs.get(index))
                .map(|pack| pack.pack_id.clone()),
            category: self
                .asset_library_category_index
                .checked_sub(1)
                .and_then(|index| categories.get(index))
                .cloned(),
            production_only: self.asset_library_production_only,
            approved_license_only: self.asset_library_production_only,
            runtime_ready_only: self.asset_library_runtime_ready_only,
            ..Default::default()
        }
    }
}

fn readiness_label(readiness: &AssetReadiness) -> &'static str {
    match readiness {
        AssetReadiness::ManualOnly => "manual only",
        AssetReadiness::PartiallyConfigured => "partial",
        AssetReadiness::RuntimeReady => "runtime ready",
        AssetReadiness::ProductionVerified => "production verified",
        AssetReadiness::ReferenceOnly => "reference only",
    }
}

fn library_search_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 31.0, (rect.w - 30.0).max(40.0), 24.0)
}
fn library_search_clear_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 24.0, rect.y + 31.0, 24.0, 24.0)
}
fn library_pack_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 63.0, rect.w, 26.0)
}
fn library_category_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 95.0, rect.w, 26.0)
}
fn library_production_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 127.0, rect.w * 0.49, 26.0)
}
fn library_ready_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w * 0.51, rect.y + 127.0, rect.w * 0.49, 26.0)
}
fn library_card_rect(rect: Rect, slot: usize) -> Rect {
    Rect::new(rect.x, rect.y + 161.0 + slot as f32 * 48.0, rect.w, 44.0)
}
fn library_prev_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x,
        rect.y + 161.0 + LIBRARY_PAGE_SIZE as f32 * 48.0,
        70.0,
        26.0,
    )
}
fn library_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 78.0, library_prev_rect(rect).y, 70.0, 26.0)
}

fn library_state_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + 156.0, library_prev_rect(rect).y, 76.0, 26.0)
}
fn library_state_next_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x + 240.0,
        library_prev_rect(rect).y,
        (rect.w - 240.0).max(100.0),
        26.0,
    )
}
