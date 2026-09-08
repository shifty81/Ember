//! Shared semantic terrain resolution consumed by runtime and editor paths.

use haven_core::{
    base_terrain, collision_class, visual_overlay, BaseTerrain, CollisionClass, TavernMap,
    TerrainOverlayKind, TileKind,
};

use crate::autotile::{resolve_terrain_transitions, ResolvedTerrainTransitions, TerrainFamily};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTerrainCell {
    pub tile: TileKind,
    pub base: BaseTerrain,
    pub family: TerrainFamily,
    pub collision: CollisionClass,
    pub overlay: Option<TerrainOverlayKind>,
    pub transitions: ResolvedTerrainTransitions,
}

impl ResolvedTerrainCell {
    pub fn is_walkable(&self) -> bool {
        self.collision == CollisionClass::WalkableGround
    }
}

pub fn resolve_terrain_cell(map: &TavernMap, x: i32, y: i32) -> Option<ResolvedTerrainCell> {
    TavernMap::idx(x, y)?;
    let tile = map.get(x, y);
    Some(ResolvedTerrainCell {
        tile,
        base: base_terrain(tile),
        family: TerrainFamily::from_tile(tile),
        collision: collision_class(tile),
        overlay: visual_overlay(tile),
        transitions: resolve_terrain_transitions(map, x, y),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_grass_transition_is_visual_and_both_cells_remain_walkable() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(5, 4, TileKind::Sand);

        let grass = resolve_terrain_cell(&map, 4, 4).expect("grass cell");
        let sand = resolve_terrain_cell(&map, 5, 4).expect("sand cell");
        assert!(grass.transitions.has_any());
        assert!(grass.is_walkable());
        assert!(sand.is_walkable());
    }

    #[test]
    fn legacy_wet_sand_resolves_to_semantic_sand() {
        let map = TavernMap::empty_with(TileKind::WetSand);
        let resolved = resolve_terrain_cell(&map, 4, 4).expect("wet sand cell");
        assert_eq!(resolved.base, BaseTerrain::Sand);
        assert_eq!(resolved.overlay, Some(TerrainOverlayKind::WetSand));
        assert!(resolved.is_walkable());
    }
}
