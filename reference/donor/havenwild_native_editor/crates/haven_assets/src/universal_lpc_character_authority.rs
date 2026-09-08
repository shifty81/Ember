use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_UNIVERSAL_LPC_AUTHORITY_PATH: &str =
    "WORKSPACE/generated/universal_lpc_character_authority_v167w.json";
pub const DEFAULT_UNIVERSAL_LPC_SOURCE_MOUNT: &str =
    "assets/source/licensed/universal_lpc_generator/spritesheets";

#[derive(Clone, Debug, Deserialize)]
pub struct UniversalLpcCharacterAuthority {
    pub schema: String,
    pub source_commit: String,
    pub sex_values: Vec<String>,
    pub age_groups: Vec<String>,
    pub credit_record_count: usize,
    #[serde(default)]
    pub license_tier_counts: HashMap<String, usize>,
    #[serde(default)]
    pub category_counts: HashMap<String, usize>,
    #[serde(default)]
    pub records: Vec<UniversalLpcCharacterRecord>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UniversalLpcCharacterRecord {
    pub source: String,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub licenses: Vec<String>,
    pub selected_license: Option<String>,
    pub license_tier: String,
    #[serde(default)]
    pub share_alike_required: bool,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

impl UniversalLpcCharacterAuthority {
    pub fn load_default() -> Result<Self, String> {
        Self::load_from_path(DEFAULT_UNIVERSAL_LPC_AUTHORITY_PATH)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let authority: Self = serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        authority.validate()?;
        Ok(authority)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "havenwild.universal_lpc_character_authority.v167w" {
            return Err(format!(
                "unsupported Universal LPC authority schema: {}",
                self.schema
            ));
        }
        if self.sex_values.len() != 2
            || self.sex_values.first().map(String::as_str) != Some("Male")
            || self.sex_values.get(1).map(String::as_str) != Some("Female")
        {
            return Err("Universal LPC authority must expose exactly Male and Female".to_string());
        }
        for required in ["Child", "Teen", "Adult", "Elder"] {
            if !self.age_groups.iter().any(|value| value == required) {
                return Err(format!(
                    "Universal LPC authority is missing age group {required}"
                ));
            }
        }
        if self.credit_record_count != self.records.len() {
            return Err(format!(
                "Universal LPC authority record count mismatch: declared {}, loaded {}",
                self.credit_record_count,
                self.records.len()
            ));
        }
        Ok(())
    }

    pub fn source_path(&self, record: &UniversalLpcCharacterRecord) -> PathBuf {
        Path::new(DEFAULT_UNIVERSAL_LPC_SOURCE_MOUNT).join(&record.source)
    }

    pub fn preferred_count(&self) -> usize {
        self.license_tier_counts
            .get("preferred")
            .copied()
            .unwrap_or(0)
    }

    pub fn conditional_count(&self) -> usize {
        self.license_tier_counts
            .get("conditional")
            .copied()
            .unwrap_or(0)
    }
}

impl UniversalLpcCharacterRecord {
    pub fn display_name(&self) -> &str {
        self.source.rsplit('/').next().unwrap_or(&self.source)
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags
            .iter()
            .any(|value| value.eq_ignore_ascii_case(tag))
    }

    pub fn is_selectable(&self, allow_share_alike: bool) -> bool {
        self.license_tier == "preferred"
            || (allow_share_alike && self.license_tier == "conditional")
    }

    pub fn source_matches(&self, query: &str) -> bool {
        if query.trim().is_empty() {
            return true;
        }
        let query = query.trim().to_ascii_lowercase();
        self.source.to_ascii_lowercase().contains(&query)
            || self.category.to_ascii_lowercase().contains(&query)
            || self
                .tags
                .iter()
                .any(|value| value.to_ascii_lowercase().contains(&query))
            || self
                .authors
                .iter()
                .any(|value| value.to_ascii_lowercase().contains(&query))
    }
}
