use crate::asset_pack::{AssetCategory, AssetPackRegistry, StableAssetRef};
use crate::semantic_asset_resolution::{AssetResolutionContext, SemanticAssetRequest, SemanticAssetResolver};
use haven_core::BuildingDefinition;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingVisualRole {
    ExteriorWall, InteriorWall, ExteriorFloor, InteriorFloor, Roof, Door, Window,
    Foundation, Corner, Stair, Porch, Trim,
}

impl BuildingVisualRole {
    pub const fn category(&self) -> AssetCategory {
        match self {
            Self::ExteriorWall | Self::InteriorWall | Self::Corner | Self::Trim => AssetCategory::Wall,
            Self::ExteriorFloor | Self::InteriorFloor => AssetCategory::Floor,
            Self::Door => AssetCategory::Door,
            Self::Roof | Self::Window | Self::Foundation | Self::Stair | Self::Porch => AssetCategory::Building,
        }
    }
    pub const fn semantic_token(&self) -> &'static str {
        match self {
            Self::ExteriorWall => "wall.exterior", Self::InteriorWall => "wall.interior",
            Self::ExteriorFloor => "floor.exterior", Self::InteriorFloor => "floor.interior",
            Self::Roof => "roof", Self::Door => "door", Self::Window => "window",
            Self::Foundation => "foundation", Self::Corner => "wall.corner", Self::Stair => "stair",
            Self::Porch => "porch", Self::Trim => "wall.trim",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildingVisualRequest {
    pub role: BuildingVisualRole,
    pub style: String,
    #[serde(default)] pub orientation: Option<String>,
    #[serde(default)] pub variant: Option<String>,
    #[serde(default)] pub context: AssetResolutionContext,
}
impl BuildingVisualRequest {
    pub fn semantic_id(&self) -> String {
        let mut id = format!("building.{}.{}", normalize(&self.style), self.role.semantic_token());
        if let Some(v) = self.orientation.as_deref().filter(|v| !v.trim().is_empty()) { id.push('.'); id.push_str(&normalize(v)); }
        if let Some(v) = self.variant.as_deref().filter(|v| !v.trim().is_empty()) { id.push('.'); id.push_str(&normalize(v)); }
        id
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedBuildingVisual { pub semantic_id: String, pub role: BuildingVisualRole, pub asset: StableAssetRef }

pub struct BuildingVisualResolver<'a> { semantic: SemanticAssetResolver<'a> }
impl<'a> BuildingVisualResolver<'a> {
    pub fn new(registry: &'a AssetPackRegistry) -> Self { Self { semantic: SemanticAssetResolver::new(registry) } }
    /// Production resolution only. No missing-piece fallback is synthesized.
    pub fn resolve(&self, request: &BuildingVisualRequest) -> Result<ResolvedBuildingVisual, String> {
        let semantic_id = request.semantic_id();
        let result = self.semantic.resolve(&SemanticAssetRequest { semantic_id: semantic_id.clone(), category: request.role.category(), context: request.context.clone() });
        result.selected.map(|asset| ResolvedBuildingVisual { semantic_id, role: request.role.clone(), asset })
            .ok_or_else(|| result.diagnostics.join("; "))
    }
}

/// Building purpose and dimensions deliberately do not choose a fixed visual prefab.
pub fn request_for_building(_building: &BuildingDefinition, style: impl Into<String>, role: BuildingVisualRole, orientation: Option<&str>) -> BuildingVisualRequest {
    BuildingVisualRequest { role, style: style.into(), orientation: orientation.map(str::to_owned), variant: None, context: AssetResolutionContext::default() }
}
fn normalize(value: &str) -> String { value.trim().to_ascii_lowercase().replace([' ', '-'], "_") }

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{BuildingInstanceId, BuildingLayout, ExteriorLayout, InteriorLayout};
    fn building(w:u32,h:u32)->BuildingDefinition { BuildingDefinition { id: BuildingInstanceId::new("estate.house.test"), archetype:"residence".into(), layout:BuildingLayout { exterior:ExteriorLayout{width:w,height:h,floors:1}, interior:InteriorLayout{width:w+4,height:h+4,floors:vec![]}, transitions:vec![] } } }
    #[test] fn visual_semantics_ignore_dimensions() {
        let a=request_for_building(&building(7,6),"Willowmere Common",BuildingVisualRole::ExteriorWall,Some("south"));
        let b=request_for_building(&building(18,13),"Willowmere Common",BuildingVisualRole::ExteriorWall,Some("south"));
        assert_eq!(a.semantic_id(),"building.willowmere_common.wall.exterior.south"); assert_eq!(a.semantic_id(),b.semantic_id());
    }
    #[test] fn roles_reuse_canonical_categories() { assert_eq!(BuildingVisualRole::ExteriorWall.category(),AssetCategory::Wall); assert_eq!(BuildingVisualRole::Door.category(),AssetCategory::Door); assert_eq!(BuildingVisualRole::Roof.category(),AssetCategory::Building); }
}
