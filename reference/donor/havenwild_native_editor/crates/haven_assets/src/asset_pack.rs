use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct AssetPackId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct AssetId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct AssetSourceId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Terrain,
    TileObject,
    Building,
    Wall,
    Floor,
    Door,
    Furniture,
    Crop,
    Tree,
    Foliage,
    Character,
    Clothing,
    Armor,
    Tool,
    Weapon,
    Animal,
    Npc,
    Animation,
    Effect,
    Ui,
    Audio,
    Music,
    Item,
    Recipe,
    Biome,
    WorldGeneration,
    Scene,
    Interior,
    Cave,
    Dungeon,
    EditorTemplate,
    Other,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSourceKind {
    Atlas,
    RawSheet,
    Image,
    Audio,
    TiledTileset,
    GodotTileset,
    Sidecar,
    Generated,
    ExternalDependency,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LicenseRecord {
    pub license_id: String,
    #[serde(default)]
    pub production_approved: bool,
    #[serde(default)]
    pub attribution: Vec<String>,
    #[serde(default)]
    pub commercial_use: bool,
    #[serde(default)]
    pub redistribution: bool,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPackDependency {
    pub pack_id: AssetPackId,
    #[serde(default)]
    pub version_requirement: Option<String>,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetSource {
    pub id: AssetSourceId,
    pub kind: AssetSourceKind,
    pub path: String,
    #[serde(default)]
    pub tile_size: Option<[u32; 2]>,
    #[serde(default)]
    pub margin: u32,
    #[serde(default)]
    pub spacing: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetVariant {
    pub id: String,
    #[serde(default)]
    pub source_id: Option<AssetSourceId>,
    #[serde(default)]
    pub atlas_region: Option<AtlasRegion>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetDefinition {
    pub id: AssetId,
    pub category: AssetCategory,
    pub semantic_id: String,
    pub source_id: AssetSourceId,
    #[serde(default)]
    pub atlas_region: Option<AtlasRegion>,
    #[serde(default)]
    pub variants: Vec<AssetVariant>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPackManifest {
    pub schema: String,
    pub id: AssetPackId,
    pub display_name: String,
    pub version: String,
    pub license: LicenseRecord,
    #[serde(default)]
    pub production_enabled: bool,
    #[serde(default)]
    pub dependencies: Vec<AssetPackDependency>,
    #[serde(default)]
    pub sources: Vec<AssetSource>,
    #[serde(default)]
    pub assets: Vec<AssetDefinition>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableAssetRef {
    pub pack_id: AssetPackId,
    pub category: AssetCategory,
    pub asset_id: AssetId,
    pub source_id: AssetSourceId,
    pub variant_id: Option<String>,
}

#[derive(Default)]
pub struct AssetPackRegistry {
    packs: BTreeMap<AssetPackId, AssetPackManifest>,
    semantic_index: BTreeMap<String, Vec<StableAssetRef>>,
}

impl AssetPackManifest {
    pub const SCHEMA: &'static str = "havenwild.asset_pack.v1";

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != Self::SCHEMA {
            errors.push(format!("unsupported asset-pack schema {}", self.schema));
        }
        if self.id.0.trim().is_empty() {
            errors.push("pack id must not be empty".to_string());
        }
        if self.production_enabled
            && (!self.license.production_approved
                || !self.license.commercial_use
                || !self.license.redistribution)
        {
            errors.push(
                "production-enabled pack must have an approved commercial/redistributable license"
                    .to_string(),
            );
        }
        let sources: BTreeSet<_> = self.sources.iter().map(|source| &source.id).collect();
        if sources.len() != self.sources.len() {
            errors.push("source ids must be unique inside a pack".to_string());
        }
        let assets: BTreeSet<_> = self.assets.iter().map(|asset| &asset.id).collect();
        if assets.len() != self.assets.len() {
            errors.push("asset ids must be unique inside a pack".to_string());
        }
        for asset in &self.assets {
            if asset.semantic_id.trim().is_empty() {
                errors.push(format!("asset {} has an empty semantic id", asset.id.0));
            }
            if !sources.contains(&asset.source_id) {
                errors.push(format!(
                    "asset {} references missing source {}",
                    asset.id.0, asset.source_id.0
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

impl AssetPackRegistry {
    pub fn mount(&mut self, pack: AssetPackManifest) -> Result<(), Vec<String>> {
        pack.validate()?;
        if self.packs.contains_key(&pack.id) {
            return Err(vec![format!("asset pack {} is already mounted", pack.id.0)]);
        }
        for asset in &pack.assets {
            self.semantic_index
                .entry(asset.semantic_id.clone())
                .or_default()
                .push(StableAssetRef {
                    pack_id: pack.id.clone(),
                    category: asset.category.clone(),
                    asset_id: asset.id.clone(),
                    source_id: asset.source_id.clone(),
                    variant_id: None,
                });
        }
        self.packs.insert(pack.id.clone(), pack);
        for refs in self.semantic_index.values_mut() {
            refs.sort_by_key(|reference| {
                std::cmp::Reverse(self.packs[&reference.pack_id].priority)
            });
        }
        Ok(())
    }

    pub fn resolve_semantic(
        &self,
        semantic_id: &str,
        category: AssetCategory,
    ) -> Vec<&StableAssetRef> {
        self.semantic_index
            .get(semantic_id)
            .into_iter()
            .flatten()
            .filter(|item| item.category == category)
            .collect()
    }

    pub fn pack(&self, pack_id: &AssetPackId) -> Option<&AssetPackManifest> {
        self.packs.get(pack_id)
    }

    pub fn asset(&self, reference: &StableAssetRef) -> Option<&AssetDefinition> {
        self.packs
            .get(&reference.pack_id)?
            .assets
            .iter()
            .find(|asset| asset.id == reference.asset_id)
    }

    pub fn mounted_packs(&self) -> impl Iterator<Item = &AssetPackManifest> {
        self.packs.values()
    }

    pub fn mounted_pack_count(&self) -> usize {
        self.packs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(id: &str, priority: i32, semantic: &str) -> AssetPackManifest {
        AssetPackManifest {
            schema: AssetPackManifest::SCHEMA.to_string(),
            id: AssetPackId(id.to_string()),
            display_name: id.to_string(),
            version: "1".to_string(),
            license: LicenseRecord {
                license_id: "CC0-1.0".to_string(),
                production_approved: true,
                attribution: vec![],
                commercial_use: true,
                redistribution: true,
                source_url: None,
            },
            production_enabled: true,
            dependencies: vec![],
            priority,
            sources: vec![AssetSource {
                id: AssetSourceId("sheet".to_string()),
                kind: AssetSourceKind::RawSheet,
                path: "sheet.png".to_string(),
                tile_size: Some([32, 32]),
                margin: 0,
                spacing: 0,
            }],
            assets: vec![AssetDefinition {
                id: AssetId("grass".to_string()),
                category: AssetCategory::Terrain,
                semantic_id: semantic.to_string(),
                source_id: AssetSourceId("sheet".to_string()),
                atlas_region: Some(AtlasRegion {
                    x: 0,
                    y: 0,
                    width: 32,
                    height: 32,
                }),
                variants: vec![],
                tags: BTreeSet::new(),
                metadata: BTreeMap::new(),
            }],
        }
    }

    #[test]
    fn multiple_packs_can_supply_the_same_semantic_asset() {
        let mut registry = AssetPackRegistry::default();
        registry.mount(pack("core", 10, "terrain.grass")).unwrap();
        registry
            .mount(pack("revised", 20, "terrain.grass"))
            .unwrap();
        let results = registry.resolve_semantic("terrain.grass", AssetCategory::Terrain);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].pack_id.0, "revised");
    }

    #[test]
    fn category_filter_prevents_cross_category_resolution() {
        let mut registry = AssetPackRegistry::default();
        registry.mount(pack("core", 10, "terrain.grass")).unwrap();
        assert!(registry
            .resolve_semantic("terrain.grass", AssetCategory::Audio)
            .is_empty());
    }
}
