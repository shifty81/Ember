use std::collections::BTreeSet;

use haven_core::{
    ObjectKind, ProjectSceneId, SceneBiome, SceneKind, SceneMap, TileKind, MAP_H, MAP_W,
};

use crate::{
    generated_surface_chunks::generate_surface_pcg_partition_with_profile,
    geographic_surface::GeographicGenerationProfile,
    highland_generation::promote_global_highland_materials_with_strength,
    island_coastline::{generate_coastline_raster, CoastlineGenerationInput},
    mainland_features::MainlandFeatureReport,
    open_world::ChunkCoord,
    scene_rectangles::{SceneRectangleManifest, SceneRectangleSpec},
    structural_landform_generation::{materialize_structural_landforms, StructuralLandformReport},
    surface_world_plan::{materialize_mainland_world_plan_compatibility, SurfaceWorldPlanV1},
};

/// Island generation settings for the continuous PCG surface.
///
/// Terrain shape remains the first authority. Natural objects are applied afterward
/// as deterministic authored object placements, never baked into or substituted for
/// terrain cells. Roads, resources, buildings, and authored civic sites remain
/// separate later-stage systems.
#[derive(Clone, Copy, Debug)]
pub struct IslandGenerationSettings {
    pub seed: u64,
    pub mountain_radius: f32,
    pub shoreline_width: f32,
    pub tree_density: f32,
    pub geography: GeographicGenerationProfile,
}

impl Default for IslandGenerationSettings {
    fn default() -> Self {
        Self {
            seed: 0x4841_5645_4e57_494c,
            mountain_radius: 0.42,
            shoreline_width: 0.12,
            tree_density: 0.08,
            geography: GeographicGenerationProfile::default(),
        }
    }
}

pub use crate::surface_population::{
    natural_object_density_for_landmass, natural_object_density_for_region,
    populate_missing_pcg_mainland_features, populate_missing_pcg_natural_objects,
    populate_pcg_natural_objects, NaturalObjectPopulationReport,
};

#[derive(Clone, Debug)]
pub struct GeneratedIslandScene {
    pub rectangle_id: String,
    pub scene: SceneMap,
}

#[derive(Clone, Debug)]
pub struct GeneratedIsland {
    pub landmass_id: i32,
    pub landmass_name: String,
    pub harbor_rectangle_id: String,
    pub harbor_scene_id: ProjectSceneId,
    pub scenes: Vec<GeneratedIslandScene>,
    pub land_cells: usize,
    pub mountain_cells: usize,
    pub shoreline_cells: usize,
    pub natural_objects: usize,
    pub mainland_features: MainlandFeatureReport,
    pub structural_landforms: StructuralLandformReport,
    /// Feature-first world planning authority for this generated surface. Z109T
    /// carries the target plan while retaining the legacy visual materializer.
    pub world_plan: SurfaceWorldPlanV1,
}

