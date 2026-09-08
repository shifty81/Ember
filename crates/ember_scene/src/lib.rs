//! Genre-neutral editable scene documents for Ember Editor.

use ember_core::StableId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneDocument {
    pub id: StableId,
    pub name: String,
    pub width_pixels: i32,
    pub height_pixels: i32,
    pub world_origin: [i32; 3],
    pub layers: Vec<SceneLayer>,
    pub entities: Vec<EntityInstance>,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SceneLayer {
    pub id: StableId,
    pub name: String,
    pub kind: SceneLayerKind,
    pub grid_size: i32,
    pub visible: bool,
    pub opacity: f32,
    pub z_index: i32,
    pub tile_cells: Vec<TileCell>,
    pub semantic_cells: Vec<SemanticCell>,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SceneLayerKind {
    Tiles,
    AutoTiles,
    SemanticGrid,
    Entities,
    Collision,
    Navigation,
    Water,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TileCell {
    pub grid: [i32; 3],
    pub source_pixels: [i32; 2],
    pub tile_id: i32,
    pub transform: TileTransform,
    pub tileset: Option<StableId>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TileTransform {
    pub flip_x: bool,
    pub flip_y: bool,
    pub transpose: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SemanticCell {
    pub grid: [i32; 3],
    pub value: i32,
    pub semantic: SemanticBinding,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SemanticBinding {
    pub material: Option<StableId>,
    pub collision: Option<CollisionSemantic>,
    pub navigation_cost: Option<f32>,
    pub tags: Vec<StableId>,
    pub behavior_graphs: Vec<StableId>,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollisionSemantic {
    Solid,
    OneWay,
    Trigger,
    Water,
    Hazard,
    Custom(StableId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityInstance {
    pub id: StableId,
    pub definition: StableId,
    pub name: String,
    pub transform: EntityTransform,
    pub components: Vec<ComponentInstance>,
    pub behavior_graphs: Vec<StableId>,
    pub tags: Vec<StableId>,
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct EntityTransform {
    pub position: [f32; 3],
    pub rotation_degrees: f32,
    pub scale: [f32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ComponentInstance {
    pub component_type: StableId,
    pub enabled: bool,
    pub properties: BTreeMap<String, Value>,
}

impl SceneDocument {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.width_pixels <= 0 || self.height_pixels <= 0 {
            errors.push("scene dimensions must be positive".to_string());
        }
        for layer in &self.layers {
            if layer.grid_size <= 0 {
                errors.push(format!("layer {} has invalid grid size", layer.name));
            }
            if !(0.0..=1.0).contains(&layer.opacity) {
                errors.push(format!(
                    "layer {} opacity must be between zero and one",
                    layer.name
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scene_with_valid_dimensions_is_valid() {
        let scene = SceneDocument {
            id: StableId::new("scene", "test").unwrap(),
            name: "Test".into(),
            width_pixels: 320,
            height_pixels: 180,
            world_origin: [0, 0, 0],
            layers: Vec::new(),
            entities: Vec::new(),
            properties: BTreeMap::new(),
        };
        assert!(scene.validate().is_ok());
    }
}
