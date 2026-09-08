use haven_core::{TavernMap, TileKind};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{
    asset_registry::AtlasRect, terrain_material_bindings::canonical_corner_tuple_material,
};

pub const LPC_MAPPED_TERRAIN_ATLAS_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/lpc_mapped_terrain_v7_32.png";
pub const LPC_MAPPED_TERRAIN_MANIFEST_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/lpc_mapped_terrain_v7_32.json";

static LPC_MAPPED_TERRAIN: OnceLock<Result<LpcMappedTerrainManifest, String>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LpcMappedTerrainEntry {
    pub rect: AtlasRect,
    pub is_mixed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalCornerTuple {
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
}

impl CanonicalCornerTuple {
    pub const fn new(corners: [&'static str; 4]) -> Self {
        Self {
            top_left: corners[0],
            top_right: corners[1],
            bottom_left: corners[2],
            bottom_right: corners[3],
        }
    }

    pub const fn corners(self) -> [&'static str; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        ]
    }

    pub fn is_mixed(self) -> bool {
        let corners = self.corners();
        corners.iter().any(|corner| corner != &corners[0])
    }

    pub fn signature(self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.top_left, self.top_right, self.bottom_left, self.bottom_right
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LpcTupleResolutionStatus {
    Exact,
    AuthoredBridge,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpcMappedTerrainDiagnostic {
    pub corners: [String; 4],
    pub signature: String,
    pub rect: Option<AtlasRect>,
    pub is_mixed: bool,
    pub status: LpcTupleResolutionStatus,
    // Compatibility field retained for existing overlays and tooling.
    pub exact_match: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpcMappedTerrainManifest {
    output: String,
    entries: Vec<LpcMappedTerrainManifestEntry>,
    tuple_index: HashMap<u64, Vec<usize>>,
}

impl LpcMappedTerrainManifest {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(LPC_MAPPED_TERRAIN_MANIFEST_PATH);
        let file = load_json::<LpcMappedTerrainManifestFile>(&manifest_path)?;
        let output = normalize_repo_relative_path(&file.output);
        if output != LPC_MAPPED_TERRAIN_ATLAS_PATH {
            return Err(format!(
                "mapped terrain atlas output mismatch: expected {}, found {}",
                LPC_MAPPED_TERRAIN_ATLAS_PATH, output
            ));
        }
        let entries: Vec<LpcMappedTerrainManifestEntry> = file
            .entries
            .into_iter()
            .map(|entry| LpcMappedTerrainManifestEntry {
                corners: [
                    entry.corners.top_left,
                    entry.corners.top_right,
                    entry.corners.bottom_left,
                    entry.corners.bottom_right,
                ],
                rect: atlas_rect_from_u32(entry.rect),
            })
            .collect();
        let mut tuple_index: HashMap<u64, Vec<usize>> = HashMap::new();
        for (index, entry) in entries.iter().enumerate() {
            tuple_index
                .entry(corner_tuple_key([
                    entry.corners[0].as_str(),
                    entry.corners[1].as_str(),
                    entry.corners[2].as_str(),
                    entry.corners[3].as_str(),
                ]))
                .or_default()
                .push(index);
        }
        Ok(Self {
            output,
            entries,
            tuple_index,
        })
    }

    pub fn atlas_path(&self) -> &str {
        &self.output
    }

    fn entry_for_corners(
        &self,
        corners: [&str; 4],
        variant_seed: u32,
    ) -> Option<LpcMappedTerrainEntry> {
        let candidates = self.tuple_index.get(&corner_tuple_key(corners))?;
        let matches: Vec<&LpcMappedTerrainManifestEntry> = candidates
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .filter(|entry| entry_matches(entry, corners))
            .collect();
        let variant_count = matches.len();
        if variant_count == 0 {
            return None;
        }
        let is_mixed = corners.iter().any(|corner| corner != &corners[0]);
        let variant_index = if is_mixed || variant_count == 1 {
            0
        } else {
            deterministic_fill_variant(corners[0], variant_seed, variant_count)
        };
        matches
            .get(variant_index)
            .map(|entry| LpcMappedTerrainEntry {
                rect: entry.rect,
                is_mixed,
            })
    }

    fn entry_for_corners_with_water_frame(
        &self,
        corners: [&str; 4],
        variant_seed: u32,
        water_frame: u32,
    ) -> Option<LpcMappedTerrainEntry> {
        let candidates = self.tuple_index.get(&corner_tuple_key(corners))?;
        let matches: Vec<&LpcMappedTerrainManifestEntry> = candidates
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .filter(|entry| entry_matches(entry, corners))
            .collect();
        let variant_count = matches.len();
        if variant_count == 0 {
            return None;
        }
        let is_mixed = corners.iter().any(|corner| corner != &corners[0]);
        let variant_index = if is_mixed || variant_count == 1 {
            0
        } else if is_animated_water_material(corners[0]) {
            animated_water_variant(variant_seed, water_frame, variant_count)
        } else {
            deterministic_fill_variant(corners[0], variant_seed, variant_count)
        };
        matches
            .get(variant_index)
            .map(|entry| LpcMappedTerrainEntry {
                rect: entry.rect,
                is_mixed,
            })
    }

    fn quiet_entry_for_corners(&self, corners: [&str; 4]) -> Option<LpcMappedTerrainEntry> {
        let candidates = self.tuple_index.get(&corner_tuple_key(corners))?;
        let entry = candidates
            .iter()
            .filter_map(|index| self.entries.get(*index))
            .find(|entry| entry_matches(entry, corners))?;
        Some(LpcMappedTerrainEntry {
            rect: entry.rect,
            is_mixed: corners.iter().any(|corner| corner != &corners[0]),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LpcMappedTerrainManifestEntry {
    corners: [String; 4],
    rect: AtlasRect,
}

fn corner_tuple_key(corners: [&str; 4]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for corner in corners {
        for byte in corner.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn entry_matches(entry: &LpcMappedTerrainManifestEntry, corners: [&str; 4]) -> bool {
    entry.corners[0] == corners[0]
        && entry.corners[1] == corners[1]
        && entry.corners[2] == corners[2]
        && entry.corners[3] == corners[3]
}

#[derive(Deserialize)]
struct LpcMappedTerrainManifestFile {
    output: String,
    entries: Vec<LpcMappedTerrainEntryFile>,
}

#[derive(Deserialize)]
struct LpcMappedTerrainEntryFile {
    corners: LpcMappedTerrainCornersFile,
    rect: [u32; 4],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LpcMappedTerrainCornersFile {
    top_left: String,
    top_right: String,
    bottom_left: String,
    bottom_right: String,
}

pub fn lpc_mapped_terrain_atlas_path() -> Result<&'static str, &'static str> {
    lpc_mapped_terrain_manifest().map(LpcMappedTerrainManifest::atlas_path)
}

pub fn lpc_mapped_terrain_manifest() -> Result<&'static LpcMappedTerrainManifest, &'static str> {
    LPC_MAPPED_TERRAIN
        .get_or_init(LpcMappedTerrainManifest::load_default)
        .as_ref()
        .map_err(|_| "mapped LPC terrain manifest unavailable")
}

pub fn lpc_mapped_terrain_diagnostic_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainDiagnostic> {
    let tuple = canonical_corner_tuple_for_map_tile(map, x, y)?;
    let corners = tuple.corners();
    let manifest = lpc_mapped_terrain_manifest().ok()?;
    let seed = cell_variant_seed(x, y);
    let exact = manifest.entry_for_corners(corners, seed);
    let bridged = if exact.is_none() {
        compatible_edge_bridge_entry_for_corners(manifest, corners, seed)
    } else {
        None
    };
    let status = if exact.is_some() {
        LpcTupleResolutionStatus::Exact
    } else if bridged.is_some() {
        LpcTupleResolutionStatus::AuthoredBridge
    } else {
        LpcTupleResolutionStatus::Unsupported
    };
    Some(LpcMappedTerrainDiagnostic {
        corners: corners.map(str::to_string),
        signature: tuple.signature(),
        rect: exact.or(bridged).map(|resolved| resolved.rect),
        is_mixed: tuple.is_mixed(),
        status,
        exact_match: status == LpcTupleResolutionStatus::Exact,
    })
}

pub fn lpc_mapped_terrain_exact_entry_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry> {
    // The terrain-map-v7 atlas already contains authored fills, edges, corners,
    // islands, and multi-material junctions. Ask for the exact four-corner tuple;
    // only tuples absent from the sheet return None.
    let corners = mapped_terrain_corners_for_map_tile(map, x, y)?;
    lpc_mapped_terrain_manifest()
        .ok()?
        .entry_for_corners(corners, cell_variant_seed(x, y))
}

pub fn lpc_mapped_terrain_entry_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry> {
    // Tiled terrain tiles are corner tuples, not center-cell edge masks. Sample
    // the four semantic cells that meet at this rendered tile.
    let corners = mapped_terrain_corners_for_map_tile(map, x, y)?;

    let manifest = lpc_mapped_terrain_manifest().ok()?;
    if let Some(exact) = manifest.entry_for_corners(corners, cell_variant_seed(x, y)) {
        return Some(exact);
    }

    // Unsupported tuple signatures must never fall through to an
    // unrelated debug-colored tile. Preserve the owning semantic floor as a
    // pure fill while diagnostics continue to report the tuple as unsupported.
    // Pair-specific transition overlays can still draw reviewed edge art on top.
    let owner = mapped_terrain_at(map, x, y)?;
    manifest.entry_for_corners([owner; 4], cell_variant_seed(x, y))
}

pub fn lpc_mapped_terrain_owner_fill_entry_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry> {
    lpc_mapped_terrain_owner_fill_entry_for_map_with_water_frame(map, x, y, 0)
}

pub fn lpc_mapped_terrain_owner_fill_entry_for_map_with_water_frame(
    map: &TavernMap,
    x: i32,
    y: i32,
    water_frame: u32,
) -> Option<LpcMappedTerrainEntry> {
    let owner = mapped_terrain_at(map, x, y)?;
    let manifest = lpc_mapped_terrain_manifest().ok()?;
    if touches_different_mapped_material(map, x, y, owner) {
        // Transition-adjacent owner cells must remain visually quiet. Animated
        // shimmer/detail variants are complete authored cells, but selecting
        // them independently under an edge tuple creates visible square blocks
        // along long coastlines. The mixed tuple already carries the authored
        // edge pixels; interior water continues to animate away from contacts.
        return manifest.quiet_entry_for_corners([owner; 4]);
    }
    manifest.entry_for_corners_with_water_frame([owner; 4], cell_variant_seed(x, y), water_frame)
}

/// Source-authored mixed tuple used as a presentation overlay.
///
/// Exact V7 tuples always win. Gravel, Rock Ground and Mud may use a reviewed
/// V7-only connector tuple when the source sheet has no direct two-material
/// pair; semantic owner fills are never changed.
///
/// The current V7 certification lane keeps every water tier inside the same
/// source family. V7 supplies exact shallow-to-medium and medium-to-deep tuples;
/// the presentation resolver derives the medium rim when raw shallow and deep
/// semantics touch. No ElizaWy or generated overlay is required.
pub fn lpc_mapped_terrain_transition_entry_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry> {
    let corners = mapped_terrain_corners_for_map_tile(map, x, y)?;
    let manifest = lpc_mapped_terrain_manifest().ok()?;
    let seed = cell_variant_seed(x, y);
    if let Some(entry) = manifest.entry_for_corners(corners, seed) {
        return entry.is_mixed.then_some(entry);
    }
    compatible_edge_bridge_entry_for_corners(manifest, corners, seed)
}

pub fn lpc_mapped_terrain_runtime_entry_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<LpcMappedTerrainEntry> {
    // Runtime base cells are always pure owner fills. Mixed authored corner
    // tuples render later in their own intersection-aligned overlay pass. This
    // keeps the gameplay cell, cursor highlight, and base material centered
    // while preserving exact authored transition pixels.
    lpc_mapped_terrain_owner_fill_entry_for_map(map, x, y)
}

mod sampling;

use sampling::compatible_edge_bridge_entry_for_corners;
pub use sampling::{
    lpc_mapped_terrain_runtime_entry_for_tile_sampler,
    lpc_mapped_terrain_transition_entry_for_tile_sampler,
};

pub fn lpc_mapped_terrain_entry_for_map_with_water_frame(
    map: &TavernMap,
    x: i32,
    y: i32,
    water_frame: u32,
) -> Option<LpcMappedTerrainEntry> {
    let corners = mapped_terrain_corners_for_map_tile(map, x, y)?;
    lpc_mapped_terrain_manifest()
        .ok()?
        .entry_for_corners_with_water_frame(corners, cell_variant_seed(x, y), water_frame)
}

pub fn lpc_mapped_terrain_quiet_entry(tile: TileKind) -> Option<LpcMappedTerrainEntry> {
    let material = mapped_terrain_name(tile)?;
    lpc_mapped_terrain_manifest()
        .ok()?
        .quiet_entry_for_corners([material; 4])
}

pub fn lpc_mapped_terrain_preview_entry(tile: TileKind) -> Option<LpcMappedTerrainEntry> {
    // Palette cards preview the exact quiet base that a one-cell brush places.
    // Larger authored marks are separate atomic stamps and must never appear
    // as clipped or partial fill variants.
    lpc_mapped_terrain_quiet_entry(tile)
}

pub fn lpc_mapped_terrain_transition_covers_map_cell(map: &TavernMap, x: i32, y: i32) -> bool {
    if mapped_terrain_name(map.get(x, y)).is_none() {
        return false;
    }
    for sample_y in (y - 1)..=y {
        for sample_x in (x - 1)..=x {
            if lpc_mapped_terrain_transition_entry_for_map(map, sample_x, sample_y).is_some() {
                return true;
            }
        }
    }
    false
}

/// Returns true when terrain-v7 owns any mapped render tile that contributes
/// to this map cell. Pure fills and mixed shoreline/depth tuples both count.
pub fn lpc_mapped_terrain_owns_map_cell(map: &TavernMap, x: i32, y: i32) -> bool {
    for sample_y in (y - 1)..=y {
        for sample_x in (x - 1)..=x {
            if lpc_mapped_terrain_entry_for_map(map, sample_x, sample_y).is_some() {
                return true;
            }
        }
    }
    false
}

pub fn canonical_corner_tuple_for_map_tile(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<CanonicalCornerTuple> {
    mapped_terrain_corners_for_map_tile(map, x, y).map(CanonicalCornerTuple::new)
}

fn mapped_terrain_corners_for_map_tile(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<[&'static str; 4]> {
    Some([
        mapped_terrain_at(map, x, y)?,
        mapped_terrain_at(map, x + 1, y)?,
        mapped_terrain_at(map, x, y + 1)?,
        mapped_terrain_at(map, x + 1, y + 1)?,
    ])
}

fn mapped_terrain_name(tile: TileKind) -> Option<&'static str> {
    canonical_corner_tuple_material(tile)
}

/// Resolve the V7 presentation material for one semantic gameplay cell.
///
/// terrain-map-v7 contains authored transitions from shallow water to its
/// medium `Water` material and from `Water` to `Water_Deep`, but it does not
/// contain a direct `Water_Shallows_*` to `Water_Deep` tuple. Keep the raw
/// freshwater/ocean identity in the map and derive a one-cell medium-water
/// presentation rim on the deep side of a shallow/deep contact. This uses only
/// complete V7 source cells and avoids generated or cross-style transition art.
fn mapped_terrain_at(map: &TavernMap, x: i32, y: i32) -> Option<&'static str> {
    TavernMap::idx(x, y)?;
    let tile = map.get(x, y);
    if matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)
        && (touches_same_domain_shallow_water(map, x, y, tile)
            || touches_authored_medium_water_land_contact(map, x, y))
    {
        return Some("Water");
    }
    mapped_terrain_name(tile)
}

fn touches_same_domain_shallow_water(map: &TavernMap, x: i32, y: i32, deep_tile: TileKind) -> bool {
    for (offset_x, offset_y) in [
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ] {
        if TavernMap::idx(x + offset_x, y + offset_y).is_none() {
            continue;
        }
        let neighbor = map.get(x + offset_x, y + offset_y);
        let same_domain_shallow = match deep_tile {
            TileKind::OceanDeep => neighbor == TileKind::OceanShallow,
            TileKind::DeepWater => matches!(neighbor, TileKind::ShallowWater | TileKind::Water),
            _ => false,
        };
        if same_domain_shallow {
            return true;
        }
    }
    false
}

/// V7 contains complete authored `Grass|Water`, `Sand|Water`,
/// `Dirt_Brown|Water`, and `Dirt_Tan|Water` tuples. When Exact mode places one
/// of those land materials directly in deep water, preserve the gameplay
/// semantics and derive only a one-cell medium-water presentation rim around
/// the contact. This gives the placed land a real authored edge without
/// converting it to sand or expanding semantic shallow water.
fn touches_authored_medium_water_land_contact(map: &TavernMap, x: i32, y: i32) -> bool {
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            if offset_x == 0 && offset_y == 0 {
                continue;
            }
            if TavernMap::idx(x + offset_x, y + offset_y).is_none() {
                continue;
            }
            let Some(neighbor) = mapped_terrain_name(map.get(x + offset_x, y + offset_y)) else {
                continue;
            };
            if matches!(neighbor, "Grass" | "Sand" | "Dirt_Brown" | "Dirt_Tan") {
                return true;
            }
        }
    }
    false
}

fn touches_different_mapped_material(map: &TavernMap, x: i32, y: i32, owner: &'static str) -> bool {
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            if offset_x == 0 && offset_y == 0 {
                continue;
            }
            let Some(neighbor) = mapped_terrain_at(map, x + offset_x, y + offset_y) else {
                continue;
            };
            if neighbor != owner {
                return true;
            }
        }
    }
    false
}

fn cell_variant_seed(x: i32, y: i32) -> u32 {
    let mut value = (x as u32).wrapping_mul(0x9e37_79b9);
    value ^= (y as u32).wrapping_mul(0x85eb_ca6b).rotate_left(13);
    value ^ (value >> 16)
}

fn is_animated_water_material(material: &str) -> bool {
    matches!(
        material,
        "Water" | "Water_Deep" | "Water_Shallows" | "Water_Shallows_Dirt" | "Water_Shallows_Sand"
    )
}

fn animated_water_variant(seed: u32, water_frame: u32, variant_count: usize) -> usize {
    if variant_count <= 1 {
        return 0;
    }
    ((seed as usize).wrapping_add(water_frame as usize)) % variant_count
}

fn deterministic_fill_variant(material: &str, seed: u32, variant_count: usize) -> usize {
    if variant_count <= 1 {
        return 0;
    }
    if material != "Grass" {
        // Non-grass alternates in terrain-map-v7 can be pieces of larger
        // authored details. They are not safe as independent repeating floor
        // variants. Use the certified quiet fill and promote reviewed detail
        // assemblies into atomic objects/stamps instead.
        return 0;
    }

    let mixed = seed
        .wrapping_mul(0x9e37_79b9)
        .rotate_left(13)
        .wrapping_add(0x85eb_ca6b);
    // Grass is the only audited single-cell detail lane. Keep the quiet fill
    // on thirty-one cells out of thirty-two to avoid checkerboard fields.
    if mixed & 31 != 0 {
        return 0;
    }
    1 + ((mixed as usize >> 5) % (variant_count - 1))
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let payload = read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&payload).map_err(|error| format!("{}: {error}", path.display()))
}

fn normalize_repo_relative_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn atlas_rect_from_u32(rect: [u32; 4]) -> AtlasRect {
    AtlasRect {
        x: rect[0] as f32,
        y: rect[1] as f32,
        w: rect[2] as f32,
        h: rect[3] as f32,
    }
}

#[cfg(test)]
#[path = "lpc_mapped_terrain/tests.rs"]
mod tests;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LpcTupleCompatibilityReport {
    pub inspected: usize,
    pub exact: usize,
    pub authored_bridges: usize,
    pub unsupported: usize,
    pub unsupported_cells: Vec<(i32, i32)>,
    pub unsupported_signatures: BTreeMap<String, usize>,
}

impl LpcTupleCompatibilityReport {
    pub fn is_compatible(&self) -> bool {
        self.unsupported == 0
    }
}

/// Audits every renderable corner tuple in a map. Generation and editor tools
/// should use this before accepting PCG terrain so unsupported topology is
/// reported rather than silently replaced by an arbitrary fill.
pub fn audit_lpc_tuple_compatibility(map: &TavernMap) -> LpcTupleCompatibilityReport {
    let mut report = LpcTupleCompatibilityReport::default();
    for y in 0..(haven_core::MAP_H as i32 - 1) {
        for x in 0..(haven_core::MAP_W as i32 - 1) {
            if mapped_terrain_corners_for_map_tile(map, x, y).is_none() {
                continue;
            }
            report.inspected += 1;
            let diagnostic = lpc_mapped_terrain_diagnostic_for_map(map, x, y);
            match diagnostic.as_ref().map(|entry| entry.status) {
                Some(LpcTupleResolutionStatus::Exact) => report.exact += 1,
                Some(LpcTupleResolutionStatus::AuthoredBridge) => report.authored_bridges += 1,
                Some(LpcTupleResolutionStatus::Unsupported) | None => {
                    report.unsupported += 1;
                    report.unsupported_cells.push((x, y));
                    if let Some(tuple) = canonical_corner_tuple_for_map_tile(map, x, y) {
                        *report
                            .unsupported_signatures
                            .entry(tuple.signature())
                            .or_default() += 1;
                    }
                }
            }
        }
    }
    report
}
