use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TERRAIN_COMPLETION_AUTHORITY_SCHEMA: &str = "havenwild.terrain_completion_authority.v0_1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainCompletionAuthority {
    pub schema: String,
    pub shared_editor_runtime_resolver: bool,
    pub block_capital_generation_until_certified: bool,
    pub water: WaterCompletionContract,
    pub cliffs: CliffCompletionContract,
    pub cave_entrances: CaveEntranceCompletionContract,
    #[serde(default)]
    pub certification_scenarios: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaterCompletionContract {
    pub semantic_freshwater_brush: bool,
    pub derive_shallow_rim: bool,
    pub derive_deep_interior: bool,
    pub recompute_after_edits: bool,
    pub narrow_channels_may_remain_shallow: bool,
    pub support_water_against_cliffs: bool,
    pub support_waterfalls: bool,
    #[serde(default)]
    pub required_neighbor_families: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliffCompletionContract {
    pub explicit_elevation_metadata: bool,
    pub derive_topology_from_neighbors: bool,
    pub support_multilevel_faces: bool,
    pub support_inside_outside_corners: bool,
    pub support_ramps_and_mountain_paths: bool,
    pub support_water_bases: bool,
    #[serde(default)]
    pub required_topologies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaveEntranceCompletionContract {
    pub first_class_mountain_feature: bool,
    pub require_valid_host_face: bool,
    pub require_traversable_approach: bool,
    pub require_destination_binding: bool,
    pub open_collision_at_threshold: bool,
    pub require_editor_validation: bool,
    #[serde(default)]
    pub supported_facings: Vec<String>,
    #[serde(default)]
    pub supported_types: Vec<String>,
}

impl TerrainCompletionAuthority {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TERRAIN_COMPLETION_AUTHORITY_SCHEMA {
            return Err(format!(
                "unsupported terrain-completion schema {}",
                self.schema
            ));
        }
        if !self.shared_editor_runtime_resolver || !self.block_capital_generation_until_certified {
            return Err("terrain completion requires one editor/runtime resolver and a capital-generation gate".to_string());
        }
        self.water.validate()?;
        self.cliffs.validate()?;
        self.cave_entrances.validate()?;
        let scenarios: BTreeSet<_> = self
            .certification_scenarios
            .iter()
            .map(String::as_str)
            .collect();
        for required in [
            "irregular_freshwater_lake",
            "narrow_shallow_river",
            "water_against_cliff",
            "multilevel_cliff",
            "mountain_path_ramp",
            "south_facing_natural_cave",
            "cave_scene_transition",
            "repaired_farmstead",
        ] {
            if !scenarios.contains(required) {
                return Err(format!("missing terrain certification scenario {required}"));
            }
        }
        Ok(())
    }
}

impl WaterCompletionContract {
    fn validate(&self) -> Result<(), String> {
        if !self.semantic_freshwater_brush
            || !self.derive_shallow_rim
            || !self.derive_deep_interior
            || !self.recompute_after_edits
            || !self.support_water_against_cliffs
        {
            return Err("freshwater authority is incomplete".to_string());
        }
        let neighbors: BTreeSet<_> = self
            .required_neighbor_families
            .iter()
            .map(String::as_str)
            .collect();
        for required in ["grass", "sand", "wet_sand", "dirt", "cliff"] {
            if !neighbors.contains(required) {
                return Err(format!(
                    "water contract is missing neighbor family {required}"
                ));
            }
        }
        Ok(())
    }
}

impl CliffCompletionContract {
    fn validate(&self) -> Result<(), String> {
        if !self.explicit_elevation_metadata
            || !self.derive_topology_from_neighbors
            || !self.support_multilevel_faces
            || !self.support_inside_outside_corners
            || !self.support_ramps_and_mountain_paths
        {
            return Err("cliff and elevation authority is incomplete".to_string());
        }
        if self.required_topologies.len() < 8 {
            return Err("cliff contract requires the full topology inventory".to_string());
        }
        Ok(())
    }
}

impl CaveEntranceCompletionContract {
    fn validate(&self) -> Result<(), String> {
        if !self.first_class_mountain_feature
            || !self.require_valid_host_face
            || !self.require_traversable_approach
            || !self.require_destination_binding
            || !self.open_collision_at_threshold
            || !self.require_editor_validation
        {
            return Err("mountain cave entrance authority is incomplete".to_string());
        }
        let facings: BTreeSet<_> = self.supported_facings.iter().map(String::as_str).collect();
        if !facings.contains("south") {
            return Err("south-facing mountain cave entrances are required".to_string());
        }
        let kinds: BTreeSet<_> = self.supported_types.iter().map(String::as_str).collect();
        for required in [
            "natural_cave",
            "mine",
            "ruined_tunnel",
            "sealed_entrance",
            "monster_den",
        ] {
            if !kinds.contains(required) {
                return Err(format!("missing cave entrance type {required}"));
            }
        }
        Ok(())
    }
}

pub fn terrain_ready_for_capital_generation(
    authority: &TerrainCompletionAuthority,
    passed_scenarios: impl IntoIterator<Item = String>,
) -> Result<bool, String> {
    authority.validate()?;
    let passed: BTreeSet<String> = passed_scenarios.into_iter().collect();
    Ok(authority
        .certification_scenarios
        .iter()
        .all(|scenario| passed.contains(scenario)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_certification_blocks_capital_generation() {
        let authority = TerrainCompletionAuthority {
            schema: TERRAIN_COMPLETION_AUTHORITY_SCHEMA.to_string(),
            shared_editor_runtime_resolver: true,
            block_capital_generation_until_certified: true,
            water: WaterCompletionContract {
                semantic_freshwater_brush: true,
                derive_shallow_rim: true,
                derive_deep_interior: true,
                recompute_after_edits: true,
                narrow_channels_may_remain_shallow: true,
                support_water_against_cliffs: true,
                support_waterfalls: true,
                required_neighbor_families: vec!["grass", "sand", "wet_sand", "dirt", "cliff"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            },
            cliffs: CliffCompletionContract {
                explicit_elevation_metadata: true,
                derive_topology_from_neighbors: true,
                support_multilevel_faces: true,
                support_inside_outside_corners: true,
                support_ramps_and_mountain_paths: true,
                support_water_bases: true,
                required_topologies: vec![
                    "north",
                    "south",
                    "east",
                    "west",
                    "inside_corner",
                    "outside_corner",
                    "vertical_stack",
                    "base",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
            },
            cave_entrances: CaveEntranceCompletionContract {
                first_class_mountain_feature: true,
                require_valid_host_face: true,
                require_traversable_approach: true,
                require_destination_binding: true,
                open_collision_at_threshold: true,
                require_editor_validation: true,
                supported_facings: vec!["south".to_string()],
                supported_types: vec![
                    "natural_cave",
                    "mine",
                    "ruined_tunnel",
                    "sealed_entrance",
                    "monster_den",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
            },
            certification_scenarios: vec![
                "irregular_freshwater_lake",
                "narrow_shallow_river",
                "water_against_cliff",
                "multilevel_cliff",
                "mountain_path_ramp",
                "south_facing_natural_cave",
                "cave_scene_transition",
                "repaired_farmstead",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        };
        assert!(!terrain_ready_for_capital_generation(&authority, Vec::<String>::new()).unwrap());
    }
}
