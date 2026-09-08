use super::*;
use haven_core::{TavernMap, TileKind};

#[test]
fn starter_world_passes_editor_validation() {
    let world = GameWorld::starter();
    let warnings = validate_world(&world);
    assert!(
        warnings.is_empty(),
        "starter world validation warnings: {warnings:#?}"
    );
}

#[test]
fn starter_island_region_graph_passes_editor_validation() {
    let graph = haven_world::region_graph::starter_island_region_graph();
    let warnings = validate_region_graph(&graph);
    assert!(
        warnings.is_empty(),
        "starter island graph validation warnings: {warnings:#?}"
    );
}

#[test]
fn starter_island_region_graph_matches_starter_world() {
    let world = GameWorld::starter();
    let graph = haven_world::region_graph::starter_island_region_graph();
    let warnings = validate_region_graph_against_world(&graph, &world);
    assert!(
        warnings.is_empty(),
        "starter island graph/world alignment warnings: {warnings:#?}"
    );
}

#[test]
fn starter_editor_world_model_passes_validation() {
    let world = GameWorld::starter();
    let graph = haven_world::region_graph::starter_island_region_graph();
    let warnings = validate_editor_world_model(&world, &graph);
    assert!(
        warnings.is_empty(),
        "starter editor model warnings: {warnings:#?}"
    );
}

#[test]
fn region_graph_inspector_summarizes_nodes_and_links() {
    let graph = haven_world::region_graph::starter_island_region_graph();
    let report = inspect_region_graph(&graph);
    assert_eq!(report.title, "Starter Island Region");
    assert!(report.lines.iter().any(|line| line == "Nodes: 7"));
    assert!(report.lines.iter().any(|line| line == "Links: 6"));
    assert!(report
        .lines
        .iter()
        .any(|line| line.contains("Estate [hub] Estate")));
}

#[test]
fn editor_world_model_starter_wraps_world_and_region_graph() {
    let model = EditorWorldModel::starter();
    assert_eq!(model.world_scene_count(), 9);
    assert_eq!(model.linked_region_scene_count(), 6);
    assert!(model.validate().is_empty());
    assert_eq!(model.region_report().title, "Starter Island Region");
}

#[test]
fn editor_world_model_reports_validation_by_section() {
    let model = EditorWorldModel::starter();
    let report = model.validation_report();
    assert!(report.is_clean());
    assert_eq!(report.total_issue_count(), 0);
    assert_eq!(report.sections.len(), 8);
    assert_eq!(report.sections[0].id, "world");
    assert_eq!(report.sections[1].id, "generated_assets");
    assert_eq!(report.sections[2].id, "region_graph");
    assert_eq!(report.sections[3].id, "world_region_alignment");
    assert_eq!(report.sections[4].id, "scene_rectangles");
    assert_eq!(report.sections[5].id, "scene_rectangle_assignments");
    assert_eq!(report.sections[6].id, "project_file");
    assert_eq!(report.sections[7].id, "animation_contract");
}

#[test]
fn inspector_reports_autotile_state_for_shared_core_system() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Road);
    map.set(5, 4, TileKind::StonePath);
    let report = inspect_cell(&map, 4, 4);
    assert!(report
        .lines
        .iter()
        .any(|line| line == "Autotile group: Road"));
    assert!(report
        .lines
        .iter()
        .any(|line| line == "Autotile mask: 0x04"));
}

#[test]
fn system_audit_tracks_shared_core_editor_runtime_shape() {
    let audit = editor_system_audit();
    assert!(audit.iter().any(|system| system.name == "Autotiling"
        && system.core_owner == "haven_assets::autotile"
        && system.status == EditorSystemStatus::Shared));
    assert!(audit.iter().any(|system| system.name == "Asset catalog"
        && system.core_owner == "haven_assets::asset_registry"
        && system.runtime_consumer == "generated terrain atlas lookup"
        && system.status == EditorSystemStatus::Shared));
    assert!(audit
        .iter()
        .any(|system| system.name == "Animation catalog"
            && system.status == EditorSystemStatus::Partial));
    assert!(audit
        .iter()
        .any(|system| system.name == "Shared command bus"
            && system.status == EditorSystemStatus::Shared));
}

#[test]
fn generated_asset_summary_reports_runtime_bindings() {
    let summary = generated_asset_registry_summary();
    let registry = generated_asset_registry().expect("generated asset registry");
    assert!(summary
        .iter()
        .any(|line| line == &format!("Manifest: {}", registry.manifest_id())));
    assert!(summary
        .iter()
        .any(|line| line == "Terrain manifest: common_base_terrain_32"));
    let expected_binding_count = registry.tile_entries().len();
    assert!(summary
        .iter()
        .any(|line| line == &format!("Tile bindings: {expected_binding_count}")));
}
