use crate::asset_pack::{AssetCategory, AssetId, AssetPackId, AssetSourceId, StableAssetRef};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LpcMappingQueueItem {
    pub queue: String,
    pub relative_path: String,
    pub category: AssetCategory,
    pub source_kind: String,
    pub proposed_asset_id: String,
    pub proposed_semantic_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub license_evidence: Vec<String>,
    pub readiness: String,
    pub priority: i32,
    pub promotion_state: String,
    pub recommended_action: String,
}

impl LpcMappingQueueItem {
    pub fn stable_ref(&self) -> StableAssetRef {
        StableAssetRef {
            pack_id: AssetPackId("lpc_revised".to_string()),
            category: self.category.clone(),
            asset_id: AssetId(self.proposed_asset_id.clone()),
            source_id: AssetSourceId("external_root".to_string()),
            variant_id: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LpcMappingQueues {
    pub schema: String,
    #[serde(default)]
    pub queue_order: Vec<String>,
    #[serde(default)]
    pub queue_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub queues: BTreeMap<String, Vec<LpcMappingQueueItem>>,
    #[serde(default)]
    pub promotion_states: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

impl LpcMappingQueues {
    pub const SCHEMA: &'static str = "havenwild.lpc_revised_mapping_queues.v1";

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let queues: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if queues.schema != Self::SCHEMA {
            return Err(format!(
                "unsupported LPC mapping queue schema {}",
                queues.schema
            ));
        }
        Ok(queues)
    }

    pub fn items(&self) -> impl Iterator<Item = &LpcMappingQueueItem> {
        self.queue_order
            .iter()
            .flat_map(|queue| self.queues.get(queue).into_iter().flatten())
    }
}
