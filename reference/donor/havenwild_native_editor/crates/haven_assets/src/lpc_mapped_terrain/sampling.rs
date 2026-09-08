use haven_core::TileKind;

use super::{
    cell_variant_seed, lpc_mapped_terrain_manifest, mapped_terrain_name, LpcMappedTerrainEntry,
    LpcMappedTerrainManifest,
};

/// Resolves a pure owner fill from an arbitrary global terrain sampler.
///
/// Exterior partitions are storage chunks of one surface. The sampler may
/// therefore read across partition edges, unlike the legacy TavernMap-only
/// resolver whose out-of-bounds cells intentionally terminate at one map.
pub fn lpc_mapped_terrain_runtime_entry_for_tile_sampler<F>(
    tile_at: &F,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry>
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    let owner = sampled_mapped_terrain_at(tile_at, x, y)?;
    let manifest = lpc_mapped_terrain_manifest().ok()?;
    if sampled_touches_different_mapped_material(tile_at, x, y, owner) {
        return manifest.quiet_entry_for_corners([owner; 4]);
    }
    manifest.entry_for_corners([owner; 4], cell_variant_seed(x, y))
}

/// Resolves one exact or reviewed connector-based mixed V7 tuple from an
/// arbitrary global terrain sampler. This is the cross-partition counterpart to
/// `lpc_mapped_terrain_transition_entry_for_map`.
pub fn lpc_mapped_terrain_transition_entry_for_tile_sampler<F>(
    tile_at: &F,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry>
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    let corners = sampled_mapped_terrain_corners(tile_at, x, y)?;
    let manifest = lpc_mapped_terrain_manifest().ok()?;
    let seed = cell_variant_seed(x, y);
    if let Some(entry) = manifest.entry_for_corners(corners, seed) {
        return entry.is_mixed.then_some(entry);
    }
    compatible_edge_bridge_entry_for_corners(manifest, corners, seed)
}

