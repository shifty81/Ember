mod animation_studio;
mod animation_studio_input;
mod animation_studio_render;
mod asset_intake_panel;
mod asset_library_panel;
mod asset_palette_panel;
mod atlas_render;
mod autotile_authoring;
mod autotile_render;
mod bulk_result;
mod building_instance_preview;
mod canvas_camera;
mod canvas_controller;
mod canvas_view;
mod character_studio;
mod clipboard_tools;
mod draw;
mod development_session;
mod pixel_library_panel;
mod world_surface_authoring_geometry;
mod editor_menu;
mod editor_theme;
mod editor_text;
mod editor_types;
mod input;
mod island_authoring;
mod island_workspace;
mod object_inspector;
mod pixel_animation_bridge;
mod pixel_color_panel;
mod pixel_layer_input;
mod pixel_layer_panel;
mod pixel_new_document;
mod pixel_studio;
mod pixel_studio_input;
mod pixel_studio_layout;
mod pixel_studio_render;
mod production_tools;
mod render_helpers;
mod scene_render_helpers;
mod scene_asset_context;
mod scene_authoring;
mod scene_bank_workspace;
mod scene_outliner;
mod scene_toolrail;
mod selection_controller;
mod sprite_canvas_authority;
mod sprite_workspace;
mod stamp_inspector_panel;
mod structural_cliff_preview;
mod ui_shell;
mod world_asset_pixel_bridge;
mod world_canvas_context;
mod world_surface_editor;
mod world_surface_authoring;
mod world_surface_authoring_ui;
mod world_surface_structural_authoring;
mod workspace_chrome;
mod workspace_shell;

use std::{collections::HashMap, time::SystemTime};

use animation_studio::AnimationStudioState;
use atlas_render::EditorTextureSet;
use canvas_camera::CanvasCameraState;
use canvas_view::{
    canvas_toolbar_button_rect, draw_canvas_rulers, draw_canvas_toolbar, draw_infinite_grid,
    draw_scene_into_rect, scene_apply_button_rect, scene_content_button_rect,
    scene_content_capacity, scene_content_page_start, scene_erase_button_rect,
    scene_layer_button_rect, scene_layer_lock_button_rect, scene_layer_opacity_down_rect,
    scene_layer_opacity_up_rect, scene_layer_visibility_button_rect,
    scene_terrain_paint_mode_button_rect, scene_tool_button_rect, CanvasToolbarKind,
};
use character_studio::CharacterStudioState;
pub(crate) use editor_types::*;
use editor_text::{draw_editor_text, measure_editor_text};
use haven_assets::{
    asset_browser::AssetBrowserSnapshot,
    asset_intake::AssetIntakeCatalog,
    building_instance::{BuildingInstanceRegistry, BuildingInstanceViewState}, building_recipe::BuildingRecipeRegistry,
    asset_palette::{
        AssetPaletteCatalog, AssetPaletteCategory, AssetPaletteState, PALETTE_OBJECTS,
    },
    live_autotile_atlas::live_autotile_atlas_registry,
    placeable_asset_registry::PublishedWorldAssetRegistry,
    runtime_asset_cache::RuntimeAssetSession,
    stamp_registry::StampRegistry,
};
use haven_core::{
    ObjectKind, PlacedObject, PlacedStamp, ProjectSceneId, SceneBiome, SceneId, SceneKind,
    SceneMap, TerrainPaintMode, TileKind, ZoneKind, MAX_STRUCTURAL_LEVEL, MAP_H, MAP_W,
};
use haven_editor::{
    animation_contract_summary, clear_scene_autotile_override, copy_scene_selection,
    create_project_scene, create_scene_transition, delete_project_scene, delete_scene_selection,
    duplicate_project_scene, editor_system_audit, editor_workspace_titles, erase_scene_cell,
    erase_scene_object, erase_scene_stamp, erase_scene_transition, flood_fill_scene,
    generated_asset_registry_summary, hit_test_scene_cell, load_active_scene_rectangle_assignments,
    load_active_scene_rectangle_manifest, load_editor_project_file_from_path, move_scene_object,
    move_scene_selection, move_scene_stamp, paint_scene_rectangle, paint_scene_tile_with_mode,
    paint_scene_zone, paste_scene_clipboard, place_scene_object_with_footprint,
    place_scene_pack_asset, place_scene_stamp, rename_project_scene, replace_scene_value,
    resize_scene_transition, scene_rectangle_contract_summary, selection_bounds_for_items,
    selection_items_in_rect, set_scene_autotile_override, update_scene_object, update_scene_stamp,
    validation_registry, adjust_world_structural_levels, copy_world_surface_rectangle,
    flood_fill_world_surface,
    paint_world_surface_cells, paint_world_surface_rectangle, paste_world_surface_clipboard,
    replace_world_surface_value, resolve_world_surface_cell, validate_world_surface_footprint,
    EditorCommand, EditorCommandBus,
    EditorCommandKind, EditorCommandSource, EditorSelection, EditorSystemStatus,
    EditorValidationReport, EditorWorldModel, GridPos, GridRect, RegionNodeId,
    SceneAuthoringLayer, SceneClipboard, SelectionItem, StampUpdateRequest,
    WorldSurfaceClipboard, WorldSurfaceValue, STARTER_PROJECT_FILE_PATH,
};
use haven_save::{load_world_from_path, save_world_to_path};
use haven_world::autotile::{AutotileSyncReport, LiveAutotileCache};
use haven_world::harbor_routes::{HarborRouteCatalog, HARBOR_ROUTE_CATALOG_PATH};
use haven_world::region_graph::{
    IslandRegionGraph, RegionLink, RegionLinkKind, RegionNode, RegionNodeKind,
};
use haven_world::scene_rectangles::{
    scene_ownership_catalog, scene_role_catalog, SceneRectangleAssignmentsFile,
    SceneRectangleManifest, SCENE_RECTANGLE_ASSIGNMENTS_PATH, SCENE_RECTANGLE_MANIFEST_PATH,
};
use macroquad::prelude::*;
use pixel_studio::PixelStudioState;
use render_helpers::draw_scissored_text;
use scene_asset_context::SceneAssetContextMenu;
use structural_cliff_preview::StructuralCliffPreviewCache;
use world_canvas_context::WorldCanvasContextMenu;
use world_surface_editor::{
    draw_scene_rectangle_map, rectangle_is_overworld_surface,
    world_scene_grid_bounds_for_landmass, world_scene_grid_rect, WorldSurfaceViewOptions,
};
use workspace_shell::{EditorWorkspaceShellState, WorkspaceResizeDrag};

