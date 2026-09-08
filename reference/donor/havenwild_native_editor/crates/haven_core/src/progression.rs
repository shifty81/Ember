use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PROGRESSION_CATALOG_SCHEMA: &str = "havenwild.progression_catalog.v0_1";
pub const MAX_ACTIVE_CLASS_SLOTS: u8 = 2;
pub const MAX_PROGRESSION_LEVEL: u8 = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressionCatalog {
    #[serde(default = "default_catalog_schema")]
    pub schema: String,
    #[serde(default)]
    pub balance: ProgressionBalance,
    #[serde(default)]
    pub classes: Vec<ClassDefinition>,
    #[serde(default)]
    pub professions: Vec<ProfessionDefinition>,
    #[serde(default)]
    pub general_skills: Vec<GeneralSkillDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionBalance {
    #[serde(default = "default_level_curve")]
    pub class_level_xp: Vec<u64>,
    #[serde(default = "default_level_curve")]
    pub profession_level_xp: Vec<u64>,
    #[serde(default = "default_level_curve")]
    pub general_skill_level_xp: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tree: SkillTreeDefinition,
    #[serde(default)]
    pub mastery: MasteryDefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProfessionDefinition {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tree: SkillTreeDefinition,
    #[serde(default)]
    pub mastery: MasteryDefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneralSkillDefinition {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tree: SkillTreeDefinition,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillTreeDefinition {
    #[serde(default)]
    pub nodes: Vec<SkillNodeDefinition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillNodeDefinition {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_required_level")]
    pub required_level: u8,
    #[serde(default = "default_point_cost")]
    pub point_cost: u8,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub exclusive_with: Vec<String>,
    #[serde(default)]
    pub effects: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasteryDefinition {
    #[serde(default = "default_mastery_level")]
    pub level: u8,
    #[serde(default)]
    pub mastery_node_id: String,
    #[serde(default)]
    pub milestone_ids: Vec<String>,
}

impl Default for ProgressionCatalog {
    fn default() -> Self {
        Self {
            schema: default_catalog_schema(),
            balance: ProgressionBalance::default(),
            classes: Vec::new(),
            professions: Vec::new(),
            general_skills: Vec::new(),
        }
    }
}

impl Default for ProgressionBalance {
    fn default() -> Self {
        Self {
            class_level_xp: default_level_curve(),
            profession_level_xp: default_level_curve(),
            general_skill_level_xp: default_level_curve(),
        }
    }
}

impl Default for MasteryDefinition {
    fn default() -> Self {
        Self {
            level: default_mastery_level(),
            mastery_node_id: String::new(),
            milestone_ids: Vec::new(),
        }
    }
}

impl ProgressionCatalog {
    pub fn class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.iter().find(|entry| entry.id == id)
    }

    pub fn profession(&self, id: &str) -> Option<&ProfessionDefinition> {
        self.professions.iter().find(|entry| entry.id == id)
    }

    pub fn general_skill(&self, id: &str) -> Option<&GeneralSkillDefinition> {
        self.general_skills.iter().find(|entry| entry.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != PROGRESSION_CATALOG_SCHEMA {
            return Err(format!(
                "unsupported progression catalog schema {}",
                self.schema
            ));
        }
        validate_level_curve("class", &self.balance.class_level_xp)?;
        validate_level_curve("profession", &self.balance.profession_level_xp)?;
        validate_level_curve("general skill", &self.balance.general_skill_level_xp)?;
        validate_definitions("class", &self.classes, |value| {
            (&value.id, &value.tree, Some(&value.mastery))
        })?;
        validate_definitions("profession", &self.professions, |value| {
            (&value.id, &value.tree, Some(&value.mastery))
        })?;
        validate_definitions("general skill", &self.general_skills, |value| {
            (&value.id, &value.tree, None)
        })?;
        Ok(())
    }
}

pub fn level_for_total_xp(curve: &[u64], total_xp: u64) -> u8 {
    if curve.is_empty() {
        return 1;
    }
    let mut level = 1u8;
    for (index, threshold) in curve.iter().enumerate().skip(1) {
        if total_xp < *threshold {
            break;
        }
        level = (index + 1).min(MAX_PROGRESSION_LEVEL as usize) as u8;
    }
    level.clamp(1, MAX_PROGRESSION_LEVEL)
}

fn validate_level_curve(label: &str, curve: &[u64]) -> Result<(), String> {
    if curve.len() != MAX_PROGRESSION_LEVEL as usize {
        return Err(format!(
            "{label} XP curve must contain exactly {MAX_PROGRESSION_LEVEL} levels"
        ));
    }
    if curve.first().copied() != Some(0) {
        return Err(format!("{label} XP curve must begin at zero"));
    }
    if curve.windows(2).any(|window| window[1] <= window[0]) {
        return Err(format!("{label} XP thresholds must increase strictly"));
    }
    Ok(())
}

fn validate_definitions<T>(
    label: &str,
    values: &[T],
    read: impl for<'a> Fn(
        &'a T,
    ) -> (
        &'a str,
        &'a SkillTreeDefinition,
        Option<&'a MasteryDefinition>,
    ),
) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for value in values {
        let (id, tree, mastery) = read(value);
        if id.trim().is_empty() {
            return Err(format!("{label} id cannot be empty"));
        }
        if !ids.insert(id) {
            return Err(format!("duplicate {label} id {id}"));
        }
        validate_tree(label, id, tree)?;
        if let Some(mastery) = mastery {
            if mastery.level == 0 || mastery.level > MAX_PROGRESSION_LEVEL {
                return Err(format!("{label} {id} mastery level is invalid"));
            }
            if !mastery.mastery_node_id.is_empty()
                && !tree
                    .nodes
                    .iter()
                    .any(|node| node.id == mastery.mastery_node_id)
            {
                return Err(format!("{label} {id} mastery node does not exist"));
            }
        }
    }
    Ok(())
}

fn validate_tree(label: &str, owner_id: &str, tree: &SkillTreeDefinition) -> Result<(), String> {
    let node_ids: BTreeSet<&str> = tree.nodes.iter().map(|node| node.id.as_str()).collect();
    if node_ids.len() != tree.nodes.len() {
        return Err(format!("{label} {owner_id} has duplicate skill node IDs"));
    }
    for node in &tree.nodes {
        if node.id.trim().is_empty() {
            return Err(format!(
                "{label} {owner_id} contains an empty skill node ID"
            ));
        }
        if node.required_level == 0 || node.required_level > MAX_PROGRESSION_LEVEL {
            return Err(format!(
                "skill node {} has an invalid required level",
                node.id
            ));
        }
        if node.point_cost == 0 {
            return Err(format!(
                "skill node {} must cost at least one point",
                node.id
            ));
        }
        for dependency in node.prerequisites.iter().chain(node.exclusive_with.iter()) {
            if !node_ids.contains(dependency.as_str()) {
                return Err(format!(
                    "skill node {} references unknown node {dependency}",
                    node.id
                ));
            }
        }
    }
    Ok(())
}

fn default_catalog_schema() -> String {
    PROGRESSION_CATALOG_SCHEMA.to_string()
}

fn default_level_curve() -> Vec<u64> {
    vec![0, 100, 300, 700, 1_400, 2_500, 4_100, 6_300, 9_200, 13_000]
}

const fn default_required_level() -> u8 {
    1
}
const fn default_point_cost() -> u8 {
    1
}
const fn default_mastery_level() -> u8 {
    MAX_PROGRESSION_LEVEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_curve_caps_at_ten() {
        let curve = default_level_curve();
        assert_eq!(level_for_total_xp(&curve, 0), 1);
        assert_eq!(level_for_total_xp(&curve, 299), 2);
        assert_eq!(level_for_total_xp(&curve, u64::MAX), 10);
    }

    #[test]
    fn empty_catalog_is_structurally_valid() {
        ProgressionCatalog::default().validate().unwrap();
    }
}