pub(super) fn compatible_edge_bridge_entry_for_corners(
    manifest: &LpcMappedTerrainManifest,
    corners: [&'static str; 4],
    variant_seed: u32,
) -> Option<LpcMappedTerrainEntry> {
    for candidate in compatible_edge_bridge_corner_candidates(corners)
        .into_iter()
        .flatten()
    {
        if candidate == corners {
            continue;
        }
        let Some(entry) = manifest.entry_for_corners(candidate, variant_seed) else {
            continue;
        };
        if entry.is_mixed {
            return Some(entry);
        }
    }
    None
}

fn compatible_edge_bridge_corner_candidates(
    corners: [&'static str; 4],
) -> [Option<[&'static str; 4]>; 2] {
    let mut materials = [None, None];
    let mut material_count = 0usize;
    for material in corners {
        if materials[..material_count].contains(&Some(material)) {
            continue;
        }
        if material_count == materials.len() {
            return [None, None];
        }
        materials[material_count] = Some(material);
        material_count += 1;
    }
    if material_count != 2 {
        return [None, None];
    }

    let first = materials[0].expect("first bridge material");
    let second = materials[1].expect("second bridge material");
    [
        compatible_edge_proxy(first, second)
            .map(|proxy| replace_corner_material(corners, first, proxy)),
        compatible_edge_proxy(second, first)
            .map(|proxy| replace_corner_material(corners, second, proxy)),
    ]
}

fn compatible_edge_proxy(material: &'static str, neighbor: &'static str) -> Option<&'static str> {
    match material {
        // Exact F3 painting may intentionally place Grass directly against
        // water. V7 does not expose a direct Grass<->Water tuple family, so use
        // Sand only for the mixed overlay while the semantic Grass owner fill
        // remains untouched. This prevents a raw square green strip without
        // mutating neighboring terrain in Exact mode.
        "Grass" => match neighbor {
            "Water" | "Water_Deep" | "Water_Shallows_Dirt" | "Water_Shallows_Sand" => {
                Some("Sand")
            }
            _ => None,
        },
        // Gravel_1 has authored V7 interiors and transitions to dirt, mud and
        // rock. Where the source sheet lacks a direct grass/sand/water tuple,
        // use a reviewed V7 connector only for the mixed edge overlay. The
        // semantic Gravel owner fill remains untouched.
        "Gravel_1" => match neighbor {
            "Grass" | "Sand" => Some("Stone_Tan"),
            "Soil" => Some("Dirt_Tan"),
            "Water" | "Water_Deep" | "Water_Shallows_Dirt" => Some("Dirt_Brown"),
            "Water_Shallows_Sand" => Some("Sand"),
            "Mudstone_Brown" => Some("Dirt_Roots"),
            "Stone_Tan" => Some("Dirt_Tan"),
            _ => None,
        },
        // Rock_Dark keeps its authored rock interior. Dirt_Roots is the V7
        // highland connector for grass/sand/path contacts; broad dirt/sand
        // connectors are used only where water or farm soil lacks rock tuples.
        "Rock_Dark" => match neighbor {
            "Grass" | "Stone_Tan" => Some("Dirt_Roots"),
            "Sand" => Some("Dirt_Brown"),
            "Soil" => Some("Dirt_Tan"),
            "Water" | "Water_Deep" | "Water_Shallows_Dirt" => Some("Dirt_Brown"),
            "Water_Shallows_Sand" => Some("Sand"),
            "Dirt_Brown" | "Dirt_Tan" => Some("Dirt_Roots"),
            _ => None,
        },
        // Mud_Brown has complete authored edges against the common land
        // materials. Sand, farm soil and water use a compatible dirt/sand
        // connector only when the direct V7 tuple is absent.
        "Mud_Brown" => match neighbor {
            "Sand" => Some("Dirt_Brown"),
            "Soil" => Some("Dirt_Tan"),
            "Water" | "Water_Deep" | "Water_Shallows_Dirt" => Some("Dirt_Brown"),
            "Water_Shallows_Sand" => Some("Sand"),
            _ => None,
        },
        _ => None,
    }
}

fn replace_corner_material(
    corners: [&'static str; 4],
    from: &'static str,
    to: &'static str,
) -> [&'static str; 4] {
    corners.map(|corner| if corner == from { to } else { corner })
}

fn sampled_mapped_terrain_corners<F>(tile_at: &F, x: i32, y: i32) -> Option<[&'static str; 4]>
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    Some([
        sampled_mapped_terrain_at(tile_at, x, y)?,
        sampled_mapped_terrain_at(tile_at, x + 1, y)?,
        sampled_mapped_terrain_at(tile_at, x, y + 1)?,
        sampled_mapped_terrain_at(tile_at, x + 1, y + 1)?,
    ])
}

fn sampled_mapped_terrain_at<F>(tile_at: &F, x: i32, y: i32) -> Option<&'static str>
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    let tile = tile_at(x, y)?;
    if matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)
        && (sampled_touches_same_domain_shallow_water(tile_at, x, y, tile)
            || sampled_touches_authored_medium_water_land_contact(tile_at, x, y))
    {
        return Some("Water");
    }
    mapped_terrain_name(tile)
}

fn sampled_touches_same_domain_shallow_water<F>(
    tile_at: &F,
    x: i32,
    y: i32,
    deep_tile: TileKind,
) -> bool
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    [
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ]
    .into_iter()
    .filter_map(|(offset_x, offset_y)| tile_at(x + offset_x, y + offset_y))
    .any(|neighbor| match deep_tile {
        TileKind::OceanDeep => neighbor == TileKind::OceanShallow,
        TileKind::DeepWater => matches!(neighbor, TileKind::ShallowWater | TileKind::Water),
        _ => false,
    })
}

fn sampled_touches_authored_medium_water_land_contact<F>(tile_at: &F, x: i32, y: i32) -> bool
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            if offset_x == 0 && offset_y == 0 {
                continue;
            }
            let Some(neighbor) = tile_at(x + offset_x, y + offset_y).and_then(mapped_terrain_name)
            else {
                continue;
            };
            if matches!(neighbor, "Grass" | "Sand" | "Dirt_Brown" | "Dirt_Tan") {
                return true;
            }
        }
    }
    false
}
fn sampled_touches_different_mapped_material<F>(
    tile_at: &F,
    x: i32,
    y: i32,
    owner: &'static str,
) -> bool
where
    F: Fn(i32, i32) -> Option<TileKind>,
{
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            if offset_x == 0 && offset_y == 0 {
                continue;
            }
            let Some(neighbor) = sampled_mapped_terrain_at(tile_at, x + offset_x, y + offset_y)
            else {
                continue;
            };
            if neighbor != owner {
                return true;
            }
        }
    }
    false
}

