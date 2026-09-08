use haven_core::{TavernMap, TileKind};
use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const N: u8 = 1 << 0;
pub const NE: u8 = 1 << 1;
pub const E: u8 = 1 << 2;
pub const SE: u8 = 1 << 3;
pub const S: u8 = 1 << 4;
pub const SW: u8 = 1 << 5;
pub const W: u8 = 1 << 6;
pub const NW: u8 = 1 << 7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutoTileNeighbors {
    pub north: bool,
    pub east: bool,
    pub south: bool,
    pub west: bool,
    pub north_east: bool,
    pub south_east: bool,
    pub south_west: bool,
    pub north_west: bool,
}

impl AutoTileNeighbors {
    pub fn cardinal_mask(self) -> u8 {
        let mut mask = 0;
        if self.north {
            mask |= N;
        }
        if self.east {
            mask |= E;
        }
        if self.south {
            mask |= S;
        }
        if self.west {
            mask |= W;
        }
        mask
    }

    pub fn mask_8way(self) -> u8 {
        let mut mask = self.cardinal_mask();
        if self.north_east {
            mask |= NE;
        }
        if self.south_east {
            mask |= SE;
        }
        if self.south_west {
            mask |= SW;
        }
        if self.north_west {
            mask |= NW;
        }
        mask
    }
}

pub fn same_autotile_group(map: &TavernMap, tile: TileKind, x: i32, y: i32) -> bool {
    let Some(group) = tile.autotile_group() else {
        return false;
    };
    TavernMap::idx(x, y)
        .map(|_| map.get(x, y).autotile_group() == Some(group))
        .unwrap_or(false)
}

pub fn autotile_neighbors(map: &TavernMap, tile: TileKind, x: i32, y: i32) -> AutoTileNeighbors {
    let north = same_autotile_group(map, tile, x, y - 1);
    let east = same_autotile_group(map, tile, x + 1, y);
    let south = same_autotile_group(map, tile, x, y + 1);
    let west = same_autotile_group(map, tile, x - 1, y);

    AutoTileNeighbors {
        north,
        east,
        south,
        west,
        north_east: north && east && same_autotile_group(map, tile, x + 1, y - 1),
        south_east: south && east && same_autotile_group(map, tile, x + 1, y + 1),
        south_west: south && west && same_autotile_group(map, tile, x - 1, y + 1),
        north_west: north && west && same_autotile_group(map, tile, x - 1, y - 1),
    }
}

pub fn autotile_mask_8way(map: &TavernMap, tile: TileKind, x: i32, y: i32) -> u8 {
    autotile_neighbors(map, tile, x, y).mask_8way()
}

pub fn dirty_autotile_cells(x: i32, y: i32) -> [(i32, i32); 9] {
    [
        (x, y),
        (x, y - 1),
        (x + 1, y - 1),
        (x + 1, y),
        (x + 1, y + 1),
        (x, y + 1),
        (x - 1, y + 1),
        (x - 1, y),
        (x - 1, y - 1),
    ]
}

pub fn dirty_autotile_cells_in_bounds(x: i32, y: i32) -> Vec<(i32, i32)> {
    dirty_autotile_cells(x, y)
        .into_iter()
        .filter(|(cell_x, cell_y)| TavernMap::idx(*cell_x, *cell_y).is_some())
        .collect()
}

pub fn deterministic_variant(seed: u32, x: i32, y: i32, variant_count: u8) -> u8 {
    if variant_count == 0 {
        return 0;
    }
    let mut value = seed ^ (x as u32).wrapping_mul(0x9e37_79b9);
    value = value.rotate_left(13) ^ (y as u32).wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    (value % variant_count as u32) as u8
}

pub const TERRAIN_AUTOTILE_ATLAS_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/terrain_autotile_47_32.png";
pub const TERRAIN_AUTOTILE_ATLAS_MANIFEST_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/terrain_autotile_47_32.json";

static TRANSITION_ATLAS_MANIFEST: OnceLock<Result<TransitionAtlasManifest, String>> =
    OnceLock::new();

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionAtlasEntry {
    pub sheet: String,
    pub group_id: String,
    pub mask4: u8,
    pub variant_index: u8,
    pub rect: crate::asset_registry::AtlasRect,
}

