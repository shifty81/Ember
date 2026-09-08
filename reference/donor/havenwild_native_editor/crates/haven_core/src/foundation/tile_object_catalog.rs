//! Canonical runtime/editor tile and placeable-object catalog.
//!
//! Keep semantic identifiers, categories, interactions, footprints, and stable
//! serialization codes together so generated atlases can expand without growing
//! the world-state implementation monolith.

use crate::TileAutoGroup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileCategory {
    Terrain,
    Floor,
    Wall,
    Farm,
    Water,
    Special,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileAuthoringStatus {
    LpcProduction,
    LpcMappingRequired,
    DerivedSystem,
    ObjectOrOverlay,
}

/// How a semantic tile participates in the world editor's universal
/// adjacency pass. This is deliberately separate from `TileAutoGroup`: the
/// latter preserves legacy same-family override/save compatibility, while
/// this policy also covers material boundaries and generated overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileAutotileBehavior {
    SemanticBoundary,
    SameFamilyAssembly,
    DerivedOverlay,
    ObjectOrStamp,
}

impl TileAutotileBehavior {
    pub fn label(self) -> &'static str {
        match self {
            Self::SemanticBoundary => "Semantic Boundary",
            Self::SameFamilyAssembly => "Same-Family Assembly",
            Self::DerivedOverlay => "Derived Overlay",
            Self::ObjectOrStamp => "Object/Stamp",
        }
    }
}

impl TileAuthoringStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::LpcProduction => "LPC Production",
            Self::LpcMappingRequired => "LPC Mapping Required",
            Self::DerivedSystem => "Derived/System",
            Self::ObjectOrOverlay => "Object/Overlay",
        }
    }
}

impl TileCategory {
    pub fn label(self) -> &'static str {
        match self {
            TileCategory::Terrain => "Terrain",
            TileCategory::Floor => "Floor",
            TileCategory::Wall => "Wall",
            TileCategory::Farm => "Farm",
            TileCategory::Water => "Water",
            TileCategory::Special => "Special",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    Grass,
    TallGrass,
    Sand,
    WetSand,
    PebbleShore,
    Road,
    StonePath,
    MountainPath,
    WoodFloor,
    PlankFloor,
    StoneFloor,
    BrickFloor,
    Wall,
    Cliff,
    MountainRock,
    Dirt,
    Bridge,
    CaveFloor,
    CaveWall,
    TilledSoil,
    WateredSoil,
    Crop,
    GreenhouseZone,
    Water,
    ShallowWater,
    DeepWater,
    OceanDeep,
    OceanShallow,
    RiverWater,
    RiverMouthBlend,
    ShoreFoam,
    MudBank,
}

impl TileKind {
    pub const ALL: [TileKind; 32] = [
        TileKind::Grass,
        TileKind::TallGrass,
        TileKind::Sand,
        TileKind::WetSand,
        TileKind::PebbleShore,
        TileKind::Road,
        TileKind::StonePath,
        TileKind::MountainPath,
        TileKind::WoodFloor,
        TileKind::PlankFloor,
        TileKind::StoneFloor,
        TileKind::BrickFloor,
        TileKind::Wall,
        TileKind::Cliff,
        TileKind::MountainRock,
        TileKind::Dirt,
        TileKind::Bridge,
        TileKind::CaveFloor,
        TileKind::CaveWall,
        TileKind::TilledSoil,
        TileKind::WateredSoil,
        TileKind::Crop,
        TileKind::GreenhouseZone,
        TileKind::Water,
        TileKind::ShallowWater,
        TileKind::DeepWater,
        TileKind::OceanDeep,
        TileKind::OceanShallow,
        TileKind::RiverWater,
        TileKind::RiverMouthBlend,
        TileKind::ShoreFoam,
        TileKind::MudBank,
    ];

    /// Reviewed semantic ground materials exposed by the production terrain
    /// workflow. Runtime compatibility tiles remain in `ALL`, but must not
    /// clutter the normal paint palette until their LPC roles are promoted.
    pub const LPC_PRODUCTION_TERRAIN: [TileKind; 12] = [
        TileKind::Grass,
        TileKind::Dirt,
        TileKind::Sand,
        TileKind::PebbleShore,
        TileKind::Road,
        TileKind::StonePath,
        TileKind::MountainPath,
        TileKind::MountainRock,
        TileKind::CaveFloor,
        TileKind::TilledSoil,
        TileKind::MudBank,
        TileKind::ShallowWater,
    ];

