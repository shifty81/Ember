use ember_assets::AssetRegistry;
use ember_level::load_level_json;
use ember_nodes::BehaviorGraph;
use ember_pixel::PixelDocument;
use ember_project::ProjectManifest;
use ember_runtime::{compile_level, BehaviorLibrary};
use std::fs;
use std::path::PathBuf;

fn certification_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../certification/ember_smoke")
}

#[test]
fn certification_project_proves_authoring_to_runtime_vertical_slice() {
    let root = certification_root();
    let project = ProjectManifest::load(&root.join("ember.project.json")).expect("project loads");
    let level =
        load_level_json(root.join("content/levels/certification.level.json")).expect("level loads");
    let graph: BehaviorGraph = serde_json::from_slice(
        &fs::read(root.join("content/behaviors/certification.behavior.json"))
            .expect("behavior file"),
    )
    .expect("behavior graph parses");
    graph.validate().expect("behavior validates");
    let mut behaviors = BehaviorLibrary::new();
    behaviors.insert(graph.id.clone(), graph);
    let mut world = compile_level(&level, &behaviors).expect("level compiles to runtime");
    world.tick_once().expect("runtime tick");
    assert_eq!(world.entities.len(), 1);
    assert_eq!(world.semantic_cells.len(), 1);

    let pixel: PixelDocument = serde_json::from_slice(
        &fs::read(root.join("content/pixel/certification.pixel.json")).expect("pixel file"),
    )
    .expect("pixel parses");
    pixel.validate().expect("pixel validates");

    let assets =
        AssetRegistry::load(&root.join("content/assets.json")).expect("asset catalog loads");
    assert_eq!(assets.iter().count(), 3);
    assert_eq!(project.name, "Ember Certification Project");
}
