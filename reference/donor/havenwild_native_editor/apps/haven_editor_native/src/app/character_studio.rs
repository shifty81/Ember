use super::render_helpers::*;
use super::*;
use haven_assets::universal_lpc_character_authority::{
    UniversalLpcCharacterAuthority, UniversalLpcCharacterRecord,
    DEFAULT_UNIVERSAL_LPC_AUTHORITY_PATH, DEFAULT_UNIVERSAL_LPC_SOURCE_MOUNT,
};

const CHARACTER_ROWS: usize = 18;
const CHARACTER_ROW_H: f32 = 27.0;
const CHARACTER_MODES: [CharacterStudioMode; 4] = [
    CharacterStudioMode::Player,
    CharacterStudioMode::Npc,
    CharacterStudioMode::Population,
    CharacterStudioMode::Validation,
];
const SEX_FILTERS: [CharacterSexFilter; 2] = [CharacterSexFilter::Male, CharacterSexFilter::Female];
const AGE_FILTERS: [CharacterAgeFilter; 4] = [
    CharacterAgeFilter::Child,
    CharacterAgeFilter::Teen,
    CharacterAgeFilter::Adult,
    CharacterAgeFilter::Elder,
];
const SLOT_FILTERS: [&str; 15] = [
    "all", "body", "head", "hair", "eyes", "beards", "torso", "legs", "feet", "hat", "weapon",
    "shield", "tools", "wings", "tail",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharacterStudioMode {
    Player,
    Npc,
    Population,
    Validation,
}

impl CharacterStudioMode {
    fn label(self) -> &'static str {
        match self {
            Self::Player => "Player",
            Self::Npc => "NPC",
            Self::Population => "Population",
            Self::Validation => "Validation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharacterSexFilter {
    Male,
    Female,
}

impl CharacterSexFilter {
    fn label(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CharacterAgeFilter {
    Child,
    Teen,
    Adult,
    Elder,
}

impl CharacterAgeFilter {
    fn label(self) -> &'static str {
        match self {
            Self::Child => "Child",
            Self::Teen => "Teen",
            Self::Adult => "Adult",
            Self::Elder => "Elder",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Child => "child",
            Self::Teen => "teen",
            Self::Adult => "adult",
            Self::Elder => "elderly",
        }
    }
}

pub(crate) struct CharacterStudioState {
    authority: Option<UniversalLpcCharacterAuthority>,
    load_error: Option<String>,
    pub mode: CharacterStudioMode,
    pub sex: CharacterSexFilter,
    pub age: CharacterAgeFilter,
    pub include_share_alike: bool,
    pub query: String,
    slot_filter: usize,
    filtered_indices: Vec<usize>,
    selected: usize,
    list_offset: usize,
}

impl CharacterStudioState {
    pub(crate) fn new() -> Self {
        let mut state = Self {
            authority: None,
            load_error: None,
            mode: CharacterStudioMode::Player,
            sex: CharacterSexFilter::Male,
            age: CharacterAgeFilter::Adult,
            include_share_alike: false,
            query: String::new(),
            slot_filter: 0,
            filtered_indices: Vec::new(),
            selected: 0,
            list_offset: 0,
        };
        state.reload();
        state
    }

    pub(crate) fn reload(&mut self) {
        match UniversalLpcCharacterAuthority::load_default() {
            Ok(authority) => {
                self.authority = Some(authority);
                self.load_error = None;
            }
            Err(error) => {
                self.authority = None;
                self.load_error = Some(error);
            }
        }
        self.rebuild_filter();
    }

    pub(crate) fn rebuild_filter(&mut self) {
        self.filtered_indices.clear();
        let Some(authority) = self.authority.as_ref() else {
            self.selected = 0;
            self.list_offset = 0;
            return;
        };
        for (index, record) in authority.records.iter().enumerate() {
            if !record.is_selectable(self.include_share_alike)
                || !record.source_matches(&self.query)
                || !self.sex_compatible(record)
                || !self.age_compatible(record)
                || !self.slot_compatible(record)
                || !self.mode_compatible(record)
            {
                continue;
            }
            self.filtered_indices.push(index);
        }
        self.selected = self
            .selected
            .min(self.filtered_indices.len().saturating_sub(1));
        self.list_offset = self
            .list_offset
            .min(self.filtered_indices.len().saturating_sub(CHARACTER_ROWS));
    }

    fn sex_compatible(&self, record: &UniversalLpcCharacterRecord) -> bool {
        let has_male = record.has_tag("male");
        let has_female = record.has_tag("female");
        if !has_male && !has_female {
            return true;
        }
        record.has_tag(self.sex.tag())
    }

    fn age_compatible(&self, record: &UniversalLpcCharacterRecord) -> bool {
        let explicit = ["child", "teen", "elderly"]
            .into_iter()
            .any(|tag| record.has_tag(tag));
        if !explicit {
            return true;
        }
        record.has_tag(self.age.tag())
    }

    fn slot_compatible(&self, record: &UniversalLpcCharacterRecord) -> bool {
        let filter = SLOT_FILTERS[self.slot_filter];
        filter == "all"
            || record.has_tag(filter)
            || record.category.eq_ignore_ascii_case(filter)
            || record.source.to_ascii_lowercase().contains(filter)
    }

    fn mode_compatible(&self, record: &UniversalLpcCharacterRecord) -> bool {
        if self.mode != CharacterStudioMode::Player {
            return true;
        }
        !["zombie", "skeleton", "lizard", "wings", "tail"]
            .into_iter()
            .any(|tag| record.has_tag(tag))
    }

    pub(crate) fn selected_record(&self) -> Option<&UniversalLpcCharacterRecord> {
        let authority = self.authority.as_ref()?;
        let record_index = *self.filtered_indices.get(self.selected)?;
        authority.records.get(record_index)
    }

    pub(crate) fn authority(&self) -> Option<&UniversalLpcCharacterAuthority> {
        self.authority.as_ref()
    }

    pub(crate) fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub(crate) fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub(crate) fn selected_visible_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn set_selected(&mut self, index: usize) {
        if self.filtered_indices.is_empty() {
            self.selected = 0;
            return;
        }
        self.selected = index.min(self.filtered_indices.len() - 1);
        if self.selected < self.list_offset {
            self.list_offset = self.selected;
        } else if self.selected >= self.list_offset + CHARACTER_ROWS {
            self.list_offset = self.selected + 1 - CHARACTER_ROWS;
        }
    }

    pub(crate) fn cycle_selected(&mut self, delta: i32) {
        let next = cycle_index(self.selected, self.filtered_indices.len(), delta);
        self.set_selected(next);
    }

    pub(crate) fn page(&mut self, delta: i32) {
        if delta < 0 {
            self.list_offset = self.list_offset.saturating_sub(CHARACTER_ROWS);
        } else {
            self.list_offset = (self.list_offset + CHARACTER_ROWS)
                .min(self.filtered_indices.len().saturating_sub(CHARACTER_ROWS));
        }
        self.set_selected(self.list_offset);
    }

    pub(crate) fn cycle_slot(&mut self, delta: i32) {
        self.slot_filter = cycle_index(self.slot_filter, SLOT_FILTERS.len(), delta);
        self.rebuild_filter();
    }

    pub(crate) fn slot_label(&self) -> &'static str {
        SLOT_FILTERS[self.slot_filter]
    }
}

pub(crate) fn character_search_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y, rect.w, 30.0)
}

