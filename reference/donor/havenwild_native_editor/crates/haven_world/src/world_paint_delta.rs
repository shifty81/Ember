use std::fs;
use std::io;
use std::path::Path;

use haven_core::SceneReference;
use serde::{Deserialize, Serialize};

use crate::{WorldPaintBrushSettings, WorldPaintReport};

pub const WORLD_PAINT_DELTA_SCHEMA: &str = "havenwild.world_paint_deltas.v0.1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintDeltaBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldPaintDeltaRecord {
    pub sequence: u64,
    pub scene_id: String,
    pub center: [i32; 2],
    pub bounds: Option<WorldPaintDeltaBounds>,
    pub family: String,
    pub layer: String,
    pub subcell_mode: String,
    pub subcell_grid: [u32; 2],
    pub strength: f32,
    pub radius_tiles: i32,
    #[serde(default)]
    pub mirror_horizontal: bool,
    #[serde(default)]
    pub mirror_vertical: bool,
    #[serde(default)]
    pub mirror_centers: Vec<[i32; 2]>,
    pub painted_tiles: usize,
    pub affected_subcells: usize,
    pub resolved_tile_kind: String,
    pub autotile_refresh_requested: bool,
    pub autotile_refreshed: bool,
    pub cleanup_mutations: usize,
    pub authoritative_note: String,
    pub client_visual_note: String,
}

impl WorldPaintDeltaRecord {
    pub fn from_report(
        scene_id: &SceneReference,
        center_x: i32,
        center_y: i32,
        sequence: u64,
        settings: WorldPaintBrushSettings,
        report: &WorldPaintReport,
    ) -> Self {
        let settings = settings.normalized();
        let bounds = report.changed_bounds.map(|bounds| WorldPaintDeltaBounds {
            min_x: bounds.min_x,
            min_y: bounds.min_y,
            max_x: bounds.max_x,
            max_y: bounds.max_y,
        });
        Self {
            sequence,
            scene_id: scene_id.code().to_string(),
            center: [center_x, center_y],
            bounds,
            family: settings.family.code().to_string(),
            layer: settings.layer.code().to_string(),
            subcell_mode: settings.subcell_mode.code().to_string(),
            subcell_grid: settings.subcell_mode.grid(),
            strength: settings.strength,
            radius_tiles: settings.radius_tiles,
            mirror_horizontal: settings.mirror_horizontal,
            mirror_vertical: settings.mirror_vertical,
            mirror_centers: report.mirror_centers.clone(),
            painted_tiles: report.painted_tiles,
            affected_subcells: report.affected_subcells,
            resolved_tile_kind: format!("{:?}", settings.target_tile()),
            autotile_refresh_requested: settings.autotile_refresh,
            autotile_refreshed: report.autotile_refreshed,
            cleanup_mutations: report.cleanup_mutations,
            authoritative_note: "Store this record or an equivalent compact operation on the host/server for replay, sync, migration, and undo history.".to_string(),
            client_visual_note: "Clients derive shoreline, foam, variation, and debris visuals from deterministic manifests; raw visual overlays are not authoritative network state.".to_string(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "#{:04} {} {} {} at {},{} mirror {}{} -> {} tile(s), {} subcell(s)",
            self.sequence,
            self.scene_id,
            self.family,
            self.subcell_mode,
            self.center[0],
            self.center[1],
            if self.mirror_horizontal { "H" } else { "-" },
            if self.mirror_vertical { "V" } else { "-" },
            self.painted_tiles,
            self.affected_subcells
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldPaintDeltaDocument {
    pub schema: String,
    pub version: u32,
    pub canonical_tile_size: [u32; 2],
    pub supported_subcell_modes: Vec<String>,
    pub multiplayer_authority: String,
    pub client_visual_policy: String,
    pub records: Vec<WorldPaintDeltaRecord>,
}

impl Default for WorldPaintDeltaDocument {
    fn default() -> Self {
        Self {
            schema: WORLD_PAINT_DELTA_SCHEMA.to_string(),
            version: 1,
            canonical_tile_size: [32, 32],
            supported_subcell_modes: vec![
                "32x32".to_string(),
                "16x16".to_string(),
                "8x8".to_string(),
                "4x4".to_string(),
            ],
            multiplayer_authority: "host/server authoritative operation log".to_string(),
            client_visual_policy:
                "derive visual overlays deterministically from tile/material IDs and manifests"
                    .to_string(),
            records: Vec::new(),
        }
    }
}

impl WorldPaintDeltaDocument {
    pub fn next_sequence(&self) -> u64 {
        self.records
            .iter()
            .map(|record| record.sequence)
            .max()
            .unwrap_or(0)
            + 1
    }

    pub fn append(&mut self, record: WorldPaintDeltaRecord) {
        self.records.push(record);
    }

    pub fn last_summary(&self) -> String {
        self.records
            .last()
            .map(WorldPaintDeltaRecord::summary)
            .unwrap_or_else(|| "No world paint deltas recorded".to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldPaintDeltaAppendReport {
    pub path: String,
    pub sequence: u64,
    pub record_count: usize,
    pub status: String,
}

impl WorldPaintDeltaAppendReport {
    pub fn status_line(&self) -> String {
        format!(
            "Paint delta #{:04} persisted ({} total) -> {}",
            self.sequence, self.record_count, self.path
        )
    }
}

pub fn load_world_paint_delta_document(
    path: impl AsRef<Path>,
) -> io::Result<WorldPaintDeltaDocument> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(WorldPaintDeltaDocument::default());
    }
    let text = fs::read_to_string(path)?;
    let doc: WorldPaintDeltaDocument = serde_json::from_str(&text).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to parse world paint delta document {}: {}",
                path.display(),
                err
            ),
        )
    })?;
    Ok(doc)
}

pub fn save_world_paint_delta_document(
    path: impl AsRef<Path>,
    doc: &WorldPaintDeltaDocument,
) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(doc).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize world paint delta document: {}", err),
        )
    })?;
    fs::write(path, text)
}

