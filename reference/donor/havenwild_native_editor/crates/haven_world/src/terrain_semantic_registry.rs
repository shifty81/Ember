use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

pub const TERRAIN_SEMANTIC_REGISTRY_SCHEMA: &str = "havenwild.terrain_world_semantic_registry.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainSemanticRegistry {
    pub schema: String,
    pub version: u32,
    pub materials: Vec<TerrainSemantic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainSemantic {
    pub id: String,
    pub code: String,
    pub stable_ordinal: u32,
    pub label: String,
    pub tile_kind: String,
    pub runtime_material: String,
    pub category: String,
    pub editor: EditorSemantic,
    pub ownership: OwnershipSemantic,
    pub runtime: RuntimeSemantic,
    pub transition: TransitionSemantic,
    pub biomes: BiomeSemantic,
    pub status: String,
    #[serde(default)]
    pub deprecated_aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorSemantic {
    pub group: String,
    pub paint_mode: String,
    pub default_visible: bool,
    pub badge: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipSemantic {
    pub lifecycle: String,
    pub authoritative_system: String,
    pub regeneration_policy: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSemantic {
    pub collision_class: String,
    pub walkable: bool,
    pub water_depth_class: Option<String>,
    pub terrain_role: String,
    pub autotile_topology: String,
    pub animated: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionSemantic {
    pub owns_transitions: bool,
    pub family: String,
    pub normalization_required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSemantic {
    pub allowed: Vec<String>,
    #[serde(rename = "seasonPolicy")]
    pub season_policy: String,
}

impl TerrainSemanticRegistry {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read terrain semantic registry: {e}"))?;
        let registry: Self = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse terrain semantic registry: {e}"))?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TERRAIN_SEMANTIC_REGISTRY_SCHEMA || self.version != 1 {
            return Err("unsupported terrain semantic registry schema/version".into());
        }
        let mut ids = HashMap::new();
        let mut codes = HashMap::new();
        let mut ordinals = HashMap::new();
        for material in &self.materials {
            if ids.insert(&material.id, ()).is_some() {
                return Err(format!("duplicate terrain id {}", material.id));
            }
            if codes.insert(&material.code, ()).is_some() {
                return Err(format!("duplicate terrain code {}", material.code));
            }
            if ordinals.insert(material.stable_ordinal, ()).is_some() {
                return Err(format!(
                    "duplicate stable ordinal {}",
                    material.stable_ordinal
                ));
            }
            if material.ownership.lifecycle == "generated"
                && material.ownership.regeneration_policy != "recompute"
            {
                return Err(format!(
                    "generated terrain {} must use recompute policy",
                    material.code
                ));
            }
        }
        Ok(())
    }

    pub fn by_code(&self, code: &str) -> Option<&TerrainSemantic> {
        self.materials
            .iter()
            .find(|m| m.code == code || m.deprecated_aliases.iter().any(|a| a == code))
    }
}
