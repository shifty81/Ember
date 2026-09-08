//! Deterministic discrete structural landforms for fresh PCG surfaces.
//!
//! Havenwild keeps smooth geological height for hydrology/material decisions,
//! but visible traversal elevation is deliberately a small authored-style set
//! of platform levels.  This pass converts the continuous geology into explicit
//! Level 0/1/2 plateaus across the complete landmass assembly so cliffs are
//! real generated structure rather than an accidental by-product of raw noise.

use std::collections::BTreeMap;

use haven_core::{SceneMap, TileKind, ZoneKind, MAP_H, MAP_W};

use crate::structural_landform_ramps::{
    choose_south_ramp_edges, directional_ramp_corridor_indices,
};
#[cfg(test)]
use crate::structural_landform_ramps::directional_ramp_orientation_for_candidate;
use crate::{
    geographic_landforms::geographic_structural_level,
    open_world::ChunkCoord,
    structural_landform_masks::{
        fill_small_structural_holes, neighborhood_density_percent, normalize_structural_contours,
        retain_broad_candidates,
    },
};

pub const STRUCTURAL_LANDFORM_GENERATION_SCHEMA: &str =
    "havenwild.structural_landform_generation.v0_2";

const PROTECTED_BUFFER_RADIUS: i32 = 6;
const GUARANTEE_RADIUS_X: i32 = 9;
const GUARANTEE_RADIUS_Y: i32 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuralLandformReport {
    pub level_zero_cells: usize,
    pub level_one_cells: usize,
    pub level_two_cells: usize,
    pub cliff_boundaries: usize,
    pub generated_ramps: usize,
    pub guaranteed_plateau: bool,
}

