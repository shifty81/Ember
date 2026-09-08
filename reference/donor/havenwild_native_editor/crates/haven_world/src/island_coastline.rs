use std::collections::{BTreeSet, VecDeque};

use haven_core::{TileKind, MAP_H, MAP_W};

#[derive(Clone, Debug)]
pub(crate) struct CoastlineRaster {
    width: usize,
    tiles: Vec<TileKind>,
    #[allow(dead_code)]
    pub(crate) land_cells: usize,
    #[allow(dead_code)]
    pub(crate) shoreline_cells: usize,
}

impl CoastlineRaster {
    pub(crate) fn tile(&self, x: usize, y: usize) -> TileKind {
        self.tiles[y * self.width + x]
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CoastlineGenerationInput<'a> {
    pub(crate) landmass_id: i32,
    pub(crate) seed: u64,
    pub(crate) shoreline_width: f32,
    pub(crate) mountain_radius: f32,
    pub(crate) min_scene_x: i32,
    pub(crate) min_scene_y: i32,
    pub(crate) grid_width: usize,
    pub(crate) grid_height: usize,
    pub(crate) occupied_scene_cells: &'a BTreeSet<(i32, i32)>,
}

pub(crate) fn generate_coastline_raster(input: CoastlineGenerationInput<'_>) -> CoastlineRaster {
    let width = input.grid_width.max(1) * MAP_W;
    let height = input.grid_height.max(1) * MAP_H;
    let occupied = occupied_tile_mask(width, height, input);
    let mut land = initial_land_mask(width, height, &occupied, input);
    enforce_open_water_boundary(width, height, &occupied, &mut land);

    for _ in 0..2 {
        land = smooth_land_mask(width, height, &occupied, &land);
        enforce_open_water_boundary(width, height, &occupied, &mut land);
    }
    ensure_land_exists(width, height, &occupied, &mut land);
    enforce_open_water_boundary(width, height, &occupied, &mut land);

    let distance_to_water = distance_from_sources(width, height, &occupied, &land, false);
    let distance_to_land = distance_from_sources(width, height, &occupied, &land, true);
    let beach_depth = shoreline_depth(input.shoreline_width);

    let mut tiles = vec![TileKind::OceanDeep; width * height];
    let mut land_cells = 0usize;
    let mut shoreline_cells = 0usize;
    for index in 0..tiles.len() {
        if !occupied[index] {
            continue;
        }
        let tile = if land[index] {
            land_cells += 1;
            match distance_to_water[index] {
                0 | 1 => {
                    shoreline_cells += 1;
                    TileKind::Sand
                }
                distance if distance <= beach_depth => {
                    shoreline_cells += 1;
                    TileKind::Sand
                }
                _ => TileKind::Grass,
            }
        } else {
            let distance = distance_to_land[index];
            if distance <= 2 {
                shoreline_cells += 1;
                TileKind::OceanShallow
            } else {
                TileKind::OceanDeep
            }
        };
        tiles[index] = tile;
    }

    CoastlineRaster {
        width,
        tiles,
        land_cells,
        shoreline_cells,
    }
}

fn occupied_tile_mask(
    width: usize,
    height: usize,
    input: CoastlineGenerationInput<'_>,
) -> Vec<bool> {
    let mut occupied = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            let scene_x = input.min_scene_x + (x / MAP_W) as i32;
            let scene_y = input.min_scene_y + (y / MAP_H) as i32;
            occupied[y * width + x] = input.occupied_scene_cells.contains(&(scene_x, scene_y));
        }
    }
    occupied
}

fn initial_land_mask(
    width: usize,
    height: usize,
    occupied: &[bool],
    input: CoastlineGenerationInput<'_>,
) -> Vec<bool> {
    let profile = IslandProfile::new(width, height, input);
    let mut land = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if !occupied[index] {
                continue;
            }
            let gx = x as f32 + 0.5;
            let gy = y as f32 + 0.5;
            let field = organic_island_field(gx, gy, profile, input.seed);
            let edge_penalty = assembly_edge_penalty(
                x,
                y,
                input.min_scene_x,
                input.min_scene_y,
                input.occupied_scene_cells,
            );
            land[index] = field + edge_penalty < 1.0;
        }
    }
    land
}