const PANEL_BG: Color = Color::new(0.055, 0.065, 0.078, 0.98);
const PANEL_EDGE: Color = Color::new(0.24, 0.29, 0.35, 1.0);
const TEXT: Color = Color::new(0.91, 0.94, 0.97, 1.0);
const MUTED: Color = Color::new(0.58, 0.64, 0.71, 1.0);
const GOOD: Color = Color::new(0.32, 0.78, 0.55, 1.0);
const WARN: Color = Color::new(0.96, 0.62, 0.30, 1.0);
const ACCENT: Color = Color::new(0.20, 0.48, 0.78, 1.0);
const CONTROL_BG: Color = Color::new(0.09, 0.11, 0.14, 1.0);

const OBJECT_BRUSHES: [ObjectKind; PALETTE_OBJECTS.len()] = PALETTE_OBJECTS;
const ZONE_BRUSHES: [ZoneKind; 17] = [
    ZoneKind::Tavern,
    ZoneKind::Kitchen,
    ZoneKind::GuestRoom,
    ZoneKind::Cellar,
    ZoneKind::Greenhouse,
    ZoneKind::Field,
    ZoneKind::Cave,
    ZoneKind::StaffOnly,
    ZoneKind::PublicPath,
    ZoneKind::TavernExterior,
    ZoneKind::Bar,
    ZoneKind::CivicLot,
    ZoneKind::MarketLot,
    ZoneKind::ResidentialLot,
    ZoneKind::ArtisanLot,
    ZoneKind::HarborLot,
    ZoneKind::AgriculturalLot,
];

use std::time::Instant;

