use haven_core::{
    level_for_total_xp, ProgressionCatalog, SkillTreeDefinition, MAX_ACTIVE_CLASS_SLOTS,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CHARACTER_PROGRESSION_SCHEMA: &str = "havenwild.character_progression.v0_1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterProgression {
    #[serde(default = "default_progression_schema")]
    pub schema: String,
    #[serde(default = "default_class_slots")]
    pub class_slots_unlocked: u8,
    #[serde(default)]
    pub active_class_ids: Vec<String>,
    #[serde(default)]
    pub classes: BTreeMap<String, ProgressionTrack>,
    #[serde(default)]
    pub professions: BTreeMap<String, ProgressionTrack>,
    #[serde(default)]
    pub general_skills: BTreeMap<String, ProgressionTrack>,
    #[serde(default)]
    pub discovered_profession_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub legacy_skill_levels: BTreeMap<String, u32>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub legacy_skill_points: u32,
    #[serde(default)]
    pub learned_recipes: BTreeSet<String>,
    #[serde(default)]
    pub unlocked_abilities: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionTrack {
    #[serde(default = "default_level")]
    pub level: u8,
    #[serde(default)]
    pub total_xp: u64,
    #[serde(default)]
    pub unspent_points: u16,
    #[serde(default)]
    pub allocated_node_ids: BTreeSet<String>,
    #[serde(default)]
    pub completed_milestone_ids: BTreeSet<String>,
    #[serde(default)]
    pub mastered: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionAward {
    pub previous_level: u8,
    pub current_level: u8,
    pub levels_gained: u8,
    pub total_xp: u64,
}

impl Default for ProgressionTrack {
    fn default() -> Self {
        Self {
            level: default_level(),
            total_xp: 0,
            unspent_points: 0,
            allocated_node_ids: BTreeSet::new(),
            completed_milestone_ids: BTreeSet::new(),
            mastered: false,
        }
    }
}

impl Default for CharacterProgression {
    fn default() -> Self {
        Self {
            schema: CHARACTER_PROGRESSION_SCHEMA.to_string(),
            class_slots_unlocked: default_class_slots(),
            active_class_ids: Vec::new(),
            classes: BTreeMap::new(),
            professions: BTreeMap::new(),
            general_skills: BTreeMap::new(),
            discovered_profession_ids: BTreeSet::new(),
            legacy_skill_levels: BTreeMap::new(),
            legacy_skill_points: 0,
            learned_recipes: BTreeSet::new(),
            unlocked_abilities: BTreeSet::new(),
        }
    }
}

impl CharacterProgression {
    pub fn validate(&self, catalog: Option<&ProgressionCatalog>) -> Result<(), String> {
        if self.schema != CHARACTER_PROGRESSION_SCHEMA {
            return Err(format!(
                "unsupported character progression schema {}",
                self.schema
            ));
        }
        if !(1..=MAX_ACTIVE_CLASS_SLOTS).contains(&self.class_slots_unlocked) {
            return Err("character class slot count must be one or two".to_string());
        }
        if self.active_class_ids.len() > self.class_slots_unlocked as usize {
            return Err("active classes exceed unlocked class slots".to_string());
        }
        let active: BTreeSet<_> = self.active_class_ids.iter().collect();
        if active.len() != self.active_class_ids.len() {
            return Err("active class IDs must be unique".to_string());
        }
        for track in self
            .classes
            .values()
            .chain(self.professions.values())
            .chain(self.general_skills.values())
        {
            track.validate()?;
        }
        if let Some(catalog) = catalog {
            catalog.validate()?;
            for class_id in self.classes.keys().chain(self.active_class_ids.iter()) {
                if catalog.class(class_id).is_none() {
                    return Err(format!("unknown class id {class_id}"));
                }
            }
            for profession_id in self
                .professions
                .keys()
                .chain(self.discovered_profession_ids.iter())
            {
                if catalog.profession(profession_id).is_none() {
                    return Err(format!("unknown profession id {profession_id}"));
                }
            }
            for skill_id in self.general_skills.keys() {
                if catalog.general_skill(skill_id).is_none() {
                    return Err(format!("unknown general skill id {skill_id}"));
                }
            }
            validate_allocations(&self.classes, |id| {
                catalog.class(id).map(|value| &value.tree)
            })?;
            validate_allocations(&self.professions, |id| {
                catalog.profession(id).map(|value| &value.tree)
            })?;
            validate_allocations(&self.general_skills, |id| {
                catalog.general_skill(id).map(|value| &value.tree)
            })?;
        }
        Ok(())
    }

    pub fn set_active_class(
        &mut self,
        slot: usize,
        class_id: &str,
        catalog: &ProgressionCatalog,
    ) -> Result<(), String> {
        if slot >= self.class_slots_unlocked as usize {
            return Err("class slot is locked".to_string());
        }
        if catalog.class(class_id).is_none() {
            return Err(format!("unknown class id {class_id}"));
        }
        if self
            .active_class_ids
            .iter()
            .enumerate()
            .any(|(index, active)| index != slot && active == class_id)
        {
            return Err("the same class cannot occupy both active slots".to_string());
        }
        self.classes.entry(class_id.to_string()).or_default();
        if slot < self.active_class_ids.len() {
            self.active_class_ids[slot] = class_id.to_string();
        } else if slot == self.active_class_ids.len() {
            self.active_class_ids.push(class_id.to_string());
        } else {
            return Err("class slots must be filled in order".to_string());
        }
        Ok(())
    }

    pub fn discover_profession(
        &mut self,
        profession_id: &str,
        catalog: &ProgressionCatalog,
    ) -> Result<bool, String> {
        if catalog.profession(profession_id).is_none() {
            return Err(format!("unknown profession id {profession_id}"));
        }
        self.professions
            .entry(profession_id.to_string())
            .or_default();
        Ok(self
            .discovered_profession_ids
            .insert(profession_id.to_string()))
    }

    pub fn award_class_xp(
        &mut self,
        class_id: &str,
        amount: u64,
        catalog: &ProgressionCatalog,
    ) -> Result<ProgressionAward, String> {
        if !self
            .active_class_ids
            .iter()
            .any(|active| active == class_id)
        {
            return Err(format!("class {class_id} is not active"));
        }
        let definition = catalog
            .class(class_id)
            .ok_or_else(|| format!("unknown class id {class_id}"))?;
        let track = self.classes.entry(class_id.to_string()).or_default();
        let award = track.award_xp(amount, &catalog.balance.class_level_xp);
        if track.level >= definition.mastery.level
            && definition
                .mastery
                .milestone_ids
                .iter()
                .all(|milestone| track.completed_milestone_ids.contains(milestone))
            && track
                .allocated_node_ids
                .contains(&definition.mastery.mastery_node_id)
        {
            track.mastered = true;
            self.class_slots_unlocked = MAX_ACTIVE_CLASS_SLOTS;
        }
        Ok(award)
    }

    pub fn award_profession_xp(
        &mut self,
        profession_id: &str,
        amount: u64,
        catalog: &ProgressionCatalog,
    ) -> Result<ProgressionAward, String> {
        if !self.discovered_profession_ids.contains(profession_id) {
            return Err(format!(
                "profession {profession_id} has not been discovered"
            ));
        }
        let definition = catalog
            .profession(profession_id)
            .ok_or_else(|| format!("unknown profession id {profession_id}"))?;
        let track = self
            .professions
            .entry(profession_id.to_string())
            .or_default();
        let award = track.award_xp(amount, &catalog.balance.profession_level_xp);
        if track.level >= definition.mastery.level
            && definition
                .mastery
                .milestone_ids
                .iter()
                .all(|milestone| track.completed_milestone_ids.contains(milestone))
            && track
                .allocated_node_ids
                .contains(&definition.mastery.mastery_node_id)
        {
            track.mastered = true;
        }
        Ok(award)
    }

    pub fn allocate_class_node(
        &mut self,
        class_id: &str,
        node_id: &str,
        catalog: &ProgressionCatalog,
    ) -> Result<(), String> {
        let tree = &catalog
            .class(class_id)
            .ok_or_else(|| format!("unknown class id {class_id}"))?
            .tree;
        allocate_node(
            self.classes.entry(class_id.to_string()).or_default(),
            node_id,
            tree,
        )
    }

    pub fn allocate_profession_node(
        &mut self,
        profession_id: &str,
        node_id: &str,
        catalog: &ProgressionCatalog,
    ) -> Result<(), String> {
        let tree = &catalog
            .profession(profession_id)
            .ok_or_else(|| format!("unknown profession id {profession_id}"))?
            .tree;
        allocate_node(
            self.professions
                .entry(profession_id.to_string())
                .or_default(),
            node_id,
            tree,
        )
    }
}

impl ProgressionTrack {
    fn validate(&self) -> Result<(), String> {
        if self.level == 0 || self.level > 10 {
            return Err("progression track level must be between one and ten".to_string());
        }
        Ok(())
    }

    fn award_xp(&mut self, amount: u64, curve: &[u64]) -> ProgressionAward {
        let previous_level = self.level;
        self.total_xp = self.total_xp.saturating_add(amount);
        self.level = level_for_total_xp(curve, self.total_xp);
        let levels_gained = self.level.saturating_sub(previous_level);
        self.unspent_points = self.unspent_points.saturating_add(levels_gained as u16);
        ProgressionAward {
            previous_level,
            current_level: self.level,
            levels_gained,
            total_xp: self.total_xp,
        }
    }
}

fn allocate_node(
    track: &mut ProgressionTrack,
    node_id: &str,
    tree: &SkillTreeDefinition,
) -> Result<(), String> {
    let node = tree
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .ok_or_else(|| format!("unknown skill node id {node_id}"))?;
    if track.allocated_node_ids.contains(node_id) {
        return Err("skill node is already allocated".to_string());
    }
    if track.level < node.required_level {
        return Err(format!("skill node requires level {}", node.required_level));
    }
    if node
        .prerequisites
        .iter()
        .any(|id| !track.allocated_node_ids.contains(id))
    {
        return Err("skill node prerequisites are not satisfied".to_string());
    }
    if node
        .exclusive_with
        .iter()
        .any(|id| track.allocated_node_ids.contains(id))
    {
        return Err("skill node conflicts with an allocated node".to_string());
    }
    if track.unspent_points < node.point_cost as u16 {
        return Err("not enough progression points".to_string());
    }
    track.unspent_points -= node.point_cost as u16;
    track.allocated_node_ids.insert(node_id.to_string());
    Ok(())
}

fn validate_allocations<'a, 'b>(
    tracks: &'a BTreeMap<String, ProgressionTrack>,
    tree_for: impl Fn(&str) -> Option<&'b SkillTreeDefinition>,
) -> Result<(), String> {
    for (id, track) in tracks {
        let tree = tree_for(id).ok_or_else(|| format!("missing skill tree for {id}"))?;
        for node_id in &track.allocated_node_ids {
            if !tree.nodes.iter().any(|node| &node.id == node_id) {
                return Err(format!("unknown allocated skill node {node_id} for {id}"));
            }
        }
    }
    Ok(())
}

fn default_progression_schema() -> String {
    CHARACTER_PROGRESSION_SCHEMA.to_string()
}
const fn default_class_slots() -> u8 {
    1
}
const fn default_level() -> u8 {
    1
}
const fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}
