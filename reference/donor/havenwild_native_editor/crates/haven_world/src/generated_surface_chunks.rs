//! Deterministic on-demand exterior chunk generation and residency contracts.
//!
//! Pass 152A keeps continuous geology as world metadata while restoring V7
//! authority over horizontal terrain transitions. Surface cells remain their
//! semantic material; cliffs are structural elevation edges and must never
//! replace grass, sand, or water cells before tuple resolution. Roads,
//! structures, vegetation, and authored details remain separate later passes.

use std::collections::{BTreeMap, BTreeSet};

use haven_core::{ProjectSceneId, SceneBiome, SceneKind, SceneMap, TileKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{
    continuous_surface::pcg_surface_scene_id,
    geographic_hydrology::{
        drainage_features_for_bounds, sample_geographic_hydrology_from_features, DrainageFeature,
        GeographicHydrologySample, GeographicWaterFeatureKind,
    },
    geographic_surface::{sample_geographic_surface, GeographicGenerationProfile, GeographicSurfaceSample},
    open_world::ChunkCoord,
    resolve_entire_tavern_map_hydrology_v2, resolve_tavern_map_elevation_cliffs_v2,
    ElevationCliffSettingsV2, HydrologySettingsV2,
};

pub const GENERATED_SURFACE_CHUNK_SCHEMA: &str = "havenwild.generated_surface_chunk.v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChunkResidencyState {
    Metadata,
    Preloaded,
    Active,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceChunkRecord {
    pub schema: String,
    pub chunk: ChunkCoord,
    pub scene_id: ProjectSceneId,
    pub generated: bool,
    pub authored_override: bool,
    pub seed: u64,
    pub residency: ChunkResidencyState,
}

#[derive(Clone, Debug, Default)]
pub struct SurfaceResidencyWindow {
    pub active: BTreeSet<(i32, i32)>,
    pub preload: BTreeSet<(i32, i32)>,
    pub records: BTreeMap<(i32, i32), SurfaceChunkRecord>,
}

impl SurfaceResidencyWindow {
    pub fn refresh(center: ChunkCoord, active_radius: i32, preload_radius: i32, seed: u64) -> Self {
        let mut window = Self::default();
        for y in center.y - preload_radius..=center.y + preload_radius {
            for x in center.x - preload_radius..=center.x + preload_radius {
                let coord = ChunkCoord::new(x, y);
                let key = (x, y);
                let residency = if (x - center.x).abs() <= active_radius
                    && (y - center.y).abs() <= active_radius
                {
                    window.active.insert(key);
                    ChunkResidencyState::Active
                } else {
                    ChunkResidencyState::Preloaded
                };
                window.preload.insert(key);
                window.records.insert(
                    key,
                    SurfaceChunkRecord {
                        schema: GENERATED_SURFACE_CHUNK_SCHEMA.to_string(),
                        chunk: coord,
                        scene_id: generated_chunk_scene_id(coord),
                        generated: true,
                        authored_override: false,
                        seed: chunk_seed(seed, coord),
                        residency,
                    },
                );
            }
        }
        window
    }
}

pub fn generated_chunk_scene_id(chunk: ChunkCoord) -> ProjectSceneId {
    ProjectSceneId::new(format!(
        "surface_x_{}_y_{}",
        signed_token(chunk.x),
        signed_token(chunk.y)
    ))
}

/// Deterministic open-ocean storage partition used outside an authored PCG
/// island assembly. Missing PCG rectangles represent ocean, never a generic
/// grass-bearing fallback chunk.
pub fn generate_open_ocean_surface_chunk(chunk: ChunkCoord) -> SceneMap {
    generate_open_ocean_partition(generated_chunk_scene_id(chunk), chunk)
}

/// Open-ocean partition that remains inside the active PCG landmass namespace.
/// This prevents an island edge from falling into the unrelated generic surface
/// generator or colliding with another island that reuses local grid coordinates.
pub fn generate_open_ocean_pcg_partition(region: &str, chunk: ChunkCoord) -> SceneMap {
    generate_open_ocean_partition(pcg_surface_scene_id(region, chunk), chunk)
}

fn generate_open_ocean_partition(scene_id: ProjectSceneId, chunk: ChunkCoord) -> SceneMap {
    let mut scene = SceneMap::blank(
        scene_id,
        format!("Open Ocean {},{}", chunk.x, chunk.y),
        SceneKind::Exterior,
        SceneBiome::Coastal,
    );
    scene.transitions.clear();
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            scene.map.set(x, y, TileKind::OceanDeep);
            scene.map.set_height(x, y, 18);
        }
    }
    scene
}

