use haven_core::TileKind;

/// Broad semantic terrain identity shared by transition, shoreline, editor,
/// and debug resolvers.
///
/// Water depth is deliberately preserved. LPC shallow-water banks and
/// shallow-to-deep rims are different authored families and cannot be selected
/// correctly when every water tile collapses into one generic identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainFamily {
    Grass,
    Dirt,
    Sand,
    WetSand,
    PebblePath,
    Road,
    WoodFloor,
    StoneFloor,
    Farm,
    ShallowWater,
    Water,
    DeepWater,
    RockWall,
    Cave,
    Greenhouse,
    Void,
}

impl TerrainFamily {
    pub fn from_tile(tile: TileKind) -> Self {
        match tile {
            TileKind::Grass | TileKind::TallGrass => Self::Grass,
            TileKind::Dirt | TileKind::MudBank => Self::Dirt,
            TileKind::Sand => Self::Sand,
            TileKind::WetSand => Self::WetSand,
            TileKind::PebbleShore | TileKind::StonePath => Self::PebblePath,
            TileKind::Road | TileKind::MountainPath | TileKind::Bridge => Self::Road,
            TileKind::WoodFloor | TileKind::PlankFloor => Self::WoodFloor,
            TileKind::StoneFloor | TileKind::BrickFloor | TileKind::MountainRock => {
                Self::StoneFloor
            }
            TileKind::TilledSoil | TileKind::WateredSoil | TileKind::Crop => Self::Farm,
            TileKind::GreenhouseZone => Self::Greenhouse,
            TileKind::ShallowWater | TileKind::OceanShallow | TileKind::ShoreFoam => {
                Self::ShallowWater
            }
            TileKind::Water | TileKind::RiverWater | TileKind::RiverMouthBlend => Self::Water,
            TileKind::DeepWater | TileKind::OceanDeep => Self::DeepWater,
            TileKind::Wall | TileKind::Cliff => Self::RockWall,
            TileKind::CaveFloor | TileKind::CaveWall => Self::Cave,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Grass => "grass",
            Self::Dirt => "dirt",
            Self::Sand => "sand",
            Self::WetSand => "wet_sand",
            Self::PebblePath => "pebble_path",
            Self::Road => "road",
            Self::WoodFloor => "wood_floor",
            Self::StoneFloor => "stone_floor",
            Self::Farm => "farm",
            Self::ShallowWater => "shallow_water",
            Self::Water => "water",
            Self::DeepWater => "deep_water",
            Self::RockWall => "rock_wall",
            Self::Cave => "cave",
            Self::Greenhouse => "greenhouse",
            Self::Void => "void",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "grass" => Some(Self::Grass),
            "dirt" => Some(Self::Dirt),
            "sand" => Some(Self::Sand),
            "wet_sand" => Some(Self::WetSand),
            "pebble_path" => Some(Self::PebblePath),
            "road" => Some(Self::Road),
            "wood_floor" => Some(Self::WoodFloor),
            "stone_floor" => Some(Self::StoneFloor),
            "farm" => Some(Self::Farm),
            "shallow_water" => Some(Self::ShallowWater),
            "water" => Some(Self::Water),
            "deep_water" => Some(Self::DeepWater),
            "rock_wall" => Some(Self::RockWall),
            "cave" => Some(Self::Cave),
            "greenhouse" => Some(Self::Greenhouse),
            "void" => Some(Self::Void),
            _ => None,
        }
    }

    pub fn is_water(self) -> bool {
        matches!(self, Self::ShallowWater | Self::Water | Self::DeepWater)
    }

    pub fn is_shallow_water(self) -> bool {
        matches!(self, Self::ShallowWater | Self::Water)
    }

    pub fn is_deep_water(self) -> bool {
        self == Self::DeepWater
    }

    pub fn is_land(self) -> bool {
        matches!(
            self,
            Self::Grass
                | Self::Dirt
                | Self::Sand
                | Self::WetSand
                | Self::PebblePath
                | Self::Road
                | Self::WoodFloor
                | Self::StoneFloor
                | Self::Farm
                | Self::RockWall
                | Self::Cave
                | Self::Greenhouse
        )
    }

    pub fn is_soft_natural(self) -> bool {
        matches!(
            self,
            Self::Grass | Self::Dirt | Self::Sand | Self::WetSand | Self::Farm | Self::Greenhouse
        )
    }

    pub fn is_constructed(self) -> bool {
        matches!(
            self,
            Self::PebblePath | Self::Road | Self::WoodFloor | Self::StoneFloor
        )
    }

    pub fn is_blocking_wall(self) -> bool {
        matches!(self, Self::RockWall | Self::Cave)
    }
}

pub fn terrain_family(tile: TileKind) -> TerrainFamily {
    TerrainFamily::from_tile(tile)
}

pub fn tile_is_water(tile: TileKind) -> bool {
    TerrainFamily::from_tile(tile).is_water()
}

pub fn tile_is_land(tile: TileKind) -> bool {
    TerrainFamily::from_tile(tile).is_land()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_depth_is_preserved_for_lpc_family_selection() {
        assert_eq!(
            TerrainFamily::from_tile(TileKind::ShallowWater),
            TerrainFamily::ShallowWater
        );
        assert_eq!(
            TerrainFamily::from_tile(TileKind::DeepWater),
            TerrainFamily::DeepWater
        );
        assert!(TerrainFamily::ShallowWater.is_water());
        assert!(TerrainFamily::DeepWater.is_water());
        assert_eq!(
            TerrainFamily::from_tile(TileKind::WetSand),
            TerrainFamily::WetSand
        );
        assert_eq!(
            TerrainFamily::from_tile(TileKind::StonePath),
            TerrainFamily::PebblePath
        );
        assert_ne!(
            TerrainFamily::from_tile(TileKind::Sand),
            TerrainFamily::from_tile(TileKind::WetSand)
        );
    }
}
