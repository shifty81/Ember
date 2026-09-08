use haven_core::{GameWorld, ObjectKind, PlacedObject, SceneMap, TileKind, ZoneKind, MAP_H, MAP_W};

use crate::{
    geographic_landforms::geographic_forest_habitat,
    mainland_features::MainlandFeatureReport,
    open_world::ChunkCoord,
    surface_world_plan::{materialize_mainland_world_plan_compatibility, SurfaceWorldPlanV1},
};

pub const fn natural_object_density_for_landmass(landmass_id: i32) -> f32 {
    if landmass_id == 0 {
        0.085
    } else {
        0.065
    }
}

pub fn natural_object_density_for_region(region: &str) -> f32 {
    if region.split('_').any(|part| part == "mainland") {
        natural_object_density_for_landmass(0)
    } else {
        natural_object_density_for_landmass(1)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NaturalObjectPopulationReport {
    pub partitions_populated: usize,
    pub trees: usize,
    pub bushes: usize,
    pub herbs_and_flowers: usize,
    pub mushrooms: usize,
    pub boulders: usize,
    pub ore_nodes: usize,
}

impl NaturalObjectPopulationReport {
    pub const fn total_objects(self) -> usize {
        self.trees
            + self.bushes
            + self.herbs_and_flowers
            + self.mushrooms
            + self.boulders
            + self.ore_nodes
    }

    fn add(&mut self, other: Self) {
        self.partitions_populated += other.partitions_populated;
        self.trees += other.trees;
        self.bushes += other.bushes;
        self.herbs_and_flowers += other.herbs_and_flowers;
        self.mushrooms += other.mushrooms;
        self.boulders += other.boulders;
        self.ore_nodes += other.ore_nodes;
    }
}

/// Populates one PCG storage partition with deterministic authored natural
/// objects. Terrain remains the placement authority: trees, bushes, flowers,
/// herbs, mushrooms, boulders, and ore nodes are objects layered over compatible
/// ground, never replacement terrain tiles.
pub fn populate_pcg_natural_objects(
    scene: &mut SceneMap,
    chunk: ChunkCoord,
    seed: u64,
    tree_density: f32,
) -> NaturalObjectPopulationReport {
    let mut report = NaturalObjectPopulationReport::default();
    let tree_density = tree_density.clamp(0.0, 0.22);

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let tile = scene.map.get(x, y);
            let gx = chunk.x * MAP_W as i32 + x;
            let gy = chunk.y * MAP_H as i32 + y;
            // Z108: populate recognizable groves/forest belts with feathered
            // woodland edges and deterministic clearings. The habitat field is
            // a bounded macro feature, not a thresholded noise contour.
            let habitat = geographic_forest_habitat(seed, gx, gy);
            let distance_to_spawn = (x - scene.spawn_x).abs().max((y - scene.spawn_y).abs());
            let boulder_roll = deterministic_cell_roll(seed, gx, gy, 0x424f_554c);
            let ore_roll = deterministic_cell_roll(seed, gx, gy, 0x4f52_454e);

            let kind =
                if distance_to_spawn > 5 && tile == TileKind::MountainRock && ore_roll < 0.018 {
                    Some(ObjectKind::OreNode)
                } else if distance_to_spawn > 4
                    && matches!(
                        tile,
                        TileKind::MountainRock | TileKind::Dirt | TileKind::Grass
                    )
                    && boulder_roll
                        < if tile == TileKind::MountainRock {
                            0.052
                        } else {
                            0.006
                        }
                {
                    Some(ObjectKind::Boulder)
                } else if tile != TileKind::Grass {
                    None
                } else {
                    let roll = deterministic_cell_roll(seed, gx, gy, 0x4e41_5455);
                    if distance_to_spawn > 4
                        && habitat > 0.40
                        && roll < tree_density * (0.55 + habitat * 0.65)
                    {
                        Some(ObjectKind::Tree)
                    } else {
                        let bush_roll = deterministic_cell_roll(seed, gx, gy, 0x4255_5348);
                        let flower_roll = deterministic_cell_roll(seed, gx, gy, 0x464c_4f57);
                        let mushroom_roll = deterministic_cell_roll(seed, gx, gy, 0x4d55_5348);
                        if distance_to_spawn > 2 && habitat > 0.28 && bush_roll < 0.025 {
                            Some(ObjectKind::Bush)
                        } else if flower_roll < if habitat < 0.18 { 0.030 } else { 0.052 } {
                            Some(ObjectKind::Herb)
                        } else if habitat > 0.58 && mushroom_roll < 0.014 {
                            Some(ObjectKind::Mushroom)
                        } else {
                            None
                        }
                    }
                };

            let Some(kind) = kind else {
                continue;
            };
            if matches!(kind, ObjectKind::Boulder | ObjectKind::OreNode)
                && !resource_footprint_is_compatible(scene, kind, x, y)
            {
                continue;
            }
            if scene.map.place_object(kind, x, y).is_none() {
                continue;
            }
            match kind {
                ObjectKind::Tree => report.trees += 1,
                ObjectKind::Bush => report.bushes += 1,
                ObjectKind::Herb => report.herbs_and_flowers += 1,
                ObjectKind::Mushroom => report.mushrooms += 1,
                ObjectKind::Boulder => report.boulders += 1,
                ObjectKind::OreNode => report.ore_nodes += 1,
                _ => {}
            }
        }
    }
    if report.total_objects() > 0 {
        report.partitions_populated = 1;
    }
    report
}