pub fn generate_surface_chunk(world_seed: u64, chunk: ChunkCoord) -> SceneMap {
    generate_surface_chunk_with_profile(world_seed, chunk, GeographicGenerationProfile::default())
}

pub fn generate_surface_chunk_with_profile(
    world_seed: u64,
    chunk: ChunkCoord,
    profile: GeographicGenerationProfile,
) -> SceneMap {
    generate_surface_partition(
        generated_chunk_scene_id(chunk),
        world_seed,
        chunk,
        profile,
    )
}

/// Generate a streamed partition inside the same PCG namespace as the initial
/// Willowmere/mainland anchor.  Missing rectangles are no longer synthesized
/// as endless open ocean: they sample the same global geography as every other
/// streamed partition, so the mainland can continue for thousands of tiles or
/// indefinitely when the world extent is Endless.
pub fn generate_surface_pcg_partition_with_profile(
    world_seed: u64,
    region: &str,
    chunk: ChunkCoord,
    profile: GeographicGenerationProfile,
) -> SceneMap {
    generate_surface_partition(
        pcg_surface_scene_id(region, chunk),
        world_seed,
        chunk,
        profile,
    )
}

fn generate_surface_partition(
    scene_id: ProjectSceneId,
    world_seed: u64,
    chunk: ChunkCoord,
    profile: GeographicGenerationProfile,
) -> SceneMap {
    let center_x = chunk.x * MAP_W as i32 + MAP_W as i32 / 2;
    let center_y = chunk.y * MAP_H as i32 + MAP_H as i32 / 2;
    let center = sample_geographic_surface(world_seed, center_x, center_y, profile);
    let mut scene = SceneMap::blank(
        scene_id,
        format!("Surface {},{}", chunk.x, chunk.y),
        SceneKind::Exterior,
        center.biome,
    );
    scene.transitions.clear();

    let min_x = chunk.x * MAP_W as i32;
    let min_y = chunk.y * MAP_H as i32;
    let max_x = min_x + MAP_W as i32 - 1;
    let max_y = min_y + MAP_H as i32 - 1;
    let drainage_features =
        drainage_features_for_bounds(world_seed, min_x, min_y, max_x, max_y, profile);

    let mut samples = vec![TerrainSample::default(); MAP_W * MAP_H];
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let gx = chunk.x * MAP_W as i32 + x;
            let gy = chunk.y * MAP_H as i32 + y;
            samples[y as usize * MAP_W + x as usize] =
                terrain_sample(world_seed, gx, gy, profile, &drainage_features);
        }
    }

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let index = y as usize * MAP_W + x as usize;
            let sample = samples[index];
            let tile = classify_surface_tile(sample);
            scene.map.set(x, y, tile);
            scene.map.set_height(x, y, sample.height);

            // Ocean and the narrow beach are always Level 0.  Freshwater that
            // belongs to a generated drainage feature deliberately retains the
            // structural level beneath it: a RiverWater path crossing Level
            // 2->1 or 1->0 is exactly what makes the structural resolver emit
            // the waterfall connector and blocking cliff around it.
            let structural_level = if matches!(
                tile,
                TileKind::OceanDeep
                    | TileKind::OceanShallow
                    | TileKind::Sand
                    | TileKind::WetSand
                    | TileKind::MudBank
            ) {
                0
            } else {
                sample.surface.structural_level
            };
            scene
                .map
                .set_structural_level(x, y, Some(structural_level));
        }
    }

    let _hydrology =
        resolve_entire_tavern_map_hydrology_v2(&mut scene.map, HydrologySettingsV2::default());
    let _structure =
        resolve_tavern_map_elevation_cliffs_v2(&scene.map, ElevationCliffSettingsV2::default());

    scene
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerrainSample {
    surface: GeographicSurfaceSample,
    hydrology: GeographicHydrologySample,
    height: u8,
}

