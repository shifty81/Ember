use super::render_helpers::*;
use super::*;
use haven_assets::asset_intake::{
    repo_root_dir, AssetIntakeCatalog, AssetIntakeTargetKind, AssetLicenseStatus,
    AssetPromotionState,
};
use haven_assets::user_asset_registry::user_asset_manifest_modified;
use std::process::Command;

impl EditorApp {
    pub(crate) fn draw_asset_intake(&self, rect: Rect) {
        draw_section_header(
            Rect::new(rect.x, rect.y, rect.w, 24.0),
            "Asset Intake",
            Some(&self.asset_intake_catalog.summary()),
        );
        draw_editor_widget(intake_scan_rect(rect), "Scan Inbox", false);
        draw_editor_widget(intake_save_rect(rect), "Save", false);

        if self.asset_intake_catalog.recipes.is_empty() {
            let empty = Rect::new(rect.x, rect.y + 82.0, rect.w, 126.0);
            draw_rectangle(
                empty.x,
                empty.y,
                empty.w,
                empty.h,
                Color::new(0.05, 0.06, 0.075, 1.0),
            );
            draw_rectangle_lines(empty.x, empty.y, empty.w, empty.h, 1.0, PANEL_EDGE);
            draw_editor_text(
                "Intake inbox is empty",
                empty.x + 14.0,
                empty.y + 34.0,
                18.0,
                TEXT,
            );
            draw_wrapped(
                "Place PNG files in assets/source/intake, then choose Scan Inbox. Every new record remains license-blocked until reviewed.",
                empty.x + 14.0,
                empty.y + 58.0,
                empty.w - 28.0,
                14.0,
                MUTED,
            );
            return;
        }

        let index = self
            .asset_intake_selected
            .min(self.asset_intake_catalog.recipes.len().saturating_sub(1));
        let recipe = &self.asset_intake_catalog.recipes[index];
        draw_editor_widget(intake_prev_rect(rect), "<", false);
        draw_editor_widget(intake_next_rect(rect), ">", false);
        draw_scissored_text(
            &format!(
                "{} / {}  {}",
                index + 1,
                self.asset_intake_catalog.recipes.len(),
                recipe.display_name
            ),
            rect.x + 42.0,
            rect.y + 104.0,
            rect.w - 84.0,
            16.0,
            TEXT,
        );
        let preview = intake_source_preview_rect(rect);
        draw_scissored_text(
            &recipe.source_path,
            rect.x,
            rect.y + 128.0,
            rect.w - preview.w - 8.0,
            13.0,
            MUTED,
        );
        draw_scissored_text(
            &recipe.stable_id,
            rect.x,
            rect.y + 146.0,
            rect.w - preview.w - 8.0,
            13.0,
            MUTED,
        );
        draw_rectangle(
            preview.x,
            preview.y,
            preview.w,
            preview.h,
            Color::new(0.08, 0.08, 0.08, 1.0),
        );
        if !self
            .editor_textures
            .draw_intake_source_preview(recipe, preview)
        {
            draw_editor_text("reload", preview.x + 8.0, preview.y + 34.0, 13.0, WARN);
            draw_rectangle_lines(preview.x, preview.y, preview.w, preview.h, 1.0, PANEL_EDGE);
        }

        draw_section_header(
            Rect::new(rect.x, rect.y + 154.0, rect.w, 24.0),
            "Runtime Target",
            Some(recipe.target.kind.label()),
        );
        draw_editor_widget(
            intake_target_kind_rect(rect),
            recipe.target.kind.label(),
            false,
        );
        draw_editor_widget(intake_target_prev_rect(rect), "<", false);
        draw_editor_widget(intake_target_next_rect(rect), ">", false);
        draw_scissored_text(
            &recipe.target.label(),
            rect.x + 92.0,
            rect.y + 204.0,
            rect.w - 170.0,
            15.0,
            GOOD,
        );

        draw_section_header(
            Rect::new(rect.x, rect.y + 216.0, rect.w, 24.0),
            "License Gate",
            Some(recipe.license.status.label()),
        );
        draw_editor_widget(intake_license_prev_rect(rect), "<", false);
        draw_editor_widget(intake_license_next_rect(rect), ">", false);
        draw_scissored_text(
            recipe.license.status.label(),
            rect.x + 42.0,
            rect.y + 266.0,
            rect.w - 84.0,
            15.0,
            if recipe.license.status.permits_promotion() {
                GOOD
            } else {
                WARN
            },
        );
        draw_editor_widget(
            intake_accept_rect(rect),
            if recipe.license.accepted {
                "License Accepted"
            } else {
                "Confirm License"
            },
            recipe.license.accepted,
        );
        draw_editor_widget(
            intake_promotion_rect(rect),
            recipe.promotion_state.label(),
            recipe.promotion_state == AssetPromotionState::Approved,
        );

        draw_section_header(
            Rect::new(rect.x, rect.y + 310.0, rect.w * 0.48, 24.0),
            "Slice",
            Some("Pixels"),
        );
        draw_intake_value_row(rect, 0, "X", recipe.slice.x as i32);
        draw_intake_value_row(rect, 1, "Y", recipe.slice.y as i32);
        draw_intake_value_row(rect, 2, "W", recipe.slice.width as i32);
        draw_intake_value_row(rect, 3, "H", recipe.slice.height as i32);

        draw_section_header(
            Rect::new(rect.x + rect.w * 0.52, rect.y + 310.0, rect.w * 0.48, 24.0),
            "Pivot",
            Some("Normalized"),
        );
        draw_intake_pivot_row(rect, 0, "X", recipe.pivot.x);
        draw_intake_pivot_row(rect, 1, "Y", recipe.pivot.y);

        draw_section_header(
            Rect::new(rect.x, rect.y + 478.0, rect.w, 24.0),
            "Footprint",
            Some(self.asset_intake_footprint_target.label()),
        );
        for (index, target) in [
            FootprintEditTarget::Visual,
            FootprintEditTarget::Collision,
            FootprintEditTarget::Interaction,
        ]
        .into_iter()
        .enumerate()
        {
            draw_editor_widget(
                intake_footprint_target_rect(rect, index),
                target.label(),
                target == self.asset_intake_footprint_target,
            );
        }
        let footprint = intake_footprint(recipe, self.asset_intake_footprint_target);
        for (row, (label, value)) in [
            ("Offset X", footprint[0]),
            ("Offset Y", footprint[1]),
            ("Width", footprint[2]),
            ("Height", footprint[3]),
        ]
        .into_iter()
        .enumerate()
        {
            draw_intake_footprint_row(rect, row, label, value);
        }

        draw_editor_widget(intake_validate_rect(rect), "Validate", false);
        draw_editor_widget(intake_bake_rect(rect), "Bake + Reload", false);
        draw_editor_widget(intake_remove_rect(rect), "Remove Draft", false);

        let issues = recipe.promotion_issues(repo_root_dir());
        draw_badge(
            Rect::new(rect.x, rect.y + rect.h - 34.0, rect.w, 20.0),
            if issues.is_empty() {
                "Promotion Ready"
            } else {
                "Promotion Blocked"
            },
            issues.is_empty(),
        );
        draw_scissored_text(
            if issues.is_empty() {
                "Promotion gate ready"
            } else {
                issues[0].as_str()
            },
            rect.x,
            rect.y + rect.h - 10.0,
            rect.w,
            13.0,
            if issues.is_empty() { GOOD } else { WARN },
        );
    }

