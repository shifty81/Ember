use serde_json::{json, Number, Value};
use std::{
    collections::{HashMap, HashSet},
    fs::{copy, create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
};

use super::{
    transition_atlas_groups::{
        expected_atlas_group_for_codes, is_water_transition_atlas_group,
        SUPPORTED_TRANSITION_ATLAS_GROUPS,
    },
    TerrainFamilySelector, TransitionMaterial, TERRAIN_TRANSITION_RULE_MANIFEST_PATH,
};

pub const TERRAIN_TRANSITION_RULE_DRAFT_PATH: &str =
    "WORKSPACE/generated/terrain_transition_rule_draft_v0_1.json";

pub const TERRAIN_TRANSITION_RULE_LIVE_BACKUP_PATH: &str =
    "WORKSPACE/generated/terrain_transition_rule_manifest_live_backup_v0_1.json";

pub const TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH: &str =
    "WORKSPACE/generated/terrain_transition_rule_draft_undo_v0_1.json";

const SELECTOR_CODES: [&str; 20] = [
    "grass",
    "dirt",
    "sand",
    "road",
    "wood_floor",
    "stone_floor",
    "farm",
    "shallow_water",
    "water",
    "deep_water",
    "rock_wall",
    "cave",
    "greenhouse",
    "land",
    "soft_natural",
    "constructed",
    "blocking_wall",
    "non_blocking_wall",
    "any",
    "void",
];

const MATERIAL_CODES: [&str; 9] = [
    "wet_sand",
    "foam",
    "shallow_water_edge",
    "sand_blend",
    "grass_fringe",
    "dirt_blend",
    "road_shoulder",
    "stone_shoulder",
    "rock_shadow",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionRuleDraftDiffKind {
    Added,
    Removed,
    Changed,
}

impl TransitionRuleDraftDiffKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftDiffEntry {
    pub id: String,
    pub manifest_index: Option<usize>,
    pub kind: TransitionRuleDraftDiffKind,
    pub changed_fields: Vec<String>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftCompareReport {
    pub live_path: PathBuf,
    pub draft_path: PathBuf,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub entries: Vec<TransitionRuleDraftDiffEntry>,
    pub summary: String,
}

impl TransitionRuleDraftCompareReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }

    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.removed > 0 || self.changed > 0
    }

    pub fn compact_lines(&self, limit: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in self.entries.iter().take(limit) {
            lines.push(entry.summary.clone());
        }
        if self.entries.len() > limit {
            lines.push(format!(
                "… {} more transition-rule draft difference(s)",
                self.entries.len() - limit
            ));
        }
        lines
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftPromotionReport {
    pub live_path: PathBuf,
    pub backup_path: PathBuf,
    pub promoted_rule_count: usize,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub summary: String,
}

impl TransitionRuleDraftPromotionReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftRestoreReport {
    pub path: PathBuf,
    pub undo_path: PathBuf,
    pub rule_count: usize,
    pub restored_from: String,
    pub summary: String,
}

impl TransitionRuleDraftRestoreReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionRuleDraftDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl TransitionRuleDraftDiagnosticSeverity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn sort_rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftDiagnostic {
    pub manifest_index: Option<usize>,
    pub rule_id: Option<String>,
    pub severity: TransitionRuleDraftDiagnosticSeverity,
    pub field: Option<String>,
    pub message: String,
}

impl TransitionRuleDraftDiagnostic {
    pub fn compact_label(&self) -> String {
        let scope = match (&self.rule_id, self.manifest_index) {
            (Some(id), Some(index)) => format!("{id}#{index}"),
            (Some(id), None) => id.clone(),
            (None, Some(index)) => format!("rule#{index}"),
            (None, None) => "manifest".to_string(),
        };
        let field = self
            .field
            .as_ref()
            .map(|field| format!(" {field}:"))
            .unwrap_or_default();
        format!("{} {scope}{field} {}", self.severity.label(), self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftValidationReport {
    pub path: PathBuf,
    pub rule_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub diagnostics: Vec<TransitionRuleDraftDiagnostic>,
    pub summary: String,
}

impl TransitionRuleDraftValidationReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }

    pub fn can_promote(&self) -> bool {
        self.errors == 0
    }

    pub fn compact_lines(&self, limit: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for diagnostic in self.diagnostics.iter().take(limit) {
            lines.push(diagnostic.compact_label());
        }
        if self.diagnostics.len() > limit {
            lines.push(format!(
                "… {} more transition-rule diagnostic(s)",
                self.diagnostics.len() - limit
            ));
        }
        if lines.is_empty() {
            lines.push("No transition-rule draft diagnostics".to_string());
        }
        lines
    }

    pub fn selected_lines(&self, manifest_index: usize, limit: usize) -> Vec<String> {
        let mut selected: Vec<String> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.manifest_index.is_none()
                    || diagnostic.manifest_index == Some(manifest_index)
            })
            .take(limit)
            .map(TransitionRuleDraftDiagnostic::compact_label)
            .collect();
        if selected.is_empty() {
            selected.push(format!(
                "No draft diagnostics for manifest rule #{manifest_index}"
            ));
        }
        selected
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftAutoFixEntry {
    pub manifest_index: Option<usize>,
    pub rule_id: Option<String>,
    pub field: String,
    pub before: String,
    pub after: String,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftAutoFixReport {
    pub path: PathBuf,
    pub rule_count: usize,
    pub changed_rule_count: usize,
    pub applied_fix_count: usize,
    pub entries: Vec<TransitionRuleDraftAutoFixEntry>,
    pub validation: TransitionRuleDraftValidationReport,
    pub summary: String,
}

impl TransitionRuleDraftAutoFixReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }

    pub fn compact_lines(&self, limit: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for entry in self.entries.iter().take(limit) {
            lines.push(entry.summary.clone());
        }
        if self.entries.len() > limit {
            lines.push(format!(
                "… {} more transition-rule draft auto-fix(es)",
                self.entries.len() - limit
            ));
        }
        if lines.is_empty() {
            lines.push("No safe transition-rule draft auto-fixes were needed".to_string());
        }
        lines
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionRuleDraftEdit {
    PriorityDelta(i32),
    CycleMaterial(i32),
    CycleCenter(i32),
    CycleNeighbor(i32),
    SyncAtlasGroup,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleDraftReport {
    pub path: PathBuf,
    pub rule_count: usize,
    pub selected_rule_id: Option<String>,
    pub summary: String,
}

impl TransitionRuleDraftReport {
    pub fn status_line(&self) -> String {
        self.summary.clone()
    }
}

pub fn transition_rule_draft_status() -> String {
    let path = transition_rule_draft_path();
    if !path.exists() {
        return format!(
            "No transition-rule draft exported yet ({})",
            TERRAIN_TRANSITION_RULE_DRAFT_PATH
        );
    }
    match read_manifest_value(&path).and_then(|value| validate_transition_rule_draft_value(&value))
    {
        Ok(rule_count) => format!(
            "Draft ready: {rule_count} rules at {}",
            TERRAIN_TRANSITION_RULE_DRAFT_PATH
        ),
        Err(error) => format!("Draft invalid: {error}"),
    }
}

pub fn transition_rule_draft_undo_status() -> String {
    let path = transition_rule_draft_undo_path();
    if !path.exists() {
        return "Draft undo: no restore snapshot yet".to_string();
    }
    match read_manifest_value(&path).and_then(|value| validate_transition_rule_draft_value(&value))
    {
        Ok(rule_count) => format!(
            "Draft undo ready: {rule_count} rule snapshot at {}",
            TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH
        ),
        Err(error) => format!("Draft undo invalid: {error}"),
    }
}

pub fn transition_rule_draft_undo_lines(limit: usize) -> Vec<String> {
    let path = transition_rule_draft_undo_path();
    if !path.exists() {
        return vec!["No transition-rule draft undo snapshot exists yet".to_string()];
    }
    let Ok(value) = read_manifest_value(&path) else {
        return vec!["Transition-rule draft undo snapshot could not be read".to_string()];
    };
    let mut lines = Vec::new();
    if let Some(draft) = value.get("draft").and_then(Value::as_object) {
        if let Some(action) = draft.get("action").and_then(Value::as_str) {
            lines.push(format!("undo snapshot action: {action}"));
        }
        if let Some(source) = draft.get("sourceManifest").and_then(Value::as_str) {
            lines.push(format!("undo source: {source}"));
        }
    }
    if let Some(rules) = value.get("rules").and_then(Value::as_array) {
        lines.push(format!("undo snapshot rules: {}", rules.len()));
    }
    if lines.is_empty() {
        lines.push("Transition-rule draft undo snapshot exists".to_string());
    }
    lines.truncate(limit);
    lines
}

pub fn transition_rule_draft_selected_summary(manifest_index: usize) -> String {
    let path = transition_rule_draft_path();
    if !path.exists() {
        return "Draft selection: no draft file yet".to_string();
    }
    let Ok(value) = read_manifest_value(&path) else {
        return "Draft selection: failed to read draft file".to_string();
    };
    let Some(rule) = rule_at_index(&value, manifest_index) else {
        return format!("Draft selection: manifest index {manifest_index} is outside draft rules");
    };
    let id = string_field(rule, "id").unwrap_or("<unnamed>");
    let center = string_field(rule, "center").unwrap_or("?");
    let neighbor = string_field(rule, "neighbor").unwrap_or("?");
    let material = string_field(rule, "material").unwrap_or("?");
    let atlas_group = string_field(rule, "atlasGroup").unwrap_or("?");
    let priority = rule
        .get("priority")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    format!(
        "Draft[{manifest_index}] {id}: {center}->{neighbor} {material}@{atlas_group} p{priority}"
    )
}

pub fn validate_transition_rule_draft_for_editor(
) -> Result<TransitionRuleDraftValidationReport, String> {
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        return Err(format!(
            "no draft file exists yet at {}",
            TERRAIN_TRANSITION_RULE_DRAFT_PATH
        ));
    }
    let value = read_manifest_value(&draft_path)?;
    let mut diagnostics = collect_transition_rule_draft_diagnostics(&value)?;
    if let Err(error) = validate_transition_rule_draft_value(&value) {
        diagnostics.push(global_diagnostic(
            TransitionRuleDraftDiagnosticSeverity::Error,
            "schema",
            &format!("strict draft validation failed: {error}"),
        ));
    }
    diagnostics.sort_by(|left, right| {
        left.severity
            .sort_rank()
            .cmp(&right.severity.sort_rank())
            .then(left.manifest_index.cmp(&right.manifest_index))
            .then(left.rule_id.cmp(&right.rule_id))
            .then(left.field.cmp(&right.field))
            .then(left.message.cmp(&right.message))
    });
    let rule_count = value
        .get("rules")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == TransitionRuleDraftDiagnosticSeverity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == TransitionRuleDraftDiagnosticSeverity::Warning)
        .count();
    let infos = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == TransitionRuleDraftDiagnosticSeverity::Info)
        .count();
    let summary = if errors == 0 && warnings == 0 {
        format!("Draft validation passed: {rule_count} rule(s), {infos} info note(s)")
    } else {
        format!(
            "Draft validation: {errors} error(s), {warnings} warning(s), {infos} info note(s) across {rule_count} rule(s)"
        )
    };
    Ok(TransitionRuleDraftValidationReport {
        path: draft_path,
        rule_count,
        errors,
        warnings,
        infos,
        diagnostics,
        summary,
    })
}

pub fn transition_rule_draft_validation_status() -> String {
    match validate_transition_rule_draft_for_editor() {
        Ok(report) => report.status_line(),
        Err(error) => format!("Draft validation unavailable: {error}"),
    }
}

pub fn transition_rule_draft_validation_lines(limit: usize) -> Vec<String> {
    match validate_transition_rule_draft_for_editor() {
        Ok(report) => report.compact_lines(limit),
        Err(error) => vec![format!("Draft validation unavailable: {error}")],
    }
}

pub fn transition_rule_draft_selected_validation_lines(
    manifest_index: usize,
    limit: usize,
) -> Vec<String> {
    match validate_transition_rule_draft_for_editor() {
        Ok(report) => report.selected_lines(manifest_index, limit),
        Err(error) => vec![format!("Draft validation unavailable: {error}")],
    }
}

pub fn export_transition_rule_draft_from_manifest() -> Result<TransitionRuleDraftReport, String> {
    let draft_path = transition_rule_draft_path();
    let had_snapshot =
        snapshot_existing_draft_for_undo("undo_snapshot_before_draft_export")?.is_some();
    let source_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_MANIFEST_PATH);
    let mut value = read_manifest_value(&source_path)?;
    let rule_count = validate_transition_rule_draft_value(&value)?;
    annotate_draft(&mut value, "exported_from_live_manifest");
    write_manifest_value(&draft_path, &value)?;
    let snapshot_note = if had_snapshot {
        " Previous draft was saved to undo."
    } else {
        ""
    };
    Ok(report(
        draft_path,
        rule_count,
        None,
        format!("Exported safe transition-rule draft with {rule_count} rules.{snapshot_note}"),
    ))
}

pub fn reset_transition_rule_draft_from_live_manifest(
) -> Result<TransitionRuleDraftRestoreReport, String> {
    let draft_path = transition_rule_draft_path();
    let undo_path = transition_rule_draft_undo_path();
    let had_snapshot =
        snapshot_existing_draft_for_undo("undo_snapshot_before_live_restore")?.is_some();
    let source_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_MANIFEST_PATH);
    let mut value = read_manifest_value(&source_path)?;
    let rule_count = validate_transition_rule_draft_value(&value)?;
    annotate_draft(&mut value, "restored_from_live_manifest");
    write_manifest_value(&draft_path, &value)?;
    let snapshot_note = if had_snapshot {
        " Previous draft was saved to undo."
    } else {
        ""
    };
    Ok(TransitionRuleDraftRestoreReport {
        path: draft_path,
        undo_path,
        rule_count,
        restored_from: TERRAIN_TRANSITION_RULE_MANIFEST_PATH.to_string(),
        summary: format!("Restored transition-rule draft from live manifest with {rule_count} rules.{snapshot_note}"),
    })
}