pub fn materialize_structural_landforms(
    scenes: &mut [SceneMap],
    chunks: &[ChunkCoord],
    seed: u64,
    mountain_strength: f32,
) -> Result<StructuralLandformReport, String> {
    if scenes.len() != chunks.len() || scenes.is_empty() {
        return Err(format!(
            "structural landform generation requires matching non-empty scene/chunk arrays ({} scenes, {} chunks)",
            scenes.len(),
            chunks.len()
        ));
    }

    let mut scene_by_chunk = BTreeMap::new();
    for (index, chunk) in chunks.iter().copied().enumerate() {
        if scene_by_chunk.insert(chunk, index).is_some() {
            return Err(format!(
                "structural landform generation received duplicate chunk {},{}",
                chunk.x, chunk.y
            ));
        }
    }

    let min_chunk_x = chunks.iter().map(|chunk| chunk.x).min().unwrap_or(0);
    let max_chunk_x = chunks.iter().map(|chunk| chunk.x).max().unwrap_or(0);
    let min_chunk_y = chunks.iter().map(|chunk| chunk.y).min().unwrap_or(0);
    let max_chunk_y = chunks.iter().map(|chunk| chunk.y).max().unwrap_or(0);
    let origin_x = min_chunk_x * MAP_W as i32;
    let origin_y = min_chunk_y * MAP_H as i32;
    let width = usize::try_from((max_chunk_x - min_chunk_x + 1) * MAP_W as i32)
        .map_err(|_| "structural landform width is invalid".to_string())?;
    let height = usize::try_from((max_chunk_y - min_chunk_y + 1) * MAP_H as i32)
        .map_err(|_| "structural landform height is invalid".to_string())?;
    let cell_count = width
        .checked_mul(height)
        .ok_or_else(|| "structural landform grid overflow".to_string())?;

    let local_cell = |global_x: i32, global_y: i32| -> Option<(usize, i32, i32)> {
        let chunk = ChunkCoord::new(
            global_x.div_euclid(MAP_W as i32),
            global_y.div_euclid(MAP_H as i32),
        );
        let scene_index = *scene_by_chunk.get(&chunk)?;
        Some((
            scene_index,
            global_x.rem_euclid(MAP_W as i32),
            global_y.rem_euclid(MAP_H as i32),
        ))
    };

    let mut present = vec![false; cell_count];
    let mut land = vec![false; cell_count];
    let mut protected = vec![false; cell_count];
    let mut raw_heights = vec![0u8; cell_count];

    for grid_y in 0..height {
        for grid_x in 0..width {
            let index = grid_y * width + grid_x;
            let global_x = origin_x + grid_x as i32;
            let global_y = origin_y + grid_y as i32;
            let Some((scene_index, x, y)) = local_cell(global_x, global_y) else {
                continue;
            };
            present[index] = true;
            let scene = &scenes[scene_index];
            let tile = scene.map.get(x, y);
            let is_land = structural_land_tile(tile);
            land[index] = is_land;
            raw_heights[index] = scene.map.get_height(x, y);
            protected[index] = is_land && protected_surface_cell(scene, x, y, tile);
        }
    }

    let protected_sources = protected
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect::<Vec<_>>();
    for index in protected_sources {
        let center_x = (index % width) as i32;
        let center_y = (index / width) as i32;
        for offset_y in -PROTECTED_BUFFER_RADIUS..=PROTECTED_BUFFER_RADIUS {
            for offset_x in -PROTECTED_BUFFER_RADIUS..=PROTECTED_BUFFER_RADIUS {
                let x = center_x + offset_x;
                let y = center_y + offset_y;
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }
                protected[y as usize * width + x as usize] = true;
            }
        }
    }

    let strength = mountain_strength.clamp(0.0, 1.0) as f64;

    // Z108: visible structural topology comes from finite macro-geographic
    // features, never from thresholded raw noise. Each global tile samples the
    // same deterministic plateau/highland feature lattice, then this assembly
    // clips that feature to land and protected civic/road corridors.
    let mut level_one_candidates = vec![false; cell_count];
    let mut level_two_candidates = vec![false; cell_count];
    for grid_y in 0..height {
        for grid_x in 0..width {
            let index = grid_y * width + grid_x;
            if !present[index] || !land[index] || protected[index] {
                continue;
            }
            let global_x = origin_x + grid_x as i32;
            let global_y = origin_y + grid_y as i32;
            let sampled_level = geographic_structural_level(
                seed,
                global_x,
                global_y,
                mountain_strength,
            );
            level_one_candidates[index] = sampled_level >= 1;
            level_two_candidates[index] = sampled_level >= 2;
        }
    }

    // The feature grammar is already broad by construction. The local density
    // filter remains as a defensive rasterization cleanup at coast/protection
    // cuts so no one-cell peninsula becomes a walk-blocking cliff artifact.
    let level_one_mask = retain_broad_candidates(
        &level_one_candidates,
        &present,
        &land,
        &protected,
        width,
        height,
        2,
        56,
    );
    let mut levels = level_one_mask
        .iter()
        .map(|raised| u8::from(*raised))
        .collect::<Vec<_>>();

    // Level 2 is a true nested feature core. Require strong Level-1 support so
    // clipping by a coast or protected road cannot leave a narrow Level-2 rim.
    let level_two_seed = level_two_candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            *candidate
                && level_one_mask[index]
                && neighborhood_density_percent(&level_one_mask, width, height, index, 2) >= 76
        })
        .collect::<Vec<_>>();
    let level_two_mask = retain_broad_candidates(
        &level_two_seed,
        &present,
        &land,
        &protected,
        width,
        height,
        2,
        58,
    );
    for (index, level_two) in level_two_mask.into_iter().enumerate() {
        if level_two {
            levels[index] = 2;
        }
    }

    fill_small_structural_holes(&mut levels, &land, &protected, width, height);
    normalize_structural_contours(&mut levels, &land, &protected, width, height);

    let mut guaranteed_plateau = false;
    if count_boundaries(&levels, &present, width, height) == 0 {
        if let Some(best_index) = best_plateau_anchor(&raw_heights, &land, &protected) {
            // A tiny/coast-clipped test assembly can legitimately miss every
            // macro feature. It still needs one usable cliff boundary for tests,
            // so normalize the eligible field back to Level 0 before stamping
            // one deterministic authored-style plateau.
            for (index, level) in levels.iter_mut().enumerate() {
                if present[index] {
                    *level = 0;
                }
            }
            stamp_guaranteed_plateau(
                &mut levels,
                &land,
                &protected,
                width,
                height,
                best_index,
                strength,
            );
            guaranteed_plateau = true;
        }
    }

    let ramp_edges = choose_south_ramp_edges(&levels, &land, &protected, width, height, seed);

    let mut report = StructuralLandformReport {
        guaranteed_plateau,
        ..StructuralLandformReport::default()
    };

    for grid_y in 0..height {
        for grid_x in 0..width {
            let index = grid_y * width + grid_x;
            if !present[index] {
                continue;
            }
            let global_x = origin_x + grid_x as i32;
            let global_y = origin_y + grid_y as i32;
            let Some((scene_index, x, y)) = local_cell(global_x, global_y) else {
                continue;
            };
            let level = levels[index];
            scenes[scene_index]
                .map
                .set_structural_level(x, y, Some(level));

            // Material and topology share the same geographic authority. A
            // protected road/civic cut can clip a Level-2 feature after the
            // pre-road material pass, so normalize stale Rock Ground back to
            // ordinary grass outside true Level-2 cells. Conversely, ensure a
            // surviving Level-2 core is visibly Rock Ground.
            let tile = scenes[scene_index].map.get(x, y);
            if level >= 2
                && matches!(tile, TileKind::Grass | TileKind::TallGrass | TileKind::Dirt)
            {
                scenes[scene_index].map.set(x, y, TileKind::MountainRock);
            } else if level < 2
                && tile == TileKind::MountainRock
                && scene_zone_allows_geographic_material_normalization(
                    scenes[scene_index].zone_at(x, y),
                )
            {
                scenes[scene_index].map.set(x, y, TileKind::Grass);
            }

            match level {
                0 => report.level_zero_cells += 1,
                1 => report.level_one_cells += 1,
                2 => report.level_two_cells += 1,
                _ => {}
            }
        }
    }

    for (upper_index, lower_index, rises_right) in ramp_edges {
        let corridor = directional_ramp_corridor_indices(
            upper_index,
            lower_index,
            width,
            height,
            rises_right,
        );
        for index in corridor {
            let grid_x = index % width;
            let grid_y = index / width;
            let global_x = origin_x + grid_x as i32;
            let global_y = origin_y + grid_y as i32;
            let Some((scene_index, x, y)) = local_cell(global_x, global_y) else {
                continue;
            };
            if ramp_compatible_tile(scenes[scene_index].map.get(x, y)) {
                scenes[scene_index].map.set(x, y, TileKind::MountainPath);
            }
        }
        report.generated_ramps += 1;
    }

    report.cliff_boundaries = count_boundaries(&levels, &present, width, height);
    Ok(report)
}