    pub(crate) fn handle_asset_intake_click(&mut self, mx: f32, my: f32, rect: Rect) -> bool {
        let mouse = vec2(mx, my);
        if !rect.contains(mouse) {
            return false;
        }
        if intake_scan_rect(rect).contains(mouse) {
            match self.asset_intake_catalog.scan_inbox() {
                Ok(added) => {
                    let _ = self.asset_intake_catalog.save_default();
                    self.asset_intake_selected = self
                        .asset_intake_catalog
                        .recipes
                        .len()
                        .saturating_sub(added.max(1));
                    self.asset_intake_source_reload_requested = true;
                    self.status_message = format!("Asset intake scan added {added} recipe(s)");
                }
                Err(error) => self.status_message = error,
            }
            return true;
        }
        if intake_save_rect(rect).contains(mouse) {
            self.status_message = self
                .asset_intake_catalog
                .save_default()
                .map(|_| "Saved asset intake recipes".to_string())
                .unwrap_or_else(|error| error);
            return true;
        }
        if self.asset_intake_catalog.recipes.is_empty() {
            return true;
        }
        if intake_source_preview_rect(rect).contains(mouse) {
            self.asset_intake_source_reload_requested = true;
            self.status_message = "Queued intake source preview reload".to_string();
            return true;
        }
        if intake_prev_rect(rect).contains(mouse) {
            self.asset_intake_selected = self.asset_intake_selected.saturating_sub(1);
            self.asset_intake_source_reload_requested = true;
            return true;
        }
        if intake_next_rect(rect).contains(mouse) {
            self.asset_intake_selected = (self.asset_intake_selected + 1)
                .min(self.asset_intake_catalog.recipes.len().saturating_sub(1));
            self.asset_intake_source_reload_requested = true;
            return true;
        }

        let index = self
            .asset_intake_selected
            .min(self.asset_intake_catalog.recipes.len().saturating_sub(1));
        if intake_target_kind_rect(rect).contains(mouse) {
            let recipe = &mut self.asset_intake_catalog.recipes[index];
            recipe.target.kind = match recipe.target.kind {
                AssetIntakeTargetKind::Tile => AssetIntakeTargetKind::Object,
                AssetIntakeTargetKind::Object => AssetIntakeTargetKind::Tile,
            };
            recipe.target.code = target_codes(recipe.target.kind)[0].to_string();
            recipe.promotion_state = AssetPromotionState::Draft;
            self.persist_intake_change("Changed intake target family");
            return true;
        }
        if intake_target_prev_rect(rect).contains(mouse)
            || intake_target_next_rect(rect).contains(mouse)
        {
            let delta = if intake_target_next_rect(rect).contains(mouse) {
                1
            } else {
                -1
            };
            let recipe = &mut self.asset_intake_catalog.recipes[index];
            let codes = target_codes(recipe.target.kind);
            let current = codes
                .iter()
                .position(|code| *code == recipe.target.code.as_str())
                .unwrap_or(0);
            recipe.target.code = codes[cycle_index(current, codes.len(), delta)].to_string();
            recipe.promotion_state = AssetPromotionState::Draft;
            self.persist_intake_change("Changed runtime binding target");
            return true;
        }
        if intake_license_prev_rect(rect).contains(mouse)
            || intake_license_next_rect(rect).contains(mouse)
        {
            let delta = if intake_license_next_rect(rect).contains(mouse) {
                1
            } else {
                -1
            };
            let recipe = &mut self.asset_intake_catalog.recipes[index];
            let current = AssetLicenseStatus::ALL
                .iter()
                .position(|status| *status == recipe.license.status)
                .unwrap_or(0);
            recipe.license.status =
                AssetLicenseStatus::ALL[cycle_index(current, AssetLicenseStatus::ALL.len(), delta)];
            recipe.license.accepted = false;
            recipe.promotion_state = AssetPromotionState::Draft;
            self.persist_intake_change("Changed asset license status; acceptance reset");
            return true;
        }
        if intake_accept_rect(rect).contains(mouse) {
            let recipe = &mut self.asset_intake_catalog.recipes[index];
            recipe.license.accepted = !recipe.license.accepted;
            recipe.promotion_state = AssetPromotionState::Draft;
            self.persist_intake_change("Updated explicit license acceptance");
            return true;
        }
        if intake_promotion_rect(rect).contains(mouse) {
            let ready = self.asset_intake_catalog.recipes[index]
                .promotion_issues(repo_root_dir())
                .is_empty();
            let recipe = &mut self.asset_intake_catalog.recipes[index];
            if ready {
                recipe.promotion_state = if recipe.promotion_state == AssetPromotionState::Approved
                {
                    AssetPromotionState::Draft
                } else {
                    AssetPromotionState::Approved
                };
                self.persist_intake_change("Updated asset promotion state");
            } else {
                self.status_message =
                    "Promotion blocked: resolve the license/source/slice warnings first"
                        .to_string();
            }
            return true;
        }
        for row in 0..4 {
            if intake_value_minus_rect(rect, row).contains(mouse) {
                self.adjust_intake_slice(index, row, -1);
                return true;
            }
            if intake_value_plus_rect(rect, row).contains(mouse) {
                self.adjust_intake_slice(index, row, 1);
                return true;
            }
        }
        for row in 0..2 {
            if intake_pivot_minus_rect(rect, row).contains(mouse) {
                self.adjust_intake_pivot(index, row, -1);
                return true;
            }
            if intake_pivot_plus_rect(rect, row).contains(mouse) {
                self.adjust_intake_pivot(index, row, 1);
                return true;
            }
        }
        for (target_index, target) in [
            FootprintEditTarget::Visual,
            FootprintEditTarget::Collision,
            FootprintEditTarget::Interaction,
        ]
        .into_iter()
        .enumerate()
        {
            if intake_footprint_target_rect(rect, target_index).contains(mouse) {
                self.asset_intake_footprint_target = target;
                return true;
            }
        }
        for row in 0..4 {
            if intake_footprint_minus_rect(rect, row).contains(mouse) {
                self.adjust_intake_footprint(index, row, -1);
                return true;
            }
            if intake_footprint_plus_rect(rect, row).contains(mouse) {
                self.adjust_intake_footprint(index, row, 1);
                return true;
            }
        }
        if intake_validate_rect(rect).contains(mouse) {
            let issues = self.asset_intake_catalog.recipes[index].promotion_issues(repo_root_dir());
            self.status_message = if issues.is_empty() {
                "Asset recipe passes the promotion gate".to_string()
            } else {
                format!("Asset recipe blocked: {}", issues.join(" | "))
            };
            return true;
        }
        if intake_bake_rect(rect).contains(mouse) {
            let save = self.asset_intake_catalog.save_default();
            self.status_message = match save.and_then(|_| run_asset_intake_bake()) {
                Ok(summary) => {
                    self.asset_hot_reload_requested = true;
                    summary
                }
                Err(error) => error,
            };
            return true;
        }
        if intake_remove_rect(rect).contains(mouse) {
            if self.asset_intake_catalog.recipes[index].promotion_state
                == AssetPromotionState::Approved
            {
                self.status_message =
                    "Approved recipes must be returned to Draft before removal".to_string();
            } else {
                let removed = self.asset_intake_catalog.recipes.remove(index);
                self.asset_intake_selected = self
                    .asset_intake_selected
                    .min(self.asset_intake_catalog.recipes.len().saturating_sub(1));
                let _ = self.asset_intake_catalog.save_default();
                self.asset_intake_source_reload_requested = true;
                self.status_message = format!("Removed intake recipe {}", removed.stable_id);
            }
            return true;
        }
        true
    }

