use crate::asset_pack::{AssetCategory, AssetPackRegistry, StableAssetRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::read_to_string;
use std::path::Path;

pub const LPC_CHARACTER_LAYER_CATALOG_SCHEMA: &str = "havenwild.lpc_character_layer_catalog.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcCharacterLayerCatalog {
    pub schema: String,
    pub profile_id: String,
    pub frame_cell: [u32; 2],
    pub directions: Vec<String>,
    pub layer_order: Vec<String>,
    pub slots: Vec<LpcCharacterLayerSlot>,
    pub animation_families: Vec<LpcAnimationFamily>,
    pub consumers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcCharacterLayerSlot {
    pub id: String,
    pub category: AssetCategory,
    pub required: bool,
    pub multiple: bool,
    #[serde(default)]
    pub allowed_tags: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpcAnimationFamily {
    pub id: String,
    pub rows: u32,
    pub frames_per_direction: u32,
    pub direction_order: Vec<String>,
    #[serde(default)]
    pub required_events: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterLayerCandidate {
    pub slot: String,
    pub reference: StableAssetRef,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterLayerIndex {
    pub by_slot: BTreeMap<String, Vec<CharacterLayerCandidate>>,
}

impl LpcCharacterLayerCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = read_to_string(path).map_err(|error| error.to_string())?;
        let catalog: Self = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != LPC_CHARACTER_LAYER_CATALOG_SCHEMA { return Err(format!("unsupported LPC character catalog schema {}", self.schema)); }
        if self.frame_cell != [64, 96] { return Err(format!("LPC character frame cell must be [64, 96], got {:?}", self.frame_cell)); }
        if self.directions.len() != 8 { return Err("LPC character profile must define eight directions".to_string()); }
        if !self.slots.iter().any(|slot| slot.id == "body/base" && slot.required) { return Err("LPC character profile requires body/base".to_string()); }
        let unique: BTreeSet<_> = self.slots.iter().map(|slot| &slot.id).collect();
        if unique.len() != self.slots.len() { return Err("LPC character slots must be unique".to_string()); }
        Ok(())
    }
}

impl CharacterLayerIndex {
    pub fn build(registry: &AssetPackRegistry, catalog: &LpcCharacterLayerCatalog) -> Self {
        let mut index = Self::default();
        for pack in registry.mounted_packs().filter(|pack| pack.production_enabled) {
            for asset in &pack.assets {
                let Some(slot) = asset.metadata.get("character_layer_slot").and_then(|value| value.as_str()) else { continue; };
                if !catalog.slots.iter().any(|definition| definition.id == slot) { continue; }
                index.by_slot.entry(slot.to_string()).or_default().push(CharacterLayerCandidate {
                    slot: slot.to_string(),
                    reference: StableAssetRef { pack_id: pack.id.clone(), category: asset.category.clone(), asset_id: asset.id.clone(), source_id: asset.source_id.clone(), variant_id: None },
                });
            }
        }
        for candidates in index.by_slot.values_mut() { candidates.sort_by(|left, right| left.reference.pack_id.cmp(&right.reference.pack_id).then_with(|| left.reference.asset_id.cmp(&right.reference.asset_id))); }
        index
    }
}

pub fn category_is_character_layer(category: &AssetCategory) -> bool {
    matches!(category, AssetCategory::Character | AssetCategory::Clothing | AssetCategory::Armor | AssetCategory::Tool | AssetCategory::Weapon | AssetCategory::Npc | AssetCategory::Animation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clothing_and_armor_are_character_layers() {
        assert!(category_is_character_layer(&AssetCategory::Clothing));
        assert!(category_is_character_layer(&AssetCategory::Armor));
        assert!(!category_is_character_layer(&AssetCategory::Terrain));
    }
}
