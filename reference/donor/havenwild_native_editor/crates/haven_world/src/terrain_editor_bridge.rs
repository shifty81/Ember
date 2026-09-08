//! Editor/runtime bridge for Havenwild Terrain Standard v1.
//!
//! Semantic `TileKind` values remain save and gameplay authority. This module
//! derives the four-corner visual tuple used by the exact authored transition
//! catalog and reports the cells dirtied by a semantic paint operation.

use haven_core::{base_terrain, BaseTerrain, TavernMap, TileKind};

use crate::{
    embedded_terrain_tuple_resolver, TerrainCornerTuple, TerrainTupleResolution,
    TerrainTupleResolver,
};

/// Corner-tuple terrain cells are authored around the intersection between four
/// semantic cell centers. A tuple sampled at `(x, y)` therefore renders half a
/// tile down and right from the semantic-cell origin. Both the native editor and
/// the runtime must use this same offset or painted terrain appears in the
/// upper-left corner of the highlighted cell.
pub const TERRAIN_TUPLE_RENDER_OFFSET_TILES: f32 = 0.5;

/// World-space origin for the authored 32x32 corner-tuple tile sampled at
/// `(x, y)`. The returned units are terrain tiles rather than pixels.
pub const fn terrain_tuple_render_origin_tiles(x: i32, y: i32) -> (f32, f32) {
    (
        x as f32 + TERRAIN_TUPLE_RENDER_OFFSET_TILES,
        y as f32 + TERRAIN_TUPLE_RENDER_OFFSET_TILES,
    )
}

/// Stable Standard-v1 ordinal used by the promoted TSX tuple catalog.
pub const fn terrain_standard_ordinal(tile: TileKind) -> u16 {
    match base_terrain(tile) {
        BaseTerrain::Grass => 5,         // Grass
        BaseTerrain::Sand => 22,         // Sand
        BaseTerrain::Dirt => 3,          // Dirt_Tan
        BaseTerrain::Pebble => 9,        // Gravel_1
        BaseTerrain::Road => 26,         // Stone_Tan
        BaseTerrain::WoodFloor => 3,     // authored fallback: Dirt_Tan
        BaseTerrain::StoneFloor => 20,   // Rock_Gray
        BaseTerrain::Farm => 25,         // Soil
        BaseTerrain::ShallowWater => 28, // Water
        BaseTerrain::DeepWater => 29,    // Water_Deep (registered/disabled visually)
        BaseTerrain::Wall => 18,         // Rock_Black
        BaseTerrain::Cliff => 19,        // Rock_Dark
        BaseTerrain::CaveFloor => 1,     // Dirt_Dark
        BaseTerrain::CaveWall => 18,     // Rock_Black
        BaseTerrain::Greenhouse => 5,    // Grass
    }
}

/// Stable Terrain Standard v1 family represented by a save-compatible semantic
/// tile. The family code is presentation metadata; gameplay stores `TileKind`.
pub const fn terrain_standard_code(tile: TileKind) -> &'static str {
    match terrain_standard_ordinal(tile) {
        1 => "Dirt_Dark",
        3 => "Dirt_Tan",
        5 => "Grass",
        9 => "Gravel_1",
        18 => "Rock_Black",
        19 => "Rock_Dark",
        20 => "Rock_Gray",
        22 => "Sand",
        25 => "Soil",
        26 => "Stone_Tan",
        28 => "Water",
        29 => "Water_Deep",
        _ => "Dirt_Tan",
    }
}

/// Tuple for the cell whose top-left semantic sample is `(x, y)`.
///
/// The corner order exactly matches the promoted TSX contract:
/// top-left, top-right, bottom-left, bottom-right.
pub fn semantic_tuple_at(map: &TavernMap, x: i32, y: i32) -> TerrainCornerTuple {
    TerrainCornerTuple::new(
        Some(terrain_standard_ordinal(map.get(x, y))),
        Some(terrain_standard_ordinal(map.get(x + 1, y))),
        Some(terrain_standard_ordinal(map.get(x, y + 1))),
        Some(terrain_standard_ordinal(map.get(x + 1, y + 1))),
    )
}

