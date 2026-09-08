#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationRegistryStatus {
    Active,
    ContractOnly,
    Planned,
}

impl ValidationRegistryStatus {
    pub fn label(self) -> &'static str {
        match self {
            ValidationRegistryStatus::Active => "active",
            ValidationRegistryStatus::ContractOnly => "contract-only",
            ValidationRegistryStatus::Planned => "planned",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationRegistryEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub owner: &'static str,
    pub status: ValidationRegistryStatus,
    pub source_path: &'static str,
}

pub fn validation_registry() -> Vec<ValidationRegistryEntry> {
    vec![
        ValidationRegistryEntry {
            id: "world",
            title: "World scenes",
            owner: "haven_editor",
            status: ValidationRegistryStatus::Active,
            source_path: "crates/haven_editor/src/lib.rs",
        },
        ValidationRegistryEntry {
            id: "generated_assets",
            title: "Generated asset contract",
            owner: "haven_assets::asset_registry",
            status: ValidationRegistryStatus::Active,
            source_path: "assets/generated/worldgen_v0_1/worldgen_asset_manifest_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "region_graph",
            title: "Island region graph",
            owner: "haven_world::region_graph",
            status: ValidationRegistryStatus::Active,
            source_path: "crates/haven_world/src/region_graph.rs",
        },
        ValidationRegistryEntry {
            id: "scene_rectangles",
            title: "Scene rectangle contract",
            owner: "haven_world::scene_rectangles",
            status: ValidationRegistryStatus::Active,
            source_path: "content/worldgen/scene_rectangle_manifest_v0_8.json",
        },
        ValidationRegistryEntry {
            id: "project_file",
            title: "Editor project shell",
            owner: "haven_editor::project_file",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/project_file_starter_v0_9.json",
        },
        ValidationRegistryEntry {
            id: "editor_commands",
            title: "Shared editor command bus",
            owner: "haven_authoring::command_bus",
            status: ValidationRegistryStatus::Active,
            source_path: "content/schemas/editor_command.schema.v009.json",
        },
        ValidationRegistryEntry {
            id: "stable_selection",
            title: "Stable authored selection and canvas hit testing",
            owner: "haven_authoring",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/selection/stable_selection_hit_testing_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "typed_edit_transactions",
            title: "Typed gesture transaction execution",
            owner: "haven_authoring::transactions",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/transactions/transactional_undo_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "production_infinite_canvas",
            title: "Production infinite-canvas authoring tools",
            owner: "haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/canvas/production_infinite_canvas_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "live_autotile_authoring",
            title: "Live autotile and terrain-transition authoring",
            owner: "haven_world::autotile + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/autotile/live_autotile_authoring_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "asset_palette_atlas_binding",
            title: "Production asset palette and atlas binding",
            owner: "haven_assets + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/assets/asset_palette_atlas_binding_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "asset_intake_atlas_authoring",
            title: "Asset intake, promotion gate, deterministic atlas bake, and hot reload",
            owner: "haven_assets + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/assets/asset_intake_atlas_authoring_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "camera_zoom_orientation_hotfix",
            title: "Editor/runtime camera zoom and orientation regression guard",
            owner: "haven_editor_native + haven_game",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/canvas/camera_zoom_orientation_hotfix_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "world_canvas_island_pcg_harbor",
            title: "World canvas context actions, procedural islands, and harbor travel routes",
            owner: "haven_world + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/world_canvas/island_context_pcg_harbor_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "production_object_outliner_inspector",
            title: "Production object outliner, inspector, and scene lifecycle",
            owner: "haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path:
                "content/editor/objects/production_object_outliner_inspector_contract_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "animation_contract",
            title: "Character animation contract",
            owner: "haven_assets::animation_contract",
            status: ValidationRegistryStatus::Active,
            source_path: "content/animations/character_animation_contract_v0_10.json",
        },
        ValidationRegistryEntry {
            id: "open_world_generation",
            title: "Havenwild open-world generation preset",
            owner: "haven_world::open_world",
            status: ValidationRegistryStatus::Active,
            source_path: "content/worldgen/havenwild_open_world_preset_v1.json",
        },
        ValidationRegistryEntry {
            id: "procedural_caves",
            title: "Procedural cave weak spots and rope ladders",
            owner: "haven_world::open_world",
            status: ValidationRegistryStatus::ContractOnly,
            source_path: "crates/haven_world/src/open_world.rs",
        },
        ValidationRegistryEntry {
            id: "pixel_editor",
            title: "Native Pixel Studio vertical slice",
            owner: "haven_pixel + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/pixel_editor/native_pixel_studio_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "pixel_document_layers",
            title: "Layered Pixel Studio document persistence and recovery",
            owner: "haven_pixel + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/pixel_editor/pixel_document_layer_system_v0_2.json",
        },
        ValidationRegistryEntry {
            id: "animation_studio",
            title: "Native Animation Studio timeline and clip authoring",
            owner: "haven_pixel + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/pixel_editor/native_animation_studio_v0_1.json",
        },
        ValidationRegistryEntry {
            id: "pixel_animation_roundtrip",
            title: "Pixel Studio and Animation Studio frame round-trip bridge",
            owner: "haven_pixel + haven_editor_native",
            status: ValidationRegistryStatus::Active,
            source_path: "content/editor/pixel_editor/pixel_animation_roundtrip_bridge_v0_1.json",
        },
    ]
}
