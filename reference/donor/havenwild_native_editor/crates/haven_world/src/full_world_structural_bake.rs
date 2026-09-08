//! Full-world structural terrain bake for finite east/west-wrapping worlds.
//!
//! Hydrology and elevation are resolved as complete-world systems during the
//! generation/persistence bake. This module assembles every chunk into one
//! semantic elevation grid, derives cliff/ramp/waterfall/cave-host structure,
//! then splits the derived cache back into deterministic per-chunk records.

use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{
    open_world::ChunkCoord, resolve_elevation_cliffs_v2, tavern_map_to_structural_surface_cells_v2,
    ElevationCliffResolveReportV2, ElevationCliffSettingsV2, ElevationGridSpecV2, StructuralCellV2,
    WorldManifestV2,
};

pub const FULL_WORLD_STRUCTURAL_BAKE_V2_SCHEMA: &str = "havenwild.full_world_structural_bake.v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkStructuralCacheV2 {
    pub chunk_x: u32,
    pub chunk_y: u32,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<StructuralCellV2>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullWorldStructuralBakeV2 {
    pub schema: String,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub wrap_east_west: bool,
    pub report: ElevationCliffResolveReportV2,
    pub chunks: Vec<ChunkStructuralCacheV2>,
}

pub fn bake_full_world_structural_terrain_v2(
    manifest: &WorldManifestV2,
    chunks: &[TavernMap],
    settings: ElevationCliffSettingsV2,
) -> Result<FullWorldStructuralBakeV2, String> {
    manifest.validate()?;

    let width_chunks = usize::try_from(manifest.width_chunks)
        .map_err(|_| "world width does not fit this platform".to_owned())?;
    let height_chunks = usize::try_from(manifest.height_chunks)
        .map_err(|_| "world height does not fit this platform".to_owned())?;
    let expected = width_chunks
        .checked_mul(height_chunks)
        .ok_or_else(|| "full-world structural chunk count overflow".to_owned())?;
    if chunks.len() != expected {
        return Err(format!(
            "full-world structural bake expected {expected} chunks, found {}",
            chunks.len()
        ));
    }

    let grid_width = width_chunks
        .checked_mul(MAP_W)
        .ok_or_else(|| "full-world structural grid width overflow".to_owned())?;
    let grid_height = height_chunks
        .checked_mul(MAP_H)
        .ok_or_else(|| "full-world structural grid height overflow".to_owned())?;
    let mut cells = vec![
        crate::SurfaceCellV1::dry(
            crate::SurfaceMaterialV1::Constructed,
            0,
            crate::GenerationStageId::CliffsStructuralTerrain,
        );
        grid_width
            .checked_mul(grid_height)
            .ok_or_else(|| "full-world structural cell count overflow".to_owned())?
    ];

    for chunk_y in 0..height_chunks {
        for chunk_x in 0..width_chunks {
            let chunk_index = chunk_y * width_chunks + chunk_x;
            let source = tavern_map_to_structural_surface_cells_v2(&chunks[chunk_index]);
            for local_y in 0..MAP_H {
                let source_row = local_y * MAP_W;
                let target_y = chunk_y * MAP_H + local_y;
                let target_row = target_y * grid_width + chunk_x * MAP_W;
                cells[target_row..target_row + MAP_W]
                    .copy_from_slice(&source[source_row..source_row + MAP_W]);
            }
        }
    }

    let (structures, report) = resolve_elevation_cliffs_v2(
        &mut cells,
        ElevationGridSpecV2 {
            width: grid_width,
            height: grid_height,
            wrap_east_west: manifest.wrap_east_west,
        },
        settings,
    )?;

    let mut chunk_caches = Vec::with_capacity(expected);
    for chunk_y in 0..height_chunks {
        for chunk_x in 0..width_chunks {
            let mut chunk_cells = Vec::with_capacity(MAP_W * MAP_H);
            for local_y in 0..MAP_H {
                let source_y = chunk_y * MAP_H + local_y;
                let source_row = source_y * grid_width + chunk_x * MAP_W;
                chunk_cells.extend_from_slice(&structures[source_row..source_row + MAP_W]);
            }
            chunk_caches.push(ChunkStructuralCacheV2 {
                chunk_x: chunk_x as u32,
                chunk_y: chunk_y as u32,
                width: MAP_W as u32,
                height: MAP_H as u32,
                cells: chunk_cells,
            });
        }
    }

    Ok(FullWorldStructuralBakeV2 {
        schema: FULL_WORLD_STRUCTURAL_BAKE_V2_SCHEMA.to_owned(),
        width_chunks: manifest.width_chunks,
        height_chunks: manifest.height_chunks,
        wrap_east_west: manifest.wrap_east_west,
        report,
        chunks: chunk_caches,
    })
}

