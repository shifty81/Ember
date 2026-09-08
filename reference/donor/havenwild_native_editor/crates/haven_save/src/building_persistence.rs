use haven_core::BuildingDefinition;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const BUILDING_WORLD_STATE_SCHEMA: &str = "havenwild.building_world_state.v1";

/// Save-owned state for one canonical logical building. Runtime scene materializations are
/// intentionally not persisted: they are reconstructed from `BuildingDefinition` and stable
/// building/floor identities when the player enters the building again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingSaveRecord {
    pub building: BuildingDefinition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_seed: Option<u64>,
    #[serde(default)]
    pub mutable_state: BTreeMap<String, String>,
}

impl BuildingSaveRecord {
    pub fn new(building: BuildingDefinition) -> Self {
        Self {
            building,
            generation_seed: None,
            mutable_state: BTreeMap::new(),
        }
    }
}

/// Versioned save container for logical building instances. The logical layout is persisted,
/// but derived `BuildingRuntimeScene` caches are not, preventing runtime-scene state from
/// becoming a second building authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWorldStateFile {
    pub schema: String,
    #[serde(default)]
    pub buildings: Vec<BuildingSaveRecord>,
}

impl Default for BuildingWorldStateFile {
    fn default() -> Self {
        Self {
            schema: BUILDING_WORLD_STATE_SCHEMA.to_string(),
            buildings: Vec::new(),
        }
    }
}

impl BuildingWorldStateFile {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != BUILDING_WORLD_STATE_SCHEMA {
            return Err(format!("unsupported building world state schema {}", self.schema));
        }
        let mut ids = BTreeSet::new();
        for record in &self.buildings {
            let id = record.building.id.as_str();
            if id.trim().is_empty() {
                return Err("building save record has an empty instance id".to_string());
            }
            if !ids.insert(id.to_string()) {
                return Err(format!("duplicate building save instance id {id}"));
            }
            if record.building.layout.exterior.width == 0
                || record.building.layout.exterior.height == 0
                || record.building.layout.interior.width == 0
                || record.building.layout.interior.height == 0
            {
                return Err(format!("building save record {id} has zero-sized layout bounds"));
            }
        }
        Ok(())
    }

    pub fn to_pretty_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|error| format!("could not serialize building world state: {error}"))
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: Self = serde_json::from_str(text)
            .map_err(|error| format!("could not parse building world state: {error}"))?;
        value.validate()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_building_round_trip_preserves_exact_independent_layout() {
        let request = haven_world::building_acceptance_requests()
            .into_iter()
            .find(|request| request.archetype == "inn")
            .unwrap();
        let building = haven_world::generate_building_layout(&request).unwrap();
        let expected = building.clone();
        let file = BuildingWorldStateFile {
            schema: BUILDING_WORLD_STATE_SCHEMA.to_string(),
            buildings: vec![BuildingSaveRecord {
                building,
                generation_seed: Some(request.seed),
                mutable_state: BTreeMap::new(),
            }],
        };
        let json = file.to_pretty_json().unwrap();
        let restored = BuildingWorldStateFile::from_json(&json).unwrap();
        assert_eq!(restored.buildings[0].building, expected);
        assert!(restored.buildings[0].building.layout.interior.width > restored.buildings[0].building.layout.exterior.width);
        assert_eq!(restored.buildings[0].building.layout.interior.floors.len(), 3);
    }

    #[test]
    fn multiple_instances_keep_distinct_save_identity() {
        let requests = haven_world::building_acceptance_requests();
        let a = haven_world::generate_building_layout(&requests[0]).unwrap();
        let b = haven_world::generate_building_layout(&requests[1]).unwrap();
        let file = BuildingWorldStateFile {
            schema: BUILDING_WORLD_STATE_SCHEMA.to_string(),
            buildings: vec![BuildingSaveRecord::new(a), BuildingSaveRecord::new(b)],
        };
        let restored = BuildingWorldStateFile::from_json(&file.to_pretty_json().unwrap()).unwrap();
        assert_ne!(restored.buildings[0].building.id, restored.buildings[1].building.id);
    }
}