pub fn generate_landmass(
    manifest: &SceneRectangleManifest,
    landmass_id: i32,
    settings: IslandGenerationSettings,
) -> Result<GeneratedIsland, String> {
    let mut rectangles: Vec<&SceneRectangleSpec> = manifest
        .scene_rectangles
        .iter()
        .filter(|rectangle| {
            rectangle.landmass_id == landmass_id
                && rectangle.grid_x.is_some()
                && rectangle.grid_y.is_some()
        })
        .collect();
    if rectangles.is_empty() {
        return Err(format!(
            "landmass {landmass_id} has no editable scene rectangles"
        ));
    }
    rectangles.sort_by_key(|rectangle| (rectangle.grid_y, rectangle.grid_x));

    let min_x = rectangles
        .iter()
        .filter_map(|entry| entry.grid_x)
        .min()
        .unwrap_or(0);
    let max_x = rectangles
        .iter()
        .filter_map(|entry| entry.grid_x)
        .max()
        .unwrap_or(0);
    let min_y = rectangles
        .iter()
        .filter_map(|entry| entry.grid_y)
        .min()
        .unwrap_or(0);
    let max_y = rectangles
        .iter()
        .filter_map(|entry| entry.grid_y)
        .max()
        .unwrap_or(0);
    let grid_w = (max_x - min_x + 1).max(1) as usize;
    let grid_h = (max_y - min_y + 1).max(1) as usize;
    let occupied_scene_cells: BTreeSet<(i32, i32)> = rectangles
        .iter()
        .filter_map(|rectangle| rectangle.grid_x.zip(rectangle.grid_y))
        .collect();
    let landmass_name = rectangles[0].landmass_name.clone();
    let harbor = choose_harbor_rectangle(&rectangles, min_x, max_x, max_y)
        .ok_or_else(|| format!("landmass {landmass_id} has no harbor candidate"))?;
    let harbor_scene_id = generated_scene_id(&landmass_name, harbor);

    let coastline = (landmass_id != 0).then(|| {
        generate_coastline_raster(CoastlineGenerationInput {
            landmass_id,
            seed: settings.seed,
            shoreline_width: settings.shoreline_width,
            mountain_radius: settings.mountain_radius,
            min_scene_x: min_x,
            min_scene_y: min_y,
            grid_width: grid_w,
            grid_height: grid_h,
            occupied_scene_cells: &occupied_scene_cells,
        })
    });

    let chunks = rectangles
        .iter()
        .map(|rectangle| {
            ChunkCoord::new(
                rectangle.grid_x.unwrap_or_default(),
                rectangle.grid_y.unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    let region_id = if landmass_id == 0 {
        "havenwild_mainland".to_string()
    } else {
        slug(&landmass_name)
    };
    let world_plan = SurfaceWorldPlanV1::build(
        landmass_id,
        landmass_name.clone(),
        region_id,
        settings.seed,
        chunks.clone(),
        harbor_scene_id.clone(),
    )?;
    let mut scene_maps = Vec::with_capacity(rectangles.len());
    for (rectangle, chunk) in rectangles.iter().zip(chunks.iter().copied()) {
        let scene = if world_plan.is_mainland() {
            // The starter rectangles are only authored anchors inside the same
            // global mainland sampled by runtime streaming. Initial creation and
            // later chunk generation therefore use identical geography/hydrology.
            generate_surface_pcg_partition_with_profile(
                world_plan.generation_seed,
                &world_plan.region_id,
                chunk,
                settings.geography,
            )
        } else {
            let scene_id = generated_scene_id(&landmass_name, rectangle);
            let mut scene = SceneMap::blank(
                scene_id,
                format!(
                    "{} [{},{}]",
                    landmass_name,
                    rectangle.grid_x.unwrap_or_default(),
                    rectangle.grid_y.unwrap_or_default()
                ),
                SceneKind::Exterior,
                SceneBiome::Temperate,
            );
            let origin_x = (rectangle.grid_x.unwrap_or(min_x) - min_x) as usize * MAP_W;
            let origin_y = (rectangle.grid_y.unwrap_or(min_y) - min_y) as usize * MAP_H;
            populate_scene_from_coastline(
                &mut scene,
                coastline.as_ref().expect("minor island coastline should exist"),
                origin_x,
                origin_y,
                world_plan.compatibility_landmass_seed(),
                settings.mountain_radius,
            );
            scene
        };
        scene_maps.push(scene);
    }

    if !world_plan.is_mainland() {
        promote_global_highland_materials_with_strength(
            &mut scene_maps,
            &world_plan.chunks,
            world_plan.compatibility_landmass_seed(),
            settings.mountain_radius,
        )?;
    }

    let mainland_features = materialize_mainland_world_plan_compatibility(
        &world_plan,
        &mut scene_maps,
        false,
    )?;

    // Smooth geological height remains hydrology/material input. Visible cliff
    // topology is a separate discrete Level-0/1/2 generation lane so every
    // fresh world has intentional structural landforms instead of depending on
    // rare MountainRock raw-height tier crossings.
    let structural_landforms = materialize_structural_landforms(
        &mut scene_maps,
        &world_plan.chunks,
        world_plan.compatibility_landmass_seed(),
        settings.mountain_radius,
    )?;

    for (scene, chunk) in scene_maps
        .iter_mut()
        .zip(world_plan.chunks.iter().copied())
    {
        populate_pcg_natural_objects(
            scene,
            chunk,
            world_plan.compatibility_landmass_seed(),
            settings.tree_density,
        );
    }
    let scenes = rectangles
        .iter()
        .zip(scene_maps)
        .map(|(rectangle, scene)| GeneratedIslandScene {
            rectangle_id: rectangle.scene_id.clone(),
            scene,
        })
        .collect::<Vec<_>>();

    // Exterior rectangles are storage partitions of one continuous surface.
    // They must not contain player-facing edge-transfer transitions; runtime
    // coordinate crossing and neighboring-partition rendering own traversal.
    let mountain_cells = scenes
        .iter()
        .map(|entry| {
            entry
                .scene
                .map
                .tiles
                .iter()
                .filter(|tile| matches!(tile, TileKind::MountainRock))
                .count()
        })
        .sum();

    let natural_objects = scenes
        .iter()
        .map(|entry| {
            entry
                .scene
                .map
                .objects
                .iter()
                .filter(|object| {
                    matches!(
                        object.kind,
                        ObjectKind::Tree
                            | ObjectKind::Bush
                            | ObjectKind::Mushroom
                            | ObjectKind::Herb
                            | ObjectKind::Boulder
                            | ObjectKind::OreNode
                    )
                })
                .count()
        })
        .sum();

    let land_cells = scenes
        .iter()
        .map(|entry| {
            entry
                .scene
                .map
                .tiles
                .iter()
                .filter(|tile| !tile.is_water())
                .count()
        })
        .sum();

    let shoreline_cells = scenes
        .iter()
        .map(|entry| {
            entry
                .scene
                .map
                .tiles
                .iter()
                .filter(|tile| {
                    matches!(
                        tile,
                        TileKind::Sand
                            | TileKind::WetSand
                            | TileKind::PebbleShore
                            | TileKind::MudBank
                            | TileKind::ShoreFoam
                    )
                })
                .count()
        })
        .sum();

    Ok(GeneratedIsland {
        landmass_id,
        landmass_name,
        harbor_rectangle_id: harbor.scene_id.clone(),
        harbor_scene_id,
        scenes,
        land_cells,
        mountain_cells,
        shoreline_cells,
        natural_objects,
        mainland_features,
        structural_landforms,
        world_plan,
    })
}

pub(crate) fn generated_scene_id(name: &str, rectangle: &SceneRectangleSpec) -> ProjectSceneId {
    let region = if rectangle.landmass_id == 0 {
        // Keep the established save/runtime namespace even though the player-
        // facing landmass name is now Alderreach.
        "havenwild_mainland".to_string()
    } else {
        slug(name)
    };
    crate::continuous_surface::pcg_surface_scene_id(
        &region,
        crate::open_world::ChunkCoord::new(
            rectangle.grid_x.unwrap_or_default(),
            rectangle.grid_y.unwrap_or_default(),
        ),
    )
}

fn choose_harbor_rectangle<'a>(
    rectangles: &'a [&SceneRectangleSpec],
    min_x: i32,
    max_x: i32,
    max_y: i32,
) -> Option<&'a SceneRectangleSpec> {
    let center_x = (min_x + max_x) as f32 * 0.5;
    rectangles
        .iter()
        .copied()
        .filter(|rectangle| rectangle.grid_y == Some(max_y))
        .min_by(|left, right| {
            let left_distance = (left.grid_x.unwrap_or(min_x) as f32 - center_x).abs();
            let right_distance = (right.grid_x.unwrap_or(min_x) as f32 - center_x).abs();
            left_distance.total_cmp(&right_distance)
        })
        .or_else(|| rectangles.first().copied())
}

fn populate_scene_from_coastline(
    scene: &mut SceneMap,
    coastline: &crate::island_coastline::CoastlineRaster,
    origin_x: usize,
    origin_y: usize,
    seed: u64,
    mountain_radius: f32,
) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let tile = coastline.tile(origin_x + x as usize, origin_y + y as usize);
            let gx = origin_x as i32 + x;
            let gy = origin_y as i32 + y;
            let height = geological_height(seed, gx, gy, tile, mountain_radius);
            scene.map.set(x, y, tile);
            scene.map.set_height(x, y, height);
        }
    }

    let (spawn_x, spawn_y) = nearest_walkable_to_center(scene);
    scene.spawn_x = spawn_x;
    scene.spawn_y = spawn_y;
}