pub(crate) struct EditorApp {
    model: EditorWorldModel,
    selected_rectangle: usize,
    selected_landmass_id: i32,
    selected_scene: usize,
    scene_list_offset: usize,
    object_list_offset: usize,
    object_filter: String,
    asset_catalog: AssetPaletteCatalog,
    asset_palette_state: AssetPaletteState,
    asset_filter: String,
    asset_category: AssetPaletteCategory,
    asset_favorites_only: bool,
    asset_recent_only: bool,
    asset_list_offset: usize,
    asset_drag: Option<asset_palette_panel::AssetPaletteDrag>,
    asset_browser: AssetBrowserSnapshot,
    asset_pack_mounted_count: usize,
    asset_pack_failure_count: usize,
    asset_pack_source_count: usize,
    asset_library_filter: String,
    asset_library_pack_index: usize,
    asset_library_category_index: usize,
    asset_library_production_only: bool,
    asset_library_runtime_ready_only: bool,
    asset_library_list_offset: usize,
    asset_library_selected: usize,
    asset_intake_catalog: AssetIntakeCatalog,
    asset_intake_selected: usize,
    asset_intake_footprint_target: FootprintEditTarget,
    asset_hot_reload_requested: bool,
    asset_intake_source_reload_requested: bool,
    asset_hot_reload_next_check: f64,
    asset_manifest_modified: Option<SystemTime>,
    editor_textures: EditorTextureSet,
    text_focus: EditorTextFocus,
    scene_name_edit: Option<SceneNameEditState>,
    scene_delete_armed: Option<ProjectSceneId>,
    scene_dock_tab: SceneDockTab,
    footprint_edit_target: FootprintEditTarget,
    scene_cursor_x: i32,
    scene_cursor_y: i32,
    last_painted_cell: Option<(i32, i32)>,
    scene_edit_tool: SceneEditTool,
    scene_layer_mode: SceneLayerMode,
    terrain_paint_mode: TerrainPaintMode,
    selected_tile: usize,
    selected_object: usize,
    selected_placeable_id: Option<String>,
    selected_placeable_preview_state: usize,
    stamp_registry: StampRegistry,
    placeable_registry: PublishedWorldAssetRegistry,
    building_recipe_registry: BuildingRecipeRegistry,
    building_instance_registry: BuildingInstanceRegistry,
    building_preview_views: HashMap<String, BuildingInstanceViewState>,
    selected_stamp_id: Option<String>,
    selection: EditorSelection,
    scene_drag: Option<SceneCanvasDrag>,
    scene_clipboard: Option<SceneClipboard>,
    scene_layers: [SceneLayerState; 4],
    selected_zone: usize,
    selected_transition_target: usize,
    selected_scene_cycle: usize,
    selected_role_cycle: usize,
    selected_ownership_cycle: usize,
    scene_rectangles: Option<SceneRectangleManifest>,
    scene_assignments: SceneRectangleAssignmentsFile,
    harbor_routes: HarborRouteCatalog,
    route_source_landmass_id: Option<i32>,
    viewport_mode: EditorViewportMode,
    workspace_shell: EditorWorkspaceShellState,
    workspace_resize_drag: Option<WorkspaceResizeDrag>,
    saved_undo_depth: usize,
    open_menu: Option<editor_menu::EditorMenuKind>,
    scene_canvas: CanvasCameraState,
    scene_canvas_states: HashMap<ProjectSceneId, CanvasCameraState>,
    autotile_caches: HashMap<ProjectSceneId, LiveAutotileCache>,
    structural_cliff_caches: HashMap<ProjectSceneId, StructuralCliffPreviewCache>,
    autotile_preview_enabled: bool,
    autotile_dirty_overlay: bool,
    scene_show_grid: bool,
    selected_autotile_preset: usize,
    world_canvas: CanvasCameraState,
    world_canvas_pan_tool: bool,
    world_edit_tool: WorldEditTool,
    world_layer_mode: WorldLayerMode,
    world_drag: Option<WorldCanvasDrag>,
    world_selection: Option<GridRect>,
    world_clipboard: Option<WorldSurfaceClipboard>,
    world_structural_level: u8,
    world_brush_radius: i32,
    world_last_painted: Option<GridPos>,
    world_show_partitions: bool,
    world_show_objects: bool,
    world_show_zones: bool,
    world_show_structural_levels: bool,
    world_cursor_x: i32,
    world_cursor_y: i32,
    scene_bank_canvas: CanvasCameraState,
    scene_bank_pan_tool: bool,
    world_canvas_context_menu: Option<WorldCanvasContextMenu>,
    scene_asset_context_menu: Option<SceneAssetContextMenu>,
    primary_pointer_owned_by_ui: bool,
    command_bus: EditorCommandBus,
    pixel_studio: PixelStudioState,
    animation_studio: AnimationStudioState,
    character_studio: CharacterStudioState,
    status_message: String,
    development_client: Option<std::process::Child>,
}