pub fn restore_transition_rule_draft_from_undo() -> Result<TransitionRuleDraftRestoreReport, String>
{
    let draft_path = transition_rule_draft_path();
    let undo_path = transition_rule_draft_undo_path();
    if !undo_path.exists() {
        return Err(format!(
            "no transition-rule draft undo snapshot exists yet at {}",
            TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH
        ));
    }
    let mut value = read_manifest_value(&undo_path)?;
    let rule_count = validate_transition_rule_draft_value(&value)?;
    annotate_draft(&mut value, "restored_from_undo_snapshot");
    write_manifest_value(&draft_path, &value)?;
    Ok(TransitionRuleDraftRestoreReport {
        path: draft_path,
        undo_path,
        rule_count,
        restored_from: TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH.to_string(),
        summary: format!(
            "Restored transition-rule draft from undo snapshot with {rule_count} rules"
        ),
    })
}

pub fn apply_transition_rule_draft_edit(
    manifest_index: usize,
    edit: TransitionRuleDraftEdit,
) -> Result<TransitionRuleDraftReport, String> {
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        export_transition_rule_draft_from_manifest()?;
    }
    let mut value = read_manifest_value(&draft_path)?;
    let rule_count = validate_transition_rule_draft_value(&value)?;
    snapshot_existing_draft_for_undo("undo_snapshot_before_draft_edit")?;
    let selected_rule_id = {
        let rule = rule_at_index_mut(&mut value, manifest_index)
            .ok_or_else(|| format!("manifest index {manifest_index} is outside draft rules"))?;
        apply_edit_to_rule(rule, edit)?;
        string_field(rule, "id").map(str::to_string)
    };
    validate_transition_rule_draft_value(&value)?;
    annotate_draft(&mut value, "edited_by_transition_rule_editor");
    write_manifest_value(&draft_path, &value)?;
    let rule_id = selected_rule_id
        .clone()
        .unwrap_or_else(|| format!("#{manifest_index}"));
    Ok(report(
        draft_path,
        rule_count,
        selected_rule_id,
        format!("Updated safe transition-rule draft for {rule_id}"),
    ))
}