    /// Semantic terrain entries currently backed by the terrain-map-v7 tuple
    /// atlas. These are safe to show in the in-game terrain editor because
    /// every item resolves through the mapped LPC atlas or a guarded generated
    /// shore/depth semantic.
    pub const LPC_MAPPED_EDITOR_TERRAIN: [TileKind; 12] = [
        TileKind::Grass,
        TileKind::Dirt,
        TileKind::Sand,
        TileKind::PebbleShore,
        TileKind::Road,
        TileKind::StonePath,
        TileKind::MountainPath,
        TileKind::MountainRock,
        TileKind::CaveFloor,
        TileKind::TilledSoil,
        TileKind::MudBank,
        TileKind::ShallowWater,
    ];

    pub fn authoring_status(self) -> TileAuthoringStatus {
        match self {
            TileKind::Grass
            | TileKind::Dirt
            | TileKind::Sand
            | TileKind::PebbleShore
            | TileKind::Road
            | TileKind::StonePath
            | TileKind::MountainPath
            | TileKind::MountainRock
            | TileKind::CaveFloor
            | TileKind::TilledSoil
            | TileKind::MudBank
            | TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater => TileAuthoringStatus::LpcProduction,
            TileKind::TallGrass | TileKind::Crop => TileAuthoringStatus::ObjectOrOverlay,
            TileKind::Cliff
            | TileKind::WetSand
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
            | TileKind::WateredSoil
            | TileKind::GreenhouseZone => TileAuthoringStatus::DerivedSystem,
            TileKind::WoodFloor
            | TileKind::PlankFloor
            | TileKind::StoneFloor
            | TileKind::BrickFloor
            | TileKind::Wall
            | TileKind::Bridge
            | TileKind::CaveWall => TileAuthoringStatus::LpcMappingRequired,
        }
    }

    /// Returns true for terrain kinds whose physical surface is water.
    ///
    /// Shore foam remains a shoreline overlay and Mud is ordinary walkable
    /// ground rather than a water surface, so callers can safely use this for
    /// path and placement exclusion.
    pub const fn is_water(self) -> bool {
        matches!(
            self,
            TileKind::Water
                | TileKind::ShallowWater
                | TileKind::DeepWater
                | TileKind::OceanDeep
                | TileKind::OceanShallow
                | TileKind::RiverWater
                | TileKind::RiverMouthBlend
        )
    }

    pub fn is_production_terrain_brush(self) -> bool {
        self.authoring_status() == TileAuthoringStatus::LpcProduction
    }

    pub fn is_lpc_mapped_editor_terrain(self) -> bool {
        Self::LPC_MAPPED_EDITOR_TERRAIN.contains(&self)
    }

    /// Every tileable environment family declares how it resolves against
    /// its eight neighbors. Caves, interiors, and modular building pieces use
    /// same-family assembly; ground/water materials use semantic boundaries.
    pub fn autotile_behavior(self) -> TileAutotileBehavior {
        match self {
            TileKind::Grass
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
            | TileKind::Dirt
            | TileKind::MountainRock
            | TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::RiverWater
            | TileKind::MudBank => TileAutotileBehavior::SemanticBoundary,
            TileKind::Road
            | TileKind::StonePath
            | TileKind::MountainPath
            | TileKind::WoodFloor
            | TileKind::PlankFloor
            | TileKind::StoneFloor
            | TileKind::BrickFloor
            | TileKind::Wall
            | TileKind::Cliff
            | TileKind::Bridge
            | TileKind::CaveFloor
            | TileKind::CaveWall
            | TileKind::TilledSoil
            | TileKind::WateredSoil => TileAutotileBehavior::SameFamilyAssembly,
            TileKind::Crop
            | TileKind::GreenhouseZone
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam => TileAutotileBehavior::DerivedOverlay,
            TileKind::TallGrass => TileAutotileBehavior::ObjectOrStamp,
        }
    }

    pub fn supports_universal_autotiling(self) -> bool {
        self.autotile_behavior() != TileAutotileBehavior::ObjectOrStamp
    }