pub fn resolve_semantic_tuple_at(
    resolver: &TerrainTupleResolver,
    map: &TavernMap,
    x: i32,
    y: i32,
) -> TerrainTupleResolution {
    resolver.resolve(semantic_tuple_at(map, x, y))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainTupleCompatibility {
    Compatible,
    UnlistedCombination,
}

/// Compatibility is derived from terrain-family pairs demonstrated by the
/// promoted tuple catalog. A multi-material junction may be contact-compatible
/// even when no single exact four-corner atlas cell exists for that signature.
pub fn terrain_tuple_compatibility(tuple: TerrainCornerTuple) -> TerrainTupleCompatibility {
    let Ok(resolver) = embedded_terrain_tuple_resolver() else {
        return TerrainTupleCompatibility::UnlistedCombination;
    };
    let mut ordinals = Vec::new();
    for ordinal in [
        tuple.top_left,
        tuple.top_right,
        tuple.bottom_left,
        tuple.bottom_right,
    ]
    .into_iter()
    .flatten()
    {
        if !ordinals.contains(&ordinal) {
            ordinals.push(ordinal);
        }
    }
    for first_index in 0..ordinals.len() {
        for second_index in first_index + 1..ordinals.len() {
            if !resolver.supports_material_pair(ordinals[first_index], ordinals[second_index]) {
                return TerrainTupleCompatibility::UnlistedCombination;
            }
        }
    }
    TerrainTupleCompatibility::Compatible
}

/// A semantic paint at `(x, y)` can affect the four tuple cells that share the
/// edited sample. This list is coordinate-only and can be wrapped/clamped by
/// the owning world topology before cache invalidation.
pub const fn tuple_cells_affected_by_semantic_edit(x: i32, y: i32) -> [(i32, i32); 4] {
    [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tuple_uses_the_canonical_corner_order() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 3, TileKind::Sand);
        map.set(5, 3, TileKind::ShallowWater);
        map.set(4, 4, TileKind::Dirt);
        map.set(5, 4, TileKind::TilledSoil);

        let tuple = semantic_tuple_at(&map, 4, 3);
        assert_eq!(tuple.top_left, Some(22));
        assert_eq!(tuple.top_right, Some(28));
        assert_eq!(tuple.bottom_left, Some(3));
        assert_eq!(tuple.bottom_right, Some(25));
    }

    #[test]
    fn compatibility_reports_known_and_unlisted_combinations() {
        assert_eq!(
            terrain_tuple_compatibility(TerrainCornerTuple::new(
                Some(28),
                Some(33),
                Some(22),
                Some(22)
            )),
            TerrainTupleCompatibility::Compatible
        );
        assert_eq!(
            terrain_tuple_compatibility(TerrainCornerTuple::new(
                Some(26),
                Some(22),
                Some(5),
                Some(5)
            )),
            TerrainTupleCompatibility::Compatible
        );
        assert_eq!(
            terrain_tuple_compatibility(TerrainCornerTuple::new(
                Some(14),
                Some(5),
                Some(23),
                Some(28)
            )),
            TerrainTupleCompatibility::UnlistedCombination
        );
    }

    #[test]
    fn semantic_tile_reports_stable_standard_family_code() {
        assert_eq!(terrain_standard_code(TileKind::Grass), "Grass");
        assert_eq!(terrain_standard_code(TileKind::Sand), "Sand");
        assert_eq!(terrain_standard_code(TileKind::ShallowWater), "Water");
    }

    #[test]
    fn semantic_edit_invalidates_only_the_four_sharing_tuple_cells() {
        assert_eq!(
            tuple_cells_affected_by_semantic_edit(10, 8),
            [(9, 7), (10, 7), (9, 8), (10, 8)]
        );
    }

    #[test]
    fn editor_and_runtime_share_the_embedded_exact_resolver() {
        let resolver = TerrainTupleResolver::embedded().expect("embedded tuple catalog");
        let map = TavernMap::empty_with(TileKind::Grass);
        let result = resolve_semantic_tuple_at(&resolver, &map, 2, 2);
        assert_eq!(result.selected_tile_id, Some(5));
        assert!(result.is_resolved());
    }

    #[test]
    fn corner_tuple_render_origin_is_half_a_tile_down_and_right() {
        assert_eq!(terrain_tuple_render_origin_tiles(7, 11), (7.5, 11.5));
    }
}
