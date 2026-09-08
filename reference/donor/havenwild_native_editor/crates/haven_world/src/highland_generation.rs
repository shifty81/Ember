//! Coherent highland material classification for continuous PCG surfaces.
//!
//! Z108 retires raw-height/noise threshold bands as a surface-material owner.
//! Highland material now follows the same bounded macro-geographic features
//! that own structural Level 1/2 topology. This prevents long Dirt and
//! MountainPath contour swaths from being painted across otherwise unrelated
//! terrain.

use std::collections::BTreeMap;

use haven_core::{SceneMap, TileKind, MAP_H, MAP_W};

use crate::{
    geographic_landforms::sample_geographic_landform,
    open_world::ChunkCoord,
};

#[allow(dead_code)]
pub(crate) const MOUNTAIN_ROCK_HEIGHT: u8 = 172;
const LEVEL_ONE_GEOLOGICAL_FLOOR: u8 = 150;
const LEVEL_TWO_GEOLOGICAL_FLOOR: u8 = 178;
const DEFAULT_MOUNTAIN_STRENGTH: f32 = 0.44;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HighlandPromotionReport {
    pub rock_ground_cells: usize,
    /// Retained for log/API compatibility. Z108 no longer paints arbitrary
    /// threshold-driven Dirt swaths in highlands.
    pub highland_dirt_cells: usize,
    /// Retained for log/API compatibility. Z108 no longer wraps rock in an
    /// automatic one-cell MountainPath band; MountainPath is reserved for
    /// authored/generated traversal connectors.
    pub shoulder_cells: usize,
}

/// Compatibility entry point used by older tools/tests. Production island
/// generation should call `promote_global_highland_materials_with_strength`
/// so the material and structural passes share the same mountain profile.
pub fn promote_global_highland_materials(
    scenes: &mut [SceneMap],
    chunks: &[ChunkCoord],
    seed: u64,
) -> Result<HighlandPromotionReport, String> {
    promote_global_highland_materials_with_strength(
        scenes,
        chunks,
        seed,
        DEFAULT_MOUNTAIN_STRENGTH,
    )
}

pub fn promote_global_highland_materials_with_strength(
    scenes: &mut [SceneMap],
    chunks: &[ChunkCoord],
    seed: u64,
    mountain_strength: f32,
) -> Result<HighlandPromotionReport, String> {
    if scenes.len() != chunks.len() || scenes.is_empty() {
        return Err(format!(
            "highland promotion requires matching non-empty scene/chunk arrays ({} scenes, {} chunks)",
            scenes.len(),
            chunks.len()
        ));
    }

    let mut scene_by_chunk = BTreeMap::new();
    for (index, chunk) in chunks.iter().copied().enumerate() {
        if scene_by_chunk.insert(chunk, index).is_some() {
            return Err(format!(
                "highland promotion received duplicate chunk {},{}",
                chunk.x, chunk.y
            ));
        }
    }

    let mut report = HighlandPromotionReport::default();
    for (scene_index, chunk) in chunks.iter().copied().enumerate() {
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let tile = scenes[scene_index].map.get(x, y);
                if !highland_eligible_surface(tile) {
                    continue;
                }
                let gx = chunk.x * MAP_W as i32 + x;
                let gy = chunk.y * MAP_H as i32 + y;
                let sample = sample_geographic_landform(seed, gx, gy, mountain_strength);
                match sample.structural_level {
                    0 => {}
                    1 => {
                        let height = scenes[scene_index].map.get_height(x, y);
                        if height < LEVEL_ONE_GEOLOGICAL_FLOOR {
                            scenes[scene_index]
                                .map
                                .set_height(x, y, LEVEL_ONE_GEOLOGICAL_FLOOR);
                        }
                    }
                    _ => {
                        let height = scenes[scene_index].map.get_height(x, y);
                        if height < LEVEL_TWO_GEOLOGICAL_FLOOR {
                            scenes[scene_index]
                                .map
                                .set_height(x, y, LEVEL_TWO_GEOLOGICAL_FLOOR);
                        }
                        if scenes[scene_index].map.get(x, y) != TileKind::MountainRock {
                            scenes[scene_index].map.set(x, y, TileKind::MountainRock);
                            report.rock_ground_cells += 1;
                        }
                    }
                }
            }
        }
    }

    // Deliberately no threshold Dirt or automatic MountainPath shoulder pass.
    // Terrain transitions render the material contact; connector generation is
    // the only procedural authority allowed to paint MountainPath in uplands.
    report.shoulder_cells = materialize_highland_shoulders(scenes, chunks)?;
    Ok(report)
}

