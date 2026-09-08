use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::read_to_string;

pub const CHARACTER_SPRITE_REGISTRY_PATH: &str =
    "content/characters/character_sprite_registry_v0_1.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterSpriteRegistry {
    pub schema: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub preview_scale: u32,
    pub direction_rows: HashMap<String, u32>,
    pub animation_columns: HashMap<String, u32>,
    pub layers: Vec<CharacterSpriteLayerAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterSpriteLayerAsset {
    pub option_id: String,
    pub asset_role: String,
    pub texture_path: String,
    pub z_order: i32,
    #[serde(default)]
    pub palette_channel: Option<String>,
    #[serde(default)]
    pub transparent: bool,
}

impl CharacterSpriteRegistry {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }

    pub fn asset_for_option(&self, option_id: &str) -> Option<&CharacterSpriteLayerAsset> {
        self.layers
            .iter()
            .find(|asset| asset.option_id == option_id)
    }
}

pub fn validate_character_sprite_registry(registry: &CharacterSpriteRegistry) -> Vec<String> {
    let mut errors = Vec::new();
    if registry.frame_width != 64 || registry.frame_height != 64 {
        errors.push(
            "character sprite layers must use the normalized 64x64 LPC frame contract".to_string(),
        );
    }
    for direction in ["down", "left", "right", "up"] {
        if !registry.direction_rows.contains_key(direction) {
            errors.push(format!("missing direction row '{direction}'"));
        }
    }
    for animation in ["idle", "walk"] {
        if !registry.animation_columns.contains_key(animation) {
            errors.push(format!("missing animation column '{animation}'"));
        }
    }
    let mut option_ids = HashSet::new();
    for asset in &registry.layers {
        if !option_ids.insert(asset.option_id.clone()) {
            errors.push(format!("duplicate sprite option id '{}'", asset.option_id));
        }
        if !asset.transparent && asset.asset_role.contains("none") {
            errors.push(format!(
                "none-role '{}' must be transparent",
                asset.asset_role
            ));
        }
        if asset.texture_path.contains("..") || !asset.texture_path.ends_with(".png") {
            errors.push(format!(
                "invalid character texture path '{}'",
                asset.texture_path
            ));
        }
    }
    errors
}
