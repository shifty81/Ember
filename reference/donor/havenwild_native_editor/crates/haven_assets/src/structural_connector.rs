use crate::building_recipe::{BuildingConnectorKind, BuildingLevelConnector};
use serde::{Deserialize, Serialize};

/// W51 normalizes traversal semantics without moving visual identity out of
/// PublishedWorldAssetRegistry or placement ownership out of BuildingRecipe/SceneMap.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuralConnectorTraversalMode {
    SameInstanceLevel,
    SceneTransition,
    ContinuousSurface,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuralConnectorKind {
    Stairs,
    Ladder,
    Ramp,
    Doorway,
    CaveMouth,
    CaveDepth,
    Bridge,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructuralConnectorProfile {
    pub id: String,
    pub kind: StructuralConnectorKind,
    pub traversal_mode: StructuralConnectorTraversalMode,
    #[serde(default)]
    pub visual_asset_id: Option<String>,
    #[serde(default)]
    pub level_delta: i32,
    #[serde(default)]
    pub reversible: bool,
    #[serde(default)]
    pub requires_interact: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StructuralConnectorRegistry {
    profiles: Vec<StructuralConnectorProfile>,
}

impl StructuralConnectorRegistry {
    pub fn havenwild_default() -> Self {
        Self {
            profiles: vec![
                profile(
                    "building.stairs.short_run",
                    StructuralConnectorKind::Stairs,
                    StructuralConnectorTraversalMode::SameInstanceLevel,
                    Some("stairs_short_run_gray"),
                    1,
                    true,
                    true,
                    &["building", "same_world"],
                ),
                profile(
                    "building.stairs.short_step",
                    StructuralConnectorKind::Stairs,
                    StructuralConnectorTraversalMode::SameInstanceLevel,
                    Some("stairs_short_step_gray"),
                    1,
                    true,
                    true,
                    &["building", "same_world"],
                ),
                profile(
                    "cave.narrow_mouth",
                    StructuralConnectorKind::CaveMouth,
                    StructuralConnectorTraversalMode::SceneTransition,
                    Some("cave_entrance_default"),
                    0,
                    true,
                    true,
                    &["cave", "surface", "aperture_1x2", "source_envelope_1x3"],
                ),
                profile(
                    "cave.depth_transition",
                    StructuralConnectorKind::CaveDepth,
                    StructuralConnectorTraversalMode::SceneTransition,
                    None,
                    0,
                    true,
                    true,
                    &["cave", "logical_connector", "deferred_exact_source"],
                ),
                profile(
                    "bridge.wood_oak.flat",
                    StructuralConnectorKind::Bridge,
                    StructuralConnectorTraversalMode::ContinuousSurface,
                    Some("bridge_wood_oak_flat_module"),
                    0,
                    true,
                    false,
                    &["bridge", "surface"],
                ),
                profile(
                    "bridge.wood_oak.arch",
                    StructuralConnectorKind::Bridge,
                    StructuralConnectorTraversalMode::ContinuousSurface,
                    Some("bridge_wood_oak_arch_module"),
                    0,
                    true,
                    false,
                    &["bridge", "surface"],
                ),
            ],
        }
    }

    pub fn profiles(&self) -> &[StructuralConnectorProfile] {
        &self.profiles
    }

    pub fn profile(&self, id: &str) -> Option<&StructuralConnectorProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn profile_for_visual(&self, asset_id: &str) -> Option<&StructuralConnectorProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.visual_asset_id.as_deref() == Some(asset_id))
    }

    /// BuildingRecipe remains the placement authority. This adapter only normalizes
    /// traversal semantics and verifies that a connector uses a known profile.
    pub fn profile_for_building_connector(
        &self,
        connector: &BuildingLevelConnector,
    ) -> Option<&StructuralConnectorProfile> {
        let profile = self.profile_for_visual(&connector.asset_id)?;
        let expected = match connector.kind {
            BuildingConnectorKind::Stairs => StructuralConnectorKind::Stairs,
            BuildingConnectorKind::Ladder => StructuralConnectorKind::Ladder,
            BuildingConnectorKind::Ramp => StructuralConnectorKind::Ramp,
            BuildingConnectorKind::Elevator => return None,
        };
        (profile.kind == expected
            && profile.traversal_mode == StructuralConnectorTraversalMode::SameInstanceLevel)
            .then_some(profile)
    }
}

fn profile(
    id: &str,
    kind: StructuralConnectorKind,
    traversal_mode: StructuralConnectorTraversalMode,
    visual_asset_id: Option<&str>,
    level_delta: i32,
    reversible: bool,
    requires_interact: bool,
    tags: &[&str],
) -> StructuralConnectorProfile {
    StructuralConnectorProfile {
        id: id.to_string(),
        kind,
        traversal_mode,
        visual_asset_id: visual_asset_id.map(str::to_string),
        level_delta,
        reversible,
        requires_interact,
        tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
    }
}
