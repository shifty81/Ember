//! Cross-chunk hydrology reconciliation for legacy surface chunks.
//!
//! A chunk cannot determine whether a border water cell is shoreline, river,
//! lake interior, or part of a larger ocean body in isolation. This module
//! assembles a complete rectangular chunk neighborhood into one semantic grid,
//! executes Hydrology V2 once, and writes the resulting water depth back to the
//! participating legacy maps.
//!
//! Missing chunks are rejected instead of being treated as land. Callers must
//! provide a complete neighborhood, normally the dirty chunks plus a one-chunk
//! halo. East/west wrapping is enabled only when the supplied window represents
//! the complete world width.

use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{
    resolve_hydrology_v2, tavern_map_to_surface_cells_v1, HydrologyGridSpecV2,
    HydrologyResolveReportV2, HydrologySettingsV2, SurfaceCellV1, WaterDepthV1, WaterKindV1,
};

pub const CHUNK_HYDROLOGY_RECONCILIATION_V2_SCHEMA: &str =
    "havenwild.chunk_hydrology_reconciliation.v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkHydrologyWindowV2 {
    pub origin_chunk_x: i32,
    pub origin_chunk_y: i32,
    pub width_chunks: usize,
    pub height_chunks: usize,
    /// This must only be true when the window covers the complete world width.
    pub wraps_complete_world_width: bool,
    /// Row-major chunk storage: `chunk_y * width_chunks + chunk_x`.
    pub chunks: Vec<TavernMap>,
}

impl ChunkHydrologyWindowV2 {
    pub fn validate(&self) -> Result<(), String> {
        if self.width_chunks == 0 || self.height_chunks == 0 {
            return Err("chunk hydrology window dimensions must be non-zero".to_owned());
        }
        let expected = self
            .width_chunks
            .checked_mul(self.height_chunks)
            .ok_or_else(|| "chunk hydrology window dimensions overflow".to_owned())?;
        if self.chunks.len() != expected {
            return Err(format!(
                "chunk hydrology window expected {expected} chunks, found {}",
                self.chunks.len()
            ));
        }
        Ok(())
    }

    pub fn chunk_index(&self, local_chunk_x: usize, local_chunk_y: usize) -> Option<usize> {
        if local_chunk_x >= self.width_chunks || local_chunk_y >= self.height_chunks {
            return None;
        }
        Some(local_chunk_y * self.width_chunks + local_chunk_x)
    }