    pub(crate) fn poll_asset_hot_reload(&mut self) {
        if get_time() < self.asset_hot_reload_next_check {
            return;
        }
        self.asset_hot_reload_next_check = get_time() + 0.75;
        let modified = user_asset_manifest_modified();
        if modified.is_some() && modified != self.asset_manifest_modified {
            self.asset_manifest_modified = modified;
            self.asset_hot_reload_requested = true;
        }
    }

    pub(crate) async fn reload_asset_outputs_if_requested(&mut self) {
        if self.asset_hot_reload_requested {
            self.asset_hot_reload_requested = false;
            match self.editor_textures.reload_user_assets().await {
                Ok(summary) => {
                    self.asset_catalog = AssetPaletteCatalog::load_default().unwrap_or_default();
                    self.asset_intake_catalog =
                        AssetIntakeCatalog::load_default().unwrap_or_default();
                    self.asset_manifest_modified = user_asset_manifest_modified();
                    self.status_message = format!("Hot reloaded promoted assets | {summary}");
                }
                Err(error) => self.status_message = format!("Asset hot reload failed: {error}"),
            }
        }
        if self.asset_intake_source_reload_requested {
            self.asset_intake_source_reload_requested = false;
            let source_path = self
                .asset_intake_catalog
                .recipes
                .get(self.asset_intake_selected)
                .map(|recipe| recipe.source_path.clone());
            if let Some(source_path) = source_path {
                if let Err(error) = self
                    .editor_textures
                    .reload_intake_source(&source_path)
                    .await
                {
                    self.status_message = error;
                }
            }
        }
    }