pub(crate) fn character_row_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x,
        rect.y + 64.0 + row as f32 * CHARACTER_ROW_H,
        rect.w,
        CHARACTER_ROW_H - 2.0,
    )
}

pub(crate) fn character_prev_page_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 32.0, (rect.w - 6.0) * 0.5, 30.0)
}

pub(crate) fn character_next_page_rect(rect: Rect) -> Rect {
    Rect::new(
        rect.x + (rect.w - 6.0) * 0.5 + 6.0,
        rect.y + rect.h - 32.0,
        (rect.w - 6.0) * 0.5,
        30.0,
    )
}

fn center_button_rect(rect: Rect, row: usize, column: usize, columns: usize) -> Rect {
    let gap = 6.0;
    let width = (rect.w - gap * (columns.saturating_sub(1)) as f32) / columns as f32;
    Rect::new(
        rect.x + column as f32 * (width + gap),
        rect.y + row as f32 * 36.0,
        width,
        30.0,
    )
}

pub(crate) fn character_reload_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 34.0, rect.w, 30.0)
}

impl EditorApp {
    pub(crate) fn draw_character_catalog(&self, rect: Rect) {
        let search = character_search_rect(rect);
        draw_rectangle(search.x, search.y, search.w, search.h, CONTROL_BG);
        draw_rectangle_lines(
            search.x,
            search.y,
            search.w,
            search.h,
            1.0,
            if self.text_focus == EditorTextFocus::CharacterAssetFilter {
                editor_theme::colors::SELECTION_OUTLINE
            } else {
                PANEL_EDGE
            },
        );
        let text = if self.character_studio.query.is_empty() {
            "Filter components..."
        } else {
            &self.character_studio.query
        };
        draw_scissored_text(
            text,
            search.x + 8.0,
            search.y + 20.0,
            search.w - 16.0,
            13.0,
            MUTED,
        );
        draw_editor_text(
            &format!(
                "{} compatible | {}",
                self.character_studio.filtered_count(),
                self.character_studio.slot_label()
            ),
            rect.x,
            rect.y + 52.0,
            13.0,
            MUTED,
        );

        let Some(authority) = self.character_studio.authority() else {
            draw_wrapped(
                self.character_studio
                    .load_error()
                    .unwrap_or("Universal LPC authority is unavailable."),
                rect.x,
                rect.y + 84.0,
                rect.w,
                15.0,
                WARN,
            );
            return;
        };

        for row in 0..CHARACTER_ROWS {
            let visible_index = self.character_studio.list_offset + row;
            let Some(record_index) = self.character_studio.filtered_indices.get(visible_index)
            else {
                break;
            };
            let record = &authority.records[*record_index];
            let row_rect = character_row_rect(rect, row);
            draw_editor_widget_tone(
                row_rect,
                &format!(
                    "{}{}",
                    if record.share_alike_required {
                        "[SA] "
                    } else {
                        ""
                    },
                    record.display_name()
                ),
                visible_index == self.character_studio.selected_visible_index(),
                if record.share_alike_required {
                    WidgetTone::Quiet
                } else {
                    WidgetTone::Standard
                },
            );
        }
        draw_editor_widget(character_prev_page_rect(rect), "Previous", false);
        draw_editor_widget(character_next_page_rect(rect), "Next", false);
    }

