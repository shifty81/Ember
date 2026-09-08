use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    resolve_world_paint_scene_transition_tile_details, WorldPaintTransitionSceneTileDetails,
    WorldPaintTransitionTileResolution,
};

pub const WORLD_PAINT_RENDER_CACHE_SCHEMA: &str = "havenwild.world_paint_render_cache.v0.1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintRenderCacheEntry {
    pub scene_id: String,
    pub x: i32,
    pub y: i32,
    pub family: String,
    pub layer: String,
    pub transition_kind: String,
    pub neighbor_bits: u8,
    pub tile_id: String,
    pub atlas_rect: [u32; 4],
    pub atlas_source: String,
    pub source_status: String,
    #[serde(default)]
    pub render_order: i32,
}

impl WorldPaintRenderCacheEntry {
    pub fn from_resolution(resolution: &WorldPaintTransitionTileResolution) -> Self {
        Self {
            scene_id: resolution.scene_id.clone(),
            x: resolution.x,
            y: resolution.y,
            family: resolution.family.clone(),
            layer: resolution.layer.clone(),
            transition_kind: resolution.transition_kind.clone(),
            neighbor_bits: resolution.neighbor_bits,
            tile_id: resolution.selected_tile_id.clone(),
            atlas_rect: resolution.selected_atlas_rect,
            atlas_source: resolution.atlas_source.clone(),
            source_status: resolution.status.clone(),
            render_order: world_paint_layer_render_order(&resolution.layer),
        }
    }

