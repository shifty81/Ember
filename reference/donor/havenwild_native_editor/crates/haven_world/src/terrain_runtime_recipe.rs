//! Canonical runtime/editor bridge for one semantic surface cell.
//!
//! Pass167Z109R deliberately does not replace Havenwild's existing terrain
//! providers or structural resolver. It packages their already-authoritative
//! outputs behind one small recipe so maps, collision, presentation caches and
//! authoring diagnostics stop independently re-interpreting `TileKind` and
//! structural levels while visual output remains unchanged.

use haven_core::{
    base_terrain, collision_class, visual_overlay, BaseTerrain, CollisionClass, SceneBiome,
    TavernMap, TerrainOverlayKind, TileAutoGroup, TileKind,
};

use crate::{
    resolve_material, resolve_shape, traversal_from_structural_cell_v2, CardinalDirectionV2,
    CliffShape15, StructuralCellV2, TerrainMaterialId, TerrainShape,
};

pub const SURFACE_TERRAIN_RECIPE_V1_SCHEMA: &str = "havenwild.surface_terrain_recipe.v1";

/// Small retained presentation record used by the existing terrain cache.
/// Shape and material remain independent so the V7 shape authority can be
/// reskinned by a compatible visual provider without changing semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfacePresentationRecipeV1 {
    pub shape: TerrainShape,
    pub material: TerrainMaterialId,
}

/// Semantic role used by the world map/minimap. The numeric map encoding is
/// kept here for save compatibility with the existing exploration snapshots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainMapRoleV1 {
    DeepWater,
    ShallowWater,
    Shore,
    Land,
    Upland,
    Highland,
    Cliff,
    Road,
    River,
    Bridge,
    Path,
}

impl TerrainMapRoleV1 {
    pub const fn exploration_code(self) -> u8 {
        match self {
            Self::DeepWater => 0,
            Self::ShallowWater => 1,
            Self::Shore => 2,
            Self::Land => 3,
            Self::Upland => 4,
            Self::Highland => 5,
            Self::Cliff => 6,
            Self::Road => 7,
            Self::River => 8,
            Self::Bridge => 9,
            Self::Path => 10,
        }
    }
}

/// Canonical semantic/structural recipe for one surface cell.
///
/// This intentionally stores no source atlas coordinates or provider-specific
/// sprite IDs. Those remain presentation concerns. Structural collision and
/// cliff topology are copied from the full-world structural bake rather than
/// inferred from the visible tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceTerrainRecipeV1 {
    pub tile: TileKind,
    pub base: BaseTerrain,
    pub collision: CollisionClass,
    pub overlay: Option<TerrainOverlayKind>,
    pub material: TerrainMaterialId,
    pub structural_level: u8,
    pub structural: Option<StructuralCellV2>,
    pub cliff_shape: Option<CliffShape15>,
    pub map_role: TerrainMapRoleV1,
}

impl SurfaceTerrainRecipeV1 {
    pub const fn map_code(self) -> u8 {
        self.map_role.exploration_code()
    }

    pub const fn has_structural_cliff(self) -> bool {
        self.cliff_shape.is_some()
    }
}

/// Resolve the presentation fields already consumed by the retained runtime
/// terrain cache. This is behavior-equivalent to calling `resolve_shape` and
/// `resolve_material` separately, but gives runtime/editor callers one shared
/// presentation bridge.
pub fn resolve_surface_presentation_recipe_v1(
    tile: TileKind,
    biome: SceneBiome,
    group: Option<TileAutoGroup>,
    mask: u8,
) -> SurfacePresentationRecipeV1 {
    SurfacePresentationRecipeV1 {
        shape: resolve_shape(group, mask),
        material: resolve_material(tile, biome),
    }
}

/// Persisted structural level authority with the same legacy compatibility
/// behavior used by the runtime map before Pass167Z109R.
pub fn structural_level_for_surface_recipe_v1(map: &TavernMap, x: i32, y: i32) -> u8 {
    map.get_structural_level(x, y).unwrap_or_else(|| {
        u8::from(matches!(
            map.get(x, y),
            TileKind::MountainRock | TileKind::Cliff | TileKind::CaveWall
        ))
    })
}

pub fn resolve_surface_terrain_recipe_v1(
    map: &TavernMap,
    biome: SceneBiome,
    x: i32,
    y: i32,
    structural: Option<StructuralCellV2>,
) -> Option<SurfaceTerrainRecipeV1> {
    TavernMap::idx(x, y)?;
    let tile = map.get(x, y);
    let structural_level = structural_level_for_surface_recipe_v1(map, x, y);
    let cliff_shape = structural.and_then(StructuralCellV2::cliff_shape_15);
    let map_role = resolve_terrain_map_role_v1(tile, structural_level, cliff_shape);

    Some(SurfaceTerrainRecipeV1 {
        tile,
        base: base_terrain(tile),
        collision: collision_class(tile),
        overlay: visual_overlay(tile),
        material: resolve_material(tile, biome),
        structural_level,
        structural,
        cliff_shape,
        map_role,
    })
}

