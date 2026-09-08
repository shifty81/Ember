use std::collections::VecDeque;

use haven_core::{SceneBiome, TavernMap, TileKind, MAP_H, MAP_W};

use super::{family_neighbors, TerrainFamily};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShorelineCell {
    pub x: i32,
    pub y: i32,
    pub tile: TileKind,
    pub touches_water: bool,
    pub touches_land: bool,
}

use super::shore_water_lifecycle::{deep_variant, is_depth_water, shallow_variant};
pub use super::shore_water_lifecycle::{
    normalize_shore_water_lifecycle_region, ShoreWaterLifecycleReport,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoastlineGenerationProfile {
    /// Number of conservative one-cell cleanup passes before tuple resolution.
    pub cleanup_passes: usize,
    /// When true, every semantic water tile is normalized to one shallow-water
    /// material. Deep water remains deferred until the shoreline is accepted.
    pub shallow_water_only: bool,
    /// Repair diagonal-only land/water contacts that cannot form stable tuple
    /// seams without an orthogonal connection.
    pub repair_diagonal_contacts: bool,
    /// Width of the semantic beach band measured in cardinal tile steps.
    pub beach_band_width: usize,
    /// Normalize checkerboards and over-complex 2x2 natural-terrain contacts
    /// before exact LPC tuple lookup. This is semantic PCG cleanup, not a
    /// renderer fallback.
    pub repair_unsupported_shapes: bool,
}

impl CoastlineGenerationProfile {
    /// Production open-world coastline profile. It preserves marine/freshwater
    /// identities and resolves a distance-based shallow band around a deep core.
    pub const fn mainland_hydrology() -> Self {
        Self {
            cleanup_passes: 2,
            shallow_water_only: false,
            repair_diagonal_contacts: true,
            beach_band_width: 3,
            repair_unsupported_shapes: true,
        }
    }

    /// Legacy/test profile retained for explicitly shallow-only scenes.
    pub const fn mainland_shallow_stabilization() -> Self {
        Self {
            shallow_water_only: true,
            ..Self::mainland_hydrology()
        }
    }
}

impl Default for CoastlineGenerationProfile {
    fn default() -> Self {
        Self::mainland_hydrology()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CoastlineCleanupReport {
    pub removed_water_speckles: usize,
    pub eroded_land_spikes: usize,
    pub shallow_water_band_tiles: usize,
    pub deep_water_tiles: usize,
    pub primary_shore_tiles: usize,
    pub secondary_shore_tiles: usize,
    pub unsupported_topology_cells: usize,
    pub unsupported_shapes_repaired: usize,
    pub preserved_structural_tiles: usize,
    pub lifecycle_mutations: usize,
}

impl CoastlineCleanupReport {
    pub fn total_mutations(self) -> usize {
        self.removed_water_speckles
            + self.eroded_land_spikes
            + self.shallow_water_band_tiles
            + self.deep_water_tiles
            + self.primary_shore_tiles
            + self.secondary_shore_tiles
            + self.unsupported_shapes_repaired
            + self.lifecycle_mutations
    }

    pub fn status_line(self) -> String {
        format!(
            "coast cleanup: {} changed, {} shallow, {} shore, {} secondary, {} lifecycle, {} speckles removed, {} tuple-shapes repaired, {} unsupported",
            self.total_mutations(),
            self.shallow_water_band_tiles,
            self.primary_shore_tiles,
            self.secondary_shore_tiles,
            self.lifecycle_mutations,
            self.removed_water_speckles,
            self.unsupported_shapes_repaired,
            self.unsupported_topology_cells
        )
    }
}

pub fn analyze_shoreline_cell(map: &TavernMap, x: i32, y: i32) -> ShorelineCell {
    let tile = map.get(x, y);
    let neighbors = family_neighbors(map, x, y);
    ShorelineCell {
        x,
        y,
        tile,
        touches_water: neighbors.water_cardinal_mask() != 0,
        touches_land: neighbors.land_cardinal_mask() != 0,
    }
}

/// Applies a conservative tile-level coastline cleanup after heightmap
/// classification. This does not replace the renderer-side transition overlay;
/// it improves the underlying generated terrain so beaches and shallow-water
/// bands become coherent before art-specific fringe tiles are drawn.
///
/// The pass intentionally mutates only generated/natural terrain families. Roads,
/// floors, walls, bridges, farm plots, greenhouse markers, and other authored
/// structures are preserved so editor/player work is not erased by a coast rebuild.
pub fn apply_coastline_tile_pass(map: &mut TavernMap, biome: SceneBiome) -> CoastlineCleanupReport {
    apply_coastline_tile_pass_with_profile(
        map,
        biome,
        CoastlineGenerationProfile::mainland_hydrology(),
    )
}

pub fn apply_coastline_tile_pass_with_profile(
    map: &mut TavernMap,
    biome: SceneBiome,
    profile: CoastlineGenerationProfile,
) -> CoastlineCleanupReport {
    let original = map.tiles.clone();
    let mut smoothed = original.clone();
    let mut report = CoastlineCleanupReport::default();

    for _ in 0..profile.cleanup_passes.max(1) {
        let cleanup_source = smoothed.clone();
        smooth_tiny_coast_artifacts(&cleanup_source, &mut smoothed, biome, &mut report);
    }
    if profile.repair_diagonal_contacts {
        let diagonal_source = smoothed.clone();
        repair_diagonal_only_contacts(&diagonal_source, &mut smoothed, biome, &mut report);
    }
    if profile.repair_unsupported_shapes {
        for _ in 0..2 {
            let topology_source = smoothed.clone();
            repair_complex_natural_topology(&topology_source, &mut smoothed, biome, &mut report);
        }
    }

    let smoothed_source = smoothed.clone();
    let distance_to_water = cardinal_distance_to_water(&smoothed_source);
    let mut next = smoothed_source.clone();

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = smoothed_source[index];
            let family = TerrainFamily::from_tile(tile);

            if family.is_water() {
                let land_neighbors = count_neighbors_with(&smoothed_source, x, y, |neighbor| {
                    TerrainFamily::from_tile(neighbor).is_land()
                });
                let land_radius = count_radius_with(&smoothed_source, x, y, 2, |neighbor| {
                    TerrainFamily::from_tile(neighbor).is_land()
                });
                let water_neighbors = count_neighbors_with(&smoothed_source, x, y, |neighbor| {
                    TerrainFamily::from_tile(neighbor).is_water()
                });

                let resolved = if matches!(tile, TileKind::RiverWater | TileKind::RiverMouthBlend) {
                    TileKind::RiverWater
                } else if profile.shallow_water_only || land_neighbors > 0 || land_radius >= 2 {
                    shallow_variant(tile)
                } else if water_neighbors >= 7 {
                    deep_variant(tile)
                } else {
                    shallow_variant(tile)
                };

                if next[index] != resolved {
                    if matches!(resolved, TileKind::ShallowWater | TileKind::OceanShallow) {
                        report.shallow_water_band_tiles += 1;
                    } else if matches!(resolved, TileKind::DeepWater | TileKind::OceanDeep) {
                        report.deep_water_tiles += 1;
                    }
                    next[index] = resolved;
                }
                continue;
            }

            if !is_cleanup_mutable_land(tile) {
                if family.is_constructed()
                    || family.is_blocking_wall()
                    || matches!(family, TerrainFamily::Farm | TerrainFamily::Greenhouse)
                {
                    report.preserved_structural_tiles += 1;
                }
                continue;
            }

            let coast_distance = distance_to_water[index];
            let shore_domain =
                nearest_water_domain(&smoothed_source, x, y, profile.beach_band_width.max(3));
            let resolved = if coast_distance == 1 {
                Some(primary_shore_tile(biome, shore_domain, tile, 1, 0))
            } else if coast_distance > 1 && coast_distance <= profile.beach_band_width {
                Some(secondary_shore_tile(biome, shore_domain, tile))
            } else {
                None
            };

            if let Some(resolved) = resolved {
                if next[index] != resolved {
                    if coast_distance == 1 {
                        report.primary_shore_tiles += 1;
                    } else {
                        report.secondary_shore_tiles += 1;
                    }
                    next[index] = resolved;
                }
            }
        }
    }

    map.tiles = next;
    if !profile.shallow_water_only {
        let lifecycle = normalize_shore_water_lifecycle_region(
            map,
            0,
            0,
            MAP_W as i32 - 1,
            MAP_H as i32 - 1,
            3,
        );
        report.lifecycle_mutations = lifecycle.total_mutations();
    }
    report
}

fn smooth_tiny_coast_artifacts(
    original: &[TileKind],
    smoothed: &mut [TileKind],
    biome: SceneBiome,
    report: &mut CoastlineCleanupReport,
) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = original[index];
            let family = TerrainFamily::from_tile(tile);
            let water_cardinal = count_cardinal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_water()
            });
            let water_neighbors = count_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_water()
            });
            let land_cardinal = count_cardinal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_land()
            });
            let land_neighbors = count_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_land()
            });

            if family.is_water() {
                // Remove single-cell puddle/noise islands from generated coasts.
                if land_neighbors >= 6 && water_cardinal <= 1 {
                    smoothed[index] = fill_land_tile_for_removed_water(biome, original, x, y);
                    report.removed_water_speckles += 1;
                }
                continue;
            }

            if is_cleanup_mutable_land(tile) && water_neighbors >= 6 && land_cardinal <= 1 {
                // Erode one-tile land spikes that create very square coast teeth.
                smoothed[index] = dominant_shallow_water_tile(original, x, y);
                report.eroded_land_spikes += 1;
            } else if !is_cleanup_mutable_land(tile)
                && (family.is_constructed() || family.is_blocking_wall())
            {
                report.preserved_structural_tiles += 1;
            }
        }
    }
}