    pub fn world_chunk_coord(
        &self,
        local_chunk_x: usize,
        local_chunk_y: usize,
    ) -> Option<(i32, i32)> {
        self.chunk_index(local_chunk_x, local_chunk_y)?;
        Some((
            self.origin_chunk_x + local_chunk_x as i32,
            self.origin_chunk_y + local_chunk_y as i32,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkHydrologyMutationV2 {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub applied_cells: u32,
    pub shallow_to_deep: u32,
    pub deep_to_shallow: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkHydrologyReconcileReportV2 {
    pub schema: String,
    pub hydrology: HydrologyResolveReportV2,
    pub chunk_mutations: Vec<ChunkHydrologyMutationV2>,
    pub cross_chunk_water_edges: u32,
    pub applied_cells: u32,
}

/// Resolves a complete rectangular chunk neighborhood as one hydrology grid.
///
/// Callers should include a one-chunk halo around edited/generated chunks and
/// only persist the chunks they own after this function returns. The complete
/// window is still required so border depth and water-body connectivity are not
/// inferred from absent data.
pub fn reconcile_chunk_hydrology_v2(
    window: &mut ChunkHydrologyWindowV2,
    settings: HydrologySettingsV2,
) -> Result<ChunkHydrologyReconcileReportV2, String> {
    window.validate()?;

    let grid_width = window
        .width_chunks
        .checked_mul(MAP_W)
        .ok_or_else(|| "chunk hydrology grid width overflow".to_owned())?;
    let grid_height = window
        .height_chunks
        .checked_mul(MAP_H)
        .ok_or_else(|| "chunk hydrology grid height overflow".to_owned())?;
    let cell_count = grid_width
        .checked_mul(grid_height)
        .ok_or_else(|| "chunk hydrology grid cell count overflow".to_owned())?;

    let mut cells = Vec::with_capacity(cell_count);
    cells.resize_with(cell_count, || {
        SurfaceCellV1::dry(
            crate::SurfaceMaterialV1::Constructed,
            0,
            crate::GenerationStageId::Hydrology,
        )
    });

    for local_chunk_y in 0..window.height_chunks {
        for local_chunk_x in 0..window.width_chunks {
            let chunk_index = window
                .chunk_index(local_chunk_x, local_chunk_y)
                .expect("validated chunk index");
            let source = tavern_map_to_surface_cells_v1(&window.chunks[chunk_index]);
            for local_y in 0..MAP_H {
                let destination_y = local_chunk_y * MAP_H + local_y;
                let source_row = local_y * MAP_W;
                let destination_row = destination_y * grid_width + local_chunk_x * MAP_W;
                cells[destination_row..destination_row + MAP_W]
                    .copy_from_slice(&source[source_row..source_row + MAP_W]);
            }
        }
    }

    let cross_chunk_water_edges = count_cross_chunk_water_edges(
        &cells,
        grid_width,
        grid_height,
        window.width_chunks,
        window.height_chunks,
    );

    let hydrology = resolve_hydrology_v2(
        &mut cells,
        HydrologyGridSpecV2 {
            width: grid_width,
            height: grid_height,
            wrap_east_west: window.wraps_complete_world_width,
        },
        settings,
    )?;

    let mut chunk_mutations = Vec::with_capacity(window.chunks.len());
    let mut total_applied = 0_u32;

    for local_chunk_y in 0..window.height_chunks {
        for local_chunk_x in 0..window.width_chunks {
            let chunk_index = window
                .chunk_index(local_chunk_x, local_chunk_y)
                .expect("validated chunk index");
            let (chunk_x, chunk_y) = window
                .world_chunk_coord(local_chunk_x, local_chunk_y)
                .expect("validated chunk coordinate");
            let map = &mut window.chunks[chunk_index];
            let mut applied = 0_u32;
            let mut shallow_to_deep = 0_u32;
            let mut deep_to_shallow = 0_u32;

            for local_y in 0..MAP_H {
                for local_x in 0..MAP_W {
                    let global_x = local_chunk_x * MAP_W + local_x;
                    let global_y = local_chunk_y * MAP_H + local_y;
                    let semantic = cells[global_y * grid_width + global_x];
                    let Some(map_index) = TavernMap::idx(local_x as i32, local_y as i32) else {
                        continue;
                    };
                    let old = map.tiles[map_index];
                    let Some(next) = resolved_legacy_water_tile(old, semantic) else {
                        continue;
                    };
                    if old == next {
                        continue;
                    }
                    match (is_deep_water(old), is_deep_water(next)) {
                        (false, true) => shallow_to_deep += 1,
                        (true, false) => deep_to_shallow += 1,
                        _ => {}
                    }
                    map.tiles[map_index] = next;
                    applied += 1;
                }
            }

            total_applied += applied;
            chunk_mutations.push(ChunkHydrologyMutationV2 {
                chunk_x,
                chunk_y,
                applied_cells: applied,
                shallow_to_deep,
                deep_to_shallow,
            });
        }
    }

    Ok(ChunkHydrologyReconcileReportV2 {
        schema: CHUNK_HYDROLOGY_RECONCILIATION_V2_SCHEMA.to_owned(),
        hydrology,
        chunk_mutations,
        cross_chunk_water_edges,
        applied_cells: total_applied,
    })
}

fn resolved_legacy_water_tile(original: TileKind, cell: SurfaceCellV1) -> Option<TileKind> {
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

fn is_deep_water(tile: TileKind) -> bool {
    matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)
}

fn count_cross_chunk_water_edges(
    cells: &[SurfaceCellV1],
    width: usize,
    height: usize,
    width_chunks: usize,
    height_chunks: usize,
) -> u32 {
    let mut count = 0_u32;

    for chunk_x in 1..width_chunks {
        let right_x = chunk_x * MAP_W;
        let left_x = right_x - 1;
        for y in 0..height {
            if cells[y * width + left_x].is_water() && cells[y * width + right_x].is_water() {
                count += 1;
            }
        }
    }

    for chunk_y in 1..height_chunks {
        let bottom_y = chunk_y * MAP_H;
        let top_y = bottom_y - 1;
        for x in 0..width {
            if cells[top_y * width + x].is_water() && cells[bottom_y * width + x].is_water() {
                count += 1;
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_crossing_chunk_edge_gets_one_depth_solution() {
        let mut left = TavernMap::empty_with(TileKind::Grass);
        let mut right = TavernMap::empty_with(TileKind::Grass);

        for y in 20..=28 {
            for x in 58..MAP_W as i32 {
                left.set(x, y, TileKind::ShallowWater);
            }
            for x in 0..=6 {
                right.set(x, y, TileKind::ShallowWater);
            }
        }

        let mut window = ChunkHydrologyWindowV2 {
            origin_chunk_x: 4,
            origin_chunk_y: 2,
            width_chunks: 2,
            height_chunks: 1,
            wraps_complete_world_width: false,
            chunks: vec![left, right],
        };

        let report = reconcile_chunk_hydrology_v2(&mut window, HydrologySettingsV2::default())
            .expect("cross-chunk hydrology");

        assert!(report.cross_chunk_water_edges > 0);
        assert_eq!(window.chunks[0].get(63, 24), TileKind::DeepWater);
        assert_eq!(window.chunks[1].get(0, 24), TileKind::DeepWater);
    }

    #[test]
    fn incomplete_chunk_storage_is_rejected() {
        let mut window = ChunkHydrologyWindowV2 {
            origin_chunk_x: 0,
            origin_chunk_y: 0,
            width_chunks: 2,
            height_chunks: 2,
            wraps_complete_world_width: false,
            chunks: vec![TavernMap::empty_with(TileKind::Grass)],
        };
        let error = reconcile_chunk_hydrology_v2(&mut window, HydrologySettingsV2::default())
            .expect_err("incomplete window must fail");
        assert!(error.contains("expected 4 chunks"));
    }
}
