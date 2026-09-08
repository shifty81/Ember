use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::read_to_string;

pub const CHARACTER_CREATION_CATALOG_PATH: &str =
    "content/characters/character_creation_catalog_v0_1.json";

/// The creation wardrobe is intentionally broad enough to offer meaningful
/// visual choice while remaining much smaller than the gameplay-acquired LPC
/// clothing catalog. Keep this aligned with the production content validator.
pub const MAX_STARTER_CLOTHING_OPTIONS: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterCreationCatalog {
    pub schema: String,
    pub design_rule: String,
    pub appearance_categories: Vec<AppearanceCategory>,
    pub starter_clothing: Vec<StarterClothingItem>,
    pub prohibited_initial_categories: Vec<String>,
    pub required_animation_aliases: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppearanceCategory {
    pub id: String,
    pub label: String,
    pub options: Vec<AppearanceOption>,
    #[serde(default)]
    pub supports_color: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppearanceOption {
    pub id: String,
    pub label: String,
    pub asset_role: String,
    #[serde(default)]
    pub default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StarterClothingItem {
    pub id: String,
    pub label: String,
    pub slot: String,
    pub asset_role: String,
    #[serde(default)]
    pub supports_color: bool,
    #[serde(default)]
    pub default: bool,
}

impl CharacterCreationCatalog {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }
}

pub fn validate_character_creation_catalog(catalog: &CharacterCreationCatalog) -> Vec<String> {
    let mut errors = Vec::new();
    let mut ids = HashSet::new();

    if catalog.appearance_categories.is_empty() {
        errors.push("character creation must expose base appearance categories".to_string());
    }

    for category in &catalog.appearance_categories {
        if category.options.is_empty() {
            errors.push(format!(
                "appearance category '{}' has no options",
                category.id
            ));
        }
        let defaults = category
            .options
            .iter()
            .filter(|option| option.default)
            .count();
        if defaults > 1 {
            errors.push(format!(
                "appearance category '{}' has multiple defaults",
                category.id
            ));
        }
        for option in &category.options {
            if !ids.insert(option.id.clone()) {
                errors.push(format!(
                    "duplicate character asset option id '{}'",
                    option.id
                ));
            }
        }
    }

    let mut starter_slots = HashSet::new();
    for item in &catalog.starter_clothing {
        let normalized = item.slot.to_ascii_lowercase();
        if normalized.contains("armor")
            || normalized.contains("weapon")
            || normalized.contains("shield")
        {
            errors.push(format!(
                "starter clothing '{}' illegally uses combat slot '{}'",
                item.id, item.slot
            ));
        }
        starter_slots.insert(normalized);
        if !ids.insert(item.id.clone()) {
            errors.push(format!("duplicate character asset option id '{}'", item.id));
        }
    }

    if catalog.starter_clothing.len() > MAX_STARTER_CLOTHING_OPTIONS {
        errors.push(format!(
            "initial creator exposes {} clothing items; keep the generic starter wardrobe bounded (maximum {})",
            catalog.starter_clothing.len(),
            MAX_STARTER_CLOTHING_OPTIONS
        ));
    }

    for item in &catalog.starter_clothing {
        if !matches!(item.slot.as_str(), "top" | "bottom" | "feet") {
            errors.push(format!(
                "starter clothing '{}' uses disallowed creation slot '{}'",
                item.id, item.slot
            ));
        }
    }

    for required in [
        "idle_down",
        "idle_up",
        "idle_left",
        "idle_right",
        "walk_8dir",
    ] {
        if !catalog
            .required_animation_aliases
            .iter()
            .any(|alias| alias == required)
        {
            errors.push(format!(
                "missing required character animation alias '{required}'"
            ));
        }
    }

    for forbidden in [
        "armor",
        "weapons",
        "shields",
        "advanced_outfits",
        "headwear",
    ] {
        if !catalog
            .prohibited_initial_categories
            .iter()
            .any(|category| category == forbidden)
        {
            errors.push(format!(
                "initial creator must explicitly prohibit '{forbidden}'"
            ));
        }
    }

    if !starter_slots.contains("top")
        || !starter_slots.contains("bottom")
        || !starter_slots.contains("feet")
    {
        errors.push(
            "starter clothing must include ordinary top, bottom, and footwear choices".to_string(),
        );
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_catalog_obeys_initial_creator_rules() {
        let catalog = CharacterCreationCatalog::load_from_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/characters/character_creation_catalog_v0_1.json"
        ))
        .expect("character creation catalog should load");
        let errors = validate_character_creation_catalog(&catalog);
        assert!(errors.is_empty(), "character creation errors: {errors:#?}");
    }
}