fn repair_diagonal_only_contacts(
    original: &[TileKind],
    smoothed: &mut [TileKind],
    biome: SceneBiome,
    report: &mut CoastlineCleanupReport,
) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = original[index];
            let family = TerrainFamily::from_tile(tile);
            let water_cardinal = count_cardinal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_water()
            });
            let water_diagonal = count_diagonal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_water()
            });
            let land_cardinal = count_cardinal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_land()
            });
            let land_diagonal = count_diagonal_neighbors_with(original, x, y, |neighbor| {
                TerrainFamily::from_tile(neighbor).is_land()
            });

            if family.is_water() && water_cardinal == 0 && water_diagonal > 0 && land_cardinal >= 2
            {
                smoothed[index] = fill_land_tile_for_removed_water(biome, original, x, y);
                report.removed_water_speckles += 1;
            } else if is_cleanup_mutable_land(tile)
                && land_cardinal == 0
                && land_diagonal > 0
                && water_cardinal >= 2
            {
                smoothed[index] = dominant_shallow_water_tile(original, x, y);
                report.eroded_land_spikes += 1;
            }
        }
    }
}

fn repair_complex_natural_topology(
    original: &[TileKind],
    smoothed: &mut [TileKind],
    biome: SceneBiome,
    report: &mut CoastlineCleanupReport,
) {
    // LPC terrain is sampled as TL/TR/BL/BR tuples. Diagonal checkerboards
    // (A/B over B/A) and four-way natural-material contacts are unstable PCG
    // micro-topology: they create corner-only seams and frequently have no
    // authored tuple. Resolve them semantically before tuple lookup.
    for y in 0..(MAP_H as i32 - 1) {
        for x in 0..(MAP_W as i32 - 1) {
            let Some(tl_i) = TavernMap::idx(x, y) else {
                continue;
            };
            let Some(tr_i) = TavernMap::idx(x + 1, y) else {
                continue;
            };
            let Some(bl_i) = TavernMap::idx(x, y + 1) else {
                continue;
            };
            let Some(br_i) = TavernMap::idx(x + 1, y + 1) else {
                continue;
            };
            let corners = [
                original[tl_i],
                original[tr_i],
                original[bl_i],
                original[br_i],
            ];
            if corners.iter().any(|tile| !is_topology_mutable(*tile)) {
                continue;
            }
            let families = corners.map(TerrainFamily::from_tile);
            let checkerboard = families[0] == families[3]
                && families[1] == families[2]
                && families[0] != families[1];
            let mut distinct = 0usize;
            for index in 0..families.len() {
                if !families[..index].contains(&families[index]) {
                    distinct += 1;
                }
            }
            if !checkerboard && distinct < 3 {
                continue;
            }

            // Prefer the cardinally supported family around the bottom-right
            // cell. This changes one semantic cell, preserves macro geography,
            // and removes the unsupported 2x2 contact deterministically.
            let replacement = dominant_natural_tile(original, x + 1, y + 1, biome);
            if smoothed[br_i] != replacement {
                smoothed[br_i] = replacement;
                report.unsupported_shapes_repaired += 1;
            }
        }
    }
}

