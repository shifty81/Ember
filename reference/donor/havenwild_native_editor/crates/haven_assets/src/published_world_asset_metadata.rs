use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PublishedWorldAssetSeasonalSource {
    pub season: String,
    pub source: String,
    #[serde(rename = "sourceRect")]
    pub source_rect: [i32; 4],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PublishedWorldAssetSourceLayer {
    pub id: String,
    pub source: String,
    #[serde(rename = "sourceRect")]
    pub source_rect: [i32; 4],
    #[serde(default)]
    pub blend: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PublishedWorldAssetProvenance {
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub source_rect: Option<[i32; 4]>,
    #[serde(default)]
    pub source_mode: Option<String>,
    #[serde(default)]
    pub license_authority: Option<String>,
    #[serde(default)]
    pub seasonal_sources: Vec<PublishedWorldAssetSeasonalSource>,
    #[serde(default)]
    pub source_layers: Vec<PublishedWorldAssetSourceLayer>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PublishedStructureSocket {
    pub id: String,
    pub direction: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub offset: [i32; 2],
    #[serde(default)]
    pub compatible_with: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PublishedStructureDefinition {
    pub connection_family: String,
    pub topology_role: String,
    #[serde(default)]
    pub facing: Option<String>,
    #[serde(default)]
    pub level_delta: i32,
    #[serde(default)]
    pub reversible: bool,
    #[serde(default)]
    pub wall_family: Option<String>,
    #[serde(default)]
    pub attachment_surface: Option<String>,
    /// Camera/presentation visibility role used by building cutaway and active-level views.
    #[serde(default)]
    pub visibility_role: Option<String>,
    /// Groups components that should be occluded/revealed together for one client camera.
    #[serde(default)]
    pub occlusion_group: Option<String>,
    /// True when visibility is presentation-only and must never mutate shared world state.
    #[serde(default)]
    pub camera_local_occlusion: bool,
    /// Optional repeat axis for trim/edge components (for example "x" or "y").
    #[serde(default)]
    pub repeat_axis: Option<String>,
    /// Optional structural assembly family used by BuildingRecipe resolvers.
    #[serde(default)]
    pub assembly_family: Option<String>,
    #[serde(default)]
    pub sockets: Vec<PublishedStructureSocket>,
}