#[derive(Clone, Copy)]
struct IslandProfile {
    center_x: f32,
    center_y: f32,
    radius_x: f32,
    radius_y: f32,
    exponent: f32,
    roughness: f32,
    warp_x: f32,
    warp_y: f32,
    lobe_scale: f32,
}

impl IslandProfile {
    fn new(width: usize, height: usize, input: CoastlineGenerationInput<'_>) -> Self {
        let id_seed =
            input.seed ^ (input.landmass_id as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let is_main = input.landmass_id == 0;
        let center_x = width as f32 * (0.50 + (hash01(3, 7, id_seed) - 0.5) * 0.065);
        let center_y = height as f32 * (0.49 + (hash01(11, 5, id_seed) - 0.5) * 0.055);
        let reserved_scale = (input.mountain_radius - 0.36).clamp(-0.08, 0.08);
        Self {
            center_x,
            center_y,
            radius_x: width as f32
                * ((if is_main { 0.405 } else { 0.385 }) + reserved_scale * 0.06),
            radius_y: height as f32
                * ((if is_main { 0.385 } else { 0.365 }) + reserved_scale * 0.05),
            exponent: if is_main { 2.15 } else { 1.95 },
            roughness: if is_main { 0.115 } else { 0.145 },
            warp_x: width as f32 * (if is_main { 0.055 } else { 0.070 }),
            warp_y: height as f32 * (if is_main { 0.060 } else { 0.075 }),
            lobe_scale: if is_main { 1.0 } else { 0.88 },
        }
    }
}

fn organic_island_field(gx: f32, gy: f32, profile: IslandProfile, seed: u64) -> f32 {
    let broad_x = smooth_noise(gx, gy, seed ^ 0x77aa_19c3, 42.0) - 0.5;
    let broad_y = smooth_noise(gx, gy, seed ^ 0x31dd_ba91, 37.0) - 0.5;
    let medium_x = smooth_noise(gx, gy, seed ^ 0xbad5_eed1, 18.0) - 0.5;
    let medium_y = smooth_noise(gx, gy, seed ^ 0x51a7_0f5e, 16.0) - 0.5;
    let warped_x = gx + broad_x * profile.warp_x + medium_x * profile.warp_x * 0.32;
    let warped_y = gy + broad_y * profile.warp_y + medium_y * profile.warp_y * 0.30;

    let central = superellipse_distance(
        warped_x,
        warped_y,
        profile.center_x,
        profile.center_y,
        profile.radius_x,
        profile.radius_y,
        profile.exponent,
    );

    let lobe = profile.lobe_scale;
    let west = superellipse_distance(
        warped_x,
        warped_y,
        profile.center_x - profile.radius_x * 0.62,
        profile.center_y + profile.radius_y * 0.06,
        profile.radius_x * 0.50 * lobe,
        profile.radius_y * 0.60 * lobe,
        2.0,
    );
    let east = superellipse_distance(
        warped_x,
        warped_y,
        profile.center_x + profile.radius_x * 0.63,
        profile.center_y - profile.radius_y * 0.10,
        profile.radius_x * 0.52 * lobe,
        profile.radius_y * 0.58 * lobe,
        2.1,
    );
    let north = superellipse_distance(
        warped_x,
        warped_y,
        profile.center_x - profile.radius_x * 0.12,
        profile.center_y - profile.radius_y * 0.68,
        profile.radius_x * 0.58 * lobe,
        profile.radius_y * 0.45 * lobe,
        2.0,
    );
    let south = superellipse_distance(
        warped_x,
        warped_y,
        profile.center_x + profile.radius_x * 0.18,
        profile.center_y + profile.radius_y * 0.67,
        profile.radius_x * 0.62 * lobe,
        profile.radius_y * 0.46 * lobe,
        2.0,
    );

    let union = central.min(west).min(east).min(north).min(south);
    let broad = smooth_noise(gx, gy, seed ^ 0xa5a5_8421, 28.0) - 0.5;
    let medium = smooth_noise(gx, gy, seed ^ 0x9911_67ac, 11.0) - 0.5;
    let fine = smooth_noise(gx, gy, seed ^ 0x55aa_c33d, 5.5) - 0.5;

    // Localized positive bumps cut readable bays into the union of lobes.
    let east_bay = gaussian_bump(
        warped_x,
        warped_y,
        profile.center_x + profile.radius_x * 0.80,
        profile.center_y + profile.radius_y * 0.24,
        profile.radius_x * 0.24,
        profile.radius_y * 0.34,
    ) * 0.30;
    let west_bay = gaussian_bump(
        warped_x,
        warped_y,
        profile.center_x - profile.radius_x * 0.78,
        profile.center_y - profile.radius_y * 0.28,
        profile.radius_x * 0.23,
        profile.radius_y * 0.30,
    ) * 0.27;
    let south_cove = gaussian_bump(
        warped_x,
        warped_y,
        profile.center_x - profile.radius_x * 0.08,
        profile.center_y + profile.radius_y * 0.92,
        profile.radius_x * 0.30,
        profile.radius_y * 0.22,
    ) * 0.24;

    union
        + broad * profile.roughness
        + medium * profile.roughness * 0.55
        + fine * profile.roughness * 0.20
        + east_bay
        + west_bay
        + south_cove
}

fn superellipse_distance(
    x: f32,
    y: f32,
    center_x: f32,
    center_y: f32,
    radius_x: f32,
    radius_y: f32,
    exponent: f32,
) -> f32 {
    let dx = ((x - center_x) / radius_x.max(1.0)).abs();
    let dy = ((y - center_y) / radius_y.max(1.0)).abs();
    (dx.powf(exponent) + dy.powf(exponent)).powf(1.0 / exponent)
}

fn gaussian_bump(
    x: f32,
    y: f32,
    center_x: f32,
    center_y: f32,
    radius_x: f32,
    radius_y: f32,
) -> f32 {
    let dx = (x - center_x) / radius_x.max(1.0);
    let dy = (y - center_y) / radius_y.max(1.0);
    (-(dx * dx + dy * dy) * 1.55).exp()
}

fn assembly_edge_penalty(
    x: usize,
    y: usize,
    min_scene_x: i32,
    min_scene_y: i32,
    occupied_scene_cells: &BTreeSet<(i32, i32)>,
) -> f32 {
    let scene_x = min_scene_x + (x / MAP_W) as i32;
    let scene_y = min_scene_y + (y / MAP_H) as i32;
    let local_x = x % MAP_W;
    let local_y = y % MAP_H;
    let mut distance = usize::MAX;
    if !occupied_scene_cells.contains(&(scene_x - 1, scene_y)) {
        distance = distance.min(local_x);
    }
    if !occupied_scene_cells.contains(&(scene_x + 1, scene_y)) {
        distance = distance.min(MAP_W - 1 - local_x);
    }
    if !occupied_scene_cells.contains(&(scene_x, scene_y - 1)) {
        distance = distance.min(local_y);
    }
    if !occupied_scene_cells.contains(&(scene_x, scene_y + 1)) {
        distance = distance.min(MAP_H - 1 - local_y);
    }
    if distance == usize::MAX || distance >= 10 {
        0.0
    } else {
        // A missing neighboring scene cell is open ocean, not hidden land.
        // Keep a full water margin at the assembly boundary, then taper the
        // penalty inward so shoreline bands can form before interior grass.
        (10 - distance) as f32 / 10.0 * 0.86
    }
}

fn enforce_open_water_boundary(width: usize, height: usize, occupied: &[bool], land: &mut [bool]) {
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if occupied[index] && tile_touches_open_space(width, height, occupied, x, y) {
                land[index] = false;
            }
        }
    }
}

