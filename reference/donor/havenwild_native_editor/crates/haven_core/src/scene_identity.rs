use serde::{Deserialize, Serialize};

use crate::{SceneId, SceneKind, MAP_H, MAP_W};

/// Stable, project-owned scene identifier used by editor data, save data, and future
/// create/delete scene workflows. This intentionally coexists with the legacy
/// `SceneId` enum during migration so runtime code can be converted safely.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProjectSceneId(String);

impl ProjectSceneId {
    pub fn new(id: impl Into<String>) -> Self {
        let id = normalize_scene_id(&id.into());
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn code(&self) -> &str {
        self.as_str()
    }

    pub fn label(&self) -> String {
        self.legacy_scene_id()
            .map(|id| id.label().to_string())
            .unwrap_or_else(|| scene_id_display_label(self.as_str()))
    }

    pub fn legacy_scene_id(&self) -> Option<SceneId> {
        SceneId::from_code(self.as_str())
    }

    pub fn is_legacy_seed(&self) -> bool {
        self.legacy_scene_id().is_some()
    }

    pub fn from_legacy_scene_id(id: SceneId) -> Self {
        Self(id.code().to_string())
    }

    /// Construct a project scene ID that is already a canonical structured identity.
    /// Unlike `new`, this preserves namespace separators used by runtime-owned scenes.
    pub fn from_canonical_id(id: impl Into<String>) -> Result<Self, String> {
        let id = id.into();
        validate_canonical_scene_id(&id)?;
        Ok(Self(id))
    }
}

impl std::fmt::Display for ProjectSceneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ProjectSceneId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ProjectSceneId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&ProjectSceneId> for ProjectSceneId {
    fn from(value: &ProjectSceneId) -> Self {
        value.clone()
    }
}

impl From<SceneId> for ProjectSceneId {
    fn from(value: SceneId) -> Self {
        Self::from_legacy_scene_id(value)
    }
}

impl PartialEq<SceneId> for ProjectSceneId {
    fn eq(&self, other: &SceneId) -> bool {
        self.legacy_scene_id() == Some(*other)
    }
}

impl PartialEq<ProjectSceneId> for SceneId {
    fn eq(&self, other: &ProjectSceneId) -> bool {
        other == self
    }
}

/// Serializable scene reference backed by a project-owned identifier.
/// Legacy helpers remain available only for seed-template and migration paths.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SceneReference(ProjectSceneId);

impl SceneReference {
    pub fn new(id: impl Into<ProjectSceneId>) -> Self {
        Self(id.into())
    }

    pub fn from_legacy_scene_id(id: SceneId) -> Self {
        Self(ProjectSceneId::from_legacy_scene_id(id))
    }

    pub fn from_canonical_id(id: impl Into<String>) -> Result<Self, String> {
        ProjectSceneId::from_canonical_id(id).map(Self)
    }

    pub fn project_id(&self) -> &ProjectSceneId {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn code(&self) -> &str {
        self.as_str()
    }

    pub fn legacy_scene_id(&self) -> Option<SceneId> {
        SceneId::from_code(self.as_str())
    }

    pub fn resolve_legacy_scene_id(&self, records: &[SceneIdentityRecord]) -> Option<SceneId> {
        self.legacy_scene_id().or_else(|| {
            records
                .iter()
                .find(|record| record.id.as_str() == self.as_str())
                .and_then(|record| record.legacy_scene_code.as_deref())
                .and_then(SceneId::from_code)
        })
    }

    pub fn resolve_record<'a>(
        &self,
        records: &'a [SceneIdentityRecord],
    ) -> Option<&'a SceneIdentityRecord> {
        records
            .iter()
            .find(|record| record.id.as_str() == self.as_str())
    }

    pub fn label(&self) -> String {
        self.legacy_scene_id()
            .map(|id| id.label().to_string())
            .unwrap_or_else(|| scene_id_display_label(self.as_str()))
    }

    pub fn is_legacy_seed(&self) -> bool {
        self.legacy_scene_id().is_some()
    }
}

impl std::fmt::Display for SceneReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<SceneId> for SceneReference {
    fn from(value: SceneId) -> Self {
        Self::from_legacy_scene_id(value)
    }
}