    pub fn to_resolution(&self) -> WorldPaintTransitionTileResolution {
        WorldPaintTransitionTileResolution {
            schema: crate::WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
            scene_id: self.scene_id.clone(),
            x: self.x,
            y: self.y,
            family: self.family.clone(),
            layer: self.layer.clone(),
            neighbor_bits: self.neighbor_bits,
            transition_kind: self.transition_kind.clone(),
            selected_tile_id: self.tile_id.clone(),
            selected_atlas_rect: self.atlas_rect,
            fallback_tile_id: String::new(),
            atlas_source: self.atlas_source.clone(),
            status: self.source_status.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintRenderCacheScene {
    pub scene_id: String,
    pub entries: Vec<WorldPaintRenderCacheEntry>,
    pub entry_count: usize,
    pub status: String,
}

impl WorldPaintRenderCacheScene {
    pub fn from_details(details: &WorldPaintTransitionSceneTileDetails) -> Self {
        let mut entries = details
            .resolutions
            .iter()
            .filter(|resolution| resolution.selected_tile_id != "none")
            .map(WorldPaintRenderCacheEntry::from_resolution)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            (
                entry.y,
                entry.x,
                entry.render_order,
                entry.layer.clone(),
                entry.tile_id.clone(),
            )
        });
        entries.dedup_by(|a, b| {
            a.scene_id == b.scene_id
                && a.x == b.x
                && a.y == b.y
                && a.layer == b.layer
                && a.tile_id == b.tile_id
        });
        Self {
            scene_id: details.scene_id.clone(),
            entry_count: entries.len(),
            status: format!(
                "Render cache scene {}: {} atlas binding(s)",
                details.scene_id,
                entries.len()
            ),
            entries,
        }
    }

    pub fn to_details(&self) -> WorldPaintTransitionSceneTileDetails {
        WorldPaintTransitionSceneTileDetails {
            schema: crate::WORLD_PAINT_TRANSITION_TILE_RESOLVER_SCHEMA.to_string(),
            scene_id: self.scene_id.clone(),
            resolutions: self
                .entries
                .iter()
                .map(WorldPaintRenderCacheEntry::to_resolution)
                .collect(),
            status: self.status.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldPaintRenderCacheDocument {
    pub schema: String,
    pub version: u32,
    pub source_material_state_path: String,
    pub source_atlas_manifest_path: String,
    pub scenes: Vec<WorldPaintRenderCacheScene>,
    pub status: String,
}

impl Default for WorldPaintRenderCacheDocument {
    fn default() -> Self {
        Self {
            schema: WORLD_PAINT_RENDER_CACHE_SCHEMA.to_string(),
            version: 1,
            source_material_state_path: String::new(),
            source_atlas_manifest_path: crate::WORLD_TILE_ATLAS_MANIFEST_PATH.to_string(),
            scenes: Vec::new(),
            status: "World paint render cache empty".to_string(),
        }
    }
}

impl WorldPaintRenderCacheDocument {
    pub fn scene(&self, scene_id: &str) -> Option<&WorldPaintRenderCacheScene> {
        self.scenes.iter().find(|scene| scene.scene_id == scene_id)
    }

    pub fn upsert_scene(&mut self, scene: WorldPaintRenderCacheScene) {
        if let Some(existing) = self
            .scenes
            .iter_mut()
            .find(|existing| existing.scene_id == scene.scene_id)
        {
            *existing = scene;
        } else {
            self.scenes.push(scene);
        }
        self.scenes.sort_by(|a, b| a.scene_id.cmp(&b.scene_id));
        self.status = format!(
            "World paint render cache: {} scene(s), {} binding(s)",
            self.scenes.len(),
            self.scenes
                .iter()
                .map(|scene| scene.entry_count)
                .sum::<usize>()
        );
    }
}

pub fn load_world_paint_render_cache_document(
    path: impl AsRef<Path>,
) -> io::Result<WorldPaintRenderCacheDocument> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(WorldPaintRenderCacheDocument::default());
    }
    let raw = fs::read_to_string(path)?;
    let doc = serde_json::from_str::<WorldPaintRenderCacheDocument>(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(doc)
}

pub fn save_world_paint_render_cache_document(
    path: impl AsRef<Path>,
    document: &WorldPaintRenderCacheDocument,
) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(document)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, raw)
}

pub fn refresh_world_paint_render_cache_scene(
    repo_root: impl AsRef<Path>,
    material_state_path: impl AsRef<Path>,
    cache_path: impl AsRef<Path>,
    scene_id: &str,
) -> io::Result<WorldPaintRenderCacheScene> {
    let details = resolve_world_paint_scene_transition_tile_details(
        repo_root,
        material_state_path.as_ref(),
        scene_id,
    )?;
    let scene = WorldPaintRenderCacheScene::from_details(&details);
    let mut document = load_world_paint_render_cache_document(cache_path.as_ref())?;
    document.source_material_state_path =
        material_state_path.as_ref().to_string_lossy().to_string();
    document.source_atlas_manifest_path = crate::WORLD_TILE_ATLAS_MANIFEST_PATH.to_string();
    document.upsert_scene(scene.clone());
    save_world_paint_render_cache_document(cache_path, &document)?;
    Ok(scene)
}

pub fn load_world_paint_render_cache_scene(
    cache_path: impl AsRef<Path>,
    scene_id: &str,
) -> io::Result<Option<WorldPaintRenderCacheScene>> {
    let document = load_world_paint_render_cache_document(cache_path)?;
    Ok(document.scene(scene_id).cloned())
}

pub fn validate_world_paint_render_cache_document(
    document: &WorldPaintRenderCacheDocument,
) -> Vec<String> {
    let mut issues = Vec::new();
    if document.schema != WORLD_PAINT_RENDER_CACHE_SCHEMA {
        issues.push(format!("Unexpected schema: {}", document.schema));
    }
    let mut seen = BTreeMap::<(String, i32, i32, String, String), usize>::new();
    for scene in &document.scenes {
        if scene.entry_count != scene.entries.len() {
            issues.push(format!(
                "Scene {} entry_count {} does not match entries {}",
                scene.scene_id,
                scene.entry_count,
                scene.entries.len()
            ));
        }
        for entry in &scene.entries {
            if entry.scene_id != scene.scene_id {
                issues.push(format!(
                    "Entry {},{} has scene_id {} inside scene {}",
                    entry.x, entry.y, entry.scene_id, scene.scene_id
                ));
            }
            if entry.atlas_rect[2] != 32 || entry.atlas_rect[3] != 32 {
                issues.push(format!(
                    "Entry {} {},{} has non-32x32 atlas rect {:?}",
                    entry.scene_id, entry.x, entry.y, entry.atlas_rect
                ));
            }
            if entry.render_order != world_paint_layer_render_order(&entry.layer) {
                issues.push(format!(
                    "Entry {} {},{} layer {} has render_order {}, expected {}",
                    entry.scene_id,
                    entry.x,
                    entry.y,
                    entry.layer,
                    entry.render_order,
                    world_paint_layer_render_order(&entry.layer)
                ));
            }
            *seen
                .entry((
                    entry.scene_id.clone(),
                    entry.x,
                    entry.y,
                    entry.layer.clone(),
                    entry.tile_id.clone(),
                ))
                .or_insert(0) += 1;
        }
    }
    for ((scene_id, x, y, layer, tile_id), count) in seen {
        if count > 1 {
            issues.push(format!(
                "Duplicate render cache entry {scene_id} {x},{y} {layer} {tile_id}: {count}"
            ));
        }
    }
    issues
}

pub fn world_paint_layer_render_order(layer: &str) -> i32 {
    match layer {
        "ground_base" => 10,
        "ground_variation" => 20,
        "ground_transition_fringe" => 30,
        "water_base" => 40,
        "water_surface_fx" => 50,
        "cave_base" => 60,
        "cave_wall_face" => 70,
        "town_surface" => 80,
        "indoor_floor" => 90,
        "debris_overlay" => 100,
        "collision_footprint" => 900,
        "occlusion_fade_mask" => 910,
        "dev_overlay" => 920,
        _ => 500,
    }
}
