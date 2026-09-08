use crate::placeable_asset_registry::{
    PublishedWorldAssetCertification, PublishedWorldAssetRegistry,
};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, fs::read_to_string, path::{Path, PathBuf}};

mod interior;
mod roof;
pub use interior::{
    BuildingFurnishingKind, BuildingFurnishingPlacement, BuildingInteractionSocket,
    BuildingLevelNavigation,
};
pub(crate) use interior::navigation_allows;
use interior::{materialize_level_furnishings, validate_interior_assets, validate_level_interior};

pub const BUILDING_RECIPE_CATALOG_PATH: &str = "content/buildings/building_recipe_catalog_v1.json";
pub const BUILDING_RECIPE_SCHEMA: &str = "havenwild.building_recipe.v1";
pub const BUILDING_RECIPE_CATALOG_SCHEMA: &str = "havenwild.building_recipe_catalog.v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingRecipeStatus {
    Candidate,
    Certified,
    Deprecated,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRecipeCatalogEntry {
    pub id: String,
    pub path: String,
    pub status: BuildingRecipeStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRecipeCatalogFile {
    pub schema: String,
    pub pass: String,
    pub authority: String,
    pub entries: Vec<BuildingRecipeCatalogEntry>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingRecipeKind {
    House,
    Shop,
    Tavern,
    Workshop,
    Barn,
    Civic,
    Utility,
    Diagnostic,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingWallEdge {
    North,
    East,
    South,
    West,
    Interior,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingOpeningKind {
    Door,
    Window,
    Archway,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingConnectorKind {
    Stairs,
    Ladder,
    Ramp,
    Elevator,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingRoofMode {
    FlatNineSlice,
    AuthoredModule,
    TopologyResolver,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRecipeClassification {
    pub kind: BuildingRecipeKind,
    #[serde(default)]
    pub diagnostic_only: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFloorFill {
    pub asset_id: String,
    /// x, y, width, height in building-local tiles.
    pub rect: [i32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingWallRun {
    pub id: String,
    pub edge: BuildingWallEdge,
    /// Building-local anchor tile for this run.
    pub start: [i32; 2],
    pub length: u32,
    #[serde(default)]
    pub visual_asset_id: Option<String>,
    #[serde(default)]
    pub visual_status: String,
    #[serde(default = "default_true")]
    pub blocks_movement: bool,
    #[serde(default)]
    pub occlusion_group: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingOpening {
    pub id: String,
    pub kind: BuildingOpeningKind,
    pub edge: BuildingWallEdge,
    pub tile: [i32; 2],
    pub asset_id: String,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRoomDefinition {
    pub id: String,
    pub label: String,
    /// x, y, width, height in building-local tiles.
    pub rect: [i32; 4],
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevelDefinition {
    pub level: i32,
    pub id: String,
    pub label: String,
    /// x, y, width, height in building-local tiles.
    pub bounds: [i32; 4],
    #[serde(default)]
    pub floor_fills: Vec<BuildingFloorFill>,
    #[serde(default)]
    pub wall_runs: Vec<BuildingWallRun>,
    #[serde(default)]
    pub openings: Vec<BuildingOpening>,
    #[serde(default)]
    pub rooms: Vec<BuildingRoomDefinition>,
    #[serde(default)]
    pub furnishings: Vec<BuildingFurnishingPlacement>,
    #[serde(default)]
    pub navigation: BuildingLevelNavigation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevelConnector {
    pub id: String,
    pub kind: BuildingConnectorKind,
    pub asset_id: String,
    pub from_level: i32,
    pub to_level: i32,
    pub from_tile: [i32; 2],
    pub to_tile: [i32; 2],
    #[serde(default)]
    pub reversible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRoofComponentSet {
    pub field: String,
    pub north_edge: String,
    pub south_edge: String,
    pub west_edge: String,
    pub east_edge: String,
    pub north_west_corner: String,
    pub north_east_corner: String,
    pub south_west_corner: String,
    pub south_east_corner: String,
}

impl BuildingRoofComponentSet {
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("field", self.field.as_str()),
            ("north_eave", self.north_edge.as_str()),
            ("south_eave", self.south_edge.as_str()),
            ("west_rake", self.west_edge.as_str()),
            ("east_rake", self.east_edge.as_str()),
            ("outer_corner_nw", self.north_west_corner.as_str()),
            ("outer_corner_ne", self.north_east_corner.as_str()),
            ("outer_corner_sw", self.south_west_corner.as_str()),
            ("outer_corner_se", self.south_east_corner.as_str()),
        ]
        .into_iter()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRoofAuthoredModulePlacement {
    pub id: String,
    pub asset_id: String,
    pub tile: [i32; 2],
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRoofDefinition {
    pub level: i32,
    pub mode: BuildingRoofMode,
    pub family: String,
    /// x, y, width, height in building-local tiles.
    pub rect: [i32; 4],
    pub occlusion_group: String,
    #[serde(default)]
    pub camera_local_occlusion: bool,
    #[serde(default)]
    pub components: Option<BuildingRoofComponentSet>,
    #[serde(default)]
    pub authored_module_asset_id: Option<String>,
    #[serde(default)]
    pub authored_modules: Vec<BuildingRoofAuthoredModulePlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingCutawayGroup {
    pub id: String,
    pub visibility_role: String,
    #[serde(default)]
    pub levels: Vec<i32>,
    #[serde(default)]
    pub camera_local: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingPersistencePolicy {
    pub interior_policy: String,
    #[serde(default)]
    pub all_levels_authoritative: bool,
    #[serde(default)]
    pub separate_scene: bool,
    #[serde(default)]
    pub portable_instance_identity: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildingRecipePieceKind {
    Floor,
    Wall,
    Opening,
    Connector,
    Furnishing,
    Roof,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRecipePiece {
    pub id: String,
    pub kind: BuildingRecipePieceKind,
    pub asset_id: String,
    pub level: i32,
    pub tile: [i32; 2],
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub occlusion_group: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingRecipeDefinition {
    pub schema: String,
    pub id: String,
    pub version: String,
    pub label: String,
    pub classification: BuildingRecipeClassification,
    pub footprint: [u32; 2],
    pub default_level: i32,
    pub levels: Vec<BuildingLevelDefinition>,
    #[serde(default)]
    pub connectors: Vec<BuildingLevelConnector>,
    pub roof: BuildingRoofDefinition,
    #[serde(default)]
    pub cutaway_groups: Vec<BuildingCutawayGroup>,
    pub persistence: BuildingPersistencePolicy,
}

impl BuildingRecipeDefinition {
    pub fn level(&self, level: i32) -> Option<&BuildingLevelDefinition> {
        self.levels.iter().find(|entry| entry.level == level)
    }

    /// Returns an opening by its recipe-stable id. W46C requires opening ids to be
    /// unique across the complete BuildingRecipe so persistent state never depends
    /// on the currently visible floor.
    pub fn opening(&self, opening_id: &str) -> Option<(i32, &BuildingOpening)> {
        self.levels.iter().find_map(|level| {
            level
                .openings
                .iter()
                .find(|opening| opening.id == opening_id)
                .map(|opening| (level.level, opening))
        })
    }

    pub fn opening_at(&self, level_number: i32, tile: [i32; 2]) -> Option<&BuildingOpening> {
        self.level(level_number)?
            .openings
            .iter()
            .find(|opening| opening.tile == tile)
    }

    /// Deterministically expands the recipe's currently-resolved visual pieces.
    /// Logical wall/collision runs with deferred facing art remain structural authority but
    /// intentionally emit no visual piece until an exact facing-specific component exists.
    pub fn materialize_structural_pieces(&self) -> Vec<BuildingRecipePiece> {
        let mut pieces = Vec::new();
        for level in &self.levels {
            for fill in &level.floor_fills {
                let [x, y, w, h] = fill.rect;
                for dy in 0..h {
                    for dx in 0..w {
                        pieces.push(BuildingRecipePiece {
                            id: format!("{}:floor:{}:{}:{}", self.id, level.id, x + dx, y + dy),
                            kind: BuildingRecipePieceKind::Floor,
                            asset_id: fill.asset_id.clone(),
                            level: level.level,
                            tile: [x + dx, y + dy],
                            state: None,
                            occlusion_group: Some("building.level_surface".to_string()),
                        });
                    }
                }
            }
            for wall in &level.wall_runs {
                let Some(asset_id) = wall.visual_asset_id.as_ref() else { continue; };
                for offset in 0..wall.length as i32 {
                    let tile = match wall.edge {
                        BuildingWallEdge::North | BuildingWallEdge::South | BuildingWallEdge::Interior =>
                            [wall.start[0] + offset, wall.start[1]],
                        BuildingWallEdge::East | BuildingWallEdge::West =>
                            [wall.start[0], wall.start[1] + offset],
                    };
                    let replaced_by_opening = level.openings.iter().any(|opening| {
                        opening.tile == tile
                            && matches!(opening.kind, BuildingOpeningKind::Door | BuildingOpeningKind::Archway)
                    });
                    if replaced_by_opening { continue; }
                    pieces.push(BuildingRecipePiece {
                        id: format!("{}:wall:{}:{}:{}", self.id, wall.id, tile[0], tile[1]),
                        kind: BuildingRecipePieceKind::Wall,
                        asset_id: asset_id.clone(),
                        level: level.level,
                        tile,
                        state: None,
                        occlusion_group: wall.occlusion_group.clone(),
                    });
                }
            }
            for opening in &level.openings {
                let wall_occlusion_group = level
                    .wall_runs
                    .iter()
                    .find(|wall| {
                        wall.edge == opening.edge
                            && wall_contains_tile(wall.edge, wall.start, wall.length, opening.tile)
                    })
                    .and_then(|wall| wall.occlusion_group.clone())
                    .or_else(|| Some("building.wall_opening".to_string()));
                pieces.push(BuildingRecipePiece {
                    id: format!("{}:opening:{}", self.id, opening.id),
                    kind: BuildingRecipePieceKind::Opening,
                    asset_id: opening.asset_id.clone(),
                    level: level.level,
                    tile: opening.tile,
                    state: opening.state.clone(),
                    occlusion_group: wall_occlusion_group,
                });
            }
            pieces.extend(materialize_level_furnishings(&self.id, level));
        }
        for connector in &self.connectors {
            pieces.push(BuildingRecipePiece {
                id: format!("{}:connector:{}", self.id, connector.id),
                kind: BuildingRecipePieceKind::Connector,
                asset_id: connector.asset_id.clone(),
                level: connector.from_level,
                tile: connector.from_tile,
                state: None,
                occlusion_group: None,
            });
        }
        roof::materialize(self, &mut pieces);
        pieces
    }

    pub fn referenced_asset_ids(&self) -> BTreeSet<&str> {
        let mut refs = BTreeSet::new();
        for level in &self.levels {
            for fill in &level.floor_fills {
                refs.insert(fill.asset_id.as_str());
            }
            for wall in &level.wall_runs {
                if let Some(asset_id) = wall.visual_asset_id.as_deref() {
                    refs.insert(asset_id);
                }
            }
            for opening in &level.openings {
                refs.insert(opening.asset_id.as_str());
            }
            refs.extend(level.furnishings.iter().filter_map(|f| f.asset_id.as_deref()));
        }
        for connector in &self.connectors {
            refs.insert(connector.asset_id.as_str());
        }
        if let Some(components) = &self.roof.components {
            refs.extend(components.iter().map(|(_, asset_id)| asset_id));
        }
        refs.extend(self.roof.authored_modules.iter().map(|module| module.asset_id.as_str()));
        if let Some(asset_id) = self.roof.authored_module_asset_id.as_deref() {
            refs.insert(asset_id);
        }
        refs
    }

    pub fn validate_structure(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.schema != BUILDING_RECIPE_SCHEMA {
            errors.push(format!("{} uses unsupported schema {}", self.id, self.schema));
        }
        if self.id.trim().is_empty() || self.label.trim().is_empty() {
            errors.push("building recipe id/label must not be empty".to_string());
        }
        if self.footprint.contains(&0) {
            errors.push(format!("{} footprint dimensions must be nonzero", self.id));
        }
        let mut level_ids = BTreeSet::new();
        let mut level_numbers = BTreeSet::new();
        let mut opening_ids = BTreeSet::new();
        let mut furnishing_ids = BTreeSet::new();
        for level in &self.levels {
            if !level_ids.insert(level.id.as_str()) {
                errors.push(format!("{} duplicates level id {}", self.id, level.id));
            }
            if !level_numbers.insert(level.level) {
                errors.push(format!("{} duplicates structural level {}", self.id, level.level));
            }
            validate_rect(self, level.bounds, &format!("level {} bounds", level.id), &mut errors);
            let mut room_ids = BTreeSet::new();
            for room in &level.rooms {
                if !room_ids.insert(room.id.as_str()) {
                    errors.push(format!("{} level {} duplicates room {}", self.id, level.id, room.id));
                }
                validate_rect(self, room.rect, &format!("room {}", room.id), &mut errors);
            }
            validate_level_interior(self, level, &room_ids, &mut furnishing_ids, &mut errors);
            for opening in &level.openings {
                if !opening_ids.insert(opening.id.as_str()) {
                    errors.push(format!(
                        "{} duplicates persistent opening id {} across structural levels",
                        self.id, opening.id
                    ));
                }
            }
            let mut wall_ids = BTreeSet::new();
            for wall in &level.wall_runs {
                if !wall_ids.insert(wall.id.as_str()) {
                    errors.push(format!("{} level {} duplicates wall run {}", self.id, level.id, wall.id));
                }
                if wall.length == 0 {
                    errors.push(format!("{} wall run {} has zero length", self.id, wall.id));
                }
                if wall.visual_asset_id.is_none() && wall.visual_status != "deferred_exact_facing" {
                    errors.push(format!(
                        "{} wall run {} has no visual asset and is not explicitly deferred_exact_facing",
                        self.id, wall.id
                    ));
                }
            }
        }
        if !level_numbers.contains(&self.default_level) {
            errors.push(format!("{} default level {} is absent", self.id, self.default_level));
        }
        if !level_numbers.contains(&0) {
            errors.push(format!("{} must define structural ground level 0", self.id));
        }
        for connector in &self.connectors {
            if !level_numbers.contains(&connector.from_level) || !level_numbers.contains(&connector.to_level) {
                errors.push(format!("{} connector {} references an absent level", self.id, connector.id));
            }
            if connector.from_level == connector.to_level {
                errors.push(format!("{} connector {} cannot connect a level to itself", self.id, connector.id));
            }
        }
        roof::validate_structure(self, &mut errors);
        if self.persistence.interior_policy != "same_world_building_instance" {
            errors.push(format!("{} must use same_world_building_instance interior policy", self.id));
        }
        if self.persistence.separate_scene {
            errors.push(format!("{} ordinary recipe may not require a separate scene", self.id));
        }
        if !self.persistence.all_levels_authoritative {
            errors.push(format!("{} must keep every structural level authoritative", self.id));
        }
        errors
    }

    pub fn validate_against_assets(&self, assets: &PublishedWorldAssetRegistry) -> Vec<String> {
        let mut errors = self.validate_structure();
        let furnishing_assets = self.furnishing_asset_ids();
        for asset_id in self.referenced_asset_ids() {
            let Some(asset) = assets.resolve_alias(asset_id) else {
                errors.push(format!("{} references unknown PublishedWorldAsset {}", self.id, asset_id));
                continue;
            };
            if matches!(
                &asset.certification,
                &PublishedWorldAssetCertification::Rejected
                    | &PublishedWorldAssetCertification::Missing
                    | &PublishedWorldAssetCertification::Placeholder
            ) {
                errors.push(format!(
                    "{} references unusable PublishedWorldAsset {} ({:?})",
                    self.id, asset_id, asset.certification
                ));
            }
            if !furnishing_assets.contains(asset_id) && asset.structure.is_none() {
                errors.push(format!("{} references non-structural asset {}", self.id, asset_id));
            }
        }
        validate_interior_assets(self, assets, &mut errors);
        if let Some(components) = &self.roof.components {
            for (expected_role, asset_id) in components.iter() {
                let Some(asset) = assets.resolve_alias(asset_id) else { continue; };
                let actual = asset.structure.as_ref().map(|value| value.topology_role.as_str());
                if actual != Some(expected_role) {
                    errors.push(format!(
                        "{} roof role {} expects topology {}, found {:?}",
                        self.id, asset_id, expected_role, actual
                    ));
                }
            }
        }
        for connector in &self.connectors {
            let Some(asset) = assets.resolve_alias(&connector.asset_id) else { continue; };
            let Some(structure) = &asset.structure else { continue; };
            let delta = (connector.to_level - connector.from_level).abs();
            if structure.level_delta.abs() != delta {
                errors.push(format!(
                    "{} connector {} changes {} level(s), but {} declares level_delta {}",
                    self.id, connector.id, delta, connector.asset_id, structure.level_delta
                ));
            }
        }
        errors
    }
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

fn default_true() -> bool { true }

fn validate_rect(
    recipe: &BuildingRecipeDefinition,
    rect: [i32; 4],
    label: &str,
    errors: &mut Vec<String>,
) {
    let [x, y, w, h] = rect;
    if x < 0 || y < 0 || w <= 0 || h <= 0 {
        errors.push(format!("{} {} must use a positive building-local rect", recipe.id, label));
        return;
    }
    if x + w > recipe.footprint[0] as i32 || y + h > recipe.footprint[1] as i32 {
        errors.push(format!("{} {} exceeds recipe footprint", recipe.id, label));
    }
}

#[derive(Clone, Debug, Default)]
pub struct BuildingRecipeRegistry {
    entries: Vec<BuildingRecipeDefinition>,
    by_id: BTreeMap<String, usize>,
}

impl BuildingRecipeRegistry {
    pub fn load_from_project_root(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();
        let catalog_path = root.join(BUILDING_RECIPE_CATALOG_PATH);
        let catalog: BuildingRecipeCatalogFile = serde_json::from_str(
            &read_to_string(&catalog_path).map_err(|error| format!("{}: {error}", catalog_path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", catalog_path.display()))?;
        if catalog.schema != BUILDING_RECIPE_CATALOG_SCHEMA {
            return Err(format!("{} uses unsupported schema {}", catalog_path.display(), catalog.schema));
        }
        if catalog.authority != "BuildingRecipeRegistry" {
            return Err(format!("{} must declare BuildingRecipeRegistry authority", catalog_path.display()));
        }
        let mut entries = Vec::new();
        let mut by_id = BTreeMap::new();
        for item in catalog.entries {
            if item.status == BuildingRecipeStatus::Deprecated { continue; }
            let relative = safe_relative_path(&item.path)?;
            let path = root.join(relative);
            let recipe: BuildingRecipeDefinition = serde_json::from_str(
                &read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?,
            )
            .map_err(|error| format!("{}: {error}", path.display()))?;
            if recipe.id != item.id {
                return Err(format!("{} catalogs {}, but recipe declares {}", path.display(), item.id, recipe.id));
            }
            let structural_errors = recipe.validate_structure();
            if !structural_errors.is_empty() {
                return Err(structural_errors.join("; "));
            }
            let index = entries.len();
            if by_id.insert(recipe.id.clone(), index).is_some() {
                return Err(format!("duplicate BuildingRecipe id {}", recipe.id));
            }
            entries.push(recipe);
        }
        Ok(Self { entries, by_id })
    }

    pub fn entries(&self) -> &[BuildingRecipeDefinition] { &self.entries }

    pub fn entry(&self, id: &str) -> Option<&BuildingRecipeDefinition> {
        self.by_id.get(id).and_then(|index| self.entries.get(*index))
    }

    pub fn validate_against_assets(&self, assets: &PublishedWorldAssetRegistry) -> Result<(), Vec<String>> {
        let errors = self.entries.iter().flat_map(|recipe| recipe.validate_against_assets(assets)).collect::<Vec<_>>();
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

fn safe_relative_path(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.is_absolute() || path.components().any(|part| matches!(part, std::path::Component::ParentDir)) {
        return Err(format!("building recipe path must be repository-relative: {value}"));
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
    fn w46_recipe_catalog_loads_and_keeps_three_levels_same_world() {
        let registry = BuildingRecipeRegistry::load_from_project_root(project_root())
            .expect("W46 building recipe catalog should load");
        let recipe = registry
            .entry("havenwild.prototype.three_level_house")
            .expect("three-level prototype recipe should exist");
        assert!(recipe.level(-1).is_some());
        assert!(recipe.level(0).is_some());
        assert!(recipe.level(1).is_some());
        assert_eq!(recipe.persistence.interior_policy, "same_world_building_instance");
        assert!(!recipe.persistence.separate_scene);
        assert!(recipe.roof.camera_local_occlusion);
    }

    #[test]
    fn w46_recipe_has_no_silent_missing_wall_visuals() {
        let registry = BuildingRecipeRegistry::load_from_project_root(project_root())
            .expect("W46 building recipe catalog should load");
        for recipe in registry.entries() {
            for level in &recipe.levels {
                for wall in &level.wall_runs {
                    if wall.visual_asset_id.is_none() {
                        assert_eq!(wall.visual_status, "deferred_exact_facing");
                    }
                }
            }
        }
    }
}
