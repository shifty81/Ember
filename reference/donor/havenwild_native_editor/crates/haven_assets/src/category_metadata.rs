use crate::asset_pack::{AssetCategory, AssetDefinition, AssetId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CATEGORY_METADATA_SCHEMA: &str = "havenwild.asset_category_metadata.v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "category", content = "data", rename_all = "snake_case")]
pub enum CategoryMetadata {
    Terrain(Box<TerrainMetadata>),
    TileObject(ObjectMetadata),
    Building(BuildingMetadata),
    Character(CharacterMetadata),
    Animation(AnimationMetadata),
    Item(ItemMetadata),
    Audio(AudioMetadata),
    Ui(UiMetadata),
    Generic(GenericMetadata),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainMetadata {
    pub schema: String,
    pub terrain_set: String,
    pub terrain_id: String,
    #[serde(default)]
    pub match_mode: TerrainMatchMode,
    #[serde(default)]
    pub peers: TerrainPeers,
    #[serde(default)]
    pub movement_cost: u16,
    #[serde(default)]
    pub collision: bool,
    #[serde(default)]
    pub footstep_material: Option<String>,
    #[serde(default)]
    pub seasonal_variants: BTreeMap<String, AssetId>,
    #[serde(default)]
    pub pcg_tags: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerrainMatchMode {
    CornersAndSides,
    CornersOnly,
    #[default]
    SidesOnly,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerrainPeers {
    #[serde(default)] pub north: Option<String>,
    #[serde(default)] pub north_east: Option<String>,
    #[serde(default)] pub east: Option<String>,
    #[serde(default)] pub south_east: Option<String>,
    #[serde(default)] pub south: Option<String>,
    #[serde(default)] pub south_west: Option<String>,
    #[serde(default)] pub west: Option<String>,
    #[serde(default)] pub north_west: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ObjectMetadata {
    pub schema: String,
    pub footprint: [u32; 2],
    #[serde(default)] pub anchor: [i32; 2],
    #[serde(default)] pub collision_shapes: Vec<CollisionShape>,
    #[serde(default)] pub interaction_sockets: Vec<InteractionSocket>,
    #[serde(default)] pub states: BTreeMap<String, AssetId>,
    #[serde(default)] pub placement_tags: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BuildingMetadata {
    pub schema: String,
    pub footprint: [u32; 2],
    /// Canonical W46 same-world structural assembly identity for ordinary buildings.
    #[serde(default)] pub building_recipe: Option<String>,
    /// Legacy/special-space transition target. Ordinary interiors/upstairs/basements use building_recipe instead.
    #[serde(default)] pub interior_scene: Option<String>,
    #[serde(default)] pub entrances: Vec<InteractionSocket>,
    #[serde(default)] pub upgrade_stages: Vec<AssetId>,
    #[serde(default)] pub collision_shapes: Vec<CollisionShape>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CharacterMetadata {
    pub schema: String,
    pub frame_size: [u32; 2],
    #[serde(default)] pub directions: Vec<String>,
    #[serde(default)] pub animation_clips: Vec<String>,
    #[serde(default)] pub layer_slots: Vec<String>,
    #[serde(default)] pub equipment_sockets: Vec<InteractionSocket>,
    #[serde(default)] pub foot_anchor: [i32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationMetadata {
    pub schema: String,
    pub clips: Vec<AnimationClip>,
    #[serde(default)] pub default_clip: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationClip {
    pub id: String,
    pub frames: Vec<AnimationFrame>,
    #[serde(default)] pub looping: bool,
    #[serde(default)] pub direction: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnimationFrame {
    pub tile_id: u32,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ItemMetadata {
    pub schema: String,
    #[serde(default)] pub stack_limit: u32,
    #[serde(default)] pub icon: Option<AssetId>,
    #[serde(default)] pub world_asset: Option<AssetId>,
    #[serde(default)] pub tags: BTreeSet<String>,
    #[serde(default)] pub properties: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioMetadata {
    pub schema: String,
    pub event: String,
    #[serde(default)] pub looping: bool,
    #[serde(default)] pub spatial: bool,
    #[serde(default)] pub volume: f32,
    #[serde(default)] pub variation_weight: u32,
    #[serde(default)] pub material_tags: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct UiMetadata {
    pub schema: String,
    pub role: String,
    #[serde(default)] pub nine_slice: Option<[u32; 4]>,
    #[serde(default)] pub states: BTreeMap<String, AssetId>,
    #[serde(default)] pub theme_tokens: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct GenericMetadata {
    pub schema: String,
    #[serde(default)] pub properties: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CollisionShape {
    pub kind: String,
    #[serde(default)] pub points: Vec<[f32; 2]>,
    #[serde(default)] pub position: [f32; 2],
    #[serde(default)] pub size: [f32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InteractionSocket {
    pub id: String,
    pub position: [f32; 2],
    #[serde(default)] pub tags: BTreeSet<String>,
}

impl CategoryMetadata {
    pub fn category(&self) -> AssetCategory {
        match self {
            Self::Terrain(_) => AssetCategory::Terrain,
            Self::TileObject(_) => AssetCategory::TileObject,
            Self::Building(_) => AssetCategory::Building,
            Self::Character(_) => AssetCategory::Character,
            Self::Animation(_) => AssetCategory::Animation,
            Self::Item(_) => AssetCategory::Item,
            Self::Audio(_) => AssetCategory::Audio,
            Self::Ui(_) => AssetCategory::Ui,
            Self::Generic(_) => AssetCategory::Other,
        }
    }

    pub fn validate_for(&self, asset: &AssetDefinition) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let expected = self.category();
        if expected != AssetCategory::Other && expected != asset.category {
            errors.push(format!("asset {} category {:?} does not match metadata {:?}", asset.id.0, asset.category, expected));
        }
        match self {
            Self::Terrain(value) if value.terrain_set.trim().is_empty() || value.terrain_id.trim().is_empty() => errors.push("terrain metadata requires terrain_set and terrain_id".to_string()),
            Self::TileObject(value) if value.footprint.contains(&0) => errors.push("object footprint dimensions must be nonzero".to_string()),
            Self::Building(value) if value.footprint.contains(&0) => errors.push("building footprint dimensions must be nonzero".to_string()),
            Self::Character(value) if value.frame_size.contains(&0) => errors.push("character frame dimensions must be nonzero".to_string()),
            Self::Animation(value) if value.clips.iter().any(|clip| clip.frames.is_empty()) => errors.push("animation clips must contain at least one frame".to_string()),
            Self::Audio(value) if value.event.trim().is_empty() => errors.push("audio metadata requires an event".to_string()),
            Self::Ui(value) if value.role.trim().is_empty() => errors.push("UI metadata requires a role".to_string()),
            _ => {}
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

pub fn attach_category_metadata(asset: &mut AssetDefinition, metadata: &CategoryMetadata) -> Result<(), String> {
    metadata.validate_for(asset).map_err(|errors| errors.join("; "))?;
    asset.metadata.insert("category_contract".to_string(), serde_json::to_value(metadata).map_err(|error| error.to_string())?);
    Ok(())
}

pub fn read_category_metadata(asset: &AssetDefinition) -> Result<Option<CategoryMetadata>, String> {
    asset.metadata.get("category_contract").map(|value| serde_json::from_value(value.clone()).map_err(|error| error.to_string())).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_pack::{AssetSourceId, AtlasRegion};

    #[test]
    fn category_contract_round_trips_on_asset_definition() {
        let mut asset = AssetDefinition { id: AssetId("grass".to_string()), category: AssetCategory::Terrain, semantic_id: "terrain.grass".to_string(), source_id: AssetSourceId("sheet".to_string()), atlas_region: Some(AtlasRegion { x: 0, y: 0, width: 32, height: 32 }), variants: vec![], tags: BTreeSet::new(), metadata: BTreeMap::new() };
        let metadata = CategoryMetadata::Terrain(Box::new(TerrainMetadata { schema: CATEGORY_METADATA_SCHEMA.to_string(), terrain_set: "natural_ground".to_string(), terrain_id: "grass".to_string(), match_mode: TerrainMatchMode::CornersAndSides, peers: TerrainPeers::default(), movement_cost: 100, collision: false, footstep_material: Some("grass".to_string()), seasonal_variants: BTreeMap::new(), pcg_tags: BTreeSet::new() }));
        attach_category_metadata(&mut asset, &metadata).unwrap();
        assert_eq!(read_category_metadata(&asset).unwrap(), Some(metadata));
    }
}
