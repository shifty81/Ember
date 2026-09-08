use serde::{Deserialize, Serialize};

/// Stable identity for a placed/authored building. Building purpose is deliberately
/// separate from geometry so two buildings of the same archetype may have unrelated sizes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BuildingInstanceId(String);

impl BuildingInstanceId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into().trim().to_ascii_lowercase().replace(' ', "_")) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingDefinition {
    pub id: BuildingInstanceId,
    pub archetype: String,
    pub layout: BuildingLayout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingLayout {
    pub exterior: ExteriorLayout,
    pub interior: InteriorLayout,
    #[serde(default)]
    pub transitions: Vec<BuildingTransition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExteriorLayout {
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_floor_count")]
    pub floors: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteriorLayout {
    /// Interior scene dimensions are intentionally independent of the exterior footprint.
    /// A compact overworld building may use a larger interior for readable navigation/gameplay.
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub floors: Vec<BuildingFloorLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingFloorLayout {
    pub level: i32,
    #[serde(default)]
    pub rooms: Vec<BuildingRoom>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingRoom {
    pub id: String,
    pub kind: BuildingRoomKind,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingRoomKind {
    Public, Private, Kitchen, Dining, Bar, Bedroom, Storage, Workshop, Shop,
    Service, Hall, Stairwell, Utility, Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingTransition {
    pub id: String,
    pub from: BuildingAnchor,
    pub to: BuildingAnchor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingAnchor {
    pub space: BuildingSpace,
    pub floor: i32,
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingSpace { Exterior, Interior }

const fn default_floor_count() -> u32 { 1 }
