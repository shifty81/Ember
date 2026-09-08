use crate::asset_pack::AssetCategory;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LpcRepositoryInventory {
    pub schema: String,
    pub source_root: String,
    pub generated_at_unix_seconds: u64,
    pub files: Vec<LpcInventoryEntry>,
    #[serde(default)]
    pub category_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LpcInventoryEntry {
    pub relative_path: String,
    pub source_kind: String,
    pub category: AssetCategory,
    pub proposed_asset_id: String,
    pub proposed_semantic_id: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub dimensions: Option<[u32; 2]>,
    #[serde(default)]
    pub frame_cell: Option<[u32; 2]>,
    #[serde(default)]
    pub animation_family: Option<String>,
    #[serde(default)]
    pub nearby_license_files: Vec<String>,
    pub readiness: LpcInventoryReadiness,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LpcInventoryReadiness {
    Indexed,
    MetadataCandidate,
    ManualMappingRequired,
    LicenseReviewRequired,
    UnsupportedSource,
}

impl LpcRepositoryInventory {
    pub const SCHEMA: &'static str = "havenwild.lpc_revised_repository_inventory.v1";

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let inventory: Self = serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != Self::SCHEMA {
            return Err(format!("unsupported LPC inventory schema {}", self.schema));
        }
        let mut paths = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for entry in &self.files {
            if !paths.insert(entry.relative_path.as_str()) {
                return Err(format!("duplicate LPC source path {}", entry.relative_path));
            }
            if !ids.insert(entry.proposed_asset_id.as_str()) {
                return Err(format!("duplicate proposed LPC asset id {}", entry.proposed_asset_id));
            }
            if entry.proposed_semantic_id.trim().is_empty() {
                return Err(format!("{} has an empty semantic id", entry.relative_path));
            }
        }
        Ok(())
    }

    pub fn entries_for_category(
        &self,
        category: AssetCategory,
    ) -> impl Iterator<Item = &LpcInventoryEntry> {
        self.files.iter().filter(move |entry| entry.category == category)
    }
}

pub fn classify_lpc_path(path: &str) -> AssetCategory {
    let value = path.replace('\\', "/").to_ascii_lowercase();
    let contains = |terms: &[&str]| terms.iter().any(|term| value.contains(term));

    if contains(&["hair", "clothes", "clothing", "dress", "shirt", "pants", "boots", "hat"])
    {
        AssetCategory::Clothing
    } else if contains(&["character", "body", "base sprites", "spritesheet"])
        && !contains(&["animal", "creature", "monster"])
    {
        AssetCategory::Character
    } else if contains(&["armor", "armour", "helmet", "shield"])
    {
        AssetCategory::Armor
    } else if contains(&["weapon", "sword", "axe", "bow", "spear", "staff"])
    {
        AssetCategory::Weapon
    } else if contains(&["tool", "hoe", "pickaxe", "watering", "fishing"])
    {
        AssetCategory::Tool
    } else if contains(&["animal", "creature", "monster", "beast", "horse"])
    {
        AssetCategory::Animal
    } else if contains(&["npc", "portrait", "faces", "face"])
    {
        AssetCategory::Npc
    } else if contains(&["animation", "walkcycle", "slash", "spellcast", "thrust", "shoot"])
    {
        AssetCategory::Animation
    } else if contains(&["terrain", "ground", "grass", "dirt", "sand", "water", "mountain", "cliff"])
    {
        AssetCategory::Terrain
    } else if contains(&["building", "house", "castle", "tower", "roof"])
    {
        AssetCategory::Building
    } else if contains(&["wall", "fence"])
    {
        AssetCategory::Wall
    } else if contains(&["floor", "carpet", "rug"])
    {
        AssetCategory::Floor
    } else if contains(&["door", "gate", "window"])
    {
        AssetCategory::Door
    } else if contains(&["furniture", "chair", "table", "bed", "shelf", "cabinet"])
    {
        AssetCategory::Furniture
    } else if contains(&["crop", "farm", "plant", "seed"])
    {
        AssetCategory::Crop
    } else if contains(&["tree", "wood", "stump"])
    {
        AssetCategory::Tree
    } else if contains(&["foliage", "flower", "bush", "shrub", "weed"])
    {
        AssetCategory::Foliage
    } else if contains(&["interior", "indoor"])
    {
        AssetCategory::Interior
    } else if contains(&["cave", "mine"])
    {
        AssetCategory::Cave
    } else if contains(&["dungeon", "crypt", "ruin"])
    {
        AssetCategory::Dungeon
    } else if contains(&["effect", "particle", "magic", "spell", "projectile"])
    {
        AssetCategory::Effect
    } else if contains(&["ui", "interface", "icon", "cursor", "button"])
    {
        AssetCategory::Ui
    } else if contains(&["item", "inventory", "food", "potion"])
    {
        AssetCategory::Item
    } else if contains(&["tile", "object", "decor", "prop"])
    {
        AssetCategory::TileObject
    } else {
        AssetCategory::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_classification_is_not_character_only() {
        assert_eq!(classify_lpc_path("terrain/water/coast.png"), AssetCategory::Terrain);
        assert_eq!(classify_lpc_path("characters/hair/long.png"), AssetCategory::Clothing);
        assert_eq!(classify_lpc_path("buildings/castle/roof.png"), AssetCategory::Building);
        assert_eq!(classify_lpc_path("animals/horse/walk.png"), AssetCategory::Animal);
        assert_eq!(classify_lpc_path("interiors/furniture/table.png"), AssetCategory::Furniture);
    }
}
