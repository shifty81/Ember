//! Coherent macro-landform sampling for Havenwild's continuous surfaces.
//!
//! The old generator promoted raw noise contours directly into visible cliffs
//! and highland materials. That produced long diagonal ribbons and repeated
//! "tiger stripe" bands. This module instead generates bounded geographic
//! features (overlapping rotated plateau ellipses) on a deterministic macro
//! lattice. Noise may still describe fine geology/hydrology, but it no longer
//! owns visible structural topology.

use serde::{Deserialize, Serialize};

pub const GEOGRAPHIC_LANDFORM_SCHEMA: &str = "havenwild.geographic_landforms.v0_1";

const MACRO_CELL_TILES: i32 = 192;
const NEIGHBOR_MACRO_RADIUS: i32 = 1;
const FEATURE_DOMAIN: u64 = 0x4745_4f46_4541_5431;
const LOBE_DOMAIN: u64 = 0x4c4f_4245_4645_4154;
const CORE_DOMAIN: u64 = 0x434f_5245_4645_4154;
const FOREST_DOMAIN: u64 = 0x464f_5245_5354_3031;
const FOREST_MACRO_CELL_TILES: i32 = 224;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeographicLandformKind {
    #[default]
    Lowland,
    UplandPlateau,
    HighlandCore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeographicLandformSample {
    pub kind: GeographicLandformKind,
    pub structural_level: u8,
    pub feature_id: u64,
}

impl GeographicLandformSample {
    pub const fn is_upland(self) -> bool {
        self.structural_level > 0
    }

    pub const fn is_highland_core(self) -> bool {
        self.structural_level >= 2
    }
}

/// Returns the deterministic macro-geographic landform at one global tile.
///
/// Features are finite, broad shapes. Sampling a tile never compares a raw
/// noise value to a cliff threshold, so the result cannot become an unbounded
/// contour stripe. Neighboring storage chunks sample the same global features
/// and therefore stitch without chunk-edge special cases.
pub fn sample_geographic_landform(
    seed: u64,
    global_x: i32,
    global_y: i32,
    mountain_strength: f32,
) -> GeographicLandformSample {
    let strength = mountain_strength.clamp(0.0, 1.0) as f64;
    let macro_x = global_x.div_euclid(MACRO_CELL_TILES);
    let macro_y = global_y.div_euclid(MACRO_CELL_TILES);

    let mut best = GeographicLandformSample::default();
    let mut best_depth = f64::INFINITY;

    for cell_y in macro_y - NEIGHBOR_MACRO_RADIUS..=macro_y + NEIGHBOR_MACRO_RADIUS {
        for cell_x in macro_x - NEIGHBOR_MACRO_RADIUS..=macro_x + NEIGHBOR_MACRO_RADIUS {
            let feature_hash = hash2(seed ^ FEATURE_DOMAIN, cell_x, cell_y);
            if !feature_enabled(feature_hash, strength) {
                continue;
            }

            let feature = PlateauFeature::from_macro_cell(seed, cell_x, cell_y, strength);
            let primary_depth = feature.primary.normalized_distance(global_x, global_y);
            let lobe_depth = feature.lobe.normalized_distance(global_x, global_y);
            let depth = primary_depth.min(lobe_depth);
            if depth > 1.0 || depth >= best_depth {
                continue;
            }

            let core_depth = feature.core.normalized_distance(global_x, global_y);
            let structural_level = if feature.has_core && core_depth <= 1.0 {
                2
            } else {
                1
            };
            best_depth = depth;
            best = GeographicLandformSample {
                kind: if structural_level >= 2 {
                    GeographicLandformKind::HighlandCore
                } else {
                    GeographicLandformKind::UplandPlateau
                },
                structural_level,
                feature_id: feature.id,
            };
        }
    }

    best
}


/// Returns a broad deterministic forest-habitat weight in the range `0..=1`.
///
/// Forests are finite geographic patches rather than a raw-noise threshold.
/// The weight deliberately feathers at the perimeter so object population can
/// produce a dense interior, sparse woodland edge, and open meadow outside.
/// A deterministic clearing may be cut from the center of some forest patches.
pub fn geographic_forest_habitat(seed: u64, global_x: i32, global_y: i32) -> f32 {
    let macro_x = global_x.div_euclid(FOREST_MACRO_CELL_TILES);
    let macro_y = global_y.div_euclid(FOREST_MACRO_CELL_TILES);
    let mut best = 0.0_f64;

    for cell_y in macro_y - 1..=macro_y + 1 {
        for cell_x in macro_x - 1..=macro_x + 1 {
            let id = hash2(seed ^ FOREST_DOMAIN, cell_x, cell_y);
            if unit_from_hash(id) >= 0.62 {
                continue;
            }

            let x0 = cell_x * FOREST_MACRO_CELL_TILES;
            let y0 = cell_y * FOREST_MACRO_CELL_TILES;
            let center_x = x0 as f64 + 52.0 + hash01(id ^ 0x31, cell_x, cell_y) * 120.0;
            let center_y = y0 as f64 + 52.0 + hash01(id ^ 0x32, cell_x, cell_y) * 120.0;
            let radius_x = 48.0 + hash01(id ^ 0x33, cell_x, cell_y) * 42.0;
            let radius_y = 38.0 + hash01(id ^ 0x34, cell_x, cell_y) * 38.0;
            let angle = hash01(id ^ 0x35, cell_x, cell_y) * std::f64::consts::PI;
            let grove = RotatedEllipse::new(center_x, center_y, radius_x, radius_y, angle);
            let distance = grove.normalized_distance(global_x, global_y);
            if distance > 1.0 {
                continue;
            }

            // normalized_distance is squared elliptical distance. Converting it
            // to a radial distance gives a generous dense core and a broad,
            // naturally thinning edge rather than a hard tree wall.
            let radial = distance.sqrt();
            let mut weight = ((1.0 - radial) / 0.55).clamp(0.0, 1.0);

            // Roughly one third of groves contain a coherent meadow clearing.
            if unit_from_hash(id ^ 0x434c_4541_5249_4e47) < 0.34 {
                let clearing = RotatedEllipse::new(
                    center_x + (hash01(id ^ 0x36, cell_x, cell_y) - 0.5) * radius_x * 0.34,
                    center_y + (hash01(id ^ 0x37, cell_x, cell_y) - 0.5) * radius_y * 0.34,
                    radius_x * (0.16 + hash01(id ^ 0x38, cell_x, cell_y) * 0.08),
                    radius_y * (0.16 + hash01(id ^ 0x39, cell_x, cell_y) * 0.08),
                    angle,
                );
                let clearing_distance = clearing.normalized_distance(global_x, global_y);
                if clearing_distance <= 1.0 {
                    weight *= clearing_distance.sqrt().clamp(0.0, 1.0);
                }
            }

            best = best.max(weight);
        }
    }

    best as f32
}

pub fn geographic_structural_level(
    seed: u64,
    global_x: i32,
    global_y: i32,
    mountain_strength: f32,
) -> u8 {
    sample_geographic_landform(seed, global_x, global_y, mountain_strength).structural_level
}

fn feature_enabled(hash: u64, strength: f64) -> bool {
    // Roughly 36-63% of macro cells host an upland feature. Because each
    // feature occupies only part of its 192x192 macro cell, actual raised-land
    // coverage remains broad but restrained.
    let threshold = 0.36 + strength * 0.27;
    unit_from_hash(hash) < threshold
}

#[derive(Clone, Copy, Debug)]
struct PlateauFeature {
    id: u64,
    primary: RotatedEllipse,
    lobe: RotatedEllipse,
    core: RotatedEllipse,
    has_core: bool,
}

impl PlateauFeature {
    fn from_macro_cell(seed: u64, cell_x: i32, cell_y: i32, strength: f64) -> Self {
        let id = hash2(seed ^ FEATURE_DOMAIN, cell_x, cell_y);
        let x0 = cell_x * MACRO_CELL_TILES;
        let y0 = cell_y * MACRO_CELL_TILES;

        // Keep feature centers away from the exact macro-cell border while
        // allowing their radii/lobes to cross it naturally.
        let center_x = x0 as f64 + 48.0 + hash01(id ^ 0x01, cell_x, cell_y) * 96.0;
        let center_y = y0 as f64 + 48.0 + hash01(id ^ 0x02, cell_x, cell_y) * 96.0;
        let radius_scale = 0.88 + strength * 0.32;
        let radius_x = (34.0 + hash01(id ^ 0x03, cell_x, cell_y) * 28.0) * radius_scale;
        let radius_y = (26.0 + hash01(id ^ 0x04, cell_x, cell_y) * 24.0) * radius_scale;
        let angle = hash01(id ^ 0x05, cell_x, cell_y) * std::f64::consts::PI;

        let primary = RotatedEllipse::new(center_x, center_y, radius_x, radius_y, angle);

        // A second overlapping lobe breaks the perfect oval silhouette without
        // turning the boundary back into a noise contour.
        let lobe_hash = hash2(seed ^ LOBE_DOMAIN, cell_x, cell_y);
        let lobe_angle = hash01(lobe_hash ^ 0x11, cell_x, cell_y) * std::f64::consts::TAU;
        let lobe_distance = radius_x.min(radius_y)
            * (0.22 + hash01(lobe_hash ^ 0x12, cell_x, cell_y) * 0.22);
        let lobe_center_x = center_x + lobe_angle.cos() * lobe_distance;
        let lobe_center_y = center_y + lobe_angle.sin() * lobe_distance;
        let lobe = RotatedEllipse::new(
            lobe_center_x,
            lobe_center_y,
            radius_x * (0.62 + hash01(lobe_hash ^ 0x13, cell_x, cell_y) * 0.18),
            radius_y * (0.62 + hash01(lobe_hash ^ 0x14, cell_x, cell_y) * 0.18),
            angle + (hash01(lobe_hash ^ 0x15, cell_x, cell_y) - 0.5) * 0.45,
        );

        let core_hash = hash2(seed ^ CORE_DOMAIN, cell_x, cell_y);
        let has_core = unit_from_hash(core_hash) < 0.42 + strength * 0.28;
        let core = RotatedEllipse::new(
            center_x + (hash01(core_hash ^ 0x21, cell_x, cell_y) - 0.5) * radius_x * 0.16,
            center_y + (hash01(core_hash ^ 0x22, cell_x, cell_y) - 0.5) * radius_y * 0.16,
            radius_x * (0.34 + hash01(core_hash ^ 0x23, cell_x, cell_y) * 0.10),
            radius_y * (0.34 + hash01(core_hash ^ 0x24, cell_x, cell_y) * 0.10),
            angle,
        );

        Self {
            id,
            primary,
            lobe,
            core,
            has_core,
        }
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
            radius_x: radius_x.max(8.0),
            radius_y: radius_y.max(8.0),
            cos_angle: angle.cos(),
            sin_angle: angle.sin(),
        }
    }

    fn normalized_distance(self, x: i32, y: i32) -> f64 {
        let dx = x as f64 - self.center_x;
        let dy = y as f64 - self.center_y;
        let local_x = dx * self.cos_angle + dy * self.sin_angle;
        let local_y = -dx * self.sin_angle + dy * self.cos_angle;
        let nx = local_x / self.radius_x;
        let ny = local_y / self.radius_y;
        nx * nx + ny * ny
    }
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
    fn geographic_landforms_are_deterministic() {
        let a = sample_geographic_landform(77, 481, -209, 0.62);
        let b = sample_geographic_landform(77, 481, -209, 0.62);
        assert_eq!(a, b);
    }

    #[test]
    fn raised_cells_form_broad_neighborhoods_instead_of_single_cell_stripes() {
        let seed = 0x4841_5645_4e57_494c;
        let strength = 0.60;
        let mut raised = 0usize;
        let mut isolated = 0usize;
        for y in -256..256 {
            for x in -256..256 {
                if geographic_structural_level(seed, x, y, strength) == 0 {
                    continue;
                }
                raised += 1;
                let support = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .into_iter()
                    .filter(|(ox, oy)| {
                        geographic_structural_level(seed, x + *ox, y + *oy, strength) > 0
                    })
                    .count();
                isolated += usize::from(support == 0);
            }
        }
        assert!(raised > 0);
        assert_eq!(isolated, 0);
    }

    #[test]
    fn forest_habitat_forms_large_patches_with_open_ground() {
        let seed = 0x464f_5245_5354_5445;
        let mut woodland = 0usize;
        let mut open = 0usize;
        for y in -320..320 {
            for x in -320..320 {
                if geographic_forest_habitat(seed, x, y) > 0.30 {
                    woodland += 1;
                } else {
                    open += 1;
                }
            }
        }
        assert!(woodland > 0);
        assert!(open > woodland / 4);
    }
}
