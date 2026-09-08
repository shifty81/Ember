//! Compatibility bridge from legacy exterior scene documents to the continuous
//! Havenwild world surface.
//!
//! Exterior maps remain loadable as authored chunk templates while the runtime
//! migrates to one seamless coordinate space. Interiors, caves, dungeons, and
//! other special areas continue to use explicit scene transitions.

use haven_core::{GameWorld, ProjectSceneId, SceneId, SceneKind, SceneMap, TileKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::generated_surface_chunks::generated_chunk_scene_id;
use crate::open_world::{ChunkCoord, WorldTileCoord};

pub const CONTINUOUS_SURFACE_SCHEMA: &str = "havenwild.continuous_surface.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceTileAddress {
    pub chunk: ChunkCoord,
    pub local_x: i32,
    pub local_y: i32,
}

impl SurfaceTileAddress {
    pub const fn local_tile(self) -> WorldTileCoord {
        WorldTileCoord::new(self.local_x, self.local_y)
    }
}

/// Splits one global surface tile into its storage partition and partition-local
/// cell. Euclidean division keeps negative coordinates stable: global -1 maps
/// to the final cell of chunk -1 rather than producing an invalid local cell.
pub fn surface_tile_address(global: WorldTileCoord) -> SurfaceTileAddress {
    let width = MAP_W as i32;
    let height = MAP_H as i32;
    SurfaceTileAddress {
        chunk: ChunkCoord::new(global.x.div_euclid(width), global.y.div_euclid(height)),
        local_x: global.x.rem_euclid(width),
        local_y: global.y.rem_euclid(height),
    }
}

