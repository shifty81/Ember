//! Deterministic finite archipelago skeleton for Havenwild's wrapped overworld.
//!
//! This pass reserves one mainland, at least ten major biome islands, optional
//! minor buildable islands, ocean-depth bands, and authored anchor regions.

use crate::open_world::WorldTileCoord;
use crate::WorldTopologyConfig;
use serde::{Deserialize, Serialize};

pub const ARCHIPELAGO_SKELETON_SCHEMA: &str = "havenwild.archipelago_skeleton.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandmassClass {
    Mainland,
    MajorIsland,
    MinorIsland,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomeIdentity {
    TemperateHeartland,
    CoastalMeadow,
    AncientForest,
    Highland,
    Marsh,
    AmberDesert,
    Frost,
    Volcanic,
    Tropical,
    AutumnWoodland,
    StormCoast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OceanDepthBand {
    Shore,
    Shallow,
    Shelf,
    Deep,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileRect {
    pub min_x: i32,
    pub min_y: i32,
    pub width: i32,
    pub height: i32,
}
impl TileRect {
    pub fn contains(&self, tile: WorldTileCoord) -> bool {
        tile.x >= self.min_x
            && tile.x < self.min_x + self.width
            && tile.y >= self.min_y
            && tile.y < self.min_y + self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredAnchorReservation {
    pub id: String,
    pub purpose: String,
    pub bounds: TileRect,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandmassSkeleton {
    pub id: String,
    pub name: String,
    pub class: LandmassClass,
    pub biome: BiomeIdentity,
    pub center: WorldTileCoord,
    pub radius_x_tiles: i32,
    pub radius_y_tiles: i32,
    pub buildable: bool,
    pub authored_anchors: Vec<AuthoredAnchorReservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchipelagoSkeleton {
    pub schema: String,
    pub seed: u64,
    pub world_width_tiles: i32,
    pub world_height_tiles: i32,
    pub shore_band_tiles: i32,
    pub shallow_band_tiles: i32,
    pub shelf_band_tiles: i32,
    pub landmasses: Vec<LandmassSkeleton>,
}

impl ArchipelagoSkeleton {
    pub fn generate(topology: &WorldTopologyConfig, seed: u64, minor_island_count: usize) -> Self {
        let w = topology.width_tiles;
        let h = topology.height_tiles;
        let mut landmasses = Vec::new();
        landmasses.push(landmass(LandmassSpec {
            id: "mainland".into(),
            name: "Havenwild Mainland".into(),
            class: LandmassClass::Mainland,
            biome: BiomeIdentity::TemperateHeartland,
            center: WorldTileCoord::new(w / 2, h / 2),
            radius_x_tiles: w / 5,
            radius_y_tiles: h / 4,
            buildable: true,
            authored_anchors: vec![anchor(
                "capital_civic_district",
                "Capital, land office, harbor, roads, and starter-region authored reserve",
                w / 2 - w / 20,
                h / 2 - h / 20,
                w / 10,
                h / 10,
                true,
            )],
        }));
        let biomes = [
            BiomeIdentity::CoastalMeadow,
            BiomeIdentity::AncientForest,
            BiomeIdentity::Highland,
            BiomeIdentity::Marsh,
            BiomeIdentity::AmberDesert,
            BiomeIdentity::Frost,
            BiomeIdentity::Volcanic,
            BiomeIdentity::Tropical,
            BiomeIdentity::AutumnWoodland,
            BiomeIdentity::StormCoast,
        ];
        for (i, biome) in biomes.into_iter().enumerate() {
            let angle = (i as f64 / 10.0) * std::f64::consts::TAU + unit(seed, i as u64) * 0.22;
            let ring_x = w as f64 * 0.37;
            let ring_y = h as f64 * 0.34;
            let x = (w as f64 / 2.0 + angle.cos() * ring_x) as i32;
            let y = (h as f64 / 2.0 + angle.sin() * ring_y) as i32;
            landmasses.push(landmass(LandmassSpec {
                id: format!("major_{:02}", i + 1),
                name: format!("Major Island {}", i + 1),
                class: LandmassClass::MajorIsland,
                biome,
                center: WorldTileCoord::new(x.rem_euclid(w), y.clamp(0, h - 1)),
                radius_x_tiles: w / 18,
                radius_y_tiles: h / 15,
                buildable: true,
                authored_anchors: vec![anchor(
                    &format!("major_{:02}_settlement_reserve", i + 1),
                    "Biome-dependent settlement, harbor, road, and landmark reserve",
                    (x - w / 50).rem_euclid(w),
                    (y - h / 50).clamp(0, h - 1),
                    w / 25,
                    h / 25,
                    true,
                )],
            }));
        }
        for i in 0..minor_island_count {
            let a = unit(seed ^ 0xa5a5_55aa, i as u64) * std::f64::consts::TAU;
            let r = 0.24 + unit(seed ^ 0x55aa_a5a5, i as u64) * 0.20;
            let x = (w as f64 / 2.0 + a.cos() * w as f64 * r) as i32;
            let y = (h as f64 / 2.0 + a.sin() * h as f64 * (r * 0.78)) as i32;
            landmasses.push(landmass(LandmassSpec {
                id: format!("minor_{:02}", i + 1),
                name: format!("Minor Island {}", i + 1),
                class: LandmassClass::MinorIsland,
                biome: BiomeIdentity::CoastalMeadow,
                center: WorldTileCoord::new(x.rem_euclid(w), y.clamp(0, h - 1)),
                radius_x_tiles: (w / 55).max(12),
                radius_y_tiles: (h / 55).max(12),
                buildable: true,
                authored_anchors: Vec::new(),
            }));
        }
        Self {
            schema: ARCHIPELAGO_SKELETON_SCHEMA.into(),
            seed,
            world_width_tiles: w,
            world_height_tiles: h,
            shore_band_tiles: 2,
            shallow_band_tiles: 8,
            shelf_band_tiles: 24,
            landmasses,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != ARCHIPELAGO_SKELETON_SCHEMA {
            return Err("unsupported archipelago skeleton schema".into());
        }
        if self.world_width_tiles <= 0 || self.world_height_tiles <= 0 {
            return Err("world dimensions must be positive".into());
        }
        if self
            .landmasses
            .iter()
            .filter(|l| l.class == LandmassClass::Mainland)
            .count()
            != 1
        {
            return Err("exactly one mainland is required".into());
        }
        if self
            .landmasses
            .iter()
            .filter(|l| l.class == LandmassClass::MajorIsland)
            .count()
            < 10
        {
            return Err("at least ten major islands are required".into());
        }
        if self
            .landmasses
            .iter()
            .any(|l| l.radius_x_tiles <= 0 || l.radius_y_tiles <= 0)
        {
            return Err("landmass radii must be positive".into());
        }
        if !self
            .landmasses
            .iter()
            .filter(|l| l.class != LandmassClass::MinorIsland)
            .all(|l| !l.authored_anchors.is_empty())
        {
            return Err("mainland and major islands require authored anchor reservations".into());
        }
        Ok(())
    }

    pub fn ocean_depth_band(&self, distance_to_land_tiles: i32) -> OceanDepthBand {
        if distance_to_land_tiles <= self.shore_band_tiles {
            OceanDepthBand::Shore
        } else if distance_to_land_tiles <= self.shallow_band_tiles {
            OceanDepthBand::Shallow
        } else if distance_to_land_tiles <= self.shelf_band_tiles {
            OceanDepthBand::Shelf
        } else {
            OceanDepthBand::Deep
        }
    }
}

struct LandmassSpec {
    id: String,
    name: String,
    class: LandmassClass,
    biome: BiomeIdentity,
    center: WorldTileCoord,
    radius_x_tiles: i32,
    radius_y_tiles: i32,
    buildable: bool,
    authored_anchors: Vec<AuthoredAnchorReservation>,
}

fn landmass(spec: LandmassSpec) -> LandmassSkeleton {
    LandmassSkeleton {
        id: spec.id,
        name: spec.name,
        class: spec.class,
        biome: spec.biome,
        center: spec.center,
        radius_x_tiles: spec.radius_x_tiles.max(1),
        radius_y_tiles: spec.radius_y_tiles.max(1),
        buildable: spec.buildable,
        authored_anchors: spec.authored_anchors,
    }
}
fn anchor(
    id: &str,
    purpose: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    required: bool,
) -> AuthoredAnchorReservation {
    AuthoredAnchorReservation {
        id: id.into(),
        purpose: purpose.into(),
        bounds: TileRect {
            min_x: x,
            min_y: y,
            width: width.max(1),
            height: height.max(1),
        },
        required,
    }
}
fn unit(seed: u64, index: u64) -> f64 {
    let mut z = seed.wrapping_add(index.wrapping_mul(0x9e3779b97f4a7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) as f64) / (u64::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_world::WorldSurfaceConfig;
    #[test]
    fn skeleton_has_mainland_and_ten_major_islands() {
        let t = WorldTopologyConfig::from_surface(&WorldSurfaceConfig::standard(7));
        let s = ArchipelagoSkeleton::generate(&t, 7, 8);
        s.validate().unwrap();
        assert_eq!(
            s.landmasses
                .iter()
                .filter(|l| l.class == LandmassClass::Mainland)
                .count(),
            1
        );
        assert_eq!(
            s.landmasses
                .iter()
                .filter(|l| l.class == LandmassClass::MajorIsland)
                .count(),
            10
        );
    }
    #[test]
    fn ocean_bands_are_ordered() {
        let t = WorldTopologyConfig::from_surface(&WorldSurfaceConfig::standard(7));
        let s = ArchipelagoSkeleton::generate(&t, 7, 0);
        assert_eq!(s.ocean_depth_band(1), OceanDepthBand::Shore);
        assert_eq!(s.ocean_depth_band(7), OceanDepthBand::Shallow);
        assert_eq!(s.ocean_depth_band(20), OceanDepthBand::Shelf);
        assert_eq!(s.ocean_depth_band(40), OceanDepthBand::Deep);
    }
}
