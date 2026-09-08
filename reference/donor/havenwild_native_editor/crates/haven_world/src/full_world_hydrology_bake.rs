//! Full-world hydrology bake for finite east/west-wrapping surface worlds.
//!
//! Runtime residency reconciliation intentionally operates on a local loaded
//! neighborhood and therefore must not infer the global wrap seam. The world
//! generation and persistence lane owns this complete-world bake. It requires
//! every surface chunk in row-major order, resolves the whole world as one
//! semantic hydrology grid, and writes the normalized legacy water tiles back
//! into the supplied chunk maps.

use haven_core::{TavernMap, TileKind};
use serde::{Deserialize, Serialize};

use crate::{
    reconcile_chunk_hydrology_v2, ChunkHydrologyReconcileReportV2, ChunkHydrologyWindowV2,
    HydrologySettingsV2, WorldManifestV2,
};

pub const FULL_WORLD_HYDROLOGY_BAKE_V2_SCHEMA: &str = "havenwild.full_world_hydrology_bake.v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullWorldHydrologyBakeReportV2 {
    pub schema: String,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub wrap_east_west: bool,
    pub seam_water_pairs_before: u32,
    pub seam_water_pairs_after: u32,
    pub reconcile: ChunkHydrologyReconcileReportV2,
}

/// Performs the canonical full-world hydrology bake.
///
/// `chunks` must contain exactly `width_chunks * height_chunks` maps in
/// row-major order. The manifest is validated before any mutation occurs.
/// Havenwild worlds are required to wrap east/west; disabling wrapping is
/// rejected by `WorldManifestV2::validate`.
pub fn bake_full_world_hydrology_v2(
    manifest: &WorldManifestV2,
    chunks: &mut Vec<TavernMap>,
    settings: HydrologySettingsV2,
) -> Result<FullWorldHydrologyBakeReportV2, String> {
    manifest.validate()?;

    let width_chunks = usize::try_from(manifest.width_chunks)
        .map_err(|_| "world width does not fit this platform".to_owned())?;
    let height_chunks = usize::try_from(manifest.height_chunks)
        .map_err(|_| "world height does not fit this platform".to_owned())?;
    let expected = width_chunks
        .checked_mul(height_chunks)
        .ok_or_else(|| "full-world chunk count overflow".to_owned())?;
    if chunks.len() != expected {
        return Err(format!(
            "full-world hydrology bake expected {expected} chunks, found {}",
            chunks.len()
        ));
    }

    let seam_water_pairs_before =
        count_wrapped_seam_water_pairs(chunks, width_chunks, height_chunks);

    let mut window = ChunkHydrologyWindowV2 {
        origin_chunk_x: 0,
        origin_chunk_y: 0,
        width_chunks,
        height_chunks,
        wraps_complete_world_width: manifest.wrap_east_west,
        chunks: std::mem::take(chunks),
    };

    let reconcile = reconcile_chunk_hydrology_v2(&mut window, settings)?;
    let seam_water_pairs_after =
        count_wrapped_seam_water_pairs(&window.chunks, width_chunks, height_chunks);
    *chunks = window.chunks;

    Ok(FullWorldHydrologyBakeReportV2 {
        schema: FULL_WORLD_HYDROLOGY_BAKE_V2_SCHEMA.to_owned(),
        width_chunks: manifest.width_chunks,
        height_chunks: manifest.height_chunks,
        wrap_east_west: manifest.wrap_east_west,
        seam_water_pairs_before,
        seam_water_pairs_after,
        reconcile,
    })
}

fn count_wrapped_seam_water_pairs(
    chunks: &[TavernMap],
    width_chunks: usize,
    height_chunks: usize,
) -> u32 {
    if width_chunks == 0 || height_chunks == 0 {
        return 0;
    }

    let mut count = 0_u32;
    for chunk_y in 0..height_chunks {
        let left_index = chunk_y * width_chunks;
        let right_index = left_index + width_chunks - 1;
        let left = &chunks[left_index];
        let right = &chunks[right_index];

        for local_y in 0..haven_core::MAP_H {
            let Some(left_cell) = TavernMap::idx(0, local_y as i32) else {
                continue;
            };
            let Some(right_cell) = TavernMap::idx((haven_core::MAP_W - 1) as i32, local_y as i32)
            else {
                continue;
            };
            if is_water(left.tiles[left_cell]) && is_water(right.tiles[right_cell]) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn is_water(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::RiverWater
            | TileKind::OceanShallow
            | TileKind::OceanDeep
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HydrologySettingsV2;

    #[test]
    fn rejects_incomplete_world_storage() {
        let manifest = WorldManifestV2::new(17, 2, 1, haven_core::MAP_W as u16);
        let mut chunks = vec![TavernMap::filled(TileKind::Grass)];
        let error =
            bake_full_world_hydrology_v2(&manifest, &mut chunks, HydrologySettingsV2::default())
                .expect_err("incomplete world must be rejected");
        assert!(error.contains("expected 2 chunks"));
    }

    #[test]
    fn wrapped_seam_is_resolved_as_one_water_neighborhood() {
        let manifest = WorldManifestV2::new(29, 2, 1, haven_core::MAP_W as u16);
        let mut left = TavernMap::filled(TileKind::Grass);
        let mut right = TavernMap::filled(TileKind::Grass);

        for y in 8..16 {
            for x in 0..4 {
                let index = TavernMap::idx(x, y).expect("valid left map cell");
                left.tiles[index] = TileKind::ShallowWater;
            }
            for x in (haven_core::MAP_W as i32 - 4)..haven_core::MAP_W as i32 {
                let index = TavernMap::idx(x, y).expect("valid right map cell");
                right.tiles[index] = TileKind::ShallowWater;
            }
        }

        let mut chunks = vec![left, right];
        let report =
            bake_full_world_hydrology_v2(&manifest, &mut chunks, HydrologySettingsV2::default())
                .expect("wrapped world hydrology should resolve");

        assert!(report.wrap_east_west);
        assert!(report.seam_water_pairs_before > 0);
        assert_eq!(
            report.seam_water_pairs_before,
            report.seam_water_pairs_after
        );
    }
}
