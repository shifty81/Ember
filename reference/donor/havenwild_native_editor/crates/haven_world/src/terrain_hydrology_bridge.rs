//! Compatibility bridge between legacy `TavernMap` terrain and canonical
//! `SurfaceCellV1` hydrology.
//!
//! This is the only supported shallow/deep-water mutation path while legacy
//! `TileKind` scene persistence remains active. The bridge converts the scene
//! into semantic cells, executes Hydrology V2, and writes only water-depth
//! results back to the legacy map. Presentation tiles remain derived elsewhere.

use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};

use crate::{
    resolve_hydrology_v2, DirtyRegion, GenerationStageId, HydrologyGridSpecV2,
    HydrologyResolveReportV2, HydrologySettingsV2, SurfaceCellV1, SurfaceMaterialV1, WaterDepthV1,
    WaterKindV1,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyHydrologyBridgeReportV2 {
    pub hydrology: HydrologyResolveReportV2,
    pub deep_to_shallow: usize,
    pub shallow_to_deep: usize,
    pub applied_cells: usize,
}

impl LegacyHydrologyBridgeReportV2 {
    pub fn total_depth_mutations(&self) -> usize {
        self.deep_to_shallow + self.shallow_to_deep
    }
}

/// Resolves hydrology for a complete legacy scene and applies water-depth
/// changes only inside `dirty_region`. The resolver still sees the full map so
/// results at the edit boundary are based on the correct neighborhood.
pub fn resolve_tavern_map_hydrology_v2(
    map: &mut TavernMap,
    dirty_region: DirtyRegion,
    settings: HydrologySettingsV2,
) -> Result<LegacyHydrologyBridgeReportV2, String> {
    let mut cells = tavern_map_to_surface_cells_v1(map);
    let hydrology = resolve_hydrology_v2(
        &mut cells,
        HydrologyGridSpecV2 {
            width: MAP_W,
            height: MAP_H,
            wrap_east_west: false,
        },
        settings,
    )?;

    let min_x = dirty_region.min_x.max(0);
    let min_y = dirty_region.min_y.max(0);
    let max_x = dirty_region.max_x.min(MAP_W as i32 - 1);
    let max_y = dirty_region.max_y.min(MAP_H as i32 - 1);

    let mut deep_to_shallow = 0;
    let mut shallow_to_deep = 0;
    let mut applied_cells = 0;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let old = map.tiles[index];
            let Some(next) = resolved_water_tile(old, cells[index]) else {
                continue;
            };
            if next == old {
                continue;
            }
            match (is_deep_water_tile(old), is_deep_water_tile(next)) {
                (true, false) => deep_to_shallow += 1,
                (false, true) => shallow_to_deep += 1,
                _ => {}
            }
            map.tiles[index] = next;
            applied_cells += 1;
        }
    }

    Ok(LegacyHydrologyBridgeReportV2 {
        hydrology,
        deep_to_shallow,
        shallow_to_deep,
        applied_cells,
    })
}

pub fn resolve_entire_tavern_map_hydrology_v2(
    map: &mut TavernMap,
    settings: HydrologySettingsV2,
) -> Result<LegacyHydrologyBridgeReportV2, String> {
    resolve_tavern_map_hydrology_v2(
        map,
        DirtyRegion {
            min_x: 0,
            min_y: 0,
            max_x: MAP_W as i32 - 1,
            max_y: MAP_H as i32 - 1,
        },
        settings,
    )
}

pub fn tavern_map_to_surface_cells_v1(map: &TavernMap) -> Vec<SurfaceCellV1> {
    map.tiles
        .iter()
        .copied()
        .enumerate()
        .map(|(index, tile)| {
            let mut cell = SurfaceCellV1::dry(
                material_for_tile(tile),
                map.heights.get(index).copied().unwrap_or_default() as i16,
                GenerationStageId::Hydrology,
            );
            let (kind, depth) = water_semantics_for_tile(tile);
            cell.water_kind = kind;
            cell.water_depth = depth;
            cell
        })
        .collect()
}

fn resolved_water_tile(original: TileKind, cell: SurfaceCellV1) -> Option<TileKind> {
    if cell.water_kind == WaterKindV1::None {
        return None;
    }
    let deep = cell.water_depth == WaterDepthV1::Deep;
    Some(match cell.water_kind {
        WaterKindV1::Ocean => {
            if deep {
                TileKind::OceanDeep
            } else {
                TileKind::OceanShallow
            }
        }
        WaterKindV1::River => TileKind::RiverWater,
        WaterKindV1::Fresh => {
            if deep {
                TileKind::DeepWater
            } else {
                TileKind::ShallowWater
            }
        }
        WaterKindV1::None => original,
    })
}

fn water_semantics_for_tile(tile: TileKind) -> (WaterKindV1, WaterDepthV1) {
    match tile {
        TileKind::DeepWater => (WaterKindV1::Fresh, WaterDepthV1::Deep),
        TileKind::Water | TileKind::ShallowWater | TileKind::ShoreFoam => {
            (WaterKindV1::Fresh, WaterDepthV1::Shallow)
        }
        TileKind::RiverWater | TileKind::RiverMouthBlend => {
            (WaterKindV1::River, WaterDepthV1::Shallow)
        }
        TileKind::OceanDeep => (WaterKindV1::Ocean, WaterDepthV1::Deep),
        TileKind::OceanShallow => (WaterKindV1::Ocean, WaterDepthV1::Shallow),
        _ => (WaterKindV1::None, WaterDepthV1::Dry),
    }
}

fn material_for_tile(tile: TileKind) -> SurfaceMaterialV1 {
    match tile {
        TileKind::Grass => SurfaceMaterialV1::Grass,
        TileKind::TallGrass => SurfaceMaterialV1::TallGrass,
        TileKind::Dirt => SurfaceMaterialV1::Dirt,
        TileKind::Sand => SurfaceMaterialV1::Sand,
        TileKind::WetSand => SurfaceMaterialV1::WetSand,
        TileKind::MountainRock | TileKind::Cliff => SurfaceMaterialV1::Rock,
        TileKind::TilledSoil | TileKind::WateredSoil => SurfaceMaterialV1::CultivatedSoil,
        TileKind::CaveFloor => SurfaceMaterialV1::Cave,
        _ => SurfaceMaterialV1::Constructed,
    }
}

fn is_deep_water_tile(tile: TileKind) -> bool {
    matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshwater_paint_derives_deep_interior_through_canonical_bridge() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        for y in 4..=10 {
            for x in 4..=10 {
                map.set(x, y, TileKind::ShallowWater);
            }
        }

        let report =
            resolve_entire_tavern_map_hydrology_v2(&mut map, HydrologySettingsV2::default())
                .expect("hydrology bridge");

        assert_eq!(map.get(4, 4), TileKind::ShallowWater);
        assert_eq!(map.get(7, 7), TileKind::DeepWater);
        assert!(report.shallow_to_deep > 0);
    }

    #[test]
    fn bridge_preserves_non_water_tiles() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(2, 2, TileKind::TilledSoil);
        resolve_entire_tavern_map_hydrology_v2(&mut map, HydrologySettingsV2::default())
            .expect("hydrology bridge");
        assert_eq!(map.get(2, 2), TileKind::TilledSoil);
    }
}
