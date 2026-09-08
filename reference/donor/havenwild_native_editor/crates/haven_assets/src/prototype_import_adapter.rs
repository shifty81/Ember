use crate::external_sources::{ExternalAssetSourceRegistry, PrototypeAssetImportQueue};
use serde::Deserialize;
use std::{collections::BTreeSet, fs::read_to_string, path::Path};

pub const PROTOTYPE_ASSET_IMPORT_ADAPTER_CATALOG_PATH: &str =
    "content/assets/prototype_imports/prototype_asset_import_adapters_v0_1.json";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportAdapterCatalog {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub default_copy_mode: String,
    #[serde(default)]
    pub adapters: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportAdapter {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    pub external_source_id: String,
    pub source_mode: String,
    pub copy_mode: String,
    pub license_gate: PrototypeAssetImportLicenseGate,
    #[serde(default)]
    pub source_inputs: Vec<PrototypeAssetImportSourceInput>,
    #[serde(default)]
    pub processing: serde_json::Value,
    pub output_plan: PrototypeAssetImportOutputPlan,
    pub release_gate: PrototypeAssetImportReleaseGate,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportLicenseGate {
    pub commercial_policy_required: String,
    pub raw_redistribution_allowed: bool,
    #[serde(default)]
    pub attribution_required_in_metadata: bool,
    #[serde(default)]
    pub release_build_requires_explicit_approval: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportSourceInput {
    pub archive_name: String,
    #[serde(default)]
    pub preferred_sheet_hints: Vec<String>,
    #[serde(default)]
    pub grid_size: Option<[u32; 2]>,
    #[serde(default)]
    pub role: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportOutputPlan {
    #[serde(default)]
    pub processed_atlas: String,
    pub metadata: String,
    #[serde(default)]
    pub generated_manifest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportReleaseGate {
    #[serde(default)]
    pub runtime_default: bool,
    #[serde(default)]
    pub public_repo_raw_asset_copy: bool,
    #[serde(default)]
    pub requires_attribution: bool,
    #[serde(default)]
    pub requires_no_standalone_asset_export: bool,
    #[serde(default)]
    pub allowed_build_profiles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrototypeAssetImportAdapterSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrototypeAssetImportAdapterIssue {
    pub severity: PrototypeAssetImportAdapterSeverity,
    pub id: String,
    pub message: String,
}

pub fn load_prototype_asset_import_adapter_catalog(
    repo_root: impl AsRef<Path>,
) -> Result<PrototypeAssetImportAdapterCatalog, String> {
    load_json(
        repo_root
            .as_ref()
            .join(PROTOTYPE_ASSET_IMPORT_ADAPTER_CATALOG_PATH),
    )
}

pub fn load_prototype_asset_import_adapter(
    repo_root: impl AsRef<Path>,
    adapter_path: &str,
) -> Result<PrototypeAssetImportAdapter, String> {
    load_json(repo_root.as_ref().join(adapter_path))
}

pub fn load_prototype_asset_import_adapters(
    repo_root: impl AsRef<Path>,
) -> Result<Vec<PrototypeAssetImportAdapter>, String> {
    let repo_root = repo_root.as_ref();
    let catalog = load_prototype_asset_import_adapter_catalog(repo_root)?;
    catalog
        .adapters
        .iter()
        .map(|path| load_prototype_asset_import_adapter(repo_root, path))
        .collect()
}

pub fn validate_prototype_asset_import_adapters(
    registry: &ExternalAssetSourceRegistry,
    queue: &PrototypeAssetImportQueue,
    catalog: &PrototypeAssetImportAdapterCatalog,
    adapters: &[PrototypeAssetImportAdapter],
) -> Vec<PrototypeAssetImportAdapterIssue> {
    let mut issues = Vec::new();
    if catalog.schema != "havenwild.prototype_asset_import_adapter_catalog.v0_1" {
        issues.push(error(
            "adapter_catalog.schema",
            format!("unsupported adapter catalog schema {}", catalog.schema),
        ));
    }
    if catalog.default_copy_mode != "no_raw_copy" {
        issues.push(error(
            "adapter_catalog.defaultCopyMode",
            "prototype adapters must default to no_raw_copy",
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
        .filter(|source| source.commercial_runtime_policy.contains("blocked"))
        .map(|source| source.id.as_str())
        .collect();
    let queued_source_ids: BTreeSet<&str> = queue
        .entries
        .iter()
        .filter(|entry| !entry.status.contains("blocked"))
        .map(|entry| entry.external_source_id.as_str())
        .collect();

    let mut adapter_ids = BTreeSet::new();
    let mut adapter_paths = BTreeSet::new();
    for path in &catalog.adapters {
        if !path.starts_with("content/assets/prototype_imports/adapters/") {
            issues.push(error(
                path,
                "adapter catalog entries must live under content/assets/prototype_imports/adapters/",
            ));
        }
        if !adapter_paths.insert(path.as_str()) {
            issues.push(error(path, "duplicate adapter catalog path"));
        }
    }

    for adapter in adapters {
        if !adapter_ids.insert(adapter.id.as_str()) {
            issues.push(error(&adapter.id, "duplicate adapter id"));
        }
        if adapter.schema != "havenwild.prototype_asset_import_adapter.v0_1" {
            issues.push(error(
                &adapter.id,
                format!("unsupported adapter schema {}", adapter.schema),
            ));
        }
        if !source_ids.contains(adapter.external_source_id.as_str()) {
            issues.push(error(
                &adapter.id,
                format!(
                    "adapter references unknown externalSourceId {}",
                    adapter.external_source_id
                ),
            ));
        }
        if blocked_source_ids.contains(adapter.external_source_id.as_str()) {
            issues.push(error(
                &adapter.id,
                format!(
                    "adapter cannot target blocked/reference-only source {}",
                    adapter.external_source_id
                ),
            ));
        }
        if !queued_source_ids.contains(adapter.external_source_id.as_str()) {
            issues.push(warning(
                &adapter.id,
                format!(
                    "adapter source {} is not in a non-blocked prototype import queue entry",
                    adapter.external_source_id
                ),
            ));
        }
        if adapter.source_mode != "user_supplied_or_quarantine" {
            issues.push(error(
                &adapter.id,
                "adapter sourceMode must be user_supplied_or_quarantine",
            ));
        }
        if adapter.copy_mode != "no_raw_copy" && adapter.copy_mode != "metadata_stub_only" {
            issues.push(error(
                &adapter.id,
                "adapter copyMode must be no_raw_copy or metadata_stub_only",
            ));
        }
        if adapter.source_inputs.is_empty() {
            issues.push(error(
                &adapter.id,
                "adapter must define at least one source input",
            ));
        }
        for input in &adapter.source_inputs {
            if input.archive_name.trim().is_empty() {
                issues.push(error(&adapter.id, "source input archiveName is empty"));
            }
            if let Some(grid) = input.grid_size {
                if grid[0] == 0 || grid[1] == 0 {
                    issues.push(error(&adapter.id, "source input gridSize must be positive"));
                }
            }
        }
        if adapter.output_plan.metadata.trim().is_empty() {
            issues.push(error(
                &adapter.id,
                "adapter outputPlan.metadata is required",
            ));
        }
        if !adapter.output_plan.metadata.starts_with("content/assets/") {
            issues.push(error(
                &adapter.id,
                format!(
                    "adapter metadata output must stay under content/assets/: {}",
                    adapter.output_plan.metadata
                ),
            ));
        }
        if !adapter.output_plan.processed_atlas.is_empty()
            && !adapter
                .output_plan
                .processed_atlas
                .starts_with("assets/processed/")
        {
            issues.push(error(
                &adapter.id,
                format!(
                    "processed atlas must stay under assets/processed/: {}",
                    adapter.output_plan.processed_atlas
                ),
            ));
        }
        if adapter.release_gate.runtime_default {
            issues.push(error(
                &adapter.id,
                "new third-party prototype adapters must not be runtimeDefault by default",
            ));
        }
        if adapter.release_gate.public_repo_raw_asset_copy {
            issues.push(error(
                &adapter.id,
                "adapter release gate must not allow public repo raw asset copy",
            ));
        }
        if !adapter.release_gate.requires_no_standalone_asset_export {
            issues.push(error(
                &adapter.id,
                "adapter release gate must require no standalone asset export",
            ));
        }
    }

    if adapters.is_empty() {
        issues.push(error(
            "adapters",
            "prototype import adapter catalog has no adapters",
        ));
    }
    issues
}

pub fn prototype_asset_import_adapter_summary(adapters: &[PrototypeAssetImportAdapter]) -> String {
    let sources: BTreeSet<&str> = adapters
        .iter()
        .map(|adapter| adapter.external_source_id.as_str())
        .collect();
    format!(
        "{} prototype asset import adapters across {} external sources",
        adapters.len(),
        sources.len()
    )
}

fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let contents =
        read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn error(id: impl Into<String>, message: impl Into<String>) -> PrototypeAssetImportAdapterIssue {
    PrototypeAssetImportAdapterIssue {
        severity: PrototypeAssetImportAdapterSeverity::Error,
        id: id.into(),
        message: message.into(),
    }
}

fn warning(id: impl Into<String>, message: impl Into<String>) -> PrototypeAssetImportAdapterIssue {
    PrototypeAssetImportAdapterIssue {
        severity: PrototypeAssetImportAdapterSeverity::Warning,
        id: id.into(),
        message: message.into(),
    }
}
