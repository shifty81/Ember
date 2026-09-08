use std::fs;
use std::io;
use std::path::Path;

use haven_core::{MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{WorldPaintDeltaRecord, WorldPaintFamily, WorldPaintLayer, WorldPaintSubcellMode};

pub const WORLD_PAINT_MATERIAL_STATE_SCHEMA: &str = "havenwild.world_paint_material_state.v0.1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldPaintMaterialCell {
    pub x: i32,
    pub y: i32,
    pub family: String,
    pub layer: String,
    pub subcell_mode: String,
    pub subcell_grid: [u32; 2],
    pub weights_u8: Vec<u8>,
    pub resolved_tile_kind: String,
    pub last_sequence: u64,
    pub edit_source: String,
}

impl WorldPaintMaterialCell {
    pub fn key(&self) -> (i32, i32, &str) {
        (self.x, self.y, self.layer.as_str())
    }

    pub fn weight_summary(&self) -> String {
        let max = self.weights_u8.iter().copied().max().unwrap_or(0);
        format!("{} weight cell(s), max {}", self.weights_u8.len(), max)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldPaintSceneMaterialState {
    pub scene_id: String,
    pub width_tiles: u32,
    pub height_tiles: u32,
    pub cells: Vec<WorldPaintMaterialCell>,
}

impl WorldPaintSceneMaterialState {
    pub fn new(scene_id: impl Into<String>) -> Self {
        Self {
            scene_id: scene_id.into(),
            width_tiles: MAP_W as u32,
            height_tiles: MAP_H as u32,
            cells: Vec::new(),
        }
    }

    pub fn upsert_cell(&mut self, cell: WorldPaintMaterialCell) {
        if let Some(existing) = self.cells.iter_mut().find(|existing| {
            existing.x == cell.x && existing.y == cell.y && existing.layer == cell.layer
        }) {
            *existing = cell;
        } else {
            self.cells.push(cell);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WorldPaintMaterialStateDocument {
    pub schema: String,
    pub version: u32,
    pub canonical_tile_size: [u32; 2],
    pub supported_subcell_modes: Vec<String>,
    pub authority_policy: String,
    pub visual_policy: String,
    pub scenes: Vec<WorldPaintSceneMaterialState>,
}

impl Default for WorldPaintMaterialStateDocument {
    fn default() -> Self {
        Self {
            schema: WORLD_PAINT_MATERIAL_STATE_SCHEMA.to_string(),
            version: 1,
            canonical_tile_size: [32, 32],
            supported_subcell_modes: vec![
                "32x32".to_string(),
                "16x16".to_string(),
                "8x8".to_string(),
                "4x4".to_string(),
            ],
            authority_policy: "host/server stores material family, layer, subcell weights, and operation sequence; rendered transitions remain derived".to_string(),
            visual_policy: "clients derive shoreline, foam, debris, and variation from material state plus deterministic manifests".to_string(),
            scenes: Vec::new(),
        }
    }
}

impl WorldPaintMaterialStateDocument {
    pub fn scene_mut(&mut self, scene_id: &str) -> &mut WorldPaintSceneMaterialState {
        if let Some(index) = self
            .scenes
            .iter()
            .position(|scene| scene.scene_id == scene_id)
        {
            return &mut self.scenes[index];
        }
        self.scenes
            .push(WorldPaintSceneMaterialState::new(scene_id));
        self.scenes.last_mut().expect("scene was just pushed")
    }

    pub fn total_cells(&self) -> usize {
        self.scenes.iter().map(|scene| scene.cells.len()).sum()
    }

    pub fn status_line(&self) -> String {
        format!(
            "Material state: {} scene(s), {} cell/layer record(s)",
            self.scenes.len(),
            self.total_cells()
        )
    }

    pub fn inspect_cell(&self, scene_id: &str, x: i32, y: i32) -> WorldPaintMaterialCellInspection {
        inspect_world_paint_material_cell(self, scene_id, x, y)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldPaintMaterialCellInspection {
    pub scene_id: String,
    pub x: i32,
    pub y: i32,
    pub entries: Vec<WorldPaintMaterialCell>,
    pub ready_for_adjacency: bool,
    pub ready_for_debris: bool,
    pub status: String,
}

impl WorldPaintMaterialCellInspection {
    pub fn empty(scene_id: impl Into<String>, x: i32, y: i32) -> Self {
        let scene_id = scene_id.into();
        Self {
            scene_id: scene_id.clone(),
            x,
            y,
            entries: Vec::new(),
            ready_for_adjacency: false,
            ready_for_debris: false,
            status: format!("No material-state record for {} {},{}", scene_id, x, y),
        }
    }

    pub fn primary(&self) -> Option<&WorldPaintMaterialCell> {
        self.entries.iter().max_by_key(|cell| cell.last_sequence)
    }

    pub fn status_line(&self) -> String {
        if let Some(primary) = self.primary() {
            format!(
                "{} {},{}: {} on {} via {} ({}), seq #{}",
                self.scene_id,
                self.x,
                self.y,
                primary.family,
                primary.layer,
                primary.subcell_mode,
                primary.weight_summary(),
                primary.last_sequence
            )
        } else {
            self.status.clone()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldPaintMaterialStateApplyReport {
    pub path: String,
    pub sequence: u64,
    pub scene_id: String,
    pub upserted_cells: usize,
    pub total_cells: usize,
    pub status: String,
}

impl WorldPaintMaterialStateApplyReport {
    pub fn status_line(&self) -> String {
        format!(
            "Material state #{} applied to {}: {} cell/layer update(s), {} total -> {}",
            self.sequence, self.scene_id, self.upserted_cells, self.total_cells, self.path
        )
    }
}

pub fn load_world_paint_material_state_document(
    path: impl AsRef<Path>,
) -> io::Result<WorldPaintMaterialStateDocument> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(WorldPaintMaterialStateDocument::default());
    }
    let text = fs::read_to_string(path)?;
    let doc: WorldPaintMaterialStateDocument = serde_json::from_str(&text).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "failed to parse world paint material state {}: {}",
                path.display(),
                err
            ),
        )
    })?;
    Ok(doc)
}

pub fn save_world_paint_material_state_document(
    path: impl AsRef<Path>,
    doc: &WorldPaintMaterialStateDocument,
) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(doc).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize world paint material state: {}", err),
        )
    })?;
    fs::write(path, text)
}