pub fn apply_transition_rule_draft_auto_fixes() -> Result<TransitionRuleDraftAutoFixReport, String>
{
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        export_transition_rule_draft_from_manifest()?;
    }
    let mut value = read_manifest_value(&draft_path)?;
    let mut entries = Vec::new();
    let mut changed_rule_indexes = HashSet::new();
    let rule_count = value
        .get("rules")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| "transition-rule draft has no rules array".to_string())?;
    snapshot_existing_draft_for_undo("undo_snapshot_before_draft_autofix")?;

    if let Some(rules) = value.get_mut("rules").and_then(Value::as_array_mut) {
        for (index, rule_value) in rules.iter_mut().enumerate() {
            let Some(rule) = rule_value.as_object_mut() else {
                continue;
            };
            let rule_id = string_field(rule, "id").map(str::to_string);
            let before_len = entries.len();
            auto_fix_rule_atlas_group(index, rule_id.as_deref(), rule, &mut entries);
            auto_fix_rule_phase_list(index, rule_id.as_deref(), rule, &mut entries);
            if entries.len() != before_len {
                changed_rule_indexes.insert(index);
            }
        }
    }

    annotate_draft(&mut value, "auto_fixed_by_transition_rule_editor");
    write_manifest_value(&draft_path, &value)?;
    let validation = validate_transition_rule_draft_for_editor()?;
    let applied_fix_count = entries.len();
    let changed_rule_count = changed_rule_indexes.len();
    let summary = if applied_fix_count == 0 {
        format!(
            "Transition-rule draft auto-fix checked {rule_count} rule(s): no safe fixes needed; {}",
            validation.status_line()
        )
    } else {
        format!(
            "Applied {applied_fix_count} safe transition-rule draft auto-fix(es) across {changed_rule_count} rule(s); {}",
            validation.status_line()
        )
    };
    Ok(TransitionRuleDraftAutoFixReport {
        path: draft_path,
        rule_count,
        changed_rule_count,
        applied_fix_count,
        entries,
        validation,
        summary,
    })
}

pub fn transition_rule_draft_auto_fix_status() -> String {
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        return "Draft auto-fix: no draft exported yet".to_string();
    }
    match read_manifest_value(&draft_path) {
        Ok(value) => {
            let mut available = 0usize;
            if let Some(rules) = value.get("rules").and_then(Value::as_array) {
                for rule_value in rules {
                    let Some(rule) = rule_value.as_object() else {
                        continue;
                    };
                    available += safe_fix_count_for_rule(rule);
                }
            }
            if available == 0 {
                "Draft auto-fix: no safe fixes currently detected".to_string()
            } else {
                format!("Draft auto-fix: {available} safe fix candidate(s) available")
            }
        }
        Err(error) => format!("Draft auto-fix unavailable: {error}"),
    }
}

