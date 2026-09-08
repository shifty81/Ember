//! Layered world-generation helpers for Havenwild.
//!
//! This module is intentionally dependency-free and does not change the active
//! starter maps by itself. Add `pub mod worldgen_layered;` in `lib.rs` to expose it.
//! Then call the helpers from individual `generate_*` scene functions as you migrate
//! from rectangle-authored maps to layered procedural scenes.

use crate::{SceneBiome, SceneId, TavernMap, TileKind, ZoneKind, MAP_H, MAP_W};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowDir {
    None,
    North,
    East,
    South,
    West,
}

impl FlowDir {
    pub fn code(self) -> &'static str {
        match self {
            FlowDir::None => "none",
            FlowDir::North => "north",
            FlowDir::East => "east",
            FlowDir::South => "south",
            FlowDir::West => "west",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorKind {
    None,
    FlowerPatch,
    Clover,
    Reeds,
    Shells,
    Driftwood,
    Pebbles,
    SmallRock,
    Mushroom,
    CaveCrystal,
    Foam,
    WaterRipple,
    LeafLitter,
    SnowDrift,
}

impl DecorKind {
    pub fn code(self) -> &'static str {
        match self {
            DecorKind::None => "none",
            DecorKind::FlowerPatch => "flower_patch",
            DecorKind::Clover => "clover",
            DecorKind::Reeds => "reeds",
            DecorKind::Shells => "shells",
            DecorKind::Driftwood => "driftwood",
            DecorKind::Pebbles => "pebbles",
            DecorKind::SmallRock => "small_rock",
            DecorKind::Mushroom => "mushroom",
            DecorKind::CaveCrystal => "cave_crystal",
            DecorKind::Foam => "foam",
            DecorKind::WaterRipple => "water_ripple",
            DecorKind::LeafLitter => "leaf_litter",
            DecorKind::SnowDrift => "snow_drift",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GeneratedCell {
    pub base: TileKind,
    pub height: u8,
    pub moisture: u8,
    pub fertility: u8,
    pub flow: FlowDir,
    pub decor: DecorKind,
    pub protected: bool,
}

impl Default for GeneratedCell {
    fn default() -> Self {
        Self {
            base: TileKind::Grass,
            height: 50,
            moisture: 50,
            fertility: 50,
            flow: FlowDir::None,
            decor: DecorKind::None,
            protected: false,
        }
    }
}

pub struct GeneratedSceneLayers {
    pub scene_id: SceneId,
    pub biome: SceneBiome,
    pub cells: Vec<GeneratedCell>,
    pub zones: Vec<ZoneKind>,
}

impl GeneratedSceneLayers {
    pub fn new(scene_id: SceneId, biome: SceneBiome) -> Self {
        Self {
            scene_id,
            biome,
            cells: vec![GeneratedCell::default(); MAP_W * MAP_H],
            zones: vec![ZoneKind::None; MAP_W * MAP_H],
        }
    }

    pub fn idx(x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return None;
        }
        Some(y as usize * MAP_W + x as usize)
    }

    pub fn cell(&self, x: i32, y: i32) -> GeneratedCell {
        Self::idx(x, y).map_or(GeneratedCell::default(), |idx| self.cells[idx])
    }

    pub fn cell_mut(&mut self, x: i32, y: i32) -> Option<&mut GeneratedCell> {
        Self::idx(x, y).map(|idx| &mut self.cells[idx])
    }

    pub fn set_zone(&mut self, x: i32, y: i32, zone: ZoneKind) {
        if let Some(idx) = Self::idx(x, y) {
            self.zones[idx] = zone;
        }
    }

    pub fn apply_to_map(&self, map: &mut TavernMap) {
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                if let Some(idx) = Self::idx(x, y) {
                    let cell = self.cells[idx];
                    map.set(x, y, cell.base);
                    map.set_height(x, y, cell.height);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayeredWorldGenConfig {
    pub seed: u32,
    pub water_level: u8,
    pub beach_width: u8,
    pub forest_one_in: u32,
    pub decor_one_in: u32,
    pub mountain_center_x: i32,
    pub mountain_center_y: i32,
}

impl Default for LayeredWorldGenConfig {
    fn default() -> Self {
        Self {
            seed: 11,
            water_level: 38,
            beach_width: 5,
            forest_one_in: 11,
            decor_one_in: 8,
            mountain_center_x: 32,
            mountain_center_y: 7,
        }
    }
}

pub fn generate_starter_island_layers(seed: u32) -> GeneratedSceneLayers {
    let config = LayeredWorldGenConfig {
        seed,
        ..Default::default()
    };
    let mut scene = GeneratedSceneLayers::new(SceneId::Farmstead, SceneBiome::Temperate);
    seed_height_and_moisture(&mut scene, config);
    classify_coastal_terrain(&mut scene, config);
    carve_river_from_mountain(&mut scene, config);
    reserve_starter_tavern_and_farm(&mut scene);
    scatter_decor(&mut scene, config);
    scene
}

pub fn generate_woods_layers(seed: u32) -> GeneratedSceneLayers {
    let config = LayeredWorldGenConfig {
        seed,
        water_level: 30,
        beach_width: 2,
        forest_one_in: 4,
        decor_one_in: 5,
        mountain_center_x: 44,
        mountain_center_y: 8,
    };
    let mut scene = GeneratedSceneLayers::new(SceneId::EastWoods, SceneBiome::Temperate);
    seed_height_and_moisture(&mut scene, config);
    classify_coastal_terrain(&mut scene, config);
    carve_river_from_mountain(&mut scene, config);
    carve_horizontal_path(&mut scene, 14, 16, TileKind::StonePath);
    scatter_decor(&mut scene, config);
    scene
}

pub fn generate_south_field_layers(seed: u32) -> GeneratedSceneLayers {
    let config = LayeredWorldGenConfig {
        seed,
        water_level: 32,
        beach_width: 3,
        forest_one_in: 29,
        decor_one_in: 10,
        mountain_center_x: 40,
        mountain_center_y: 5,
    };
    let mut scene = GeneratedSceneLayers::new(SceneId::SouthField, SceneBiome::Coastal);
    seed_height_and_moisture(&mut scene, config);
    classify_coastal_terrain(&mut scene, config);
    carve_horizontal_path(&mut scene, 2, 2, TileKind::StonePath);

    for y in 7..26 {
        for x in 4..42 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = if hash_cell(seed, x, y).is_multiple_of(7) {
                    TileKind::Dirt
                } else {
                    TileKind::TilledSoil
                };
                cell.fertility = 62 + (hash_cell(seed ^ 0x7711, x, y) % 28) as u8;
                cell.protected = true;
            }
            scene.set_zone(x, y, ZoneKind::Field);
        }
    }

    for y in 20..23 {
        for x in 10..15 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = TileKind::GreenhouseZone;
            }
            scene.set_zone(x, y, ZoneKind::Greenhouse);
        }
    }

    scatter_decor(&mut scene, config);
    scene
}

fn seed_height_and_moisture(scene: &mut GeneratedSceneLayers, config: LayeredWorldGenConfig) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let edge_distance = x.min(y).min(MAP_W as i32 - 1 - x).min(MAP_H as i32 - 1 - y);
            let edge_penalty = (10 - edge_distance).max(0) * 4;
            let dx = x - config.mountain_center_x;
            let dy = y - config.mountain_center_y;
            let mountain = (42 - ((dx * dx + dy * dy) / 7)).max(0);
            let broad = (hash_cell(config.seed, x / 3, y / 3) % 18) as i32 - 8;
            let detail = (hash_cell(config.seed ^ 0xA53C, x, y) % 9) as i32 - 4;
            let height = (48 + mountain + broad + detail - edge_penalty).clamp(0, 100) as u8;
            let moisture = (72 - height as i32 / 2
                + (hash_cell(config.seed ^ 0x99D1, x, y) % 32) as i32)
                .clamp(0, 100) as u8;
            let fertility = (45 + moisture as i32 / 3 - height as i32 / 5
                + (hash_cell(config.seed ^ 0x7177, x, y) % 22) as i32)
                .clamp(0, 100) as u8;

            if let Some(cell) = scene.cell_mut(x, y) {
                cell.height = height;
                cell.moisture = moisture;
                cell.fertility = fertility;
            }
        }
    }
}

