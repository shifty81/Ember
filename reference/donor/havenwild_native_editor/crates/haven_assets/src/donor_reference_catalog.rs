use crate::external_sources::ExternalAssetSourceRegistry;
use serde::Deserialize;
use std::{collections::BTreeSet, fs::read_to_string, path::Path};

pub const DONOR_REFERENCE_ASSET_CATALOG_PATH: &str =
    "content/assets/donor_reference/donor_reference_asset_catalog_v0_1.json";
pub const HAVENWILD_ASSET_CATALOG_PATH: &str =
    "content/assets/catalog/havenwild_asset_catalog_v0_1.json";
pub const PIXEL_EDITOR_REFERENCE_WORKBENCH_PATH: &str =
    "content/assets/pixel_editor/pixel_editor_reference_workbench_v0_1.json";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DonorReferenceAssetCatalog {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub rules: serde_json::Value,
    #[serde(default)]
    pub records: Vec<DonorReferenceAssetRecord>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DonorReferenceAssetRecord {
    pub id: String,
    pub external_source_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub catalog_role: String,
    #[serde(default)]
    pub tile_grid_profiles: Vec<[u32; 2]>,
    #[serde(default)]
    pub scale_adapter_modes: Vec<String>,
    #[serde(default)]
    pub families: Vec<String>,
    pub reference_use_policy: String,
    pub prototype_ingest_policy: String,
    pub editor_visibility: DonorEditorVisibility,
    #[serde(default)]
    pub pixel_editor_reference_mode: String,
    #[serde(default)]
    pub runtime_restrictions: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DonorEditorVisibility {
    #[serde(default)]
    pub standalone_editor: String,
    #[serde(default)]
    pub in_game_editor_dev_mode: String,
    #[serde(default)]
    pub lite_pixel_editor_panel: String,
    #[serde(default)]
    pub runtime_game_default: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HavenwildAssetCatalogIndex {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub indexes: Vec<HavenwildAssetCatalogIndexEntry>,
    #[serde(default)]
    pub editor_contracts: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HavenwildAssetCatalogIndexEntry {
    pub id: String,
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub editor_use: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PixelEditorReferenceWorkbench {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub default_mode: String,
    #[serde(default)]
    pub allowed_reference_actions: Vec<String>,
    #[serde(default)]
    pub blocked_actions: Vec<String>,
    #[serde(default)]
    pub tile_set_templates: Vec<PixelEditorTileSetTemplate>,
    #[serde(default)]
    pub catalog_path: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PixelEditorTileSetTemplate {
    pub id: String,
    pub tile_size: [u32; 2],
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub source_reference_families: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DonorReferenceSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DonorReferenceIssue {
    pub severity: DonorReferenceSeverity,
    pub id: String,
    pub message: String,
}

pub fn load_donor_reference_asset_catalog(
    repo_root: impl AsRef<Path>,
) -> Result<DonorReferenceAssetCatalog, String> {
    load_json(repo_root.as_ref().join(DONOR_REFERENCE_ASSET_CATALOG_PATH))
}

pub fn load_havenwild_asset_catalog_index(
    repo_root: impl AsRef<Path>,
) -> Result<HavenwildAssetCatalogIndex, String> {
    load_json(repo_root.as_ref().join(HAVENWILD_ASSET_CATALOG_PATH))
}

pub fn load_pixel_editor_reference_workbench(
    repo_root: impl AsRef<Path>,
) -> Result<PixelEditorReferenceWorkbench, String> {
    load_json(
        repo_root
            .as_ref()
            .join(PIXEL_EDITOR_REFERENCE_WORKBENCH_PATH),
    )
}

pub fn validate_donor_reference_asset_catalog(
    registry: &ExternalAssetSourceRegistry,
    catalog: &DonorReferenceAssetCatalog,
    asset_catalog: &HavenwildAssetCatalogIndex,
    workbench: &PixelEditorReferenceWorkbench,
) -> Vec<DonorReferenceIssue> {
    let mut issues = Vec::new();
    if catalog.schema != "havenwild.donor_reference_asset_catalog.v0_1" {
        issues.push(error(
            "donor_catalog.schema",
            format!("unsupported donor catalog schema {}", catalog.schema),
        ));
    }
    if asset_catalog.schema != "havenwild.asset_catalog.v0_1" {
        issues.push(error(
            "asset_catalog.schema",
            format!("unsupported asset catalog schema {}", asset_catalog.schema),
        ));
    }
    if workbench.schema != "havenwild.pixel_editor_reference_workbench.v0_1" {
        issues.push(error(
            "pixel_editor_workbench.schema",
            format!(
                "unsupported pixel editor reference schema {}",
                workbench.schema
            ),
        ));
    }
    if workbench.catalog_path != DONOR_REFERENCE_ASSET_CATALOG_PATH {
        issues.push(error(
            "pixel_editor_workbench.catalogPath",
            format!(
                "pixel editor workbench must point at {}, found {}",
                DONOR_REFERENCE_ASSET_CATALOG_PATH, workbench.catalog_path
            ),
        ));
    }

    let source_ids: BTreeSet<&str> = registry
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    let blocked_source_ids: BTreeSet<&str> = registry
        .sources
        .iter()
        .filter(|source| {
            source.commercial_runtime_policy.contains("blocked")
                || source.license_status.contains("unverified")
        })
        .map(|source| source.id.as_str())
        .collect();
    let mut donor_ids = BTreeSet::new();
    for record in &catalog.records {
        if !donor_ids.insert(record.id.as_str()) {
            issues.push(error(&record.id, "duplicate donor reference id"));
        }
        if !source_ids.contains(record.external_source_id.as_str()) {
            issues.push(error(
                &record.id,
                format!(
                    "donor record references unknown externalSourceId {}",
                    record.external_source_id
                ),
            ));
        }
        if record.tile_grid_profiles.is_empty() {
            issues.push(error(
                &record.id,
                "donor record must declare at least one tileGridProfile",
            ));
        }
        for [w, h] in &record.tile_grid_profiles {
            if *w == 0 || *h == 0 || *w > 256 || *h > 256 {
                issues.push(error(
                    &record.id,
                    format!(
                        "tileGridProfile {:?} is outside supported editor preview bounds",
                        [*w, *h]
                    ),
                ));
            }
        }
        if record.families.is_empty() {
            issues.push(warning(
                &record.id,
                "donor record has no families/tags for editor filtering",
            ));
        }
        if record.editor_visibility.standalone_editor.is_empty()
            || record.editor_visibility.in_game_editor_dev_mode.is_empty()
            || record.editor_visibility.lite_pixel_editor_panel.is_empty()
            || record.editor_visibility.runtime_game_default.is_empty()
        {
            issues.push(error(
                &record.id,
                "donor editorVisibility must declare standaloneEditor, inGameEditorDevMode, litePixelEditorPanel, and runtimeGameDefault",
            ));
        }
        if blocked_source_ids.contains(record.external_source_id.as_str()) {
            if !record.prototype_ingest_policy.contains("blocked")
                && !record.prototype_ingest_policy.contains("license")
                && !record.prototype_ingest_policy.contains("attribution")
            {
                issues.push(error(
                    &record.id,
                    "blocked/unverified source donor record must remain blocked until license/attribution gate passes",
                ));
            }
            if record.editor_visibility.runtime_game_default != "blocked" {
                issues.push(error(
                    &record.id,
                    "blocked/unverified source must have runtimeGameDefault=blocked",
                ));
            }
        }
        if record.pixel_editor_reference_mode.is_empty() {
            issues.push(warning(
                &record.id,
                "pixelEditorReferenceMode should describe reference/recreation behavior",
            ));
        }
    }

    let index_paths: BTreeSet<&str> = asset_catalog
        .indexes
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    for required in [
        "content/assets/external_sources/external_asset_sources_v0_1.json",
        DONOR_REFERENCE_ASSET_CATALOG_PATH,
        "content/assets/prototype_imports/prototype_asset_import_queue_v0_1.json",
        "content/assets/prototype_imports/prototype_asset_import_adapters_v0_1.json",
    ] {
        if !index_paths.contains(required) {
            issues.push(error(
                "asset_catalog.indexes",
                format!("unified asset catalog is missing required index path {required}"),
            ));
        }
    }

    if !workbench
        .blocked_actions
        .iter()
        .any(|action| action.contains("trace_restricted_asset_pixels"))
    {
        issues.push(error(
            "pixel_editor_workbench.blockedActions",
            "pixel editor reference workbench must explicitly block tracing restricted donor pixels",
        ));
    }
    if workbench.tile_set_templates.is_empty() {
        issues.push(error(
            "pixel_editor_workbench.tileSetTemplates",
            "pixel editor workbench must define at least one Havenwild-original tileset template",
        ));
    }
    issues
}

pub fn donor_reference_records_for_family<'a>(
    catalog: &'a DonorReferenceAssetCatalog,
    family: &str,
) -> Vec<&'a DonorReferenceAssetRecord> {
    catalog
        .records
        .iter()
        .filter(|record| record.families.iter().any(|tag| tag == family))
        .collect()
}

fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn warning(id: impl Into<String>, message: impl Into<String>) -> DonorReferenceIssue {
    DonorReferenceIssue {
        severity: DonorReferenceSeverity::Warning,
        id: id.into(),
        message: message.into(),
    }
}

fn error(id: impl Into<String>, message: impl Into<String>) -> DonorReferenceIssue {
    DonorReferenceIssue {
        severity: DonorReferenceSeverity::Error,
        id: id.into(),
        message: message.into(),
    }
}
