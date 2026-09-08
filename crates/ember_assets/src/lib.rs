use ember_core::{ProjectPath, StableId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: StableId,
    pub kind: AssetKind,
    pub source: ProjectPath,
    pub provider: StableId,
    pub revision: String,
    pub dependencies: Vec<StableId>,
    pub attribution: Vec<AttributionRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetKind {
    Texture,
    Tileset,
    Animation,
    Audio,
    Font,
    Shader,
    Scene,
    World,
    Entity,
    BehaviorGraph,
    Data,
    Custom(String),
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttributionRecord {
    pub creator: String,
    pub license: String,
    pub source_url: Option<String>,
}
#[derive(Default)]
pub struct AssetRegistry {
    records: BTreeMap<StableId, AssetRecord>,
}
impl AssetRegistry {
    pub fn insert(&mut self, r: AssetRecord) -> Option<AssetRecord> {
        self.records.insert(r.id.clone(), r)
    }
    pub fn get(&self, id: &StableId) -> Option<&AssetRecord> {
        self.records.get(id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &AssetRecord> {
        self.records.values()
    }
}
pub trait AssetProvider {
    fn id(&self) -> StableId;
    fn scan(&self) -> Result<Vec<AssetRecord>, AssetError>;
}
#[derive(Debug)]
pub struct AssetError(pub String);
impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AssetError {}
