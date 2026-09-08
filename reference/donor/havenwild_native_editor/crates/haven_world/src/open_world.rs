//! Havenwild open-world and cave-generation contracts.
//!
//! This module is intentionally data-first. It gives the editor, runtime, save
//! system, and future generator one shared vocabulary for the new direction:
//! a Factorio-esque seeded overworld plus procedural/destructible caves.

use serde::{Deserialize, Serialize};

pub const HAVENWILD_WORLDGEN_VERSION: &str = "havenwild.worldgen.v1";
pub const DEFAULT_OVERWORLD_CHUNK_SIZE_TILES: i32 = 64;
pub const DEFAULT_OVERWORLD_SIZE_TILES: i32 = 1024;
pub const DEFAULT_CAVE_CHUNK_SIZE_TILES: i32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldSizePreset {
    Small,
    Standard,
    Large,
    Huge,
}

impl WorldSizePreset {
    pub fn label(self) -> &'static str {
        match self {
            WorldSizePreset::Small => "Small",
            WorldSizePreset::Standard => "Standard",
            WorldSizePreset::Large => "Large",
            WorldSizePreset::Huge => "Huge",
        }
    }

    pub fn tiles(self) -> i32 {
        match self {
            WorldSizePreset::Small => 512,
            WorldSizePreset::Standard => 1024,
            WorldSizePreset::Large => 1536,
            WorldSizePreset::Huge => 2048,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldTileCoord {
    pub x: i32,
    pub y: i32,
}

impl WorldTileCoord {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn chunk(self, chunk_size_tiles: i32) -> ChunkCoord {
        ChunkCoord {
            x: self.x.div_euclid(chunk_size_tiles.max(1)),
            y: self.y.div_euclid(chunk_size_tiles.max(1)),
        }
    }

    pub fn local_in_chunk(self, chunk_size_tiles: i32) -> (i32, i32) {
        let size = chunk_size_tiles.max(1);
        (self.x.rem_euclid(size), self.y.rem_euclid(size))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSurfaceConfig {
    pub seed: u64,
    pub generator_version: &'static str,
    pub preset: WorldSizePreset,
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub chunk_size_tiles: i32,
    pub transitions_are_exterior_only: bool,
    pub wrap_east_west: bool,
    pub clamp_north_south: bool,
}

impl WorldSurfaceConfig {
    pub fn standard(seed: u64) -> Self {
        let preset = WorldSizePreset::Standard;
        Self {
            seed,
            generator_version: HAVENWILD_WORLDGEN_VERSION,
            preset,
            width_tiles: preset.tiles(),
            height_tiles: preset.tiles(),
            chunk_size_tiles: DEFAULT_OVERWORLD_CHUNK_SIZE_TILES,
            transitions_are_exterior_only: false,
            wrap_east_west: true,
            clamp_north_south: true,
        }
    }

    pub fn chunk_count_x(&self) -> i32 {
        ceil_div(self.width_tiles.max(1), self.chunk_size_tiles.max(1))
    }

    pub fn chunk_count_y(&self) -> i32 {
        ceil_div(self.height_tiles.max(1), self.chunk_size_tiles.max(1))
    }

    pub fn total_chunk_count(&self) -> i32 {
        self.chunk_count_x() * self.chunk_count_y()
    }

    pub fn contains_tile(&self, tile: WorldTileCoord) -> bool {
        tile.x >= 0 && tile.y >= 0 && tile.x < self.width_tiles && tile.y < self.height_tiles
    }

    pub fn topology(&self) -> crate::world_topology::WorldTopologyConfig {
        let mut topology = crate::world_topology::WorldTopologyConfig::from_surface(self);
        if !self.wrap_east_west {
            topology.horizontal_wrap = crate::world_topology::HorizontalWrapMode::Disabled;
        }
        topology
    }

    pub fn canonical_tile(&self, tile: WorldTileCoord) -> Option<WorldTileCoord> {
        self.topology().canonical_tile(tile)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequiredAnchorKind {
    StarterTavernPlot,
    TownCenter,
    CityHall,
    Harbor,
    StarterCaveEntrance,
    MainRoadNetwork,
    EarlyForest,
    EarlyForageZone,
    EarlyFertileSoil,
}

impl RequiredAnchorKind {
    pub fn id(self) -> &'static str {
        match self {
            RequiredAnchorKind::StarterTavernPlot => "starter_tavern_plot",
            RequiredAnchorKind::TownCenter => "town_center",
            RequiredAnchorKind::CityHall => "city_hall",
            RequiredAnchorKind::Harbor => "harbor",
            RequiredAnchorKind::StarterCaveEntrance => "starter_cave_entrance",
            RequiredAnchorKind::MainRoadNetwork => "main_road_network",
            RequiredAnchorKind::EarlyForest => "early_forest",
            RequiredAnchorKind::EarlyForageZone => "early_forage_zone",
            RequiredAnchorKind::EarlyFertileSoil => "early_fertile_soil",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RequiredAnchorKind::StarterTavernPlot => "Starter tavern plot",
            RequiredAnchorKind::TownCenter => "Town center",
            RequiredAnchorKind::CityHall => "City Hall / permits",
            RequiredAnchorKind::Harbor => "Harbor / ferry",
            RequiredAnchorKind::StarterCaveEntrance => "Starter cave entrance",
            RequiredAnchorKind::MainRoadNetwork => "Main road network",
            RequiredAnchorKind::EarlyForest => "Early forest",
            RequiredAnchorKind::EarlyForageZone => "Early forage zone",
            RequiredAnchorKind::EarlyFertileSoil => "Early fertile soil",
        }
    }
}

pub fn required_open_world_anchors() -> &'static [RequiredAnchorKind] {
    &[
        RequiredAnchorKind::StarterTavernPlot,
        RequiredAnchorKind::TownCenter,
        RequiredAnchorKind::CityHall,
        RequiredAnchorKind::Harbor,
        RequiredAnchorKind::StarterCaveEntrance,
        RequiredAnchorKind::MainRoadNetwork,
        RequiredAnchorKind::EarlyForest,
        RequiredAnchorKind::EarlyForageZone,
        RequiredAnchorKind::EarlyFertileSoil,
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldAnchorClaim {
    pub kind: RequiredAnchorKind,
    pub tile: WorldTileCoord,
    pub radius_tiles: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveDepthBand {
    Upper,
    Middle,
    Deep,
    Ancient,
}

impl CaveDepthBand {
    pub fn for_depth(depth: i32) -> Self {
        match depth {
            d if d <= 2 => CaveDepthBand::Upper,
            3..=5 => CaveDepthBand::Middle,
            6..=10 => CaveDepthBand::Deep,
            _ => CaveDepthBand::Ancient,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CaveDepthBand::Upper => "Upper cave",
            CaveDepthBand::Middle => "Middle cave",
            CaveDepthBand::Deep => "Deep cave",
            CaveDepthBand::Ancient => "Ancient depth",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveWeakSpotKind {
    CrackedWall,
    WeakFloor,
    CollapsedPassage,
    SuspiciousRubble,
    OrePocketPlug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveRevealKind {
    SideRoom,
    OrePocket,
    TreasureNook,
    LowerShaft,
    BiomePocket,
    UndergroundWater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalRequirement {
    None,
    RopeLadder,
    ReinforcedLadder,
    WinchLift,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaveGeneratorConfig {
    pub seed: u64,
    pub entrance_id: &'static str,
    pub chunk_size_tiles: i32,
    pub infinite_depth: bool,
    pub weak_spot_frequency_per_chunk: u8,
    pub lower_shaft_frequency_per_chunk: u8,
    pub rope_ladder_required_for_lower_shafts: bool,
}

impl CaveGeneratorConfig {
    pub fn starter_mountain(seed: u64) -> Self {
        Self {
            seed,
            entrance_id: "starter_mountain_cave",
            chunk_size_tiles: DEFAULT_CAVE_CHUNK_SIZE_TILES,
            infinite_depth: true,
            weak_spot_frequency_per_chunk: 2,
            lower_shaft_frequency_per_chunk: 1,
            rope_ladder_required_for_lower_shafts: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaveChunkCoord {
    pub x: i32,
    pub y: i32,
    pub depth: i32,
}

impl CaveChunkCoord {
    pub const fn new(x: i32, y: i32, depth: i32) -> Self {
        Self { x, y, depth }
    }

    pub fn depth_band(self) -> CaveDepthBand {
        CaveDepthBand::for_depth(self.depth)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaveWeakSpotClaim {
    pub kind: CaveWeakSpotKind,
    pub reveal: CaveRevealKind,
    pub coord: CaveChunkCoord,
    pub local_tile: (i32, i32),
    pub required_blast_power: u8,
    pub traversal_requirement: TraversalRequirement,
}

impl CaveWeakSpotClaim {
    pub fn lower_shaft(coord: CaveChunkCoord, local_tile: (i32, i32)) -> Self {
        Self {
            kind: CaveWeakSpotKind::WeakFloor,
            reveal: CaveRevealKind::LowerShaft,
            coord,
            local_tile,
            required_blast_power: 2,
            traversal_requirement: TraversalRequirement::RopeLadder,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenWorldValidationReport {
    pub status: OpenWorldValidationStatus,
    pub messages: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenWorldValidationStatus {
    Pass,
    Warn,
    Fail,
}

impl OpenWorldValidationReport {
    pub fn status_line(&self) -> String {
        let label = match self.status {
            OpenWorldValidationStatus::Pass => "PASS",
            OpenWorldValidationStatus::Warn => "WARN",
            OpenWorldValidationStatus::Fail => "FAIL",
        };
        format!("{label}: {} message(s)", self.messages.len())
    }
}

pub fn validate_open_world_surface_config(
    config: &WorldSurfaceConfig,
    anchors: &[WorldAnchorClaim],
) -> OpenWorldValidationReport {
    let mut messages = Vec::new();
    let mut has_failure = false;

    if config.generator_version != HAVENWILD_WORLDGEN_VERSION {
        messages.push(format!(
            "Generator version should be {} but was {}",
            HAVENWILD_WORLDGEN_VERSION, config.generator_version
        ));
        has_failure = true;
    }

    if config.chunk_size_tiles <= 0 || config.width_tiles <= 0 || config.height_tiles <= 0 {
        messages.push("World surface dimensions and chunk size must be positive".to_string());
        has_failure = true;
    }

    if config.width_tiles < WorldSizePreset::Small.tiles()
        || config.height_tiles < WorldSizePreset::Small.tiles()
    {
        messages.push("Overworld is below the intended minimum open-world size".to_string());
    }

    if config.transitions_are_exterior_only {
        messages.push(
            "Outdoor traversal should be continuous; transitions should be reserved for enclosed spaces"
                .to_string(),
        );
        has_failure = true;
    }

    for required in required_open_world_anchors() {
        match anchors.iter().find(|anchor| anchor.kind == *required) {
            Some(anchor) if !config.contains_tile(anchor.tile) => {
                messages.push(format!(
                    "Required anchor '{}' is outside the overworld bounds",
                    required.id()
                ));
                has_failure = true;
            }
            Some(_) => {}
            None => {
                messages.push(format!(
                    "Missing required anchor '{}': {}",
                    required.id(),
                    required.label()
                ));
                has_failure = true;
            }
        }
    }

    if messages.is_empty() {
        messages.push(format!(
            "{} {}x{} tiles, {}x{} chunks",
            config.preset.label(),
            config.width_tiles,
            config.height_tiles,
            config.chunk_count_x(),
            config.chunk_count_y()
        ));
    }

    OpenWorldValidationReport {
        status: if has_failure {
            OpenWorldValidationStatus::Fail
        } else if messages.len() > 1 {
            OpenWorldValidationStatus::Warn
        } else {
            OpenWorldValidationStatus::Pass
        },
        messages,
    }
}

pub fn starter_anchor_claims(config: &WorldSurfaceConfig) -> Vec<WorldAnchorClaim> {
    let center_x = config.width_tiles / 2;
    let center_y = config.height_tiles / 2;
    vec![
        WorldAnchorClaim {
            kind: RequiredAnchorKind::StarterTavernPlot,
            tile: WorldTileCoord::new(center_x - 24, center_y + 16),
            radius_tiles: 16,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::TownCenter,
            tile: WorldTileCoord::new(center_x + 96, center_y),
            radius_tiles: 48,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::CityHall,
            tile: WorldTileCoord::new(center_x + 104, center_y - 8),
            radius_tiles: 8,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::Harbor,
            tile: WorldTileCoord::new(center_x + 160, center_y + 80),
            radius_tiles: 32,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::StarterCaveEntrance,
            tile: WorldTileCoord::new(center_x - 80, center_y - 48),
            radius_tiles: 12,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::MainRoadNetwork,
            tile: WorldTileCoord::new(center_x + 32, center_y + 16),
            radius_tiles: 160,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::EarlyForest,
            tile: WorldTileCoord::new(center_x - 128, center_y + 32),
            radius_tiles: 64,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::EarlyForageZone,
            tile: WorldTileCoord::new(center_x - 64, center_y + 96),
            radius_tiles: 48,
        },
        WorldAnchorClaim {
            kind: RequiredAnchorKind::EarlyFertileSoil,
            tile: WorldTileCoord::new(center_x + 8, center_y + 64),
            radius_tiles: 48,
        },
    ]
}

const fn ceil_div(value: i32, divisor: i32) -> i32 {
    (value + divisor - 1) / divisor
}
