use ember_core::{ProjectPath, StableId};
use ember_packages::MountKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: StableId,
    pub kind: AssetKind,
    pub source: ProjectPath,
    pub provider: StableId,
    pub revision: String,
    #[serde(default)]
    pub dependencies: Vec<StableId>,
    #[serde(default)]
    pub attribution: Vec<AttributionRecord>,
    #[serde(default)]
    pub source_hash: Option<String>,
    #[serde(default)]
    pub derived_from: Option<StableId>,
    #[serde(default = "default_true")]
    pub authoritative: bool,
}

fn default_true() -> bool {
    true
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetMount {
    pub id: StableId,
    pub kind: MountKind,
    pub root: ProjectPath,
    pub writable: bool,
    pub authoritative: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetCatalog {
    pub schema_version: u32,
    pub project_id: StableId,
    #[serde(default)]
    pub mounts: Vec<AssetMount>,
    #[serde(default)]
    pub records: Vec<AssetRecord>,
}

#[derive(Default)]
pub struct AssetRegistry {
    records: BTreeMap<StableId, AssetRecord>,
    mounts: BTreeMap<StableId, AssetMount>,
}

impl AssetRegistry {
    pub fn insert(&mut self, record: AssetRecord) -> Option<AssetRecord> {
        self.records.insert(record.id.clone(), record)
    }

    pub fn insert_mount(&mut self, mount: AssetMount) -> Option<AssetMount> {
        self.mounts.insert(mount.id.clone(), mount)
    }

    pub fn get(&self, id: &StableId) -> Option<&AssetRecord> {
        self.records.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetRecord> {
        self.records.values()
    }

    pub fn mounts(&self) -> impl Iterator<Item = &AssetMount> {
        self.mounts.values()
    }

    pub fn from_catalog(catalog: AssetCatalog) -> Result<Self, AssetError> {
        if catalog.schema_version != 1 {
            return Err(AssetError(format!(
                "unsupported asset catalog schema {}",
                catalog.schema_version
            )));
        }
        let mut registry = Self::default();
        for mount in catalog.mounts {
            if registry.insert_mount(mount).is_some() {
                return Err(AssetError("duplicate asset mount".into()));
            }
        }
        for record in catalog.records {
            if registry.insert(record).is_some() {
                return Err(AssetError("duplicate asset id".into()));
            }
        }
        registry.validate()?;
        Ok(registry)
    }

    pub fn load(path: &Path) -> Result<Self, AssetError> {
        let bytes = fs::read(path).map_err(|error| AssetError(error.to_string()))?;
        let catalog: AssetCatalog =
            serde_json::from_slice(&bytes).map_err(|error| AssetError(error.to_string()))?;
        Self::from_catalog(catalog)
    }

    pub fn save_atomic(&self, path: &Path, project_id: StableId) -> Result<(), AssetError> {
        let catalog = AssetCatalog {
            schema_version: 1,
            project_id,
            mounts: self.mounts.values().cloned().collect(),
            records: self.records.values().cloned().collect(),
        };
        let bytes =
            serde_json::to_vec_pretty(&catalog).map_err(|error| AssetError(error.to_string()))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| AssetError(error.to_string()))?;
        }
        let temp = path.with_extension("ember-assets.tmp");
        {
            let mut file =
                fs::File::create(&temp).map_err(|error| AssetError(error.to_string()))?;
            file.write_all(&bytes)
                .map_err(|error| AssetError(error.to_string()))?;
            file.flush()
                .map_err(|error| AssetError(error.to_string()))?;
            let _ = file.sync_all();
        }
        if path.exists() {
            fs::remove_file(path).map_err(|error| AssetError(error.to_string()))?;
        }
        fs::rename(temp, path).map_err(|error| AssetError(error.to_string()))?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), AssetError> {
        let ids: BTreeSet<_> = self.records.keys().cloned().collect();
        for record in self.records.values() {
            if record.revision.trim().is_empty() {
                return Err(AssetError(format!(
                    "asset {} has empty revision",
                    record.id
                )));
            }
            for dependency in &record.dependencies {
                if !ids.contains(dependency) {
                    return Err(AssetError(format!(
                        "asset {} depends on missing asset {}",
                        record.id, dependency
                    )));
                }
            }
        }
        Ok(())
    }
}

pub trait AssetProvider {
    fn id(&self) -> StableId;
    fn scan(&self) -> Result<Vec<AssetRecord>, AssetError>;
}

#[derive(Debug)]
pub struct AssetError(pub String);
impl std::fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_rejects_missing_dependency() {
        let catalog = AssetCatalog {
            schema_version: 1,
            project_id: StableId::new("project", "test").unwrap(),
            mounts: vec![],
            records: vec![AssetRecord {
                id: StableId::new("asset", "one").unwrap(),
                kind: AssetKind::Data,
                source: ProjectPath::parse("content/one.json").unwrap(),
                provider: StableId::new("provider", "project").unwrap(),
                revision: "1".into(),
                dependencies: vec![StableId::new("asset", "missing").unwrap()],
                attribution: vec![],
                source_hash: None,
                derived_from: None,
                authoritative: true,
            }],
        };
        assert!(AssetRegistry::from_catalog(catalog).is_err());
    }
}