pub fn transition_rule_draft_auto_fix_lines(limit: usize) -> Vec<String> {
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        return vec!["Draft auto-fix unavailable: no draft exported yet".to_string()];
    }
    let Ok(value) = read_manifest_value(&draft_path) else {
        return vec!["Draft auto-fix unavailable: failed to read draft".to_string()];
    };
    let mut lines = Vec::new();
    if let Some(rules) = value.get("rules").and_then(Value::as_array) {
        for (index, rule_value) in rules.iter().enumerate() {
            let Some(rule) = rule_value.as_object() else {
                continue;
            };
            let id = string_field(rule, "id").unwrap_or("<unnamed>");
            if let Some(line) = atlas_group_fix_preview(index, id, rule) {
                lines.push(line);
            }
            if let Some(line) = phase_list_fix_preview(index, id, rule) {
                lines.push(line);
            }
            if lines.len() >= limit {
                break;
            }
        }
    }
    if lines.is_empty() {
        lines.push("No safe transition-rule draft auto-fixes currently available".to_string());
    }
    lines
}

pub fn transition_rule_draft_compare_status() -> String {
    match compare_transition_rule_draft_to_live() {
        Ok(report) => report.status_line(),
        Err(error) => format!("Draft compare unavailable: {error}"),
    }
}

pub fn transition_rule_draft_compare_lines(limit: usize) -> Vec<String> {
    match compare_transition_rule_draft_to_live() {
        Ok(report) => report.compact_lines(limit),
        Err(error) => vec![format!("Draft compare unavailable: {error}")],
    }
}

pub fn compare_transition_rule_draft_to_live() -> Result<TransitionRuleDraftCompareReport, String> {
    let live_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_MANIFEST_PATH);
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        return Err(format!(
            "no draft file exists yet at {}",
            TERRAIN_TRANSITION_RULE_DRAFT_PATH
        ));
    }
    let live = read_manifest_value(&live_path)?;
    let draft = read_manifest_value(&draft_path)?;
    validate_transition_rule_draft_value(&live)?;
    validate_transition_rule_draft_value(&draft)?;

    let live_rules = indexed_rules_by_id(&live)?;
    let draft_rules = indexed_rules_by_id(&draft)?;
    let mut entries = Vec::new();
    let mut unchanged = 0usize;

    for (id, live_rule) in &live_rules {
        match draft_rules.get(id) {
            Some(draft_rule) => {
                let changed_fields = changed_rule_fields(live_rule.rule, draft_rule.rule);
                if changed_fields.is_empty() {
                    unchanged += 1;
                } else {
                    entries.push(TransitionRuleDraftDiffEntry {
                        id: id.clone(),
                        manifest_index: Some(draft_rule.index),
                        kind: TransitionRuleDraftDiffKind::Changed,
                        summary: format!("changed {id}: {}", changed_fields.join(", ")),
                        changed_fields,
                    });
                }
            }
            None => entries.push(TransitionRuleDraftDiffEntry {
                id: id.clone(),
                manifest_index: Some(live_rule.index),
                kind: TransitionRuleDraftDiffKind::Removed,
                changed_fields: Vec::new(),
                summary: format!("removed {id}"),
            }),
        }
    }

    for (id, draft_rule) in &draft_rules {
        if !live_rules.contains_key(id) {
            entries.push(TransitionRuleDraftDiffEntry {
                id: id.clone(),
                manifest_index: Some(draft_rule.index),
                kind: TransitionRuleDraftDiffKind::Added,
                changed_fields: Vec::new(),
                summary: format!("added {id}"),
            });
        }
    }

    entries.sort_by_key(|entry| {
        (
            diff_sort_key(entry.kind),
            entry.manifest_index.unwrap_or(usize::MAX),
            entry.id.clone(),
        )
    });
    let added = entries
        .iter()
        .filter(|entry| entry.kind == TransitionRuleDraftDiffKind::Added)
        .count();
    let removed = entries
        .iter()
        .filter(|entry| entry.kind == TransitionRuleDraftDiffKind::Removed)
        .count();
    let changed = entries
        .iter()
        .filter(|entry| entry.kind == TransitionRuleDraftDiffKind::Changed)
        .count();
    let summary = if added == 0 && removed == 0 && changed == 0 {
        format!("Draft compare: no live/draft differences ({unchanged} unchanged rules)")
    } else {
        format!(
            "Draft compare: {changed} changed, {added} added, {removed} removed, {unchanged} unchanged"
        )
    };

    Ok(TransitionRuleDraftCompareReport {
        live_path,
        draft_path,
        added,
        removed,
        changed,
        unchanged,
        entries,
        summary,
    })
}

pub fn promote_transition_rule_draft_to_manifest(
) -> Result<TransitionRuleDraftPromotionReport, String> {
    let compare = compare_transition_rule_draft_to_live()?;
    let validation = validate_transition_rule_draft_for_editor()?;
    if !validation.can_promote() {
        return Err(format!(
            "promotion blocked by transition-rule draft diagnostics: {}",
            validation.status_line()
        ));
    }
    let draft_path = transition_rule_draft_path();
    let live_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_MANIFEST_PATH);
    let backup_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_LIVE_BACKUP_PATH);

    let mut draft = read_manifest_value(&draft_path)?;
    let promoted_rule_count = validate_transition_rule_draft_value(&draft)?;
    strip_draft_metadata_for_live_manifest(&mut draft);
    validate_transition_rule_draft_value(&draft)?;

    if let Some(parent) = backup_path.parent() {
        create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    copy(&live_path, &backup_path).map_err(|error| {
        format!(
            "failed to back up live transition manifest from {} to {}: {error}",
            live_path.display(),
            backup_path.display()
        )
    })?;
    write_manifest_value(&live_path, &draft)?;

    Ok(TransitionRuleDraftPromotionReport {
        live_path,
        backup_path,
        promoted_rule_count,
        added: compare.added,
        removed: compare.removed,
        changed: compare.changed,
        summary: format!(
            "Promoted transition-rule draft: {} rule(s), {} changed, {} added, {} removed. Restart/reload to refresh runtime manifest cache.",
            promoted_rule_count, compare.changed, compare.added, compare.removed
        ),
    })
}

#[derive(Clone, Copy)]
struct IndexedRule<'a> {
    index: usize,
    rule: &'a serde_json::Map<String, Value>,
}

fn indexed_rules_by_id(value: &Value) -> Result<HashMap<String, IndexedRule<'_>>, String> {
    let rules = value
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "transition-rule manifest has no rules array".to_string())?;
    let mut by_id = HashMap::new();
    for (index, rule) in rules.iter().enumerate() {
        let object = rule
            .as_object()
            .ok_or_else(|| format!("transition rule {index} is not an object"))?;
        let id = string_field(object, "id")
            .ok_or_else(|| format!("transition rule {index} missing id"))?
            .to_string();
        if by_id
            .insert(
                id.clone(),
                IndexedRule {
                    index,
                    rule: object,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate transition rule id {id}"));
        }
    }
    Ok(by_id)
}

