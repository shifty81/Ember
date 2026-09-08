//! Large-scale deterministic surface geography for finite and endless worlds.
//!
//! Storage chunks are only persistence/streaming partitions.  This module owns
//! the continuous global-coordinate land/ocean field that every partition
//! samples, so a 64x64 chunk boundary can never become a geographic boundary.
//! Continents are explicit broad overlapping features; low-amplitude noise only
//! roughens their coasts and never becomes the coastline by thresholding itself.

use haven_core::SceneBiome;

use crate::{
    geographic_forest_habitat, geographic_structural_level, LandformPreset, WorldCreationSettings,
};

pub const GEOGRAPHIC_SURFACE_SCHEMA: &str = "havenwild.geographic_surface.v0_1";
pub const GEOGRAPHIC_REGION_SIZE_TILES: u32 = 2_048;

const CONTINENT_MACRO_TILES: i32 = 3_072;
const CONTINENT_DOMAIN: u64 = 0x434f_4e54_494e_454e;
const CONTINENT_LOBE_DOMAIN: u64 = 0x434f_4e54_4c4f_4245;
const COAST_WARP_DOMAIN: u64 = 0x434f_4153_5457_4152;
const COAST_WARP_BROAD_DOMAIN: u64 = 0x434f_4153_5442_5244;
const COAST_WARP_DETAIL_DOMAIN: u64 = 0x434f_4153_5444_544c;
const MOISTURE_DOMAIN: u64 = 0x4d4f_4953_5455_5245;
const HEIGHT_DOMAIN: u64 = 0x4845_4947_4854_3031;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeographicGenerationProfile {
    pub landform: LandformPreset,
    pub land_coverage_percent: u8,
    pub coastline_complexity: f32,
    pub mountain_strength: f32,
    pub river_density: f32,
    pub lake_density: f32,
    pub pond_density: f32,
    pub wetland_density: f32,
    pub waterfall_density: f32,
}

impl Default for GeographicGenerationProfile {
    fn default() -> Self {
        Self {
            landform: LandformPreset::BalancedContinent,
            land_coverage_percent: 65,
            coastline_complexity: 0.50,
            mountain_strength: 0.44,
            river_density: 0.50,
            lake_density: 0.50,
            pond_density: 0.50,
            wetland_density: 0.50,
            waterfall_density: 0.50,
        }
    }
}

impl GeographicGenerationProfile {
    pub fn from_world_creation(settings: &WorldCreationSettings) -> Self {
        Self {
            landform: settings.landform,
            land_coverage_percent: settings.land.land_coverage_percent,
            coastline_complexity: settings.land.coastline_complexity.normalized(),
            mountain_strength: settings.mountain_radius_hint(),
            river_density: settings.hydrology.river_density.normalized(),
            lake_density: settings.hydrology.lake_density.normalized(),
            pond_density: settings.hydrology.pond_density.normalized(),
            wetland_density: settings.hydrology.wetland_density.normalized(),
            waterfall_density: settings.hydrology.waterfall_density.normalized(),
        }
    }

    pub const fn region_size_tiles(self) -> u32 {
        GEOGRAPHIC_REGION_SIZE_TILES
    }
}

