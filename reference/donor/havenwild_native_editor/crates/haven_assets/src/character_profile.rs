use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::read_to_string;

use crate::character_creation::{
    validate_character_creation_catalog, CharacterCreationCatalog, StarterClothingItem,
};

pub const CHARACTER_PROFILE_POLICY_PATH: &str =
    "content/characters/character_profile_policy_v0_1.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterProfilePolicy {
    pub schema: String,
    pub profile_rule: String,
    pub color_channels: Vec<ColorChannelPolicy>,
    pub equipment_categories: Vec<EquipmentCategoryPolicy>,
    pub initial_creator_allowed_slots: Vec<String>,
    pub progression_only_categories: Vec<String>,
    pub required_neutral_idle_aliases: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColorChannelPolicy {
    pub id: String,
    pub label: String,
    pub applies_to_categories: Vec<String>,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquipmentCategoryPolicy {
    pub id: String,
    pub label: String,
    pub creator_access: CreatorAccess,
    pub unlock_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreatorAccess {
    Starter,
    ProgressionOnly,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterProfileSelection {
    pub schema: String,
    pub character_id: String,
    pub display_name: String,
    pub appearance: BTreeMap<String, String>,
    pub colors: BTreeMap<String, String>,
    pub equipped_starter_clothing: BTreeMap<String, String>,
}

impl CharacterProfilePolicy {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }
}

pub fn build_default_character_profile(
    catalog: &CharacterCreationCatalog,
) -> CharacterProfileSelection {
    let appearance = catalog
        .appearance_categories
        .iter()
        .filter_map(|category| {
            category
                .options
                .iter()
                .find(|option| option.default)
                .or_else(|| category.options.first())
                .map(|option| (category.id.clone(), option.id.clone()))
        })
        .collect();

    let equipped_starter_clothing = catalog
        .starter_clothing
        .iter()
        .filter(|item| item.default)
        .map(|item| (item.slot.clone(), item.id.clone()))
        .collect();

    CharacterProfileSelection {
        schema: "havenwild.character_profile_selection.v0.1".to_string(),
        character_id: "new_character".to_string(),
        display_name: "New Character".to_string(),
        appearance,
        colors: BTreeMap::new(),
        equipped_starter_clothing,
    }
}

pub fn validate_character_profile_policy(
    policy: &CharacterProfilePolicy,
    catalog: &CharacterCreationCatalog,
) -> Vec<String> {
    let mut errors = validate_character_creation_catalog(catalog);
    let allowed_slots: HashSet<_> = policy
        .initial_creator_allowed_slots
        .iter()
        .map(|slot| slot.to_ascii_lowercase())
        .collect();
    let progression: HashSet<_> = policy
        .progression_only_categories
        .iter()
        .map(|category| category.to_ascii_lowercase())
        .collect();

    for item in &catalog.starter_clothing {
        if !allowed_slots.contains(&item.slot.to_ascii_lowercase()) {
            errors.push(format!(
                "starter clothing '{}' uses slot '{}' not allowed by the profile policy",
                item.id, item.slot
            ));
        }
    }

    for required in ["armor", "weapons", "shields", "advanced_outfits"] {
        if !progression.contains(required) {
            errors.push(format!(
                "profile policy must keep '{required}' progression-only"
            ));
        }
    }

    let access_by_id: HashMap<_, _> = policy
        .equipment_categories
        .iter()
        .map(|category| (category.id.to_ascii_lowercase(), &category.creator_access))
        .collect();
    for required in ["armor", "weapons", "shields"] {
        if !matches!(
            access_by_id.get(required),
            Some(CreatorAccess::ProgressionOnly)
        ) {
            errors.push(format!(
                "equipment category '{required}' must be progression-only"
            ));
        }
    }

    for required in ["idle_down", "idle_up", "idle_left", "idle_right"] {
        if !policy
            .required_neutral_idle_aliases
            .iter()
            .any(|alias| alias == required)
        {
            errors.push(format!(
                "profile policy is missing neutral idle alias '{required}'"
            ));
        }
    }

    let category_ids: HashSet<_> = catalog
        .appearance_categories
        .iter()
        .map(|category| category.id.as_str())
        .collect();
    for channel in &policy.color_channels {
        for category in &channel.applies_to_categories {
            if !category_ids.contains(category.as_str()) {
                errors.push(format!(
                    "color channel '{}' references unknown appearance category '{}'",
                    channel.id, category
                ));
            }
        }
    }

    errors
}

pub fn validate_character_profile_selection(
    selection: &CharacterProfileSelection,
    catalog: &CharacterCreationCatalog,
    policy: &CharacterProfilePolicy,
) -> Vec<String> {
    let mut errors = Vec::new();
    let appearance_options: HashMap<_, HashSet<_>> = catalog
        .appearance_categories
        .iter()
        .map(|category| {
            (
                category.id.as_str(),
                category
                    .options
                    .iter()
                    .map(|option| option.id.as_str())
                    .collect(),
            )
        })
        .collect();

    for category in &catalog.appearance_categories {
        match selection.appearance.get(&category.id) {
            Some(option) if appearance_options[category.id.as_str()].contains(option.as_str()) => {}
            Some(option) => errors.push(format!(
                "appearance category '{}' selected unknown option '{}'",
                category.id, option
            )),
            None => errors.push(format!(
                "appearance category '{}' has no selection",
                category.id
            )),
        }
    }

    let clothing_by_id: HashMap<_, &StarterClothingItem> = catalog
        .starter_clothing
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let allowed_slots: HashSet<_> = policy
        .initial_creator_allowed_slots
        .iter()
        .map(|slot| slot.as_str())
        .collect();
    for (slot, item_id) in &selection.equipped_starter_clothing {
        if !allowed_slots.contains(slot.as_str()) {
            errors.push(format!("profile selected prohibited creator slot '{slot}'"));
            continue;
        }
        match clothing_by_id.get(item_id.as_str()) {
            Some(item) if item.slot == *slot => {}
            Some(item) => errors.push(format!(
                "starter clothing '{}' belongs to slot '{}' but was equipped in '{}'",
                item_id, item.slot, slot
            )),
            None => errors.push(format!(
                "profile selected unknown starter clothing '{item_id}'"
            )),
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_profile_policy_and_default_profile_are_valid() {
        let catalog = CharacterCreationCatalog::load_from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/characters/character_creation_catalog_v0_1.json"
        ))
        .expect("character creation catalog should load");
        let policy = CharacterProfilePolicy::load_from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/characters/character_profile_policy_v0_1.json"
        ))
        .expect("character profile policy should load");

        assert!(
            validate_character_profile_policy(&policy, &catalog).is_empty(),
            "production character profile policy should be valid"
        );
        let profile = build_default_character_profile(&catalog);
        assert!(
            validate_character_profile_selection(&profile, &catalog, &policy).is_empty(),
            "default character profile should be valid"
        );
    }
}