pub fn apply_world_paint_delta_record_to_material_state_path(
    path: impl AsRef<Path>,
    record: &WorldPaintDeltaRecord,
) -> io::Result<WorldPaintMaterialStateApplyReport> {
    let path_ref = path.as_ref();
    let mut doc = load_world_paint_material_state_document(path_ref)?;
    let upserted_cells = apply_world_paint_delta_record_to_material_state(&mut doc, record);
    let issue_count = validate_world_paint_material_state_document(&doc).len();
    save_world_paint_material_state_document(path_ref, &doc)?;
    Ok(WorldPaintMaterialStateApplyReport {
        path: path_ref.display().to_string(),
        sequence: record.sequence,
        scene_id: record.scene_id.clone(),
        upserted_cells,
        total_cells: doc.total_cells(),
        status: if issue_count == 0 {
            "ok".to_string()
        } else {
            format!("saved with {} validation issue(s)", issue_count)
        },
    })
}

pub fn apply_world_paint_delta_record_to_material_state(
    doc: &mut WorldPaintMaterialStateDocument,
    record: &WorldPaintDeltaRecord,
) -> usize {
    let family = WorldPaintFamily::from_code(&record.family);
    let layer = WorldPaintLayer::from_code(&record.layer);
    let subcell_mode = WorldPaintSubcellMode::from_code(&record.subcell_mode);
    if family.is_none() || layer.is_none() || subcell_mode.is_none() {
        return 0;
    }

    let scene = doc.scene_mut(&record.scene_id);
    let mut count = 0usize;
    for [paint_center_x, paint_center_y] in material_state_centers(record) {
        for [x, y] in cells_in_brush(paint_center_x, paint_center_y, record.radius_tiles) {
            if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
                continue;
            }
            let weights = weights_for_subcell_grid(record.subcell_grid, record.strength);
            scene.upsert_cell(WorldPaintMaterialCell {
                x,
                y,
                family: record.family.clone(),
                layer: record.layer.clone(),
                subcell_mode: record.subcell_mode.clone(),
                subcell_grid: record.subcell_grid,
                weights_u8: weights,
                resolved_tile_kind: record.resolved_tile_kind.clone(),
                last_sequence: record.sequence,
                edit_source: "world_paint_delta".to_string(),
            });
            count += 1;
        }
    }
    count
}

pub fn material_state_centers(record: &WorldPaintDeltaRecord) -> Vec<[i32; 2]> {
    if !record.mirror_centers.is_empty() {
        let mut centers = record.mirror_centers.clone();
        centers.sort_unstable();
        centers.dedup();
        return centers;
    }
    vec![record.center]
}

pub fn cells_in_brush(center_x: i32, center_y: i32, radius_tiles: i32) -> Vec<[i32; 2]> {
    let radius = radius_tiles.max(1);
    let radius_f = radius as f32 + 0.001;
    let mut cells = Vec::new();
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            let dx = x - center_x;
            let dy = y - center_y;
            let distance = ((dx * dx + dy * dy) as f32).sqrt();
            if distance <= radius_f {
                cells.push([x, y]);
            }
        }
    }
    cells
}

pub fn weights_for_subcell_grid(grid: [u32; 2], strength: f32) -> Vec<u8> {
    let clamped = strength.clamp(0.0, 1.0);
    let weight = (clamped * 255.0).round() as u8;
    let count = (grid[0].max(1) * grid[1].max(1)) as usize;
    vec![weight; count]
}