impl From<ProjectSceneId> for SceneReference {
    fn from(value: ProjectSceneId) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SceneReference {
    fn from(value: &str) -> Self {
        Self::new(ProjectSceneId::from(value))
    }
}

impl From<String> for SceneReference {
    fn from(value: String) -> Self {
        Self::new(ProjectSceneId::from(value))
    }
}

impl PartialEq<SceneId> for SceneReference {
    fn eq(&self, other: &SceneId) -> bool {
        self.legacy_scene_id() == Some(*other)
    }
}

impl PartialEq<SceneReference> for SceneId {
    fn eq(&self, other: &SceneReference) -> bool {
        other == self
    }
}

impl PartialEq<ProjectSceneId> for SceneReference {
    fn eq(&self, other: &ProjectSceneId) -> bool {
        self.project_id() == other
    }
}

impl PartialEq<SceneReference> for ProjectSceneId {
    fn eq(&self, other: &SceneReference) -> bool {
        other == self
    }
}

fn scene_id_display_label(id: &str) -> String {
    let mut label = String::new();
    for (index, word) in id.split('_').filter(|word| !word.is_empty()).enumerate() {
        if index > 0 {
            label.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            label.push(first.to_ascii_uppercase());
            label.extend(chars);
        }
    }
    if label.is_empty() {
        "Scene".to_string()
    } else {
        label
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneSurfaceRole {
    OverworldSurface,
    InteriorBank,
    CaveBank,
    SpecialBank,
}

impl SceneSurfaceRole {
    pub fn code(self) -> &'static str {
        match self {
            SceneSurfaceRole::OverworldSurface => "overworld_surface",
            SceneSurfaceRole::InteriorBank => "interior_bank",
            SceneSurfaceRole::CaveBank => "cave_bank",
            SceneSurfaceRole::SpecialBank => "special_bank",
        }
    }

    pub fn from_kind(kind: SceneKind) -> Self {
        match kind {
            SceneKind::Exterior => SceneSurfaceRole::OverworldSurface,
            SceneKind::Interior => SceneSurfaceRole::InteriorBank,
            SceneKind::Cave => SceneSurfaceRole::CaveBank,
        }
    }

    pub fn is_world_canvas_surface(self) -> bool {
        self == SceneSurfaceRole::OverworldSurface
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneCanvasPlacement {
    pub canvas_x: i32,
    pub canvas_y: i32,
    pub canvas_w_tiles: i32,
    pub canvas_h_tiles: i32,
    pub role: SceneSurfaceRole,
}

impl SceneCanvasPlacement {
    pub fn overworld(
        canvas_x: i32,
        canvas_y: i32,
        canvas_w_tiles: i32,
        canvas_h_tiles: i32,
    ) -> Self {
        Self {
            canvas_x,
            canvas_y,
            canvas_w_tiles,
            canvas_h_tiles,
            role: SceneSurfaceRole::OverworldSurface,
        }
    }

    pub fn bank(canvas_x: i32, canvas_y: i32, role: SceneSurfaceRole) -> Self {
        Self {
            canvas_x,
            canvas_y,
            canvas_w_tiles: MAP_W as i32,
            canvas_h_tiles: MAP_H as i32,
            role,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneIdentityRecord {
    pub id: ProjectSceneId,
    pub display_name: String,
    pub legacy_scene_code: Option<String>,
    pub kind_code: String,
    pub placement: SceneCanvasPlacement,
    pub can_delete: bool,
    pub can_resize: bool,
    pub can_move_on_canvas: bool,
    pub creation_locked_reason: Option<String>,
}

impl SceneIdentityRecord {
    pub fn from_legacy(
        legacy: SceneId,
        kind: SceneKind,
        placement: SceneCanvasPlacement,
        can_delete: bool,
    ) -> Self {
        Self {
            id: ProjectSceneId::from_legacy_scene_id(legacy),
            display_name: legacy.label().to_string(),
            legacy_scene_code: Some(legacy.code().to_string()),
            kind_code: kind.code().to_string(),
            placement,
            can_delete,
            can_resize: true,
            can_move_on_canvas: true,
            creation_locked_reason: if can_delete {
                None
            } else {
                Some(
                    "Seed scene is protected until save/transition migration is complete."
                        .to_string(),
                )
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneIdentityMigrationPlan {
    pub schema: String,
    pub current_stage: String,
    pub legacy_enum_backed: bool,
    pub supports_arbitrary_scene_creation: bool,
    pub supports_arbitrary_scene_deletion: bool,
    pub target_scene_id_type: String,
    pub bridge_reference_type: String,
    pub transition_targets_migrated: bool,
    pub active_scene_reference_migrated: bool,
    pub scene_map_identity_migrated: bool,
    pub default_scene_tiles: [usize; 2],
    pub target_large_scene_tiles: [usize; 2],
    pub first_island_canvas_tiles: [usize; 2],
    pub records: Vec<SceneIdentityRecord>,
}

impl SceneIdentityMigrationPlan {
    pub fn default_seed() -> Self {
        Self {
            schema: "haven.scene_identity_migration.v0_1".to_string(),
            current_stage: "scene_registry_foundation_normalization".to_string(),
            legacy_enum_backed: false,
            supports_arbitrary_scene_creation: false,
            supports_arbitrary_scene_deletion: false,
            target_scene_id_type: "ProjectSceneId".to_string(),
            bridge_reference_type: "SceneReference".to_string(),
            transition_targets_migrated: true,
            active_scene_reference_migrated: true,
            scene_map_identity_migrated: true,
            default_scene_tiles: [MAP_W, MAP_H],
            target_large_scene_tiles: [144, 96],
            first_island_canvas_tiles: [768, 512],
            records: default_scene_identity_records(),
        }
    }
}

pub fn default_scene_identity_records() -> Vec<SceneIdentityRecord> {
    vec![
        SceneIdentityRecord::from_legacy(
            SceneId::Farmstead,
            SceneKind::Exterior,
            SceneCanvasPlacement::overworld(144, 112, 96, 64),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::NorthRoad,
            SceneKind::Exterior,
            SceneCanvasPlacement::overworld(96, 96, 96, 48),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::SouthField,
            SceneKind::Exterior,
            SceneCanvasPlacement::overworld(144, 176, 96, 64),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::EastWoods,
            SceneKind::Exterior,
            SceneCanvasPlacement::overworld(240, 112, 96, 96),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::CaveMouth,
            SceneKind::Cave,
            SceneCanvasPlacement::bank(416, 32, SceneSurfaceRole::CaveBank),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::CaveDepths,
            SceneKind::Cave,
            SceneCanvasPlacement::bank(416, 104, SceneSurfaceRole::CaveBank),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::TavernInterior,
            SceneKind::Interior,
            SceneCanvasPlacement::bank(416, 192, SceneSurfaceRole::InteriorBank),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::Cellar,
            SceneKind::Cave,
            SceneCanvasPlacement::bank(416, 264, SceneSurfaceRole::InteriorBank),
            false,
        ),
        SceneIdentityRecord::from_legacy(
            SceneId::GuestFloor,
            SceneKind::Interior,
            SceneCanvasPlacement::bank(416, 336, SceneSurfaceRole::InteriorBank),
            false,
        ),
    ]
}


pub fn validate_canonical_scene_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("canonical scene id cannot be empty".to_string());
    }
    if raw.trim() != raw {
        return Err("canonical scene id cannot contain leading or trailing whitespace".to_string());
    }
    if raw.contains('/') || raw.contains('\\') || raw.contains("..") {
        return Err("canonical scene id cannot contain path syntax".to_string());
    }
    if raw.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err("canonical scene id cannot contain whitespace or control characters".to_string());
    }
    if !raw.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '.' | '_' | '-')) {
        return Err("canonical scene id contains an unsupported character".to_string());
    }
    if raw.starts_with(':') || raw.ends_with(':') || raw.split(':').any(str::is_empty) {
        return Err("canonical scene id contains an empty namespace component".to_string());
    }
    Ok(())
}

pub fn normalize_scene_id(raw: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;
    for ch in raw.trim().chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch == '_' || ch == '-' || ch.is_whitespace() {
            Some('_')
        } else {
            None
        };
        if let Some(ch) = normalized {
            if ch == '_' {
                if !last_was_underscore && !out.is_empty() {
                    out.push('_');
                }
                last_was_underscore = true;
            } else {
                out.push(ch);
                last_was_underscore = false;
            }
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "scene".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_reference_round_trips_legacy_seed_ids() {
        let reference = SceneReference::from(SceneId::TavernInterior);
        assert_eq!(reference.code(), "tavern_interior");
        assert_eq!(reference.legacy_scene_id(), Some(SceneId::TavernInterior));
        assert_eq!(reference.label(), "Tavern Interior");
    }

    #[test]
    fn scene_reference_retains_arbitrary_project_ids() {
        let reference = SceneReference::from("Harbor Annex 02");
        assert_eq!(reference.code(), "harbor_annex_02");
        assert_eq!(reference.legacy_scene_id(), None);
        assert_eq!(reference.label(), "Harbor Annex 02");
        let json = serde_json::to_string(&reference).expect("serialize scene reference");
        let decoded: SceneReference =
            serde_json::from_str(&json).expect("deserialize scene reference");
        assert_eq!(decoded, reference);
    }

    #[test]
    fn canonical_scene_ids_preserve_runtime_namespace_structure() {
        let reference = SceneReference::from_canonical_id(
            "building:willowmere.tavern.001:interior:floor:0",
        )
        .expect("valid canonical scene id");
        assert_eq!(
            reference.code(),
            "building:willowmere.tavern.001:interior:floor:0"
        );
        let json = serde_json::to_string(&reference).expect("serialize canonical scene reference");
        let decoded: SceneReference =
            serde_json::from_str(&json).expect("deserialize canonical scene reference");
        assert_eq!(decoded, reference);
    }

    #[test]
    fn canonical_scene_ids_reject_path_and_empty_namespace_syntax() {
        for invalid in [
            "",
            "building::interior",
            "building:../interior",
            "building:foo\\bar",
            "building:foo bar:interior",
        ] {
            assert!(ProjectSceneId::from_canonical_id(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn scene_reference_can_resolve_identity_record_aliases() {
        let mut record = default_scene_identity_records()
            .into_iter()
            .find(|record| record.legacy_scene_code.as_deref() == Some("farmstead"))
            .expect("farmstead identity record");
        record.id = ProjectSceneId::new("starter_homestead");
        let reference = SceneReference::from("starter_homestead");
        assert_eq!(
            reference.resolve_legacy_scene_id(&[record]),
            Some(SceneId::Farmstead)
        );
    }
}
