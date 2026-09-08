use haven_core::{ObjectFootprint, ObjectKind, TileKind};
use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const GENERATED_WORLDGEN_ROOT: &str = "assets/generated/worldgen_v0_1";
pub const GENERATED_WORLDGEN_MANIFEST_PATH: &str =
    "assets/generated/worldgen_v0_1/worldgen_asset_manifest_v0_1.json";
pub const GENERATED_BASE_TERRAIN_ATLAS_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/common_base_terrain_32.png";
pub const PROTOTYPE_TERRAIN_ATLAS_PATH: &str = GENERATED_BASE_TERRAIN_ATLAS_PATH;
pub const OBJECT_ATLAS_PATH: &str = "assets/generated/havenwild_lpc_objects_160x192_v2.png";
pub const OBJECT_ATLAS_MANIFEST_PATH: &str =
    "assets/generated/havenwild_lpc_objects_160x192_v2.json";
pub const OBJECT_ATLAS_CELL_WIDTH: f32 = 160.0;
pub const OBJECT_ATLAS_CELL_HEIGHT: f32 = 192.0;
pub const OBJECT_ATLAS_COLUMNS: usize = 8;
const GENERATED_BASE_TERRAIN_MANIFEST_NAME: &str = "terrain/common_base_terrain_32.json";

static GENERATED_ASSET_REGISTRY: OnceLock<Result<GeneratedAssetRegistry, String>> = OnceLock::new();
static OBJECT_FOOTPRINT_REGISTRY: OnceLock<Result<GeneratedObjectFootprintRegistry, String>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtlasRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub const MAX_TILE_VISUAL_VARIANTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileAssetEntry {
    pub stable_id: &'static str,
    pub sheet: &'static str,
    pub rect: AtlasRect,
    variant_rects: [AtlasRect; MAX_TILE_VISUAL_VARIANTS],
    variant_count: u8,
}

impl TileAssetEntry {
    pub fn variant_count(self) -> usize {
        self.variant_count.max(1) as usize
    }