fn changed_rule_fields(
    live: &serde_json::Map<String, Value>,
    draft: &serde_json::Map<String, Value>,
) -> Vec<String> {
    let fields = [
        "center",
        "neighbor",
        "material",
        "atlasGroup",
        "appliesTo",
        "priority",
        "reason",
    ];
    fields
        .iter()
        .filter(|field| live.get(**field) != draft.get(**field))
        .map(|field| field.to_string())
        .collect()
}

fn diff_sort_key(kind: TransitionRuleDraftDiffKind) -> u8 {
    match kind {
        TransitionRuleDraftDiffKind::Changed => 0,
        TransitionRuleDraftDiffKind::Added => 1,
        TransitionRuleDraftDiffKind::Removed => 2,
    }
}

fn strip_draft_metadata_for_live_manifest(value: &mut Value) {
    if let Value::Object(object) = value {
        object.remove("draft");
        object.insert(
            "lastDraftPromotion".to_string(),
            json!({
                "sourceDraft": TERRAIN_TRANSITION_RULE_DRAFT_PATH,
                "backupPath": TERRAIN_TRANSITION_RULE_LIVE_BACKUP_PATH,
                "note": "Promoted deliberately through the in-game transition-rule editor draft workflow."
            }),
        );
    }
}

type TransitionDraftMatchKey = (String, String, String, i64);
type TransitionDraftMatchUse = (usize, String, String);

fn collect_transition_rule_draft_diagnostics(
    value: &Value,
) -> Result<Vec<TransitionRuleDraftDiagnostic>, String> {
    let rules = value
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "transition-rule draft has no rules array".to_string())?;
    let mut diagnostics = Vec::new();
    let mut ids = HashSet::new();
    let mut match_keys: HashMap<TransitionDraftMatchKey, Vec<TransitionDraftMatchUse>> =
        HashMap::new();

    if value.get("draft").is_none() {
        diagnostics.push(global_diagnostic(
            TransitionRuleDraftDiagnosticSeverity::Info,
            "draft",
            "live manifest shape is valid, but draft metadata is not present",
        ));
    }

    for (index, rule) in rules.iter().enumerate() {
        let Some(object) = rule.as_object() else {
            diagnostics.push(rule_diagnostic(
                Some(index),
                None,
                TransitionRuleDraftDiagnosticSeverity::Error,
                "rule",
                "rule entry is not an object",
            ));
            continue;
        };

        let id = string_field(object, "id")
            .map(str::to_string)
            .unwrap_or_else(|| format!("<unnamed-{index}>"));
        if id.starts_with("<unnamed") {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.clone()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "id",
                "rule id is missing",
            ));
        } else if !ids.insert(id.clone()) {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.clone()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "id",
                "duplicate rule id; promotion would make rule lookup ambiguous",
            ));
        }

        let center = parse_selector_for_diagnostics(object, index, &id, "center", &mut diagnostics);
        let neighbor =
            parse_selector_for_diagnostics(object, index, &id, "neighbor", &mut diagnostics);
        let material = parse_material_for_diagnostics(object, index, &id, &mut diagnostics);
        let atlas_group = string_field(object, "atlasGroup").map(str::to_string);
        validate_atlas_group_for_diagnostics(
            index,
            &id,
            atlas_group.as_deref(),
            string_field(object, "center"),
            string_field(object, "neighbor"),
            material,
            &mut diagnostics,
        );
        let phases = parse_phases_for_diagnostics(object, index, &id, &mut diagnostics);
        let priority = object.get("priority").and_then(Value::as_i64);
        if priority.is_none() {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.clone()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "priority",
                "priority must be an integer",
            ));
        }

        if let (Some(center), Some(neighbor), Some(material)) = (center, neighbor, material) {
            validate_selector_material_combination(
                index,
                &id,
                center,
                neighbor,
                material,
                atlas_group.as_deref(),
                &mut diagnostics,
            );
            if center.code() == neighbor.code() && center != TerrainFamilySelector::Any {
                diagnostics.push(rule_diagnostic(
                    Some(index),
                    Some(id.clone()),
                    TransitionRuleDraftDiagnosticSeverity::Warning,
                    "center/neighbor",
                    "center and neighbor selectors are identical; this rule may mask more specific edge cases",
                ));
            }
        }

        if let (Some(priority), Some(atlas_group)) = (priority, atlas_group.as_deref()) {
            for phase in phases {
                let center_code = string_field(object, "center").unwrap_or("?").to_string();
                let neighbor_code = string_field(object, "neighbor").unwrap_or("?").to_string();
                let material_code = string_field(object, "material").unwrap_or("?").to_string();
                let key = (phase, center_code, neighbor_code, priority);
                match_keys.entry(key).or_default().push((
                    index,
                    id.clone(),
                    format!("{material_code}@{atlas_group}"),
                ));
            }
        }
    }

    for ((_phase, _center, _neighbor, _priority), entries) in match_keys {
        if entries.len() <= 1 {
            continue;
        }
        let mut materials = HashSet::new();
        for (_, _, material) in &entries {
            materials.insert(material.clone());
        }
        let severity = if materials.len() > 1 {
            TransitionRuleDraftDiagnosticSeverity::Error
        } else {
            TransitionRuleDraftDiagnosticSeverity::Warning
        };
        let message = if materials.len() > 1 {
            "same selector/phase/priority has multiple material outcomes; winner selection would be ambiguous"
        } else {
            "same selector/phase/priority is duplicated; remove or merge duplicate draft rules"
        };
        for (index, id, _) in entries {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id),
                severity,
                "priority",
                message,
            ));
        }
    }

    Ok(diagnostics)
}

fn parse_selector_for_diagnostics(
    object: &serde_json::Map<String, Value>,
    index: usize,
    id: &str,
    field: &str,
    diagnostics: &mut Vec<TransitionRuleDraftDiagnostic>,
) -> Option<TerrainFamilySelector> {
    let Some(code) = string_field(object, field) else {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            field,
            "selector is missing",
        ));
        return None;
    };
    match TerrainFamilySelector::from_code(code) {
        Some(selector) => Some(selector),
        None => {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                field,
                &format!("unknown selector {code}"),
            ));
            None
        }
    }
}