    pub(crate) fn draw_character_studio_workspace(&self, rect: Rect) {
        let content = Rect::new(rect.x + 14.0, rect.y + 14.0, rect.w - 28.0, rect.h - 28.0);
        for (index, mode) in CHARACTER_MODES.into_iter().enumerate() {
            draw_editor_widget(
                center_button_rect(content, 0, index, CHARACTER_MODES.len()),
                mode.label(),
                self.character_studio.mode == mode,
            );
        }
        for (index, sex) in SEX_FILTERS.into_iter().enumerate() {
            draw_editor_widget(
                center_button_rect(content, 1, index, 2),
                sex.label(),
                self.character_studio.sex == sex,
            );
        }
        for (index, age) in AGE_FILTERS.into_iter().enumerate() {
            draw_editor_widget(
                center_button_rect(content, 2, index, AGE_FILTERS.len()),
                age.label(),
                self.character_studio.age == age,
            );
        }
        draw_editor_widget(center_button_rect(content, 3, 0, 3), "Previous Slot", false);
        draw_editor_widget(
            center_button_rect(content, 3, 1, 3),
            self.character_studio.slot_label(),
            true,
        );
        draw_editor_widget(center_button_rect(content, 3, 2, 3), "Next Slot", false);
        draw_editor_widget(
            center_button_rect(content, 4, 0, 2),
            "Preferred Only",
            !self.character_studio.include_share_alike,
        );
        draw_editor_widget(
            center_button_rect(content, 4, 1, 2),
            "Include CC-BY-SA",
            self.character_studio.include_share_alike,
        );

        let preview = Rect::new(
            content.x,
            content.y + 198.0,
            content.w,
            (content.h - 292.0).max(180.0),
        );
        draw_rectangle(
            preview.x,
            preview.y,
            preview.w,
            preview.h,
            editor_theme::colors::CANVAS_SURROUND,
        );
        draw_rectangle_lines(preview.x, preview.y, preview.w, preview.h, 1.0, PANEL_EDGE);
        if let Some(record) = self.character_studio.selected_record() {
            draw_editor_text(
                "Selected Universal LPC layer",
                preview.x + 18.0,
                preview.y + 30.0,
                20.0,
                TEXT,
            );
            draw_scissored_text(
                &record.source,
                preview.x + 18.0,
                preview.y + 58.0,
                preview.w - 36.0,
                16.0,
                TEXT,
            );
            let badge = if record.share_alike_required {
                "CC-BY-SA: commercial use allowed; release bundle required"
            } else {
                "Preferred commercial license pool"
            };
            draw_editor_text(
                badge,
                preview.x + 18.0,
                preview.y + 88.0,
                15.0,
                if record.share_alike_required {
                    WARN
                } else {
                    GOOD
                },
            );
            draw_wrapped(
                "Source-native preview and recipe composition use the mounted Universal LPC repository. This workspace filters the complete catalog without copying tens of thousands of source sheets into the permanent project. Pixel edits create Havenwild-owned overrides and invalidate only affected composites.",
                preview.x + 18.0,
                preview.y + 124.0,
                preview.w - 36.0,
                16.0,
                MUTED,
            );
        } else {
            draw_editor_text(
                "No compatible layer selected",
                preview.x + 18.0,
                preview.y + 36.0,
                20.0,
                WARN,
            );
        }

        let summary_y = preview.y + preview.h + 24.0;
        if let Some(authority) = self.character_studio.authority() {
            for (index, line) in [
                format!("Authority records: {}", authority.credit_record_count),
                format!("Preferred commercial: {}", authority.preferred_count()),
                format!("Conditional ShareAlike: {}", authority.conditional_count()),
                format!("Source commit: {}", authority.source_commit),
            ]
            .into_iter()
            .enumerate()
            {
                draw_editor_text(
                    &line,
                    content.x,
                    summary_y + index as f32 * 22.0,
                    15.0,
                    MUTED,
                );
            }
        }
    }