pub const PARTITIONED_SURFACE_STRUCTURAL_BAKE_V2_SCHEMA: &str =
    "havenwild.partitioned_surface_structural_bake.v2";

/// One derived structural cache keyed by the signed storage-partition
/// coordinate used by continuous PCG surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionStructuralCacheV2 {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub width: u32,
    pub height: u32,
    pub cells: Vec<StructuralCellV2>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionedSurfaceStructuralBakeV2 {
    pub schema: String,
    pub min_chunk_x: i32,
    pub min_chunk_y: i32,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub wrap_east_west: bool,
    pub report: ElevationCliffResolveReportV2,
    pub chunks: Vec<PartitionStructuralCacheV2>,
}

/// Resolves every loaded partition of one active continuous-surface region as
/// one elevation grid. Missing slots inside the region rectangle are treated as
/// open ocean, so an island coast never becomes an artificial map boundary.
pub fn bake_partitioned_surface_structural_terrain_v2(
    partitions: &[(ChunkCoord, TavernMap)],
    wrap_east_west: bool,
    settings: ElevationCliffSettingsV2,
) -> Result<PartitionedSurfaceStructuralBakeV2, String> {
    if partitions.is_empty() {
        return Err("partitioned structural bake requires at least one chunk".to_owned());
    }

    let min_chunk_x = partitions
        .iter()
        .map(|(chunk, _)| chunk.x)
        .min()
        .unwrap_or(0);
    let max_chunk_x = partitions
        .iter()
        .map(|(chunk, _)| chunk.x)
        .max()
        .unwrap_or(0);
    let min_chunk_y = partitions
        .iter()
        .map(|(chunk, _)| chunk.y)
        .min()
        .unwrap_or(0);
    let max_chunk_y = partitions
        .iter()
        .map(|(chunk, _)| chunk.y)
        .max()
        .unwrap_or(0);
    let width_chunks = usize::try_from(max_chunk_x - min_chunk_x + 1)
        .map_err(|_| "partitioned structural width is invalid".to_owned())?;
    let height_chunks = usize::try_from(max_chunk_y - min_chunk_y + 1)
        .map_err(|_| "partitioned structural height is invalid".to_owned())?;
    let grid_width = width_chunks
        .checked_mul(MAP_W)
        .ok_or_else(|| "partitioned structural grid width overflow".to_owned())?;
    let grid_height = height_chunks
        .checked_mul(MAP_H)
        .ok_or_else(|| "partitioned structural grid height overflow".to_owned())?;

    let ocean_map = TavernMap::filled(TileKind::OceanDeep);
    let ocean_cell = tavern_map_to_structural_surface_cells_v2(&ocean_map)
        .into_iter()
        .next()
        .ok_or_else(|| "open-ocean structural fallback is empty".to_owned())?;
    let mut cells = vec![
        ocean_cell;
        grid_width.checked_mul(grid_height).ok_or_else(|| {
            "partitioned structural cell count overflow".to_owned()
        })?
    ];
    let mut seen = std::collections::BTreeSet::new();

    for (chunk, map) in partitions {
        if !seen.insert((chunk.x, chunk.y)) {
            return Err(format!(
                "duplicate structural partition coordinate {},{}",
                chunk.x, chunk.y
            ));
        }
        let chunk_x = usize::try_from(chunk.x - min_chunk_x)
            .map_err(|_| "partition x offset is invalid".to_owned())?;
        let chunk_y = usize::try_from(chunk.y - min_chunk_y)
            .map_err(|_| "partition y offset is invalid".to_owned())?;
        let source = tavern_map_to_structural_surface_cells_v2(map);
        for local_y in 0..MAP_H {
            let source_row = local_y * MAP_W;
            let target_y = chunk_y * MAP_H + local_y;
            let target_row = target_y * grid_width + chunk_x * MAP_W;
            cells[target_row..target_row + MAP_W]
                .copy_from_slice(&source[source_row..source_row + MAP_W]);
        }
    }

    let (structures, report) = resolve_elevation_cliffs_v2(
        &mut cells,
        ElevationGridSpecV2 {
            width: grid_width,
            height: grid_height,
            wrap_east_west,
        },
        settings,
    )?;

    let mut chunks = Vec::with_capacity(partitions.len());
    for (chunk, _) in partitions {
        let chunk_x = usize::try_from(chunk.x - min_chunk_x)
            .map_err(|_| "partition x offset is invalid".to_owned())?;
        let chunk_y = usize::try_from(chunk.y - min_chunk_y)
            .map_err(|_| "partition y offset is invalid".to_owned())?;
        let mut chunk_cells = Vec::with_capacity(MAP_W * MAP_H);
        for local_y in 0..MAP_H {
            let source_y = chunk_y * MAP_H + local_y;
            let source_row = source_y * grid_width + chunk_x * MAP_W;
            chunk_cells.extend_from_slice(&structures[source_row..source_row + MAP_W]);
        }
        chunks.push(PartitionStructuralCacheV2 {
            chunk_x: chunk.x,
            chunk_y: chunk.y,
            width: MAP_W as u32,
            height: MAP_H as u32,
            cells: chunk_cells,
        });
    }

    Ok(PartitionedSurfaceStructuralBakeV2 {
        schema: PARTITIONED_SURFACE_STRUCTURAL_BAKE_V2_SCHEMA.to_owned(),
        min_chunk_x,
        min_chunk_y,
        width_chunks: width_chunks as u32,
        height_chunks: height_chunks as u32,
        wrap_east_west,
        report,
        chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn rejects_incomplete_world_storage() {
        let manifest = WorldManifestV2::new(7, 2, 1, MAP_W as u16);
        let chunks = vec![TavernMap::filled(TileKind::Grass)];
        let error = bake_full_world_structural_terrain_v2(
            &manifest,
            &chunks,
            ElevationCliffSettingsV2::default(),
        )
        .expect_err("incomplete world must be rejected");
        assert!(error.contains("expected 2 chunks"));
    }

    #[test]
    fn partitioned_bake_derives_structure_across_signed_chunk_boundary() {
        let mut left = TavernMap::filled(TileKind::MountainRock);
        let mut right = TavernMap::filled(TileKind::Grass);
        left.heights.fill(112);
        right.heights.fill(48);
        let bake = bake_partitioned_surface_structural_terrain_v2(
            &[
                (ChunkCoord::new(-1, 2), left),
                (ChunkCoord::new(0, 2), right),
            ],
            false,
            ElevationCliffSettingsV2::default(),
        )
        .expect("partitioned structural bake");
        assert!(bake.report.cliff_edges > 0);
        assert_eq!(bake.min_chunk_x, -1);
        assert_eq!(bake.chunks.len(), 2);
    }

    #[test]
    fn derives_structure_across_chunk_boundary() {
        let manifest = WorldManifestV2::new(11, 2, 1, MAP_W as u16);
        let mut left = TavernMap::filled(TileKind::MountainRock);
        let mut right = TavernMap::filled(TileKind::Grass);
        for height in &mut left.heights {
            *height = 112;
        }
        for height in &mut right.heights {
            *height = 48;
        }
        let bake = bake_full_world_structural_terrain_v2(
            &manifest,
            &[left, right],
            ElevationCliffSettingsV2::default(),
        )
        .expect("full-world structural bake");
        assert!(bake.report.cliff_edges > 0);
        assert_eq!(bake.chunks.len(), 2);
    }
}
