use std::collections::BTreeSet;

use haven_assets::authored_terrain_contacts::{
    lpc_mapped_terrain_supports_tile_pair, normalize_lpc_authored_material_contacts_region,
};
use haven_core::{TavernMap, TerrainPaintMode, MAP_H, MAP_W};
use haven_world::{
    normalize_shore_water_lifecycle_region, resolve_tavern_map_hydrology_v2, DirtyRegion,
    HydrologySettingsV2,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainPaintModeReport {
    pub shoreline_mutations: usize,
    pub hydrology_mutations: usize,
    pub authored_contact_mutations: usize,
    pub unsupported_contacts: usize,
}

impl TerrainPaintModeReport {
    pub const fn total_mutations(self) -> usize {
        self.shoreline_mutations + self.hydrology_mutations + self.authored_contact_mutations
    }
}

/// Apply the canonical post-paint terrain policy shared by the native editor,
/// player World Builder, automation and runtime development tools.
pub fn apply_terrain_paint_mode_to_map(
    map: &mut TavernMap,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    mode: TerrainPaintMode,
) -> Result<TerrainPaintModeReport, String> {
    const LOCAL_PAD: i32 = 4;
    let scan_min_x = (min_x - LOCAL_PAD).max(0);
    let scan_min_y = (min_y - LOCAL_PAD).max(0);
    let scan_max_x = (max_x + LOCAL_PAD).min(MAP_W as i32 - 1);
    let scan_max_y = (max_y + LOCAL_PAD).min(MAP_H as i32 - 1);

    match mode {
        TerrainPaintMode::Exact => Ok(TerrainPaintModeReport {
            unsupported_contacts: count_unsupported_lpc_contacts(map, min_x, min_y, max_x, max_y),
            ..TerrainPaintModeReport::default()
        }),
        TerrainPaintMode::Coastline => {
            let shoreline = normalize_shore_water_lifecycle_region(
                map, scan_min_x, scan_min_y, scan_max_x, scan_max_y, 3,
            );
            let contacts = normalize_lpc_authored_material_contacts_region(
                map, scan_min_x, scan_min_y, scan_max_x, scan_max_y,
            );
            Ok(TerrainPaintModeReport {
                shoreline_mutations: shoreline.total_mutations(),
                hydrology_mutations: 0,
                authored_contact_mutations: contacts.total_mutations(),
                unsupported_contacts: count_unsupported_lpc_contacts(
                    map, min_x, min_y, max_x, max_y,
                ),
            })
        }
        TerrainPaintMode::Hydrology => {
            let hydrology = resolve_tavern_map_hydrology_v2(
                map,
                DirtyRegion {
                    min_x: scan_min_x,
                    min_y: scan_min_y,
                    max_x: scan_max_x,
                    max_y: scan_max_y,
                },
                HydrologySettingsV2::default(),
            )?;
            let contacts = normalize_lpc_authored_material_contacts_region(
                map, scan_min_x, scan_min_y, scan_max_x, scan_max_y,
            );
            Ok(TerrainPaintModeReport {
                shoreline_mutations: 0,
                hydrology_mutations: hydrology.total_depth_mutations(),
                authored_contact_mutations: contacts.total_mutations(),
                unsupported_contacts: count_unsupported_lpc_contacts(
                    map, min_x, min_y, max_x, max_y,
                ),
            })
        }
    }
}

fn count_unsupported_lpc_contacts(
    map: &TavernMap,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> usize {
    let mut contacts = BTreeSet::new();
    for y in min_y.max(0)..=max_y.min(MAP_H as i32 - 1) {
        for x in min_x.max(0)..=max_x.min(MAP_W as i32 - 1) {
            let tile = map.get(x, y);
            for (ox, oy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let nx = x + ox;
                let ny = y + oy;
                if TavernMap::idx(nx, ny).is_none() {
                    continue;
                }
                let neighbor = map.get(nx, ny);
                if tile == neighbor || lpc_mapped_terrain_supports_tile_pair(tile, neighbor) {
                    continue;
                }
                let first = (x, y);
                let second = (nx, ny);
                contacts.insert(if first <= second {
                    (first, second)
                } else {
                    (second, first)
                });
            }
        }
    }
    contacts.len()
}