    fn persist_intake_change(&mut self, message: &str) {
        self.status_message = self
            .asset_intake_catalog
            .save_default()
            .map(|_| message.to_string())
            .unwrap_or_else(|error| error);
    }

    fn adjust_intake_slice(&mut self, index: usize, row: usize, delta: i32) {
        let recipe = &mut self.asset_intake_catalog.recipes[index];
        match row {
            0 => recipe.slice.x = add_u32(recipe.slice.x, delta),
            1 => recipe.slice.y = add_u32(recipe.slice.y, delta),
            2 => recipe.slice.width = add_u32(recipe.slice.width, delta).max(1),
            3 => recipe.slice.height = add_u32(recipe.slice.height, delta).max(1),
            _ => {}
        }
        recipe.promotion_state = AssetPromotionState::Draft;
        self.persist_intake_change("Updated source slice");
    }

    fn adjust_intake_pivot(&mut self, index: usize, row: usize, delta: i32) {
        let recipe = &mut self.asset_intake_catalog.recipes[index];
        if row == 0 {
            recipe.pivot.x += delta;
        } else {
            recipe.pivot.y += delta;
        }
        recipe.promotion_state = AssetPromotionState::Draft;
        self.persist_intake_change("Updated asset pivot");
    }

    fn adjust_intake_footprint(&mut self, index: usize, row: usize, delta: i32) {
        let target = self.asset_intake_footprint_target;
        let recipe = &mut self.asset_intake_catalog.recipes[index];
        let footprint = intake_footprint_mut(recipe, target);
        footprint[row] += delta;
        if row >= 2 {
            footprint[row] = footprint[row].max(1);
        }
        recipe.promotion_state = AssetPromotionState::Draft;
        self.persist_intake_change("Updated authored footprint");
    }
}

