use super::terrain_pattern_v2::TerrainIdV2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainPatternSourceV2 {
    Pcg,
    Editor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainPatternTraceV2 {
    pub source: TerrainPatternSourceV2,
    pub x: i32,
    pub y: i32,
    pub center: TerrainIdV2,
    pub requested_signature: String,
    pub selected_candidate_id: Option<String>,
    pub exact: bool,
    pub rejection_reason: Option<String>,
}

impl TerrainPatternTraceV2 {
    pub fn unsupported(
        source: TerrainPatternSourceV2,
        x: i32,
        y: i32,
        center: TerrainIdV2,
        requested_signature: String,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            source,
            x,
            y,
            center,
            requested_signature,
            selected_candidate_id: None,
            exact: false,
            rejection_reason: Some(reason.into()),
        }
    }
}
