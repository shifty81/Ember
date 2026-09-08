use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::read_to_string,
    path::Path,
};

pub const WORLD_TILE_CONTRACT_PATH: &str =
    "content/assets/world_tiles/world_tile_contract_v0_1.json";
pub const HAVENWILD_WORLD_ENVIRONMENT_TEST_ATLAS_MANIFEST_PATH: &str =
    "content/assets/world_tiles/havenwild_world_environment_test_v0_1.json";
pub const CANONICAL_WORLD_TILE_SIZE: [u32; 2] = [32, 32];
pub const SUPPORTED_SUBCELL_SIZES: [[u32; 2]; 3] = [[16, 16], [8, 8], [4, 4]];

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileContract {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    pub canonical_tile_size: [u32; 2],
    #[serde(default)]
    pub supported_donor_grid_sizes: Vec<[u32; 2]>,
    pub runtime_canonical_bake_size: [u32; 2],
    #[serde(default)]
    pub subcell_masks: Vec<WorldTileSubcellMask>,
    #[serde(default)]
    pub environment_families_v1: Vec<String>,
    #[serde(default)]
    pub layer_stack_v1: Vec<WorldTileLayerRole>,
    #[serde(default)]
    pub brush_modes_v1: Vec<String>,
    #[serde(default)]
    pub naming_rules: WorldTileNamingRules,
    #[serde(default)]
    pub multiplayer_rules: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileSubcellMask {
    pub id: String,
    pub cell_size: [u32; 2],
    pub grid: [u32; 2],
    #[serde(default)]
    pub purpose: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileLayerRole {
    pub order: u32,
    pub id: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub rendered: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileNamingRules {
    #[serde(default)]
    pub base_tile_pattern: String,
    #[serde(default)]
    pub transition_pattern: String,
    #[serde(default)]
    pub overlay_pattern: String,
    #[serde(default)]
    pub strict_case: String,
    #[serde(default)]
    pub allowed_suffix: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileAtlasManifest {
    pub schema: String,
    pub id: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    pub canonical_tile_size: [u32; 2],
    pub source_atlas: String,
    pub source_image_size: [u32; 2],
    pub columns: u32,
    pub rows: u32,
    #[serde(default)]
    pub source_policy: String,
    #[serde(default)]
    pub runtime_default: bool,
    #[serde(default)]
    pub records: Vec<WorldTileRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorldTileRecord {
    pub id: String,
    pub source_atlas: String,
    pub atlas_rect: [u32; 4],
    pub grid_size: [u32; 2],
    pub family: String,
    pub layer: String,
    #[serde(default)]
    pub variant: String,
    #[serde(default)]
    pub autotile_role: String,
    #[serde(default)]
    pub supports_subcell_paint: bool,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub runtime_default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldTileContractSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldTileContractIssue {
    pub severity: WorldTileContractSeverity,
    pub id: String,
    pub message: String,
}

pub fn load_world_tile_contract(repo_root: impl AsRef<Path>) -> Result<WorldTileContract, String> {
    load_json(repo_root.as_ref().join(WORLD_TILE_CONTRACT_PATH))
}

pub fn load_world_tile_atlas_manifest(
    repo_root: impl AsRef<Path>,
) -> Result<WorldTileAtlasManifest, String> {
    load_json(
        repo_root
            .as_ref()
            .join(HAVENWILD_WORLD_ENVIRONMENT_TEST_ATLAS_MANIFEST_PATH),
    )
}

pub fn validate_world_tile_contract(
    contract: &WorldTileContract,
    manifest: &WorldTileAtlasManifest,
) -> Vec<WorldTileContractIssue> {
    let mut issues = Vec::new();
    if contract.schema != "havenwild.world_tile_contract.v0_1" {
        issues.push(error(
            "world_tile_contract.schema",
            format!("unsupported schema {}", contract.schema),
        ));
    }
    if contract.canonical_tile_size != CANONICAL_WORLD_TILE_SIZE {
        issues.push(error(
            "world_tile_contract.canonicalTileSize",
            format!(
                "canonical tile size must be {:?}",
                CANONICAL_WORLD_TILE_SIZE
            ),
        ));
    }
    if contract.runtime_canonical_bake_size != CANONICAL_WORLD_TILE_SIZE {
        issues.push(error(
            "world_tile_contract.runtimeCanonicalBakeSize",
            "runtime canonical bake size must stay 32x32",
        ));
    }
    let declared_subcells: BTreeSet<[u32; 2]> = contract
        .subcell_masks
        .iter()
        .map(|mask| mask.cell_size)
        .collect();
    for required in SUPPORTED_SUBCELL_SIZES {
        if !declared_subcells.contains(&required) {
            issues.push(error(
                "world_tile_contract.subcellMasks",
                format!("missing required subcell size {:?}", required),
            ));
        }
    }
    let families: BTreeSet<&str> = contract
        .environment_families_v1
        .iter()
        .map(String::as_str)
        .collect();
    for required in ["sand", "water", "cave", "paved_brick", "wood_plank"] {
        if !families.contains(required) {
            issues.push(error(
                "world_tile_contract.environmentFamiliesV1",
                format!("missing required world environment family {required}"),
            ));
        }
    }
    let layer_orders: BTreeMap<u32, &String> = contract
        .layer_stack_v1
        .iter()
        .map(|layer| (layer.order, &layer.id))
        .collect();
    if layer_orders.len() != contract.layer_stack_v1.len() {
        issues.push(error(
            "world_tile_contract.layerStackV1",
            "duplicate layer order in world tile layer stack",
        ));
    }
    let layer_ids: BTreeSet<&str> = contract
        .layer_stack_v1
        .iter()
        .map(|layer| layer.id.as_str())
        .collect();
    for required in [
        "ground_base",
        "water_base",
        "ground_transition_fringe",
        "cave_base",
        "town_surface",
        "indoor_floor",
        "collision_footprint",
        "occlusion_fade_mask",
        "dev_overlay",
    ] {
        if !layer_ids.contains(required) {
            issues.push(error(
                "world_tile_contract.layerStackV1",
                format!("missing required layer {required}"),
            ));
        }
    }
    if manifest.schema != "havenwild.world_tile_atlas_manifest.v0_1" {
        issues.push(error(
            "world_tile_atlas_manifest.schema",
            format!("unsupported schema {}", manifest.schema),
        ));
    }
    if manifest.canonical_tile_size != CANONICAL_WORLD_TILE_SIZE {
        issues.push(error(
            "world_tile_atlas_manifest.canonicalTileSize",
            "atlas manifest must be canonical 32x32",
        ));
    }
    if manifest.source_image_size[0] != manifest.columns * 32
        || manifest.source_image_size[1] != manifest.rows * 32
    {
        issues.push(error(
            "world_tile_atlas_manifest.sourceImageSize",
            "source image size must match columns/rows at 32px",
        ));
    }
    let mut record_ids = BTreeSet::new();
    for record in &manifest.records {
        if !record_ids.insert(record.id.as_str()) {
            issues.push(error(&record.id, "duplicate world tile record id"));
        }
        if record.grid_size != CANONICAL_WORLD_TILE_SIZE {
            issues.push(error(
                &record.id,
                "world tile record gridSize must be 32x32",
            ));
        }
        if record.atlas_rect[2] != 32
            || record.atlas_rect[3] != 32
            || record.atlas_rect[0] % 32 != 0
            || record.atlas_rect[1] % 32 != 0
        {
            issues.push(error(
                &record.id,
                "atlasRect must be aligned to the 32x32 canonical grid",
            ));
        }
        if !record.id.ends_with("_32x32") {
            issues.push(error(
                &record.id,
                "world tile record id must end with _32x32",
            ));
        }
        if !families.contains(record.family.as_str()) && record.family != "debug" {
            issues.push(error(
                &record.id,
                format!("unknown world tile family {}", record.family),
            ));
        }
        if !layer_ids.contains(record.layer.as_str()) {
            issues.push(error(
                &record.id,
                format!("unknown world tile layer {}", record.layer),
            ));
        }
    }
    issues
}

fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let text =
        read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn error(id: impl Into<String>, message: impl Into<String>) -> WorldTileContractIssue {
    WorldTileContractIssue {
        severity: WorldTileContractSeverity::Error,
        id: id.into(),
        message: message.into(),
    }
}
