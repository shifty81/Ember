use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
};

use haven_core::{TavernMap, TileKind, MAP_H, MAP_W};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShoreWaterLifecycleReport {
    pub deep_to_shallow: usize,
    pub shallow_to_deep: usize,
    pub dry_sand_to_wet: usize,
    pub stale_wet_to_dry: usize,
    pub generated_foam: usize,
    pub removed_stale_foam: usize,
    pub generated_river_mouths: usize,
    pub removed_stale_river_mouths: usize,
    pub unsupported_depth_topology_to_shallow: usize,
}

impl ShoreWaterLifecycleReport {
    pub fn total_mutations(self) -> usize {
        self.deep_to_shallow
            + self.shallow_to_deep
            + self.dry_sand_to_wet
            + self.stale_wet_to_dry
            + self.generated_foam
            + self.removed_stale_foam
            + self.generated_river_mouths
            + self.removed_stale_river_mouths
            + self.unsupported_depth_topology_to_shallow
    }

    pub fn status_line(self) -> String {
        format!(
            "shore lifecycle: {} changed, wet +{}/-{}, foam +{}/-{}, mouths +{}/-{}, depth {}/{}, authored-topology {}",
            self.total_mutations(),
            self.dry_sand_to_wet,
            self.stale_wet_to_dry,
            self.generated_foam,
            self.removed_stale_foam,
            self.generated_river_mouths,
            self.removed_stale_river_mouths,
            self.deep_to_shallow,
            self.shallow_to_deep,
            self.unsupported_depth_topology_to_shallow,
        )
    }
}

/// Normalizes the generated shoreline lifecycle in a bounded edit region.
///
/// Normalizes only semantic water depth in the authoritative gameplay map.
///
/// Presentation-only shoreline treatments such as wet sand, shore foam, and
/// river-mouth blends are resolved by the terrain presentation cache. They must
/// never rewrite gameplay terrain or influence collision. Deep water and deep
/// ocean still receive a shallow semantic buffer before contacting land.
pub fn normalize_shore_water_lifecycle_region(
    map: &mut TavernMap,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    _passes: usize,
) -> ShoreWaterLifecycleReport {
    const CARDINAL_COST: usize = 10;
    const DIAGONAL_COST: usize = 14;
    const SHALLOW_BAND_COST: usize = 18;

    let mut report = ShoreWaterLifecycleReport::default();
    let snapshot = map.tiles.clone();
    let mut distance = vec![usize::MAX; snapshot.len()];
    let mut queue: BinaryHeap<(Reverse<usize>, i32, i32)> = BinaryHeap::new();

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = snapshot[index];
            if !is_depth_water(tile) {
                continue;
            }
            // Only in-map non-water neighbors define a shoreline. The finite
            // map boundary is not a beach, so open ocean remains deep at the
            // streaming/world edge instead of gaining a false shallow frame.
            let boundary = [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ]
            .iter()
            .any(|&(ox, oy)| {
                TavernMap::idx(x + ox, y + oy).is_some_and(|slot| !is_depth_water(snapshot[slot]))
            });
            if boundary {
                distance[index] = 0;
                queue.push((Reverse(0), x, y));
            }
        }
    }

    while let Some((Reverse(cost), x, y)) = queue.pop() {
        let Some(index) = TavernMap::idx(x, y) else {
            continue;
        };
        if cost != distance[index] {
            continue;
        }
        for (ox, oy, step) in [
            (-1, -1, DIAGONAL_COST),
            (0, -1, CARDINAL_COST),
            (1, -1, DIAGONAL_COST),
            (-1, 0, CARDINAL_COST),
            (1, 0, CARDINAL_COST),
            (-1, 1, DIAGONAL_COST),
            (0, 1, CARDINAL_COST),
            (1, 1, DIAGONAL_COST),
        ] {
            let Some(neighbor_index) = TavernMap::idx(x + ox, y + oy) else {
                continue;
            };
            if !is_depth_water(snapshot[neighbor_index]) {
                continue;
            }
            let next = cost.saturating_add(step);
            if next < distance[neighbor_index] {
                distance[neighbor_index] = next;
                queue.push((Reverse(next), x + ox, y + oy));
            }
        }
    }

    let force_shallow_component =
        shallow_only_depth_components(&snapshot, &distance, SHALLOW_BAND_COST);

    let scan_min_x = min_x.max(0);
    let scan_min_y = min_y.max(0);
    let scan_max_x = max_x.min(MAP_W as i32 - 1);
    let scan_max_y = max_y.min(MAP_H as i32 - 1);

    for y in scan_min_y..=scan_max_y {
        for x in scan_min_x..=scan_max_x {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = snapshot[index];
            if !is_depth_water(tile) {
                continue;
            }

            let resolved = if matches!(tile, TileKind::RiverWater | TileKind::RiverMouthBlend) {
                TileKind::RiverWater
            } else if force_shallow_component[index] || distance[index] <= SHALLOW_BAND_COST {
                shallow_variant(tile)
            } else {
                deep_variant(tile)
            };

            if resolved == tile {
                continue;
            }
            match (tile, resolved) {
                (
                    TileKind::DeepWater | TileKind::OceanDeep,
                    TileKind::ShallowWater | TileKind::OceanShallow,
                ) => report.deep_to_shallow += 1,
                (
                    TileKind::ShallowWater | TileKind::OceanShallow | TileKind::Water,
                    TileKind::DeepWater | TileKind::OceanDeep,
                ) => report.shallow_to_deep += 1,
                _ => {}
            }
            map.set(x, y, resolved);
        }
    }

    report.unsupported_depth_topology_to_shallow += normalize_authored_depth_topology_region(
        map, scan_min_x, scan_min_y, scan_max_x, scan_max_y,
    );

    report
}

