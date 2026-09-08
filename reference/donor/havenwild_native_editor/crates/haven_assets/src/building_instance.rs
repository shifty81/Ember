use crate::building_recipe::{
    BuildingOpeningKind, BuildingRecipeDefinition, BuildingRecipePiece, BuildingRecipePieceKind,
    BuildingRecipeRegistry, BuildingWallEdge,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{read_to_string},
    path::{Path, PathBuf},
};

mod authoring;
mod interior;
mod persistence;
mod worldgen;

pub use persistence::{BuildingInstancePersistentState, BuildingInstanceWorldStateFile};

pub const BUILDING_INSTANCE_CATALOG_PATH: &str =
    "content/buildings/building_instance_catalog_v1.json";
pub const BUILDING_INSTANCE_SCHEMA: &str = "havenwild.building_instance.v1";
pub const BUILDING_INSTANCE_CATALOG_SCHEMA: &str = "havenwild.building_instance_catalog.v1";
pub const BUILDING_INSTANCE_WORLD_STATE_SCHEMA: &str =
    "havenwild.building_instance_world_state.v1";
pub const BUILDING_INSTANCE_WORLD_STATE_PASS: &str = "167Z109W46C";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingInstanceStatus {
    Candidate,
    Certified,
    Deprecated,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildingInstanceOrigin {
    #[default]
    Authored,
    Worldgen,
    PlayerBuilt,
    Diagnostic,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildingPlacementSpace {
    #[default]
    SceneLocal,
    ContinuousSurface,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstanceCatalogEntry {
    pub id: String,
    pub path: String,
    pub status: BuildingInstanceStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstanceCatalogFile {
    pub schema: String,
    pub pass: String,
    pub authority: String,
    pub entries: Vec<BuildingInstanceCatalogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstanceDefinition {
    pub schema: String,
    pub id: String,
    pub recipe_id: String,
    /// For scene-local placement this is authoritative. For continuous-surface
    /// placement this records the authored/source partition used for lineage.
    pub scene_id: String,
    /// Bottom-world footprint origin in scene-local tiles. Continuous-surface
    /// instances resolve this field per active storage partition at runtime.
    pub anchor_tile: [i32; 2],
    #[serde(default)]
    pub initial_level: i32,
    #[serde(default)]
    pub diagnostic_only: bool,
    #[serde(default)]
    pub origin: BuildingInstanceOrigin,
    #[serde(default)]
    pub placement_space: BuildingPlacementSpace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_region_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_anchor_tile: Option<[i32; 2]>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInstanceViewState {
    pub active_level: i32,
    pub inside: bool,
}

impl BuildingInstanceViewState {
    pub fn for_definition(definition: &BuildingInstanceDefinition) -> Self {
        Self {
            active_level: definition.initial_level,
            inside: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingInstanceVisiblePiece {
    pub instance_id: String,
    pub world_tile: [i32; 2],
    pub piece: BuildingRecipePiece,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingConnectorTraversal {
    pub instance_id: String,
    pub connector_id: String,
    pub from_level: i32,
    pub to_level: i32,
    pub destination_world_tile: [i32; 2],
}

impl BuildingInstanceDefinition {
    pub fn validate(&self, recipes: &BuildingRecipeRegistry) -> Vec<String> {
        let mut errors = Vec::new();
        if self.schema != BUILDING_INSTANCE_SCHEMA {
            errors.push(format!("{} uses unsupported schema {}", self.id, self.schema));
        }
        if self.id.trim().is_empty() || self.scene_id.trim().is_empty() {
            errors.push("building instance id/scene_id must not be empty".to_string());
        }
        if self.placement_space == BuildingPlacementSpace::ContinuousSurface
            && self.global_anchor_tile.is_none()
        {
            errors.push(format!(
                "{} continuous-surface placement requires globalAnchorTile",
                self.id
            ));
        }
        let Some(recipe) = recipes.entry(&self.recipe_id) else {
            errors.push(format!("{} references unknown BuildingRecipe {}", self.id, self.recipe_id));
            return errors;
        };
        if recipe.level(self.initial_level).is_none() {
            errors.push(format!(
                "{} initial level {} does not exist in {}",
                self.id, self.initial_level, self.recipe_id
            ));
        }
        errors
    }

    pub fn authoritative_anchor_tile(&self) -> [i32; 2] {
        if self.placement_space == BuildingPlacementSpace::ContinuousSurface {
            self.global_anchor_tile.unwrap_or(self.anchor_tile)
        } else {
            self.anchor_tile
        }
    }

    pub fn world_tile(&self, local_tile: [i32; 2]) -> [i32; 2] {
        [
            self.anchor_tile[0] + local_tile[0],
            self.anchor_tile[1] + local_tile[1],
        ]
    }

    pub fn local_tile(&self, world_tile: [i32; 2]) -> [i32; 2] {
        [
            world_tile[0] - self.anchor_tile[0],
            world_tile[1] - self.anchor_tile[1],
        ]
    }

    pub fn footprint_contains_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        world_tile: [i32; 2],
    ) -> bool {
        let local = self.local_tile(world_tile);
        local[0] >= 0
            && local[1] >= 0
            && local[0] < recipe.footprint[0] as i32
            && local[1] < recipe.footprint[1] as i32
    }

    pub fn room_contains_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        level: i32,
        world_tile: [i32; 2],
    ) -> bool {
        let Some(level) = recipe.level(level) else {
            return false;
        };
        let local = self.local_tile(world_tile);
        level.rooms.iter().any(|room| rect_contains(room.rect, local))
    }

    pub fn wall_blocks_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        level_number: i32,
        world_tile: [i32; 2],
    ) -> bool {
        let Some(level) = recipe.level(level_number) else {
            return false;
        };
        let local = self.local_tile(world_tile);
        level.wall_runs.iter().any(|wall| {
            if !wall.blocks_movement || !wall_contains_tile(wall.edge, wall.start, wall.length, local) {
                return false;
            }
            let passable_opening = level.openings.iter().any(|opening| {
                opening.tile == local
                    && opening.edge == wall.edge
                    && matches!(opening.kind, BuildingOpeningKind::Door | BuildingOpeningKind::Archway)
            });
            !passable_opening
        })
    }

    pub fn refresh_view_from_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        state: &mut BuildingInstanceViewState,
        world_tile: [i32; 2],
    ) {
        state.inside = self.room_contains_world_tile(recipe, state.active_level, world_tile);
        if !self.footprint_contains_world_tile(recipe, world_tile) && state.active_level != recipe.default_level {
            state.active_level = recipe.default_level;
            state.inside = false;
        }
    }

    pub fn visible_pieces(
        &self,
        recipe: &BuildingRecipeDefinition,
        state: BuildingInstanceViewState,
    ) -> Vec<BuildingInstanceVisiblePiece> {
        let mut result = recipe
            .materialize_structural_pieces()
            .into_iter()
            .filter(|piece| piece_visible(recipe, piece, state))
            .map(|piece| BuildingInstanceVisiblePiece {
                instance_id: self.id.clone(),
                world_tile: self.world_tile(piece.tile),
                piece,
            })
            .collect::<Vec<_>>();
        if state.inside {
            for connector in &recipe.connectors {
                if connector.reversible && connector.to_level == state.active_level {
                    let piece = BuildingRecipePiece {
                        id: format!("{}:connector:{}:return", recipe.id, connector.id),
                        kind: BuildingRecipePieceKind::Connector,
                        asset_id: connector.asset_id.clone(),
                        level: connector.to_level,
                        tile: connector.to_tile,
                        state: None,
                        occlusion_group: None,
                    };
                    result.push(BuildingInstanceVisiblePiece {
                        instance_id: self.id.clone(),
                        world_tile: self.world_tile(piece.tile),
                        piece,
                    });
                }
            }
        }
        result
    }

    pub fn connector_traversal_at(
        &self,
        recipe: &BuildingRecipeDefinition,
        state: BuildingInstanceViewState,
        world_tile: [i32; 2],
    ) -> Option<BuildingConnectorTraversal> {
        let local = self.local_tile(world_tile);
        for connector in &recipe.connectors {
            if connector.from_level == state.active_level && connector.from_tile == local {
                return Some(BuildingConnectorTraversal {
                    instance_id: self.id.clone(),
                    connector_id: connector.id.clone(),
                    from_level: connector.from_level,
                    to_level: connector.to_level,
                    destination_world_tile: self.world_tile(connector.to_tile),
                });
            }
            if connector.reversible
                && connector.to_level == state.active_level
                && connector.to_tile == local
            {
                return Some(BuildingConnectorTraversal {
                    instance_id: self.id.clone(),
                    connector_id: connector.id.clone(),
                    from_level: connector.to_level,
                    to_level: connector.from_level,
                    destination_world_tile: self.world_tile(connector.from_tile),
                });
            }
        }
        None
    }
}

fn piece_visible(
    recipe: &BuildingRecipeDefinition,
    piece: &BuildingRecipePiece,
    state: BuildingInstanceViewState,
) -> bool {
    match piece.kind {
        BuildingRecipePieceKind::Roof => !state.inside,
        BuildingRecipePieceKind::Floor | BuildingRecipePieceKind::Connector | BuildingRecipePieceKind::Furnishing => {
            state.inside && piece.level == state.active_level
        }
        BuildingRecipePieceKind::Wall | BuildingRecipePieceKind::Opening => {
            let interior_only = piece
                .occlusion_group
                .as_deref()
                .is_some_and(|group| group.starts_with("building.wall_interior."));
            if state.inside {
                piece.level == state.active_level
                    && !piece
                        .occlusion_group
                        .as_deref()
                        .is_some_and(|group| is_camera_cutaway_group(recipe, group, state.active_level))
            } else {
                piece.level == recipe.default_level && !interior_only
            }
        }
    }
}

fn is_camera_cutaway_group(recipe: &BuildingRecipeDefinition, group: &str, level: i32) -> bool {
    recipe.cutaway_groups.iter().any(|entry| {
        entry.id == group
            && entry.camera_local
            && entry.visibility_role == "wall_occluder"
            && (entry.levels.is_empty() || entry.levels.contains(&level))
    })
}

fn wall_contains_tile(
    edge: BuildingWallEdge,
    start: [i32; 2],
    length: u32,
    tile: [i32; 2],
) -> bool {
    let length = length as i32;
    match edge {
        BuildingWallEdge::North | BuildingWallEdge::South | BuildingWallEdge::Interior => {
            tile[1] == start[1] && tile[0] >= start[0] && tile[0] < start[0] + length
        }
        BuildingWallEdge::East | BuildingWallEdge::West => {
            tile[0] == start[0] && tile[1] >= start[1] && tile[1] < start[1] + length
        }
    }
}

fn rect_contains(rect: [i32; 4], tile: [i32; 2]) -> bool {
    let [x, y, w, h] = rect;
    tile[0] >= x && tile[1] >= y && tile[0] < x + w && tile[1] < y + h
}

fn rects_intersect(left: [i32; 4], right: [i32; 4]) -> bool {
    left[0] < right[0] + right[2]
        && left[0] + left[2] > right[0]
        && left[1] < right[1] + right[3]
        && left[1] + left[3] > right[1]
}

#[derive(Clone, Debug, Default)]
pub struct BuildingInstanceRegistry {
    entries: Vec<BuildingInstanceDefinition>,
    by_id: BTreeMap<String, usize>,
    by_scene: BTreeMap<String, Vec<usize>>,
    authored_base: BTreeMap<String, BuildingInstanceDefinition>,
    authored_catalog: BTreeMap<String, BuildingInstanceCatalogEntry>,
    generated_base: BTreeMap<String, BuildingInstanceDefinition>,
    world_upserts: BTreeMap<String, BuildingInstanceDefinition>,
    removed_instance_ids: BTreeSet<String>,
    persistent_states: BTreeMap<String, BuildingInstancePersistentState>,
    pending_deleted_authored_paths: BTreeSet<PathBuf>,
    world_revision: u64,
}

impl BuildingInstanceRegistry {
    pub fn load_from_project_root(
        root: impl AsRef<Path>,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<Self, String> {
        let root = root.as_ref();
        let catalog_path = root.join(BUILDING_INSTANCE_CATALOG_PATH);
        let catalog: BuildingInstanceCatalogFile = serde_json::from_str(
            &read_to_string(&catalog_path)
                .map_err(|error| format!("{}: {error}", catalog_path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", catalog_path.display()))?;
        if catalog.schema != BUILDING_INSTANCE_CATALOG_SCHEMA {
            return Err(format!(
                "{} uses unsupported schema {}",
                catalog_path.display(), catalog.schema
            ));
        }
        if catalog.authority != "BuildingInstanceRegistry" {
            return Err(format!(
                "{} must declare BuildingInstanceRegistry authority",
                catalog_path.display()
            ));
        }

        let mut registry = Self::default();
        for item in catalog.entries {
            if item.status == BuildingInstanceStatus::Deprecated {
                registry.authored_catalog.insert(item.id.clone(), item);
                continue;
            }
            let relative = safe_relative_path(&item.path)?;
            let path = root.join(&relative);
            let mut definition: BuildingInstanceDefinition = serde_json::from_str(
                &read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?,
            )
            .map_err(|error| format!("{}: {error}", path.display()))?;
            if definition.id != item.id {
                return Err(format!(
                    "{} catalogs {}, but instance declares {}",
                    path.display(), item.id, definition.id
                ));
            }
            if definition.diagnostic_only {
                definition.origin = BuildingInstanceOrigin::Diagnostic;
            }
            let errors = definition.validate(recipes);
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
            if registry.authored_base.insert(definition.id.clone(), definition).is_some() {
                return Err(format!("duplicate BuildingInstance id {}", item.id));
            }
            registry.authored_catalog.insert(item.id.clone(), item);
        }
        registry.rebuild_effective(recipes)?;
        Ok(registry)
    }

    pub fn entries(&self) -> &[BuildingInstanceDefinition] {
        &self.entries
    }

    pub fn entry(&self, id: &str) -> Option<&BuildingInstanceDefinition> {
        self.by_id.get(id).and_then(|index| self.entries.get(*index))
    }

    pub fn for_scene(&self, scene_id: &str) -> impl Iterator<Item = &BuildingInstanceDefinition> {
        self.by_scene
            .get(scene_id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.entries.get(*index))
    }

    pub fn resolved_for_scene(
        &self,
        scene_id: &str,
        surface_region: Option<&str>,
        scene_global_origin: Option<[i32; 2]>,
        scene_size: [i32; 2],
        recipes: &BuildingRecipeRegistry,
    ) -> Vec<BuildingInstanceDefinition> {
        let mut result = Vec::new();
        for definition in &self.entries {
            match definition.placement_space {
                BuildingPlacementSpace::SceneLocal => {
                    if definition.scene_id == scene_id {
                        result.push(definition.clone());
                    }
                }
                BuildingPlacementSpace::ContinuousSurface => {
                    let Some(origin) = scene_global_origin else { continue; };
                    if definition.surface_region_id.as_deref() != surface_region {
                        continue;
                    }
                    let Some(global_anchor) = definition.global_anchor_tile else { continue; };
                    let Some(recipe) = recipes.entry(&definition.recipe_id) else { continue; };
                    let building_rect = [
                        global_anchor[0],
                        global_anchor[1],
                        recipe.footprint[0] as i32,
                        recipe.footprint[1] as i32,
                    ];
                    let scene_rect = [origin[0], origin[1], scene_size[0], scene_size[1]];
                    if !rects_intersect(building_rect, scene_rect) {
                        continue;
                    }
                    let mut resolved = definition.clone();
                    resolved.scene_id = scene_id.to_string();
                    resolved.anchor_tile = [
                        global_anchor[0] - origin[0],
                        global_anchor[1] - origin[1],
                    ];
                    result.push(resolved);
                }
            }
        }
        result
    }

    pub fn initial_view_states(&self) -> BTreeMap<String, BuildingInstanceViewState> {
        self.entries
            .iter()
            .map(|definition| {
                (
                    definition.id.clone(),
                    BuildingInstanceViewState::for_definition(definition),
                )
            })
            .collect()
    }

    pub fn validate_unique_scene_placements(&self) -> Result<(), Vec<String>> {
        let mut seen = BTreeSet::new();
        let mut errors = Vec::new();
        for definition in &self.entries {
            let anchor = definition.authoritative_anchor_tile();
            let region = if definition.placement_space == BuildingPlacementSpace::ContinuousSurface {
                definition.surface_region_id.as_deref().unwrap_or("")
            } else {
                definition.scene_id.as_str()
            };
            let key = (
                definition.placement_space,
                region,
                anchor[0],
                anchor[1],
                definition.recipe_id.as_str(),
            );
            if !seen.insert(key) {
                errors.push(format!(
                    "duplicate building placement {} at {:?} in {}",
                    definition.recipe_id, anchor, region
                ));
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    fn rebuild_effective(&mut self, recipes: &BuildingRecipeRegistry) -> Result<(), String> {
        let mut merged = self.authored_base.clone();
        for (id, definition) in &self.generated_base {
            if merged.insert(id.clone(), definition.clone()).is_some() {
                return Err(format!("worldgen BuildingInstance id {id} collides with authored authority"));
            }
        }
        for id in &self.removed_instance_ids {
            merged.remove(id);
        }
        for (id, definition) in &self.world_upserts {
            merged.insert(id.clone(), definition.clone());
        }
        let mut entries = merged.into_values().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        let mut by_id = BTreeMap::new();
        let mut by_scene: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, definition) in entries.iter().enumerate() {
            let errors = definition.validate(recipes);
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
            if by_id.insert(definition.id.clone(), index).is_some() {
                return Err(format!("duplicate BuildingInstance id {}", definition.id));
            }
            if definition.placement_space == BuildingPlacementSpace::SceneLocal {
                by_scene.entry(definition.scene_id.clone()).or_default().push(index);
            }
        }
        self.entries = entries;
        self.by_id = by_id;
        self.by_scene = by_scene;
        Ok(())
    }
}

fn safe_relative_path(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(format!(
            "building instance path must be repository-relative: {value}"
        ));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn w46b_instance_catalog_loads_same_world_prototype() {
        let recipes = BuildingRecipeRegistry::load_from_project_root(project_root())
            .expect("W46 BuildingRecipe catalog should load");
        let instances = BuildingInstanceRegistry::load_from_project_root(project_root(), &recipes)
            .expect("W46B BuildingInstance catalog should load");
        let instance = instances
            .entry("havenwild.acceptance.three_level_house")
            .expect("diagnostic BuildingInstance should exist");
        assert_eq!(instance.scene_id, "building_instance_acceptance");
        assert_eq!(instance.initial_level, 0);
        instances
            .validate_unique_scene_placements()
            .expect("building placements should be unique");
    }

    #[test]
    fn w46b_connector_traversal_stays_in_same_instance() {
        let recipes = BuildingRecipeRegistry::load_from_project_root(project_root()).unwrap();
        let instances = BuildingInstanceRegistry::load_from_project_root(project_root(), &recipes)
            .unwrap();
        let recipe = recipes.entry("havenwild.prototype.three_level_house").unwrap();
        let instance = instances.entry("havenwild.acceptance.three_level_house").unwrap();
        let state = BuildingInstanceViewState {
            active_level: 0,
            inside: true,
        };
        let upstairs = instance
            .connector_traversal_at(recipe, state, instance.world_tile([6, 4]))
            .expect("ground-floor upstairs connector should resolve");
        assert_eq!((upstairs.from_level, upstairs.to_level), (0, 1));
        assert_eq!(upstairs.instance_id, instance.id);
    }

    #[test]
    fn w46c_camera_state_is_not_part_of_persistent_state_schema() {
        let state = BuildingInstancePersistentState::new("example");
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("inside"));
        assert!(!json.contains("activeLevel"));
    }

    #[test]
    fn w46c_opening_override_round_trips_as_delta() {
        let recipes = BuildingRecipeRegistry::load_from_project_root(project_root()).unwrap();
        let mut instances = BuildingInstanceRegistry::load_from_project_root(project_root(), &recipes)
            .unwrap();
        let recipe = recipes.entry("havenwild.prototype.three_level_house").unwrap();
        let (_, door) = recipe.opening("front_door").unwrap();
        assert_eq!(instances.effective_opening_state("havenwild.acceptance.three_level_house", door), Some("open_left"));
        instances
            .set_opening_state("havenwild.acceptance.three_level_house", recipe, "front_door", "closed")
            .unwrap();
        assert_eq!(instances.effective_opening_state("havenwild.acceptance.three_level_house", door), Some("closed"));
    }
}