    pub(crate) fn draw_character_studio_inspector(&self, rect: Rect) {
        let mut y = rect.y;
        draw_editor_text("Character Recipe Source", rect.x, y, 22.0, TEXT);
        y += 30.0;
        for line in [
            format!("Mode: {}", self.character_studio.mode.label()),
            format!("Sex: {}", self.character_studio.sex.label()),
            format!("Age: {}", self.character_studio.age.label()),
            format!(
                "Pool: {}",
                if self.character_studio.include_share_alike {
                    "Preferred + ShareAlike"
                } else {
                    "Preferred only"
                }
            ),
            format!(
                "Compatible records: {}",
                self.character_studio.filtered_count()
            ),
        ] {
            draw_editor_text(&line, rect.x, y, 16.0, TEXT);
            y += 23.0;
        }
        y += 8.0;
        if let Some(record) = self.character_studio.selected_record() {
            draw_editor_text("Selected Layer", rect.x, y, 20.0, TEXT);
            y += 26.0;
            draw_wrapped(&record.source, rect.x, y, rect.w, 15.0, TEXT);
            y += 52.0;
            for line in [
                format!("Category: {}", record.category),
                format!("License tier: {}", record.license_tier),
                format!(
                    "Selected license: {}",
                    record.selected_license.as_deref().unwrap_or("blocked")
                ),
                format!("Authors: {}", record.authors.len()),
                format!("Source links: {}", record.urls.len()),
                format!(
                    "Mounted: {}",
                    self.character_studio
                        .authority()
                        .map(|authority| authority.source_path(record).is_file())
                        .unwrap_or(false)
                ),
            ] {
                draw_editor_text(
                    &line,
                    rect.x,
                    y,
                    15.0,
                    if record.share_alike_required {
                        WARN
                    } else {
                        MUTED
                    },
                );
                y += 22.0;
            }
            y += 8.0;
            draw_editor_text("Tags", rect.x, y, 18.0, TEXT);
            y += 24.0;
            draw_wrapped(&record.tags.join(", "), rect.x, y, rect.w, 14.0, MUTED);
            y += 72.0;
            draw_editor_text("Source mount", rect.x, y, 18.0, TEXT);
            y += 24.0;
            draw_wrapped(
                DEFAULT_UNIVERSAL_LPC_SOURCE_MOUNT,
                rect.x,
                y,
                rect.w,
                14.0,
                MUTED,
            );
        } else if let Some(error) = self.character_studio.load_error() {
            draw_wrapped(error, rect.x, y, rect.w, 15.0, WARN);
            y += 84.0;
            draw_editor_text("Generate authority with:", rect.x, y, 17.0, TEXT);
            y += 24.0;
            draw_wrapped(
                "tools/automation/characters/Bootstrap-UniversalLpcGenerator.cmd",
                rect.x,
                y,
                rect.w,
                14.0,
                MUTED,
            );
        }
        draw_editor_widget(
            character_reload_rect(rect),
            "Reload Universal LPC Authority",
            false,
        );
        draw_editor_text(
            DEFAULT_UNIVERSAL_LPC_AUTHORITY_PATH,
            rect.x,
            character_reload_rect(rect).y - 10.0,
            11.0,
            MUTED,
        );
    }

