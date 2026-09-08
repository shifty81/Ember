use haven_core::TavernMap;

use super::terrain_family::TerrainFamily;

pub const N: u8 = 1 << 0;
pub const E: u8 = 1 << 1;
pub const S: u8 = 1 << 2;
pub const W: u8 = 1 << 3;
pub const NE: u8 = 1 << 4;
pub const SE: u8 = 1 << 5;
pub const SW: u8 = 1 << 6;
pub const NW: u8 = 1 << 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardinalDirection {
    North,
    East,
    South,
    West,
}

impl CardinalDirection {
    pub const ALL: [CardinalDirection; 4] = [
        CardinalDirection::North,
        CardinalDirection::East,
        CardinalDirection::South,
        CardinalDirection::West,
    ];

    pub fn offset(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    pub fn bit(self) -> u8 {
        match self {
            Self::North => N,
            Self::East => E,
            Self::South => S,
            Self::West => W,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagonalDirection {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl DiagonalDirection {
    pub const ALL: [DiagonalDirection; 4] = [
        DiagonalDirection::NorthEast,
        DiagonalDirection::SouthEast,
        DiagonalDirection::SouthWest,
        DiagonalDirection::NorthWest,
    ];

    pub fn offset(self) -> (i32, i32) {
        match self {
            Self::NorthEast => (1, -1),
            Self::SouthEast => (1, 1),
            Self::SouthWest => (-1, 1),
            Self::NorthWest => (-1, -1),
        }
    }

    pub fn bit(self) -> u8 {
        match self {
            Self::NorthEast => NE,
            Self::SouthEast => SE,
            Self::SouthWest => SW,
            Self::NorthWest => NW,
        }
    }

    pub fn cardinals(self) -> (CardinalDirection, CardinalDirection) {
        match self {
            Self::NorthEast => (CardinalDirection::North, CardinalDirection::East),
            Self::SouthEast => (CardinalDirection::South, CardinalDirection::East),
            Self::SouthWest => (CardinalDirection::South, CardinalDirection::West),
            Self::NorthWest => (CardinalDirection::North, CardinalDirection::West),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyNeighbors {
    pub center: TerrainFamily,
    pub north: TerrainFamily,
    pub east: TerrainFamily,
    pub south: TerrainFamily,
    pub west: TerrainFamily,
    pub north_east: TerrainFamily,
    pub south_east: TerrainFamily,
    pub south_west: TerrainFamily,
    pub north_west: TerrainFamily,
}

impl FamilyNeighbors {
    pub fn family_at_direction(self, direction: CardinalDirection) -> TerrainFamily {
        match direction {
            CardinalDirection::North => self.north,
            CardinalDirection::East => self.east,
            CardinalDirection::South => self.south,
            CardinalDirection::West => self.west,
        }
    }

    pub fn family_at_diagonal(self, direction: DiagonalDirection) -> TerrainFamily {
        match direction {
            DiagonalDirection::NorthEast => self.north_east,
            DiagonalDirection::SouthEast => self.south_east,
            DiagonalDirection::SouthWest => self.south_west,
            DiagonalDirection::NorthWest => self.north_west,
        }
    }

    pub fn same_family_cardinal_mask(self) -> u8 {
        let mut mask = 0;
        for direction in CardinalDirection::ALL {
            if self.family_at_direction(direction) == self.center {
                mask |= direction.bit();
            }
        }
        mask
    }

    pub fn different_family_cardinal_mask(self) -> u8 {
        let mut mask = 0;
        for direction in CardinalDirection::ALL {
            let neighbor = self.family_at_direction(direction);
            if neighbor != self.center && neighbor != TerrainFamily::Void {
                mask |= direction.bit();
            }
        }
        mask
    }

    pub fn water_cardinal_mask(self) -> u8 {
        let mut mask = 0;
        for direction in CardinalDirection::ALL {
            if self.family_at_direction(direction).is_water() {
                mask |= direction.bit();
            }
        }
        mask
    }

    pub fn land_cardinal_mask(self) -> u8 {
        let mut mask = 0;
        for direction in CardinalDirection::ALL {
            if self.family_at_direction(direction).is_land() {
                mask |= direction.bit();
            }
        }
        mask
    }
}

pub fn family_at(map: &TavernMap, x: i32, y: i32) -> TerrainFamily {
    TavernMap::idx(x, y)
        .map(|_| {
            let tile = map.get(x, y);
            // Bridge visuals own a water underlay and a separate structure
            // layer. Boundary resolution must see that underlay, not a road,
            // or adjacent water paints square dirt banks against the deck.
            if tile == haven_core::TileKind::Bridge {
                TerrainFamily::ShallowWater
            } else {
                TerrainFamily::from_tile(tile)
            }
        })
        .unwrap_or(TerrainFamily::Void)
}

pub fn family_neighbors(map: &TavernMap, x: i32, y: i32) -> FamilyNeighbors {
    FamilyNeighbors {
        center: family_at(map, x, y),
        north: family_at(map, x, y - 1),
        east: family_at(map, x + 1, y),
        south: family_at(map, x, y + 1),
        west: family_at(map, x - 1, y),
        north_east: family_at(map, x + 1, y - 1),
        south_east: family_at(map, x + 1, y + 1),
        south_west: family_at(map, x - 1, y + 1),
        north_west: family_at(map, x - 1, y - 1),
    }
}

#[cfg(test)]
mod bridge_underlay_tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn bridge_cells_present_shallow_water_to_transition_resolution() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::Bridge);
        assert_eq!(family_at(&map, 4, 4), TerrainFamily::ShallowWater);
        assert_eq!(
            TerrainFamily::from_tile(TileKind::Bridge),
            TerrainFamily::Road
        );
    }
}