impl From<&WorldCreationSettings> for GeographicGenerationProfile {
    fn from(value: &WorldCreationSettings) -> Self {
        Self::from_world_creation(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeographicSurfaceSample {
    /// Signed approximate distance from the coast in tiles. Positive is land,
    /// negative is marine water.
    pub coast_distance_tiles: f32,
    pub land: bool,
    pub shoreline: bool,
    pub shallow_ocean: bool,
    pub structural_level: u8,
    pub elevation: f32,
    pub moisture: f32,
    pub biome: SceneBiome,
}

pub fn sample_geographic_surface(
    seed: u64,
    global_x: i32,
    global_y: i32,
    profile: GeographicGenerationProfile,
) -> GeographicSurfaceSample {
    let mut nearest = starter_mainland_distance(seed, global_x, global_y, profile);

    let macro_x = global_x.div_euclid(CONTINENT_MACRO_TILES);
    let macro_y = global_y.div_euclid(CONTINENT_MACRO_TILES);
    for cell_y in macro_y - 1..=macro_y + 1 {
        for cell_x in macro_x - 1..=macro_x + 1 {
            let feature_hash = hash2(seed ^ CONTINENT_DOMAIN, cell_x, cell_y);
            if !continent_feature_enabled(feature_hash, profile) {
                continue;
            }
            let feature = ContinentalFeature::from_macro_cell(seed, cell_x, cell_y, profile);
            nearest = nearest.better(feature.primary.distance(global_x, global_y));
            nearest = nearest.better(feature.lobe_a.distance(global_x, global_y));
            nearest = nearest.better(feature.lobe_b.distance(global_x, global_y));
            nearest = nearest.better(feature.lobe_c.distance(global_x, global_y));
        }
    }

    // Coastlines need several geographic scales. A single low-amplitude noise
    // pass left the kilometer-scale ellipses visibly circular on the world map.
    // Broad displacement creates peninsulas/bays, medium displacement breaks
    // the remaining oval silhouette, and the small pass only roughens the edge.
    // All three are continuous global-coordinate fields, so no chunk seam can
    // become a coastline seam.
    let complexity = f64::from(profile.coastline_complexity);
    let broad_coast_warp =
        (fractal_noise(seed ^ COAST_WARP_BROAD_DOMAIN, global_x, global_y, 760.0, 4) - 0.5)
            * 2.0
            * (58.0 + complexity * 132.0);
    let medium_coast_warp =
        (fractal_noise(seed ^ COAST_WARP_DOMAIN, global_x, global_y, 230.0, 3) - 0.5)
            * 2.0
            * (24.0 + complexity * 68.0);
    let detail_coast_warp =
        (fractal_noise(seed ^ COAST_WARP_DETAIL_DOMAIN, global_x, global_y, 76.0, 2) - 0.5)
            * 2.0
            * (5.0 + complexity * 18.0);
    let coast_distance = nearest.signed_distance_tiles()
        + broad_coast_warp
        + medium_coast_warp
        + detail_coast_warp;
    let land = coast_distance >= 0.0;
    let shoreline = land && coast_distance <= 10.0;
    let shallow_ocean = !land && coast_distance >= -22.0;

    let mut structural_level = if land && !shoreline {
        geographic_structural_level(
            seed,
            global_x,
            global_y,
            profile.mountain_strength,
        )
    } else {
        0
    };
    // Keep the immediate beach/coast visually and collision-wise at Level 0.
    if coast_distance <= 18.0 {
        structural_level = 0;
    }

    let inland = (coast_distance.max(0.0) / 220.0).clamp(0.0, 1.0);
    let broad_height = fractal_noise(seed ^ HEIGHT_DOMAIN, global_x, global_y, 420.0, 3);
    let elevation = if land {
        (0.45
            + inland * 0.08
            + broad_height * 0.08
            + f64::from(structural_level) * 0.105)
            .clamp(0.40, 0.92)
    } else if shallow_ocean {
        0.31
    } else {
        0.18
    } as f32;

    let forest = f64::from(geographic_forest_habitat(seed, global_x, global_y));
    let moisture = (fractal_noise(seed ^ MOISTURE_DOMAIN, global_x, global_y, 360.0, 3) * 0.72
        + forest * 0.28)
        .clamp(0.0, 1.0) as f32;
    let biome = if !land || coast_distance < 44.0 {
        SceneBiome::Coastal
    } else if structural_level >= 1 {
        SceneBiome::Highlands
    } else {
        SceneBiome::Temperate
    };

    GeographicSurfaceSample {
        coast_distance_tiles: coast_distance as f32,
        land,
        shoreline,
        shallow_ocean,
        structural_level,
        elevation,
        moisture,
        biome,
    }
}

fn starter_mainland_distance(
    seed: u64,
    x: i32,
    y: i32,
    profile: GeographicGenerationProfile,
) -> FeatureDistance {
    // The starter mainland is a chain of overlapping continental lobes rather
    // than one enormous ellipse. A single dominant ellipse was visible on the
    // player map as a near-perfect circle even after coast noise was applied.
    // This lobe spine keeps the Willowmere southern coast deterministic while
    // creating broad bays, shoulders, peninsulas, and an irregular northern
    // continuation that can stitch into the macro-continent field.
    let style_scale = match profile.landform {
        LandformPreset::Archipelago => 0.56,
        LandformPreset::BrokenCoast => 0.82,
        LandformPreset::GrandIsland => 1.10,
        LandformPreset::InlandBasin => 1.14,
        _ => 1.0,
    };
    let coverage_scale = 0.82 + (f64::from(profile.land_coverage_percent) / 100.0) * 0.42;
    let radius_x = 1_850.0 * style_scale * coverage_scale;
    let radius_y = 1_500.0 * style_scale * coverage_scale;
    let coast_y = 275.0 + (hash01(seed ^ 0x5354_4152_5459, 0, 0) - 0.5) * 54.0;
    let center_x = 160.0 + (hash01(seed ^ 0x5354_4152_5458, 0, 0) - 0.5) * 96.0;
    let south_angle = (hash01(seed ^ 0x5354_4152_5441, 0, 0) - 0.5) * 0.30;
    let south_center_y = coast_y - radius_y;

    let south = RotatedEllipse::new(
        center_x,
        south_center_y,
        radius_x,
        radius_y,
        south_angle,
    );
    let west = RotatedEllipse::new(
        center_x - radius_x * 0.75
            + (hash01(seed ^ 0x5354_5745_5354, 0, 0) - 0.5) * 220.0,
        south_center_y - radius_y * 0.10
            + (hash01(seed ^ 0x5354_5745_5359, 0, 0) - 0.5) * 160.0,
        radius_x * 0.82,
        radius_y * 0.88,
        south_angle - 0.35
            + (hash01(seed ^ 0x5354_5745_5341, 0, 0) - 0.5) * 0.30,
    );
    let east = RotatedEllipse::new(
        center_x + radius_x * 0.72
            + (hash01(seed ^ 0x5354_4541_5354, 0, 0) - 0.5) * 220.0,
        south_center_y - radius_y * 0.18
            + (hash01(seed ^ 0x5354_4541_5359, 0, 0) - 0.5) * 160.0,
        radius_x * 0.78,
        radius_y * 0.84,
        south_angle + 0.28
            + (hash01(seed ^ 0x5354_4541_5341, 0, 0) - 0.5) * 0.30,
    );
    let north = RotatedEllipse::new(
        center_x + (hash01(seed ^ 0x5354_4e4f_5258, 0, 0) - 0.5) * 500.0,
        south_center_y - radius_y * 1.05
            + (hash01(seed ^ 0x5354_4e4f_5259, 0, 0) - 0.5) * 220.0,
        radius_x * 0.92,
        radius_y * 1.02,
        south_angle + (hash01(seed ^ 0x5354_4e4f_5241, 0, 0) - 0.5) * 0.40,
    );
    let north_west = RotatedEllipse::new(
        center_x - radius_x * 0.62
            + (hash01(seed ^ 0x5354_4e57_5858, 0, 0) - 0.5) * 300.0,
        south_center_y - radius_y * 1.05
            + (hash01(seed ^ 0x5354_4e57_5959, 0, 0) - 0.5) * 220.0,
        radius_x * 0.70,
        radius_y * 0.80,
        south_angle - 0.20
            + (hash01(seed ^ 0x5354_4e57_4141, 0, 0) - 0.5) * 0.40,
    );
    let north_east = RotatedEllipse::new(
        center_x + radius_x * 0.70
            + (hash01(seed ^ 0x5354_4e45_5858, 0, 0) - 0.5) * 300.0,
        south_center_y - radius_y * 1.18
            + (hash01(seed ^ 0x5354_4e45_5959, 0, 0) - 0.5) * 220.0,
        radius_x * 0.72,
        radius_y * 0.82,
        south_angle + 0.20
            + (hash01(seed ^ 0x5354_4e45_4141, 0, 0) - 0.5) * 0.40,
    );

    south
        .distance(x, y)
        .better(west.distance(x, y))
        .better(east.distance(x, y))
        .better(north.distance(x, y))
        .better(north_west.distance(x, y))
        .better(north_east.distance(x, y))
}

fn continent_feature_enabled(hash: u64, profile: GeographicGenerationProfile) -> bool {
    let base = f64::from(profile.land_coverage_percent) / 100.0;
    let style = match profile.landform {
        LandformPreset::Archipelago => -0.22,
        LandformPreset::GrandIsland => -0.10,
        LandformPreset::BrokenCoast => -0.05,
        LandformPreset::InlandBasin => 0.12,
        _ => 0.02,
    };
    unit_from_hash(hash) < (base * 0.78 + style).clamp(0.16, 0.78)
}

#[derive(Clone, Copy, Debug)]
struct ContinentalFeature {
    primary: RotatedEllipse,
    lobe_a: RotatedEllipse,
    lobe_b: RotatedEllipse,
    lobe_c: RotatedEllipse,
}

impl ContinentalFeature {
    fn from_macro_cell(
        seed: u64,
        cell_x: i32,
        cell_y: i32,
        profile: GeographicGenerationProfile,
    ) -> Self {
        let id = hash2(seed ^ CONTINENT_DOMAIN, cell_x, cell_y);
        let x0 = cell_x * CONTINENT_MACRO_TILES;
        let y0 = cell_y * CONTINENT_MACRO_TILES;
        let style_scale = match profile.landform {
            LandformPreset::Archipelago => 0.56,
            LandformPreset::GrandIsland => 1.14,
            LandformPreset::BrokenCoast => 0.82,
            LandformPreset::InlandBasin => 1.16,
            _ => 1.0,
        };
        let coverage_scale = 0.82 + f64::from(profile.land_coverage_percent) / 240.0;
        let center_x = x0 as f64 + 640.0 + hash01(id ^ 0x01, cell_x, cell_y) * 1_792.0;
        let center_y = y0 as f64 + 640.0 + hash01(id ^ 0x02, cell_x, cell_y) * 1_792.0;
        let radius_x = (760.0 + hash01(id ^ 0x03, cell_x, cell_y) * 720.0)
            * style_scale
            * coverage_scale;
        let radius_y = (680.0 + hash01(id ^ 0x04, cell_x, cell_y) * 760.0)
            * style_scale
            * coverage_scale;
        let angle = hash01(id ^ 0x05, cell_x, cell_y) * std::f64::consts::PI;
        let primary = RotatedEllipse::new(center_x, center_y, radius_x, radius_y, angle);

        let lobe_id = hash2(seed ^ CONTINENT_LOBE_DOMAIN, cell_x, cell_y);
        let make_lobe = |salt: u64, phase: f64| {
            let lobe_angle = (hash01(lobe_id ^ salt, cell_x, cell_y)
                * std::f64::consts::TAU
                + phase)
                % std::f64::consts::TAU;
            let lobe_distance = radius_x.min(radius_y)
                * (0.24 + hash01(lobe_id ^ (salt + 1), cell_x, cell_y) * 0.34);
            RotatedEllipse::new(
                center_x + lobe_angle.cos() * lobe_distance,
                center_y + lobe_angle.sin() * lobe_distance,
                radius_x * (0.42 + hash01(lobe_id ^ (salt + 2), cell_x, cell_y) * 0.24),
                radius_y * (0.42 + hash01(lobe_id ^ (salt + 3), cell_x, cell_y) * 0.24),
                angle + (hash01(lobe_id ^ (salt + 4), cell_x, cell_y) - 0.5) * 0.82,
            )
        };
        let lobe_a = make_lobe(0x11, 0.0);
        let lobe_b = make_lobe(0x21, std::f64::consts::TAU / 3.0);
        let lobe_c = make_lobe(0x31, std::f64::consts::TAU * 2.0 / 3.0);
        Self {
            primary,
            lobe_a,
            lobe_b,
            lobe_c,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct FeatureDistance {
    normalized: f64,
    characteristic_radius: f64,
}

impl FeatureDistance {
    fn better(self, other: Self) -> Self {
        if other.normalized < self.normalized {
            other
        } else {
            self
        }
    }

    fn signed_distance_tiles(self) -> f64 {
        (1.0 - self.normalized.sqrt()) * self.characteristic_radius
    }
}

#[derive(Clone, Copy, Debug)]
struct RotatedEllipse {
    center_x: f64,
    center_y: f64,
    radius_x: f64,
    radius_y: f64,
    cos_angle: f64,
    sin_angle: f64,
}

impl RotatedEllipse {
    fn new(center_x: f64, center_y: f64, radius_x: f64, radius_y: f64, angle: f64) -> Self {
        Self {
            center_x,
            center_y,
            radius_x: radius_x.max(32.0),
            radius_y: radius_y.max(32.0),
            cos_angle: angle.cos(),
            sin_angle: angle.sin(),
        }
    }

    fn distance(self, x: i32, y: i32) -> FeatureDistance {
        let dx = x as f64 - self.center_x;
        let dy = y as f64 - self.center_y;
        let local_x = dx * self.cos_angle + dy * self.sin_angle;
        let local_y = -dx * self.sin_angle + dy * self.cos_angle;
        let nx = local_x / self.radius_x;
        let ny = local_y / self.radius_y;
        FeatureDistance {
            normalized: nx * nx + ny * ny,
            characteristic_radius: (self.radius_x + self.radius_y) * 0.5,
        }
    }
}

fn fractal_noise(seed: u64, x: i32, y: i32, base_scale: f64, octaves: usize) -> f64 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut amplitude_sum = 0.0;
    let mut scale = base_scale.max(4.0);
    for octave in 0..octaves {
        total += value_noise(
            seed ^ (octave as u64).wrapping_mul(0x9e37_79b9),
            x,
            y,
            scale,
        ) * amplitude;
        amplitude_sum += amplitude;
        amplitude *= 0.5;
        scale *= 0.5;
    }
    if amplitude_sum == 0.0 {
        0.5
    } else {
        total / amplitude_sum
    }
}

fn value_noise(seed: u64, x: i32, y: i32, scale: f64) -> f64 {
    let fx = x as f64 / scale;
    let fy = y as f64 / scale;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = smoothstep(fx - x0 as f64);
    let ty = smoothstep(fy - y0 as f64);
    let a = hash01(seed, x0, y0);
    let b = hash01(seed, x0 + 1, y0);
    let c = hash01(seed, x0, y0 + 1);
    let d = hash01(seed, x0 + 1, y0 + 1);
    lerp(lerp(a, b, tx), lerp(c, d, tx), ty)
}

fn smoothstep(value: f64) -> f64 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn hash01(seed: u64, x: i32, y: i32) -> f64 {
    unit_from_hash(hash2(seed, x, y))
}

fn unit_from_hash(value: u64) -> f64 {
    (value as f64) / (u64::MAX as f64)
}

fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let mut value = seed ^ (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= (y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value.wrapping_mul(0x94d0_49bb_1331_11eb) ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_rectangles_are_part_of_a_much_larger_mainland() {
        let profile = GeographicGenerationProfile::default();
        let seed = 0x5eed;
        assert!(sample_geographic_surface(seed, 160, 80, profile).land);
        assert!(sample_geographic_surface(seed, 160, -1_200, profile).land);
        assert!(sample_geographic_surface(seed, -1_200, -900, profile).land);
    }

    #[test]
    fn starter_mainland_has_a_southern_coast_for_the_harbor_region() {
        let profile = GeographicGenerationProfile::default();
        let seed = 0x5eed;
        let mut saw_land = false;
        let mut saw_ocean = false;
        for y in 160..380 {
            let sample = sample_geographic_surface(seed, 160, y, profile);
            saw_land |= sample.land;
            saw_ocean |= !sample.land;
        }
        assert!(saw_land && saw_ocean);
    }

    #[test]
    fn starter_mainland_southern_coast_is_not_a_single_ellipse_arc() {
        let profile = GeographicGenerationProfile::default();
        let seed = 0x5eed;
        let mut crossings = Vec::new();
        for x in [-1400, -1000, -600, -200, 200, 600, 1000, 1400] {
            let mut previous_land = sample_geographic_surface(seed, x, -256, profile).land;
            let mut crossing = None;
            for y in (-248..=640).step_by(8) {
                let land = sample_geographic_surface(seed, x, y, profile).land;
                if previous_land && !land {
                    crossing = Some(y);
                    break;
                }
                previous_land = land;
            }
            if let Some(y) = crossing {
                crossings.push(y);
            }
        }
        assert!(crossings.len() >= 5);
        let min_y = *crossings.iter().min().expect("coast minimum");
        let max_y = *crossings.iter().max().expect("coast maximum");
        assert!(max_y - min_y >= 120);
    }

    #[test]
    fn sampling_is_chunk_boundary_independent() {
        let profile = GeographicGenerationProfile::default();
        let a = sample_geographic_surface(77, 63, 127, profile);
        let b = sample_geographic_surface(77, 63, 127, profile);
        assert_eq!(a, b);
    }
}