impl EditorApp {
    pub(crate) fn new_without_textures() -> Self {
        let asset_catalog = AssetPaletteCatalog::load_default().unwrap_or_default();
        let asset_palette_state = AssetPaletteState::load_default();
        let asset_browser = AssetBrowserSnapshot::load_project(std::path::Path::new("."));
        let asset_intake_catalog = AssetIntakeCatalog::load_default().unwrap_or_default();
        let asset_session = RuntimeAssetSession::discover_tolerant(std::path::Path::new("."));
        let asset_pack_mounted_count = asset_session.registry.mounted_pack_count();
        let asset_pack_failure_count = asset_session.discovery.failed_count();
        let asset_pack_source_count = asset_session.sources.len();
        let stamp_registry = StampRegistry::load_discovered(&asset_session)
            .or_else(|_| StampRegistry::load_default())
            .unwrap_or_default();
        let placeable_registry = PublishedWorldAssetRegistry::load_discovered(&asset_session)
            .unwrap_or_default();
        let building_recipe_registry = BuildingRecipeRegistry::load_from_project_root(".").unwrap_or_default();
        let building_instance_registry = BuildingInstanceRegistry::load_from_project_root(".", &building_recipe_registry).unwrap_or_default();
        let building_preview_views = building_instance_registry
            .initial_view_states()
            .into_iter()
            .map(|(id, mut state)| { state.inside = true; (id, state) })
            .collect();
        let editor_textures = EditorTextureSet::empty();
        let asset_manifest_modified = None;
        let texture_summary = "asset textures deferred until the first visible frame".to_string();
        let mut model = EditorWorldModel::starter();
        let editor_world_path = development_session::editor_world_path();
        if let Ok(mut saved_world) = load_world_from_path(&editor_world_path.to_string_lossy()) {
            if let Ok(descriptor) = development_session::DevelopmentWorldDescriptor::load() {
                if development_session::ensure_acceptance_crate(&mut saved_world, &descriptor).is_ok() {
                    let _ = save_world_to_path(&editor_world_path.to_string_lossy(), &saved_world);
                }
            }
            model.world = saved_world;
        }
        let canonicalized_world_assets =
            placeable_registry.canonicalize_world_aliases(&mut model.world);
        let mut app = Self {
            model,
            selected_rectangle: 0,
            selected_landmass_id: 0,
            selected_scene: 0,
            scene_list_offset: 0,
            object_list_offset: 0,
            object_filter: String::new(),
            asset_catalog,
            asset_palette_state,
            asset_filter: String::new(),
            asset_category: AssetPaletteCategory::All,
            asset_favorites_only: false,
            asset_recent_only: false,
            asset_list_offset: 0,
            asset_drag: None,
            asset_browser,
            asset_pack_mounted_count,
            asset_pack_failure_count,
            asset_pack_source_count,
            asset_library_filter: String::new(),
            asset_library_pack_index: 0,
            asset_library_category_index: 0,
            asset_library_production_only: false,
            asset_library_runtime_ready_only: false,
            asset_library_list_offset: 0,
            asset_library_selected: 0,
            asset_intake_catalog,
            asset_intake_selected: 0,
            asset_intake_footprint_target: FootprintEditTarget::Visual,
            asset_hot_reload_requested: false,
            asset_intake_source_reload_requested: false,
            asset_hot_reload_next_check: 0.0,
            asset_manifest_modified,
            editor_textures,
            text_focus: EditorTextFocus::None,
            scene_name_edit: None,
            scene_delete_armed: None,
            scene_dock_tab: SceneDockTab::Assets,
            footprint_edit_target: FootprintEditTarget::Visual,
            scene_cursor_x: 24,
            scene_cursor_y: 16,
            last_painted_cell: None,
            scene_edit_tool: SceneEditTool::Paint,
            scene_layer_mode: SceneLayerMode::Terrain,
            terrain_paint_mode: TerrainPaintMode::Exact,
            selected_tile: 5,
            selected_object: 0,
            selected_placeable_id: placeable_registry
                .entries()
                .first()
                .map(|entry| entry.stable_id.clone()),
            selected_placeable_preview_state: 0,
            stamp_registry,
            placeable_registry,
            building_recipe_registry,
            building_instance_registry,
            building_preview_views,
            selected_stamp_id: None,
            selection: EditorSelection::default(),
            scene_drag: None,
            scene_clipboard: None,
            scene_layers: [
                SceneLayerState::default(),
                SceneLayerState::default(),
                SceneLayerState {
                    visible: false,
                    locked: false,
                    opacity: 1.0,
                },
                SceneLayerState {
                    visible: false,
                    locked: false,
                    opacity: 1.0,
                },
            ],
            selected_zone: 0,
            selected_transition_target: 1,
            selected_scene_cycle: 0,
            selected_role_cycle: 0,
            selected_ownership_cycle: 0,
            scene_rectangles: load_active_scene_rectangle_manifest().ok(),
            scene_assignments: load_active_scene_rectangle_assignments().unwrap_or(
                SceneRectangleAssignmentsFile {
                    schema: "havenwild.scene_rectangle_assignments.v008".to_string(),
                    assignments: Vec::new(),
                },
            ),
            harbor_routes: HarborRouteCatalog::load_from_path(HARBOR_ROUTE_CATALOG_PATH)
                .unwrap_or_default(),
            route_source_landmass_id: None,
            viewport_mode: EditorViewportMode::SceneRectangles,
            workspace_shell: EditorWorkspaceShellState::load_default(),
            workspace_resize_drag: None,
            saved_undo_depth: 0,
            open_menu: None,
            scene_canvas: CanvasCameraState::default(),
            scene_canvas_states: HashMap::new(),
            autotile_caches: HashMap::new(),
            structural_cliff_caches: HashMap::new(),
            autotile_preview_enabled: true,
            autotile_dirty_overlay: false,
            scene_show_grid: false,
            selected_autotile_preset: 15,
            world_canvas: CanvasCameraState::default(),
            world_canvas_pan_tool: false,
            world_edit_tool: WorldEditTool::Select,
            world_layer_mode: WorldLayerMode::Terrain,
            world_drag: None,
            world_selection: None,
            world_clipboard: None,
            world_structural_level: 1,
            world_brush_radius: 0,
            world_last_painted: None,
            world_show_partitions: false,
            world_show_objects: true,
            world_show_zones: false,
            world_show_structural_levels: false,
            world_cursor_x: 0,
            world_cursor_y: 0,
            scene_bank_canvas: CanvasCameraState::default(),
            scene_bank_pan_tool: false,
            world_canvas_context_menu: None,
            scene_asset_context_menu: None,
            primary_pointer_owned_by_ui: false,
            command_bus: EditorCommandBus::with_limit(16),
            pixel_studio: PixelStudioState::new(),
            animation_studio: AnimationStudioState::new(),
            character_studio: CharacterStudioState::new(),
            development_client: None,
            status_message: format!(
                "Asset palette ready | published aliases canonicalized={} | {} | {}",
                canonicalized_world_assets,
                live_autotile_atlas_registry()
                    .map(|registry| registry.coverage_summary())
                    .unwrap_or_else(|error| format!("live atlas warning: {error}")),
                texture_summary
            ),
        };
        app.restore_generated_harbor_routes();
        if let Ok(descriptor) = development_session::DevelopmentWorldDescriptor::load() {
            let requested_scene = ProjectSceneId::new(descriptor.default_scene.as_str());
            let farmstead_scene = ProjectSceneId::new("farmstead");
            let selected = app
                .model
                .world
                .scenes
                .position(&requested_scene)
                .map(|index| (index, requested_scene.clone(), false))
                .or_else(|| {
                    app.model
                        .world
                        .scenes
                        .position(&farmstead_scene)
                        .map(|index| (index, farmstead_scene.clone(), true))
                });
            if let Some((index, scene_id, used_farmstead_fallback)) = selected {
                app.selected_scene = index;
                app.ensure_scene_visible();
                if let Some(scene) = app.model.world.scenes.get(index) {
                    app.scene_cursor_x = scene.spawn_x;
                    app.scene_cursor_y = scene.spawn_y;
                }
                app.viewport_mode = EditorViewportMode::SceneMap;
                app.status_message = if used_farmstead_fallback {
                    format!(
                        "Development world loaded | {} | default {} unavailable; opened canonical Estate",
                        descriptor.world_id, descriptor.default_scene
                    )
                } else {
                    format!(
                        "Development world loaded | {} | {}",
                        descriptor.world_id, scene_id
                    )
                };
            } else {
                app.status_message = format!(
                    "Development world loaded, but neither default scene {} nor canonical Estate is available",
                    descriptor.default_scene
                );
            }
        }
        app
    }