    pub fn label(self) -> &'static str {
        match self {
            TileKind::Grass => "Grass",
            TileKind::TallGrass => "Tall Grass",
            TileKind::Sand => "Sand",
            TileKind::WetSand => "Wet Sand",
            TileKind::PebbleShore => "Gravel",
            TileKind::Road => "Road",
            TileKind::StonePath => "Stone Path",
            TileKind::MountainPath => "Mountain Path",
            TileKind::WoodFloor => "Wood Floor",
            TileKind::PlankFloor => "Plank Floor",
            TileKind::StoneFloor => "Stone Floor",
            TileKind::BrickFloor => "Brick Floor",
            TileKind::Wall => "Wall",
            TileKind::Cliff => "Cliff",
            TileKind::MountainRock => "Rock Ground",
            TileKind::Dirt => "Dirt",
            TileKind::Bridge => "Bridge",
            TileKind::CaveFloor => "Cave Floor",
            TileKind::CaveWall => "Cave Wall",
            TileKind::TilledSoil => "Tilled Soil",
            TileKind::WateredSoil => "Watered Tilled Soil",
            TileKind::Crop => "Crop Seedling",
            TileKind::GreenhouseZone => "Greenhouse Zone",
            TileKind::Water => "Fresh Water",
            TileKind::ShallowWater => "Shallow Water",
            TileKind::DeepWater => "Deep Water",
            TileKind::OceanDeep => "Deep Ocean",
            TileKind::OceanShallow => "Shallow Ocean",
            TileKind::RiverWater => "River Water",
            TileKind::RiverMouthBlend => "River Mouth Blend",
            TileKind::ShoreFoam => "Shore Foam",
            TileKind::MudBank => "Mud",
        }
    }

    pub fn walkable(self) -> bool {
        crate::terrain_contract::is_walkable_ground(self)
    }

    pub fn category(self) -> TileCategory {
        match self {
            TileKind::Grass
            | TileKind::TallGrass
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
            | TileKind::Road
            | TileKind::StonePath
            | TileKind::MountainPath
            | TileKind::MountainRock
            | TileKind::Dirt
            | TileKind::MudBank => TileCategory::Terrain,
            TileKind::WoodFloor
            | TileKind::PlankFloor
            | TileKind::StoneFloor
            | TileKind::BrickFloor
            | TileKind::Bridge
            | TileKind::CaveFloor => TileCategory::Floor,
            TileKind::Wall | TileKind::Cliff | TileKind::CaveWall => TileCategory::Wall,
            TileKind::TilledSoil
            | TileKind::WateredSoil
            | TileKind::Crop
            | TileKind::GreenhouseZone => TileCategory::Farm,
            TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam => TileCategory::Water,
        }
    }