pub(super) fn normalize_authored_depth_topology_region(
    map: &mut TavernMap,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
) -> usize {
    const NORTH: u8 = 1 << 0;
    const EAST: u8 = 1 << 1;
    const SOUTH: u8 = 1 << 2;
    const WEST: u8 = 1 << 3;
    const SUPPORTED: [u8; 8] = [
        NORTH,
        NORTH | EAST,
        EAST,
        EAST | SOUTH,
        SOUTH,
        SOUTH | WEST,
        WEST,
        WEST | NORTH,
    ];

    #[derive(Debug)]
    struct DepthTopologyCandidate {
        x: i32,
        y: i32,
        tile: TileKind,
        edge_count: u32,
        shallow_contacts: Vec<(i32, i32)>,
    }

    // Normalize against one immutable snapshot. A single unsupported contact
    // can be observed from more than one neighboring deep cell: for example,
    // opposite north/south shallows produce one direct opposite-edge request
    // and two equivalent double-diagonal requests on the cells beside it.
    // Those observations describe the same authored-topology conflict and must
    // collapse to one semantic repair rather than eroding three deep cells.
    let snapshot = map.tiles.clone();
    let mut candidates = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let Some(index) = TavernMap::idx(x, y) else {
                continue;
            };
            let tile = snapshot[index];
            if !matches!(tile, TileKind::DeepWater | TileKind::OceanDeep) {
                continue;
            }
            let domain = depth_water_domain(tile);
            let is_same_domain_shallow = |sample_x: i32, sample_y: i32| {
                TavernMap::idx(sample_x, sample_y).is_some_and(|neighbor| {
                    let neighbor_tile = snapshot[neighbor];
                    depth_water_domain(neighbor_tile) == domain
                        && matches!(
                            neighbor_tile,
                            TileKind::Water | TileKind::ShallowWater | TileKind::OceanShallow
                        )
                })
            };
            let mut edge_mask = 0u8;
            let mut shallow_contacts = Vec::new();
            for (ox, oy, bit) in [(0, -1, NORTH), (1, 0, EAST), (0, 1, SOUTH), (-1, 0, WEST)] {
                if is_same_domain_shallow(x + ox, y + oy) {
                    edge_mask |= bit;
                    shallow_contacts.push((x + ox, y + oy));
                }
            }

            let mut inner_corner_count = 0u32;
            for (ox, oy, adjacent_edges) in [
                (1, -1, NORTH | EAST),
                (1, 1, SOUTH | EAST),
                (-1, 1, SOUTH | WEST),
                (-1, -1, NORTH | WEST),
            ] {
                if is_same_domain_shallow(x + ox, y + oy) && edge_mask & adjacent_edges == 0 {
                    inner_corner_count += 1;
                    shallow_contacts.push((x + ox, y + oy));
                }
            }

            let has_transition = edge_mask != 0 || inner_corner_count != 0;
            let representable = if edge_mask == 0 {
                inner_corner_count <= 1
            } else {
                SUPPORTED.contains(&edge_mask) && inner_corner_count == 0
            };
            if has_transition && !representable {
                shallow_contacts
                    .sort_unstable_by_key(|&(contact_x, contact_y)| (contact_y, contact_x));
                shallow_contacts.dedup();
                candidates.push(DepthTopologyCandidate {
                    x,
                    y,
                    tile,
                    edge_count: edge_mask.count_ones(),
                    shallow_contacts,
                });
            }
        }
    }

    // Keep one canonical repair for each exact shallow-contact set. Prefer the
    // cell with the strongest direct cardinal evidence, then use stable map
    // order as the tie-break. This retains real independent anomalies while
    // eliminating duplicate observations of the same two-cell conflict.
    let mut replacements: Vec<DepthTopologyCandidate> = Vec::new();
    for candidate in candidates {
        if let Some(existing) = replacements
            .iter_mut()
            .find(|existing| existing.shallow_contacts == candidate.shallow_contacts)
        {
            let candidate_priority = (candidate.edge_count, Reverse((candidate.y, candidate.x)));
            let existing_priority = (existing.edge_count, Reverse((existing.y, existing.x)));
            if candidate_priority > existing_priority {
                *existing = candidate;
            }
        } else {
            replacements.push(candidate);
        }
    }

    let changed = replacements.len();
    for candidate in replacements {
        map.set(candidate.x, candidate.y, shallow_variant(candidate.tile));
    }
    changed
}

