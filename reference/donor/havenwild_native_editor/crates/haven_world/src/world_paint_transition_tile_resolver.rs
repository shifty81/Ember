use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    load_world_paint_material_state_document, resolve_world_paint_material_adjacency,
    resolve_world_paint_material_scene_adjacency, WorldPaintMaterialAdjacencyReport,
};

pub const WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA: &str =
    "havenwild.world_paint_transition_tile_resolver.v0.1";
pub const WORLD_TILE_ATLAS_MANIFEST_PATH: &str =
    "content/assets/world_tiles/havenwild_world_environment_test_v0_1.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldPaintTransitionAtlasManifest {
    pub records: Vec<WorldPaintTransitionTileRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldPaintTransitionTileRecord {
    pub id: String,
    pub atlas_rect: [u32; 4],
    pub family: String,
    pub layer: String,
    #[serde(default)]
    pub autotile_role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintTransitionTileResolution {
    pub schema: String,
    pub scene_id: String,
    pub x: i32,
    pub y: i32,
    pub family: String,
    pub layer: String,
    pub neighbor_bits: u8,
    pub transition_kind: String,
    pub selected_tile_id: String,
    pub selected_atlas_rect: [u32; 4],
    pub fallback_tile_id: String,
    pub atlas_source: String,
    pub status: String,
}

impl WorldPaintTransitionTileResolution {
    pub fn status_line(&self) -> String {
        format!(
            "Tile resolve {} {},{}: {} {} bits={} -> {} ({})",
            self.scene_id,
            self.x,
            self.y,
            self.family,
            self.transition_kind,
            self.neighbor_bits,
            self.selected_tile_id,
            self.status
        )
    }

    pub fn empty(scene_id: impl Into<String>, x: i32, y: i32, status: impl Into<String>) -> Self {
        Self {
            schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
            scene_id: scene_id.into(),
            x,
            y,
            family: "none".to_string(),
            layer: "none".to_string(),
            neighbor_bits: 0,
            transition_kind: "none".to_string(),
            selected_tile_id: "none".to_string(),
            selected_atlas_rect: [0, 0, 0, 0],
            fallback_tile_id: "none".to_string(),
            atlas_source: WORLD_TILE_ATLAS_MANIFEST_PATH.to_string(),
            status: status.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintTransitionSceneTileDetails {
    pub schema: String,
    pub scene_id: String,
    pub resolutions: Vec<WorldPaintTransitionTileResolution>,
    pub status: String,
}

impl WorldPaintTransitionSceneTileDetails {
    pub fn status_line(&self) -> String {
        format!(
            "Tile render binding scene {}: {} resolved tile record(s)",
            self.scene_id,
            self.resolutions.len()
        )
    }
}

pub struct WorldPaintTransitionSceneResolutionReport {
    pub schema: String,
    pub scene_id: String,
    pub resolved_cells: usize,
    pub shoreline_tiles: usize,
    pub cave_edge_tiles: usize,
    pub paved_brick_edge_tiles: usize,
    pub wood_floor_edge_tiles: usize,
    pub fallback_tiles: usize,
    pub status: String,
}

impl WorldPaintTransitionSceneResolutionReport {
    pub fn status_line(&self) -> String {
        format!(
            "Tile resolve scene {}: {} resolved, shore {}, cave {}, brick {}, wood {}, fallbacks {}",
            self.scene_id,
            self.resolved_cells,
            self.shoreline_tiles,
            self.cave_edge_tiles,
            self.paved_brick_edge_tiles,
            self.wood_floor_edge_tiles,
            self.fallback_tiles
        )
    }
}

pub fn resolve_world_paint_transition_tile(
    repo_root: impl AsRef<Path>,
    material_state_path: impl AsRef<Path>,
    scene_id: &str,
    x: i32,
    y: i32,
) -> io::Result<WorldPaintTransitionTileResolution> {
    let manifest = load_transition_manifest(repo_root.as_ref())?;
    let state = load_world_paint_material_state_document(material_state_path)?;
    let adjacency = resolve_world_paint_material_adjacency(&state, scene_id, x, y);
    Ok(resolve_transition_tile_from_adjacency(&manifest, adjacency))
}

pub fn resolve_world_paint_scene_transition_tile_details(
    repo_root: impl AsRef<Path>,
    material_state_path: impl AsRef<Path>,
    scene_id: &str,
) -> io::Result<WorldPaintTransitionSceneTileDetails> {
    let state = load_world_paint_material_state_document(material_state_path)?;
    let Some(scene) = state.scenes.iter().find(|scene| scene.scene_id == scene_id) else {
        return Ok(WorldPaintTransitionSceneTileDetails {
            schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
            scene_id: scene_id.to_string(),
            resolutions: Vec::new(),
            status: format!("No material-state scene {scene_id}"),
        });
    };
    if scene.cells.is_empty() {
        return Ok(WorldPaintTransitionSceneTileDetails {
            schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
            scene_id: scene_id.to_string(),
            resolutions: Vec::new(),
            status: format!("No material-state cells for scene {scene_id}"),
        });
    }
    let manifest = load_transition_manifest(repo_root.as_ref())?;
    let mut coords = scene
        .cells
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<Vec<_>>();
    coords.sort_unstable();
    coords.dedup();
    let mut resolutions = Vec::with_capacity(coords.len());
    for (x, y) in coords {
        let adjacency = resolve_world_paint_material_adjacency(&state, scene_id, x, y);
        let resolved = resolve_transition_tile_from_adjacency(&manifest, adjacency);
        if resolved.selected_tile_id != "none" {
            resolutions.push(resolved);
        }
    }
    let mut details = WorldPaintTransitionSceneTileDetails {
        schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
        scene_id: scene_id.to_string(),
        resolutions,
        status: String::new(),
    };
    details.status = details.status_line();
    Ok(details)
}

pub fn resolve_world_paint_scene_transition_tiles(
    repo_root: impl AsRef<Path>,
    material_state_path: impl AsRef<Path>,
    scene_id: &str,
) -> io::Result<WorldPaintTransitionSceneResolutionReport> {
    let state = load_world_paint_material_state_document(material_state_path)?;
    let scene_summary = resolve_world_paint_material_scene_adjacency(&state, scene_id);
    let mut report = WorldPaintTransitionSceneResolutionReport {
        schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
        scene_id: scene_id.to_string(),
        resolved_cells: 0,
        shoreline_tiles: 0,
        cave_edge_tiles: 0,
        paved_brick_edge_tiles: 0,
        wood_floor_edge_tiles: 0,
        fallback_tiles: 0,
        status: String::new(),
    };
    if scene_summary.cell_reports == 0 {
        report.status = format!("No material-state cells for scene {scene_id}");
        return Ok(report);
    }
    let Some(scene) = state.scenes.iter().find(|scene| scene.scene_id == scene_id) else {
        report.status = format!("No material-state scene {scene_id}");
        return Ok(report);
    };
    let manifest = load_transition_manifest(repo_root.as_ref())?;
    let mut coords = scene
        .cells
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<Vec<_>>();
    coords.sort_unstable();
    coords.dedup();
    for (x, y) in coords {
        let adjacency = resolve_world_paint_material_adjacency(&state, scene_id, x, y);
        let resolved = resolve_transition_tile_from_adjacency(&manifest, adjacency);
        if resolved.selected_tile_id != "none" {
            report.resolved_cells += 1;
        }
        report.shoreline_tiles += (resolved.transition_kind == "shoreline") as usize;
        report.cave_edge_tiles += (resolved.transition_kind == "cave_edge") as usize;
        report.paved_brick_edge_tiles += (resolved.transition_kind == "paved_brick_edge") as usize;
        report.wood_floor_edge_tiles += (resolved.transition_kind == "wood_floor_edge") as usize;
        report.fallback_tiles += (resolved.status.contains("fallback")) as usize;
    }
    report.status = report.status_line();
    Ok(report)
}

fn resolve_transition_tile_from_adjacency(
    manifest: &WorldPaintTransitionAtlasManifest,
    adjacency: WorldPaintMaterialAdjacencyReport,
) -> WorldPaintTransitionTileResolution {
    if adjacency.primary_family == "none" {
        return WorldPaintTransitionTileResolution::empty(
            adjacency.scene_id,
            adjacency.x,
            adjacency.y,
            adjacency.status,
        );
    }
    let (transition_kind, preferred_family, preferred_layer, preferred_role) =
        if adjacency.shoreline_candidate {
            ("shoreline", "sand", "ground_transition_fringe", "shoreline")
        } else if adjacency.cave_edge_candidate {
            ("cave_edge", "cave", "cave_wall_face", "wall_edge")
        } else if adjacency.paved_brick_edge_candidate {
            ("paved_brick_edge", "paved_brick", "town_surface", "edge")
        } else if adjacency.wood_floor_edge_candidate {
            ("wood_floor_edge", "wood_plank", "indoor_floor", "edge")
        } else {
            (
                "base",
                adjacency.primary_family.as_str(),
                adjacency.primary_layer.as_str(),
                "base",
            )
        };

    let neighbor_bits = adjacency.neighbor_mask.cardinal_bits();
    let preferred_index = preferred_variant_index(neighbor_bits);
    let mut candidates = manifest
        .records
        .iter()
        .filter(|record| record.family == preferred_family)
        .filter(|record| record.layer == preferred_layer)
        .filter(|record| record.autotile_role == preferred_role)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        candidates = manifest
            .records
            .iter()
            .filter(|record| record.family == preferred_family)
            .filter(|record| record.layer == preferred_layer)
            .collect::<Vec<_>>();
    }
    if candidates.is_empty() {
        return WorldPaintTransitionTileResolution::empty(
            adjacency.scene_id,
            adjacency.x,
            adjacency.y,
            format!("No atlas candidates for family={preferred_family} layer={preferred_layer} role={preferred_role}"),
        );
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    let idx = preferred_index % candidates.len();
    let selected = candidates[idx];
    let fallback = candidates[0];
    let status = if selected.id == fallback.id {
        format!("selected first/fallback candidate role={preferred_role}")
    } else {
        format!("selected deterministic variant index {preferred_index} role={preferred_role}")
    };
    WorldPaintTransitionTileResolution {
        schema: WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
        scene_id: adjacency.scene_id,
        x: adjacency.x,
        y: adjacency.y,
        family: preferred_family.to_string(),
        layer: preferred_layer.to_string(),
        neighbor_bits,
        transition_kind: transition_kind.to_string(),
        selected_tile_id: selected.id.clone(),
        selected_atlas_rect: selected.atlas_rect,
        fallback_tile_id: fallback.id.clone(),
        atlas_source: WORLD_TILE_ATLAS_MANIFEST_PATH.to_string(),
        status,
    }
}

fn preferred_variant_index(neighbor_bits: u8) -> usize {
    // Keep deterministic and cheap: cardinal bits choose the natural 16-entry atlas row.
    // This intentionally avoids storing rendered overlays as authoritative state.
    neighbor_bits as usize
}

fn load_transition_manifest(repo_root: &Path) -> io::Result<WorldPaintTransitionAtlasManifest> {
    let path = repo_root.join(WORLD_TILE_ATLAS_MANIFEST_PATH);
    let text = read_to_string(&path)?;
    let manifest =
        serde_json::from_str::<WorldPaintTransitionAtlasManifest>(&text).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse {}: {err}", path.display()),
            )
        })?;
    Ok(manifest)
}

#[allow(dead_code)]
fn record_counts_by_family(records: &[WorldPaintTransitionTileRecord]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in records {
        *counts.entry(record.family.clone()).or_insert(0) += 1;
    }
    counts
}
