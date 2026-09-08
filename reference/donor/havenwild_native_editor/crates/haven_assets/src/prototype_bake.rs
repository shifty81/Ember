use crate::{
    external_sources::{ExternalAssetSourceRegistry, PrototypeAssetImportQueue},
    prototype_import_adapter::{
        PrototypeAssetImportAdapter, PrototypeAssetImportAdapterCatalog,
        PROTOTYPE_ASSET_IMPORT_ADAPTER_CATALOG_PATH,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, path::Path};

pub const PROTOTYPE_ASSET_LOCAL_BAKE_PLAN_PATH: &str =
    "content/assets/prototype_imports/prototype_asset_local_bake_plan_v0_1.json";
pub const PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH: &str =
    "WORKSPACE/generated/prototype_imports/prototype_asset_bake_dry_run_report_v0_1.json";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetLocalBakePlan {
    pub schema: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub dry_run_only: bool,
    #[serde(default)]
    pub copy_raw_assets: bool,
    #[serde(default)]
    pub generate_runtime_assets: bool,
    pub report_path: String,
    #[serde(default)]
    pub source_lookup_roots: Vec<String>,
    #[serde(default)]
    pub blocked_policy_tokens: Vec<String>,
    #[serde(default)]
    pub entries: Vec<PrototypeAssetLocalBakePlanEntry>,
    #[serde(default)]
    pub blocked_source_probe_policy: serde_json::Value,
    #[serde(default)]
    pub safety_notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetLocalBakePlanEntry {
    pub id: String,
    pub adapter_id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub expected_source_state: String,
    #[serde(default)]
    pub allowed_output_mode: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrototypeAssetBakeStatus {
    Ready,
    MissingSource,
    RefusedBlockedSource,
    PlanError,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetBakeDryRunInputStatus {
    pub archive_name: String,
    pub role: String,
    pub found: bool,
    pub checked_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetBakeDryRunEntry {
    pub id: String,
    pub adapter_id: String,
    pub external_source_id: String,
    pub status: PrototypeAssetBakeStatus,
    pub inputs: Vec<PrototypeAssetBakeDryRunInputStatus>,
    pub output_metadata: String,
    pub output_processed_atlas: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetBakeDryRunBlockedProbe {
    pub source_id: String,
    pub policy: String,
    pub status: PrototypeAssetBakeStatus,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrototypeAssetBakeDryRunReport {
    pub schema: String,
    pub dry_run_only: bool,
    pub copy_raw_assets: bool,
    pub generate_runtime_assets: bool,
    pub adapter_catalog_path: String,
    pub report_path: String,
    pub entries: Vec<PrototypeAssetBakeDryRunEntry>,
    pub blocked_source_probes: Vec<PrototypeAssetBakeDryRunBlockedProbe>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrototypeAssetBakePlanSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrototypeAssetBakePlanIssue {
    pub severity: PrototypeAssetBakePlanSeverity,
    pub id: String,
    pub message: String,
}

pub fn load_prototype_asset_local_bake_plan(
    repo_root: impl AsRef<Path>,
) -> Result<PrototypeAssetLocalBakePlan, String> {
    load_json(
        repo_root
            .as_ref()
            .join(PROTOTYPE_ASSET_LOCAL_BAKE_PLAN_PATH),
    )
}

pub fn validate_prototype_asset_local_bake_plan(
    plan: &PrototypeAssetLocalBakePlan,
    adapter_catalog: &PrototypeAssetImportAdapterCatalog,
    adapters: &[PrototypeAssetImportAdapter],
) -> Vec<PrototypeAssetBakePlanIssue> {
    let mut issues = Vec::new();
    if plan.schema != "havenwild.prototype_asset_local_bake_plan.v0_1" {
        issues.push(error(
            "plan.schema",
            format!("unsupported bake plan schema {}", plan.schema),
        ));
    }
    if plan.enabled_by_default {
        issues.push(error(
            "plan.enabledByDefault",
            "prototype asset bake must remain disabled by default",
        ));
    }
    if !plan.dry_run_only {
        issues.push(error(
            "plan.dryRunOnly",
            "pass 23 bake path must remain dry-run only",
        ));
    }
    if plan.copy_raw_assets {
        issues.push(error(
            "plan.copyRawAssets",
            "dry-run bake must not copy raw third-party assets",
        ));
    }
    if plan.generate_runtime_assets {
        issues.push(error(
            "plan.generateRuntimeAssets",
            "dry-run bake must not generate runtime assets",
        ));
    }
    if plan.report_path != PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH {
        issues.push(error(
            "plan.reportPath",
            format!(
                "reportPath must be {}, found {}",
                PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH, plan.report_path
            ),
        ));
    }
    if adapter_catalog.schema != "havenwild.prototype_asset_import_adapter_catalog.v0_1" {
        issues.push(error(
            "adapterCatalog.schema",
            format!(
                "unsupported adapter catalog schema {}",
                adapter_catalog.schema
            ),
        ));
    }

    let adapters_by_id: BTreeMap<&str, &PrototypeAssetImportAdapter> = adapters
        .iter()
        .map(|adapter| (adapter.id.as_str(), adapter))
        .collect();
    for entry in &plan.entries {
        if !adapters_by_id.contains_key(entry.adapter_id.as_str()) {
            issues.push(error(
                &entry.id,
                format!(
                    "bake plan references missing adapterId {}",
                    entry.adapter_id
                ),
            ));
        }
        if entry.allowed_output_mode != "report_only_no_runtime_assets" {
            issues.push(error(
                &entry.id,
                "bake plan entries must stay report_only_no_runtime_assets in this pass",
            ));
        }
    }
    if plan.entries.is_empty() {
        issues.push(error(
            "plan.entries",
            "bake plan must include at least one dry-run entry",
        ));
    }
    issues
}

pub fn dry_run_prototype_asset_bake(
    repo_root: impl AsRef<Path>,
    registry: &ExternalAssetSourceRegistry,
    queue: &PrototypeAssetImportQueue,
    plan: &PrototypeAssetLocalBakePlan,
    adapters: &[PrototypeAssetImportAdapter],
) -> PrototypeAssetBakeDryRunReport {
    let repo_root = repo_root.as_ref();
    let adapters_by_id: BTreeMap<&str, &PrototypeAssetImportAdapter> = adapters
        .iter()
        .map(|adapter| (adapter.id.as_str(), adapter))
        .collect();
    let sources_by_id: BTreeMap<&str, _> = registry
        .sources
        .iter()
        .map(|source| (source.id.as_str(), source))
        .collect();
    let mut entries = Vec::new();

    for plan_entry in &plan.entries {
        let Some(adapter) = adapters_by_id.get(plan_entry.adapter_id.as_str()) else {
            entries.push(PrototypeAssetBakeDryRunEntry {
                id: plan_entry.id.clone(),
                adapter_id: plan_entry.adapter_id.clone(),
                external_source_id: String::new(),
                status: PrototypeAssetBakeStatus::PlanError,
                inputs: Vec::new(),
                output_metadata: String::new(),
                output_processed_atlas: String::new(),
                notes: vec!["missing adapter; no bake attempted".to_string()],
            });
            continue;
        };
        let Some(source) = sources_by_id.get(adapter.external_source_id.as_str()) else {
            entries.push(PrototypeAssetBakeDryRunEntry {
                id: plan_entry.id.clone(),
                adapter_id: adapter.id.clone(),
                external_source_id: adapter.external_source_id.clone(),
                status: PrototypeAssetBakeStatus::PlanError,
                inputs: Vec::new(),
                output_metadata: adapter.output_plan.metadata.clone(),
                output_processed_atlas: adapter.output_plan.processed_atlas.clone(),
                notes: vec!["missing external source; no bake attempted".to_string()],
            });
            continue;
        };
        if is_blocked_policy(
            &source.commercial_runtime_policy,
            &source.license_status,
            plan,
        ) {
            entries.push(PrototypeAssetBakeDryRunEntry {
                id: plan_entry.id.clone(),
                adapter_id: adapter.id.clone(),
                external_source_id: adapter.external_source_id.clone(),
                status: PrototypeAssetBakeStatus::RefusedBlockedSource,
                inputs: Vec::new(),
                output_metadata: adapter.output_plan.metadata.clone(),
                output_processed_atlas: adapter.output_plan.processed_atlas.clone(),
                notes: vec!["blocked source refused before checking inputs".to_string()],
            });
            continue;
        }

        let mut all_found = true;
        let mut input_statuses = Vec::new();
        for input in &adapter.source_inputs {
            let checked_paths = candidate_source_paths(
                source.id.as_str(),
                source.raw_quarantine_path.as_str(),
                &input.archive_name,
            );
            let found = checked_paths
                .iter()
                .any(|candidate| repo_root.join(candidate).exists());
            all_found &= found;
            input_statuses.push(PrototypeAssetBakeDryRunInputStatus {
                archive_name: input.archive_name.clone(),
                role: input.role.clone(),
                found,
                checked_paths,
            });
        }
        let status = if all_found {
            PrototypeAssetBakeStatus::Ready
        } else {
            PrototypeAssetBakeStatus::MissingSource
        };
        entries.push(PrototypeAssetBakeDryRunEntry {
            id: plan_entry.id.clone(),
            adapter_id: adapter.id.clone(),
            external_source_id: adapter.external_source_id.clone(),
            status,
            inputs: input_statuses,
            output_metadata: adapter.output_plan.metadata.clone(),
            output_processed_atlas: adapter.output_plan.processed_atlas.clone(),
            notes: vec![
                "dry-run only; no raw assets copied and no runtime atlas generated".to_string(),
            ],
        });
    }

    let mut blocked_source_probes = Vec::new();
    for queue_entry in &queue.entries {
        if !queue_entry.status.contains("blocked") {
            continue;
        }
        if let Some(source) = sources_by_id.get(queue_entry.external_source_id.as_str()) {
            blocked_source_probes.push(PrototypeAssetBakeDryRunBlockedProbe {
                source_id: source.id.clone(),
                policy: source.commercial_runtime_policy.clone(),
                status: PrototypeAssetBakeStatus::RefusedBlockedSource,
                reason: "blocked prototype import queue entry refused by dry-run bake gate"
                    .to_string(),
            });
        }
    }

    let ready = entries
        .iter()
        .filter(|entry| entry.status == PrototypeAssetBakeStatus::Ready)
        .count();
    let missing = entries
        .iter()
        .filter(|entry| entry.status == PrototypeAssetBakeStatus::MissingSource)
        .count();
    let refused = blocked_source_probes.len()
        + entries
            .iter()
            .filter(|entry| entry.status == PrototypeAssetBakeStatus::RefusedBlockedSource)
            .count();
    PrototypeAssetBakeDryRunReport {
        schema: "havenwild.prototype_asset_bake_dry_run_report.v0_1".to_string(),
        dry_run_only: true,
        copy_raw_assets: false,
        generate_runtime_assets: false,
        adapter_catalog_path: PROTOTYPE_ASSET_IMPORT_ADAPTER_CATALOG_PATH.to_string(),
        report_path: plan.report_path.clone(),
        entries,
        blocked_source_probes,
        summary: format!("ready={ready}; missing_source={missing}; refused_blocked={refused}"),
    }
}

fn candidate_source_paths(
    source_id: &str,
    quarantine_path: &str,
    archive_name: &str,
) -> Vec<String> {
    let slug = source_slug(source_id);
    vec![
        format!("WORKSPACE/imports/third_party/{slug}/{archive_name}"),
        format!("{}{}", quarantine_path, archive_name),
        format!("WORKSPACE/imports/{archive_name}"),
        format!("WORKSPACE/imports/third_party/{archive_name}"),
    ]
}

fn source_slug(source_id: &str) -> String {
    source_id
        .strip_prefix("third_party.")
        .unwrap_or(source_id)
        .replace(['.', '-'], "_")
}

fn is_blocked_policy(
    policy: &str,
    license_status: &str,
    plan: &PrototypeAssetLocalBakePlan,
) -> bool {
    let policy = policy.to_ascii_lowercase();
    let license_status = license_status.to_ascii_lowercase();
    plan.blocked_policy_tokens.iter().any(|token| {
        let token = token.to_ascii_lowercase();
        policy.contains(&token) || license_status.contains(&token)
    })
}

fn load_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let path = path.as_ref();
    let contents =
        read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn error(id: impl Into<String>, message: impl Into<String>) -> PrototypeAssetBakePlanIssue {
    PrototypeAssetBakePlanIssue {
        severity: PrototypeAssetBakePlanSeverity::Error,
        id: id.into(),
        message: message.into(),
    }
}
