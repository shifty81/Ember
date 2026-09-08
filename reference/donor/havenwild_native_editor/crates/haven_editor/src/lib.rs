pub mod autotile_edit;
pub mod bulk_edit;
pub mod building_layout_inspector;
pub mod command_bus;
pub mod object_inspector;
pub mod project_file;
mod region_validation;
pub mod scene_clipboard;
pub mod scene_edit;
pub mod scene_management;
pub mod scene_structure_edit;
mod scope_notes;
pub mod stamp_edit;
pub mod stamp_inspector;
pub mod validation_registry;
pub mod world_surface_edit;

pub use autotile_edit::{clear_scene_autotile_override, set_scene_autotile_override};
pub use building_layout_inspector::{inspect_building_layout, BuildingFloorInspectorSummary, BuildingLayoutInspectorReport};
pub use bulk_edit::{
    flood_fill_scene, paint_scene_rectangle, replace_scene_value, selection_bounds_for_items,
    selection_items_in_rect,
};
pub use command_bus::{
    CommandHistoryStep, EditorCommand, EditorCommandBus, EditorCommandKind, EditorCommandSource,
    GridPos,
};
use haven_assets::{
    animation_contract::{
        load_character_animation_contract_from_path,
        validate_character_animation_contract_manifest, CharacterAnimationContractManifest,
        CHARACTER_ANIMATION_CONTRACT_PATH,
    },
    asset_registry::{generated_asset_registry, GENERATED_BASE_TERRAIN_ATLAS_PATH},
};
pub use haven_authoring::{
    apply_terrain_paint_mode_to_map, hit_test_scene_cell, inspect_cell, inspect_scene_cell,
    transition_grid_rect, validate_world, AuthoringCanvasTransform, AuthoringPalette as EditorPalette,
    CanvasHit, CanvasPoint, CanvasRect, AUTHORING_CANVAS_MAX_ZOOM, AUTHORING_CANVAS_MIN_ZOOM, visible_grid_bounds_in_world,
    EditOperation, EditTransaction, EditTransactionBatch, EditorSelection, GridRect,
    InspectorReport, ObjectId, RegionNodeId, SceneAuthoringLayer, SelectionItem, StampInstanceId,
    TerrainPaintModeReport, TransitionId, ZoneId,
};
use haven_core::{AssetRecord, GameWorld, ProjectSceneId, SceneId};
use haven_world::{
    region_graph::{IslandRegionGraph, RegionLink, RegionNode, RegionNodeKind},
    scene_rectangles::{
        load_scene_rectangle_assignments_from_path, load_scene_rectangle_manifest_from_path,
        SceneRectangleAssignmentsFile, SceneRectangleManifest, SCENE_RECTANGLE_ASSIGNMENTS_PATH,
        SCENE_RECTANGLE_MANIFEST_PATH,
    },
};
pub use object_inspector::update_scene_object;
pub use project_file::{
    load_editor_project_file_from_path, starter_project_file, EditorProjectFile,
    STARTER_PROJECT_FILE_PATH,
};
use region_validation::{scene_should_exist_in_region_graph, world_has_transition_between};
pub use scene_clipboard::{
    copy_scene_selection, delete_scene_selection, move_scene_selection, paste_scene_clipboard,
    ClipboardObject, ClipboardStamp, ClipboardTransition, SceneBulkEditOutcome, SceneClipboard,
};
pub use scene_edit::{
    duplicate_scene_object, erase_scene_cell, erase_scene_object, move_scene_object,
    paint_scene_tile, paint_scene_tile_with_mode, paint_scene_zone, place_scene_object,
    place_scene_object_with_footprint, place_scene_pack_asset, SceneEditOutcome,
};
pub use scene_management::{
    create_project_scene, delete_project_scene, duplicate_project_scene, rename_project_scene,
};
pub use scene_structure_edit::{
    create_scene_transition, erase_scene_transition, resize_scene_transition, set_scene_height,
};
pub use scope_notes::*;
pub use stamp_edit::{erase_scene_stamp, move_scene_stamp, place_scene_stamp};
pub use stamp_inspector::{update_scene_stamp, StampUpdateRequest};
use std::path::PathBuf;
pub use validation_registry::{
    validation_registry, ValidationRegistryEntry, ValidationRegistryStatus,
};
pub use world_surface_edit::{
    adjust_world_structural_levels, copy_world_surface_rectangle, flood_fill_world_surface,
    paint_world_surface_cells, paint_world_surface_rectangle, paste_world_surface_clipboard,
    replace_world_surface_value, resolve_world_surface_cell, validate_world_surface_footprint,
    world_surface_bounds, WorldSurfaceCellAddress, WorldSurfaceClipboard, WorldSurfaceEditOutcome,
    WorldSurfaceLayer, WorldSurfaceValue,
};