    pub(crate) async fn load_editor_assets(&mut self) {
        // Load only the compact runtime atlas set during startup. Raw intake
        // source sheets are intentionally loaded on demand from the Intake tab;
        // eager decoding of a large source sheet could block the Windows event
        // loop long enough for the editor window to be ghosted as unresponsive.
        let editor_textures = EditorTextureSet::load().await;
        self.asset_manifest_modified = editor_textures.user_registry().modified();
        let texture_summary = editor_textures.readiness_summary();
        self.editor_textures = editor_textures;
        self.status_message =
            format!("Editor assets loaded | {texture_summary} | intake previews load on demand");
    }
}

pub(crate) fn window_conf() -> Conf {
    let high_dpi = std::env::var("HAVENWILD_EDITOR_HIGH_DPI")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    Conf {
        window_title: "Havenwild Native Editor".to_string(),
        window_width: 1600,
        window_height: 900,
        high_dpi,
        sample_count: 1,
        ..Default::default()
    }
}

fn safe_mode_requested() -> bool {
    std::env::args().any(|argument| argument == "--safe-mode")
        || std::env::var("HAVENWILD_EDITOR_SAFE_MODE")
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
}

fn draw_startup_screen(message: &str) {
    set_default_camera();
    gl_use_default_material();
    clear_background(Color::new(0.025, 0.03, 0.04, 1.0));
    let width = screen_width();
    let height = screen_height();
    let panel = Rect::new(
        (width - 620.0).max(40.0) * 0.5,
        (height - 180.0).max(40.0) * 0.5,
        620.0_f32.min(width - 40.0),
        180.0_f32.min(height - 40.0),
    );
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, PANEL_BG);
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, PANEL_EDGE);
    draw_editor_text(
        "Havenwild Native Editor",
        panel.x + 28.0,
        panel.y + 54.0,
        30.0,
        TEXT,
    );
    draw_editor_text(message, panel.x + 28.0, panel.y + 96.0, 21.0, MUTED);
    draw_editor_text(
        "Use --safe-mode if an imported asset causes startup trouble.",
        panel.x + 28.0,
        panel.y + 135.0,
        16.0,
        MUTED,
    );
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown editor panic".to_string()
    }
}

