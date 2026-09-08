//! Compatibility bridge between legacy `TavernMap` elevation storage and the
//! canonical semantic elevation/cliff resolver.
//!
//! Legacy scenes continue to persist `TileKind` plus byte heights during the
//! migration. This bridge converts them to `SurfaceCellV1`, executes the same
//! structural resolver used by world generation, and returns derived cliff,
//! ramp, collision, waterfall, and cave-host data without rewriting horizontal
//! surface materials into visual cliff tiles.

use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{
    resolve_elevation_cliffs_v2, tavern_map_to_surface_cells_v1, ElevationCliffResolveReportV2,
    ElevationCliffSettingsV2, ElevationGridSpecV2, StructuralCellV2, SurfaceCellV1,
};

pub const LEGACY_CLIFF_BRIDGE_V2_SCHEMA: &str = "havenwild.legacy_cliff_bridge.v2";

/// One structural platform level equals one resolver step. Raw generated
/// height remains a smooth 0-255 simulation field for hydrology/material
/// decisions and never creates traversal tiers.
pub const STRUCTURAL_ELEVATION_RESOLVER_STEP_V2: i16 = 2;

/// Compatibility fallback for old/AUTO maps that do not yet persist explicit
/// structural levels. Only the legacy MountainRock material represents one
/// raised platform. MountainPath deliberately stays at Level 0 because paths
/// are connector/road semantics, not elevation authority. Fresh PCG worlds and
/// editor-authored worlds persist `structural_levels` and bypass this fallback.
pub fn structural_elevation_for_tile_v2(tile: TileKind, _raw_height: u8) -> i16 {
    if tile == TileKind::MountainRock {
        STRUCTURAL_ELEVATION_RESOLVER_STEP_V2
    } else {
        0
    }
}

/// Converts a persisted TavernMap into the structural-elevation domain. This
/// is intentionally separate from `tavern_map_to_surface_cells_v1`, which is
/// still the raw-height adapter used by hydrology and other simulation lanes.
pub fn tavern_map_to_structural_surface_cells_v2(map: &TavernMap) -> Vec<SurfaceCellV1> {
    let mut cells = tavern_map_to_surface_cells_v1(map);
    for (index, cell) in cells.iter_mut().enumerate() {
        let tile = map.tiles.get(index).copied().unwrap_or(TileKind::Grass);
        let raw_height = map.heights.get(index).copied().unwrap_or_default();
        let authored_level = map
            .structural_levels
            .get(index)
            .copied()
            .filter(|level| *level <= haven_core::MAX_STRUCTURAL_LEVEL);
        cell.elevation = authored_level.map_or_else(
            || structural_elevation_for_tile_v2(tile, raw_height),
            |level| i16::from(level) * STRUCTURAL_ELEVATION_RESOLVER_STEP_V2,
        );
    }
    cells
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyCliffBridgeResultV2 {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub structural_cells: Vec<StructuralCellV2>,
    pub report: ElevationCliffResolveReportV2,
}

impl LegacyCliffBridgeResultV2 {
    pub fn structural_at(&self, x: i32, y: i32) -> Option<StructuralCellV2> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        self.structural_cells
            .get(y as usize * self.width as usize + x as usize)
            .copied()
    }
}

