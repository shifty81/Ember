use crate::{CharacterProgression, CharacterVitalsState};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_dir, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CHARACTER_PROFILE_SCHEMA: &str = "havenwild.character_profile.v1";
pub const CHARACTER_WORLD_LINK_SCHEMA: &str = "havenwild.character_world_link.v1";
pub const MAX_PERSISTENT_CHARACTERS: usize = 5;

/// Save-compatible character recipe envelope.
///
/// The full typed LPC recipe authority is normalized separately. Keeping the
/// persisted payload as JSON here prevents haven_save from depending on an
/// absent haven_core type and preserves existing/future recipe fields during
/// migration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct CharacterRecipe(pub serde_json::Value);

impl CharacterRecipe {
    pub fn validate(&self) -> Result<(), String> {
        let object = self
            .0
            .as_object()
            .ok_or_else(|| "character recipe must be a JSON object".to_string())?;

        if let Some(schema) = object.get("schema") {
            let schema = schema
                .as_str()
                .ok_or_else(|| "character recipe schema must be a string".to_string())?;
            if schema.trim().is_empty() {
                return Err("character recipe schema must not be empty".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldSaveId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableAssetRef {
    pub pack_id: String,
    pub category: String,
    pub asset_id: String,
    pub source_id: String,
    #[serde(default)]
    pub variant_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterAppearanceLayer {
    pub slot: String,
    pub asset: PortableAssetRef,
    #[serde(default)]
    pub palette_id: Option<String>,
    #[serde(default)]
    pub tint_rgba: Option<[u8; 4]>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterAppearance {
    pub body_profile: String,
    pub animation_profile: String,
    pub portrait_profile: String,
    #[serde(default)]
    pub layers: Vec<CharacterAppearanceLayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterProfile {
    pub schema: String,
    pub character_id: CharacterId,
    pub display_name: String,
    pub appearance: CharacterAppearance,
    #[serde(default)]
    pub character_recipe: Option<CharacterRecipe>,
    #[serde(default)]
    pub progression: CharacterProgression,
    #[serde(default)]
    pub vitals: CharacterVitalsState,
    #[serde(default)]
    pub equipment: Vec<PortableAssetRef>,
    #[serde(default)]
    pub portable_inventory: Vec<String>,
    pub created_unix_seconds: u64,
    pub last_played_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterWorldLink {
    pub schema: String,
    pub character_id: CharacterId,
    pub world_id: WorldSaveId,
    #[serde(default)]
    pub world_position: [i32; 2],
    #[serde(default)]
    pub world_scene: String,
    #[serde(default)]
    pub world_reputation: std::collections::BTreeMap<String, i32>,
    #[serde(default)]
    pub world_relationships: std::collections::BTreeMap<String, i32>,
    #[serde(default)]
    pub world_quest_flags: Vec<String>,
    pub last_played_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterProfileStore {
    pub root: PathBuf,
}

impl CharacterId {
    pub fn new_unique() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(format!("character_{nanos:032x}"))
    }
}

impl WorldSaveId {
    pub fn new_unique() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(format!("world_{nanos:032x}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        let value = self.0.trim();
        if !value.starts_with("world_") {
            return Err("world id must begin with world_".to_string());
        }
        if value.len() > 80
            || !value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return Err("world id contains unsafe path characters".to_string());
        }
        Ok(())
    }
}

impl CharacterProfile {
    pub fn new(display_name: impl Into<String>, appearance: CharacterAppearance) -> Self {
        let now = unix_seconds_now();
        Self {
            schema: CHARACTER_PROFILE_SCHEMA.to_string(),
            character_id: CharacterId::new_unique(),
            display_name: display_name.into(),
            appearance,
            character_recipe: None,
            progression: CharacterProgression::default(),
            vitals: CharacterVitalsState::default(),
            equipment: Vec::new(),
            portable_inventory: Vec::new(),
            created_unix_seconds: now,
            last_played_unix_seconds: now,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CHARACTER_PROFILE_SCHEMA {
            return Err(format!("unsupported character schema {}", self.schema));
        }
        if self.character_id.0.trim().is_empty() {
            return Err("character id must not be empty".to_string());
        }
        if self.display_name.trim().is_empty() {
            return Err("character display name must not be empty".to_string());
        }
        if self
            .appearance
            .layers
            .iter()
            .filter(|layer| layer.slot == "body/base" && layer.enabled)
            .count()
            != 1
        {
            return Err(
                "character appearance must contain exactly one enabled body/base layer".to_string(),
            );
        }
        if let Some(recipe) = &self.character_recipe {
            recipe.validate()?;
        }
        self.progression.validate(None)?;
        self.vitals.validate()?;
        Ok(())
    }
}

impl CharacterProfileStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure(&self) -> Result<(), String> {
        create_dir_all(&self.root).map_err(|error| error.to_string())
    }

    pub fn scan(&self) -> Result<Vec<CharacterProfile>, String> {
        self.ensure()?;
        let mut profiles = Vec::new();
        for entry in read_dir(&self.root).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path().join("profile.json");
            if path.is_file() {
                profiles.push(load_character_profile(&path)?);
            }
        }
        profiles.sort_by(|left, right| {
            right
                .last_played_unix_seconds
                .cmp(&left.last_played_unix_seconds)
                .then_with(|| left.character_id.cmp(&right.character_id))
        });
        Ok(profiles)
    }

    pub fn create(&self, profile: &CharacterProfile) -> Result<PathBuf, String> {
        profile.validate()?;
        let profiles = self.scan()?;
        if profiles.len() >= MAX_PERSISTENT_CHARACTERS {
            return Err(format!(
                "character limit reached ({MAX_PERSISTENT_CHARACTERS})"
            ));
        }
        let directory = self.root.join(&profile.character_id.0);
        if directory.exists() {
            return Err(format!(
                "character {} already exists",
                profile.character_id.0
            ));
        }
        create_dir_all(&directory).map_err(|error| error.to_string())?;
        let path = directory.join("profile.json");
        save_json_atomic(&path, profile)?;
        Ok(path)
    }

    pub fn update(&self, profile: &CharacterProfile) -> Result<(), String> {
        profile.validate()?;
        let path = self.root.join(&profile.character_id.0).join("profile.json");
        if !path.is_file() {
            return Err(format!(
                "character {} does not exist",
                profile.character_id.0
            ));
        }
        save_json_atomic(&path, profile)
    }

    pub fn delete(&self, character_id: &CharacterId) -> Result<(), String> {
        let directory = self.root.join(&character_id.0);
        if directory.exists() {
            remove_dir_all(directory).map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

pub fn load_character_profile(path: &Path) -> Result<CharacterProfile, String> {
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let profile: CharacterProfile =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    profile.validate()?;
    Ok(profile)
}

pub fn character_world_link_path(world_root: &Path, character_id: &CharacterId) -> PathBuf {
    world_root
        .join("characters")
        .join(format!("{}.json", character_id.0))
}

pub fn save_character_world_link(
    world_root: &Path,
    link: &CharacterWorldLink,
) -> Result<(), String> {
    if link.schema != CHARACTER_WORLD_LINK_SCHEMA {
        return Err("unsupported character-world link schema".to_string());
    }
    save_json_atomic(
        &character_world_link_path(world_root, &link.character_id),
        link,
    )
}

pub fn load_character_world_link(
    world_root: &Path,
    character_id: &CharacterId,
) -> Result<CharacterWorldLink, String> {
    let path = character_world_link_path(world_root, character_id);
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let link: CharacterWorldLink = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    if link.schema != CHARACTER_WORLD_LINK_SCHEMA {
        return Err("unsupported character-world link schema".to_string());
    }
    Ok(link)
}

impl CharacterWorldLink {
    pub fn new(character_id: CharacterId, world_id: WorldSaveId) -> Self {
        Self {
            schema: CHARACTER_WORLD_LINK_SCHEMA.to_string(),
            character_id,
            world_id,
            world_position: [0, 0],
            world_scene: String::new(),
            world_reputation: Default::default(),
            world_relationships: Default::default(),
            world_quest_flags: Vec::new(),
            last_played_unix_seconds: unix_seconds_now(),
        }
    }

    pub fn touch(&mut self) {
        self.last_played_unix_seconds = unix_seconds_now();
    }
}

pub fn scan_unlimited_world_saves(root: &Path) -> Result<Vec<WorldSaveId>, String> {
    create_dir_all(root).map_err(|error| error.to_string())?;
    let mut worlds = Vec::new();
    for entry in read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("world_") {
                worlds.push(WorldSaveId(name));
            }
        }
    }
    worlds.sort();
    Ok(worlds)
}

fn save_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    let payload = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    write(&temporary, payload).map_err(|error| error.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance() -> CharacterAppearance {
        CharacterAppearance {
            body_profile: "lpc.standard".to_string(),
            animation_profile: "lpc.eight_direction".to_string(),
            portrait_profile: "lpc.generated".to_string(),
            layers: vec![CharacterAppearanceLayer {
                slot: "body/base".to_string(),
                asset: PortableAssetRef {
                    pack_id: "havenwild_characters".to_string(),
                    category: "character".to_string(),
                    asset_id: "body_base".to_string(),
                    source_id: "lpc_character_dependency".to_string(),
                    variant_id: None,
                },
                palette_id: None,
                tint_rgba: None,
                enabled: true,
            }],
        }
    }

    #[test]
    fn profile_requires_one_base_body() {
        let profile = CharacterProfile::new("Aster", appearance());
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn persistent_character_limit_is_five() {
        assert_eq!(MAX_PERSISTENT_CHARACTERS, 5);
    }
}
