use crate::{
    asset_registry::{object_asset_entry, tile_asset_entry, AtlasRect},
    lpc_mapped_terrain::{lpc_mapped_terrain_atlas_path, lpc_mapped_terrain_preview_entry},
    stamp_registry::StampRegistry,
    user_asset_registry::load_user_asset_registry_default,
};
use haven_core::{ObjectKind, TileKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
};

pub const ASSET_PALETTE_STATE_PATH: &str = ".local/editor/asset_palette_state.json";

pub const PALETTE_OBJECTS: [ObjectKind; 25] = [
    ObjectKind::Table,
    ObjectKind::Chair,
    ObjectKind::Bar,
    ObjectKind::Keg,
    ObjectKind::Bed,
    ObjectKind::Fireplace,
    ObjectKind::GreenhouseMarker,
    ObjectKind::Tree,
    ObjectKind::Bush,
    ObjectKind::Boulder,
    ObjectKind::OreNode,
    ObjectKind::Mushroom,
    ObjectKind::Herb,
    ObjectKind::Crate,
    ObjectKind::Barrel,
    ObjectKind::Scarecrow,
    ObjectKind::Fence,
    ObjectKind::Lamp,
    ObjectKind::Bench,
    ObjectKind::Stump,
    ObjectKind::Log,
    ObjectKind::Sign,
    ObjectKind::Door,
    ObjectKind::Stairs,
    ObjectKind::CaveEntrance,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetPaletteCategory {
    All,
    /// Natural walkable ground materials.
    Terrain,
    /// Roads, civic paths, and mountain trails.
    Paths,
    Floor,
    Wall,
    /// Doors, stairs, bridges, and cave entrances.
    Access,
    Farm,
    Water,
    /// Rock ground and derived structural-cliff authoring entries.
    Elevation,
    /// Trees, plants, mushrooms, logs, and other natural decoration.
    Nature,
    /// Boulders, ore nodes, and harvestable world resources.
    Resources,
    /// Furniture, storage, lighting, and civic placeables.
    Objects,
    Stamps,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetPaletteTreeGroup {
    Terrain,
    Structures,
    WorldObjects,
}

impl AssetPaletteTreeGroup {
    pub const ALL: [Self; 3] = [Self::Terrain, Self::Structures, Self::WorldObjects];

    pub fn label(self) -> &'static str {
        match self {
            Self::Terrain => "Terrain",
            Self::Structures => "Structures",
            Self::WorldObjects => "World Objects",
        }
    }

    pub fn first_category(self) -> AssetPaletteCategory {
        match self {
            Self::Terrain => AssetPaletteCategory::Terrain,
            Self::Structures => AssetPaletteCategory::Floor,
            Self::WorldObjects => AssetPaletteCategory::Nature,
        }
    }
}

impl AssetPaletteCategory {
    pub const ALL: [Self; 13] = [
        Self::All,
        Self::Terrain,
        Self::Paths,
        Self::Water,
        Self::Elevation,
        Self::Floor,
        Self::Wall,
        Self::Access,
        Self::Nature,
        Self::Resources,
        Self::Objects,
        Self::Farm,
        Self::Stamps,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Assets",
            Self::Terrain => "Ground",
            Self::Paths => "Paths & Roads",
            Self::Floor => "Floors",
            Self::Wall => "Walls",
            Self::Access => "Doors & Access",
            Self::Farm => "Farming",
            Self::Water => "Water",
            Self::Elevation => "Elevation & Cliffs",
            Self::Nature => "Trees & Flora",
            Self::Resources => "Rocks & Resources",
            Self::Objects => "Furniture & Props",
            Self::Stamps => "Reusable Stamps",
        }
    }

    pub fn tree_group(self) -> Option<AssetPaletteTreeGroup> {
        match self {
            Self::Terrain | Self::Paths | Self::Water | Self::Elevation => {
                Some(AssetPaletteTreeGroup::Terrain)
            }
            Self::Floor | Self::Wall | Self::Access => Some(AssetPaletteTreeGroup::Structures),
            Self::Nature | Self::Resources | Self::Objects | Self::Farm => {
                Some(AssetPaletteTreeGroup::WorldObjects)
            }
            Self::All | Self::Stamps => None,
        }
    }

    pub fn children(group: AssetPaletteTreeGroup) -> &'static [Self] {
        match group {
            AssetPaletteTreeGroup::Terrain => {
                &[Self::Terrain, Self::Paths, Self::Water, Self::Elevation]
            }
            AssetPaletteTreeGroup::Structures => &[Self::Floor, Self::Wall, Self::Access],
            AssetPaletteTreeGroup::WorldObjects => {
                &[Self::Nature, Self::Resources, Self::Objects, Self::Farm]
            }
        }
    }

    pub fn matches(self, entry: &AssetPaletteEntry) -> bool {
        self == Self::All || self == entry.category
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetPaletteKind {
    Tile(TileKind),
    Object(ObjectKind),
    Stamp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetProvenance {
    SourcePureNormalized,
    ProjectGenerated,
    ProjectOwnedImport,
    MissingRuntimeBinding,
}

impl AssetProvenance {
    pub fn label(self) -> &'static str {
        match self {
            Self::SourcePureNormalized => "source-pure normalized atlas",
            Self::ProjectGenerated => "project-generated",
            Self::ProjectOwnedImport => "promoted project asset",
            Self::MissingRuntimeBinding => "missing runtime binding",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssetPaletteEntry {
    pub stable_id: String,
    pub label: String,
    pub category: AssetPaletteCategory,
    pub kind: AssetPaletteKind,
    pub sheet: Option<String>,
    pub rect: Option<AtlasRect>,
    pub provenance: AssetProvenance,
    pub warning: Option<String>,
    pub keywords: Vec<String>,
}

impl AssetPaletteEntry {
    pub fn runtime_ready(&self) -> bool {
        self.sheet.is_some() && self.rect.is_some() && self.warning.is_none()
    }

    pub fn matches_search(&self, search: &str) -> bool {
        let search = search.trim().to_ascii_lowercase();
        if search.is_empty() {
            return true;
        }
        self.label.to_ascii_lowercase().contains(&search)
            || self.stable_id.to_ascii_lowercase().contains(&search)
            || self
                .keywords
                .iter()
                .any(|keyword| keyword.to_ascii_lowercase().contains(&search))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetPaletteCatalog {
    entries: Vec<AssetPaletteEntry>,
}

impl AssetPaletteCatalog {
    pub fn load_default() -> Result<Self, String> {
        let root = repo_root_dir();
        let user_registry = load_user_asset_registry_default()
            .unwrap_or_else(|_| crate::user_asset_registry::UserAssetRegistry::empty());
        let mut entries = Vec::new();
        for tile in TileKind::ALL {
            // WetSand remains a legacy save alias only, and WateredSoil is a
            // farming-state result produced by watering TilledSoil. Neither
            // belongs in the direct terrain/asset paint palette.
            if matches!(tile, TileKind::WetSand | TileKind::WateredSoil) {
                continue;
            }
            let generated = tile_asset_entry(tile);
            let imported = user_registry.binding_for_tile(tile);
            let mapped = tile
                .is_lpc_mapped_editor_terrain()
                .then(|| {
                    Some((
                        lpc_mapped_terrain_atlas_path().ok()?.to_string(),
                        lpc_mapped_terrain_preview_entry(tile)?.rect,
                        AssetProvenance::SourcePureNormalized,
                    ))
                })
                .flatten();
            let (sheet, rect, provenance) = mapped
                .or_else(|| {
                    imported.map(|binding| {
                        (
                            binding.sheet.clone(),
                            binding.rect,
                            AssetProvenance::ProjectOwnedImport,
                        )
                    })
                })
                .unwrap_or_else(|| {
                    (
                        generated.sheet.to_string(),
                        generated.rect,
                        AssetProvenance::ProjectGenerated,
                    )
                });
            let sheet_exists = root.join(&sheet).is_file();
            entries.push(AssetPaletteEntry {
                stable_id: format!("tile/{}", tile.code()),
                label: if tile.is_lpc_mapped_editor_terrain() {
                    format!("{} [V7]", tile.label())
                } else {
                    tile.label().to_string()
                },
                category: category_for_tile_kind(tile),
                kind: AssetPaletteKind::Tile(tile),
                sheet: Some(sheet.clone()),
                rect: Some(rect),
                provenance,
                warning: (!sheet_exists).then(|| format!("missing atlas {sheet}")),
                keywords: vec![
                    "tile".to_string(),
                    tile.category().label().to_ascii_lowercase(),
                    tile.code().replace('_', " "),
                    if tile.is_lpc_mapped_editor_terrain() {
                        "v7 terrain style".to_string()
                    } else {
                        "project terrain".to_string()
                    },
                ],
            });
        }
        for object in PALETTE_OBJECTS {
            let imported = user_registry.binding_for_object(object);
            let generated = object_asset_entry(object);
            let (sheet, rect, warning, provenance) = if let Some(binding) = imported {
                let exists = root.join(&binding.sheet).is_file();
                (
                    Some(binding.sheet.clone()),
                    Some(binding.rect),
                    (!exists).then(|| format!("missing promoted atlas {}", binding.sheet)),
                    AssetProvenance::ProjectOwnedImport,
                )
            } else {
                match generated {
                    Some(binding) => {
                        let exists = root.join(binding.sheet).is_file();
                        (
                            Some(binding.sheet.to_string()),
                            Some(binding.rect),
                            (!exists).then(|| format!("missing generated atlas {}", binding.sheet)),
                            AssetProvenance::ProjectGenerated,
                        )
                    }
                    None => (
                        None,
                        None,
                        Some(format!(
                            "{} has no generated runtime sprite binding yet",
                            object.label()
                        )),
                        AssetProvenance::MissingRuntimeBinding,
                    ),
                }
            };
            entries.push(AssetPaletteEntry {
                stable_id: format!("object/{}", object.code()),
                label: object.label().to_string(),
                category: category_for_object(object),
                kind: AssetPaletteKind::Object(object),
                sheet,
                rect,
                provenance,
                warning,
                keywords: vec!["object".to_string(), object.code().replace('_', " ")],
            });
        }
        let stamp_registry = StampRegistry::load_default().unwrap_or_default();
        for stamp in stamp_registry.entries() {
            let sheet_exists = root.join(&stamp.sheet).is_file();
            entries.push(AssetPaletteEntry {
                stable_id: format!("stamp/{}", stamp.stable_id),
                label: stamp.label.clone(),
                category: category_for_stamp(&stamp.category),
                kind: AssetPaletteKind::Stamp,
                sheet: Some(stamp.sheet.clone()),
                rect: Some(stamp.rect),
                provenance: AssetProvenance::ProjectGenerated,
                warning: (!sheet_exists).then(|| format!("missing stamp atlas {}", stamp.sheet)),
                keywords: vec![
                    "stamp".to_string(),
                    stamp.category.clone(),
                    format!("{}x{}", stamp.footprint.visual_w, stamp.footprint.visual_h),
                    stamp.stable_id.replace('_', " "),
                ],
            });
        }
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[AssetPaletteEntry] {
        &self.entries
    }

    pub fn entry(&self, stable_id: &str) -> Option<&AssetPaletteEntry> {
        self.entries
            .iter()
            .find(|entry| entry.stable_id == stable_id)
    }

    pub fn filtered<'a>(
        &'a self,
        category: AssetPaletteCategory,
        search: &str,
        favorites_only: bool,
        state: &AssetPaletteState,
    ) -> Vec<&'a AssetPaletteEntry> {
        self.entries
            .iter()
            .filter(|entry| category.matches(entry))
            .filter(|entry| entry.matches_search(search))
            .filter(|entry| !favorites_only || state.is_favorite(&entry.stable_id))
            .collect()
    }

    pub fn coverage_summary(&self) -> String {
        let ready = self
            .entries
            .iter()
            .filter(|entry| entry.runtime_ready())
            .count();
        let warnings = self.entries.len().saturating_sub(ready);
        format!(
            "{} palette assets: {} runtime-ready, {} warning{}",
            self.entries.len(),
            ready,
            warnings,
            if warnings == 1 { "" } else { "s" }
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPaletteState {
    #[serde(default)]
    favorites: BTreeSet<String>,
    #[serde(default)]
    recent: Vec<String>,
}

impl AssetPaletteState {
    pub fn load_default() -> Self {
        let path = repo_root_dir().join(ASSET_PALETTE_STATE_PATH);
        read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save_default(&self) -> Result<(), String> {
        let path = repo_root_dir().join(ASSET_PALETTE_STATE_PATH);
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize asset palette state: {error}"))?;
        write(&path, raw).map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    pub fn favorites(&self) -> &BTreeSet<String> {
        &self.favorites
    }

    pub fn recent(&self) -> &[String] {
        &self.recent
    }

    pub fn is_favorite(&self, stable_id: &str) -> bool {
        self.favorites.contains(stable_id)
    }

    pub fn toggle_favorite(&mut self, stable_id: &str) -> bool {
        if self.favorites.remove(stable_id) {
            false
        } else {
            self.favorites.insert(stable_id.to_string());
            true
        }
    }

    pub fn mark_recent(&mut self, stable_id: &str) {
        self.recent.retain(|candidate| candidate != stable_id);
        self.recent.insert(0, stable_id.to_string());
        self.recent.truncate(12);
    }
}

fn category_for_tile_kind(tile: TileKind) -> AssetPaletteCategory {
    match tile {
        TileKind::Road | TileKind::StonePath | TileKind::MountainPath => {
            AssetPaletteCategory::Paths
        }
        TileKind::Water
        | TileKind::ShallowWater
        | TileKind::DeepWater
        | TileKind::OceanDeep
        | TileKind::OceanShallow
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend
        | TileKind::ShoreFoam => AssetPaletteCategory::Water,
        TileKind::MountainRock | TileKind::Cliff => AssetPaletteCategory::Elevation,
        TileKind::WoodFloor
        | TileKind::PlankFloor
        | TileKind::StoneFloor
        | TileKind::BrickFloor => AssetPaletteCategory::Floor,
        TileKind::Wall | TileKind::CaveWall => AssetPaletteCategory::Wall,
        TileKind::Bridge => AssetPaletteCategory::Access,
        TileKind::TilledSoil
        | TileKind::WateredSoil
        | TileKind::Crop
        | TileKind::GreenhouseZone => AssetPaletteCategory::Farm,
        TileKind::Grass
        | TileKind::TallGrass
        | TileKind::Sand
        | TileKind::WetSand
        | TileKind::PebbleShore
        | TileKind::Dirt
        | TileKind::CaveFloor
        | TileKind::MudBank => AssetPaletteCategory::Terrain,
    }
}

fn category_for_stamp(category: &str) -> AssetPaletteCategory {
    if category.starts_with("terrain/cliffs") {
        AssetPaletteCategory::Elevation
    } else {
        AssetPaletteCategory::Stamps
    }
}

fn category_for_object(object: ObjectKind) -> AssetPaletteCategory {
    match object {
        ObjectKind::Tree
        | ObjectKind::Bush
        | ObjectKind::Mushroom
        | ObjectKind::Herb
        | ObjectKind::Stump
        | ObjectKind::Log => AssetPaletteCategory::Nature,
        ObjectKind::Boulder | ObjectKind::OreNode => AssetPaletteCategory::Resources,
        ObjectKind::GreenhouseMarker | ObjectKind::Scarecrow | ObjectKind::Fence => {
            AssetPaletteCategory::Farm
        }
        ObjectKind::Door | ObjectKind::Stairs | ObjectKind::CaveEntrance => {
            AssetPaletteCategory::Access
        }
        ObjectKind::Table
        | ObjectKind::Chair
        | ObjectKind::Bar
        | ObjectKind::Keg
        | ObjectKind::Bed
        | ObjectKind::Fireplace
        | ObjectKind::Crate
        | ObjectKind::Barrel
        | ObjectKind::Well
        | ObjectKind::Lamp
        | ObjectKind::Bench
        | ObjectKind::Sign => AssetPaletteCategory::Objects,
    }
}

fn repo_root_dir() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if current.join("assets").is_dir() && current.join("content").is_dir() {
        return current;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_covers_tiles_and_supported_objects() {
        let catalog = AssetPaletteCatalog::load_default().expect("palette should load");
        let non_paintable_state_tiles = 2; // WetSand alias + WateredSoil farm state.
        assert_eq!(
            catalog.entries().len(),
            TileKind::ALL.len() - non_paintable_state_tiles
                + PALETTE_OBJECTS.len()
                + StampRegistry::load_default()
                    .expect("stamp registry")
                    .entries()
                    .len()
        );
        assert!(catalog.entry("tile/wet_sand").is_none());
        assert!(catalog.entry("tile/watered_soil").is_none());
        assert!(catalog.entry("tile/mud_bank").is_some());
        assert!(catalog.entry("tile/grass").is_some());
        assert!(catalog.entry("object/table").is_some());
        assert!(catalog
            .entries()
            .iter()
            .any(|entry| matches!(entry.kind, AssetPaletteKind::Stamp)));
        assert!(!catalog
            .entries()
            .iter()
            .any(|entry| entry.stable_id.contains("oga_lpc.cliff")));
    }

    #[test]
    fn favorites_and_recent_assets_remain_deterministic() {
        let mut state = AssetPaletteState::default();
        assert!(state.toggle_favorite("tile/grass"));
        state.mark_recent("tile/grass");
        state.mark_recent("object/table");
        assert!(state.is_favorite("tile/grass"));
        assert_eq!(state.recent()[0], "object/table");
    }

    #[test]
    fn semantic_folder_tree_routes_common_assets() {
        assert_eq!(
            category_for_tile_kind(TileKind::Grass),
            AssetPaletteCategory::Terrain
        );
        assert_eq!(
            category_for_tile_kind(TileKind::Road),
            AssetPaletteCategory::Paths
        );
        assert_eq!(
            category_for_tile_kind(TileKind::MountainRock),
            AssetPaletteCategory::Elevation
        );
        assert_eq!(
            category_for_object(ObjectKind::Tree),
            AssetPaletteCategory::Nature
        );
        assert_eq!(
            category_for_object(ObjectKind::Boulder),
            AssetPaletteCategory::Resources
        );
        assert_eq!(
            category_for_object(ObjectKind::Table),
            AssetPaletteCategory::Objects
        );
        assert_eq!(
            AssetPaletteCategory::children(AssetPaletteTreeGroup::Structures),
            &[
                AssetPaletteCategory::Floor,
                AssetPaletteCategory::Wall,
                AssetPaletteCategory::Access
            ]
        );
    }
}