    pub fn autotile_group(self) -> Option<TileAutoGroup> {
        match self {
            TileKind::Road | TileKind::StonePath | TileKind::MountainPath | TileKind::Bridge => {
                Some(TileAutoGroup::Road)
            }
            TileKind::WoodFloor | TileKind::PlankFloor => Some(TileAutoGroup::WoodFloor),
            TileKind::StoneFloor | TileKind::BrickFloor => Some(TileAutoGroup::StoneFloor),
            TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam => Some(TileAutoGroup::Water),
            TileKind::Wall => Some(TileAutoGroup::Wall),
            TileKind::Cliff => Some(TileAutoGroup::Cliff),
            TileKind::CaveWall => Some(TileAutoGroup::CaveWall),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            TileKind::Grass => "grass",
            TileKind::TallGrass => "tall_grass",
            TileKind::Sand => "sand",
            TileKind::WetSand => "wet_sand",
            TileKind::PebbleShore => "pebble_shore",
            TileKind::Road => "road",
            TileKind::StonePath => "stone_path",
            TileKind::MountainPath => "mountain_path",
            TileKind::WoodFloor => "wood_floor",
            TileKind::PlankFloor => "plank_floor",
            TileKind::StoneFloor => "stone_floor",
            TileKind::BrickFloor => "brick_floor",
            TileKind::Wall => "wall",
            TileKind::Cliff => "cliff",
            TileKind::MountainRock => "mountain_rock",
            TileKind::Dirt => "dirt",
            TileKind::Bridge => "bridge",
            TileKind::CaveFloor => "cave_floor",
            TileKind::CaveWall => "cave_wall",
            TileKind::TilledSoil => "tilled_soil",
            TileKind::WateredSoil => "watered_soil",
            TileKind::Crop => "crop",
            TileKind::GreenhouseZone => "greenhouse_zone",
            TileKind::Water => "water",
            TileKind::ShallowWater => "shallow_water",
            TileKind::DeepWater => "deep_water",
            TileKind::OceanDeep => "ocean_deep",
            TileKind::OceanShallow => "ocean_shallow",
            TileKind::RiverWater => "river_water",
            TileKind::RiverMouthBlend => "river_mouth_blend",
            TileKind::ShoreFoam => "shore_foam",
            TileKind::MudBank => "mud_bank",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "grass" => Some(TileKind::Grass),
            "tall_grass" => Some(TileKind::TallGrass),
            "sand" => Some(TileKind::Sand),
            "wet_sand" => Some(TileKind::WetSand),
            "pebble_shore" | "pebble_ground" | "gravel" => Some(TileKind::PebbleShore),
            "road" => Some(TileKind::Road),
            "stone_path" => Some(TileKind::StonePath),
            "mountain_path" => Some(TileKind::MountainPath),
            "wood_floor" => Some(TileKind::WoodFloor),
            "plank_floor" => Some(TileKind::PlankFloor),
            "stone_floor" => Some(TileKind::StoneFloor),
            "brick_floor" => Some(TileKind::BrickFloor),
            "wall" => Some(TileKind::Wall),
            "cliff" => Some(TileKind::Cliff),
            "mountain_rock" => Some(TileKind::MountainRock),
            "dirt" => Some(TileKind::Dirt),
            "bridge" => Some(TileKind::Bridge),
            "cave_floor" => Some(TileKind::CaveFloor),
            "cave_wall" => Some(TileKind::CaveWall),
            "tilled_soil" => Some(TileKind::TilledSoil),
            "watered_soil" => Some(TileKind::WateredSoil),
            "crop" | "crop_seedling" => Some(TileKind::Crop),
            "greenhouse_zone" => Some(TileKind::GreenhouseZone),
            "water" | "fresh_water" => Some(TileKind::Water),
            "shallow_water" => Some(TileKind::ShallowWater),
            "deep_water" => Some(TileKind::DeepWater),
            "ocean_deep" | "deep_ocean" => Some(TileKind::OceanDeep),
            "ocean_shallow" | "shallow_ocean" => Some(TileKind::OceanShallow),
            "river_water" => Some(TileKind::RiverWater),
            "river_mouth_blend" => Some(TileKind::RiverMouthBlend),
            "shore_foam" => Some(TileKind::ShoreFoam),
            "mud_bank" | "mud" => Some(TileKind::MudBank),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileInteraction {
    None,
    Forage,
    Hoe,
    Water,
    Harvest,
    Rest,
    Blocked,
    Enter,
}

impl TileInteraction {
    pub const ALL: [TileInteraction; 8] = [
        TileInteraction::None,
        TileInteraction::Forage,
        TileInteraction::Hoe,
        TileInteraction::Water,
        TileInteraction::Harvest,
        TileInteraction::Rest,
        TileInteraction::Blocked,
        TileInteraction::Enter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TileInteraction::None => "None",
            TileInteraction::Forage => "Forage",
            TileInteraction::Hoe => "Hoe",
            TileInteraction::Water => "Water",
            TileInteraction::Harvest => "Harvest",
            TileInteraction::Rest => "Rest",
            TileInteraction::Blocked => "Blocked",
            TileInteraction::Enter => "Enter",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            TileInteraction::None => "none",
            TileInteraction::Forage => "forage",
            TileInteraction::Hoe => "hoe",
            TileInteraction::Water => "water",
            TileInteraction::Harvest => "harvest",
            TileInteraction::Rest => "rest",
            TileInteraction::Blocked => "blocked",
            TileInteraction::Enter => "enter",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "none" => Some(TileInteraction::None),
            "forage" => Some(TileInteraction::Forage),
            "hoe" => Some(TileInteraction::Hoe),
            "water" => Some(TileInteraction::Water),
            "harvest" => Some(TileInteraction::Harvest),
            "rest" => Some(TileInteraction::Rest),
            "blocked" => Some(TileInteraction::Blocked),
            "enter" => Some(TileInteraction::Enter),
            _ => None,
        }
    }
}

impl TileKind {
    pub fn default_interaction(self) -> TileInteraction {
        match self {
            TileKind::Grass
            | TileKind::TallGrass
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
            | TileKind::MudBank
            | TileKind::MountainRock
            | TileKind::CaveFloor => TileInteraction::Forage,
            TileKind::Road
            | TileKind::StonePath
            | TileKind::MountainPath
            | TileKind::Bridge
            | TileKind::WoodFloor
            | TileKind::PlankFloor
            | TileKind::StoneFloor
            | TileKind::BrickFloor => TileInteraction::Rest,
            TileKind::Wall | TileKind::Cliff | TileKind::CaveWall => TileInteraction::Blocked,
            TileKind::Dirt | TileKind::GreenhouseZone => TileInteraction::Hoe,
            TileKind::TilledSoil | TileKind::WateredSoil => TileInteraction::Water,
            TileKind::Crop => TileInteraction::Harvest,
            TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam => TileInteraction::Water,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    Table,
    Chair,
    Bar,
    Keg,
    Bed,
    Fireplace,
    GreenhouseMarker,
    Tree,
    Bush,
    Boulder,
    OreNode,
    Mushroom,
    Herb,
    Crate,
    Barrel,
    Well,
    Scarecrow,
    Fence,
    Lamp,
    Bench,
    Stump,
    Log,
    Sign,
    Door,
    Stairs,
    CaveEntrance,
}

impl ObjectKind {
    pub fn label(self) -> &'static str {
        match self {
            ObjectKind::Table => "Table",
            ObjectKind::Chair => "Chair",
            ObjectKind::Bar => "Bar",
            ObjectKind::Keg => "Keg",
            ObjectKind::Bed => "Guest Bed",
            ObjectKind::Fireplace => "Fireplace",
            ObjectKind::GreenhouseMarker => "Greenhouse Marker",
            ObjectKind::Tree => "Tree",
            ObjectKind::Bush => "Berry Bush",
            ObjectKind::Boulder => "Boulder",
            ObjectKind::OreNode => "Ore Node",
            ObjectKind::Mushroom => "Mushroom",
            ObjectKind::Herb => "Wild Herb",
            ObjectKind::Crate => "Crate",
            ObjectKind::Barrel => "Barrel",
            ObjectKind::Well => "Well",
            ObjectKind::Scarecrow => "Scarecrow",
            ObjectKind::Fence => "Fence Segment",
            ObjectKind::Lamp => "Lamp Post",
            ObjectKind::Bench => "Bench",
            ObjectKind::Stump => "Tree Stump",
            ObjectKind::Log => "Fallen Log",
            ObjectKind::Sign => "Signboard",
            ObjectKind::Door => "Door",
            ObjectKind::Stairs => "Stairs",
            ObjectKind::CaveEntrance => "Cave Entrance",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            ObjectKind::Table => "table",
            ObjectKind::Chair => "chair",
            ObjectKind::Bar => "bar",
            ObjectKind::Keg => "keg",
            ObjectKind::Bed => "bed",
            ObjectKind::Fireplace => "fireplace",
            ObjectKind::GreenhouseMarker => "greenhouse_marker",
            ObjectKind::Tree => "tree",
            ObjectKind::Bush => "bush",
            ObjectKind::Boulder => "boulder",
            ObjectKind::OreNode => "ore_node",
            ObjectKind::Mushroom => "mushroom",
            ObjectKind::Herb => "herb",
            ObjectKind::Crate => "crate",
            ObjectKind::Barrel => "barrel",
            ObjectKind::Well => "well",
            ObjectKind::Scarecrow => "scarecrow",
            ObjectKind::Fence => "fence",
            ObjectKind::Lamp => "lamp",
            ObjectKind::Bench => "bench",
            ObjectKind::Stump => "stump",
            ObjectKind::Log => "log",
            ObjectKind::Sign => "sign",
            ObjectKind::Door => "door",
            ObjectKind::Stairs => "stairs",
            ObjectKind::CaveEntrance => "cave_entrance",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "table" => Some(ObjectKind::Table),
            "chair" => Some(ObjectKind::Chair),
            "bar" => Some(ObjectKind::Bar),
            "keg" => Some(ObjectKind::Keg),
            "bed" => Some(ObjectKind::Bed),
            "fireplace" => Some(ObjectKind::Fireplace),
            "greenhouse_marker" => Some(ObjectKind::GreenhouseMarker),
            "tree" => Some(ObjectKind::Tree),
            "bush" => Some(ObjectKind::Bush),
            "boulder" => Some(ObjectKind::Boulder),
            "ore_node" => Some(ObjectKind::OreNode),
            "mushroom" => Some(ObjectKind::Mushroom),
            "herb" => Some(ObjectKind::Herb),
            "crate" => Some(ObjectKind::Crate),
            "barrel" => Some(ObjectKind::Barrel),
            "well" => Some(ObjectKind::Well),
            "scarecrow" => Some(ObjectKind::Scarecrow),
            "fence" => Some(ObjectKind::Fence),
            "lamp" => Some(ObjectKind::Lamp),
            "bench" => Some(ObjectKind::Bench),
            "stump" => Some(ObjectKind::Stump),
            "log" => Some(ObjectKind::Log),
            "sign" => Some(ObjectKind::Sign),
            "door" => Some(ObjectKind::Door),
            "stairs" => Some(ObjectKind::Stairs),
            "cave_entrance" => Some(ObjectKind::CaveEntrance),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "tile_object_catalog_tests.rs"]
mod terrain_collision_contract_tests;