fn is_topology_mutable(tile: TileKind) -> bool {
    is_cleanup_mutable_land(tile) || TerrainFamily::from_tile(tile).is_water()
}

fn dominant_natural_tile(tiles: &[TileKind], x: i32, y: i32, biome: SceneBiome) -> TileKind {
    let mut water = 0usize;
    let mut sand = 0usize;
    let mut dirt = 0usize;
    let mut grass = 0usize;
    for (ox, oy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        let Some(slot) = TavernMap::idx(x + ox, y + oy) else {
            continue;
        };
        let tile = tiles[slot];
        let family = TerrainFamily::from_tile(tile);
        if family.is_water() {
            water += 1;
        } else if matches!(tile, TileKind::Sand | TileKind::WetSand) {
            sand += 1;
        } else if matches!(tile, TileKind::Dirt) {
            dirt += 1;
        } else if family.is_land() {
            grass += 1;
        }
    }
    let best = water.max(sand).max(dirt).max(grass);
    if water == best {
        dominant_shallow_water_tile(tiles, x, y)
    } else if sand == best {
        primary_shore_tile(biome, ShoreWaterDomain::Inland, TileKind::Sand, 1, 0)
    } else if dirt == best {
        TileKind::Dirt
    } else {
        TileKind::Grass
    }
}