impl Default for TerrainSample {
    fn default() -> Self {
        Self {
            surface: GeographicSurfaceSample {
                coast_distance_tiles: 0.0,
                land: true,
                shoreline: false,
                shallow_ocean: false,
                structural_level: 0,
                elevation: 0.5,
                moisture: 0.5,
                biome: SceneBiome::Temperate,
            },
            hydrology: GeographicHydrologySample::default(),
            height: 128,
        }
    }
}

fn terrain_sample(
    world_seed: u64,
    gx: i32,
    gy: i32,
    profile: GeographicGenerationProfile,
    drainage_features: &[DrainageFeature],
) -> TerrainSample {
    let surface = sample_geographic_surface(world_seed, gx, gy, profile);
    let hydrology = sample_geographic_hydrology_from_features(
        world_seed,
        gx,
        gy,
        profile,
        drainage_features,
    );
    let mut height = (f64::from(surface.elevation) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8;
    if surface.structural_level == 1 {
        height = height.max(150);
    } else if surface.structural_level >= 2 {
        height = height.max(178);
    }
    TerrainSample {
        surface,
        hydrology,
        height,
    }
}

fn classify_surface_tile(sample: TerrainSample) -> TileKind {
    match sample.hydrology.kind {
        GeographicWaterFeatureKind::SourcePool => return TileKind::ShallowWater,
        GeographicWaterFeatureKind::River => return TileKind::RiverWater,
        GeographicWaterFeatureKind::SinkPond | GeographicWaterFeatureKind::SinkLake => {
            return TileKind::ShallowWater;
        }
        GeographicWaterFeatureKind::RiverMouth => return TileKind::RiverMouthBlend,
        GeographicWaterFeatureKind::None => {}
    }

    if !sample.surface.land {
        return if sample.surface.shallow_ocean {
            TileKind::OceanShallow
        } else {
            TileKind::OceanDeep
        };
    }
    if sample.surface.shoreline {
        return TileKind::Sand;
    }
    if sample.surface.structural_level >= 2 {
        return TileKind::MountainRock;
    }
    if sample.surface.moisture > 0.78 && sample.surface.structural_level == 0 {
        return TileKind::TallGrass;
    }
    TileKind::Grass
}

fn signed_token(value: i32) -> String {
    if value < 0 {
        format!("n{}", value.unsigned_abs())
    } else {
        format!("p{value}")
    }
}

fn chunk_seed(world_seed: u64, chunk: ChunkCoord) -> u64 {
    hash2(world_seed, chunk.x, chunk.y)
}

fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let mut value = seed ^ (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= (y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value.wrapping_mul(0x94d0_49bb_1331_11eb) ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_chunks_are_stable_and_transition_free() {
        let coord = ChunkCoord::new(4, -2);
        let a = generate_surface_chunk(42, coord);
        let b = generate_surface_chunk(42, coord);
        assert_eq!(a.id, b.id);
        assert_eq!(a.map.tiles, b.map.tiles);
        assert_eq!(a.map.heights, b.map.heights);
        assert!(a.transitions.is_empty());
    }

    #[test]
    fn residency_window_is_three_by_three_inside_five_by_five() {
        let window = SurfaceResidencyWindow::refresh(ChunkCoord::new(0, 0), 1, 2, 7);
        assert_eq!(window.active.len(), 9);
        assert_eq!(window.preload.len(), 25);
    }

    #[test]
    fn geology_is_not_quantized_into_eight_tile_rectangles() {
        let scene = generate_surface_chunk(91, ChunkCoord::new(1, 2));
        let mut flat_height_blocks = 0usize;
        let mut sampled_blocks = 0usize;
        for y in (0..MAP_H as i32 - 7).step_by(8) {
            for x in (0..MAP_W as i32 - 7).step_by(8) {
                sampled_blocks += 1;
                let first_height = scene.map.get_height(x, y);
                let height_is_quantized = (y..y + 8)
                    .all(|cy| (x..x + 8).all(|cx| scene.map.get_height(cx, cy) == first_height));
                flat_height_blocks += height_is_quantized as usize;
            }
        }

        // Terrain classes can legitimately remain uniform across broad ocean,
        // grassland, or mountain regions. The geological source field must not
        // repeat one height across old-style 8x8 generation cells.
        assert!(sampled_blocks > 0);
        // Broad geographic plateaus/ocean can legitimately contain completely
        // flat 8x8 samples. The regression we are guarding against is the old
        // generator where the *entire field* was quantized on that grid. Keep
        // this test statistical instead of rejecting valid macro-landforms.
        assert!(
            flat_height_blocks < sampled_blocks / 2,
            "legacy 8x8 quantization appears dominant: {flat_height_blocks}/{sampled_blocks} sampled blocks are flat"
        );
    }

    #[test]
    fn generated_coastline_never_rewrites_surface_as_cliff() {
        for seed in [1_u64, 7, 91, 0x151, 0x5eed] {
            for cy in -2..=2 {
                for cx in -2..=2 {
                    let scene = generate_surface_chunk(seed, ChunkCoord::new(cx, cy));
                    assert!(
                        !scene.map.tiles.contains(&TileKind::Cliff),
                        "surface generator emitted Cliff for seed {seed} at chunk {cx},{cy}"
                    );
                }
            }
        }
    }

    #[test]
    fn generated_surface_preserves_real_marine_water_semantics() {
        let profile = GeographicGenerationProfile::default();
        let mut found_ocean = false;
        let mut found_land = false;
        for cy in 1..=6 {
            for cx in -2..=4 {
                let scene = generate_surface_chunk_with_profile(0x152a, ChunkCoord::new(cx, cy), profile);
                found_ocean |= scene
                    .map
                    .tiles
                    .iter()
                    .any(|tile| matches!(tile, TileKind::OceanDeep | TileKind::OceanShallow));
                found_land |= scene
                    .map
                    .tiles
                    .iter()
                    .any(|tile| matches!(tile, TileKind::Grass | TileKind::TallGrass | TileKind::MountainRock));
            }
        }
        assert!(found_ocean, "expected the starter mainland window to reach marine water");
        assert!(found_land, "expected the starter mainland window to contain land");
    }

    #[test]
    fn generated_surface_contains_real_height_variation() {
        let scene = generate_surface_chunk(0x151, ChunkCoord::new(3, 3));
        let min = *scene.map.heights.iter().min().expect("height minimum");
        let max = *scene.map.heights.iter().max().expect("height maximum");
        assert!(max.saturating_sub(min) >= 12);
    }
    #[test]
    fn open_ocean_fallback_never_contains_generated_land() {
        let scene = generate_open_ocean_pcg_partition("havenwild_mainland", ChunkCoord::new(8, -3));
        assert_eq!(scene.id.code(), "pcg_havenwild_mainland_8_n3");
        assert_eq!(scene.kind, SceneKind::Exterior);
        assert!(scene.transitions.is_empty());
        assert!(scene
            .map
            .tiles
            .iter()
            .all(|tile| *tile == TileKind::OceanDeep));
    }
}