fn run_asset_intake_bake() -> Result<String, String> {
    let root = repo_root_dir();
    let script = root.join("tools/automation/assets/Bake-AssetIntakeAtlasV66.py");
    let candidates: [(&str, &[&str]); 3] = [("python", &[]), ("python3", &[]), ("py", &["-3"])];
    let mut errors = Vec::new();
    for (program, prefix) in candidates {
        let mut command = Command::new(program);
        command.current_dir(&root);
        command.args(prefix);
        command.arg(&script);
        match command.output() {
            Ok(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
            Ok(output) => errors.push(format!(
                "{program}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => errors.push(format!("{program}: {error}")),
        }
    }
    Err(format!("Asset bake failed: {}", errors.join(" | ")))
}

fn target_codes(kind: AssetIntakeTargetKind) -> Vec<&'static str> {
    match kind {
        AssetIntakeTargetKind::Tile => TileKind::ALL.iter().map(|kind| kind.code()).collect(),
        AssetIntakeTargetKind::Object => OBJECT_BRUSHES.iter().map(|kind| kind.code()).collect(),
    }
}

fn add_u32(value: u32, delta: i32) -> u32 {
    if delta >= 0 {
        value.saturating_add(delta as u32)
    } else {
        value.saturating_sub((-delta) as u32)
    }
}

fn intake_footprint(
    recipe: &haven_assets::asset_intake::AssetIntakeRecipe,
    target: FootprintEditTarget,
) -> [i32; 4] {
    match target {
        FootprintEditTarget::Visual => recipe.footprint.visual,
        FootprintEditTarget::Collision => recipe.footprint.collision,
        FootprintEditTarget::Interaction => recipe.footprint.interaction,
    }
}

fn intake_footprint_mut(
    recipe: &mut haven_assets::asset_intake::AssetIntakeRecipe,
    target: FootprintEditTarget,
) -> &mut [i32; 4] {
    match target {
        FootprintEditTarget::Visual => &mut recipe.footprint.visual,
        FootprintEditTarget::Collision => &mut recipe.footprint.collision,
        FootprintEditTarget::Interaction => &mut recipe.footprint.interaction,
    }
}

fn intake_scan_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 54.0, (rect.w - 6.0) * 0.62, 28.0)
}
fn intake_save_rect(rect: Rect) -> Rect {
    let scan = intake_scan_rect(rect);
    Rect::new(scan.x + scan.w + 6.0, scan.y, rect.w - scan.w - 6.0, 28.0)
}
fn intake_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 88.0, 34.0, 28.0)
}
fn intake_source_preview_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 64.0, rect.y + 116.0, 60.0, 54.0)
}
fn intake_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 34.0, rect.y + 88.0, 34.0, 28.0)
}
fn intake_target_kind_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 182.0, 84.0, 28.0)
}
fn intake_target_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 72.0, rect.y + 182.0, 32.0, 28.0)
}
fn intake_target_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 34.0, rect.y + 182.0, 32.0, 28.0)
}
fn intake_license_prev_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 244.0, 32.0, 28.0)
}
fn intake_license_next_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 32.0, rect.y + 244.0, 32.0, 28.0)
}
fn intake_accept_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + 278.0, (rect.w - 6.0) * 0.62, 28.0)
}
fn intake_promotion_rect(rect: Rect) -> Rect {
    let accept = intake_accept_rect(rect);
    Rect::new(
        accept.x + accept.w + 6.0,
        accept.y,
        rect.w - accept.w - 6.0,
        28.0,
    )
}
fn intake_value_y(rect: Rect, row: usize) -> f32 {
    rect.y + 340.0 + row as f32 * 34.0
}
fn intake_value_minus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(rect.x + 60.0, intake_value_y(rect, row), 28.0, 26.0)
}
fn intake_value_plus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(rect.x + 94.0, intake_value_y(rect, row), 28.0, 26.0)
}
fn intake_pivot_minus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 70.0,
        intake_value_y(rect, row),
        28.0,
        26.0,
    )
}
fn intake_pivot_plus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 36.0,
        intake_value_y(rect, row),
        28.0,
        26.0,
    )
}
fn intake_footprint_target_rect(rect: Rect, index: usize) -> Rect {
    let width = (rect.w - 10.0) / 3.0;
    Rect::new(
        rect.x + index as f32 * (width + 5.0),
        rect.y + 506.0,
        width,
        26.0,
    )
}
fn intake_footprint_y(rect: Rect, row: usize) -> f32 {
    rect.y + 538.0 + row as f32 * 31.0
}
fn intake_footprint_minus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 72.0,
        intake_footprint_y(rect, row),
        30.0,
        25.0,
    )
}
fn intake_footprint_plus_rect(rect: Rect, row: usize) -> Rect {
    Rect::new(
        rect.x + rect.w - 36.0,
        intake_footprint_y(rect, row),
        30.0,
        25.0,
    )
}
fn intake_validate_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 78.0, (rect.w - 6.0) * 0.38, 28.0)
}
fn intake_bake_rect(rect: Rect) -> Rect {
    let validate = intake_validate_rect(rect);
    Rect::new(
        validate.x + validate.w + 6.0,
        validate.y,
        rect.w - validate.w - 6.0,
        28.0,
    )
}
fn intake_remove_rect(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y + rect.h - 44.0, rect.w, 26.0)
}