fn dominant_shallow_water_tile(tiles: &[TileKind], x: i32, y: i32) -> TileKind {
    let mut ocean = 0usize;
    let mut river = 0usize;
    let mut inland = 0usize;
    for oy in -1..=1 {
        for ox in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            let Some(index) = TavernMap::idx(x + ox, y + oy) else {
                continue;
            };
            match tiles[index] {
                TileKind::OceanDeep | TileKind::OceanShallow => ocean += 1,
                TileKind::RiverWater | TileKind::RiverMouthBlend => river += 1,
                TileKind::Water | TileKind::ShallowWater | TileKind::DeepWater => inland += 1,
                _ => {}
            }
        }
    }

    if ocean >= river.max(inland) && ocean > 0 {
        TileKind::OceanShallow
    } else if river >= inland && river > 0 {
        TileKind::RiverWater
    } else {
        TileKind::ShallowWater
    }
}

fn is_cleanup_mutable_land(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Grass
            | TileKind::TallGrass
            | TileKind::Dirt
            | TileKind::Sand
            | TileKind::WetSand
            | TileKind::PebbleShore
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShoreWaterDomain {
    Marine,
    Inland,
}

fn nearest_water_domain(tiles: &[TileKind], x: i32, y: i32, radius: usize) -> ShoreWaterDomain {
    let radius = radius.max(1) as i32;
    let mut marine_distance = i32::MAX;
    let mut inland_distance = i32::MAX;
    for oy in -radius..=radius {
        for ox in -radius..=radius {
            let Some(index) = TavernMap::idx(x + ox, y + oy) else {
                continue;
            };
            let distance = ox.abs() + oy.abs();
            match tiles[index] {
                TileKind::OceanDeep | TileKind::OceanShallow => {
                    marine_distance = marine_distance.min(distance)
                }
                tile if is_depth_water(tile) => inland_distance = inland_distance.min(distance),
                _ => {}
            }
        }
    }
    if marine_distance < inland_distance {
        ShoreWaterDomain::Marine
    } else {
        ShoreWaterDomain::Inland
    }
}

fn primary_shore_tile(
    biome: SceneBiome,
    domain: ShoreWaterDomain,
    current: TileKind,
    water_cardinal: usize,
    water_diagonal: usize,
) -> TileKind {
    let _ = (water_cardinal, water_diagonal);
    match (biome, domain) {
        (SceneBiome::Coastal, ShoreWaterDomain::Marine) => TileKind::Sand,
        (SceneBiome::Coastal, ShoreWaterDomain::Inland)
        | (SceneBiome::Temperate, ShoreWaterDomain::Inland) => {
            if matches!(current, TileKind::Sand | TileKind::WetSand) {
                current
            } else {
                TileKind::MudBank
            }
        }
        (SceneBiome::Temperate, ShoreWaterDomain::Marine) => TileKind::Sand,
        (SceneBiome::Highlands, _) => TileKind::PebbleShore,
        (SceneBiome::Cave, _) => current,
    }
}

fn secondary_shore_tile(
    biome: SceneBiome,
    domain: ShoreWaterDomain,
    current: TileKind,
) -> TileKind {
    match (biome, domain) {
        (SceneBiome::Coastal, ShoreWaterDomain::Marine) => TileKind::Sand,
        (SceneBiome::Coastal, ShoreWaterDomain::Inland)
        | (SceneBiome::Temperate, ShoreWaterDomain::Inland) => {
            if matches!(current, TileKind::Sand | TileKind::WetSand) {
                current
            } else {
                TileKind::Dirt
            }
        }
        (SceneBiome::Temperate, ShoreWaterDomain::Marine) => TileKind::Sand,
        (SceneBiome::Highlands, _) => TileKind::PebbleShore,
        (SceneBiome::Cave, _) => current,
    }
}

fn fill_land_tile_for_removed_water(
    biome: SceneBiome,
    tiles: &[TileKind],
    x: i32,
    y: i32,
) -> TileKind {
    let mut sand_like = 0;
    let mut dirt_like = 0;
    let mut grass_like = 0;
    for oy in -1..=1 {
        for ox in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            let Some(idx) = TavernMap::idx(x + ox, y + oy) else {
                continue;
            };
            match tiles[idx] {
                TileKind::Sand | TileKind::WetSand => sand_like += 1,
                TileKind::Dirt
                | TileKind::PebbleShore
                | TileKind::MudBank
                | TileKind::TilledSoil
                | TileKind::WateredSoil
                | TileKind::Crop => dirt_like += 1,
                TileKind::Grass | TileKind::TallGrass => grass_like += 1,
                _ => {}
            }
        }
    }

    if sand_like >= dirt_like && sand_like >= grass_like && sand_like > 0 {
        return secondary_shore_tile(biome, ShoreWaterDomain::Inland, TileKind::Sand);
    }
    if dirt_like >= grass_like && dirt_like > 0 {
        return TileKind::Dirt;
    }
    match biome {
        SceneBiome::Coastal => TileKind::Sand,
        SceneBiome::Highlands => TileKind::PebbleShore,
        SceneBiome::Cave => TileKind::CaveFloor,
        SceneBiome::Temperate => TileKind::Grass,
    }
}