/// Resolves structural terrain for one complete legacy scene.
///
/// This function deliberately does not replace grass, sand, rock, or water
/// surface tiles with `TileKind::Cliff`. Cliff faces are derived structural
/// output and are consumed by rendering, collision, cave placement, and editor
/// diagnostics through this returned cache.
pub fn resolve_tavern_map_elevation_cliffs_v2(
    map: &TavernMap,
    settings: ElevationCliffSettingsV2,
) -> Result<LegacyCliffBridgeResultV2, String> {
    let mut cells = tavern_map_to_structural_surface_cells_v2(map);
    let (structural_cells, report) = resolve_elevation_cliffs_v2(
        &mut cells,
        ElevationGridSpecV2 {
            width: MAP_W,
            height: MAP_H,
            wrap_east_west: false,
        },
        settings,
    )?;

    Ok(LegacyCliffBridgeResultV2 {
        schema: LEGACY_CLIFF_BRIDGE_V2_SCHEMA.to_owned(),
        width: MAP_W as u32,
        height: MAP_H as u32,
        structural_cells,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn bridge_derives_material_fallback_cliffs_without_rewriting_surface_tiles() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        for x in 0..MAP_W as i32 {
            map.set(x, 0, TileKind::MountainRock);
            map.set(x, 1, TileKind::MountainRock);
            map.set(x, 2, TileKind::Grass);
        }
        let before = map.tiles.clone();

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("legacy cliff bridge");

        assert!(result.report.cliff_edges > 0);
        assert_eq!(map.tiles, before);
    }

    #[test]
    fn raw_height_never_creates_internal_mountainrock_tiers() {
        let mut map = TavernMap::empty_with(TileKind::MountainRock);
        map.set_height(10, 10, 48);
        map.set_height(11, 10, 232);

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("discrete structural bridge");
        let left = result.structural_at(10, 10).expect("left structural cell");
        let right = result.structural_at(11, 10).expect("right structural cell");

        assert!(!left.exposed_edges.contains(crate::EdgeMaskV2::EAST));
        assert!(!right.blocked_edges.contains(crate::EdgeMaskV2::WEST));
    }

    #[test]
    fn legacy_mountainrock_to_ground_boundary_creates_one_visible_step() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(10, 10, TileKind::MountainRock);
        map.set(11, 10, TileKind::Grass);
        map.set_height(10, 10, 48);
        map.set_height(11, 10, 232);

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("legacy material fallback");
        let high = result.structural_at(10, 10).expect("high structural cell");
        let low = result.structural_at(11, 10).expect("low structural cell");

        assert!(high.exposed_edges.contains(crate::EdgeMaskV2::EAST));
        assert!(low.blocked_edges.contains(crate::EdgeMaskV2::WEST));
        assert_eq!(high.face_segments, 1);
    }

    #[test]
    fn mountain_path_is_not_implicitly_raised_by_legacy_fallback() {
        assert_eq!(structural_elevation_for_tile_v2(TileKind::MountainPath, 255), 0);
        assert_eq!(
            structural_elevation_for_tile_v2(TileKind::MountainRock, 0),
            STRUCTURAL_ELEVATION_RESOLVER_STEP_V2
        );
    }

    #[test]
    fn general_grass_never_inherits_structural_cliffs_from_geology() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set_height(10, 10, 232);
        map.set_height(11, 10, 48);

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("grass structural bridge");
        let high_noise = result.structural_at(10, 10).expect("high-noise grass cell");
        let low_noise = result.structural_at(11, 10).expect("low-noise grass cell");

        assert!(high_noise.exposed_edges.is_empty());
        assert!(low_noise.blocked_edges.is_empty());
    }

    #[test]
    fn explicit_structural_levels_override_geological_fallback() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set_height(10, 10, 232);
        map.set_structural_level(10, 10, Some(2));
        map.set_structural_level(11, 10, Some(0));

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("explicit structural levels");
        let upper = result.structural_at(10, 10).expect("upper structural cell");
        let lower = result.structural_at(11, 10).expect("lower structural cell");

        assert!(upper.exposed_edges.contains(crate::EdgeMaskV2::EAST));
        assert!(lower.blocked_edges.contains(crate::EdgeMaskV2::WEST));
        assert_eq!(upper.face_segments, 2);
    }

    #[test]
    fn explicit_level_four_produces_four_visible_face_segments() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set_structural_level(10, 10, Some(haven_core::MAX_STRUCTURAL_LEVEL));
        map.set_structural_level(10, 11, Some(0));

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("explicit Level 4 structural face");
        let upper = result.structural_at(10, 10).expect("upper structural cell");

        assert_eq!(haven_core::MAX_STRUCTURAL_LEVEL, 4);
        assert!(upper.exposed_edges.contains(crate::EdgeMaskV2::SOUTH));
        assert_eq!(upper.edge_delta(crate::EdgeMaskV2::SOUTH), 8);
        assert_eq!(upper.face_segments, 4);
    }

    #[test]
    fn ordinary_coastline_is_not_promoted_to_a_structural_cliff() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(10, 10, TileKind::Grass);
        map.set_height(10, 10, 220);
        map.set(11, 10, TileKind::Sand);
        map.set_height(11, 10, 34);

        let result =
            resolve_tavern_map_elevation_cliffs_v2(&map, ElevationCliffSettingsV2::default())
                .expect("coast structural bridge");
        let grass = result.structural_at(10, 10).expect("grass structural cell");
        let sand = result.structural_at(11, 10).expect("sand structural cell");

        assert!(!grass.exposed_edges.contains(crate::EdgeMaskV2::EAST));
        assert!(!sand.blocked_edges.contains(crate::EdgeMaskV2::WEST));
    }
}
