use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use super::{TerrainFamily, TransitionMaterial};

pub const TERRAIN_TRANSITION_RULE_MANIFEST_PATH: &str =
    "content/worldgen/terrain_transition_rule_manifest_v0_1.json";

// Runtime rule selection mirrors the editor contract: highest priority matching rule wins.
// Hardcoded resolver matches are fallback-only safety behavior.

static TERRAIN_TRANSITION_RULE_MANIFEST: OnceLock<
    Mutex<&'static Result<TerrainTransitionRuleManifest, String>>,
> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionRulePhase {
    Edge,
    Corner,
}

impl TransitionRulePhase {
    pub fn code(self) -> &'static str {
        match self {
            Self::Edge => "edge",
            Self::Corner => "corner",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainFamilySelector {
    Family(TerrainFamily),
    Land,
    SoftNatural,
    Constructed,
    BlockingWall,
    NonBlockingWall,
    Any,
}

impl TerrainFamilySelector {
    pub fn from_code(code: &str) -> Option<Self> {
        TerrainFamily::from_code(code)
            .map(Self::Family)
            .or(match code {
                "land" => Some(Self::Land),
                "soft_natural" => Some(Self::SoftNatural),
                "constructed" => Some(Self::Constructed),
                "blocking_wall" => Some(Self::BlockingWall),
                "non_blocking_wall" => Some(Self::NonBlockingWall),
                "any" => Some(Self::Any),
                _ => None,
            })
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Family(family) => family.code(),
            Self::Land => "land",
            Self::SoftNatural => "soft_natural",
            Self::Constructed => "constructed",
            Self::BlockingWall => "blocking_wall",
            Self::NonBlockingWall => "non_blocking_wall",
            Self::Any => "any",
        }
    }

    pub fn matches(self, family: TerrainFamily) -> bool {
        match self {
            Self::Family(expected) => family == expected,
            Self::Land => family.is_land(),
            Self::SoftNatural => family.is_soft_natural(),
            Self::Constructed => family.is_constructed(),
            Self::BlockingWall => family.is_blocking_wall(),
            Self::NonBlockingWall => family != TerrainFamily::Void && !family.is_blocking_wall(),
            Self::Any => family != TerrainFamily::Void,
        }
    }

    pub fn representative_family(self) -> TerrainFamily {
        match self {
            Self::Family(family) => family,
            Self::Land | Self::SoftNatural | Self::NonBlockingWall | Self::Any => {
                TerrainFamily::Grass
            }
            Self::Constructed => TerrainFamily::Road,
            Self::BlockingWall => TerrainFamily::RockWall,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRule {
    pub id: String,
    pub center: TerrainFamilySelector,
    pub neighbor: TerrainFamilySelector,
    pub material: TransitionMaterial,
    pub atlas_group: String,
    pub applies_to: Vec<TransitionRulePhase>,
    pub priority: i32,
    pub reason: Option<String>,
}

impl TerrainTransitionRule {
    pub fn applies_to_phase(&self, phase: TransitionRulePhase) -> bool {
        self.applies_to.contains(&phase)
    }

    pub fn matches(
        &self,
        phase: TransitionRulePhase,
        center: TerrainFamily,
        neighbor: TerrainFamily,
    ) -> bool {
        self.applies_to_phase(phase)
            && self.center.matches(center)
            && self.neighbor.matches(neighbor)
    }

    pub fn debug_label(&self) -> String {
        format!(
            "{}: {} -> {} = {} @ {}",
            self.id,
            self.center.code(),
            self.neighbor.code(),
            self.material.code(),
            self.atlas_group
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRuleManifest {
    pub schema: String,
    pub id: String,
    pub kind: String,
    pub version: String,
    pub rules: Vec<TerrainTransitionRule>,
    manifest_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRuleManifestReloadReport {
    pub manifest_path: PathBuf,
    pub old_rule_count: usize,
    pub new_rule_count: usize,
    pub old_summary: String,
    pub new_summary: String,
}

impl TerrainTransitionRuleManifestReloadReport {
    pub fn status_line(&self) -> String {
        format!(
            "Reloaded terrain transition rules: {} -> {} rule(s)",
            self.old_rule_count, self.new_rule_count
        )
    }
}

impl TerrainTransitionRuleManifest {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(TERRAIN_TRANSITION_RULE_MANIFEST_PATH);
        let file = load_json::<TerrainTransitionRuleManifestFile>(&manifest_path)?;
        let mut rules = Vec::with_capacity(file.rules.len());
        for rule in file.rules {
            rules.push(TerrainTransitionRule::try_from(rule)?);
        }
        let loaded = Self {
            schema: file.schema,
            id: file.id,
            kind: file.kind,
            version: file.version,
            rules,
            manifest_path,
        };
        loaded.validate()?;
        Ok(loaded)
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn coverage_summary(&self) -> String {
        format!(
            "{} -> {} terrain transition rules",
            self.id,
            self.rules.len()
        )
    }

    pub fn material_for(
        &self,
        phase: TransitionRulePhase,
        center: TerrainFamily,
        neighbor: TerrainFamily,
    ) -> Option<TransitionMaterial> {
        self.best_rule_for(phase, center, neighbor)
            .map(|rule| rule.material)
    }

    pub fn atlas_group_for_material(&self, material: TransitionMaterial) -> Option<&str> {
        self.rules
            .iter()
            .filter(|rule| rule.material == material)
            .max_by_key(|rule| rule.priority)
            .map(|rule| rule.atlas_group.as_str())
    }

    pub fn best_rule_for(
        &self,
        phase: TransitionRulePhase,
        center: TerrainFamily,
        neighbor: TerrainFamily,
    ) -> Option<&TerrainTransitionRule> {
        self.rules
            .iter()
            .filter(|rule| rule.matches(phase, center, neighbor))
            .max_by_key(|rule| rule.priority)
    }

    fn validate(&self) -> Result<(), String> {
        if self.rules.is_empty() {
            return Err(format!("{} has no terrain transition rules", self.id));
        }
        for rule in &self.rules {
            if rule.id.trim().is_empty() {
                return Err(format!(
                    "{} has a terrain transition rule with an empty id",
                    self.id
                ));
            }
            if rule.atlas_group.trim().is_empty() {
                return Err(format!(
                    "{} rule {} has an empty atlas group",
                    self.id, rule.id
                ));
            }
            if rule.applies_to.is_empty() {
                return Err(format!(
                    "{} rule {} has no applies_to phases",
                    self.id, rule.id
                ));
            }
        }
        Ok(())
    }
}

impl TryFrom<TerrainTransitionRuleFile> for TerrainTransitionRule {
    type Error = String;

    fn try_from(file: TerrainTransitionRuleFile) -> Result<Self, Self::Error> {
        let center = TerrainFamilySelector::from_code(&file.center).ok_or_else(|| {
            format!(
                "unknown terrain transition center selector: {}",
                file.center
            )
        })?;
        let neighbor = TerrainFamilySelector::from_code(&file.neighbor).ok_or_else(|| {
            format!(
                "unknown terrain transition neighbor selector: {}",
                file.neighbor
            )
        })?;
        let material = TransitionMaterial::from_code(&file.material)
            .ok_or_else(|| format!("unknown terrain transition material: {}", file.material))?;
        let mut applies_to = Vec::with_capacity(file.applies_to.len());
        for phase in file.applies_to {
            applies_to.push(match phase.as_str() {
                "edge" => TransitionRulePhase::Edge,
                "corner" => TransitionRulePhase::Corner,
                other => return Err(format!("unknown transition rule phase: {other}")),
            });
        }
        Ok(Self {
            id: file.id,
            center,
            neighbor,
            material,
            atlas_group: file.atlas_group,
            applies_to,
            priority: file.priority,
            reason: file.reason,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
struct TerrainTransitionRuleManifestFile {
    schema: String,
    id: String,
    kind: String,
    version: String,
    rules: Vec<TerrainTransitionRuleFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct TerrainTransitionRuleFile {
    id: String,
    center: String,
    neighbor: String,
    material: String,
    #[serde(rename = "atlasGroup")]
    atlas_group: String,
    #[serde(rename = "appliesTo")]
    applies_to: Vec<String>,
    priority: i32,
    reason: Option<String>,
}

pub fn terrain_transition_rule_manifest(
) -> Result<&'static TerrainTransitionRuleManifest, &'static str> {
    let cached = terrain_transition_rule_manifest_cache()
        .lock()
        .expect("terrain transition rule manifest cache mutex poisoned");
    let result = *cached;
    result.as_ref().map_err(|error| error.as_str())
}

pub fn terrain_transition_rule_manifest_cache_summary() -> String {
    terrain_transition_rule_manifest()
        .map(|manifest| manifest.coverage_summary())
        .unwrap_or_else(|error| format!("terrain transition rule manifest unavailable: {error}"))
}

pub fn reload_terrain_transition_rule_manifest(
) -> Result<TerrainTransitionRuleManifestReloadReport, String> {
    let old_result = {
        let cached = terrain_transition_rule_manifest_cache()
            .lock()
            .expect("terrain transition rule manifest cache mutex poisoned");
        *cached
    };
    let old_rule_count = old_result
        .as_ref()
        .map(|manifest| manifest.rules.len())
        .unwrap_or(0);
    let old_summary = old_result
        .as_ref()
        .map(|manifest| manifest.coverage_summary())
        .unwrap_or_else(|error| format!("cached manifest unavailable: {error}"));

    let fresh_manifest = TerrainTransitionRuleManifest::load_default()?;
    let manifest_path = fresh_manifest.manifest_path().to_path_buf();
    let new_rule_count = fresh_manifest.rules.len();
    let new_summary = fresh_manifest.coverage_summary();
    let fresh_result: &'static Result<TerrainTransitionRuleManifest, String> =
        Box::leak(Box::new(Ok(fresh_manifest)));

    let mut cached = terrain_transition_rule_manifest_cache()
        .lock()
        .expect("terrain transition rule manifest cache mutex poisoned");
    *cached = fresh_result;

    Ok(TerrainTransitionRuleManifestReloadReport {
        manifest_path,
        old_rule_count,
        new_rule_count,
        old_summary,
        new_summary,
    })
}

fn terrain_transition_rule_manifest_cache(
) -> &'static Mutex<&'static Result<TerrainTransitionRuleManifest, String>> {
    TERRAIN_TRANSITION_RULE_MANIFEST.get_or_init(|| {
        Mutex::new(Box::leak(Box::new(
            TerrainTransitionRuleManifest::load_default(),
        )))
    })
}

pub fn transition_rule_material(
    phase: TransitionRulePhase,
    center: TerrainFamily,
    neighbor: TerrainFamily,
) -> Option<TransitionMaterial> {
    terrain_transition_rule_manifest()
        .ok()
        .and_then(|manifest| manifest.material_for(phase, center, neighbor))
}

pub fn transition_rule_atlas_group_for_material(
    material: TransitionMaterial,
) -> Option<&'static str> {
    let manifest = terrain_transition_rule_manifest().ok()?;
    manifest.atlas_group_for_material(material)
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_world")
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_manifest_loads_transition_rules() {
        let manifest = TerrainTransitionRuleManifest::load_default()
            .expect("terrain transition rule manifest should load");
        assert_eq!(manifest.id, "terrain_transition_rule_manifest_v0_1");
        assert!(manifest
            .manifest_path()
            .ends_with(TERRAIN_TRANSITION_RULE_MANIFEST_PATH));
        assert!(manifest.rules.len() >= 12);
        assert!(manifest
            .coverage_summary()
            .contains("terrain transition rules"));
    }

    #[test]
    fn manifest_resolves_owner_side_shoreline_and_depth_rules() {
        let manifest = TerrainTransitionRuleManifest::load_default()
            .expect("terrain transition rule manifest should load");
        assert_eq!(
            manifest.material_for(
                TransitionRulePhase::Edge,
                TerrainFamily::ShallowWater,
                TerrainFamily::Sand
            ),
            Some(TransitionMaterial::WetSand)
        );
        assert_eq!(
            manifest.material_for(
                TransitionRulePhase::Edge,
                TerrainFamily::DeepWater,
                TerrainFamily::ShallowWater
            ),
            Some(TransitionMaterial::ShallowWaterEdge)
        );
    }

    #[test]
    fn family_selectors_match_grouped_runtime_families() {
        assert!(TerrainFamilySelector::Land.matches(TerrainFamily::Sand));
        assert!(TerrainFamilySelector::SoftNatural.matches(TerrainFamily::Farm));
        assert!(TerrainFamilySelector::Constructed.matches(TerrainFamily::Road));
        assert!(TerrainFamilySelector::BlockingWall.matches(TerrainFamily::RockWall));
        assert!(TerrainFamilySelector::NonBlockingWall.matches(TerrainFamily::Grass));
        assert!(!TerrainFamilySelector::NonBlockingWall.matches(TerrainFamily::RockWall));
    }
    #[test]
    fn manifest_cache_can_reload_without_restart() {
        let before = terrain_transition_rule_manifest_cache_summary();
        let report = reload_terrain_transition_rule_manifest()
            .expect("terrain transition rule manifest should reload in-session");
        assert!(report
            .manifest_path
            .ends_with(TERRAIN_TRANSITION_RULE_MANIFEST_PATH));
        assert!(report.new_rule_count >= 12);
        assert!(report
            .status_line()
            .contains("Reloaded terrain transition rules"));
        assert!(!before.is_empty());
    }
}