fn parse_material_for_diagnostics(
    object: &serde_json::Map<String, Value>,
    index: usize,
    id: &str,
    diagnostics: &mut Vec<TransitionRuleDraftDiagnostic>,
) -> Option<TransitionMaterial> {
    let Some(code) = string_field(object, "material") else {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "material",
            "material is missing",
        ));
        return None;
    };
    match TransitionMaterial::from_code(code) {
        Some(material) => Some(material),
        None => {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "material",
                &format!("unknown transition material {code}"),
            ));
            None
        }
    }
}

fn parse_phases_for_diagnostics(
    object: &serde_json::Map<String, Value>,
    index: usize,
    id: &str,
    diagnostics: &mut Vec<TransitionRuleDraftDiagnostic>,
) -> Vec<String> {
    let Some(values) = object.get("appliesTo").and_then(Value::as_array) else {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "appliesTo",
            "appliesTo array is missing",
        ));
        return Vec::new();
    };
    if values.is_empty() {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "appliesTo",
            "appliesTo array is empty",
        ));
        return Vec::new();
    }
    let mut phases = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let Some(phase) = value.as_str() else {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "appliesTo",
                "phase entry is not a string",
            ));
            continue;
        };
        if phase != "edge" && phase != "corner" {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Error,
                "appliesTo",
                &format!("unknown transition phase {phase}"),
            ));
            continue;
        }
        if !seen.insert(phase.to_string()) {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Warning,
                "appliesTo",
                &format!("duplicate transition phase {phase}"),
            ));
        }
        phases.push(phase.to_string());
    }
    phases
}

fn validate_atlas_group_for_diagnostics(
    index: usize,
    id: &str,
    atlas_group: Option<&str>,
    center_code: Option<&str>,
    neighbor_code: Option<&str>,
    material: Option<TransitionMaterial>,
    diagnostics: &mut Vec<TransitionRuleDraftDiagnostic>,
) {
    let Some(atlas_group) = atlas_group else {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "atlasGroup",
            "atlas group is missing",
        ));
        return;
    };
    if atlas_group.trim().is_empty() {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "atlasGroup",
            "atlas group is empty",
        ));
        return;
    }
    if !SUPPORTED_TRANSITION_ATLAS_GROUPS.contains(&atlas_group) {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Error,
            "atlasGroup",
            &format!("atlas group {atlas_group} is not supported by the current terrain transition atlas manifest"),
        ));
    }
    if let Some(material) = material {
        let expected = expected_atlas_group_for_codes(center_code, neighbor_code, material);
        if atlas_group != expected {
            diagnostics.push(rule_diagnostic(
                Some(index),
                Some(id.to_string()),
                TransitionRuleDraftDiagnosticSeverity::Warning,
                "atlasGroup",
                &format!(
                    "material {} normally uses atlas group {expected}",
                    material.code()
                ),
            ));
        }
    }
}

fn validate_selector_material_combination(
    index: usize,
    id: &str,
    center: TerrainFamilySelector,
    neighbor: TerrainFamilySelector,
    material: TransitionMaterial,
    atlas_group: Option<&str>,
    diagnostics: &mut Vec<TransitionRuleDraftDiagnostic>,
) {
    let touches_water =
        selector_can_resolve_to_water(center) || selector_can_resolve_to_water(neighbor);
    let uses_water_material = matches!(
        material,
        TransitionMaterial::WetSand
            | TransitionMaterial::Foam
            | TransitionMaterial::ShallowWaterEdge
            | TransitionMaterial::SandBlend
    );
    if uses_water_material && !touches_water {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Warning,
            "material",
            "shoreline/water material is assigned to a rule that does not explicitly touch water",
        ));
    }
    if touches_water && !uses_water_material && !matches!(material, TransitionMaterial::RockShadow)
    {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Warning,
            "material",
            "water-touching rule uses a non-shoreline material; verify this is intentional",
        ));
    }

    let constructed_or_wall = selector_can_resolve_to_constructed_or_wall(center)
        || selector_can_resolve_to_constructed_or_wall(neighbor);
    if matches!(
        material,
        TransitionMaterial::RoadShoulder
            | TransitionMaterial::StoneShoulder
            | TransitionMaterial::RockShadow
    ) && !constructed_or_wall
    {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Warning,
            "material",
            "constructed/cliff material is assigned to a rule without a constructed or wall selector",
        ));
    }

    if matches!(center, TerrainFamilySelector::Family(family) if family == super::TerrainFamily::Void)
        || matches!(neighbor, TerrainFamilySelector::Family(family) if family == super::TerrainFamily::Void)
    {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Info,
            "selector",
            "void selector only affects off-map/border samples; validate edge behavior in scene borders",
        ));
    }

    if is_water_transition_atlas_group(atlas_group) && !uses_water_material {
        diagnostics.push(rule_diagnostic(
            Some(index),
            Some(id.to_string()),
            TransitionRuleDraftDiagnosticSeverity::Warning,
            "atlasGroup",
            "water-bank and depth-rim atlas groups should be paired with a water transition material",
        ));
    }
}

fn selector_can_resolve_to_water(selector: TerrainFamilySelector) -> bool {
    match selector {
        TerrainFamilySelector::Family(family) => family.is_water(),
        TerrainFamilySelector::Any => true,
        _ => false,
    }
}

fn selector_can_resolve_to_constructed_or_wall(selector: TerrainFamilySelector) -> bool {
    match selector {
        TerrainFamilySelector::Family(family) => {
            family.is_constructed() || family.is_blocking_wall()
        }
        TerrainFamilySelector::Constructed
        | TerrainFamilySelector::BlockingWall
        | TerrainFamilySelector::NonBlockingWall => true,
        TerrainFamilySelector::Any => true,
        _ => false,
    }
}

fn rule_diagnostic(
    manifest_index: Option<usize>,
    rule_id: Option<String>,
    severity: TransitionRuleDraftDiagnosticSeverity,
    field: &str,
    message: &str,
) -> TransitionRuleDraftDiagnostic {
    TransitionRuleDraftDiagnostic {
        manifest_index,
        rule_id,
        severity,
        field: Some(field.to_string()),
        message: message.to_string(),
    }
}

fn global_diagnostic(
    severity: TransitionRuleDraftDiagnosticSeverity,
    field: &str,
    message: &str,
) -> TransitionRuleDraftDiagnostic {
    TransitionRuleDraftDiagnostic {
        manifest_index: None,
        rule_id: None,
        severity,
        field: Some(field.to_string()),
        message: message.to_string(),
    }
}

