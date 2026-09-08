use serde::Deserialize;
use std::{collections::BTreeSet, fs::read_to_string, path::Path};

pub const EXTERNAL_ASSET_SOURCES_PATH: &str =
    "content/assets/external_sources/external_asset_sources_v0_1.json";
pub const PROTOTYPE_ASSET_IMPORT_QUEUE_PATH: &str =
    "content/assets/prototype_imports/prototype_asset_import_queue_v0_1.json";

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ExternalAssetSourceRegistry {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub sources: Vec<ExternalAssetSource>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAssetSource {
    pub id: String,
    pub display_name: String,
    pub author: String,
    pub official_source_url: String,
    #[serde(default)]
    pub uploaded_archives: Vec<String>,
    pub license_status: String,
    pub commercial_runtime_policy: String,
    pub public_repo_policy: String,
    pub raw_asset_policy: String,
    pub raw_quarantine_path: String,
    #[serde(default)]
    pub processed_prototype_path: Option<String>,
    #[serde(default)]
    pub candidate_families: Vec<String>,
    pub recommended_use: String,
    pub prohibited_use: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportQueue {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub default_import_mode: String,
    #[serde(default)]
    pub entries: Vec<PrototypeAssetImportEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetImportEntry {
    pub id: String,
    pub external_source_id: String,
    pub status: String,
    #[serde(default)]
    pub source_archive_names: Vec<String>,
    #[serde(default)]
    pub candidate_families: Vec<String>,
    #[serde(default)]
    pub target_outputs: Vec<String>,
    #[serde(default)]
    pub runtime_use: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExternalAssetIntakeSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalAssetIntakeIssue {
    pub severity: ExternalAssetIntakeSeverity,
    pub id: String,
    pub message: String,
}

pub fn load_external_asset_source_registry(
    repo_root: impl AsRef<Path>,
) -> Result<ExternalAssetSourceRegistry, String> {
    load_json(repo_root.as_ref().join(EXTERNAL_ASSET_SOURCES_PATH))
}

pub fn load_prototype_asset_import_queue(
    repo_root: impl AsRef<Path>,
) -> Result<PrototypeAssetImportQueue, String> {
    load_json(repo_root.as_ref().join(PROTOTYPE_ASSET_IMPORT_QUEUE_PATH))
}

pub fn validate_external_asset_intake(
    registry: &ExternalAssetSourceRegistry,
    queue: &PrototypeAssetImportQueue,
) -> Vec<ExternalAssetIntakeIssue> {
    let mut issues = Vec::new();
    if registry.schema != "havenwild.external_asset_sources.v0_1" {
        issues.push(error(
            "registry.schema",
            format!(
                "unsupported external asset registry schema {}",
                registry.schema
            ),
        ));
    }
    if queue.schema != "havenwild.prototype_asset_import_queue.v0_1" {
        issues.push(error(
            "queue.schema",
            format!("unsupported prototype import queue schema {}", queue.schema),
        ));
    }

    let mut source_ids = BTreeSet::new();
    for source in &registry.sources {
        if !source_ids.insert(source.id.clone()) {
            issues.push(error(&source.id, "duplicate external asset source id"));
        }
        if !source
            .raw_quarantine_path
            .starts_with("assets/reference_quarantine/third_party/")
        {
            issues.push(error(
                &source.id,
                format!(
                    "rawQuarantinePath must stay under assets/reference_quarantine/third_party/: {}",
                    source.raw_quarantine_path
                ),
            ));
        }
        if source.uploaded_archives.is_empty() {
            issues.push(warning(
                &source.id,
                "source has no uploadedArchives recorded",
            ));
        }
        if source.license_status.contains("non_commercial")
            && !source.commercial_runtime_policy.contains("blocked")
        {
            issues.push(error(
                &source.id,
                "non-commercial source must have a blocked commercialRuntimePolicy",
            ));
        }
        if source.license_status.contains("blocked")
            && !source.commercial_runtime_policy.contains("blocked")
        {
            issues.push(error(
                &source.id,
                "blocked source must have a blocked commercialRuntimePolicy",
            ));
        }
        if source.commercial_runtime_policy.contains("allowed")
            && source.processed_prototype_path.is_none()
        {
            issues.push(warning(
                &source.id,
                "commercial/prototype-allowed source has no processedPrototypePath",
            ));
        }
        if source.public_repo_policy.is_empty() || source.raw_asset_policy.is_empty() {
            issues.push(error(
                &source.id,
                "publicRepoPolicy and rawAssetPolicy must be explicit",
            ));
        }
    }

    let mut queue_ids = BTreeSet::new();
    for entry in &queue.entries {
        if !queue_ids.insert(entry.id.clone()) {
            issues.push(error(&entry.id, "duplicate prototype import entry id"));
        }
        if !source_ids.contains(&entry.external_source_id) {
            issues.push(error(
                &entry.id,
                format!(
                    "prototype import entry references missing externalSourceId {}",
                    entry.external_source_id
                ),
            ));
        }
        if entry.status.contains("blocked") && !entry.target_outputs.is_empty() {
            issues.push(error(
                &entry.id,
                "blocked prototype import entry must not declare targetOutputs",
            ));
        }
        for output in &entry.target_outputs {
            if !(output.starts_with("assets/processed/") || output.starts_with("content/assets/")) {
                issues.push(error(
                    &entry.id,
                    format!("target output must stay in processed asset metadata area: {output}"),
                ));
            }
        }
    }

    if registry.sources.is_empty() {
        issues.push(error(
            "registry.sources",
            "external asset source registry is empty",
        ));
    }
    if queue.entries.is_empty() {
        issues.push(warning(
            "queue.entries",
            "prototype import queue has no candidate entries",
        ));
    }
    issues
}

pub fn external_asset_intake_summary(
    registry: &ExternalAssetSourceRegistry,
    queue: &PrototypeAssetImportQueue,
) -> String {
    let allowed = registry
        .sources
        .iter()
        .filter(|source| source.commercial_runtime_policy.contains("allowed"))
        .count();
    let blocked = registry
        .sources
        .iter()
        .filter(|source| source.commercial_runtime_policy.contains("blocked"))
        .count();
    format!(
        "{} external sources ({} allowed candidates, {} blocked/reference-only), {} prototype queue entries",
        registry.sources.len(),
        allowed,
        blocked,
        queue.entries.len()
    )
}

fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let contents =
        read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn error(id: impl Into<String>, message: impl Into<String>) -> ExternalAssetIntakeIssue {
    ExternalAssetIntakeIssue {
        severity: ExternalAssetIntakeSeverity::Error,
        id: id.into(),
        message: message.into(),
    }
}

fn warning(id: impl Into<String>, message: impl Into<String>) -> ExternalAssetIntakeIssue {
    ExternalAssetIntakeIssue {
        severity: ExternalAssetIntakeSeverity::Warning,
        id: id.into(),
        message: message.into(),
    }
}