    pub fn rect_for_cell(self, x: i32, y: i32) -> AtlasRect {
        let count = self.variant_count();
        if count <= 1 {
            return self.rect;
        }
        let index = deterministic_tile_variant(self.stable_id, x, y, count);
        self.variant_rects[index]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectAssetEntry {
    pub stable_id: &'static str,
    pub sheet: &'static str,
    pub rect: AtlasRect,
    pub foot_anchor: (f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedAssetRegistry {
    manifest_id: String,
    manifest_path: PathBuf,
    terrain_manifest_path: PathBuf,
    terrain_manifest_id: String,
    terrain_atlas_path: String,
    tile_entries: [TileAssetEntry; TileKind::ALL.len()],
}

impl GeneratedAssetRegistry {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(GENERATED_WORLDGEN_MANIFEST_PATH);
        let (manifest_id, terrain_manifest_path) =
            match load_json::<GeneratedRootManifest>(&manifest_path) {
                Ok(manifest) => {
                    let terrain_manifest_relative = manifest
                        .atlases
                        .iter()
                        .find(|path| path.as_str() == GENERATED_BASE_TERRAIN_MANIFEST_NAME)
                        .cloned()
                        .ok_or_else(|| {
                            format!(
                                "generated asset manifest {} does not declare {}",
                                manifest.id, GENERATED_BASE_TERRAIN_MANIFEST_NAME
                            )
                        })?;
                    let root_dir = manifest_path.parent().ok_or_else(|| {
                        format!("missing parent directory for {}", manifest_path.display())
                    })?;
                    (manifest.id, root_dir.join(&terrain_manifest_relative))
                }
                Err(_) => (
                    "lpc_only_direct_terrain_manifest".to_string(),
                    repo_root_dir()
                        .join(GENERATED_WORLDGEN_ROOT)
                        .join(GENERATED_BASE_TERRAIN_MANIFEST_NAME),
                ),
            };
        let terrain_manifest = load_json::<GeneratedTerrainManifest>(&terrain_manifest_path)?;
        let terrain_output = normalize_repo_relative_path(&terrain_manifest.output);
        if terrain_output != GENERATED_BASE_TERRAIN_ATLAS_PATH {
            return Err(format!(
                "generated terrain atlas output mismatch: expected {}, found {}",
                GENERATED_BASE_TERRAIN_ATLAS_PATH, terrain_output
            ));
        }

        let tile_entries = TileKind::ALL.map(|tile| {
            let manifest_id = generated_tile_manifest_id(tile);
            let manifest_entry = terrain_manifest
                .tiles
                .iter()
                .find(|entry| entry.id == manifest_id)
                .unwrap_or_else(|| {
                    panic!(
                        "missing generated terrain binding for TileKind::{} ({manifest_id}) in {}",
                        tile.label(),
                        terrain_manifest_path.display()
                    )
                });

            let rect = atlas_rect_from_u32(manifest_entry.rect);
            let mut variant_rects = [rect; MAX_TILE_VISUAL_VARIANTS];
            for (index, variant) in manifest_entry
                .variant_rects
                .iter()
                .take(MAX_TILE_VISUAL_VARIANTS)
                .enumerate()
            {
                variant_rects[index] = atlas_rect_from_u32(*variant);
            }
            let variant_count = manifest_entry
                .variant_rects
                .len()
                .clamp(1, MAX_TILE_VISUAL_VARIANTS) as u8;

            TileAssetEntry {
                stable_id: manifest_id,
                sheet: GENERATED_BASE_TERRAIN_ATLAS_PATH,
                rect,
                variant_rects,
                variant_count,
            }
        });

        Ok(Self {
            manifest_id,
            manifest_path,
            terrain_manifest_path,
            terrain_manifest_id: terrain_manifest.id,
            terrain_atlas_path: terrain_output,
            tile_entries,
        })
    }

    pub fn manifest_id(&self) -> &str {
        &self.manifest_id
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn terrain_manifest_id(&self) -> &str {
        &self.terrain_manifest_id
    }

    pub fn terrain_manifest_path(&self) -> &Path {
        &self.terrain_manifest_path
    }

    pub fn terrain_atlas_path(&self) -> &str {
        &self.terrain_atlas_path
    }

    pub fn tile_entry(&self, tile: TileKind) -> TileAssetEntry {
        self.tile_entries[tile_index(tile)]
    }

    pub fn tile_entries(&self) -> &[TileAssetEntry] {
        &self.tile_entries
    }

    pub fn coverage_summary(&self) -> String {
        format!(
            "{} -> {} bindings from {}",
            self.manifest_id,
            self.tile_entries.len(),
            self.terrain_manifest_id
        )
    }
}

pub fn tile_asset_entry(tile: TileKind) -> TileAssetEntry {
    generated_asset_registry()
        .unwrap_or_else(|error| panic!("failed to load generated asset registry: {error}"))
        .tile_entry(tile)
}

pub fn tile_asset_rect(tile: TileKind, x: i32, y: i32) -> AtlasRect {
    tile_asset_entry(tile).rect_for_cell(x, y)
}

pub fn object_asset_entry(kind: ObjectKind) -> Option<ObjectAssetEntry> {
    object_asset_entry_for_cell(kind, 0, 0)
}

pub fn object_asset_entry_for_cell(
    kind: ObjectKind,
    cell_x: i32,
    cell_y: i32,
) -> Option<ObjectAssetEntry> {
    let (index, stable_id) = object_asset_binding_for_cell(kind, cell_x, cell_y)?;
    Some(ObjectAssetEntry {
        stable_id,
        sheet: OBJECT_ATLAS_PATH,
        rect: AtlasRect {
            x: (index % OBJECT_ATLAS_COLUMNS) as f32 * OBJECT_ATLAS_CELL_WIDTH,
            y: (index / OBJECT_ATLAS_COLUMNS) as f32 * OBJECT_ATLAS_CELL_HEIGHT,
            w: OBJECT_ATLAS_CELL_WIDTH,
            h: OBJECT_ATLAS_CELL_HEIGHT,
        },
        foot_anchor: (OBJECT_ATLAS_CELL_WIDTH * 0.5, OBJECT_ATLAS_CELL_HEIGHT),
    })
}

pub fn audited_object_footprint_for_cell(
    kind: ObjectKind,
    cell_x: i32,
    cell_y: i32,
) -> ObjectFootprint {
    if !is_manifest_backed_natural_object(kind) {
        return kind.footprint_for_cell(cell_x, cell_y);
    }
    let Some((_, stable_id)) = object_asset_binding_for_cell(kind, cell_x, cell_y) else {
        return kind.footprint_for_cell(cell_x, cell_y);
    };
    object_footprint_registry()
        .ok()
        .and_then(|registry| registry.footprint(stable_id, kind))
        .unwrap_or_else(|| kind.footprint_for_cell(cell_x, cell_y))
}

fn is_manifest_backed_natural_object(kind: ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::Tree
            | ObjectKind::Bush
            | ObjectKind::Boulder
            | ObjectKind::OreNode
            | ObjectKind::Mushroom
            | ObjectKind::Herb
            | ObjectKind::Stump
            | ObjectKind::Log
    )
}

fn object_asset_binding_for_cell(
    kind: ObjectKind,
    cell_x: i32,
    cell_y: i32,
) -> Option<(usize, &'static str)> {
    let binding = match kind {
        ObjectKind::Tree => {
            const TREE_VARIANTS: [(usize, &str); 8] = [
                (0, "oak_tree"),
                (2, "oak_tree_variant_02"),
                (3, "oak_tree_variant_03"),
                (12, "oak_tree_variant_04"),
                (32, "oak_tree_variant_05"),
                (33, "oak_tree_variant_06"),
                (34, "oak_tree_variant_07"),
                (35, "oak_tree_variant_08"),
            ];
            TREE_VARIANTS[deterministic_object_variant("tree", cell_x, cell_y, TREE_VARIANTS.len())]
        }
        ObjectKind::Bush => {
            const BUSH_VARIANTS: [(usize, &str); 4] = [
                (1, "berry_bush"),
                (13, "berry_bush_variant_02"),
                (36, "berry_bush_variant_03"),
                (37, "berry_bush_variant_04"),
            ];
            BUSH_VARIANTS[deterministic_object_variant("bush", cell_x, cell_y, BUSH_VARIANTS.len())]
        }
        ObjectKind::Boulder => {
            const ROCK_VARIANTS: [(usize, &str); 4] = [
                (4, "boulder"),
                (38, "boulder_variant_02"),
                (39, "boulder_variant_03"),
                (40, "boulder_variant_04"),
            ];
            ROCK_VARIANTS
                [deterministic_object_variant("boulder", cell_x, cell_y, ROCK_VARIANTS.len())]
        }
        ObjectKind::OreNode => (5, "ore_node"),
        ObjectKind::Mushroom => {
            const MUSHROOM_VARIANTS: [(usize, &str); 4] = [
                (6, "forage_mushroom"),
                (14, "forage_mushroom_variant_02"),
                (41, "forage_mushroom_variant_03"),
                (42, "forage_mushroom_variant_04"),
            ];
            MUSHROOM_VARIANTS
                [deterministic_object_variant("mushroom", cell_x, cell_y, MUSHROOM_VARIANTS.len())]
        }
        ObjectKind::Herb => {
            const PLANT_VARIANTS: [(usize, &str); 8] = [
                (7, "wild_herb"),
                (15, "wildflower_patch"),
                (27, "reed_patch"),
                (43, "wild_herb_variant_02"),
                (44, "wild_herb_variant_03"),
                (45, "wildflower_patch_variant_02"),
                (46, "reed_patch_variant_02"),
                (47, "wildflower_patch_variant_03"),
            ];
            PLANT_VARIANTS
                [deterministic_object_variant("plant", cell_x, cell_y, PLANT_VARIANTS.len())]
        }
        ObjectKind::Table => (8, "table_round"),
        ObjectKind::Chair => (9, "chair_wood"),
        ObjectKind::Bar => (10, "bar_counter"),
        ObjectKind::Keg => (11, "keg"),
        ObjectKind::Bed => (16, "bed_basic"),
        ObjectKind::Fireplace => (17, "fireplace"),
        // W43B: current door cache has no audited source; structure certification will republish it.
        ObjectKind::Door => return None,
        // W43B: Ladder.png is not stairs. Keep the semantic ObjectKind but fail closed until real stairs are certified.
        ObjectKind::Stairs => return None,
        ObjectKind::Crate => (20, "crate_stack"),
        ObjectKind::Barrel => (21, "barrel"),
        // Z83 exposed ElizaWy `Water Cooler.png` as `well_pump`. It is not a
        // period-appropriate well and remains quarantined until a real authored
        // well/fountain is promoted.
        ObjectKind::Well => return None,
        // W43B: Dress Form / missing-cache substitutions are not valid scarecrow artwork.
        ObjectKind::Scarecrow => return None,
        // W43B: current generated fence cell has no audited LPC source.
        ObjectKind::Fence => return None,
        ObjectKind::Lamp => (25, "lamp_post"),
        // W43B: construction tape is a development placeholder, not a greenhouse visual.
        ObjectKind::GreenhouseMarker => return None,
        // W43B: long ottoman is not a production bench.
        ObjectKind::Bench => return None,
        ObjectKind::Stump => (29, "tree_stump"),
        // W43B: Lumber.png reads as cut lumber, not a fallen tree trunk.
        ObjectKind::Log => return None,
        // W43B: standing screen is not a production signboard.
        ObjectKind::Sign => return None,
        ObjectKind::CaveEntrance => return None,
    };
    Some(binding)
}

#[derive(Clone, Debug)]
struct GeneratedObjectFootprintRegistry {
    entries: std::collections::HashMap<String, GeneratedObjectFootprintEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedObjectFootprintEntry {
    id: String,
    visual_footprint_tiles: [i32; 2],
    collision_footprint_tiles: [i32; 2],
    #[serde(default)]
    occludes_player: bool,
    #[serde(default)]
    fade_when_player_behind: bool,
}

#[derive(Deserialize)]
struct GeneratedObjectAtlasManifest {
    objects: Vec<GeneratedObjectFootprintEntry>,
}

impl GeneratedObjectFootprintRegistry {
    fn load_default() -> Result<Self, String> {
        let path = repo_root_dir().join(OBJECT_ATLAS_MANIFEST_PATH);
        let manifest = load_json::<GeneratedObjectAtlasManifest>(&path)?;
        let entries = manifest
            .objects
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect();
        Ok(Self { entries })
    }

    fn footprint(&self, stable_id: &str, kind: ObjectKind) -> Option<ObjectFootprint> {
        let entry = self.entries.get(stable_id)?;
        let mut footprint = kind.default_footprint();
        let visual_w = entry.visual_footprint_tiles[0].max(1);
        let visual_h = entry.visual_footprint_tiles[1].max(1);
        let collision_w = entry.collision_footprint_tiles[0].max(0);
        let collision_h = entry.collision_footprint_tiles[1].max(0);
        footprint.visual_w = visual_w;
        footprint.visual_h = visual_h;
        footprint.visual_offset_x = if collision_w == 1 && visual_w % 2 == 1 {
            -(visual_w / 2)
        } else {
            0
        };
        footprint.visual_offset_y = 1 - visual_h;
        footprint.collision_offset_x = 0;
        footprint.collision_offset_y = 0;
        footprint.collision_w = collision_w;
        footprint.collision_h = collision_h;
        footprint.interaction_offset_x = 0;
        footprint.interaction_offset_y = 0;
        footprint.interaction_w = 1;
        footprint.interaction_h = 1;
        footprint.blocks_movement = collision_w > 0 && collision_h > 0;
        footprint.occludes_player = entry.occludes_player;
        footprint.fade_when_player_behind = entry.fade_when_player_behind;
        Some(footprint)
    }
}

fn object_footprint_registry() -> Result<&'static GeneratedObjectFootprintRegistry, &'static str> {
    OBJECT_FOOTPRINT_REGISTRY
        .get_or_init(GeneratedObjectFootprintRegistry::load_default)
        .as_ref()
        .map_err(|_| "generated object footprint registry unavailable")
}

fn deterministic_object_variant(stable_id: &str, x: i32, y: i32, count: usize) -> usize {
    deterministic_tile_variant(stable_id, x, y, count)
}

pub fn generated_asset_registry() -> Result<&'static GeneratedAssetRegistry, &'static str> {
    GENERATED_ASSET_REGISTRY
        .get_or_init(GeneratedAssetRegistry::load_default)
        .as_ref()
        .map_err(|error| error.as_str())
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_core")
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

fn generated_tile_manifest_id(tile: TileKind) -> &'static str {
    match tile {
        TileKind::Crop => "crop_seedling",
        _ => tile.code(),
    }
}

fn tile_index(tile: TileKind) -> usize {
    TileKind::ALL
        .iter()
        .position(|candidate| *candidate == tile)
        .unwrap_or_else(|| panic!("TileKind::{:?} missing from TileKind::ALL", tile))
}

#[derive(Deserialize)]
struct GeneratedRootManifest {
    id: String,
    atlases: Vec<String>,
}

#[derive(Deserialize)]
struct GeneratedTerrainManifest {
    id: String,
    output: String,
    tiles: Vec<GeneratedTerrainTileEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedTerrainTileEntry {
    id: String,
    rect: [u32; 4],
    #[serde(default)]
    variant_rects: Vec<[u32; 4]>,
}

fn atlas_rect_from_u32(rect: [u32; 4]) -> AtlasRect {
    AtlasRect {
        x: rect[0] as f32,
        y: rect[1] as f32,
        w: rect[2] as f32,
        h: rect[3] as f32,
    }
}

fn deterministic_tile_variant(stable_id: &str, x: i32, y: i32, count: usize) -> usize {
    let mut hash = 2_166_136_261u32;
    for byte in stable_id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash ^= (x as u32).wrapping_mul(0x9E37_79B9);
    hash = hash.rotate_left(13);
    hash ^= (y as u32).wrapping_mul(0x85EB_CA6B);
    (hash as usize) % count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_asset_registry_reads_worldgen_manifest() {
        let registry =
            GeneratedAssetRegistry::load_default().expect("generated registry should load");
        assert!(matches!(
            registry.manifest_id(),
            "worldgen_asset_manifest_v0_1" | "lpc_only_direct_terrain_manifest"
        ));
        assert_eq!(registry.terrain_manifest_id(), "common_base_terrain_32");
        assert_eq!(
            registry.terrain_atlas_path(),
            GENERATED_BASE_TERRAIN_ATLAS_PATH
        );
        assert!(registry
            .manifest_path()
            .ends_with("assets/generated/worldgen_v0_1/worldgen_asset_manifest_v0_1.json"));
        assert!(registry
            .terrain_manifest_path()
            .ends_with("assets/generated/worldgen_v0_1/terrain/common_base_terrain_32.json"));
    }

    fn generated_manifest_entry(id: &str) -> GeneratedTerrainTileEntry {
        let manifest_path = repo_root_dir()
            .join(GENERATED_WORLDGEN_ROOT)
            .join(GENERATED_BASE_TERRAIN_MANIFEST_NAME);
        let manifest: GeneratedTerrainManifest =
            load_json(&manifest_path).expect("generated terrain manifest should load");
        manifest
            .tiles
            .into_iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("missing generated terrain entry {id}"))
    }

    #[test]
    fn tile_entries_follow_generated_worldgen_manifest() {
        for tile in [TileKind::Grass, TileKind::DeepWater] {
            let entry = tile_asset_entry(tile);
            let manifest = generated_manifest_entry(generated_tile_manifest_id(tile));
            assert_eq!(entry.stable_id, generated_tile_manifest_id(tile));
            assert_eq!(entry.rect, atlas_rect_from_u32(manifest.rect));
        }
    }

    #[test]
    fn crop_tile_uses_generated_crop_seedling_binding() {
        let crop = tile_asset_entry(TileKind::Crop);
        let manifest = generated_manifest_entry("crop_seedling");
        assert_eq!(crop.stable_id, "crop_seedling");
        assert_eq!(crop.rect, atlas_rect_from_u32(manifest.rect));
    }

    #[test]
    fn tile_visual_variants_are_deterministic_and_manifest_backed() {
        let grass = tile_asset_entry(TileKind::Grass);
        let manifest = generated_manifest_entry("grass");
        let expected_count = manifest
            .variant_rects
            .len()
            .clamp(1, MAX_TILE_VISUAL_VARIANTS);
        assert_eq!(grass.variant_count(), expected_count);
        assert_eq!(grass.rect_for_cell(3, 7), grass.rect_for_cell(3, 7));
        if expected_count > 1 {
            assert!((0..64)
                .map(|x| grass.rect_for_cell(x, 5))
                .any(|rect| rect != grass.rect));
        } else {
            assert_eq!(grass.rect_for_cell(3, 7), grass.rect);
        }
    }

    #[test]
    fn object_entries_use_natural_scale_lpc_slots() {
        let table =
            object_asset_entry(ObjectKind::Table).expect("table should have an atlas entry");
        assert_eq!(table.stable_id, "table_round");
        assert_eq!(
            table.rect,
            AtlasRect {
                x: 0.0,
                y: 192.0,
                w: 160.0,
                h: 192.0
            }
        );

        assert!(object_asset_entry(ObjectKind::CaveEntrance).is_none());
        assert!(object_asset_entry(ObjectKind::Well).is_none());
    }

    #[test]
    fn audited_natural_footprints_follow_the_selected_authored_variant() {
        let tree = audited_object_footprint_for_cell(ObjectKind::Tree, 3, 9);
        assert_eq!((tree.visual_w, tree.visual_h), (3, 4));
        assert_eq!((tree.collision_w, tree.collision_h), (1, 1));

        let bush = audited_object_footprint_for_cell(ObjectKind::Bush, 4, 9);
        assert_eq!((bush.visual_w, bush.visual_h), (1, 1));
        assert_eq!((bush.collision_w, bush.collision_h), (1, 1));

        let mushroom = audited_object_footprint_for_cell(ObjectKind::Mushroom, 5, 9);
        assert_eq!((mushroom.visual_w, mushroom.visual_h), (1, 1));
        assert_eq!((mushroom.collision_w, mushroom.collision_h), (0, 0));
        assert!(!mushroom.blocks_movement);
    }

    #[test]
    fn audited_boulder_footprint_matches_the_same_deterministic_visual_variant() {
        for x in 0..64 {
            let entry = object_asset_entry_for_cell(ObjectKind::Boulder, x, 7)
                .expect("boulder atlas entry");
            let footprint = audited_object_footprint_for_cell(ObjectKind::Boulder, x, 7);
            match entry.stable_id {
                "boulder" => assert_eq!((footprint.visual_w, footprint.visual_h), (1, 1)),
                "boulder_variant_02" => {
                    assert_eq!((footprint.visual_w, footprint.visual_h), (2, 1))
                }
                "boulder_variant_03" | "boulder_variant_04" => {
                    assert_eq!((footprint.visual_w, footprint.visual_h), (2, 2))
                }
                other => panic!("unexpected boulder variant {other}"),
            }
        }
    }

    #[test]
    fn furniture_keeps_gameplay_authored_footprints_instead_of_atlas_crop_bounds() {
        let table = audited_object_footprint_for_cell(ObjectKind::Table, 0, 0);
        assert_eq!((table.visual_w, table.visual_h), (2, 2));
        assert_eq!((table.collision_w, table.collision_h), (2, 2));
    }

    #[test]
    fn tree_entries_use_multiple_elizawy_source_components() {
        let ids = (0..32)
            .map(|x| {
                object_asset_entry_for_cell(ObjectKind::Tree, x, 9)
                    .expect("tree should have an atlas entry")
                    .stable_id
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            ids.len() >= 4,
            "tree placement should exercise authored variants"
        );
        assert!(ids.iter().all(|id| id.starts_with("oak_tree")));
    }
}