    pub(crate) fn update_character_studio_input(&mut self) {
        if is_key_pressed(KeyCode::Down) {
            self.character_studio.cycle_selected(1);
        }
        if is_key_pressed(KeyCode::Up) {
            self.character_studio.cycle_selected(-1);
        }
        if is_key_pressed(KeyCode::PageDown) {
            self.character_studio.page(1);
        }
        if is_key_pressed(KeyCode::PageUp) {
            self.character_studio.page(-1);
        }
        if is_key_pressed(KeyCode::R) {
            self.character_studio.reload();
            self.status_message = "Reloaded Universal LPC character authority".to_string();
        }
    }

    pub(crate) fn handle_character_studio_click(&mut self, mx: f32, my: f32) -> bool {
        let layout = self.shell_layout();
        let mouse = vec2(mx, my);
        let list = layout.list_content;
        if character_search_rect(list).contains(mouse) {
            self.text_focus = EditorTextFocus::CharacterAssetFilter;
            return true;
        }
        for row in 0..CHARACTER_ROWS {
            if character_row_rect(list, row).contains(mouse) {
                let index = self.character_studio.list_offset + row;
                self.character_studio.set_selected(index);
                return true;
            }
        }
        if character_prev_page_rect(list).contains(mouse) {
            self.character_studio.page(-1);
            return true;
        }
        if character_next_page_rect(list).contains(mouse) {
            self.character_studio.page(1);
            return true;
        }

        let content = Rect::new(
            layout.center_panel.x + 14.0,
            layout.center_panel.y + 14.0,
            layout.center_panel.w - 28.0,
            layout.center_panel.h - 28.0,
        );
        for (index, mode) in CHARACTER_MODES.into_iter().enumerate() {
            if center_button_rect(content, 0, index, CHARACTER_MODES.len()).contains(mouse) {
                self.character_studio.mode = mode;
                self.character_studio.rebuild_filter();
                return true;
            }
        }
        for (index, sex) in SEX_FILTERS.into_iter().enumerate() {
            if center_button_rect(content, 1, index, 2).contains(mouse) {
                self.character_studio.sex = sex;
                self.character_studio.rebuild_filter();
                return true;
            }
        }
        for (index, age) in AGE_FILTERS.into_iter().enumerate() {
            if center_button_rect(content, 2, index, AGE_FILTERS.len()).contains(mouse) {
                self.character_studio.age = age;
                self.character_studio.rebuild_filter();
                return true;
            }
        }
        if center_button_rect(content, 3, 0, 3).contains(mouse) {
            self.character_studio.cycle_slot(-1);
            return true;
        }
        if center_button_rect(content, 3, 2, 3).contains(mouse) {
            self.character_studio.cycle_slot(1);
            return true;
        }
        if center_button_rect(content, 4, 0, 2).contains(mouse) {
            self.character_studio.include_share_alike = false;
            self.character_studio.rebuild_filter();
            return true;
        }
        if center_button_rect(content, 4, 1, 2).contains(mouse) {
            self.character_studio.include_share_alike = true;
            self.character_studio.rebuild_filter();
            return true;
        }
        if character_reload_rect(layout.inspector_content).contains(mouse) {
            self.character_studio.reload();
            self.status_message = "Reloaded Universal LPC character authority".to_string();
            return true;
        }
        false
    }
}