/// Z108 compatibility no-op. The old implementation painted a complete
/// one-cell MountainPath ring around MountainRock, which visually amplified
/// noise contours into the reported brown/tiger-stripe swaths. Keep the API so
/// editor/repair callers remain source-compatible while retiring that behavior.
pub fn materialize_highland_shoulders(
    scenes: &mut [SceneMap],
    chunks: &[ChunkCoord],
) -> Result<usize, String> {
    if scenes.len() != chunks.len() || scenes.is_empty() {
        return Err(format!(
            "highland shoulder pass requires matching non-empty scene/chunk arrays ({} scenes, {} chunks)",
            scenes.len(),
            chunks.len()
        ));
    }
    Ok(0)
}

fn highland_eligible_surface(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Grass | TileKind::TallGrass | TileKind::Dirt | TileKind::MountainRock
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{ProjectSceneId, SceneBiome, SceneKind};

    fn scene(id: &str, fill: TileKind, height: u8) -> SceneMap {
        let mut scene = SceneMap::blank(
            ProjectSceneId::new(id),
            id,
            SceneKind::Exterior,
            SceneBiome::Temperate,
        );
        scene.map.tiles.fill(fill);
        scene.map.heights.fill(height);
        scene
    }

    #[test]
    fn geographic_highlands_cross_storage_partition_boundaries() {
        // Do not assume one historical seed happens to place a level-two core
        // inside these exact two storage chunks. Find a deterministic feature
        // that actually crosses the x=MAP_W partition seam, then prove the
        // material pass samples the same global feature on both sides.
        let seam_x = MAP_W as i32;
        let (seed, seam_y) = (1_u64..=256)
            .find_map(|seed| {
                (-256..=256).find_map(|global_y| {
                    let left = sample_geographic_landform(seed, seam_x - 1, global_y, 0.80);
                    let right = sample_geographic_landform(seed, seam_x, global_y, 0.80);
                    (left.structural_level >= 2
                        && right.structural_level >= 2
                        && left.feature_id == right.feature_id)
                        .then_some((seed, global_y))
                })
            })
            .expect("expected a deterministic highland core crossing the storage seam");

        let chunk_y = seam_y.div_euclid(MAP_H as i32);
        let local_y = seam_y.rem_euclid(MAP_H as i32);
        let chunks = [ChunkCoord::new(0, chunk_y), ChunkCoord::new(1, chunk_y)];
        let mut scenes = vec![
            scene("pcg_test_0", TileKind::Grass, 120),
            scene("pcg_test_1", TileKind::Grass, 120),
        ];
        let report = promote_global_highland_materials_with_strength(
            &mut scenes,
            &chunks,
            seed,
            0.80,
        )
        .expect("promote");

        assert!(report.rock_ground_cells > 0);
        assert_eq!(scenes[0].map.get(MAP_W as i32 - 1, local_y), TileKind::MountainRock);
        assert_eq!(scenes[1].map.get(0, local_y), TileKind::MountainRock);
        assert_eq!(report.highland_dirt_cells, 0);
        assert_eq!(report.shoulder_cells, 0);
    }

    #[test]
    fn highlands_do_not_generate_automatic_mountain_path_bands() {
        let mut scenes = vec![scene("pcg_test_0_0", TileKind::Grass, 150)];
        let report = promote_global_highland_materials_with_strength(
            &mut scenes,
            &[ChunkCoord::new(0, 0)],
            11,
            1.0,
        )
        .expect("promote");

        assert_eq!(report.highland_dirt_cells, 0);
        assert_eq!(report.shoulder_cells, 0);
        assert!(!scenes[0].map.tiles.contains(&TileKind::MountainPath));
    }
}
