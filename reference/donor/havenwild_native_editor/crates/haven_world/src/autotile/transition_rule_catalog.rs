use super::{
    terrain_transition_rule_manifest, TerrainFamilySelector, TerrainTransitionRule,
    TransitionMaterial, TransitionRulePhase,
};

/// Editor-facing filters for browsing data-authored terrain transition rules.
///
/// The rule catalog intentionally stays in haven_world instead of haven_game so
/// the runtime overlay, standalone editor, validators, and future web tools all
/// inspect the same manifest-derived view of shoreline/terrain transition rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionRuleCatalogFilter {
    All,
    Edge,
    Corner,
    Shoreline,
    Natural,
    Constructed,
}

impl TransitionRuleCatalogFilter {
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Edge,
        Self::Corner,
        Self::Shoreline,
        Self::Natural,
        Self::Constructed,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Edge => "Edge",
            Self::Corner => "Corner",
            Self::Shoreline => "Shoreline",
            Self::Natural => "Natural",
            Self::Constructed => "Constructed",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Edge => "edge",
            Self::Corner => "corner",
            Self::Shoreline => "shoreline",
            Self::Natural => "natural",
            Self::Constructed => "constructed",
        }
    }

    pub fn next(self, delta: i32) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0) as i32;
        let len = Self::ALL.len() as i32;
        Self::ALL[(current + delta).rem_euclid(len) as usize]
    }

    pub fn matches(self, rule: &TerrainTransitionRule) -> bool {
        match self {
            Self::All => true,
            Self::Edge => rule.applies_to_phase(TransitionRulePhase::Edge),
            Self::Corner => rule.applies_to_phase(TransitionRulePhase::Corner),
            Self::Shoreline => {
                rule_touches_water(rule) || rule.material == TransitionMaterial::Foam
            }
            Self::Natural => {
                selector_is_natural(rule.center)
                    || selector_is_natural(rule.neighbor)
                    || matches!(
                        rule.material,
                        TransitionMaterial::GrassFringe
                            | TransitionMaterial::DirtBlend
                            | TransitionMaterial::SandBlend
                    )
            }
            Self::Constructed => {
                selector_is_constructed(rule.center)
                    || selector_is_constructed(rule.neighbor)
                    || matches!(
                        rule.material,
                        TransitionMaterial::RoadShoulder
                            | TransitionMaterial::StoneShoulder
                            | TransitionMaterial::RockShadow
                    )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionRuleCatalogRow {
    pub manifest_index: usize,
    pub id: String,
    pub center_selector: String,
    pub neighbor_selector: String,
    pub material: TransitionMaterial,
    pub atlas_group: String,
    pub applies_to: Vec<TransitionRulePhase>,
    pub priority: i32,
    pub reason: Option<String>,
}

impl TransitionRuleCatalogRow {
    pub fn phases_label(&self) -> String {
        self.applies_to
            .iter()
            .map(|phase| phase.code())
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn short_pair_label(&self) -> String {
        format!("{} -> {}", self.center_selector, self.neighbor_selector)
    }

    pub fn list_label(&self) -> String {
        format!(
            "{} | {} | {} | p{}",
            self.id,
            self.short_pair_label(),
            self.material.code(),
            self.priority
        )
    }
}

pub fn transition_rule_catalog_rows(
    filter: TransitionRuleCatalogFilter,
) -> Result<Vec<TransitionRuleCatalogRow>, String> {
    let manifest = terrain_transition_rule_manifest().map_err(|error| error.to_string())?;
    let mut rows = Vec::new();
    for (manifest_index, rule) in manifest.rules.iter().enumerate() {
        if !filter.matches(rule) {
            continue;
        }
        rows.push(TransitionRuleCatalogRow {
            manifest_index,
            id: rule.id.clone(),
            center_selector: rule.center.code().to_string(),
            neighbor_selector: rule.neighbor.code().to_string(),
            material: rule.material,
            atlas_group: rule.atlas_group.clone(),
            applies_to: rule.applies_to.clone(),
            priority: rule.priority,
            reason: rule.reason.clone(),
        });
    }
    rows.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.id.cmp(&right.id))
    });
    Ok(rows)
}

pub fn transition_rule_catalog_summary(
    filter: TransitionRuleCatalogFilter,
) -> Result<String, String> {
    let manifest = terrain_transition_rule_manifest().map_err(|error| error.to_string())?;
    let visible = manifest
        .rules
        .iter()
        .filter(|rule| filter.matches(rule))
        .count();
    Ok(format!(
        "{}: {} visible / {} authored transition rules",
        filter.label(),
        visible,
        manifest.rules.len()
    ))
}

fn rule_touches_water(rule: &TerrainTransitionRule) -> bool {
    selector_is_water(rule.center)
        || selector_is_water(rule.neighbor)
        || matches!(
            rule.material,
            TransitionMaterial::WetSand
                | TransitionMaterial::Foam
                | TransitionMaterial::ShallowWaterEdge
                | TransitionMaterial::SandBlend
        ) && matches!(
            rule.atlas_group.as_str(),
            "grass_bank_over_shallow"
                | "dirt_bank_over_shallow"
                | "sand_bank_over_shallow"
                | "shallow_rim_over_deep"
        )
}

fn selector_is_water(selector: TerrainFamilySelector) -> bool {
    matches!(selector, TerrainFamilySelector::Family(family) if family.is_water())
}

fn selector_is_natural(selector: TerrainFamilySelector) -> bool {
    match selector {
        TerrainFamilySelector::Family(family) => family.is_soft_natural(),
        TerrainFamilySelector::Land | TerrainFamilySelector::SoftNatural => true,
        _ => false,
    }
}

fn selector_is_constructed(selector: TerrainFamilySelector) -> bool {
    match selector {
        TerrainFamilySelector::Family(family) => {
            family.is_constructed() || family.is_blocking_wall()
        }
        TerrainFamilySelector::Constructed
        | TerrainFamilySelector::BlockingWall
        | TerrainFamilySelector::NonBlockingWall => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_rows_load_and_filter() {
        let all_rows = transition_rule_catalog_rows(TransitionRuleCatalogFilter::All)
            .expect("transition rule catalog should load");
        let shoreline_rows = transition_rule_catalog_rows(TransitionRuleCatalogFilter::Shoreline)
            .expect("shoreline filter should load");
        assert!(all_rows.len() >= shoreline_rows.len());
        assert!(shoreline_rows.iter().any(|row| row.id.contains("water")));
    }

    #[test]
    fn filter_cycles_in_manifest_order() {
        assert_eq!(
            TransitionRuleCatalogFilter::All.next(1),
            TransitionRuleCatalogFilter::Edge
        );
        assert_eq!(
            TransitionRuleCatalogFilter::All.next(-1),
            TransitionRuleCatalogFilter::Constructed
        );
    }
}
