use haven_core::{GameWorld, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

pub const SCENE_RECTANGLE_MANIFEST_PATH: &str =
    "content/worldgen/scene_rectangle_manifest_v0_8.json";
pub const SCENE_RECTANGLE_ASSIGNMENTS_PATH: &str =
    "content/worldgen/scene_rectangle_assignments_v0_8.json";
pub const SCENE_ROLE_CATALOG: [&str; 10] = [
    "home_tavern_mountain_base",
    "mountain_pass",
    "farm_plot",
    "forest_border",
    "cave_entry",
    "cave_depth",
    "owned_land_plot",
    "harbor",
    "main_city_district",
    "beach",
];
pub const SCENE_OWNERSHIP_CATALOG: [&str; 6] = [
    "player_home",
    "public_route",
    "working_land",
    "wilderness",
    "hazard",
    "civic",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneRectangleManifest {
    pub schema: String,
    pub name: String,
    pub world_layout: String,
    pub scene_count: usize,
    pub special_scene_count: usize,
    pub scene_scale_targets: SceneScaleTargets,
    pub edge_contract: SceneEdgeContract,
    #[serde(default)]
    pub archipelago_generation: ArchipelagoGenerationSpec,
    pub scene_rectangles: Vec<SceneRectangleSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchipelagoGenerationSpec {
    pub seed: u64,
    pub reroll_index: u32,
    pub layout_version: u32,
    pub canvas_size_px: [i32; 2],
    pub outer_margin_px: i32,
    pub minimum_island_gap_px: i32,
    pub placement_mode: String,
}

impl Default for ArchipelagoGenerationSpec {
    fn default() -> Self {
        Self {
            seed: 1_337,
            reroll_index: 0,
            layout_version: 1,
            canvas_size_px: [1_800, 1_200],
            outer_margin_px: 70,
            minimum_island_gap_px: 120,
            placement_mode: "seeded_collision_safe_ring".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneScaleTargets {
    pub small_test_chunk_tiles: [i32; 2],
    pub standard_outdoor_scene_tiles: [i32; 2],
    pub large_special_scene_tiles: [i32; 2],
    pub city_district_scene_tiles: [i32; 2],
    pub gameplay_camera_tiles_approx: Vec<[i32; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneEdgeContract {
    pub seam_validation_band_tiles: i32,
    pub decoration_safe_band_tiles: i32,
    pub camera_void_rule: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneRectangleSpec {
    pub scene_id: String,
    pub landmass_id: i32,
    pub landmass_name: String,
    pub kind: String,
    pub grid_x: Option<i32>,
    pub grid_y: Option<i32>,
    pub world_rect_preview_px: [i32; 4],
    pub tile_size: [i32; 2],
    pub edge_contract: String,
    pub streaming: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneRectangleAssignmentsFile {
    pub schema: String,
    pub assignments: Vec<SceneRectangleAssignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SceneRectangleAssignment {
    pub scene_code: String,
    pub rectangle_id: String,
    pub role: String,
    #[serde(default = "default_scene_rectangle_ownership")]
    pub ownership: String,
    pub notes: Option<String>,
}

impl SceneRectangleManifest {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize {path}: {error}"))?;
        std::fs::write(path, data).map_err(|error| format!("failed to write {path}: {error}"))
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let standard_count = self
            .scene_rectangles
            .iter()
            .filter(|rectangle| rectangle.grid_x.is_some() && rectangle.grid_y.is_some())
            .count();
        if standard_count != self.scene_count {
            warnings.push(format!(
                "scene rectangle manifest lists {standard_count} standard rectangles but scene_count says {}",
                self.scene_count
            ));
        }
        if self.edge_contract.seam_validation_band_tiles <= 0 {
            warnings.push("scene rectangle seam validation band must be positive".to_string());
        }
        if self.edge_contract.decoration_safe_band_tiles
            < self.edge_contract.seam_validation_band_tiles
        {
            warnings.push(
                "scene rectangle decoration-safe band should be >= seam validation band"
                    .to_string(),
            );
        }
        if self.scene_scale_targets.standard_outdoor_scene_tiles[0] <= 0
            || self.scene_scale_targets.standard_outdoor_scene_tiles[1] <= 0
        {
            warnings.push("standard outdoor scene size must be positive".to_string());
        }
        if self.archipelago_generation.canvas_size_px[0] <= 0
            || self.archipelago_generation.canvas_size_px[1] <= 0
        {
            warnings.push("archipelago canvas size must be positive".to_string());
        }
        if self.archipelago_generation.minimum_island_gap_px < 0 {
            warnings.push("archipelago minimum island gap cannot be negative".to_string());
        }
        if self.archipelago_generation.outer_margin_px < 0 {
            warnings.push("archipelago outer margin cannot be negative".to_string());
        }

        let mut seen_scene_ids = Vec::new();
        let mut seen_grid_positions = Vec::new();
        for rectangle in &self.scene_rectangles {
            if rectangle.scene_id.trim().is_empty() {
                warnings.push("scene rectangle has empty scene_id".to_string());
            }
            if seen_scene_ids
                .iter()
                .any(|scene_id| scene_id == &rectangle.scene_id)
            {
                warnings.push(format!(
                    "scene rectangle manifest contains duplicate scene id {}",
                    rectangle.scene_id
                ));
            }
            seen_scene_ids.push(rectangle.scene_id.clone());

            if rectangle.tile_size[0] <= 0 || rectangle.tile_size[1] <= 0 {
                warnings.push(format!(
                    "{} has invalid tile size {}x{}",
                    rectangle.scene_id, rectangle.tile_size[0], rectangle.tile_size[1]
                ));
            }
            if rectangle.world_rect_preview_px[2] <= 0 || rectangle.world_rect_preview_px[3] <= 0 {
                warnings.push(format!(
                    "{} has invalid preview rect size {}x{}",
                    rectangle.scene_id,
                    rectangle.world_rect_preview_px[2],
                    rectangle.world_rect_preview_px[3]
                ));
            }
            if !rectangle.kind.starts_with("special_") {
                let Some(grid_x) = rectangle.grid_x else {
                    warnings.push(format!("{} is missing grid_x", rectangle.scene_id));
                    continue;
                };
                let Some(grid_y) = rectangle.grid_y else {
                    warnings.push(format!("{} is missing grid_y", rectangle.scene_id));
                    continue;
                };
                let key = (rectangle.landmass_id, grid_x, grid_y);
                if seen_grid_positions.contains(&key) {
                    warnings.push(format!(
                        "{} duplicates landmass/grid position {},{},{}",
                        rectangle.scene_id, rectangle.landmass_id, grid_x, grid_y
                    ));
                }
                seen_grid_positions.push(key);
            }
        }
        warnings
    }
    pub fn validate_against_world(&self, _world: &GameWorld) -> Vec<String> {
        let _ = (MAP_W, MAP_H);
        Vec::new()
    }
}

pub fn load_scene_rectangle_manifest_from_path(
    path: &str,
) -> Result<SceneRectangleManifest, String> {
    SceneRectangleManifest::load_from_path(path)
}

impl SceneRectangleAssignmentsFile {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize {path}: {error}"))?;
        std::fs::write(path, data).map_err(|error| format!("failed to write {path}: {error}"))
    }

    pub fn validate(&self, manifest: &SceneRectangleManifest, world: &GameWorld) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut seen_scene_codes = Vec::new();
        let mut seen_rectangle_ids = Vec::new();
        for assignment in &self.assignments {
            if !manifest
                .scene_rectangles
                .iter()
                .any(|rectangle| rectangle.scene_id == assignment.rectangle_id)
                && !assignment.rectangle_id.starts_with("SP_")
            {
                warnings.push(format!(
                    "scene rectangle assignment references unknown rectangle {}",
                    assignment.rectangle_id
                ));
            }
            if world
                .scenes
                .iter()
                .all(|scene| scene.id.code() != assignment.scene_code)
            {
                warnings.push(format!(
                    "scene rectangle assignment references unknown scene {}",
                    assignment.scene_code
                ));
            }
            if seen_scene_codes.contains(&assignment.scene_code) {
                warnings.push(format!(
                    "scene {} is assigned to multiple scene rectangles",
                    assignment.scene_code
                ));
            }
            if seen_rectangle_ids.contains(&assignment.rectangle_id) {
                warnings.push(format!(
                    "scene rectangle {} is assigned to multiple scenes",
                    assignment.rectangle_id
                ));
            }
            if !SCENE_ROLE_CATALOG.contains(&assignment.role.as_str()) {
                warnings.push(format!(
                    "scene rectangle assignment {} uses unknown role {}",
                    assignment.rectangle_id, assignment.role
                ));
            }
            if !SCENE_OWNERSHIP_CATALOG.contains(&assignment.ownership.as_str()) {
                warnings.push(format!(
                    "scene rectangle assignment {} uses unknown ownership {}",
                    assignment.rectangle_id, assignment.ownership
                ));
            }
            seen_scene_codes.push(assignment.scene_code.clone());
            seen_rectangle_ids.push(assignment.rectangle_id.clone());
        }
        for required in [
            "farmstead",
            "north_road",
            "south_field",
            "east_woods",
            "cave_mouth",
        ] {
            if !self
                .assignments
                .iter()
                .any(|assignment| assignment.scene_code == required)
            {
                warnings.push(format!(
                    "required prototype scene {} is not mapped to a scene rectangle yet",
                    required
                ));
            }
        }
        for required_role in [
            "home_tavern_mountain_base",
            "mountain_pass",
            "farm_plot",
            "forest_border",
            "cave_entry",
        ] {
            if !self
                .assignments
                .iter()
                .any(|assignment| assignment.role == required_role)
            {
                warnings.push(format!(
                    "scene rectangle assignments do not yet cover required role {}",
                    required_role
                ));
            }
        }
        for required_ownership in [
            "player_home",
            "public_route",
            "working_land",
            "wilderness",
            "hazard",
        ] {
            if !self
                .assignments
                .iter()
                .any(|assignment| assignment.ownership == required_ownership)
            {
                warnings.push(format!(
                    "scene rectangle assignments do not yet cover required ownership {}",
                    required_ownership
                ));
            }
        }
        warnings
    }

    pub fn assignment_for_rectangle(
        &self,
        rectangle_id: &str,
    ) -> Option<&SceneRectangleAssignment> {
        self.assignments
            .iter()
            .find(|assignment| assignment.rectangle_id == rectangle_id)
    }

    pub fn assign_scene_to_rectangle(
        &mut self,
        scene_code: &str,
        rectangle_id: &str,
        role: &str,
        ownership: &str,
    ) {
        self.assignments.retain(|assignment| {
            assignment.scene_code != scene_code && assignment.rectangle_id != rectangle_id
        });
        self.assignments.push(SceneRectangleAssignment {
            scene_code: scene_code.to_string(),
            rectangle_id: rectangle_id.to_string(),
            role: role.to_string(),
            ownership: ownership.to_string(),
            notes: None,
        });
    }

    pub fn clear_rectangle(&mut self, rectangle_id: &str) {
        self.assignments
            .retain(|assignment| assignment.rectangle_id != rectangle_id);
    }
}

pub fn load_scene_rectangle_assignments_from_path(
    path: &str,
) -> Result<SceneRectangleAssignmentsFile, String> {
    SceneRectangleAssignmentsFile::load_from_path(path)
}

pub fn scene_role_catalog() -> &'static [&'static str] {
    &SCENE_ROLE_CATALOG
}

pub fn scene_ownership_catalog() -> &'static [&'static str] {
    &SCENE_OWNERSHIP_CATALOG
}

fn default_scene_rectangle_ownership() -> String {
    "wilderness".to_string()
}