pub struct EditorWorldModel {
    pub project: EditorProjectFile,
    pub world: GameWorld,
    pub region_graph: IslandRegionGraph,
}

impl EditorWorldModel {
    pub fn starter() -> Self {
        let world = GameWorld::starter();
        Self {
            project: load_editor_project_file_from_path(&repo_path(STARTER_PROJECT_FILE_PATH))
                .unwrap_or_else(|_| starter_project_file(&world)),
            world,
            region_graph: haven_world::region_graph::starter_island_region_graph(),
        }
    }

    pub fn validate(&self) -> Vec<String> {
        validate_editor_world_model(&self.world, &self.region_graph)
    }

    pub fn validation_report(&self) -> EditorValidationReport {
        validate_editor_world_model_report(&self.world, &self.region_graph)
    }

    pub fn region_report(&self) -> InspectorReport {
        inspect_region_graph(&self.region_graph)
    }

    pub fn world_scene_count(&self) -> usize {
        self.world.scenes.len()
    }

    pub fn linked_region_scene_count(&self) -> usize {
        self.region_graph
            .nodes
            .iter()
            .filter(|node| node.scene_id.is_some())
            .count()
    }

    pub fn project_summary(&self) -> Vec<String> {
        self.project.summary_lines()
    }
}

pub fn scene_rectangle_contract_summary() -> Vec<String> {
    summarize_scene_rectangle_contract(&load_scene_rectangle_manifest_from_path(&repo_path(
        SCENE_RECTANGLE_MANIFEST_PATH,
    )))
}

pub fn load_active_scene_rectangle_manifest() -> Result<SceneRectangleManifest, String> {
    load_scene_rectangle_manifest_from_path(&repo_path(SCENE_RECTANGLE_MANIFEST_PATH))
}

pub fn load_active_scene_rectangle_assignments() -> Result<SceneRectangleAssignmentsFile, String> {
    load_scene_rectangle_assignments_from_path(&repo_path(SCENE_RECTANGLE_ASSIGNMENTS_PATH))
}

pub fn animation_contract_summary() -> Vec<String> {
    summarize_animation_contract(&load_character_animation_contract_from_path(&repo_path(
        CHARACTER_ANIMATION_CONTRACT_PATH,
    )))
}