pub(super) fn shallow_variant(tile: TileKind) -> TileKind {
    match tile {
        TileKind::OceanDeep | TileKind::OceanShallow => TileKind::OceanShallow,
        TileKind::RiverWater | TileKind::RiverMouthBlend => TileKind::RiverWater,
        _ => TileKind::ShallowWater,
    }
}

pub(super) fn deep_variant(tile: TileKind) -> TileKind {
    match tile {
        TileKind::OceanDeep | TileKind::OceanShallow => TileKind::OceanDeep,
        TileKind::RiverWater | TileKind::RiverMouthBlend => TileKind::RiverWater,
        _ => TileKind::DeepWater,
    }
}

pub(super) fn is_depth_water(tile: TileKind) -> bool {
    depth_water_domain(tile).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DepthWaterDomain {
    Marine,
    Inland,
    River,
}

fn depth_water_domain(tile: TileKind) -> Option<DepthWaterDomain> {
    match tile {
        TileKind::OceanDeep | TileKind::OceanShallow => Some(DepthWaterDomain::Marine),
        TileKind::RiverWater | TileKind::RiverMouthBlend => Some(DepthWaterDomain::River),
        TileKind::Water | TileKind::ShallowWater | TileKind::DeepWater => {
            Some(DepthWaterDomain::Inland)
        }
        _ => None,
    }
}

fn shallow_only_depth_components(
    tiles: &[TileKind],
    distance: &[usize],
    shallow_band_cost: usize,
) -> Vec<bool> {
    // A deep-water core smaller than 3x3 reads as a square hole rather than a
    // coherent depth region. Keep those small bodies entirely shallow while
    // allowing large lakes and oceans to retain their two-cell shallow rim.
    const MIN_DEEP_CORE_CELLS: usize = 9;

    let mut visited = vec![false; tiles.len()];
    let mut force_shallow = vec![false; tiles.len()];

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let Some(start) = TavernMap::idx(x, y) else {
                continue;
            };
            if visited[start] {
                continue;
            }
            let Some(domain) = depth_water_domain(tiles[start]) else {
                continue;
            };
            if domain == DepthWaterDomain::River {
                visited[start] = true;
                continue;
            }

            let mut queue = VecDeque::new();
            queue.push_back((x, y));
            let mut component = Vec::new();
            let mut deep_core_cells = 0usize;
            visited[start] = true;

            while let Some((cell_x, cell_y)) = queue.pop_front() {
                let Some(index) = TavernMap::idx(cell_x, cell_y) else {
                    continue;
                };
                component.push(index);
                if distance[index] > shallow_band_cost {
                    deep_core_cells += 1;
                }

                for (offset_x, offset_y) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let neighbor_x = cell_x + offset_x;
                    let neighbor_y = cell_y + offset_y;
                    let Some(neighbor) = TavernMap::idx(neighbor_x, neighbor_y) else {
                        continue;
                    };
                    if visited[neighbor] || depth_water_domain(tiles[neighbor]) != Some(domain) {
                        continue;
                    }
                    visited[neighbor] = true;
                    queue.push_back((neighbor_x, neighbor_y));
                }
            }

            if deep_core_cells < MIN_DEEP_CORE_CELLS {
                for index in component {
                    force_shallow[index] = true;
                }
            }
        }
    }

    force_shallow
}