fn classify_coastal_terrain(scene: &mut GeneratedSceneLayers, config: LayeredWorldGenConfig) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let mut cell = scene.cell(x, y);
            let water_delta = cell.height as i32 - config.water_level as i32;
            cell.base = if water_delta < -8 {
                TileKind::DeepWater
            } else if water_delta < -2 {
                TileKind::Water
            } else if water_delta < 1 {
                TileKind::ShallowWater
            } else if water_delta <= config.beach_width as i32 / 2 {
                TileKind::WetSand
            } else if water_delta <= config.beach_width as i32 {
                TileKind::Sand
            } else if cliff_score(scene, x, y) >= 18 {
                TileKind::Cliff
            } else if cell.height > 78 {
                TileKind::MountainRock
            } else if cell.moisture > 70 && hash_cell(config.seed ^ 0xBEEF, x, y).is_multiple_of(5)
            {
                TileKind::TallGrass
            } else {
                TileKind::Grass
            };

            if let Some(dst) = scene.cell_mut(x, y) {
                *dst = cell;
            }
        }
    }
}

fn carve_river_from_mountain(scene: &mut GeneratedSceneLayers, config: LayeredWorldGenConfig) {
    let mut x = config.mountain_center_x.clamp(2, MAP_W as i32 - 3);
    let mut y = (config.mountain_center_y + 3).clamp(2, MAP_H as i32 - 3);

    for step in 0..96 {
        let next = choose_downhill_step(scene, x, y, config.seed ^ step as u32);
        let flow = match (next.0 - x, next.1 - y) {
            (1, 0) => FlowDir::East,
            (-1, 0) => FlowDir::West,
            (0, 1) => FlowDir::South,
            (0, -1) => FlowDir::North,
            _ => FlowDir::None,
        };
        for oy in -1i32..=1 {
            for ox in -1i32..=1 {
                if ox.abs() + oy.abs() > 1 {
                    continue;
                }
                if let Some(cell) = scene.cell_mut(x + ox, y + oy) {
                    cell.base = if ox == 0 && oy == 0 {
                        TileKind::ShallowWater
                    } else {
                        TileKind::WetSand
                    };
                    cell.moisture = cell.moisture.max(88);
                    cell.flow = flow;
                    cell.decor = if ox == 0 && oy == 0 {
                        DecorKind::WaterRipple
                    } else {
                        DecorKind::Reeds
                    };
                }
            }
        }
        x = next.0;
        y = next.1;
        if x <= 1 || y <= 1 || x >= MAP_W as i32 - 2 || y >= MAP_H as i32 - 2 {
            break;
        }
    }
}