fn auto_fix_rule_atlas_group(
    index: usize,
    rule_id: Option<&str>,
    rule: &mut serde_json::Map<String, Value>,
    entries: &mut Vec<TransitionRuleDraftAutoFixEntry>,
) {
    let Some(material_code) = string_field(rule, "material") else {
        return;
    };
    let Some(material) = TransitionMaterial::from_code(material_code) else {
        return;
    };
    let expected = expected_atlas_group_for_rule(rule, material);
    let current = string_field(rule, "atlasGroup").unwrap_or("").to_string();
    if current == expected {
        return;
    }
    rule.insert(
        "atlasGroup".to_string(),
        Value::String(expected.to_string()),
    );
    push_auto_fix_entry(
        entries,
        Some(index),
        rule_id,
        "atlasGroup",
        &current,
        expected,
        &format!(
            "fixed atlasGroup for {} from {} to {}",
            rule_id.unwrap_or("<unnamed>"),
            display_empty(&current),
            expected
        ),
    );
}

fn auto_fix_rule_phase_list(
    index: usize,
    rule_id: Option<&str>,
    rule: &mut serde_json::Map<String, Value>,
    entries: &mut Vec<TransitionRuleDraftAutoFixEntry>,
) {
    let Some(values) = rule.get_mut("appliesTo").and_then(Value::as_array_mut) else {
        return;
    };
    let before = phase_list_label(values);
    let mut seen = HashSet::new();
    let mut cleaned = Vec::new();
    for value in values.iter() {
        let Some(phase) = value.as_str() else {
            continue;
        };
        if phase != "edge" && phase != "corner" {
            continue;
        }
        if seen.insert(phase.to_string()) {
            cleaned.push(Value::String(phase.to_string()));
        }
    }
    if cleaned.is_empty() {
        return;
    }
    let after = phase_list_label(&cleaned);
    if before == after {
        return;
    }
    *values = cleaned;
    push_auto_fix_entry(
        entries,
        Some(index),
        rule_id,
        "appliesTo",
        &before,
        &after,
        &format!(
            "normalized appliesTo for {} from [{}] to [{}]",
            rule_id.unwrap_or("<unnamed>"),
            before,
            after
        ),
    );
}

fn safe_fix_count_for_rule(rule: &serde_json::Map<String, Value>) -> usize {
    let mut count = 0usize;
    if let Some(material_code) = string_field(rule, "material") {
        if let Some(material) = TransitionMaterial::from_code(material_code) {
            let expected = expected_atlas_group_for_rule(rule, material);
            if string_field(rule, "atlasGroup").unwrap_or("") != expected {
                count += 1;
            }
        }
    }
    if let Some(values) = rule.get("appliesTo").and_then(Value::as_array) {
        let before = phase_list_label(values);
        let mut seen = HashSet::new();
        let mut cleaned = Vec::new();
        for value in values {
            let Some(phase) = value.as_str() else {
                continue;
            };
            if phase != "edge" && phase != "corner" {
                continue;
            }
            if seen.insert(phase.to_string()) {
                cleaned.push(Value::String(phase.to_string()));
            }
        }
        if !cleaned.is_empty() && phase_list_label(&cleaned) != before {
            count += 1;
        }
    }
    count
}

fn atlas_group_fix_preview(
    index: usize,
    id: &str,
    rule: &serde_json::Map<String, Value>,
) -> Option<String> {
    let material_code = string_field(rule, "material")?;
    let material = TransitionMaterial::from_code(material_code)?;
    let expected = expected_atlas_group_for_rule(rule, material);
    let current = string_field(rule, "atlasGroup").unwrap_or("");
    if current == expected {
        return None;
    }
    Some(format!(
        "fixable rule#{index} {id} atlasGroup: {} -> {expected}",
        display_empty(current)
    ))
}

fn phase_list_fix_preview(
    index: usize,
    id: &str,
    rule: &serde_json::Map<String, Value>,
) -> Option<String> {
    let values = rule.get("appliesTo")?.as_array()?;
    let before = phase_list_label(values);
    let mut seen = HashSet::new();
    let mut cleaned = Vec::new();
    for value in values {
        let Some(phase) = value.as_str() else {
            continue;
        };
        if phase != "edge" && phase != "corner" {
            continue;
        }
        if seen.insert(phase.to_string()) {
            cleaned.push(Value::String(phase.to_string()));
        }
    }
    if cleaned.is_empty() {
        return None;
    }
    let after = phase_list_label(&cleaned);
    if before == after {
        return None;
    }
    Some(format!(
        "fixable rule#{index} {id} appliesTo: [{before}] -> [{after}]"
    ))
}

fn push_auto_fix_entry(
    entries: &mut Vec<TransitionRuleDraftAutoFixEntry>,
    manifest_index: Option<usize>,
    rule_id: Option<&str>,
    field: &str,
    before: &str,
    after: &str,
    summary: &str,
) {
    entries.push(TransitionRuleDraftAutoFixEntry {
        manifest_index,
        rule_id: rule_id.map(str::to_string),
        field: field.to_string(),
        before: before.to_string(),
        after: after.to_string(),
        summary: summary.to_string(),
    });
}