pub fn append_world_paint_delta_record(
    path: impl AsRef<Path>,
    record: WorldPaintDeltaRecord,
) -> io::Result<WorldPaintDeltaAppendReport> {
    let path_ref = path.as_ref();
    let mut doc = load_world_paint_delta_document(path_ref)?;
    let sequence = record.sequence;
    doc.append(record);
    let issue_count = validate_world_paint_delta_document(&doc).len();
    save_world_paint_delta_document(path_ref, &doc)?;
    Ok(WorldPaintDeltaAppendReport {
        path: path_ref.display().to_string(),
        sequence,
        record_count: doc.records.len(),
        status: if issue_count == 0 {
            "ok".to_string()
        } else {
            format!("saved with {} validation issue(s)", issue_count)
        },
    })
}

pub fn validate_world_paint_delta_document(doc: &WorldPaintDeltaDocument) -> Vec<String> {
    let mut issues = Vec::new();
    if doc.schema != WORLD_PAINT_DELTA_SCHEMA {
        issues.push(format!("unexpected schema: {}", doc.schema));
    }
    if doc.canonical_tile_size != [32, 32] {
        issues.push(format!(
            "canonical_tile_size must be [32,32], found {:?}",
            doc.canonical_tile_size
        ));
    }
    for mode in ["32x32", "16x16", "8x8", "4x4"] {
        if !doc
            .supported_subcell_modes
            .iter()
            .any(|candidate| candidate == mode)
        {
            issues.push(format!("missing supported subcell mode {}", mode));
        }
    }
    let mut last_sequence = 0u64;
    for record in &doc.records {
        if record.sequence == 0 {
            issues.push("record sequence must be non-zero".to_string());
        }
        if record.sequence <= last_sequence {
            issues.push(format!(
                "record sequence {} is not strictly increasing",
                record.sequence
            ));
        }
        last_sequence = record.sequence;
        if record.painted_tiles == 0 {
            issues.push(format!("record {} painted zero tiles", record.sequence));
        }
        if !doc
            .supported_subcell_modes
            .iter()
            .any(|mode| mode == &record.subcell_mode)
        {
            issues.push(format!(
                "record {} has unsupported subcell mode {}",
                record.sequence, record.subcell_mode
            ));
        }
        if record.family.trim().is_empty() || record.layer.trim().is_empty() {
            issues.push(format!("record {} has empty family/layer", record.sequence));
        }
        if record.strength <= 0.0 || record.strength > 1.0 {
            issues.push(format!("record {} strength outside 0..1", record.sequence));
        }
        if record.radius_tiles < 1 {
            issues.push(format!("record {} radius must be >= 1", record.sequence));
        }
        if record.mirror_centers.is_empty() {
            issues.push(format!("record {} has no mirror centers", record.sequence));
        }
        if !record.mirror_horizontal && !record.mirror_vertical && record.mirror_centers.len() > 1 {
            issues.push(format!(
                "record {} has mirror centers but mirror flags are off",
                record.sequence
            ));
        }
    }
    issues
}
