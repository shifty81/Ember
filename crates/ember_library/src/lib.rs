//! Versioned Ember Library/Vault and project-template contracts.

use ember_core::{ProjectPath, StableId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LibraryKind {
    Asset,
    Package,
    Template,
    NodeSystem,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum LibraryLocator {
    Project(ProjectPath),
    Vault {
        namespace: String,
        key: String,
        version: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryEntry {
    pub id: StableId,
    pub kind: LibraryKind,
    pub version: String,
    pub locator: LibraryLocator,
    #[serde(default)]
    pub dependencies: Vec<StableId>,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    pub provenance: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryCatalog {
    pub entries: BTreeMap<StableId, LibraryEntry>,
}

impl LibraryCatalog {
    pub fn insert(&mut self, entry: LibraryEntry) -> Option<LibraryEntry> {
        self.entries.insert(entry.id.clone(), entry)
    }

    pub fn validate(&self) -> Result<(), String> {
        for entry in self.entries.values() {
            if entry.version.trim().is_empty() {
                return Err(format!("library entry {} has empty version", entry.id));
            }
            if let LibraryLocator::Vault {
                namespace,
                key,
                version,
            } = &entry.locator
            {
                if namespace.trim().is_empty() || key.trim().is_empty() || version.trim().is_empty()
                {
                    return Err(format!(
                        "library entry {} has invalid Vault locator",
                        entry.id
                    ));
                }
            }
            for dependency in &entry.dependencies {
                if !self.entries.contains_key(dependency) {
                    return Err(format!(
                        "library entry {} depends on missing entry {dependency}",
                        entry.id
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn promotion_plan(
        &self,
        id: &StableId,
        destination: ProjectPath,
    ) -> Result<PromotionPlan, String> {
        let entry = self
            .entries
            .get(id)
            .ok_or_else(|| format!("library entry {id} was not found"))?;
        Ok(PromotionPlan {
            entry: entry.id.clone(),
            source: entry.locator.clone(),
            destination,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotionPlan {
    pub entry: StableId,
    pub source: LibraryLocator,
    pub destination: ProjectPath,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateFile {
    pub source: ProjectPath,
    pub destination: ProjectPath,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectTemplate {
    pub id: StableId,
    pub name: String,
    pub files: Vec<TemplateFile>,
}

impl ProjectTemplate {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("project template name is empty".into());
        }
        let mut destinations = BTreeSet::new();
        for file in &self.files {
            let destination = file
                .destination
                .as_path()
                .to_string_lossy()
                .replace('\\', "/");
            if !destinations.insert(destination.clone()) {
                return Err(format!(
                    "project template duplicates destination {destination}"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_entry_builds_explicit_promotion_plan() {
        let id = StableId::new("library", "grass_tiles").unwrap();
        let mut catalog = LibraryCatalog::default();
        catalog.insert(LibraryEntry {
            id: id.clone(),
            kind: LibraryKind::Asset,
            version: "1.0.0".into(),
            locator: LibraryLocator::Vault {
                namespace: "ember".into(),
                key: "tiles/grass".into(),
                version: "1.0.0".into(),
            },
            dependencies: vec![],
            tags: BTreeSet::new(),
            provenance: Some("project-owned".into()),
        });
        catalog.validate().unwrap();
        let plan = catalog
            .promotion_plan(&id, ProjectPath::parse("content/tiles/grass.png").unwrap())
            .unwrap();
        assert_eq!(plan.entry, id);
    }
}