fn tile_touches_open_space(
    width: usize,
    height: usize,
    occupied: &[bool],
    x: usize,
    y: usize,
) -> bool {
    x == 0
        || y == 0
        || x + 1 == width
        || y + 1 == height
        || !occupied[y * width + x - 1]
        || !occupied[y * width + x + 1]
        || !occupied[(y - 1) * width + x]
        || !occupied[(y + 1) * width + x]
}

fn smooth_land_mask(width: usize, height: usize, occupied: &[bool], land: &[bool]) -> Vec<bool> {
    let mut next = land.to_vec();
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if !occupied[index] {
                next[index] = false;
                continue;
            }
            let mut neighbors = 0u8;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let neighbor = ny as usize * width + nx as usize;
                    if occupied[neighbor] && land[neighbor] {
                        neighbors += 1;
                    }
                }
            }
            next[index] = if land[index] {
                neighbors >= 3
            } else {
                neighbors >= 5
            };
        }
    }
    next
}

fn ensure_land_exists(width: usize, height: usize, occupied: &[bool], land: &mut [bool]) {
    if land.iter().any(|value| *value) {
        return;
    }
    let center_x = width / 2;
    let center_y = height / 2;
    let radius = width.min(height).max(8) / 6;
    for y in center_y.saturating_sub(radius)..=(center_y + radius).min(height - 1) {
        for x in center_x.saturating_sub(radius)..=(center_x + radius).min(width - 1) {
            let index = y * width + x;
            let dx = x.abs_diff(center_x);
            let dy = y.abs_diff(center_y);
            if occupied[index] && dx * dx + dy * dy <= radius * radius {
                land[index] = true;
            }
        }
    }
}

