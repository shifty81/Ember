use super::*;
use std::{
    fs::{create_dir_all, remove_file, write},
    path::{Path, PathBuf},
};

impl BuildingInstanceRegistry {
    pub fn place_authored_instance(
        &mut self,
        mut definition: BuildingInstanceDefinition,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<String, String> {
        if definition.diagnostic_only {
            return Err(
                "diagnostic-only BuildingInstances cannot be created through ordinary authoring"
                    .to_string(),
            );
        }
        definition.origin = BuildingInstanceOrigin::Authored;
        if definition.id.trim().is_empty() {
            definition.id = self.next_authored_id(&definition.scene_id, &definition.recipe_id);
        }
        if self.entry(&definition.id).is_some() || self.authored_base.contains_key(&definition.id) {
            return Err(format!("BuildingInstance id {} already exists", definition.id));
        }
        let errors = definition.validate(recipes);
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        let id = definition.id.clone();
        let path = authored_instance_relative_path(&id);
        self.authored_catalog.insert(
            id.clone(),
            BuildingInstanceCatalogEntry {
                id: id.clone(),
                path: path_string(&path),
                status: BuildingInstanceStatus::Candidate,
            },
        );
        self.authored_base.insert(id.clone(), definition);
        self.rebuild_effective(recipes)?;
        Ok(id)
    }

    pub fn move_authored_instance(
        &mut self,
        instance_id: &str,
        anchor_tile: [i32; 2],
        global_anchor_tile: Option<[i32; 2]>,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<(), String> {
        let definition = self
            .authored_base
            .get_mut(instance_id)
            .ok_or_else(|| format!("{} is not an authored BuildingInstance", instance_id))?;
        if definition.diagnostic_only {
            return Err(format!("{} is diagnostic-only and cannot be moved", instance_id));
        }
        definition.anchor_tile = anchor_tile;
        if definition.placement_space == BuildingPlacementSpace::ContinuousSurface {
            definition.global_anchor_tile = global_anchor_tile.or(Some(anchor_tile));
        }
        self.rebuild_effective(recipes)
    }

    pub fn delete_authored_instance(
        &mut self,
        instance_id: &str,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<(), String> {
        let definition = self
            .authored_base
            .get(instance_id)
            .ok_or_else(|| format!("{} is not an authored BuildingInstance", instance_id))?;
        if definition.diagnostic_only {
            return Err(format!("{} is diagnostic-only and cannot be deleted", instance_id));
        }
        if let Some(catalog) = self.authored_catalog.remove(instance_id) {
            self.pending_deleted_authored_paths
                .insert(safe_relative_path(&catalog.path)?);
        }
        self.authored_base.remove(instance_id);
        self.persistent_states.remove(instance_id);
        self.rebuild_effective(recipes)
    }

    pub fn save_authored_to_project_root(&mut self, root: impl AsRef<Path>) -> Result<(), String> {
        let root = root.as_ref();
        for (id, definition) in &self.authored_base {
            let Some(entry) = self.authored_catalog.get(id) else {
                return Err(format!("authored BuildingInstance {id} has no catalog path"));
            };
            let relative = safe_relative_path(&entry.path)?;
            let path = root.join(relative);
            if let Some(parent) = path.parent() {
                create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
            }
            let payload = serde_json::to_string_pretty(definition).map_err(|error| error.to_string())?;
            write(&path, format!("{payload}\n"))
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
        let mut entries = self.authored_catalog.values().cloned().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        let catalog = BuildingInstanceCatalogFile {
            schema: BUILDING_INSTANCE_CATALOG_SCHEMA.to_string(),
            pass: BUILDING_INSTANCE_WORLD_STATE_PASS.to_string(),
            authority: "BuildingInstanceRegistry".to_string(),
            entries,
        };
        let catalog_path = root.join(BUILDING_INSTANCE_CATALOG_PATH);
        let payload = serde_json::to_string_pretty(&catalog).map_err(|error| error.to_string())?;
        write(&catalog_path, format!("{payload}\n"))
            .map_err(|error| format!("{}: {error}", catalog_path.display()))?;
        for relative in std::mem::take(&mut self.pending_deleted_authored_paths) {
            let path = root.join(relative);
            if path.exists() {
                remove_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            }
        }
        Ok(())
    }

    fn next_authored_id(&self, scene_id: &str, recipe_id: &str) -> String {
        let scene = slug(scene_id);
        let recipe = slug(recipe_id.rsplit('.').next().unwrap_or(recipe_id));
        for ordinal in 1u32.. {
            let candidate = format!("authored.{scene}.{recipe}.{ordinal:04}");
            if self.entry(&candidate).is_none() && !self.authored_base.contains_key(&candidate) {
                return candidate;
            }
        }
        unreachable!()
    }
}

fn authored_instance_relative_path(id: &str) -> PathBuf {
    PathBuf::from("content/buildings/instances").join(format!("{}_v1.json", slug(id)))
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