fn draw_intake_value_row(rect: Rect, row: usize, label: &str, value: i32) {
    let y = intake_value_y(rect, row);
    draw_editor_text(&format!("{label}: {value}"), rect.x, y + 18.0, 14.0, TEXT);
    draw_editor_widget(intake_value_minus_rect(rect, row), "-", false);
    draw_editor_widget(intake_value_plus_rect(rect, row), "+", false);
}
fn draw_intake_pivot_row(rect: Rect, row: usize, label: &str, value: i32) {
    let y = intake_value_y(rect, row);
    draw_editor_text(
        &format!("{label}: {value}"),
        rect.x + rect.w * 0.52,
        y + 18.0,
        14.0,
        TEXT,
    );
    draw_editor_widget(intake_pivot_minus_rect(rect, row), "-", false);
    draw_editor_widget(intake_pivot_plus_rect(rect, row), "+", false);
}
fn draw_intake_footprint_row(rect: Rect, row: usize, label: &str, value: i32) {
    let y = intake_footprint_y(rect, row);
    draw_editor_text(&format!("{label}: {value}"), rect.x, y + 18.0, 14.0, TEXT);
    draw_editor_widget(intake_footprint_minus_rect(rect, row), "-", false);
    draw_editor_widget(intake_footprint_plus_rect(rect, row), "+", false);
}
