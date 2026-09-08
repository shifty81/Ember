use crate::asset_pack::{AssetCategory, AssetPackId, AssetPackRegistry, StableAssetRef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetResolutionContext {
    pub biome: Option<String>,
    pub season: Option<String>,
    pub preferred_packs: Vec<AssetPackId>,
    pub required_tags: BTreeSet<String>,
    pub allow_reference_only: bool,
    pub explicit_pack: Option<AssetPackId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticAssetRequest {
    pub semantic_id: String,
    pub category: AssetCategory,
    #[serde(default)]
    pub context: AssetResolutionContext,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetResolutionCandidate {
    pub stable_ref: StableAssetRef,
    pub score: i64,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetResolutionResult {
    pub selected: Option<StableAssetRef>,
    pub candidates: Vec<AssetResolutionCandidate>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticAssetResolutionPolicy {
    #[serde(default)]
    pub pack_priority_bonus: BTreeMap<AssetPackId, i64>,
    #[serde(default = "default_preferred_pack_bonus")]
    pub preferred_pack_bonus: i64,
    #[serde(default = "default_explicit_pack_bonus")]
    pub explicit_pack_bonus: i64,
    #[serde(default = "default_tag_bonus")]
    pub tag_bonus: i64,
}

const fn default_preferred_pack_bonus() -> i64 {
    10_000
}
const fn default_explicit_pack_bonus() -> i64 {
    1_000_000
}
const fn default_tag_bonus() -> i64 {
    100
}

impl Default for SemanticAssetResolutionPolicy {
    fn default() -> Self {
        Self {
            pack_priority_bonus: BTreeMap::new(),
            preferred_pack_bonus: default_preferred_pack_bonus(),
            explicit_pack_bonus: default_explicit_pack_bonus(),
            tag_bonus: default_tag_bonus(),
        }
    }
}

pub struct SemanticAssetResolver<'a> {
    registry: &'a AssetPackRegistry,
    policy: SemanticAssetResolutionPolicy,
}

impl<'a> SemanticAssetResolver<'a> {
    pub fn new(registry: &'a AssetPackRegistry) -> Self {
        Self {
            registry,
            policy: SemanticAssetResolutionPolicy::default(),
        }
    }

    pub fn with_policy(
        registry: &'a AssetPackRegistry,
        policy: SemanticAssetResolutionPolicy,
    ) -> Self {
        Self { registry, policy }
    }

    pub fn resolve(&self, request: &SemanticAssetRequest) -> AssetResolutionResult {
        let mut result = AssetResolutionResult::default();
        for stable_ref in self
            .registry
            .resolve_semantic(&request.semantic_id, request.category.clone())
        {
            let Some(pack) = self.registry.pack(&stable_ref.pack_id) else {
                continue;
            };
            if !request.context.allow_reference_only && !pack.production_enabled {
                continue;
            }
            let Some(asset) = self.registry.asset(stable_ref) else {
                continue;
            };
            if !request
                .context
                .required_tags
                .iter()
                .all(|tag| asset.tags.contains(tag))
            {
                continue;
            }

            let mut score = i64::from(pack.priority);
            let mut reasons = vec![format!("pack priority {}", pack.priority)];
            score += self
                .policy
                .pack_priority_bonus
                .get(&pack.id)
                .copied()
                .unwrap_or_default();
            if request.context.preferred_packs.contains(&pack.id) {
                score += self.policy.preferred_pack_bonus;
                reasons.push("preferred pack".to_string());
            }
            if request.context.explicit_pack.as_ref() == Some(&pack.id) {
                score += self.policy.explicit_pack_bonus;
                reasons.push("explicit pack".to_string());
            }
            let matched_tags = request
                .context
                .required_tags
                .intersection(&asset.tags)
                .count() as i64;
            score += matched_tags * self.policy.tag_bonus;
            result.candidates.push(AssetResolutionCandidate {
                stable_ref: (*stable_ref).clone(),
                score,
                reasons,
            });
        }
        result.candidates.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.stable_ref.pack_id.cmp(&right.stable_ref.pack_id))
                .then_with(|| left.stable_ref.asset_id.cmp(&right.stable_ref.asset_id))
        });
        result.selected = result
            .candidates
            .first()
            .map(|candidate| candidate.stable_ref.clone());
        if result.selected.is_none() {
            result.diagnostics.push(format!(
                "no production-compatible provider for {} in {:?}",
                request.semantic_id, request.category
            ));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_pack::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn pack(id: &str, priority: i32, enabled: bool) -> AssetPackManifest {
        AssetPackManifest {
            schema: AssetPackManifest::SCHEMA.to_string(),
            id: AssetPackId(id.into()),
            display_name: id.into(),
            version: "1".into(),
            priority,
            production_enabled: enabled,
            license: LicenseRecord {
                license_id: "test".into(),
                production_approved: enabled,
                attribution: vec![],
                commercial_use: enabled,
                redistribution: enabled,
                source_url: None,
            },
            dependencies: vec![],
            sources: vec![AssetSource {
                id: AssetSourceId("sheet".into()),
                kind: AssetSourceKind::RawSheet,
                path: "sheet.png".into(),
                tile_size: Some([32, 32]),
                margin: 0,
                spacing: 0,
            }],
            assets: vec![AssetDefinition {
                id: AssetId("grass".into()),
                category: AssetCategory::Terrain,
                semantic_id: "terrain.grass".into(),
                source_id: AssetSourceId("sheet".into()),
                atlas_region: None,
                variants: vec![],
                tags: BTreeSet::from(["temperate".into()]),
                metadata: BTreeMap::new(),
            }],
        }
    }

    #[test]
    fn production_resolution_excludes_reference_only_packs() {
        let mut registry = AssetPackRegistry::default();
        registry.mount(pack("core", 10, true)).unwrap();
        registry.mount(pack("reference", 100, false)).unwrap();
        let result = SemanticAssetResolver::new(&registry).resolve(&SemanticAssetRequest {
            semantic_id: "terrain.grass".into(),
            category: AssetCategory::Terrain,
            context: AssetResolutionContext::default(),
        });
        assert_eq!(result.selected.unwrap().pack_id.0, "core");
    }

    #[test]
    fn explicit_pack_selection_is_open_ended() {
        let mut registry = AssetPackRegistry::default();
        registry.mount(pack("core", 100, true)).unwrap();
        registry.mount(pack("biome_pack", 1, true)).unwrap();
        let result = SemanticAssetResolver::new(&registry).resolve(&SemanticAssetRequest {
            semantic_id: "terrain.grass".into(),
            category: AssetCategory::Terrain,
            context: AssetResolutionContext {
                explicit_pack: Some(AssetPackId("biome_pack".into())),
                ..Default::default()
            },
        });
        assert_eq!(result.selected.unwrap().pack_id.0, "biome_pack");
    }
}