pub fn inspect_world_paint_material_cell(
    doc: &WorldPaintMaterialStateDocument,
    scene_id: &str,
    x: i32,
    y: i32,
) -> WorldPaintMaterialCellInspection {
    let Some(scene) = doc.scenes.iter().find(|scene| scene.scene_id == scene_id) else {
        return WorldPaintMaterialCellInspection::empty(scene_id.to_string(), x, y);
    };

    let mut entries: Vec<WorldPaintMaterialCell> = scene
        .cells
        .iter()
        .filter(|cell| cell.x == x && cell.y == y)
        .cloned()
        .collect();
    entries.sort_by(|a, b| {
        a.layer
            .cmp(&b.layer)
            .then(a.last_sequence.cmp(&b.last_sequence))
            .then(a.family.cmp(&b.family))
    });

    if entries.is_empty() {
        return WorldPaintMaterialCellInspection::empty(scene_id.to_string(), x, y);
    }

    let has_water = entries.iter().any(|entry| entry.family == "water");
    let has_ground = entries.iter().any(|entry| {
        matches!(
            entry.family.as_str(),
            "sand" | "cave" | "paved_brick" | "wood_plank"
        )
    });
    let ready_for_adjacency = entries.iter().any(|entry| {
        matches!(entry.family.as_str(), "water" | "sand" | "cave")
            && matches!(
                entry.layer.as_str(),
                "water_base" | "ground_base" | "cave_base"
            )
    });
    let ready_for_debris = entries.iter().any(|entry| {
        matches!(
            entry.family.as_str(),
            "sand" | "cave" | "paved_brick" | "wood_plank"
        )
    });
    let status = format!(
        "Material inspector: {} entry(s), adjacency {}, debris {}, shore candidate {}",
        entries.len(),
        if ready_for_adjacency {
            "ready"
        } else {
            "not-ready"
        },
        if ready_for_debris {
            "ready"
        } else {
            "not-ready"
        },
        if has_water && has_ground { "yes" } else { "no" }
    );

    WorldPaintMaterialCellInspection {
        scene_id: scene_id.to_string(),
        x,
        y,
        entries,
        ready_for_adjacency,
        ready_for_debris,
        status,
    }
}

pub fn inspect_world_paint_material_cell_path(
    path: impl AsRef<Path>,
    scene_id: &str,
    x: i32,
    y: i32,
) -> io::Result<WorldPaintMaterialCellInspection> {
    let doc = load_world_paint_material_state_document(path)?;
    Ok(inspect_world_paint_material_cell(&doc, scene_id, x, y))
}

pub fn validate_world_paint_material_state_document(
    doc: &WorldPaintMaterialStateDocument,
) -> Vec<String> {
    let mut issues = Vec::new();
    if doc.schema != WORLD_PAINT_MATERIAL_STATE_SCHEMA {
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
    for scene in &doc.scenes {
        if scene.scene_id.trim().is_empty() {
            issues.push("material state scene has empty scene_id".to_string());
        }
        if scene.width_tiles != MAP_W as u32 || scene.height_tiles != MAP_H as u32 {
            issues.push(format!(
                "scene {} dimensions must be {}x{}, found {}x{}",
                scene.scene_id, MAP_W, MAP_H, scene.width_tiles, scene.height_tiles
            ));
        }
        for cell in &scene.cells {
            if cell.x < 0 || cell.y < 0 || cell.x >= MAP_W as i32 || cell.y >= MAP_H as i32 {
                issues.push(format!(
                    "scene {} cell {},{} outside scene",
                    scene.scene_id, cell.x, cell.y
                ));
            }
            if WorldPaintFamily::from_code(&cell.family).is_none() {
                issues.push(format!(
                    "scene {} cell {},{} unknown family {}",
                    scene.scene_id, cell.x, cell.y, cell.family
                ));
            }
            if WorldPaintLayer::from_code(&cell.layer).is_none() {
                issues.push(format!(
                    "scene {} cell {},{} unknown layer {}",
                    scene.scene_id, cell.x, cell.y, cell.layer
                ));
            }
            let Some(mode) = WorldPaintSubcellMode::from_code(&cell.subcell_mode) else {
                issues.push(format!(
                    "scene {} cell {},{} unknown subcell mode {}",
                    scene.scene_id, cell.x, cell.y, cell.subcell_mode
                ));
                continue;
            };
            if cell.subcell_grid != mode.grid() {
                issues.push(format!(
                    "scene {} cell {},{} subcell grid does not match mode",
                    scene.scene_id, cell.x, cell.y
                ));
            }
            let expected_weights =
                (cell.subcell_grid[0].max(1) * cell.subcell_grid[1].max(1)) as usize;
            if cell.weights_u8.len() != expected_weights {
                issues.push(format!(
                    "scene {} cell {},{} expected {} weights, found {}",
                    scene.scene_id,
                    cell.x,
                    cell.y,
                    expected_weights,
                    cell.weights_u8.len()
                ));
            }
            if cell.last_sequence == 0 {
                issues.push(format!(
                    "scene {} cell {},{} missing sequence",
                    scene.scene_id, cell.x, cell.y
                ));
            }
        }
    }
    issues
}