fn resource_footprint_is_compatible(scene: &SceneMap, kind: ObjectKind, x: i32, y: i32) -> bool {
    let object = PlacedObject::new(kind, x, y);
    let (cx, cy, cw, ch) = object.collision_rect();
    for fy in cy..cy + ch.max(1) {
        for fx in cx..cx + cw.max(1) {
            if scene.zone_at(fx, fy) != ZoneKind::None {
                return false;
            }
            let tile = scene.map.get(fx, fy);
            let compatible = match kind {
                ObjectKind::Boulder => {
                    matches!(
                        tile,
                        TileKind::Grass | TileKind::Dirt | TileKind::MountainRock
                    )
                }
                ObjectKind::OreNode => tile == TileKind::MountainRock,
                _ => true,
            };
            if !compatible {
                return false;
            }
        }
    }
    true
}

/// Restores deterministic natural/resource objects into older PCG saves that
/// were generated before authored surface population was enabled. Existing
/// natural or player-authored objects are never removed or duplicated.
pub fn populate_missing_pcg_natural_objects(
    world: &mut GameWorld,
    world_seed: u64,
) -> NaturalObjectPopulationReport {
    let targets = world
        .scenes
        .iter()
        .filter_map(|scene| {
            let (region, chunk) = crate::parse_pcg_surface_scene_id(&scene.id)?;
            Some((scene.id.clone(), region, chunk))
        })
        .collect::<Vec<_>>();

    let mut total = NaturalObjectPopulationReport::default();
    for (scene_id, region, chunk) in targets {
        let Some(scene) = world.scene_mut_by_id(&scene_id) else {
            continue;
        };
        let region_seed = stable_region_seed(world_seed, &region);
        let report = populate_pcg_natural_objects(
            scene,
            chunk,
            region_seed,
            natural_object_density_for_region(&region),
        );
        total.add(report);
    }
    total
}

/// Materializes missing Willowmere roads, civic foundations, and a cave host
/// into an existing mainland PCG save. Existing road layouts are preserved.
pub fn populate_missing_pcg_mainland_features(
    world: &mut GameWorld,
    world_seed: u64,
) -> Result<MainlandFeatureReport, String> {
    let targets = world
        .scenes
        .iter()
        .filter_map(|scene| {
            let (region, chunk) = crate::parse_pcg_surface_scene_id(&scene.id)?;
            region
                .split('_')
                .any(|part| part == "mainland")
                .then(|| (scene.id.clone(), chunk))
        })
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Ok(MainlandFeatureReport::default());
    }

    let mut scenes = targets
        .iter()
        .filter_map(|(scene_id, _)| world.scene_by_id(scene_id).cloned())
        .collect::<Vec<_>>();
    let chunks = targets.iter().map(|(_, chunk)| *chunk).collect::<Vec<_>>();
    if scenes.len() != targets.len() {
        return Err("mainland feature migration lost a PCG scene during collection".to_string());
    }
    let harbor_index = chunks
        .iter()
        .enumerate()
        .max_by_key(|(_, chunk)| (chunk.y, -chunk.x.abs()))
        .map(|(index, _)| index)
        .unwrap_or(0);
    let harbor_id = scenes
        .get(harbor_index)
        .map(|scene| scene.id.clone())
        .ok_or_else(|| {
            "mainland feature migration could not identify a harbor partition".to_string()
        })?;
    let world_plan = SurfaceWorldPlanV1::build(
        0,
        "Alderreach",
        "havenwild_mainland",
        world_seed,
        chunks.clone(),
        harbor_id,
    )?;
    let highland_repairs = crate::materialize_highland_shoulders(&mut scenes, &chunks)?;
    let mut report = materialize_mainland_world_plan_compatibility(
        &world_plan,
        &mut scenes,
        true,
    )?;
    report.repaired_legacy_tiles += highland_repairs;
    for ((scene_id, _), replacement) in targets.iter().zip(scenes) {
        if let Some(scene) = world.scene_mut_by_id(scene_id) {
            *scene = replacement;
        }
    }
    Ok(report)
}

fn deterministic_cell_roll(seed: u64, x: i32, y: i32, salt: u64) -> f32 {
    let mut value = seed ^ salt;
    value ^= (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value = value.rotate_left(21);
    value ^= (y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    (value as f64 / u64::MAX as f64) as f32
}

fn stable_region_seed(world_seed: u64, region: &str) -> u64 {
    let mut value = world_seed ^ 0x5245_4749_4f4e;
    for byte in region.bytes() {
        value ^= byte as u64;
        value = value.wrapping_mul(0x100_0000_01b3);
    }
    value
}
