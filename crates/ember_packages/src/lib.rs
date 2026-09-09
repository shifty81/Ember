use ember_core::{ProjectPath, StableId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub id: StableId,
    pub version: String,
    #[serde(default)]
    pub dependencies: Vec<PackageDependency>,
    #[serde(default)]
    pub content_roots: Vec<ProjectPath>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub providers: Vec<ProviderContribution>,
    #[serde(default)]
    pub migrations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageDependency {
    pub id: StableId,
    pub version_requirement: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderContribution {
    pub capability: String,
    pub provider: StableId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MountKind {
    Project,
    Package,
    Vault,
    Generated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentMount {
    pub id: StableId,
    pub kind: MountKind,
    pub root: ProjectPath,
    pub writable: bool,
    pub authoritative: bool,
}

#[derive(Default)]
pub struct PackageRegistry {
    manifests: BTreeMap<StableId, PackageManifest>,
}

impl PackageManifest {
    pub fn load(path: &Path) -> Result<Self, PackageError> {
        let bytes = fs::read(path)?;
        let manifest: Self = serde_json::from_slice(&bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), PackageError> {
        if self.schema_version != 1 {
            return Err(PackageError::Invalid(format!(
                "package {} has unsupported schema {}",
                self.id, self.schema_version
            )));
        }
        if self.version.trim().is_empty() {
            return Err(PackageError::Invalid(format!(
                "package {} has empty version",
                self.id
            )));
        }
        Ok(())
    }
}

impl PackageRegistry {
    pub fn insert(&mut self, manifest: PackageManifest) -> Option<PackageManifest> {
        self.manifests.insert(manifest.id.clone(), manifest)
    }

    pub fn get(&self, id: &StableId) -> Option<&PackageManifest> {
        self.manifests.get(id)
    }

    pub fn resolve(&self, enabled: &[StableId]) -> Result<Vec<StableId>, PackageError> {
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut ordered = Vec::new();
        for id in enabled {
            self.visit(id, &mut visiting, &mut visited, &mut ordered)?;
        }
        Ok(ordered)
    }

    fn visit(
        &self,
        id: &StableId,
        visiting: &mut BTreeSet<StableId>,
        visited: &mut BTreeSet<StableId>,
        ordered: &mut Vec<StableId>,
    ) -> Result<(), PackageError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.clone()) {
            return Err(PackageError::Invalid(format!(
                "package dependency cycle at {id}"
            )));
        }
        let manifest = self
            .manifests
            .get(id)
            .ok_or_else(|| PackageError::Missing(id.clone()))?;
        for dependency in &manifest.dependencies {
            if dependency.optional && !self.manifests.contains_key(&dependency.id) {
                continue;
            }
            self.visit(&dependency.id, visiting, visited, ordered)?;
        }
        visiting.remove(id);
        visited.insert(id.clone());
        ordered.push(id.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub enum PackageError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Missing(StableId),
    Invalid(String),
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "package I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "package JSON failed: {error}"),
            Self::Missing(id) => write!(formatter, "missing package {id}"),
            Self::Invalid(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for PackageError {}
impl From<std::io::Error> for PackageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for PackageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::new("package", value).unwrap()
    }

    fn manifest(value: &str, dependencies: Vec<PackageDependency>) -> PackageManifest {
        PackageManifest {
            schema_version: 1,
            id: id(value),
            version: "1.0.0".into(),
            dependencies,
            content_roots: vec![],
            capabilities: vec![],
            providers: vec![],
            migrations: vec![],
        }
    }

    #[test]
    fn resolver_orders_dependencies_first() {
        let mut registry = PackageRegistry::default();
        let _ = registry.insert(manifest("base", vec![]));
        let _ = registry.insert(manifest(
            "game",
            vec![PackageDependency {
                id: id("base"),
                version_requirement: ">=1".into(),
                optional: false,
            }],
        ));
        assert_eq!(
            registry.resolve(&[id("game")]).unwrap(),
            vec![id("base"), id("game")]
        );
    }
}
