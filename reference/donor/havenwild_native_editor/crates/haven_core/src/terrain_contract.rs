//! Canonical semantic-terrain and collision contract.
//!
//! World state stores `TileKind` for save compatibility, but runtime systems
//! must classify it through this module before making gameplay decisions.
//! Presentation-only shoreline identities never become collision authority.

use crate::TileKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerrainPaintMode {
    /// Paint only the selected semantic cells. No shoreline or hydrology
    /// lifecycle is run implicitly.
    #[default]
    Exact,
    /// Rebuild a local authored shoreline around the edited region.
    Coastline,
    /// Re-resolve semantic water depth using canonical hydrology in the edited
    /// neighborhood.
    Hydrology,
}

impl TerrainPaintMode {
    pub const ALL: [Self; 3] = [Self::Exact, Self::Coastline, Self::Hydrology];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Exact => "Exact",
            Self::Coastline => "Coast",
            Self::Hydrology => "Hydrology",
        }
    }

    pub const fn help(self) -> &'static str {
        match self {
            Self::Exact => {
                "Paint only selected cells; unsupported contacts remain visible for review"
            }
            Self::Coastline => "Build a local authored shoreline/depth band around painted land",
            Self::Hydrology => "Re-resolve canonical shallow and deep water near the edit",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BaseTerrain {
    Grass,
    Sand,
    Dirt,
    Pebble,
    Road,
    WoodFloor,
    StoneFloor,
    Farm,
    ShallowWater,
    DeepWater,
    Wall,
    Cliff,
    CaveFloor,
    CaveWall,
    Greenhouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainOverlayKind {
    TallGrass,
    WetSand,
    ShoreFoam,
    RiverMouthTint,
    Crop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CollisionClass {
    WalkableGround,
    Water,
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterDepthClass {
    None,
    Shallow,
    Deep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaterSourceClass {
    None,
    Fresh,
    Salt,
    Brackish,
}

impl WaterSourceClass {
    pub const fn can_fill_watering_container(self) -> bool {
        matches!(self, Self::Fresh)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FarmingClass {
    None,
    Tillable,
    Tilled,
    Watered,
    CropOccupied,
    Greenhouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiggingClass {
    None,
    SoftSoil,
    Sand,
    Gravel,
    Rock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingClass {
    Allowed,
    FoundationRequired,
    Forbidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingSupport {
    Standard,
    Foundation,
    Bridge,
}

impl BuildingSupport {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Foundation => "Foundation",
            Self::Bridge => "Bridge",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FootstepSurface {
    Grass,
    Dirt,
    Sand,
    Gravel,
    Road,
    Wood,
    Stone,
    FarmSoil,
    Water,
    Cave,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainGameplayProfile {
    pub collision: CollisionClass,
    pub water_depth: WaterDepthClass,
    pub movement_cost: u16,
    pub farming: FarmingClass,
    pub digging: DiggingClass,
    pub building: BuildingClass,
    pub footstep: FootstepSurface,
    pub fishable: bool,
}

impl WaterDepthClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Shallow => "Shallow",
            Self::Deep => "Deep",
        }
    }
}
impl FarmingClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Tillable => "Tillable",
            Self::Tilled => "Tilled",
            Self::Watered => "Watered",
            Self::CropOccupied => "Crop Occupied",
            Self::Greenhouse => "Greenhouse",
        }
    }
}
impl DiggingClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::SoftSoil => "Soft Soil",
            Self::Sand => "Sand",
            Self::Gravel => "Gravel",
            Self::Rock => "Rock",
        }
    }
}
impl BuildingClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allowed => "Allowed",
            Self::FoundationRequired => "Foundation Required",
            Self::Forbidden => "Forbidden",
        }
    }
}

impl BuildingClass {
    pub const fn allows(self, support: BuildingSupport) -> bool {
        match (self, support) {
            (Self::Allowed, _) => true,
            (Self::FoundationRequired, BuildingSupport::Foundation | BuildingSupport::Bridge) => {
                true
            }
            (Self::FoundationRequired, BuildingSupport::Standard) => false,
            (Self::Forbidden, _) => false,
        }
    }
}
impl FootstepSurface {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Grass => "Grass",
            Self::Dirt => "Dirt",
            Self::Sand => "Sand",
            Self::Gravel => "Gravel",
            Self::Road => "Road",
            Self::Wood => "Wood",
            Self::Stone => "Stone",
            Self::FarmSoil => "Farm Soil",
            Self::Water => "Water",
            Self::Cave => "Cave",
            Self::None => "None",
        }
    }
}

