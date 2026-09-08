use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, path::Path};

pub const ASSET_LOCAL_IMPORT_INSTRUCTIONS_PATH: &str =
    "content/assets/prototype_imports/prototype_asset_local_import_instructions_v0_1.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetLocalImportInstructionCatalog {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub rules: serde_json::Value,
    #[serde(default)]
    pub records: Vec<AssetLocalImportInstructionRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetLocalImportInstructionRecord {
    pub external_source_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub local_import_status: String,
    #[serde(default)]
    pub preferred_workspace_import_path: String,
    #[serde(default)]
    pub preferred_quarantine_path: String,
    #[serde(default)]
    pub accepted_input_files: Vec<String>,
    #[serde(default)]
    pub accepted_input_notes: Vec<String>,
    #[serde(default)]
    pub dry_run_command: String,
    #[serde(default)]
    pub editor_display: String,
    #[serde(default)]
    pub runtime_use_rule: String,
    #[serde(default)]
    pub instruction: String,
}

impl AssetLocalImportInstructionRecord {
    pub fn primary_input_label(&self) -> String {
        if self.accepted_input_files.is_empty() {
            "no exact filenames recorded".to_string()
        } else {
            self.accepted_input_files.join(" | ")
        }
    }

    pub fn preferred_path_label(&self) -> String {
        if !self.preferred_workspace_import_path.is_empty() {
            self.preferred_workspace_import_path.clone()
        } else if !self.preferred_quarantine_path.is_empty() {
            self.preferred_quarantine_path.clone()
        } else {
            "no local import path recorded".to_string()
        }
    }

    pub fn short_status_label(&self) -> &'static str {
        let status = self.local_import_status.to_ascii_lowercase();
        if status.contains("blocked") || status.contains("unverified") {
            "blocked/reference"
        } else if status.contains("candidate") {
            "candidate"
        } else {
            "reference"
        }
    }
}

pub fn load_asset_local_import_instructions(
    repo_root: impl AsRef<Path>,
) -> Result<AssetLocalImportInstructionCatalog, String> {
    let path = repo_root
        .as_ref()
        .join(ASSET_LOCAL_IMPORT_INSTRUCTIONS_PATH);
    let contents =
        read_to_string(&path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn build_asset_local_import_instruction_index(
    catalog: &AssetLocalImportInstructionCatalog,
) -> BTreeMap<String, AssetLocalImportInstructionRecord> {
    catalog
        .records
        .iter()
        .cloned()
        .map(|record| (record.external_source_id.clone(), record))
        .collect()
}

pub fn asset_local_import_instruction_for_source(
    external_source_id: &str,
    index: &BTreeMap<String, AssetLocalImportInstructionRecord>,
) -> AssetLocalImportInstructionRecord {
    index
        .get(external_source_id)
        .cloned()
        .unwrap_or_else(|| AssetLocalImportInstructionRecord {
            external_source_id: external_source_id.to_string(),
            display_name: external_source_id.to_string(),
            local_import_status: "missing_instruction_record".to_string(),
            preferred_workspace_import_path: "WORKSPACE/imports/third_party/".to_string(),
            preferred_quarantine_path: "assets/reference_quarantine/third_party/".to_string(),
            accepted_input_files: Vec::new(),
            accepted_input_notes: vec![
                "No per-source instruction record exists yet; keep this source quarantined/reference-only.".to_string(),
            ],
            dry_run_command: "python tools/automation/assets/DryRun-PrototypeAssetBakeV29.py".to_string(),
            editor_display: "show_missing_instruction_warning".to_string(),
            runtime_use_rule: "reference_only_until_instruction_record_exists".to_string(),
            instruction: "Add an asset-local-import instruction record before dry-run or bake.".to_string(),
        })
}

pub fn asset_local_import_instruction_summary_line(
    catalog: &AssetLocalImportInstructionCatalog,
) -> String {
    let candidates = catalog
        .records
        .iter()
        .filter(|record| record.local_import_status.contains("candidate"))
        .count();
    let blocked = catalog
        .records
        .iter()
        .filter(|record| {
            let status = record.local_import_status.to_ascii_lowercase();
            status.contains("blocked") || status.contains("unverified")
        })
        .count();
    format!(
        "local import instructions: {} source(s) | {} candidate | {} blocked/reference-gated",
        catalog.records.len(),
        candidates,
        blocked
    )
}