fn cardinal_distance_to_water(tiles: &[TileKind]) -> Vec<usize> {
    let mut distance = vec![usize::MAX; tiles.len()];
    let mut queue = VecDeque::new();

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            if TerrainFamily::from_tile(tiles[index]).is_water() {
                distance[index] = 0;
                queue.push_back((x, y));
            }
        }
    }

    while let Some((x, y)) = queue.pop_front() {
        let Some(index) = TavernMap::idx(x, y) else {
            continue;
        };
        let next_distance = distance[index].saturating_add(1);
        for (ox, oy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let Some(neighbor_index) = TavernMap::idx(x + ox, y + oy) else {
                continue;
            };
            if next_distance < distance[neighbor_index] {
                distance[neighbor_index] = next_distance;
                queue.push_back((x + ox, y + oy));
            }
        }
    }

    distance
}

fn count_neighbors_with(
    tiles: &[TileKind],
    x: i32,
    y: i32,
    predicate: impl Fn(TileKind) -> bool,
) -> usize {
    let mut count = 0;
    for oy in -1..=1 {
        for ox in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            if let Some(idx) = TavernMap::idx(x + ox, y + oy) {
                if predicate(tiles[idx]) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn count_cardinal_neighbors_with(
    tiles: &[TileKind],
    x: i32,
    y: i32,
    predicate: impl Fn(TileKind) -> bool,
) -> usize {
    let mut count = 0;
    for (ox, oy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        if let Some(idx) = TavernMap::idx(x + ox, y + oy) {
            if predicate(tiles[idx]) {
                count += 1;
            }
        }
    }
    count
}

fn count_diagonal_neighbors_with(
    tiles: &[TileKind],
    x: i32,
    y: i32,
    predicate: impl Fn(TileKind) -> bool,
) -> usize {
    let mut count = 0;
    for (ox, oy) in [(-1, -1), (1, -1), (1, 1), (-1, 1)] {
        if let Some(idx) = TavernMap::idx(x + ox, y + oy) {
            if predicate(tiles[idx]) {
                count += 1;
            }
        }
    }
    count
}

fn count_radius_with(
    tiles: &[TileKind],
    x: i32,
    y: i32,
    radius: i32,
    predicate: impl Fn(TileKind) -> bool,
) -> usize {
    let mut count = 0;
    for oy in -radius..=radius {
        for ox in -radius..=radius {
            if ox == 0 && oy == 0 {
                continue;
            }
            if let Some(idx) = TavernMap::idx(x + ox, y + oy) {
                if predicate(tiles[idx]) {
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
#[path = "shoreline_regression_tests.rs"]
mod depth_domain_regression_tests;

#[cfg(test)]
#[path = "shoreline_resolver_tests.rs"]
mod tests;