fn phase_list_label(values: &[Value]) -> String {
    values
        .iter()
        .map(|value| value.as_str().unwrap_or("<non-string>").to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn display_empty(value: &str) -> &str {
    if value.trim().is_empty() {
        "<empty>"
    } else {
        value
    }
}

fn apply_edit_to_rule(
    rule: &mut serde_json::Map<String, Value>,
    edit: TransitionRuleDraftEdit,
) -> Result<(), String> {
    match edit {
        TransitionRuleDraftEdit::PriorityDelta(delta) => {
            let current = rule.get("priority").and_then(Value::as_i64).unwrap_or(0) as i32;
            let next = (current + delta).clamp(-999, 999);
            rule.insert("priority".to_string(), Value::Number(Number::from(next)));
        }
        TransitionRuleDraftEdit::CycleMaterial(delta) => {
            cycle_string_field(rule, "material", &MATERIAL_CODES, delta)?;
            sync_atlas_group_for_current_material(rule)?;
        }
        TransitionRuleDraftEdit::CycleCenter(delta) => {
            cycle_string_field(rule, "center", &SELECTOR_CODES, delta)?;
        }
        TransitionRuleDraftEdit::CycleNeighbor(delta) => {
            cycle_string_field(rule, "neighbor", &SELECTOR_CODES, delta)?;
        }
        TransitionRuleDraftEdit::SyncAtlasGroup => {
            sync_atlas_group_for_current_material(rule)?;
        }
    }
    Ok(())
}

fn cycle_string_field(
    rule: &mut serde_json::Map<String, Value>,
    field: &str,
    allowed: &[&str],
    delta: i32,
) -> Result<(), String> {
    if allowed.is_empty() {
        return Err(format!("cannot cycle empty allowed list for {field}"));
    }
    let current = string_field(rule, field).unwrap_or(allowed[0]);
    let current_index = allowed
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(0) as i32;
    let next_index = (current_index + delta).rem_euclid(allowed.len() as i32) as usize;
    rule.insert(
        field.to_string(),
        Value::String(allowed[next_index].to_string()),
    );
    Ok(())
}

fn expected_atlas_group_for_rule(
    rule: &serde_json::Map<String, Value>,
    material: TransitionMaterial,
) -> &'static str {
    expected_atlas_group_for_codes(
        string_field(rule, "center"),
        string_field(rule, "neighbor"),
        material,
    )
}

fn sync_atlas_group_for_current_material(
    rule: &mut serde_json::Map<String, Value>,
) -> Result<(), String> {
    let material_code = string_field(rule, "material").unwrap_or("sand_blend");
    let material = TransitionMaterial::from_code(material_code)
        .ok_or_else(|| format!("unknown draft transition material: {material_code}"))?;
    let expected = expected_atlas_group_for_rule(rule, material);
    rule.insert(
        "atlasGroup".to_string(),
        Value::String(expected.to_string()),
    );
    Ok(())
}

fn validate_transition_rule_draft_value(value: &Value) -> Result<usize, String> {
    let rules = value
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "transition-rule draft has no rules array".to_string())?;
    if rules.is_empty() {
        return Err("transition-rule draft rules array is empty".to_string());
    }
    let mut seen_ids = HashSet::new();
    for (index, rule) in rules.iter().enumerate() {
        let object = rule
            .as_object()
            .ok_or_else(|| format!("draft rule {index} is not an object"))?;
        let id = string_field(object, "id").unwrap_or("<unnamed>");
        if !seen_ids.insert(id.to_string()) {
            return Err(format!("duplicate transition-rule draft id {id}"));
        }
        let center = string_field(object, "center")
            .ok_or_else(|| format!("draft rule {id} missing center"))?;
        if TerrainFamilySelector::from_code(center).is_none() {
            return Err(format!(
                "draft rule {id} has unknown center selector {center}"
            ));
        }
        let neighbor = string_field(object, "neighbor")
            .ok_or_else(|| format!("draft rule {id} missing neighbor"))?;
        if TerrainFamilySelector::from_code(neighbor).is_none() {
            return Err(format!(
                "draft rule {id} has unknown neighbor selector {neighbor}"
            ));
        }
        let material = string_field(object, "material")
            .ok_or_else(|| format!("draft rule {id} missing material"))?;
        if TransitionMaterial::from_code(material).is_none() {
            return Err(format!("draft rule {id} has unknown material {material}"));
        }
        let atlas_group = string_field(object, "atlasGroup")
            .ok_or_else(|| format!("draft rule {id} missing atlasGroup"))?;
        if atlas_group.trim().is_empty() {
            return Err(format!("draft rule {id} has empty atlasGroup"));
        }
        let applies_to = object
            .get("appliesTo")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("draft rule {id} missing appliesTo"))?;
        if applies_to.is_empty() {
            return Err(format!("draft rule {id} has empty appliesTo"));
        }
        for phase in applies_to {
            let phase = phase
                .as_str()
                .ok_or_else(|| format!("draft rule {id} has non-string appliesTo entry"))?;
            if phase != "edge" && phase != "corner" {
                return Err(format!("draft rule {id} has unknown phase {phase}"));
            }
        }
        if object.get("priority").and_then(Value::as_i64).is_none() {
            return Err(format!("draft rule {id} missing integer priority"));
        }
    }
    Ok(rules.len())
}

fn annotate_draft(value: &mut Value, action: &str) {
    if let Value::Object(object) = value {
        object.insert(
            "draft".to_string(),
            json!({
                "safeDraft": true,
                "action": action,
                "sourceManifest": TERRAIN_TRANSITION_RULE_MANIFEST_PATH,
                "promotionPolicy": "Review this draft and copy/promote deliberately; runtime still reads the live content/worldgen manifest.",
                "editableFields": ["priority", "material", "atlasGroup", "center", "neighbor"]
            }),
        );
    }
}

fn report(
    path: PathBuf,
    rule_count: usize,
    selected_rule_id: Option<String>,
    summary: String,
) -> TransitionRuleDraftReport {
    TransitionRuleDraftReport {
        path,
        rule_count,
        selected_rule_id,
        summary,
    }
}

fn rule_at_index(value: &Value, manifest_index: usize) -> Option<&serde_json::Map<String, Value>> {
    value
        .get("rules")?
        .as_array()?
        .get(manifest_index)?
        .as_object()
}

fn rule_at_index_mut(
    value: &mut Value,
    manifest_index: usize,
) -> Option<&mut serde_json::Map<String, Value>> {
    value
        .get_mut("rules")?
        .as_array_mut()?
        .get_mut(manifest_index)?
        .as_object_mut()
}

fn string_field<'a>(object: &'a serde_json::Map<String, Value>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

fn read_manifest_value(path: &Path) -> Result<Value, String> {
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_manifest_value(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let pretty = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize transition-rule draft: {error}"))?;
    write(path, pretty).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn snapshot_existing_draft_for_undo(action: &str) -> Result<Option<usize>, String> {
    let draft_path = transition_rule_draft_path();
    if !draft_path.exists() {
        return Ok(None);
    }
    let undo_path = transition_rule_draft_undo_path();
    match read_manifest_value(&draft_path) {
        Ok(mut value) => {
            let rule_count = validate_transition_rule_draft_value(&value).unwrap_or_default();
            annotate_draft(&mut value, action);
            write_manifest_value(&undo_path, &value)?;
            Ok(Some(rule_count))
        }
        Err(_) => {
            if let Some(parent) = undo_path.parent() {
                create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            copy(&draft_path, &undo_path).map_err(|error| {
                format!(
                    "failed to preserve invalid transition-rule draft undo snapshot from {} to {}: {error}",
                    draft_path.display(),
                    undo_path.display()
                )
            })?;
            Ok(Some(0))
        }
    }
}

fn transition_rule_draft_path() -> PathBuf {
    repo_root_dir().join(TERRAIN_TRANSITION_RULE_DRAFT_PATH)
}

fn transition_rule_draft_undo_path() -> PathBuf {
    repo_root_dir().join(TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH)
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_world")
}

#[cfg(test)]
mod tests;
