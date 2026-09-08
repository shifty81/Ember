use super::*;
use crate::building_recipe::{BuildingFurnishingPlacement, BuildingOpening, BuildingRecipePieceKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{create_dir_all, read_to_string, write},
    path::Path,
};

/// Save-backed state belonging to simulation/persistence. Camera-local cutaway
/// fields intentionally do not exist here.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstancePersistentState {
    pub instance_id: String,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub opening_states: BTreeMap<String, String>,
    #[serde(default)]
    pub furnishing_states: BTreeMap<String, String>,
}

impl BuildingInstancePersistentState {
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            revision: 0,
            opening_states: BTreeMap::new(),
            furnishing_states: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstanceWorldStateFile {
    pub schema: String,
    pub pass: String,
    pub world_id: String,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub upserts: Vec<BuildingInstanceDefinition>,
    #[serde(default)]
    pub removed_instance_ids: Vec<String>,
    #[serde(default)]
    pub instance_states: Vec<BuildingInstancePersistentState>,
}

impl BuildingInstanceRegistry {
    pub fn persistent_state(&self, instance_id: &str) -> Option<&BuildingInstancePersistentState> {
        self.persistent_states.get(instance_id)
    }

    pub fn effective_opening_state<'a>(
        &'a self,
        instance_id: &str,
        opening: &'a BuildingOpening,
    ) -> Option<&'a str> {
        self.persistent_states
            .get(instance_id)
            .and_then(|state| state.opening_states.get(&opening.id))
            .map(String::as_str)
            .or(opening.state.as_deref())
    }

    pub fn effective_furnishing_state<'a>(
        &'a self,
        instance_id: &str,
        furnishing: &'a BuildingFurnishingPlacement,
    ) -> Option<&'a str> {
        self.persistent_states
            .get(instance_id)
            .and_then(|state| state.furnishing_states.get(&furnishing.id))
            .map(String::as_str)
            .or(furnishing.state.as_deref())
    }

    pub fn set_opening_state(
        &mut self,
        instance_id: &str,
        recipe: &BuildingRecipeDefinition,
        opening_id: &str,
        state: impl Into<String>,
    ) -> Result<u64, String> {
        let (_, opening) = recipe
            .opening(opening_id)
            .ok_or_else(|| format!("{} has no opening {}", recipe.id, opening_id))?;
        if self.entry(instance_id).is_none() {
            return Err(format!("unknown BuildingInstance {instance_id}"));
        }
        let state = state.into();
        let persistent = self
            .persistent_states
            .entry(instance_id.to_string())
            .or_insert_with(|| BuildingInstancePersistentState::new(instance_id));
        if opening.state.as_deref() == Some(state.as_str()) {
            persistent.opening_states.remove(opening_id);
        } else {
            persistent.opening_states.insert(opening_id.to_string(), state);
        }
        persistent.revision = persistent.revision.saturating_add(1);
        self.world_revision = self.world_revision.saturating_add(1);
        Ok(persistent.revision)
    }

    pub fn set_furnishing_state(
        &mut self,
        instance_id: &str,
        recipe: &BuildingRecipeDefinition,
        furnishing_id: &str,
        state: impl Into<String>,
    ) -> Result<u64, String> {
        let (_, furnishing) = recipe
            .furnishing(furnishing_id)
            .ok_or_else(|| format!("{} has no furnishing {}", recipe.id, furnishing_id))?;
        if self.entry(instance_id).is_none() {
            return Err(format!("unknown BuildingInstance {instance_id}"));
        }
        let state = state.into();
        let persistent = self
            .persistent_states
            .entry(instance_id.to_string())
            .or_insert_with(|| BuildingInstancePersistentState::new(instance_id));
        if furnishing.state.as_deref() == Some(state.as_str()) {
            persistent.furnishing_states.remove(furnishing_id);
        } else {
            persistent.furnishing_states.insert(furnishing_id.to_string(), state);
        }
        persistent.revision = persistent.revision.saturating_add(1);
        self.world_revision = self.world_revision.saturating_add(1);
        Ok(persistent.revision)
    }

    /// Replaces one instance's authoritative persistent opening-state delta from
    /// a validated host replication snapshot without fabricating camera state or
    /// incrementing local gameplay revisions.
    pub fn apply_replicated_opening_states(
        &mut self,
        instance_id: &str,
        recipe: &BuildingRecipeDefinition,
        revision: u64,
        opening_states: BTreeMap<String, String>,
    ) -> Result<(), String> {
        if self.entry(instance_id).is_none() {
            return Err(format!("unknown BuildingInstance {instance_id}"));
        }
        for (opening_id, value) in &opening_states {
            if recipe.opening(opening_id).is_none() {
                return Err(format!(
                    "replicated state for {instance_id} references unknown opening {opening_id}"
                ));
            }
            if value.trim().is_empty() {
                return Err(format!(
                    "replicated state for {instance_id} opening {opening_id} is empty"
                ));
            }
        }
        let persistent = self
            .persistent_states
            .entry(instance_id.to_string())
            .or_insert_with(|| BuildingInstancePersistentState::new(instance_id));
        persistent.revision = revision;
        persistent.opening_states = opening_states;
        Ok(())
    }

    pub fn apply_replicated_furnishing_states(
        &mut self,
        instance_id: &str,
        recipe: &BuildingRecipeDefinition,
        revision: u64,
        furnishing_states: BTreeMap<String, String>,
    ) -> Result<(), String> {
        if self.entry(instance_id).is_none() {
            return Err(format!("unknown BuildingInstance {instance_id}"));
        }
        for (furnishing_id, value) in &furnishing_states {
            if recipe.furnishing(furnishing_id).is_none() {
                return Err(format!(
                    "replicated state for {instance_id} references unknown furnishing {furnishing_id}"
                ));
            }
            if value.trim().is_empty() {
                return Err(format!(
                    "replicated state for {instance_id} furnishing {furnishing_id} is empty"
                ));
            }
        }
        let persistent = self
            .persistent_states
            .entry(instance_id.to_string())
            .or_insert_with(|| BuildingInstancePersistentState::new(instance_id));
        persistent.revision = revision;
        persistent.furnishing_states = furnishing_states;
        Ok(())
    }

    pub fn visible_pieces_for_instance(
        &self,
        instance: &BuildingInstanceDefinition,
        recipe: &BuildingRecipeDefinition,
        state: BuildingInstanceViewState,
    ) -> Vec<BuildingInstanceVisiblePiece> {
        let mut pieces = instance.visible_pieces(recipe, state);
        for visible in &mut pieces {
            match visible.piece.kind {
                BuildingRecipePieceKind::Opening => {
                    if let Some(opening) = recipe.opening_at(visible.piece.level, visible.piece.tile) {
                        visible.piece.state = self
                            .effective_opening_state(&instance.id, opening)
                            .map(str::to_string);
                    }
                }
                BuildingRecipePieceKind::Furnishing => {
                    let prefix = format!("{}:furnishing:", recipe.id);
                    if let Some(furnishing_id) = visible.piece.id.strip_prefix(&prefix) {
                        if let Some((_, furnishing)) = recipe.furnishing(furnishing_id) {
                            visible.piece.state = self
                                .effective_furnishing_state(&instance.id, furnishing)
                                .map(str::to_string);
                        }
                    }
                }
                _ => {}
            }
        }
        pieces
    }

    pub fn upsert_world_instance(
        &mut self,
        definition: BuildingInstanceDefinition,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<(), String> {
        let errors = definition.validate(recipes);
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.removed_instance_ids.remove(&definition.id);
        self.world_upserts.insert(definition.id.clone(), definition);
        self.world_revision = self.world_revision.saturating_add(1);
        self.rebuild_effective(recipes)
    }

    pub fn remove_world_instance(
        &mut self,
        instance_id: &str,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<(), String> {
        if self.entry(instance_id).is_none() && !self.world_upserts.contains_key(instance_id) {
            return Err(format!("unknown BuildingInstance {instance_id}"));
        }
        self.world_upserts.remove(instance_id);
        self.removed_instance_ids.insert(instance_id.to_string());
        self.persistent_states.remove(instance_id);
        self.world_revision = self.world_revision.saturating_add(1);
        self.rebuild_effective(recipes)
    }

    pub fn clear_world_overlay(&mut self, recipes: &BuildingRecipeRegistry) -> Result<(), String> {
        self.world_upserts.clear();
        self.removed_instance_ids.clear();
        self.persistent_states.clear();
        self.world_revision = 0;
        self.rebuild_effective(recipes)
    }

    pub fn apply_world_state_from_path(
        &mut self,
        path: impl AsRef<Path>,
        expected_world_id: &str,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<bool, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(false);
        }
        let document: BuildingInstanceWorldStateFile = serde_json::from_str(
            &read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", path.display()))?;
        if document.schema != BUILDING_INSTANCE_WORLD_STATE_SCHEMA {
            return Err(format!("{} uses unsupported schema {}", path.display(), document.schema));
        }
        if document.world_id != expected_world_id {
            return Err(format!(
                "{} belongs to world {}, expected {}",
                path.display(), document.world_id, expected_world_id
            ));
        }
        let mut upserts = BTreeMap::new();
        for definition in document.upserts {
            let errors = definition.validate(recipes);
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
            if upserts.insert(definition.id.clone(), definition).is_some() {
                return Err("duplicate BuildingInstance upsert in world state".to_string());
            }
        }
        let removed = document
            .removed_instance_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut states = BTreeMap::new();
        for state in document.instance_states {
            if states.insert(state.instance_id.clone(), state).is_some() {
                return Err("duplicate BuildingInstance persistent state in world state".to_string());
            }
        }
        if upserts.keys().any(|id| removed.contains(id)) {
            return Err("BuildingInstance world state cannot upsert and tombstone the same id".to_string());
        }
        if states.keys().any(|id| removed.contains(id)) {
            return Err("BuildingInstance world state cannot persist state for a tombstoned id".to_string());
        }
        self.world_upserts = upserts;
        self.removed_instance_ids = removed;
        self.persistent_states = states;
        self.world_revision = document.revision;
        self.rebuild_effective(recipes)?;
        for state in self.persistent_states.values() {
            let definition = self
                .entry(&state.instance_id)
                .ok_or_else(|| format!("persistent state references unknown BuildingInstance {}", state.instance_id))?;
            let recipe = recipes
                .entry(&definition.recipe_id)
                .ok_or_else(|| format!("{} references missing recipe {}", definition.id, definition.recipe_id))?;
            for (opening_id, value) in &state.opening_states {
                if recipe.opening(opening_id).is_none() {
                    return Err(format!(
                        "persistent state for {} references unknown opening {}",
                        state.instance_id, opening_id
                    ));
                }
                if value.trim().is_empty() {
                    return Err(format!(
                        "persistent state for {} opening {} has empty state",
                        state.instance_id, opening_id
                    ));
                }
            }
            for (furnishing_id, value) in &state.furnishing_states {
                if recipe.furnishing(furnishing_id).is_none() {
                    return Err(format!(
                        "persistent state for {} references unknown furnishing {}",
                        state.instance_id, furnishing_id
                    ));
                }
                if value.trim().is_empty() {
                    return Err(format!(
                        "persistent state for {} furnishing {} has empty state",
                        state.instance_id, furnishing_id
                    ));
                }
            }
        }
        Ok(true)
    }

    pub fn save_world_state_to_path(
        &self,
        path: impl AsRef<Path>,
        world_id: &str,
    ) -> Result<(), String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        let document = BuildingInstanceWorldStateFile {
            schema: BUILDING_INSTANCE_WORLD_STATE_SCHEMA.to_string(),
            pass: BUILDING_INSTANCE_WORLD_STATE_PASS.to_string(),
            world_id: world_id.to_string(),
            revision: self.world_revision,
            upserts: self.world_upserts.values().cloned().collect(),
            removed_instance_ids: self.removed_instance_ids.iter().cloned().collect(),
            instance_states: self.persistent_states.values().cloned().collect(),
        };
        let payload = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
        write(path, format!("{payload}\n")).map_err(|error| format!("{}: {error}", path.display()))
    }
}