impl TerrainGameplayProfile {
    pub const fn walkable(self) -> bool {
        matches!(self.collision, CollisionClass::WalkableGround)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionReason {
    Open,
    OutOfBounds,
    Terrain {
        tile: TileKind,
        class: CollisionClass,
    },
    Object {
        index: usize,
    },
    Stamp {
        index: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionResult {
    pub blocked: bool,
    pub reason: CollisionReason,
}

impl CollisionResult {
    pub const fn open() -> Self {
        Self {
            blocked: false,
            reason: CollisionReason::Open,
        }
    }

    pub const fn blocked(reason: CollisionReason) -> Self {
        Self {
            blocked: true,
            reason,
        }
    }
}

pub const fn base_terrain(tile: TileKind) -> BaseTerrain {
    match tile {
        TileKind::Grass | TileKind::TallGrass => BaseTerrain::Grass,
        TileKind::Sand | TileKind::WetSand => BaseTerrain::Sand,
        TileKind::Dirt | TileKind::MudBank => BaseTerrain::Dirt,
        TileKind::PebbleShore | TileKind::StonePath => BaseTerrain::Pebble,
        TileKind::Road | TileKind::MountainPath | TileKind::Bridge => BaseTerrain::Road,
        TileKind::WoodFloor | TileKind::PlankFloor => BaseTerrain::WoodFloor,
        TileKind::StoneFloor | TileKind::BrickFloor => BaseTerrain::StoneFloor,
        TileKind::TilledSoil | TileKind::WateredSoil | TileKind::Crop => BaseTerrain::Farm,
        TileKind::GreenhouseZone => BaseTerrain::Greenhouse,
        TileKind::Water
        | TileKind::ShallowWater
        | TileKind::OceanShallow
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend => BaseTerrain::ShallowWater,
        TileKind::ShoreFoam => BaseTerrain::Sand,
        TileKind::DeepWater | TileKind::OceanDeep => BaseTerrain::DeepWater,
        TileKind::Wall => BaseTerrain::Wall,
        TileKind::Cliff => BaseTerrain::Cliff,
        TileKind::MountainRock => BaseTerrain::StoneFloor,
        TileKind::CaveFloor => BaseTerrain::CaveFloor,
        TileKind::CaveWall => BaseTerrain::CaveWall,
    }
}

pub const fn visual_overlay(tile: TileKind) -> Option<TerrainOverlayKind> {
    match tile {
        TileKind::TallGrass => Some(TerrainOverlayKind::TallGrass),
        TileKind::WetSand => Some(TerrainOverlayKind::WetSand),
        TileKind::ShoreFoam => Some(TerrainOverlayKind::ShoreFoam),
        TileKind::RiverMouthBlend => Some(TerrainOverlayKind::RiverMouthTint),
        TileKind::Crop => Some(TerrainOverlayKind::Crop),
        _ => None,
    }
}

pub const fn terrain_gameplay_profile(tile: TileKind) -> TerrainGameplayProfile {
    let base = base_terrain(tile);
    let collision = match base {
        BaseTerrain::ShallowWater | BaseTerrain::DeepWater => CollisionClass::Water,
        BaseTerrain::Wall | BaseTerrain::Cliff | BaseTerrain::CaveWall => CollisionClass::Solid,
        _ => CollisionClass::WalkableGround,
    };
    let water_depth = match base {
        BaseTerrain::ShallowWater => WaterDepthClass::Shallow,
        BaseTerrain::DeepWater => WaterDepthClass::Deep,
        _ => WaterDepthClass::None,
    };
    let farming = match tile {
        TileKind::Grass | TileKind::Dirt | TileKind::MudBank => FarmingClass::Tillable,
        TileKind::TilledSoil => FarmingClass::Tilled,
        TileKind::WateredSoil => FarmingClass::Watered,
        TileKind::Crop => FarmingClass::CropOccupied,
        TileKind::GreenhouseZone => FarmingClass::Greenhouse,
        _ => FarmingClass::None,
    };
    let digging = match tile {
        TileKind::MountainRock => DiggingClass::Rock,
        _ => match base {
            BaseTerrain::Grass
            | BaseTerrain::Dirt
            | BaseTerrain::Farm
            | BaseTerrain::Greenhouse => DiggingClass::SoftSoil,
            BaseTerrain::Sand => DiggingClass::Sand,
            BaseTerrain::Pebble => DiggingClass::Gravel,
            BaseTerrain::Cliff | BaseTerrain::CaveWall => DiggingClass::Rock,
            _ => DiggingClass::None,
        },
    };
    let building = match tile {
        TileKind::MountainRock => BuildingClass::FoundationRequired,
        _ => match collision {
            CollisionClass::Solid => BuildingClass::Forbidden,
            CollisionClass::Water => BuildingClass::FoundationRequired,
            CollisionClass::WalkableGround => BuildingClass::Allowed,
        },
    };
    let footstep = match base {
        BaseTerrain::Grass => FootstepSurface::Grass,
        BaseTerrain::Sand => FootstepSurface::Sand,
        BaseTerrain::Dirt => FootstepSurface::Dirt,
        BaseTerrain::Pebble => FootstepSurface::Gravel,
        BaseTerrain::Road => FootstepSurface::Road,
        BaseTerrain::WoodFloor => FootstepSurface::Wood,
        BaseTerrain::StoneFloor | BaseTerrain::Cliff | BaseTerrain::Wall => FootstepSurface::Stone,
        BaseTerrain::Farm | BaseTerrain::Greenhouse => FootstepSurface::FarmSoil,
        BaseTerrain::ShallowWater | BaseTerrain::DeepWater => FootstepSurface::Water,
        BaseTerrain::CaveFloor | BaseTerrain::CaveWall => FootstepSurface::Cave,
    };
    let movement_cost = match tile {
        TileKind::MountainRock => 110,
        _ => match base {
            BaseTerrain::Road | BaseTerrain::WoodFloor | BaseTerrain::StoneFloor => 80,
            BaseTerrain::Grass
            | BaseTerrain::Dirt
            | BaseTerrain::CaveFloor
            | BaseTerrain::Greenhouse => 100,
            BaseTerrain::Sand | BaseTerrain::Pebble | BaseTerrain::Farm => 115,
            BaseTerrain::ShallowWater => 160,
            BaseTerrain::DeepWater
            | BaseTerrain::Wall
            | BaseTerrain::Cliff
            | BaseTerrain::CaveWall => u16::MAX,
        },
    };

    TerrainGameplayProfile {
        collision,
        water_depth,
        movement_cost,
        farming,
        digging,
        building,
        footstep,
        fishable: matches!(
            water_depth,
            WaterDepthClass::Shallow | WaterDepthClass::Deep
        ),
    }
}

pub const fn water_source_class(tile: TileKind) -> WaterSourceClass {
    match tile {
        TileKind::Water | TileKind::ShallowWater | TileKind::DeepWater | TileKind::RiverWater => {
            WaterSourceClass::Fresh
        }
        TileKind::OceanShallow | TileKind::OceanDeep => WaterSourceClass::Salt,
        TileKind::RiverMouthBlend => WaterSourceClass::Brackish,
        _ => WaterSourceClass::None,
    }
}

pub const fn can_fill_watering_container_from(tile: TileKind) -> bool {
    water_source_class(tile).can_fill_watering_container()
}

pub const fn collision_class(tile: TileKind) -> CollisionClass {
    terrain_gameplay_profile(tile).collision
}

pub const fn is_walkable_ground(tile: TileKind) -> bool {
    terrain_gameplay_profile(tile).walkable()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_shore_tiles_do_not_create_solid_collision() {
        assert_eq!(base_terrain(TileKind::WetSand), BaseTerrain::Sand);
        assert_eq!(
            collision_class(TileKind::WetSand),
            CollisionClass::WalkableGround
        );
        assert_eq!(base_terrain(TileKind::MudBank), BaseTerrain::Dirt);
        assert_eq!(
            collision_class(TileKind::MudBank),
            CollisionClass::WalkableGround
        );
    }

    #[test]
    fn legacy_foam_is_visual_only_and_cannot_block_land_traversal() {
        assert_eq!(
            visual_overlay(TileKind::ShoreFoam),
            Some(TerrainOverlayKind::ShoreFoam)
        );
        assert_eq!(base_terrain(TileKind::ShoreFoam), BaseTerrain::Sand);
        assert_eq!(
            collision_class(TileKind::ShoreFoam),
            CollisionClass::WalkableGround
        );
    }
    #[test]
    fn mud_is_a_ground_material_not_a_visual_overlay() {
        assert_eq!(visual_overlay(TileKind::MudBank), None);
        let mud = terrain_gameplay_profile(TileKind::MudBank);
        assert!(mud.walkable());
        assert_eq!(mud.farming, FarmingClass::Tillable);
    }

    #[test]
    fn gameplay_profile_is_semantic_and_not_visual() {
        assert_eq!(
            terrain_gameplay_profile(TileKind::WetSand),
            terrain_gameplay_profile(TileKind::Sand)
        );
        assert_eq!(
            terrain_gameplay_profile(TileKind::RiverMouthBlend),
            terrain_gameplay_profile(TileKind::ShallowWater)
        );
        assert_eq!(
            terrain_gameplay_profile(TileKind::ShoreFoam),
            terrain_gameplay_profile(TileKind::Sand)
        );
    }

    #[test]
    fn gameplay_profile_centralizes_farming_water_and_building_rules() {
        let watered = terrain_gameplay_profile(TileKind::WateredSoil);
        assert_eq!(watered.farming, FarmingClass::Watered);
        assert_eq!(watered.building, BuildingClass::Allowed);

        let shallow = terrain_gameplay_profile(TileKind::ShallowWater);
        assert_eq!(shallow.water_depth, WaterDepthClass::Shallow);
        assert!(shallow.fishable);
        assert_eq!(shallow.building, BuildingClass::FoundationRequired);

        let cliff = terrain_gameplay_profile(TileKind::Cliff);
        assert_eq!(cliff.collision, CollisionClass::Solid);
        assert_eq!(cliff.digging, DiggingClass::Rock);
        assert_eq!(cliff.building, BuildingClass::Forbidden);
    }

    #[test]
    fn mountain_rock_is_walkable_natural_rock_terrain() {
        let mountain_rock = terrain_gameplay_profile(TileKind::MountainRock);

        assert_eq!(
            base_terrain(TileKind::MountainRock),
            BaseTerrain::StoneFloor
        );
        assert_eq!(mountain_rock.collision, CollisionClass::WalkableGround);
        assert_eq!(mountain_rock.digging, DiggingClass::Rock);
        assert_eq!(mountain_rock.building, BuildingClass::FoundationRequired);
        assert_eq!(mountain_rock.footstep, FootstepSurface::Stone);
        assert_eq!(mountain_rock.movement_cost, 110);
        assert!(!mountain_rock.fishable);
    }

    #[test]
    fn marine_and_freshwater_share_water_collision_but_not_container_use() {
        assert_eq!(
            terrain_gameplay_profile(TileKind::OceanShallow).water_depth,
            WaterDepthClass::Shallow
        );
        assert_eq!(
            water_source_class(TileKind::OceanShallow),
            WaterSourceClass::Salt
        );
        assert_eq!(
            water_source_class(TileKind::OceanDeep),
            WaterSourceClass::Salt
        );
        assert_eq!(
            water_source_class(TileKind::RiverWater),
            WaterSourceClass::Fresh
        );
        assert!(can_fill_watering_container_from(TileKind::ShallowWater));
        assert!(!can_fill_watering_container_from(TileKind::OceanShallow));
        assert!(!can_fill_watering_container_from(TileKind::RiverMouthBlend));
    }

    #[test]
    fn building_support_policy_distinguishes_standard_foundation_and_bridge() {
        assert!(BuildingClass::Allowed.allows(BuildingSupport::Standard));
        assert!(BuildingClass::Allowed.allows(BuildingSupport::Foundation));
        assert!(!BuildingClass::FoundationRequired.allows(BuildingSupport::Standard));
        assert!(BuildingClass::FoundationRequired.allows(BuildingSupport::Foundation));
        assert!(BuildingClass::FoundationRequired.allows(BuildingSupport::Bridge));
        assert!(!BuildingClass::Forbidden.allows(BuildingSupport::Bridge));
    }
}