fn geological_height(seed: u64, x: i32, y: i32, tile: TileKind, mountain_radius: f32) -> u8 {
    let base = height_for_tile(tile);
    if tile.is_water() || matches!(tile, TileKind::Sand | TileKind::WetSand | TileKind::MudBank) {
        return base;
    }
    let broad = geological_noise(seed ^ 0x4252_4f41, x, y, 168.0);
    let ridge = 1.0 - (geological_noise(seed ^ 0x5249_4447, x, y, 92.0) * 2.0 - 1.0).abs();
    let strength = mountain_radius.clamp(0.0, 1.0) as f64;
    let value = 48.0 + broad * 88.0 + ridge * 148.0 * strength;
    value.round().clamp(48.0, 232.0) as u8
}

pub(crate) fn geological_noise(seed: u64, x: i32, y: i32, scale: f64) -> f64 {
    let fx = x as f64 / scale;
    let fy = y as f64 / scale;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = (fx - x0 as f64).powi(2) * (3.0 - 2.0 * (fx - x0 as f64));
    let ty = (fy - y0 as f64).powi(2) * (3.0 - 2.0 * (fy - y0 as f64));
    let sample = |sx: i32, sy: i32| -> f64 {
        let mut value = seed ^ (sx as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        value ^= (sy as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
        value ^= value >> 30;
        value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value ^= value >> 27;
        value = value.wrapping_mul(0x94d0_49bb_1331_11eb) ^ (value >> 31);
        value as f64 / u64::MAX as f64
    };
    let a = sample(x0, y0);
    let b = sample(x0 + 1, y0);
    let c = sample(x0, y0 + 1);
    let d = sample(x0 + 1, y0 + 1);
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}

fn nearest_walkable_to_center(scene: &SceneMap) -> (i32, i32) {
    let center_x = MAP_W as i32 / 2;
    let center_y = MAP_H as i32 / 2;
    let max_radius = MAP_W.max(MAP_H) as i32;
    for radius in 0..=max_radius {
        for y in center_y - radius..=center_y + radius {
            for x in center_x - radius..=center_x + radius {
                if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
                    continue;
                }
                if scene.map.get(x, y).walkable() {
                    return (x, y);
                }
            }
        }
    }
    (center_x, center_y)
}

fn height_for_tile(tile: TileKind) -> u8 {
    match tile {
        TileKind::OceanDeep | TileKind::DeepWater => 16,
        TileKind::OceanShallow
        | TileKind::ShallowWater
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend
        | TileKind::ShoreFoam => 24,
        TileKind::Sand | TileKind::WetSand | TileKind::MudBank => 34,
        TileKind::Grass => 48,
        _ => 48,
    }
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut pending_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !result.is_empty() {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if result.is_empty() {
        "island".to_string()
    } else {
        result
    }
}


// Pass167Z109Q: keep island generation production code focused; tests remain
// in the same module scope through include so behavior and private access are unchanged.
include!("island_pcg_tests.rs");