/// Preserve the existing map priority exactly while making the decision shared
/// and testable. Routes/bridges/rivers win over structural presentation; baked
/// cliff topology wins over the persisted level color.
pub const fn resolve_terrain_map_role_v1(
    tile: TileKind,
    structural_level: u8,
    cliff_shape: Option<CliffShape15>,
) -> TerrainMapRoleV1 {
    if matches!(tile, TileKind::Road) {
        return TerrainMapRoleV1::Road;
    }
    if matches!(tile, TileKind::Bridge) {
        return TerrainMapRoleV1::Bridge;
    }
    if matches!(
        tile,
        TileKind::StonePath | TileKind::MountainPath | TileKind::Dirt
    ) {
        return TerrainMapRoleV1::Path;
    }
    if matches!(tile, TileKind::RiverWater | TileKind::RiverMouthBlend) {
        return TerrainMapRoleV1::River;
    }
    if cliff_shape.is_some() {
        return TerrainMapRoleV1::Cliff;
    }
    if structural_level >= 2 {
        return TerrainMapRoleV1::Highland;
    }
    if structural_level == 1 {
        return TerrainMapRoleV1::Upland;
    }

    match tile {
        TileKind::OceanDeep | TileKind::DeepWater => TerrainMapRoleV1::DeepWater,
        TileKind::OceanShallow | TileKind::ShallowWater | TileKind::Water => {
            TerrainMapRoleV1::ShallowWater
        }
        TileKind::Sand | TileKind::WetSand | TileKind::PebbleShore => TerrainMapRoleV1::Shore,
        TileKind::MountainRock | TileKind::Cliff | TileKind::CaveWall => {
            TerrainMapRoleV1::Highland
        }
        _ => TerrainMapRoleV1::Land,
    }
}

/// Canonical two-cell structural traversal decision. Projected south-face
/// occupancy and explicit connector corridors remain separate because they are
/// multi-cell feature-footprint queries, but callers no longer duplicate the
/// two-sided edge test itself.
pub fn surface_structural_move_blocked_v1(
    source: Option<StructuralCellV2>,
    destination: Option<StructuralCellV2>,
    direction: CardinalDirectionV2,
) -> bool {
    let source_query = source.map(|cell| traversal_from_structural_cell_v2(cell, direction));
    let destination_query = destination
        .map(|cell| traversal_from_structural_cell_v2(cell, direction.opposite()));
    source_query.is_some_and(|query| query.blocked || query.elevation_drop > 0)
        || destination_query.is_some_and(|query| query.blocked || query.elevation_drop > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EdgeMaskV2;

    #[test]
    fn map_role_keeps_route_and_cliff_priority() {
        assert_eq!(
            resolve_terrain_map_role_v1(TileKind::Road, 2, Some(CliffShape15::South)),
            TerrainMapRoleV1::Road
        );
        assert_eq!(
            resolve_terrain_map_role_v1(TileKind::Grass, 1, Some(CliffShape15::South)),
            TerrainMapRoleV1::Cliff
        );
        assert_eq!(
            resolve_terrain_map_role_v1(TileKind::Grass, 2, None),
            TerrainMapRoleV1::Highland
        );
    }

    #[test]
    fn recipe_uses_baked_structural_shape_without_rewriting_surface_tile() {
        let map = TavernMap::empty_with(TileKind::Grass);
        let structural = StructuralCellV2 {
            exposed_edges: EdgeMaskV2(EdgeMaskV2::SOUTH),
            ..StructuralCellV2::default()
        };
        let recipe = resolve_surface_terrain_recipe_v1(
            &map,
            SceneBiome::Temperate,
            4,
            4,
            Some(structural),
        )
        .expect("recipe");
        assert_eq!(recipe.tile, TileKind::Grass);
        assert_eq!(recipe.cliff_shape, Some(CliffShape15::South));
        assert_eq!(recipe.map_role, TerrainMapRoleV1::Cliff);
    }

    #[test]
    fn structural_move_blocking_is_two_sided() {
        let source = StructuralCellV2 {
            exposed_edges: EdgeMaskV2(EdgeMaskV2::SOUTH),
            edge_deltas: [0, 0, 2, 0],
            ..StructuralCellV2::default()
        };
        assert!(surface_structural_move_blocked_v1(
            Some(source),
            None,
            CardinalDirectionV2::South
        ));
    }

    #[test]
    fn presentation_bridge_keeps_shape_and_material_resolution() {
        let presentation = resolve_surface_presentation_recipe_v1(
            TileKind::Sand,
            SceneBiome::Coastal,
            Some(TileAutoGroup::Road),
            0x03,
        );
        assert_eq!(presentation.shape, TerrainShape::OuterNorthEast);
        assert_eq!(presentation.material, TerrainMaterialId::SandDry);
    }
}