pub fn surface_global_tile(chunk: ChunkCoord, local_x: i32, local_y: i32) -> WorldTileCoord {
    WorldTileCoord::new(
        chunk.x * MAP_W as i32 + local_x,
        chunk.y * MAP_H as i32 + local_y,
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExteriorChunkBinding {
    pub scene_id: ProjectSceneId,
    pub chunk: ChunkCoord,
    pub authored_override: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceRuntimeState {
    pub manifest: ContinuousSurfaceManifest,
    pub active_chunk: ChunkCoord,
    pub global_tile: WorldTileCoord,
}

impl SurfaceRuntimeState {
    pub fn starter(active_scene: &ProjectSceneId, local_tile: WorldTileCoord) -> Self {
        let manifest = ContinuousSurfaceManifest::legacy_starter_bridge();
        Self::from_manifest(manifest, active_scene, local_tile)
    }

    /// Builds surface navigation from the exterior partitions actually loaded
    /// in the current world. PCG island rectangles are storage partitions of
    /// one surface region, not independent player-facing scenes.
    pub fn for_world(world: &GameWorld, local_tile: WorldTileCoord) -> Self {
        let manifest = ContinuousSurfaceManifest::for_world(world);
        Self::from_manifest(manifest, world.active_scene.project_id(), local_tile)
    }

    fn from_manifest(
        manifest: ContinuousSurfaceManifest,
        active_scene: &ProjectSceneId,
        local_tile: WorldTileCoord,
    ) -> Self {
        let active_chunk = manifest
            .chunk_for_scene(active_scene)
            .unwrap_or(ChunkCoord::new(0, 0));
        let global_tile = WorldTileCoord::new(
            active_chunk.x * MAP_W as i32 + local_tile.x,
            active_chunk.y * MAP_H as i32 + local_tile.y,
        );
        Self {
            manifest,
            active_chunk,
            global_tile,
        }
    }

    pub fn binding_for_chunk(&self, chunk: ChunkCoord) -> Option<&ExteriorChunkBinding> {
        self.manifest
            .exterior_bindings
            .iter()
            .find(|binding| binding.chunk == chunk)
    }

    pub fn active_window(&self) -> Vec<ChunkCoord> {
        chunk_window(self.active_chunk, self.manifest.active_radius_chunks)
    }

    pub fn preload_window(&self) -> Vec<ChunkCoord> {
        chunk_window(self.active_chunk, self.manifest.preload_radius_chunks)
    }

    pub fn update_from_local_tile(&mut self, local_tile: WorldTileCoord) {
        self.global_tile = WorldTileCoord::new(
            self.active_chunk.x * MAP_W as i32 + local_tile.x,
            self.active_chunk.y * MAP_H as i32 + local_tile.y,
        );
    }

    pub fn move_to_chunk(&mut self, chunk: ChunkCoord, local_tile: WorldTileCoord) {
        self.active_chunk = chunk;
        self.update_from_local_tile(local_tile);
    }
}

pub fn chunk_window(center: ChunkCoord, radius: i32) -> Vec<ChunkCoord> {
    let radius = radius.max(0);
    let mut chunks = Vec::new();
    for y in center.y - radius..=center.y + radius {
        for x in center.x - radius..=center.x + radius {
            chunks.push(ChunkCoord::new(x, y));
        }
    }
    chunks
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuousSurfaceManifest {
    pub schema: String,
    pub chunk_size_tiles: i32,
    pub active_radius_chunks: i32,
    pub preload_radius_chunks: i32,
    /// Active PCG landmass namespace. Missing coordinates inside this namespace
    /// stream from the same deterministic geographic field as the starter
    /// rectangles; storage partitions never define geography themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pcg_region: Option<String>,
    pub exterior_bindings: Vec<ExteriorChunkBinding>,
}

impl ContinuousSurfaceManifest {
    pub fn legacy_starter_bridge() -> Self {
        Self {
            schema: CONTINUOUS_SURFACE_SCHEMA.to_string(),
            chunk_size_tiles: 64,
            active_radius_chunks: 1,
            preload_radius_chunks: 2,
            pcg_region: None,
            exterior_bindings: vec![
                binding(SceneId::Farmstead, 0, 0),
                binding(SceneId::NorthRoad, 0, -1),
                binding(SceneId::SouthField, 0, 1),
                binding(SceneId::EastWoods, 1, 0),
            ],
        }
    }

    /// Creates the active continuous-surface partition registry from loaded
    /// world data. The active PCG landmass selects its own region so different
    /// islands may reuse local chunk coordinates without colliding.
    pub fn for_world(world: &GameWorld) -> Self {
        let active_pcg_region =
            parse_pcg_surface_scene_id(world.active_scene.project_id()).map(|(region, _)| region);
        let mut bindings = std::collections::BTreeMap::new();

        if active_pcg_region.is_none() {
            for binding in Self::legacy_starter_bridge().exterior_bindings {
                if world.scene_by_id(&binding.scene_id).is_some() {
                    bindings.insert(binding.chunk, binding);
                }
            }
        }

        for scene in &world.scenes {
            if scene.kind != SceneKind::Exterior {
                continue;
            }
            if let Some(chunk) = parse_generated_chunk_scene_id(&scene.id) {
                // Generic generated terrain belongs only to the non-PCG surface
                // lane. A PCG mainland keeps its own namespace while streamed
                // coordinates sample the same global geographic authority.
                if active_pcg_region.is_none() {
                    bindings
                        .entry(chunk)
                        .or_insert_with(|| ExteriorChunkBinding {
                            scene_id: scene.id.clone(),
                            chunk,
                            authored_override: false,
                        });
                }
                continue;
            }
            let Some((region, chunk)) = parse_pcg_surface_scene_id(&scene.id) else {
                continue;
            };
            // PCG partitions are a separate continuous-surface namespace. They
            // must never replace the authored Farmstead/NorthRoad/SouthField/
            // EastWoods bridge merely because a dormant PCG partition happens
            // to use the same chunk coordinate. This was the cause of Play
            // Farmstead using Farmstead collision/interactions while rendering
            // a PCG island at chunk 0,0.
            let Some(active_region) = active_pcg_region.as_deref() else {
                continue;
            };
            if active_region != region.as_str() {
                continue;
            }
            bindings.insert(
                chunk,
                ExteriorChunkBinding {
                    scene_id: scene.id.clone(),
                    chunk,
                    authored_override: true,
                },
            );
        }

        Self {
            schema: CONTINUOUS_SURFACE_SCHEMA.to_string(),
            chunk_size_tiles: MAP_W as i32,
            active_radius_chunks: 1,
            preload_radius_chunks: 2,
            pcg_region: active_pcg_region,
            exterior_bindings: bindings.into_values().collect(),
        }
    }

    pub fn binding_for_chunk(&self, chunk: ChunkCoord) -> Option<&ExteriorChunkBinding> {
        self.exterior_bindings
            .iter()
            .find(|binding| binding.chunk == chunk)
    }

    pub fn binding_for_scene(&self, scene_id: &ProjectSceneId) -> Option<&ExteriorChunkBinding> {
        self.exterior_bindings
            .iter()
            .find(|binding| &binding.scene_id == scene_id)
    }

    pub fn chunk_for_scene(&self, scene_id: &ProjectSceneId) -> Option<ChunkCoord> {
        self.binding_for_scene(scene_id)
            .map(|binding| binding.chunk)
            .or_else(|| parse_generated_chunk_scene_id(scene_id))
            .or_else(|| parse_pcg_surface_scene_id(scene_id).map(|(_, chunk)| chunk))
    }

    pub fn scene_id_for_chunk(&self, chunk: ChunkCoord) -> ProjectSceneId {
        self.exterior_bindings
            .iter()
            .find(|binding| binding.chunk == chunk)
            .map(|binding| binding.scene_id.clone())
            .unwrap_or_else(|| {
                self.pcg_region
                    .as_deref()
                    .map(|region| pcg_surface_scene_id(region, chunk))
                    .unwrap_or_else(|| generated_chunk_scene_id(chunk))
            })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CONTINUOUS_SURFACE_SCHEMA {
            return Err(format!(
                "unsupported continuous surface schema {}",
                self.schema
            ));
        }
        if self.chunk_size_tiles <= 0
            || self.active_radius_chunks < 0
            || self.preload_radius_chunks < self.active_radius_chunks
        {
            return Err("continuous surface radii or chunk size are invalid".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PcgSurfaceMigrationReport {
    pub marine_tiles_reclassified: usize,
    pub exterior_transitions_removed: usize,
}

impl PcgSurfaceMigrationReport {
    pub const fn total_mutations(self) -> usize {
        self.marine_tiles_reclassified + self.exterior_transitions_removed
    }
}

/// Removes obsolete edge-transfer triggers from PCG exterior partitions. These
/// partitions are storage chunks of one world surface; explicit transitions are
/// reserved for interiors, caves, ruins, dungeons, and special instances.
pub fn strip_pcg_exterior_transitions(world: &mut GameWorld) -> usize {
    let mut removed = 0usize;
    for scene in world.scenes.iter_mut() {
        if scene.kind != SceneKind::Exterior || parse_pcg_surface_scene_id(&scene.id).is_none() {
            continue;
        }
        removed += scene.transitions.len();
        scene.transitions.clear();
    }
    removed
}

/// Migrates pre-Z77 PCG coast water without regenerating player-authored land.
/// Only water connected to an open partition boundary is marine; enclosed
/// ponds remain freshwater. Marine cells within two tiles of land become
/// `OceanShallow`, while the connected interior becomes `OceanDeep`.
pub fn migrate_legacy_pcg_ocean_domains(world: &mut GameWorld) -> PcgSurfaceMigrationReport {
    use std::collections::{BTreeMap, BTreeSet, VecDeque};

    let exterior_transitions_removed = strip_pcg_exterior_transitions(world);
    let mut regions: BTreeMap<String, Vec<(ProjectSceneId, ChunkCoord)>> = BTreeMap::new();
    for scene in &world.scenes {
        if scene.kind != SceneKind::Exterior {
            continue;
        }
        if let Some((region, chunk)) = parse_pcg_surface_scene_id(&scene.id) {
            regions
                .entry(region)
                .or_default()
                .push((scene.id.clone(), chunk));
        }
    }

    type LocalTileChange = (i32, i32, TileKind);
    type GlobalCell = (ProjectSceneId, i32, i32, TileKind);

    let mut changes: BTreeMap<ProjectSceneId, Vec<LocalTileChange>> = BTreeMap::new();
    for partitions in regions.into_values() {
        let mut cells: BTreeMap<(i32, i32), GlobalCell> = BTreeMap::new();
        for (scene_id, chunk) in &partitions {
            let Some(scene) = world.scene_by_id(scene_id) else {
                continue;
            };
            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    let global = (chunk.x * MAP_W as i32 + x, chunk.y * MAP_H as i32 + y);
                    cells.insert(global, (scene_id.clone(), x, y, scene.map.get(x, y)));
                }
            }
        }

        let eligible_water = |tile: TileKind| {
            matches!(
                tile,
                TileKind::Water
                    | TileKind::ShallowWater
                    | TileKind::DeepWater
                    | TileKind::OceanShallow
                    | TileKind::OceanDeep
            )
        };
        let mut marine = BTreeSet::new();
        let mut queue = VecDeque::new();
        for (&coord, (_, _, _, tile)) in &cells {
            if !eligible_water(*tile) {
                continue;
            }
            let touches_open_partition = [(0, -1), (1, 0), (0, 1), (-1, 0)]
                .iter()
                .any(|(dx, dy)| !cells.contains_key(&(coord.0 + dx, coord.1 + dy)));
            if touches_open_partition {
                marine.insert(coord);
                queue.push_back(coord);
            }
        }
        while let Some((x, y)) = queue.pop_front() {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let next = (x + dx, y + dy);
                if marine.contains(&next) {
                    continue;
                }
                if cells
                    .get(&next)
                    .is_some_and(|(_, _, _, tile)| eligible_water(*tile))
                {
                    marine.insert(next);
                    queue.push_back(next);
                }
            }
        }

        for (global_x, global_y) in marine {
            let Some((scene_id, local_x, local_y, current)) = cells.get(&(global_x, global_y))
            else {
                continue;
            };
            let near_land = (-2..=2).any(|dy| {
                (-2..=2).any(|dx| {
                    (dx != 0 || dy != 0)
                        && cells
                            .get(&(global_x + dx, global_y + dy))
                            .is_some_and(|(_, _, _, tile)| !tile.is_water())
                })
            });
            let target = if near_land {
                TileKind::OceanShallow
            } else {
                TileKind::OceanDeep
            };
            if *current != target {
                changes
                    .entry(scene_id.clone())
                    .or_default()
                    .push((*local_x, *local_y, target));
            }
        }
    }

    let marine_tiles_reclassified = changes.values().map(Vec::len).sum();
    for (scene_id, scene_changes) in changes {
        let Some(scene) = world.scene_mut_by_id(&scene_id) else {
            continue;
        };
        for (x, y, tile) in scene_changes {
            scene.map.set(x, y, tile);
        }
    }

    PcgSurfaceMigrationReport {
        marine_tiles_reclassified,
        exterior_transitions_removed,
    }
}

pub fn scene_is_surface_chunk(scene: &SceneMap) -> bool {
    scene.kind == SceneKind::Exterior
}

pub fn transition_uses_surface_streaming(source: &SceneMap, target: &SceneMap) -> bool {
    scene_is_surface_chunk(source) && scene_is_surface_chunk(target)
}

/// Recognizes an exterior storage-partition identity even when the target is
/// not loaded yet. Historical PCG edge transitions must remain inert instead
/// of falling back to a player-facing scene transfer while streaming catches up.
pub fn scene_id_is_surface_partition(scene_id: &ProjectSceneId) -> bool {
    parse_pcg_surface_scene_id(scene_id).is_some()
        || parse_generated_chunk_scene_id(scene_id).is_some()
        || scene_id.legacy_scene_id().is_some_and(|legacy| {
            matches!(
                legacy,
                SceneId::Farmstead | SceneId::NorthRoad | SceneId::SouthField | SceneId::EastWoods
            )
        })
}

/// Parses the historical PCG rectangle identity from the right so landmass
/// slugs may contain underscores. These rectangles are open-world surface
/// partitions and use their trailing grid coordinates as chunk coordinates.
pub fn parse_pcg_surface_scene_id(scene_id: &ProjectSceneId) -> Option<(String, ChunkCoord)> {
    let rest = scene_id.code().strip_prefix("pcg_")?;
    let mut parts = rest.rsplitn(3, '_');
    let y = parse_surface_grid_token(parts.next()?)?;
    let x = parse_surface_grid_token(parts.next()?)?;
    let region = parts.next()?.trim();
    if region.is_empty() {
        return None;
    }
    Some((region.to_string(), ChunkCoord::new(x, y)))
}

/// Builds the normalization-safe identity used by every authored or synthesized
/// partition belonging to one PCG landmass namespace.
pub fn pcg_surface_scene_id(region: &str, chunk: ChunkCoord) -> ProjectSceneId {
    ProjectSceneId::new(format!(
        "pcg_{}_{}_{}",
        region.trim_matches('_'),
        encode_pcg_grid_token(chunk.x),
        encode_pcg_grid_token(chunk.y)
    ))
}

/// Encodes signed PCG partition coordinates without using `-`, because
/// `ProjectSceneId` normalizes hyphens into separators. Non-negative values
/// retain the historical decimal representation; negative values use `n#`.
pub(crate) fn encode_pcg_grid_token(value: i32) -> String {
    if value < 0 {
        format!("n{}", i64::from(value).abs())
    } else {
        value.to_string()
    }
}

/// Decodes the normalization-safe coordinate tokens shared by PCG and
/// generated exterior storage partitions. Plain decimal non-negative tokens
/// remain accepted for historical generated IDs.
fn parse_surface_grid_token(token: &str) -> Option<i32> {
    if let Some(digits) = token.strip_prefix('n') {
        let magnitude = digits.parse::<u32>().ok()?;
        return i32::try_from(-i64::from(magnitude)).ok();
    }
    let digits = token.strip_prefix('p').unwrap_or(token);
    let value = digits.parse::<u32>().ok()?;
    i32::try_from(value).ok()
}

pub fn parse_generated_chunk_scene_id(scene_id: &ProjectSceneId) -> Option<ChunkCoord> {
    let code = scene_id.code();
    let rest = code.strip_prefix("surface_x_")?;
    let (x, y) = rest.split_once("_y_")?;
    Some(ChunkCoord::new(
        parse_surface_grid_token(x)?,
        parse_surface_grid_token(y)?,
    ))
}

fn binding(scene: SceneId, x: i32, y: i32) -> ExteriorChunkBinding {
    ExteriorChunkBinding {
        scene_id: ProjectSceneId::from(scene),
        chunk: ChunkCoord::new(x, y),
        authored_override: true,
    }
}

#[cfg(test)]
#[path = "continuous_surface_tests.rs"]
mod tests;
