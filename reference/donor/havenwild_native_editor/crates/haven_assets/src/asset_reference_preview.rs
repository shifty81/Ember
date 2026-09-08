use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, path::Path};

pub const ASSET_REFERENCE_PREVIEW_CATALOG_PATH: &str =
    "content/assets/reference_previews/asset_reference_preview_catalog_v0_1.json";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetReferencePreviewCatalog {
    pub schema: String,
    pub updated: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub rules: serde_json::Value,
    #[serde(default)]
    pub records: Vec<AssetReferencePreviewRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetReferencePreviewRecord {
    pub external_source_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub preview_status: String,
    #[serde(default)]
    pub preview_kind: String,
    #[serde(default)]
    pub safe_preview_path: String,
    #[serde(default)]
    pub thumbnail_path: String,
    #[serde(default)]
    pub contact_sheet_path: String,
    #[serde(default)]
    pub generated_by: String,
    #[serde(default)]
    pub source_policy: String,
    #[serde(default)]
    pub editor_display: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl AssetReferencePreviewRecord {
    pub fn preview_path_label(&self) -> &str {
        if !self.contact_sheet_path.is_empty() {
            &self.contact_sheet_path
        } else if !self.safe_preview_path.is_empty() {
            &self.safe_preview_path
        } else if !self.thumbnail_path.is_empty() {
            &self.thumbnail_path
        } else {
            "WORKSPACE/generated/asset_reference_previews/"
        }
    }

    pub fn availability_label(&self, repo_root: impl AsRef<Path>) -> &'static str {
        if self.preview_path_exists(repo_root) {
            "preview ready"
        } else {
            "preview missing"
        }
    }

    pub fn preview_path_exists(&self, repo_root: impl AsRef<Path>) -> bool {
        let path = self.preview_path_label();
        !path.is_empty() && repo_root.as_ref().join(path).exists()
    }

    pub fn is_reference_only(&self) -> bool {
        let policy = self.source_policy.to_ascii_lowercase();
        policy.contains("reference_only")
            || policy.contains("noncommercial")
            || policy.contains("unverified")
    }
}

pub fn load_asset_reference_preview_catalog(
    repo_root: impl AsRef<Path>,
) -> Result<AssetReferencePreviewCatalog, String> {
    let path = repo_root
        .as_ref()
        .join(ASSET_REFERENCE_PREVIEW_CATALOG_PATH);
    let text = read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

pub fn build_asset_reference_preview_index(
    catalog: &AssetReferencePreviewCatalog,
) -> BTreeMap<String, AssetReferencePreviewRecord> {
    catalog
        .records
        .iter()
        .map(|record| (record.external_source_id.clone(), record.clone()))
        .collect()
}

pub fn asset_reference_preview_for_source(
    external_source_id: &str,
    display_name: &str,
    index: &BTreeMap<String, AssetReferencePreviewRecord>,
) -> AssetReferencePreviewRecord {
    index
        .get(external_source_id)
        .cloned()
        .unwrap_or_else(|| AssetReferencePreviewRecord {
            external_source_id: external_source_id.to_string(),
            display_name: display_name.to_string(),
            preview_status: "preview_catalog_missing_record".to_string(),
            preview_kind: "none".to_string(),
            safe_preview_path: "WORKSPACE/generated/asset_reference_previews/".to_string(),
            thumbnail_path: String::new(),
            contact_sheet_path: String::new(),
            generated_by: "no preview metadata record found".to_string(),
            source_policy: "reference_only_until_preview_metadata_exists".to_string(),
            editor_display: "show_missing_preview_metadata".to_string(),
            notes: vec!["Add this source to the asset reference preview catalog before previewing it in the editor.".to_string()],
        })
}

pub fn asset_reference_preview_summary_line(
    catalog: &AssetReferencePreviewCatalog,
    repo_root: impl AsRef<Path>,
) -> String {
    let ready = catalog
        .records
        .iter()
        .filter(|record| record.preview_path_exists(repo_root.as_ref()))
        .count();
    let reference_only = catalog
        .records
        .iter()
        .filter(|record| record.is_reference_only())
        .count();
    format!(
        "asset previews: {} record(s) | {} local preview(s) ready | {} reference-only/restricted",
        catalog.records.len(),
        ready,
        reference_only
    )
}