fn scene_zone_allows_geographic_material_normalization(zone: ZoneKind) -> bool {
    matches!(zone, ZoneKind::None)
}

fn structural_land_tile(tile: TileKind) -> bool {
    // Inland freshwater remains attached to the structural surface beneath it.
    // That is what lets a river/pool on Level 2 continue across Level 2->1->0
    // boundaries and produce required waterfall connectors. Marine water and
    // shoreline presentation remain Level 0.
    !matches!(
        tile,
        TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverMouthBlend
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
            | TileKind::MudBank
            | TileKind::ShoreFoam
            | TileKind::Cliff
            | TileKind::Wall
            | TileKind::CaveWall
    )
}

fn protected_surface_cell(scene: &SceneMap, x: i32, y: i32, tile: TileKind) -> bool {
    !matches!(scene.zone_at(x, y), ZoneKind::None | ZoneKind::Cave)
        || matches!(
            tile,
            TileKind::Road
                | TileKind::StonePath
                | TileKind::Bridge
                | TileKind::WoodFloor
                | TileKind::PlankFloor
                | TileKind::StoneFloor
                | TileKind::BrickFloor
        )
}

fn ramp_compatible_tile(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Grass | TileKind::Dirt | TileKind::MountainRock | TileKind::MountainPath
    )
}

