//! Deterministic cook/package planning for Ember projects.

use ember_core::{ProjectPath, StableId};
use ember_project::BuildTarget;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookKind {
    ProjectManifest,
    Scene,
    Asset,
    Package,
    RuntimeBinary,
    Metadata,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookEntry {
    pub source: ProjectPath,
    pub output: ProjectPath,
    pub kind: CookKind,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookPlan {
    pub schema_version: u32,
    pub project_id: StableId,
    pub target: BuildTarget,
    pub entries: Vec<CookEntry>,
}

impl CookPlan {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported cook-plan schema {}",
                self.schema_version
            ));
        }
        let mut outputs = BTreeSet::new();
        for entry in &self.entries {
            let output = entry.output.as_path().to_string_lossy().replace('\\', "/");
            if !outputs.insert(output.clone()) {
                return Err(format!("duplicate package output {output}"));
            }
        }
        Ok(())
    }

    pub fn ordered_entries(&self) -> Vec<&CookEntry> {
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.output.as_path().to_string_lossy().replace('\\', "/"));
        entries
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub project_id: StableId,
    pub target: BuildTarget,
    pub files: Vec<ProjectPath>,
}

impl ReleaseManifest {
    pub fn from_plan(plan: &CookPlan) -> Result<Self, String> {
        plan.validate()?;
        Ok(Self {
            schema_version: 1,
            project_id: plan.project_id.clone(),
            target: plan.target.clone(),
            files: plan
                .ordered_entries()
                .into_iter()
                .map(|entry| entry.output.clone())
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_manifest_is_deterministically_ordered() {
        let plan = CookPlan {
            schema_version: 1,
            project_id: StableId::new("project", "test").unwrap(),
            target: BuildTarget::WindowsPortable,
            entries: vec![
                CookEntry {
                    source: ProjectPath::parse("content/b").unwrap(),
                    output: ProjectPath::parse("data/b").unwrap(),
                    kind: CookKind::Asset,
                    required: true,
                },
                CookEntry {
                    source: ProjectPath::parse("content/a").unwrap(),
                    output: ProjectPath::parse("data/a").unwrap(),
                    kind: CookKind::Asset,
                    required: true,
                },
            ],
        };
        let release = ReleaseManifest::from_plan(&plan).unwrap();
        assert_eq!(release.files[0].as_path().to_string_lossy(), "data/a");
    }
}
