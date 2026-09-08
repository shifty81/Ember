use crate::asset_pack::{AssetCategory, AssetPackRegistry, StableAssetRef};
use crate::semantic_asset_resolution::{
    AssetResolutionContext, AssetResolutionResult, SemanticAssetRequest, SemanticAssetResolver,
};
use crate::{ObjectKind, TileKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeAssetConsumer {
    Pcg,
    F3Editor,
    Renderer,
    ObjectPlacement,
    Character,
    Animation,
    Audio,
    Ui,
    Inventory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAssetRequest {
    pub consumer: RuntimeAssetConsumer,
    pub semantic_id: String,
    pub category: AssetCategory,
    pub context: AssetResolutionContext,
}

impl RuntimeAssetRequest {
    pub fn resolve(&self, registry: &AssetPackRegistry) -> AssetResolutionResult {
        SemanticAssetResolver::new(registry).resolve(&SemanticAssetRequest {
            semantic_id: self.semantic_id.clone(),
            category: self.category.clone(),
            context: self.context.clone(),
        })
    }
}

pub fn terrain_semantic_id(tile: TileKind) -> &'static str {
    match tile {
        TileKind::Grass | TileKind::TallGrass => "terrain.grass",
        TileKind::Sand => "terrain.sand",
        TileKind::WetSand => "terrain.sand.wet",
        TileKind::PebbleShore => "terrain.shore.pebble",
        TileKind::Dirt => "terrain.dirt",
        TileKind::Road => "terrain.path.road",
        TileKind::StonePath => "terrain.path.stone",
        TileKind::MountainPath => "terrain.path.mountain",
        TileKind::Bridge => "terrain.bridge",
        TileKind::Water | TileKind::ShallowWater | TileKind::OceanShallow => {
            "terrain.water.shallow"
        }
        TileKind::DeepWater | TileKind::OceanDeep => "terrain.water.deep",
        TileKind::RiverWater => "terrain.water.river",
        TileKind::RiverMouthBlend => "terrain.water.river_mouth",
        TileKind::ShoreFoam => "terrain.shore.foam",
        TileKind::MudBank => "terrain.shore.mud_bank",
        TileKind::WoodFloor => "terrain.floor.wood",
        TileKind::PlankFloor => "terrain.floor.plank",
        TileKind::StoneFloor => "terrain.floor.stone",
        TileKind::BrickFloor => "terrain.floor.brick",
        TileKind::Wall => "terrain.wall.default",
        TileKind::Cliff => "terrain.cliff",
        TileKind::MountainRock => "terrain.rock.mountain",
        TileKind::CaveFloor => "terrain.cave.floor",
        TileKind::CaveWall => "terrain.cave.wall",
        TileKind::TilledSoil => "terrain.soil.tilled",
        TileKind::WateredSoil => "terrain.soil.watered",
        TileKind::Crop => "terrain.crop",
        TileKind::GreenhouseZone => "terrain.greenhouse",
    }
}

pub fn terrain_request(
    consumer: RuntimeAssetConsumer,
    tile: TileKind,
    context: AssetResolutionContext,
) -> RuntimeAssetRequest {
    RuntimeAssetRequest {
        consumer,
        semantic_id: terrain_semantic_id(tile).to_string(),
        category: AssetCategory::Terrain,
        context,
    }
}

pub fn object_semantic_id(kind: ObjectKind) -> String {
    format!("object.{}", format!("{kind:?}").to_ascii_lowercase())
}

pub fn object_request(kind: ObjectKind, context: AssetResolutionContext) -> RuntimeAssetRequest {
    RuntimeAssetRequest {
        consumer: RuntimeAssetConsumer::ObjectPlacement,
        semantic_id: object_semantic_id(kind),
        category: AssetCategory::TileObject,
        context,
    }
}

pub fn semantic_request(
    consumer: RuntimeAssetConsumer,
    category: AssetCategory,
    semantic_id: impl Into<String>,
    context: AssetResolutionContext,
) -> RuntimeAssetRequest {
    RuntimeAssetRequest {
        consumer,
        semantic_id: semantic_id.into(),
        category,
        context,
    }
}

pub fn selected_ref(
    registry: &AssetPackRegistry,
    request: &RuntimeAssetRequest,
) -> Option<StableAssetRef> {
    request.resolve(registry).selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcg_and_editor_share_identical_terrain_semantics() {
        let context = AssetResolutionContext::default();
        let pcg = terrain_request(RuntimeAssetConsumer::Pcg, TileKind::StonePath, context.clone());
        let editor = terrain_request(RuntimeAssetConsumer::F3Editor, TileKind::StonePath, context);
        assert_eq!(pcg.semantic_id, "terrain.path.stone");
        assert_eq!(pcg.semantic_id, editor.semantic_id);
        assert_eq!(pcg.category, editor.category);
    }

    #[test]
    fn path_materials_do_not_collapse_to_one_semantic_identity() {
        assert_ne!(terrain_semantic_id(TileKind::Road), terrain_semantic_id(TileKind::StonePath));
        assert_ne!(terrain_semantic_id(TileKind::Road), terrain_semantic_id(TileKind::MountainPath));
        assert_ne!(terrain_semantic_id(TileKind::StonePath), terrain_semantic_id(TileKind::MountainPath));
    }
}