fn choose_downhill_step(scene: &GeneratedSceneLayers, x: i32, y: i32, seed: u32) -> (i32, i32) {
    let candidates = [
        (x + 1, y),
        (x - 1, y),
        (x, y + 1),
        (x, y - 1),
        (x + 1, y + 1),
        (x - 1, y + 1),
    ];
    let mut best = (x, y);
    let mut best_score = i32::MAX;
    for (cx, cy) in candidates {
        let cell = scene.cell(cx, cy);
        let coast_pull = (MAP_H as i32 - cy).abs() / 2 + (MAP_W as i32 - cx).abs() / 3;
        let wobble = (hash_cell(seed, cx, cy) % 9) as i32;
        let score = cell.height as i32 + coast_pull + wobble;
        if score < best_score {
            best_score = score;
            best = (cx, cy);
        }
    }
    best
}

fn reserve_starter_tavern_and_farm(scene: &mut GeneratedSceneLayers) {
    // Tavern footprint.
    for y in 3..13 {
        for x in 15..32 {
            let porch = y == 12 && (20..=26).contains(&x);
            let wall = !porch && (x == 15 || x == 31 || y == 3 || y == 12);
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = if wall {
                    TileKind::Wall
                } else {
                    TileKind::WoodFloor
                };
                cell.protected = true;
            }
            scene.set_zone(x, y, ZoneKind::Tavern);
        }
    }

    // Main road.
    carve_horizontal_path(scene, 14, 15, TileKind::Road);
    for x in 18..29 {
        if let Some(cell) = scene.cell_mut(x, 13) {
            cell.base = TileKind::Road;
            cell.protected = true;
        }
    }

    // Starter field.
    for y in 20..28 {
        for x in 5..18 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = if (x + y) % 5 == 0 {
                    TileKind::Dirt
                } else {
                    TileKind::TilledSoil
                };
                cell.fertility = cell.fertility.max(65);
                cell.protected = true;
            }
            scene.set_zone(x, y, ZoneKind::Field);
        }
    }

    // Starter greenhouse marker zone.
    for y in 21..24 {
        for x in 10..16 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = TileKind::GreenhouseZone;
                cell.protected = true;
            }
            scene.set_zone(x, y, ZoneKind::Greenhouse);
        }
    }

    // Cave mouth visual bias.
    for y in 5..10 {
        for x in 40..46 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = if x == 43 && y == 7 {
                    TileKind::MountainPath
                } else {
                    TileKind::MountainRock
                };
                cell.protected = true;
            }
        }
    }
}