fn shoreline_depth(width_setting: f32) -> u16 {
    let normalized = ((width_setting.clamp(0.04, 0.24) - 0.04) / 0.20).clamp(0.0, 1.0);
    2 + (normalized * 2.0).round() as u16
}

fn distance_from_sources(
    width: usize,
    height: usize,
    occupied: &[bool],
    land: &[bool],
    source_is_land: bool,
) -> Vec<u16> {
    let mut distance = vec![u16::MAX; width * height];
    let mut queue = VecDeque::new();
    for index in 0..distance.len() {
        if occupied[index] && land[index] == source_is_land {
            distance[index] = 0;
            queue.push_back(index);
        }
    }

    while let Some(index) = queue.pop_front() {
        let x = index % width;
        let y = index / width;
        let next_distance = distance[index].saturating_add(1);
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let neighbor = ny as usize * width + nx as usize;
            if !occupied[neighbor] || next_distance >= distance[neighbor] {
                continue;
            }
            distance[neighbor] = next_distance;
            queue.push_back(neighbor);
        }
    }
    distance
}

fn hash01(x: i32, y: i32, seed: u64) -> f32 {
    let mut value = seed
        ^ (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    (value as u32) as f32 / u32::MAX as f32
}

fn smooth_noise(x: f32, y: f32, seed: u64, scale: f32) -> f32 {
    let sx = x / scale.max(1.0);
    let sy = y / scale.max(1.0);
    let x0 = sx.floor() as i32;
    let y0 = sy.floor() as i32;
    let tx = smoothstep(sx - sx.floor());
    let ty = smoothstep(sy - sy.floor());
    let a = hash01(x0, y0, seed);
    let b = hash01(x0 + 1, y0, seed);
    let c = hash01(x0, y0 + 1, seed);
    let d = hash01(x0 + 1, y0 + 1, seed);
    lerp(lerp(a, b, tx), lerp(c, d, tx), ty)
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(a: f32, b: f32, amount: f32) -> f32 {
    a + (b - a) * amount
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autotile::terrain_family::TerrainFamily;

    fn full_grid(width: i32, height: i32) -> BTreeSet<(i32, i32)> {
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .collect()
    }

    #[test]
    fn main_island_outline_is_not_a_single_ellipse_or_circle() {
        let occupied = full_grid(5, 4);
        let raster = generate_coastline_raster(CoastlineGenerationInput {
            landmass_id: 0,
            seed: 0x4841_5645_4e57_494c,
            shoreline_width: 0.12,
            mountain_radius: 0.42,
            min_scene_x: 0,
            min_scene_y: 0,
            grid_width: 5,
            grid_height: 4,
            occupied_scene_cells: &occupied,
        });
        let height = 4 * MAP_H;
        let width = 5 * MAP_W;
        let mut extents = Vec::new();
        for y in 0..height {
            let xs: Vec<usize> = (0..width)
                .filter(|x| !TerrainFamily::from_tile(raster.tile(*x, y)).is_water())
                .collect();
            if let (Some(first), Some(last)) = (xs.first(), xs.last()) {
                extents.push((*first, *last));
            }
        }
        let distinct_widths: BTreeSet<usize> = extents
            .iter()
            .map(|(first, last)| last.saturating_sub(*first))
            .collect();
        assert!(distinct_widths.len() > 18);
        assert!(raster.land_cells > 8_000);
        assert!(raster.shoreline_cells > 400);
    }

    #[test]
    fn occupied_edge_next_to_missing_scene_stays_water_after_smoothing() {
        let mut occupied = full_grid(2, 2);
        occupied.remove(&(1, 1));
        let raster = generate_coastline_raster(CoastlineGenerationInput {
            landmass_id: 2,
            seed: 42,
            shoreline_width: 0.12,
            mountain_radius: 0.42,
            min_scene_x: 0,
            min_scene_y: 0,
            grid_width: 2,
            grid_height: 2,
            occupied_scene_cells: &occupied,
        });

        for x in MAP_W..MAP_W * 2 {
            assert!(matches!(
                raster.tile(x, MAP_H - 1),
                TileKind::OceanDeep | TileKind::OceanShallow
            ));
        }
    }

    #[test]
    fn missing_scene_slot_is_treated_as_open_water() {
        let mut occupied = full_grid(2, 2);
        occupied.remove(&(1, 1));
        let raster = generate_coastline_raster(CoastlineGenerationInput {
            landmass_id: 2,
            seed: 42,
            shoreline_width: 0.12,
            mountain_radius: 0.42,
            min_scene_x: 0,
            min_scene_y: 0,
            grid_width: 2,
            grid_height: 2,
            occupied_scene_cells: &occupied,
        });
        for y in MAP_H..MAP_H * 2 {
            for x in MAP_W..MAP_W * 2 {
                assert!(
                    TerrainFamily::from_tile(raster.tile(x, y)).is_water(),
                    "missing scene slot cell {x},{y} must remain semantic open water"
                );
            }
        }
    }
    #[test]
    fn generated_coast_uses_marine_shallow_and_deep_semantics() {
        let occupied = full_grid(5, 4);
        let raster = generate_coastline_raster(CoastlineGenerationInput {
            landmass_id: 0,
            seed: 0x4841_5645_4e57_494c,
            shoreline_width: 0.12,
            mountain_radius: 0.42,
            min_scene_x: 0,
            min_scene_y: 0,
            grid_width: 5,
            grid_height: 4,
            occupied_scene_cells: &occupied,
        });
        let mut shallow = 0usize;
        let mut deep = 0usize;
        for y in 0..4 * MAP_H {
            for x in 0..5 * MAP_W {
                match raster.tile(x, y) {
                    TileKind::OceanShallow => shallow += 1,
                    TileKind::OceanDeep => deep += 1,
                    _ => {}
                }
            }
        }
        assert!(shallow > 0, "coast must include a marine shallow band");
        assert!(deep > 0, "open ocean must include deep marine water");
    }
}