fn draw_fault_screen(message: &str) {
    set_default_camera();
    gl_use_default_material();
    clear_background(Color::new(0.035, 0.02, 0.025, 1.0));
    draw_editor_text("Editor frame fault contained", 36.0, 58.0, 30.0, WARN);
    draw_editor_text(
        "The editor stayed open so the diagnostic can be read.",
        36.0,
        94.0,
        20.0,
        TEXT,
    );
    draw_scissored_text(
        message,
        36.0,
        136.0,
        (screen_width() - 72.0).max(120.0),
        18.0,
        MUTED,
    );
    draw_editor_text(
        "Press R to rebuild the editor shell in safe mode.",
        36.0,
        184.0,
        18.0,
        TEXT,
    );
    draw_editor_text(
        "See logs/haven_editor_native_crash.log for the full panic record.",
        36.0,
        214.0,
        16.0,
        MUTED,
    );
}

pub(crate) async fn run() {
    let mut safe_mode = safe_mode_requested();
    draw_startup_screen("Creating the editor shell...");
    next_frame().await;

    let mut app = loop {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            EditorApp::new_without_textures,
        )) {
            Ok(app) => break app,
            Err(payload) => {
                let message = format!("Startup fault: {}", panic_message(payload));
                loop {
                    draw_fault_screen(&message);
                    draw_editor_text(
                        "Press R to retry the startup shell, or close the window.",
                        36.0,
                        246.0,
                        17.0,
                        TEXT,
                    );
                    if is_key_pressed(KeyCode::R) {
                        break;
                    }
                    next_frame().await;
                }
            }
        }
    };
    if std::env::args().any(|argument| argument == "--pixel-studio") {
        app.viewport_mode = EditorViewportMode::PixelStudio;
        app.status_message = "Pixel Studio opened from the Bash workflow".to_string();
    }
    if std::env::args().any(|argument| argument == "--animation-studio") {
        app.viewport_mode = EditorViewportMode::AnimationStudio;
        app.status_message = "Animation Studio opened from the Bash workflow".to_string();
    }
    if std::env::args().any(|argument| argument == "--character-studio") {
        app.viewport_mode = EditorViewportMode::CharacterStudio;
        app.status_message = "Character Studio opened from the Bash workflow".to_string();
    }

    crate::append_editor_log(&editor_text::initialize_editor_font());

    let initial_draw = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.draw()));
    if let Err(payload) = initial_draw {
        let message = format!("Initial draw fault: {}", panic_message(payload));
        loop {
            draw_fault_screen(&message);
            if is_key_pressed(KeyCode::R) {
                app = EditorApp::new_without_textures();
                app.status_message = "Recovered in safe mode after initial draw fault".to_string();
                safe_mode = true;
                break;
            }
            next_frame().await;
        }
    }
    next_frame().await;
    if safe_mode {
        app.status_message = "Safe mode active: atlas textures were not loaded".to_string();
    } else {
        draw_startup_screen("Loading optional atlas textures...");
        next_frame().await;
        app.load_editor_assets().await;
        crate::append_editor_log(&format!(
            "post-asset {}",
            editor_text::initialize_editor_font()
        ));
    }

    let mut frame_fault: Option<String> = None;
    let mut last_slow_update_log_at = -10.0_f64;
    let mut last_slow_reload_log_at = -10.0_f64;
    let mut last_slow_draw_log_at = -10.0_f64;
    loop {
        if let Some(message) = frame_fault.as_deref() {
            draw_fault_screen(message);
            if is_key_pressed(KeyCode::R) {
                app = EditorApp::new_without_textures();
                app.status_message =
                    "Recovered in safe mode after a contained frame fault".to_string();
                frame_fault = None;
            }
            next_frame().await;
            continue;
        }

        let update_started = Instant::now();
        let update_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app.update();
            app.poll_asset_hot_reload();
        }));
        let update_elapsed = update_started.elapsed();
        if update_elapsed.as_millis() >= 250 && get_time() - last_slow_update_log_at >= 5.0 {
            last_slow_update_log_at = get_time();
            crate::append_editor_log(&format!(
                "slow editor update: {} ms in {:?}",
                update_elapsed.as_millis(),
                app.viewport_mode
            ));
        }
        if let Err(payload) = update_result {
            frame_fault = Some(format!("Update fault: {}", panic_message(payload)));
            continue;
        }

        let reload_started = Instant::now();
        app.reload_asset_outputs_if_requested().await;
        let reload_elapsed = reload_started.elapsed();
        if reload_elapsed.as_millis() >= 250 && get_time() - last_slow_reload_log_at >= 5.0 {
            last_slow_reload_log_at = get_time();
            crate::append_editor_log(&format!(
                "slow editor asset reload: {} ms",
                reload_elapsed.as_millis()
            ));
        }

        let draw_started = Instant::now();
        let draw_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.draw()));
        let draw_elapsed = draw_started.elapsed();
        if draw_elapsed.as_millis() >= 250 && get_time() - last_slow_draw_log_at >= 5.0 {
            last_slow_draw_log_at = get_time();
            crate::append_editor_log(&format!(
                "slow editor draw: {} ms in {:?}",
                draw_elapsed.as_millis(),
                app.viewport_mode
            ));
        }
        if let Err(payload) = draw_result {
            frame_fault = Some(format!("Draw fault: {}", panic_message(payload)));
            continue;
        }
        next_frame().await;
    }
}
