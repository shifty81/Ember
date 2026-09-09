use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Declared,
    Candidate,
    Certified,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub id: String,
    pub version: u32,
    pub state: CapabilityState,
    pub owner: String,
    #[serde(default)]
    pub mutation: bool,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityCatalog {
    pub schema_version: u32,
    pub project_id: String,
    #[serde(default)]
    pub capabilities: Vec<CapabilityRecord>,
}

impl CapabilityCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        let catalog: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported capability catalog schema {}",
                self.schema_version
            ));
        }
        if self.project_id.trim().is_empty() {
            return Err("capability catalog project_id is empty".into());
        }
        let mut ids = BTreeSet::new();
        for capability in &self.capabilities {
            if capability.id.trim().is_empty() {
                return Err("capability id is empty".into());
            }
            if capability.owner.trim().is_empty() {
                return Err(format!("capability {} has no owner", capability.id));
            }
            if capability.version == 0 {
                return Err(format!("capability {} has version zero", capability.id));
            }
            if !ids.insert(capability.id.clone()) {
                return Err(format!("duplicate capability {}", capability.id));
            }
        }
        for capability in &self.capabilities {
            for dependency in &capability.dependencies {
                if !ids.contains(dependency) {
                    return Err(format!(
                        "capability {} depends on missing capability {}",
                        capability.id, dependency
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn certified_ids(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .filter(|capability| capability.state == CapabilityState::Certified)
            .map(|capability| capability.id.clone())
            .collect()
    }

    pub fn by_id(&self) -> BTreeMap<&str, &CapabilityRecord> {
        self.capabilities
            .iter()
            .map(|capability| (capability.id.as_str(), capability))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certified_projection_filters_non_certified_capabilities() {
        let catalog = CapabilityCatalog {
            schema_version: 1,
            project_id: "ember".into(),
            capabilities: vec![
                CapabilityRecord {
                    id: "gate.full".into(),
                    version: 1,
                    state: CapabilityState::Certified,
                    owner: "pcc".into(),
                    mutation: false,
                    requires_approval: false,
                    platforms: vec![],
                    dependencies: vec![],
                    evidence: vec![],
                },
                CapabilityRecord {
                    id: "editor.play_test".into(),
                    version: 1,
                    state: CapabilityState::Candidate,
                    owner: "ember_session".into(),
                    mutation: true,
                    requires_approval: false,
                    platforms: vec!["windows".into()],
                    dependencies: vec!["gate.full".into()],
                    evidence: vec![],
                },
            ],
        };
        assert_eq!(catalog.certified_ids(), vec!["gate.full"]);
    }
}
