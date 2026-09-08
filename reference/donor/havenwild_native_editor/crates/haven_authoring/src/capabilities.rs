use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Stable capability IDs shared by every Havenwild authoring frontend.
///
/// Frontends decide which capabilities they expose; the canonical operation
/// implementation remains in shared domain/authoring crates rather than in a UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuthoringCapability {
    InspectWorld,
    PaintTerrain,
    ReplaceTerrain,
    ModifyWater,
    SetStructuralLevel,
    PlaceObject,
    DeleteObject,
    PlaceStructure,
    DeleteStructure,
    CreateZone,
    ModifyRoads,
    PlaceNpc,
    ModifyWorldSettings,
    RegenerateWorld,
    LiveReload,
    RuntimeDiagnostics,
    DebugSpawn,
    Teleport,
    ModifyAssetSource,
    ModifyAssetMetadata,
    ModifyCollisionRecipe,
    ModifyStructuralRecipe,
}

impl AuthoringCapability {
    pub const ALL: [Self; 22] = [
        Self::InspectWorld,
        Self::PaintTerrain,
        Self::ReplaceTerrain,
        Self::ModifyWater,
        Self::SetStructuralLevel,
        Self::PlaceObject,
        Self::DeleteObject,
        Self::PlaceStructure,
        Self::DeleteStructure,
        Self::CreateZone,
        Self::ModifyRoads,
        Self::PlaceNpc,
        Self::ModifyWorldSettings,
        Self::RegenerateWorld,
        Self::LiveReload,
        Self::RuntimeDiagnostics,
        Self::DebugSpawn,
        Self::Teleport,
        Self::ModifyAssetSource,
        Self::ModifyAssetMetadata,
        Self::ModifyCollisionRecipe,
        Self::ModifyStructuralRecipe,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::InspectWorld => "world.inspect",
            Self::PaintTerrain => "world.terrain.paint",
            Self::ReplaceTerrain => "world.terrain.replace",
            Self::ModifyWater => "world.water.modify",
            Self::SetStructuralLevel => "world.structure.level",
            Self::PlaceObject => "world.object.place",
            Self::DeleteObject => "world.object.delete",
            Self::PlaceStructure => "world.structure.place",
            Self::DeleteStructure => "world.structure.delete",
            Self::CreateZone => "world.zone.create",
            Self::ModifyRoads => "world.road.modify",
            Self::PlaceNpc => "world.npc.place",
            Self::ModifyWorldSettings => "world.settings.modify",
            Self::RegenerateWorld => "world.pcg.regenerate",
            Self::LiveReload => "runtime.live_reload",
            Self::RuntimeDiagnostics => "runtime.diagnostics",
            Self::DebugSpawn => "runtime.debug_spawn",
            Self::Teleport => "runtime.teleport",
            Self::ModifyAssetSource => "asset.source.modify",
            Self::ModifyAssetMetadata => "asset.metadata.modify",
            Self::ModifyCollisionRecipe => "developer.collision.modify",
            Self::ModifyStructuralRecipe => "developer.recipe.modify",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthoringFrontendKind {
    NativeDeveloperEditor,
    PlayerWorldBuilder,
    DeveloperOverlay,
    Automation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringProfile {
    pub kind: AuthoringFrontendKind,
    capabilities: BTreeSet<AuthoringCapability>,
}

impl AuthoringProfile {
    pub fn native_developer_editor() -> Self {
        Self::new(
            AuthoringFrontendKind::NativeDeveloperEditor,
            AuthoringCapability::ALL,
        )
    }

    pub fn player_world_builder() -> Self {
        Self::new(
            AuthoringFrontendKind::PlayerWorldBuilder,
            [
                AuthoringCapability::InspectWorld,
                AuthoringCapability::PaintTerrain,
                AuthoringCapability::ReplaceTerrain,
                AuthoringCapability::ModifyWater,
                AuthoringCapability::SetStructuralLevel,
                AuthoringCapability::PlaceObject,
                AuthoringCapability::DeleteObject,
                AuthoringCapability::PlaceStructure,
                AuthoringCapability::DeleteStructure,
                AuthoringCapability::CreateZone,
                AuthoringCapability::ModifyRoads,
                AuthoringCapability::PlaceNpc,
                AuthoringCapability::ModifyWorldSettings,
            ],
        )
    }

    pub fn developer_overlay() -> Self {
        Self::new(
            AuthoringFrontendKind::DeveloperOverlay,
            [
                AuthoringCapability::InspectWorld,
                AuthoringCapability::RuntimeDiagnostics,
                AuthoringCapability::LiveReload,
                AuthoringCapability::DebugSpawn,
                AuthoringCapability::Teleport,
            ],
        )
    }

    pub fn automation() -> Self {
        Self::new(
            AuthoringFrontendKind::Automation,
            [
                AuthoringCapability::InspectWorld,
                AuthoringCapability::PaintTerrain,
                AuthoringCapability::ReplaceTerrain,
                AuthoringCapability::ModifyWater,
                AuthoringCapability::SetStructuralLevel,
                AuthoringCapability::PlaceObject,
                AuthoringCapability::DeleteObject,
                AuthoringCapability::PlaceStructure,
                AuthoringCapability::DeleteStructure,
                AuthoringCapability::CreateZone,
                AuthoringCapability::ModifyRoads,
                AuthoringCapability::PlaceNpc,
                AuthoringCapability::ModifyWorldSettings,
                AuthoringCapability::RegenerateWorld,
            ],
        )
    }

    pub fn new(
        kind: AuthoringFrontendKind,
        capabilities: impl IntoIterator<Item = AuthoringCapability>,
    ) -> Self {
        Self {
            kind,
            capabilities: capabilities.into_iter().collect(),
        }
    }

    pub fn allows(&self, capability: AuthoringCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn capability_ids(&self) -> Vec<&'static str> {
        self.capabilities
            .iter()
            .map(|capability| capability.id())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_world_builder_cannot_modify_developer_asset_authority() {
        let profile = AuthoringProfile::player_world_builder();
        assert!(profile.allows(AuthoringCapability::PaintTerrain));
        assert!(profile.allows(AuthoringCapability::SetStructuralLevel));
        assert!(!profile.allows(AuthoringCapability::ModifyAssetSource));
        assert!(!profile.allows(AuthoringCapability::ModifyCollisionRecipe));
    }

    #[test]
    fn developer_overlay_is_diagnostics_not_player_world_builder() {
        let profile = AuthoringProfile::developer_overlay();
        assert!(profile.allows(AuthoringCapability::RuntimeDiagnostics));
        assert!(profile.allows(AuthoringCapability::LiveReload));
        assert!(!profile.allows(AuthoringCapability::PaintTerrain));
        assert!(!profile.allows(AuthoringCapability::PlaceStructure));
    }
}
