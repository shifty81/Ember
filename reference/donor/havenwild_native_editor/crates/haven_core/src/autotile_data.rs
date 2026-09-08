use std::fmt;

/// Authored terrain families that use adjacency-driven tile variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileAutoGroup {
    Road,
    WoodFloor,
    StoneFloor,
    Water,
    Wall,
    Cliff,
    CaveWall,
}

impl TileAutoGroup {
    pub const ALL: [TileAutoGroup; 7] = [
        TileAutoGroup::Road,
        TileAutoGroup::WoodFloor,
        TileAutoGroup::StoneFloor,
        TileAutoGroup::Water,
        TileAutoGroup::Wall,
        TileAutoGroup::Cliff,
        TileAutoGroup::CaveWall,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TileAutoGroup::Road => "Road",
            TileAutoGroup::WoodFloor => "Wood Floor",
            TileAutoGroup::StoneFloor => "Stone Floor",
            TileAutoGroup::Water => "Water",
            TileAutoGroup::Wall => "Wall",
            TileAutoGroup::Cliff => "Cliff",
            TileAutoGroup::CaveWall => "Cave Wall",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            TileAutoGroup::Road => "road",
            TileAutoGroup::WoodFloor => "wood_floor",
            TileAutoGroup::StoneFloor => "stone_floor",
            TileAutoGroup::Water => "water",
            TileAutoGroup::Wall => "wall",
            TileAutoGroup::Cliff => "cliff",
            TileAutoGroup::CaveWall => "cave_wall",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "road" => Some(Self::Road),
            "wood_floor" => Some(Self::WoodFloor),
            "stone_floor" => Some(Self::StoneFloor),
            "water" => Some(Self::Water),
            "wall" => Some(Self::Wall),
            "cliff" => Some(Self::Cliff),
            "cave_wall" => Some(Self::CaveWall),
            _ => None,
        }
    }
}

impl fmt::Display for TileAutoGroup {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

/// A persistent manual adjacency mask for one autotiled map cell.
///
/// Bits follow the shared world neighbor-mask contract:
/// N/E/S/W are bits 0-3 and NE/SE/SW/NW are bits 4-7.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AutotileOverride {
    pub x: i32,
    pub y: i32,
    pub group: TileAutoGroup,
    pub mask: u8,
}

impl AutotileOverride {
    pub fn new(x: i32, y: i32, group: TileAutoGroup, mask: u8) -> Self {
        Self { x, y, group, mask }
    }
}
