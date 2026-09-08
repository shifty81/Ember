use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};
use serde::Deserialize;
use std::{fs::read_to_string, path::Path, sync::OnceLock};

use crate::terrain_material_bindings::canonical_corner_tuple_material;

const LPC_MAPPED_TERRAIN_MANIFEST_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/lpc_mapped_terrain_v7_32.json";

static MATERIAL_TUPLES: OnceLock<Result<Vec<[String; 4]>, String>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LpcAuthoredContactRepairReport {
    pub mountain_path_shore_shoulders: usize,
}

impl LpcAuthoredContactRepairReport {
    pub const fn total_mutations(self) -> usize {
        self.mountain_path_shore_shoulders
    }
}

pub fn lpc_mapped_terrain_supports_tile_pair(first: TileKind, second: TileKind) -> bool {
    if first == second || (is_water_tile(first) && is_water_tile(second)) {
        return true;
    }
    let Some(first_material) = canonical_corner_tuple_material(first) else {
        return true;
    };
    let Some(second_material) = canonical_corner_tuple_material(second) else {
        return true;
    };
    material_tuples().is_some_and(|tuples| {
        tuples.iter().any(|corners| {
            let has_first = corners.iter().any(|corner| corner == first_material);
            let has_second = corners.iter().any(|corner| corner == second_material);
            has_first && has_second
        })
    })
}

pub fn normalize_lpc_authored_material_contacts_region(
    map: &mut TavernMap,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> LpcAuthoredContactRepairReport {
    let snapshot = map.tiles.clone();
    let mut report = LpcAuthoredContactRepairReport::default();
    for y in (min_y - 1).max(0)..=(max_y + 1).min(MAP_H as i32 - 1) {
        for x in (min_x - 1).max(0)..=(max_x + 1).min(MAP_W as i32 - 1) {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            if snapshot[index] != TileKind::MountainPath {
                continue;
            }
            let touches_shore = [(0, -1), (1, 0), (0, 1), (-1, 0)]
                .iter()
                .filter_map(|(ox, oy)| TavernMap::idx(x + ox, y + oy))
                .map(|slot| snapshot[slot])
                .any(|tile| matches!(tile, TileKind::Sand | TileKind::WetSand));
            if touches_shore {
                // V7 Dirt_Roots has no authored tuple with V7 Sand. V7 Dirt_Tan
                // has exact tuples with both sides, so it
                // becomes a one-cell road shoulder instead of invented pixels.
                map.set(x, y, TileKind::Road);
                report.mountain_path_shore_shoulders += 1;
            }
        }
    }
    report
}

fn material_tuples() -> Option<&'static [[String; 4]]> {
    MATERIAL_TUPLES
        .get_or_init(load_material_tuples)
        .as_ref()
        .ok()
        .map(Vec::as_slice)
}

fn load_material_tuples() -> Result<Vec<[String; 4]>, String> {
    let path = repo_root_dir().join(LPC_MAPPED_TERRAIN_MANIFEST_PATH);
    let raw = read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(manifest
        .entries
        .into_iter()
        .map(|entry| {
            [
                entry.corners.top_left,
                entry.corners.top_right,
                entry.corners.bottom_left,
                entry.corners.bottom_right,
            ]
        })
        .collect())
}

fn repo_root_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

fn is_water_tile(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
    )
}

#[derive(Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

#[derive(Deserialize)]
struct ManifestEntry {
    corners: ManifestCorners,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestCorners {
    top_left: String,
    top_right: String,
    bottom_left: String,
    bottom_right: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountain_path_and_sand_require_an_authored_road_shoulder() {
        assert!(!lpc_mapped_terrain_supports_tile_pair(
            TileKind::MountainPath,
            TileKind::Sand,
        ));
        assert!(lpc_mapped_terrain_supports_tile_pair(
            TileKind::Road,
            TileKind::Sand,
        ));
        assert!(lpc_mapped_terrain_supports_tile_pair(
            TileKind::MountainPath,
            TileKind::Road,
        ));
    }

    #[test]
    fn mountain_path_touching_sand_is_repaired_with_an_authored_road_shoulder() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(10, 10, TileKind::MountainPath);
        map.set(11, 10, TileKind::Sand);

        let report = normalize_lpc_authored_material_contacts_region(&mut map, 10, 10, 11, 10);

        assert_eq!(report.mountain_path_shore_shoulders, 1);
        assert_eq!(map.get(10, 10), TileKind::Road);
        assert_eq!(map.get(11, 10), TileKind::Sand);
    }

    #[test]
    fn mountain_path_touching_general_gravel_keeps_its_v7_material() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(10, 10, TileKind::MountainPath);
        map.set(11, 10, TileKind::PebbleShore);

        let report = normalize_lpc_authored_material_contacts_region(&mut map, 10, 10, 11, 10);

        assert_eq!(report.total_mutations(), 0);
        assert_eq!(map.get(10, 10), TileKind::MountainPath);
        assert_eq!(map.get(11, 10), TileKind::PebbleShore);
    }

    #[test]
    fn mountain_path_not_touching_shore_keeps_its_v7_material() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(10, 10, TileKind::MountainPath);
        map.set(11, 10, TileKind::Road);

        let report = normalize_lpc_authored_material_contacts_region(&mut map, 10, 10, 11, 10);

        assert_eq!(report.total_mutations(), 0);
        assert_eq!(map.get(10, 10), TileKind::MountainPath);
    }
}