pub fn generated_asset_registry_summary() -> Vec<String> {
    match generated_asset_registry() {
        Ok(registry) => vec![
            format!("Manifest: {}", registry.manifest_id()),
            format!("Terrain manifest: {}", registry.terrain_manifest_id()),
            format!("Tile bindings: {}", registry.tile_entries().len()),
            format!("Atlas: {}", registry.terrain_atlas_path()),
            format!("Expected atlas: {}", GENERATED_BASE_TERRAIN_ATLAS_PATH),
        ],
        Err(error) => vec![format!("Generated assets unavailable: {error}")],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSystemAudit {
    pub name: &'static str,
    pub core_owner: &'static str,
    pub editor_surface: &'static str,
    pub runtime_consumer: &'static str,
    pub status: EditorSystemStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorSystemStatus {
    Shared,
    Partial,
    Missing,
}

impl EditorSystemStatus {
    pub fn label(self) -> &'static str {
        match self {
            EditorSystemStatus::Shared => "shared",
            EditorSystemStatus::Partial => "partial",
            EditorSystemStatus::Missing => "missing",
        }
    }
}

pub fn editor_system_audit() -> Vec<EditorSystemAudit> {
    vec![
        EditorSystemAudit {
            name: "Region graph",
            core_owner: "haven_world::region_graph",
            editor_surface: "Inspector + validation",
            runtime_consumer: "world transition alignment",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Scene tilemap",
            core_owner: "haven_core::SceneMap/TavernMap",
            editor_surface: "Palette + cell inspector",
            runtime_consumer: "game scene renderer/save",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Autotiling",
            core_owner: "haven_assets::autotile",
            editor_surface: "cell inspector mask diagnostics",
            runtime_consumer: "game tile presentation",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Asset catalog",
            core_owner: "haven_assets::asset_registry",
            editor_surface: "reference/catalog review",
            runtime_consumer: "generated terrain atlas lookup",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Animation catalog",
            core_owner: "haven_assets::animation_contract",
            editor_surface: "contract validation + future character workspace",
            runtime_consumer: "future character controller",
            status: EditorSystemStatus::Partial,
        },
        EditorSystemAudit {
            name: "World generation",
            core_owner: "haven_world + haven_core worldgen loaders",
            editor_surface: "pending generator profile editor",
            runtime_consumer: "starter scene generation",
            status: EditorSystemStatus::Partial,
        },
        EditorSystemAudit {
            name: "Scene rectangles",
            core_owner: "haven_world::scene_rectangles",
            editor_surface: "scene rectangle contract validation",
            runtime_consumer: "future streaming/runtime bounds",
            status: EditorSystemStatus::Partial,
        },
        EditorSystemAudit {
            name: "Project shell",
            core_owner: "haven_editor::project_file",
            editor_surface: "native editor project summary",
            runtime_consumer: "runtime/editor boot configuration",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Shared command bus",
            core_owner: "haven_authoring::command_bus",
            editor_surface: "undo/redo + command history",
            runtime_consumer: "in-game overlay mutations",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Collision/navigation",
            core_owner: "haven_core + content/worldgen footprint rules",
            editor_surface: "collision/interaction overlays + footprint inspector",
            runtime_consumer: "walkability + placement checks",
            status: EditorSystemStatus::Shared,
        },
        EditorSystemAudit {
            name: "Economy/recipes",
            core_owner: "haven_sim + future content schemas",
            editor_surface: "pending data tables",
            runtime_consumer: "pending sim systems",
            status: EditorSystemStatus::Missing,
        },
        EditorSystemAudit {
            name: "Quests/scripts/events",
            core_owner: "haven_sim + future script/event schemas",
            editor_surface: "pending event graph",
            runtime_consumer: "pending event runner",
            status: EditorSystemStatus::Missing,
        },
    ]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorValidationReport {
    pub sections: Vec<EditorValidationSection>,
}

impl EditorValidationReport {
    pub fn is_clean(&self) -> bool {
        self.sections.iter().all(EditorValidationSection::is_clean)
    }

    pub fn total_issue_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.messages.len())
            .sum()
    }

    pub fn flattened_messages(&self) -> Vec<String> {
        self.sections
            .iter()
            .flat_map(|section| {
                section
                    .messages
                    .iter()
                    .map(|message| format!("{}: {}", section.title, message))
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorValidationSection {
    pub id: &'static str,
    pub title: &'static str,
    pub messages: Vec<String>,
}

impl EditorValidationSection {
    pub fn new(id: &'static str, title: &'static str, messages: Vec<String>) -> Self {
        Self {
            id,
            title,
            messages,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.messages.is_empty()
    }
}

pub fn validate_region_graph(graph: &IslandRegionGraph) -> Vec<String> {
    let mut warnings = Vec::new();

    if graph.id.trim().is_empty() {
        warnings.push("region graph has no id".to_string());
    }
    if graph.nodes.is_empty() {
        warnings.push(format!("{} has no region nodes", graph.display_name));
        return warnings;
    }
    if graph.links.is_empty() {
        warnings.push(format!("{} has no region links", graph.display_name));
    }
    if !graph.landmass.center.is_normalized() {
        warnings.push(format!(
            "{} landmass center is outside normalized preview coordinates",
            graph.display_name
        ));
    }
    if !graph.landmass.radius.is_normalized()
        || graph.landmass.radius.x <= 0.0
        || graph.landmass.radius.y <= 0.0
    {
        warnings.push(format!(
            "{} has invalid landmass radius",
            graph.display_name
        ));
    }

    let mut seen_node_ids: Vec<RegionNodeId> = Vec::new();
    let mut seen_scene_ids: Vec<ProjectSceneId> = Vec::new();
    for node in &graph.nodes {
        if node.id.as_str().trim().is_empty() {
            warnings.push(format!(
                "{} has a region node with no id",
                graph.display_name
            ));
        }
        if seen_node_ids.contains(&node.id) {
            warnings.push(format!("duplicate region node id: {}", node.id));
        }
        seen_node_ids.push(node.id.clone());

        if !node.position.is_normalized() {
            warnings.push(format!(
                "region node '{}' is outside island preview bounds",
                node.id
            ));
        }

        match node.scene_id.as_ref() {
            Some(scene_id) => {
                if seen_scene_ids.contains(scene_id) {
                    warnings.push(format!(
                        "scene {} is assigned to multiple region nodes",
                        scene_id.label()
                    ));
                }
                seen_scene_ids.push(scene_id.clone());
            }
            None => {
                if node.kind != RegionNodeKind::FutureHarbor {
                    warnings.push(format!(
                        "region node '{}' has no scene_id but is not marked future content",
                        node.id
                    ));
                }
            }
        }
    }

    for link in &graph.links {
        if graph.node(&link.from).is_none() {
            warnings.push(format!(
                "region link starts at missing node '{}'",
                link.from
            ));
        }
        if graph.node(&link.to).is_none() {
            warnings.push(format!("region link ends at missing node '{}'", link.to));
        }
        if link.from == link.to {
            warnings.push(format!("region link '{}' loops to itself", link.from));
        }
    }

    for required in [
        SceneId::Farmstead,
        SceneId::NorthRoad,
        SceneId::SouthField,
        SceneId::EastWoods,
        SceneId::CaveMouth,
        SceneId::CaveDepths,
    ] {
        let required_id = ProjectSceneId::from(required);
        if !seen_scene_ids.contains(&required_id) {
            warnings.push(format!(
                "{} is missing required island region scene {}",
                graph.display_name,
                required.label()
            ));
        }
    }

    let connected = |a: &str, b: &str| {
        graph.links.iter().any(|link| {
            (link.from.as_str() == a && link.to.as_str() == b)
                || (link.from.as_str() == b && link.to.as_str() == a)
        })
    };
    if !connected("farmstead", "south_field") {
        warnings.push("starter island graph does not connect Estate to South Field".to_string());
    }
    if !connected("farmstead", "east_woods") {
        warnings.push("starter island graph does not connect Estate to East Woods".to_string());
    }
    if !connected("east_woods", "cave_mouth") {
        warnings.push("starter island graph does not connect East Woods to Cave Mouth".to_string());
    }

    warnings
}

pub fn validate_region_graph_against_world(
    graph: &IslandRegionGraph,
    world: &GameWorld,
) -> Vec<String> {
    let mut warnings = Vec::new();

    for node in &graph.nodes {
        if let Some(scene_id) = node.scene_id.as_ref() {
            if world.scene_by_id(scene_id).is_none() {
                warnings.push(format!(
                    "region node '{}' references missing scene {}",
                    node.id,
                    scene_id.label()
                ));
            }
        }
    }

    for scene in &world.scenes {
        let Some(legacy_id) = scene.id.legacy_scene_id() else {
            continue;
        };
        if scene_should_exist_in_region_graph(scene.kind, legacy_id)
            && graph
                .nodes
                .iter()
                .all(|node| node.scene_id.as_ref() != Some(&scene.id))
        {
            warnings.push(format!(
                "{} exists in the world but is missing from the island region graph",
                scene.name
            ));
        }
    }

    for link in &graph.links {
        let Some(from_node) = graph.node(&link.from) else {
            continue;
        };
        let Some(to_node) = graph.node(&link.to) else {
            continue;
        };
        let (Some(from_scene), Some(to_scene)) =
            (from_node.scene_id.as_ref(), to_node.scene_id.as_ref())
        else {
            continue;
        };

        if !world_has_transition_between(world, from_scene, to_scene) {
            warnings.push(format!(
                "region link {} -> {} is not represented by world transitions",
                from_node.label, to_node.label
            ));
        }
    }

    warnings
}

pub fn inspect_region_graph(graph: &IslandRegionGraph) -> InspectorReport {
    let mut lines = vec![
        format!("Graph ID: {}", graph.id),
        format!("Role: {}", graph.role),
        format!("Nodes: {}", graph.nodes.len()),
        format!("Links: {}", graph.links.len()),
        format!(
            "Landmass: center {:.2},{:.2} radius {:.2},{:.2} noise {:.2}",
            graph.landmass.center.x,
            graph.landmass.center.y,
            graph.landmass.radius.x,
            graph.landmass.radius.y,
            graph.landmass.shore_noise
        ),
    ];

    lines.push("Scene nodes:".to_string());
    for node in &graph.nodes {
        lines.push(region_node_line(node));
    }

    lines.push("Scene links:".to_string());
    for link in &graph.links {
        lines.push(region_link_line(graph, link));
    }

    InspectorReport {
        title: graph.display_name.to_string(),
        lines,
    }
}

pub fn validate_editor_world_model(world: &GameWorld, graph: &IslandRegionGraph) -> Vec<String> {
    validate_editor_world_model_report(world, graph).flattened_messages()
}

pub fn validate_editor_world_model_report(
    world: &GameWorld,
    graph: &IslandRegionGraph,
) -> EditorValidationReport {
    let project = starter_project_file(world);
    EditorValidationReport {
        sections: vec![
            EditorValidationSection::new("world", "World Scenes", validate_world(world)),
            EditorValidationSection::new(
                "generated_assets",
                "Generated Asset Contract",
                validate_generated_asset_registry(),
            ),
            EditorValidationSection::new(
                "region_graph",
                "Island Region Graph",
                validate_region_graph(graph),
            ),
            EditorValidationSection::new(
                "world_region_alignment",
                "World/Region Alignment",
                validate_region_graph_against_world(graph, world),
            ),
            EditorValidationSection::new(
                "scene_rectangles",
                "Scene Rectangle Contract",
                validate_scene_rectangle_contract(world),
            ),
            EditorValidationSection::new(
                "scene_rectangle_assignments",
                "Scene Rectangle Assignments",
                validate_scene_rectangle_assignments(world),
            ),
            EditorValidationSection::new("project_file", "Project Shell", project.validate()),
            EditorValidationSection::new(
                "animation_contract",
                "Character Animation Contract",
                validate_animation_contract(),
            ),
        ],
    }
}

fn validate_generated_asset_registry() -> Vec<String> {
    match generated_asset_registry() {
        Ok(_) => Vec::new(),
        Err(error) => vec![format!(
            "Generated worldgen asset registry failed to load: {error}"
        )],
    }
}

fn validate_scene_rectangle_contract(world: &GameWorld) -> Vec<String> {
    match load_scene_rectangle_manifest_from_path(&repo_path(SCENE_RECTANGLE_MANIFEST_PATH)) {
        Ok(manifest) => {
            let mut warnings = manifest.validate();
            warnings.extend(manifest.validate_against_world(world));
            warnings
        }
        Err(error) => vec![format!("Scene rectangle contract failed to load: {error}")],
    }
}

fn validate_animation_contract() -> Vec<String> {
    match load_character_animation_contract_from_path(&repo_path(CHARACTER_ANIMATION_CONTRACT_PATH))
    {
        Ok(contract) => validate_character_animation_contract_manifest(&contract),
        Err(error) => vec![format!("Animation contract failed to load: {error}")],
    }
}

fn validate_scene_rectangle_assignments(world: &GameWorld) -> Vec<String> {
    let manifest =
        load_scene_rectangle_manifest_from_path(&repo_path(SCENE_RECTANGLE_MANIFEST_PATH));
    let assignments =
        load_scene_rectangle_assignments_from_path(&repo_path(SCENE_RECTANGLE_ASSIGNMENTS_PATH));
    match (manifest, assignments) {
        (Ok(manifest), Ok(assignments)) => assignments.validate(&manifest, world),
        (Err(error), _) => vec![format!("Scene rectangle manifest failed to load: {error}")],
        (_, Err(error)) => vec![format!(
            "Scene rectangle assignments failed to load: {error}"
        )],
    }
}

fn summarize_scene_rectangle_contract(
    manifest: &Result<SceneRectangleManifest, String>,
) -> Vec<String> {
    match manifest {
        Ok(manifest) => vec![
            format!("Scene rectangles: {}", manifest.scene_count),
            format!("Special regions: {}", manifest.special_scene_count),
            format!(
                "Outdoor target: {}x{}",
                manifest.scene_scale_targets.standard_outdoor_scene_tiles[0],
                manifest.scene_scale_targets.standard_outdoor_scene_tiles[1]
            ),
            format!(
                "Seam band: {} / safe band: {}",
                manifest.edge_contract.seam_validation_band_tiles,
                manifest.edge_contract.decoration_safe_band_tiles
            ),
        ],
        Err(error) => vec![format!("Scene rectangle contract unavailable: {error}")],
    }
}

fn summarize_animation_contract(
    manifest: &Result<CharacterAnimationContractManifest, String>,
) -> Vec<String> {
    match manifest {
        Ok(manifest) => vec![
            format!(
                "Anim cell: {}x{}",
                manifest.locked_first_sheet.cell_size[0], manifest.locked_first_sheet.cell_size[1]
            ),
            format!(
                "Directions: {} | Walk frames: {}",
                manifest.locked_first_sheet.directions,
                manifest.locked_first_sheet.frames_per_direction
            ),
            format!(
                "Sheet size: {}x{}",
                manifest.locked_first_sheet.sheet_size[0],
                manifest.locked_first_sheet.sheet_size[1]
            ),
            format!("Anchor: {}", manifest.locked_first_sheet.root_anchor),
        ],
        Err(error) => vec![format!("Animation contract unavailable: {error}")],
    }
}

fn repo_path(relative: &str) -> String {
    repo_root_dir()
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

fn repo_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("haven_editor crate should live under crates/<name>")
}

fn region_node_line(node: &RegionNode) -> String {
    let scene = node
        .scene_id
        .as_ref()
        .map(ProjectSceneId::label)
        .unwrap_or_else(|| "future scene".to_string());
    format!(
        "- {} [{}] {} at {:.2},{:.2} ({})",
        node.label,
        node.kind.code(),
        scene,
        node.position.x,
        node.position.y,
        node.biome.label()
    )
}

fn region_link_line(graph: &IslandRegionGraph, link: &RegionLink) -> String {
    let from = graph
        .node(&link.from)
        .map(|node| node.label.as_str())
        .unwrap_or_else(|| link.from.as_str());
    let to = graph
        .node(&link.to)
        .map(|node| node.label.as_str())
        .unwrap_or_else(|| link.to.as_str());
    format!("- {from} -> {to} [{}]", link.kind.code())
}

pub fn validate_asset_record(asset: &AssetRecord) -> Vec<String> {
    let mut warnings = Vec::new();
    if asset.id.trim().is_empty() {
        warnings.push("asset id is empty".to_string());
    }
    if asset.source.trim().is_empty() {
        warnings.push(format!("{} has no source path", asset.id));
    }
    if asset.output.trim().is_empty() {
        warnings.push(format!("{} has no output path", asset.id));
    }
    if asset.missing_license() {
        warnings.push(format!("{} needs license metadata", asset.id));
    }
    warnings
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