fn best_plateau_anchor(raw_heights: &[u8], land: &[bool], protected: &[bool]) -> Option<usize> {
    raw_heights
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, _)| land[*index] && !protected[*index])
        .max_by_key(|(_, height)| *height)
        .map(|(index, _)| index)
}

fn stamp_guaranteed_plateau(
    levels: &mut [u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
    center_index: usize,
    strength: f64,
) {
    let center_x = (center_index % width) as i32;
    let center_y = (center_index / width) as i32;
    let outer_limit =
        GUARANTEE_RADIUS_X * GUARANTEE_RADIUS_X * GUARANTEE_RADIUS_Y * GUARANTEE_RADIUS_Y;
    for offset_y in -GUARANTEE_RADIUS_Y..=GUARANTEE_RADIUS_Y {
        for offset_x in -GUARANTEE_RADIUS_X..=GUARANTEE_RADIUS_X {
            let x = center_x + offset_x;
            let y = center_y + offset_y;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let ellipse = offset_x * offset_x * GUARANTEE_RADIUS_Y * GUARANTEE_RADIUS_Y
                + offset_y * offset_y * GUARANTEE_RADIUS_X * GUARANTEE_RADIUS_X;
            if ellipse > outer_limit {
                continue;
            }
            let index = y as usize * width + x as usize;
            if land[index] && !protected[index] {
                levels[index] = 1;
            }
        }
    }

    if strength < 0.50 {
        return;
    }
    let inner_x = 4;
    let inner_y = 3;
    let inner_limit = inner_x * inner_x * inner_y * inner_y;
    for offset_y in -inner_y..=inner_y {
        for offset_x in -inner_x..=inner_x {
            let x = center_x + offset_x;
            let y = center_y + offset_y;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let ellipse =
                offset_x * offset_x * inner_y * inner_y + offset_y * offset_y * inner_x * inner_x;
            if ellipse > inner_limit {
                continue;
            }
            let index = y as usize * width + x as usize;
            if land[index] && !protected[index] {
                levels[index] = 2;
            }
        }
    }
}

fn count_boundaries(levels: &[u8], present: &[bool], width: usize, height: usize) -> usize {
    let mut boundaries = 0usize;
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if !present[index] {
                continue;
            }
            if x + 1 < width {
                let east = index + 1;
                boundaries += usize::from(present[east] && levels[index] != levels[east]);
            }
            if y + 1 < height {
                let south = index + width;
                boundaries += usize::from(present[south] && levels[index] != levels[south]);
            }
        }
    }
    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{ProjectSceneId, SceneBiome, SceneKind};

    fn scene(id: &str, height: u8) -> SceneMap {
        let mut scene = SceneMap::blank(
            ProjectSceneId::new(id),
            id,
            SceneKind::Exterior,
            SceneBiome::Temperate,
        );
        scene.map.tiles.fill(TileKind::Grass);
        scene.map.heights.fill(height);
        scene
    }

    #[test]
    fn fresh_surface_materializes_explicit_structural_levels() {
        let mut scenes = vec![scene("pcg_test_0_0", 190)];
        let report =
            materialize_structural_landforms(&mut scenes, &[ChunkCoord::new(0, 0)], 19, 0.60)
                .expect("landforms");

        assert!(report.level_one_cells + report.level_two_cells > 0);
        assert!(scenes[0]
            .map
            .structural_levels
            .iter()
            .all(|level| *level <= haven_core::MAX_STRUCTURAL_LEVEL));
    }

    #[test]
    fn flat_world_still_receives_a_deterministic_cliff_plateau() {
        let mut scenes = vec![scene("pcg_test_0_0", 96)];
        let report =
            materialize_structural_landforms(&mut scenes, &[ChunkCoord::new(0, 0)], 23, 0.25)
                .expect("landforms");

        assert!(report.guaranteed_plateau);
        assert!(report.cliff_boundaries > 0);
        assert!(report.level_one_cells > 0);
    }

    #[test]
    fn directional_ramp_corridors_follow_the_authored_six_cell_diagonals() {
        let width = 7;
        let height = 7;
        let upper = 2 * width + 3;
        let lower = upper + width;

        let rise_right = directional_ramp_corridor_indices(upper, lower, width, height, true);
        let rise_left = directional_ramp_corridor_indices(upper, lower, width, height, false);

        let coords = |indices: Vec<usize>| {
            indices
                .into_iter()
                .map(|index| (index % width, index / width))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            coords(rise_right),
            vec![(4, 1), (4, 2), (3, 2), (3, 3), (2, 3), (2, 4)]
        );
        assert_eq!(
            coords(rise_left),
            vec![(2, 1), (2, 2), (3, 2), (3, 3), (4, 3), (4, 4)]
        );
    }

    #[test]
    fn directional_ramp_orientation_requires_the_complete_authored_level_footprint() {
        let width = 7;
        let height = 7;
        let upper = 2 * width + 3;
        let lower = upper + width;
        let mut levels = vec![0u8; width * height];
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        for index in directional_ramp_corridor_indices(upper, lower, width, height, true) {
            let y = index / width;
            levels[index] = if y <= 2 { 1 } else { 0 };
        }
        assert_eq!(
            directional_ramp_orientation_for_candidate(
                upper, lower, &levels, &land, &protected, width, height, 7
            ),
            Some(true)
        );

        let blocked = 1 * width + 4;
        let mut protected_blocked = protected.clone();
        protected_blocked[blocked] = true;
        assert_eq!(
            directional_ramp_orientation_for_candidate(
                upper, lower, &levels, &land, &protected_blocked, width, height, 7
            ),
            None
        );
    }

    #[test]
    fn each_disconnected_raised_component_receives_a_south_gateway() {
        let width = 12;
        let height = 6;
        let mut levels = vec![0u8; width * height];

        // Two disconnected Level-1 plateaus, inset from every map edge so each
        // component has room for the complete six-cell directional ramp corridor.
        for y in 1..=2 {
            for x in 2..=4 {
                levels[y * width + x] = 1;
            }
            for x in 7..=9 {
                levels[y * width + x] = 1;
            }
        }

        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];
        let ramps = choose_south_ramp_edges(&levels, &land, &protected, width, height, 0x51f7);

        let left_component_has_gateway = ramps.iter().any(|(upper, _, _)| {
            let x = *upper % width;
            let y = *upper / width;
            (2..=4).contains(&x) && (1..=2).contains(&y)
        });
        let right_component_has_gateway = ramps.iter().any(|(upper, _, _)| {
            let x = *upper % width;
            let y = *upper / width;
            (7..=9).contains(&x) && (1..=2).contains(&y)
        });

        assert!(
            left_component_has_gateway,
            "left disconnected raised component did not receive a south gateway: {ramps:?}"
        );
        assert!(
            right_component_has_gateway,
            "right disconnected raised component did not receive a south gateway: {ramps:?}"
        );
    }

    #[test]
    fn public_paths_are_kept_out_of_structural_cliff_boundaries() {
        let mut scene = scene("pcg_test_0_0", 190);
        for y in 0..MAP_H as i32 {
            scene.map.set(MAP_W as i32 / 2, y, TileKind::Road);
            scene.set_zone(MAP_W as i32 / 2, y, ZoneKind::PublicPath);
        }
        let mut scenes = vec![scene];
        materialize_structural_landforms(&mut scenes, &[ChunkCoord::new(0, 0)], 29, 0.60)
            .expect("landforms");

        let road_x = MAP_W as i32 / 2;
        for y in 0..MAP_H as i32 {
            for offset_x in -PROTECTED_BUFFER_RADIUS..=PROTECTED_BUFFER_RADIUS {
                let x = road_x + offset_x;
                if (0..MAP_W as i32).contains(&x) {
                    assert_eq!(scenes[0].map.get_structural_level(x, y), Some(0));
                }
            }
        }
    }

}