fn carve_horizontal_path(scene: &mut GeneratedSceneLayers, y0: i32, y1: i32, tile: TileKind) {
    for y in y0..=y1 {
        for x in 0..MAP_W as i32 {
            if let Some(cell) = scene.cell_mut(x, y) {
                cell.base = tile;
                cell.protected = true;
            }
        }
    }
}

fn scatter_decor(scene: &mut GeneratedSceneLayers, config: LayeredWorldGenConfig) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(cell) = scene.cell_mut(x, y) else {
                continue;
            };
            if cell.protected || cell.decor != DecorKind::None {
                continue;
            }
            let roll = hash_cell(config.seed ^ 0xDEC0, x, y);
            cell.decor = match cell.base {
                TileKind::Grass | TileKind::TallGrass
                    if roll.is_multiple_of(config.decor_one_in) =>
                {
                    if cell.moisture > 62 {
                        DecorKind::FlowerPatch
                    } else {
                        DecorKind::Clover
                    }
                }
                TileKind::Sand | TileKind::WetSand if roll.is_multiple_of(7) => {
                    if roll.is_multiple_of(3) {
                        DecorKind::Driftwood
                    } else {
                        DecorKind::Shells
                    }
                }
                TileKind::WetSand | TileKind::ShallowWater if roll.is_multiple_of(5) => {
                    DecorKind::Reeds
                }
                TileKind::MountainPath | TileKind::Dirt | TileKind::PebbleShore
                    if roll.is_multiple_of(11) =>
                {
                    DecorKind::SmallRock
                }
                TileKind::CaveFloor if roll.is_multiple_of(13) => DecorKind::Mushroom,
                TileKind::CaveWall if roll.is_multiple_of(31) => DecorKind::CaveCrystal,
                TileKind::Water | TileKind::DeepWater if roll.is_multiple_of(4) => {
                    DecorKind::WaterRipple
                }
                _ => DecorKind::None,
            };
        }
    }
}

fn cliff_score(scene: &GeneratedSceneLayers, x: i32, y: i32) -> u8 {
    let center = scene.cell(x, y).height as i32;
    let mut max_delta = 0;
    for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
        let delta = (center - scene.cell(nx, ny).height as i32).abs();
        max_delta = max_delta.max(delta);
    }
    max_delta as u8
}

pub fn visual_variant(seed: u32, x: i32, y: i32, variants: u32) -> u32 {
    if variants == 0 {
        return 0;
    }
    hash_cell(seed ^ 0x51A7, x, y) % variants
}

pub fn is_fishable(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
    )
}

pub fn supports_decor(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Grass
            | TileKind::TallGrass
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
            | TileKind::Dirt
            | TileKind::MountainPath
            | TileKind::CaveFloor
            | TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
            | TileKind::MudBank
    )
}

fn hash_cell(seed: u32, x: i32, y: i32) -> u32 {
    let mut n = seed ^ (x as u32).wrapping_mul(374_761_393);
    n = n.wrapping_add((y as u32).wrapping_mul(668_265_263));
    n ^= n >> 13;
    n = n.wrapping_mul(1_274_126_177);
    n ^ (n >> 16)
}
