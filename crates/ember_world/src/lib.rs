use ember_core::StableId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cell3 {
    pub x: i32,
    pub y: i32,
    pub z: i16,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoxelCell {
    pub material: StableId,
    pub shape: VoxelShape,
    pub liquid: Option<LiquidCell>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VoxelShape {
    Solid,
    Slope { rise: i8, run: i8 },
    Stair,
    Platform,
    Custom(StableId),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LiquidCell {
    pub kind: StableId,
    pub fill: f32,
    pub flow: [f32; 2],
    pub speed: f32,
}
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct WorldVolume {
    cells: BTreeMap<Cell3, VoxelCell>,
}
impl WorldVolume {
    pub fn set(&mut self, p: Cell3, c: VoxelCell) {
        self.cells.insert(p, c);
    }
    pub fn get(&self, p: &Cell3) -> Option<&VoxelCell> {
        self.cells.get(p)
    }
    pub fn remove(&mut self, p: &Cell3) -> Option<VoxelCell> {
        self.cells.remove(p)
    }
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TerrainMaterial {
    pub id: StableId,
    pub erosion_resistance: f32,
    pub permeability: f32,
    pub friction: f32,
    pub gameplay_tags: Vec<StableId>,
}
