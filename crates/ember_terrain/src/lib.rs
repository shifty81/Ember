//! Semantic terrain authoring foundation for Ember.
//!
//! Terrain is land/structure semantics. Water is intentionally represented by a
//! separate layer so rendering and simulation can evolve independently.

use ember_core::StableId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MASK_NORTH: u8 = 0b0001;
pub const MASK_EAST: u8 = 0b0010;
pub const MASK_SOUTH: u8 = 0b0100;
pub const MASK_WEST: u8 = 0b1000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell2 {
    pub x: i32,
    pub y: i32,
}

impl Cell2 {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn offset(self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceRole {
    Ground,
    Path,
    Road,
    CliffTop,
    CliffFace,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainCell {
    pub material: StableId,
    pub elevation: i16,
    pub role: SurfaceRole,
    #[serde(default)]
    pub visual_variant: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainComposer {
    pub cells: BTreeMap<Cell2, TerrainCell>,
}

impl TerrainComposer {
    pub fn set(&mut self, cell: Cell2, value: TerrainCell) -> Option<TerrainCell> {
        self.cells.insert(cell, value)
    }

    pub fn get(&self, cell: Cell2) -> Option<&TerrainCell> {
        self.cells.get(&cell)
    }

    pub fn remove(&mut self, cell: Cell2) -> Option<TerrainCell> {
        self.cells.remove(&cell)
    }

    pub fn paint_rect(&mut self, min: Cell2, max: Cell2, value: TerrainCell) -> usize {
        let x0 = min.x.min(max.x);
        let x1 = min.x.max(max.x);
        let y0 = min.y.min(max.y);
        let y1 = min.y.max(max.y);
        let mut count = 0usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cells.insert(Cell2::new(x, y), value.clone());
                count += 1;
            }
        }
        count
    }

    pub fn cardinal_mask(&self, cell: Cell2) -> u8 {
        let Some(center) = self.cells.get(&cell) else {
            return 0;
        };
        let mut mask = 0u8;
        for (delta, bit) in [
            ((0, -1), MASK_NORTH),
            ((1, 0), MASK_EAST),
            ((0, 1), MASK_SOUTH),
            ((-1, 0), MASK_WEST),
        ] {
            if self
                .cells
                .get(&cell.offset(delta.0, delta.1))
                .is_some_and(|neighbor| {
                    neighbor.material == center.material
                        && neighbor.elevation == center.elevation
                        && neighbor.role == center.role
                })
            {
                mask |= bit;
            }
        }
        mask
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutotileRule {
    pub mask: u8,
    pub tile_id: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutotileRuleSet {
    pub rules: Vec<AutotileRule>,
    pub fallback_tile_id: u32,
}

impl AutotileRuleSet {
    pub fn resolve(&self, mask: u8) -> u32 {
        self.rules
            .iter()
            .find(|rule| rule.mask == mask)
            .map(|rule| rule.tile_id)
            .unwrap_or(self.fallback_tile_id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaterCell {
    pub surface_elevation: i16,
    pub fill: u8,
    pub flow: [i8; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaterLayer {
    pub cells: BTreeMap<Cell2, WaterCell>,
}

impl WaterLayer {
    pub fn set(&mut self, cell: Cell2, value: WaterCell) -> Option<WaterCell> {
        self.cells.insert(cell, value)
    }

    pub fn get(&self, cell: Cell2) -> Option<&WaterCell> {
        self.cells.get(&cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grass() -> TerrainCell {
        TerrainCell {
            material: StableId::new("terrain_material", "grass").unwrap(),
            elevation: 0,
            role: SurfaceRole::Ground,
            visual_variant: 0,
        }
    }

    #[test]
    fn cardinal_mask_resolves_matching_neighbors() {
        let mut terrain = TerrainComposer::default();
        terrain.set(Cell2::new(0, 0), grass());
        terrain.set(Cell2::new(0, -1), grass());
        terrain.set(Cell2::new(1, 0), grass());
        assert_eq!(
            terrain.cardinal_mask(Cell2::new(0, 0)),
            MASK_NORTH | MASK_EAST
        );
    }

    #[test]
    fn water_remains_separate_from_land_semantics() {
        let mut water = WaterLayer::default();
        water.set(
            Cell2::new(4, 8),
            WaterCell {
                surface_elevation: 2,
                fill: 255,
                flow: [1, 0],
            },
        );
        assert_eq!(water.get(Cell2::new(4, 8)).unwrap().flow, [1, 0]);
    }
}