impl TransitionAtlasEntry {
    pub fn stable_debug_id(&self) -> String {
        format!("{}_mask_{:02}", self.group_id, self.mask4)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionAtlasInnerCornerEntry {
    pub sheet: String,
    pub group_id: String,
    pub direction: String,
    pub rect: crate::asset_registry::AtlasRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionAtlasManifest {
    pub id: String,
    pub kind: String,
    pub version: String,
    pub tile_size: u32,
    pub padding: u32,
    pub autotile_format: String,
    pub groups: Vec<String>,
    pub variants: Vec<TransitionAtlasManifestVariant>,
    pub inner_corner_variants: Vec<TransitionAtlasManifestInnerCornerVariant>,
    pub source: Option<String>,
    pub output: String,
    pub license: Option<String>,
    manifest_path: PathBuf,
}

impl TransitionAtlasManifest {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(TERRAIN_AUTOTILE_ATLAS_MANIFEST_PATH);
        let mut manifest = load_json::<TransitionAtlasManifestFile>(&manifest_path)?;
        let output = normalize_repo_relative_path(&manifest.output);
        if output != TERRAIN_AUTOTILE_ATLAS_PATH {
            return Err(format!(
                "transition atlas output mismatch: expected {}, found {} in {}",
                TERRAIN_AUTOTILE_ATLAS_PATH,
                output,
                manifest_path.display()
            ));
        }
        manifest.output = output;
        let loaded = Self {
            id: manifest.id,
            kind: manifest.kind,
            version: manifest.version,
            tile_size: manifest.tile_size,
            padding: manifest.padding,
            autotile_format: manifest.autotile_format,
            groups: manifest.groups,
            variants: manifest.variants,
            inner_corner_variants: manifest.inner_corner_variants,
            source: manifest.source,
            output: manifest.output,
            license: manifest.license,
            manifest_path,
        };
        loaded.validate()?;
        Ok(loaded)
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn atlas_path(&self) -> &str {
        &self.output
    }

    pub fn coverage_summary(&self) -> String {
        format!(
            "{} -> {} groups, {} outer + {} inner transition variants",
            self.id,
            self.groups.len(),
            self.variants.len(),
            self.inner_corner_variants.len()
        )
    }

    pub fn group_is_supported(&self, group_id: &str) -> bool {
        self.groups.iter().any(|group| group == group_id)
    }

    pub fn entry_for_mask(&self, group_id: &str, mask4: u8) -> Option<TransitionAtlasEntry> {
        let mask4 = mask4 & 0b1111;
        let variant = self
            .variants
            .iter()
            .find(|variant| variant.group == group_id && variant.mask4 == mask4)
            .or_else(|| self.fallback_variant(group_id, mask4))?;

        Some(TransitionAtlasEntry {
            sheet: self.output.clone(),
            group_id: variant.group.clone(),
            mask4: variant.mask4,
            variant_index: variant.variant_index,
            rect: crate::asset_registry::AtlasRect {
                x: variant.rect[0] as f32,
                y: variant.rect[1] as f32,
                w: variant.rect[2] as f32,
                h: variant.rect[3] as f32,
            },
        })
    }

    pub fn inner_corner_entry(
        &self,
        group_id: &str,
        direction: &str,
    ) -> Option<TransitionAtlasInnerCornerEntry> {
        let variant = self
            .inner_corner_variants
            .iter()
            .find(|variant| variant.group == group_id && variant.direction == direction)?;
        Some(TransitionAtlasInnerCornerEntry {
            sheet: self.output.clone(),
            group_id: variant.group.clone(),
            direction: variant.direction.clone(),
            rect: crate::asset_registry::AtlasRect {
                x: variant.rect[0] as f32,
                y: variant.rect[1] as f32,
                w: variant.rect[2] as f32,
                h: variant.rect[3] as f32,
            },
        })
    }

    fn fallback_variant(
        &self,
        group_id: &str,
        mask4: u8,
    ) -> Option<&TransitionAtlasManifestVariant> {
        let candidates = self
            .variants
            .iter()
            .filter(|variant| variant.group == group_id);
        if let Some(full_minus_one) = candidates
            .clone()
            .find(|variant| mask4 == 15 && variant.mask4 == 14)
        {
            return Some(full_minus_one);
        }

        candidates
            .filter(|variant| variant.mask4 & mask4 == variant.mask4)
            .max_by_key(|variant| variant.mask4.count_ones())
            .or_else(|| {
                self.variants
                    .iter()
                    .filter(|variant| variant.group == group_id)
                    .min_by_key(|variant| (variant.mask4 ^ mask4).count_ones())
            })
    }

    fn validate(&self) -> Result<(), String> {
        if self.tile_size == 0 {
            return Err(format!("{} has invalid tile_size 0", self.id));
        }
        if self.groups.is_empty() {
            return Err(format!("{} has no transition groups", self.id));
        }
        if self.variants.is_empty() {
            return Err(format!("{} has no transition variants", self.id));
        }
        if self.inner_corner_variants.is_empty() {
            return Err(format!(
                "{} has no inner-corner transition variants",
                self.id
            ));
        }
        for group in &self.groups {
            let count = self
                .variants
                .iter()
                .filter(|variant| &variant.group == group)
                .count();
            if count == 0 {
                return Err(format!(
                    "{} declares group {group} with no variants",
                    self.id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TransitionAtlasManifestVariant {
    pub id: String,
    pub group: String,
    #[serde(rename = "variantIndex")]
    pub variant_index: u8,
    pub mask4: u8,
    pub col: u32,
    pub row: u32,
    pub rect: [u32; 4],
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct TransitionAtlasManifestInnerCornerVariant {
    pub id: String,
    pub group: String,
    pub direction: String,
    pub rect: [u32; 4],
}

#[derive(Deserialize)]
struct TransitionAtlasManifestFile {
    id: String,
    kind: String,
    version: String,
    tile_size: u32,
    padding: u32,
    autotile_format: String,
    groups: Vec<String>,
    variants: Vec<TransitionAtlasManifestVariant>,
    #[serde(default, rename = "innerCornerVariants")]
    inner_corner_variants: Vec<TransitionAtlasManifestInnerCornerVariant>,
    source: Option<String>,
    output: String,
    license: Option<String>,
}

pub fn transition_atlas_manifest() -> Result<&'static TransitionAtlasManifest, &'static str> {
    TRANSITION_ATLAS_MANIFEST
        .get_or_init(TransitionAtlasManifest::load_default)
        .as_ref()
        .map_err(|error| error.as_str())
}

pub fn transition_atlas_texture_path() -> Result<&'static str, &'static str> {
    transition_atlas_manifest().map(TransitionAtlasManifest::atlas_path)
}

/// Placeholder atlases are useful for validating mask selection, but their
/// hard-edged diagnostic cells should not be presented as final terrain art.
pub fn transition_atlas_is_placeholder() -> bool {
    transition_atlas_manifest()
        .map(|manifest| {
            manifest
                .autotile_format
                .to_ascii_lowercase()
                .contains("placeholder")
        })
        .unwrap_or(true)
}

/// Return the atlas cell for a 4-way terrain transition mask. Bindings are read
/// from `terrain_autotile_47_32.json` so future shoreline, road, cliff, cave,
/// and water sheets can be extended without changing renderer row/column math.
pub fn transition_atlas_entry(group_id: &str, mask4: u8) -> Option<TransitionAtlasEntry> {
    transition_atlas_manifest()
        .ok()
        .and_then(|manifest| manifest.entry_for_mask(group_id, mask4))
}

pub fn transition_atlas_inner_corner_entry(
    group_id: &str,
    direction: &str,
) -> Option<TransitionAtlasInnerCornerEntry> {
    transition_atlas_manifest()
        .ok()
        .and_then(|manifest| manifest.inner_corner_entry(group_id, direction))
}

pub fn transition_atlas_group_is_supported(group_id: &str) -> bool {
    transition_atlas_manifest()
        .map(|manifest| manifest.group_is_supported(group_id))
        .unwrap_or_else(|_| {
            matches!(
                group_id,
                "grass_over_dirt"
                    | "grass_over_sand"
                    | "sand_over_wet_sand"
                    | "pebble_path_over_dirt"
                    | "grass_bank_over_shallow"
                    | "dirt_bank_over_shallow"
                    | "sand_bank_over_shallow"
                    | "shallow_rim_over_deep"
                    | "riverbank_mud"
            )
        })
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn normalize_repo_relative_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_autotile_has_empty_mask() {
        let map = TavernMap::empty_with(TileKind::Grass);
        assert_eq!(autotile_mask_8way(&map, TileKind::Road, 4, 4), 0);
    }

    #[test]
    fn mask_uses_cardinals_and_gated_corners() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::Road);
        map.set(4, 3, TileKind::Road);
        map.set(5, 4, TileKind::Road);
        map.set(5, 3, TileKind::Road);
        map.set(3, 3, TileKind::Road);

        let mask = autotile_mask_8way(&map, TileKind::Road, 4, 4);
        assert_eq!(mask & N, N);
        assert_eq!(mask & E, E);
        assert_eq!(mask & NE, NE);
        assert_eq!(mask & NW, 0);
    }

    #[test]
    fn out_of_bounds_neighbors_do_not_match() {
        let map = TavernMap::empty_with(TileKind::Wall);
        let neighbors = autotile_neighbors(&map, TileKind::Wall, 0, 0);
        assert!(!neighbors.north);
        assert!(!neighbors.west);
        assert!(neighbors.east);
        assert!(neighbors.south);
    }

    #[test]
    fn dirty_cells_are_bounded_when_requested() {
        let dirty = dirty_autotile_cells_in_bounds(0, 0);
        assert_eq!(dirty.len(), 4);
        assert!(dirty.contains(&(0, 0)));
        assert!(dirty.contains(&(1, 0)));
        assert!(dirty.contains(&(1, 1)));
        assert!(dirty.contains(&(0, 1)));
    }

    #[test]
    fn deterministic_variant_is_stable_and_bounded() {
        let first = deterministic_variant(42, 7, -3, 5);
        let second = deterministic_variant(42, 7, -3, 5);
        assert_eq!(first, second);
        assert!(first < 5);
        assert_eq!(deterministic_variant(42, 7, -3, 0), 0);
    }

    #[test]
    fn transition_atlas_manifest_reads_generated_metadata() {
        let manifest =
            TransitionAtlasManifest::load_default().expect("transition atlas manifest should load");
        assert_eq!(manifest.id, "terrain_autotile_47_32");
        assert_eq!(manifest.atlas_path(), TERRAIN_AUTOTILE_ATLAS_PATH);
        assert!(manifest.group_is_supported("sand_bank_over_shallow"));
        assert!(manifest
            .manifest_path()
            .ends_with(TERRAIN_AUTOTILE_ATLAS_MANIFEST_PATH));
        assert!(manifest.coverage_summary().contains("transition variants"));
    }

    #[test]
    fn transition_atlas_entry_maps_mask_to_padded_rect() {
        let entry = transition_atlas_entry("sand_bank_over_shallow", 3).expect("sand bank binding");
        assert_eq!(entry.sheet, TERRAIN_AUTOTILE_ATLAS_PATH);
        assert_eq!(entry.group_id, "sand_bank_over_shallow");
        assert_eq!(entry.mask4, 3);
        assert_eq!(entry.rect.w, 32.0);
        assert_eq!(entry.rect.h, 32.0);
        assert_eq!((entry.rect.x - 2.0) % 34.0, 0.0);
        assert_eq!((entry.rect.y - 2.0) % 34.0, 0.0);
    }

    #[test]
    fn riverbank_full_mask_is_explicitly_baked() {
        let entry = transition_atlas_entry("riverbank_mud", 15).expect("riverbank_mud binding");
        assert_eq!(entry.mask4, 15);
        assert_eq!(entry.group_id, "riverbank_mud");
        assert_eq!(entry.rect.w, 32.0);
        assert_eq!(entry.rect.h, 32.0);
    }
    #[test]
    fn dry_wet_sand_group_has_outer_and_inner_roles() {
        let edge =
            transition_atlas_entry("sand_over_wet_sand", 3).expect("dry/wet sand outer binding");
        assert_eq!(edge.group_id, "sand_over_wet_sand");
        assert_eq!(edge.mask4, 3);
        assert!(transition_atlas_inner_corner_entry("sand_over_wet_sand", "north_west").is_some());
    }

    #[test]
    fn pebble_path_group_is_intentionally_outer_only() {
        assert!(transition_atlas_entry("pebble_path_over_dirt", 1).is_some());
        assert!(
            transition_atlas_inner_corner_entry("pebble_path_over_dirt", "north_west").is_none()
        );
    }
}
