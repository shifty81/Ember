use ember_animation::{clip_from_tag, ClipPlayback};
use ember_bridge::{AdapterRouter, RouteRecord, RouteTarget};
use ember_capabilities::{CapabilityCatalog, CapabilityRecord, CapabilityState};
use ember_commands::CommandHistory;
use ember_core::{ProjectPath, StableId};
use ember_graph::{
    GraphConnection, PortDefinition, PortDirection, PortKind, TypedGraph, TypedNode, TypedNodeKind,
};
use ember_ide::{apply_text_edits, TextEdit};
use ember_level::{tile_layer, LevelDocument};
use ember_level_tools::{FillTilesCommand, RectI};
use ember_library::{LibraryCatalog, LibraryEntry, LibraryKind, LibraryLocator};
use ember_packaging::{CookEntry, CookKind, CookPlan, ReleaseManifest};
use ember_pixel::{FrameTag, PixelDocument, PlaybackDirection, RepeatMode};
use ember_project::BuildTarget;
use ember_spatial::{SpatialMode, SpatialTransform, Transform2PointFiveD};
use ember_studios::{AudioEventDocument, AudioLayer};
use ember_terrain::{Cell2, SurfaceRole, TerrainCell, TerrainComposer, MASK_EAST};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn fnd_11_to_20_cross_lane_certification() {
    let grass = StableId::new("terrain_material", "grass").unwrap();
    let mut terrain = TerrainComposer::default();
    terrain.set(
        Cell2::new(0, 0),
        TerrainCell {
            material: grass.clone(),
            elevation: 0,
            role: SurfaceRole::Ground,
            visual_variant: 0,
        },
    );
    terrain.set(
        Cell2::new(1, 0),
        TerrainCell {
            material: grass,
            elevation: 0,
            role: SurfaceRole::Ground,
            visual_variant: 0,
        },
    );
    assert_eq!(terrain.cardinal_mask(Cell2::new(0, 0)), MASK_EAST);

    let mut pixel = PixelDocument::new(8, 8);
    let frame2 = pixel.add_frame(90);
    pixel.frame_tags.push(FrameTag {
        name: "idle".into(),
        from_frame: 1,
        to_frame: frame2,
        direction: PlaybackDirection::Forward,
        repeat: RepeatMode::Loop,
    });
    let clip = clip_from_tag(&pixel, "idle", StableId::new("animation", "idle").unwrap()).unwrap();
    assert_eq!(clip.playback, ClipPlayback::Loop);

    let mut level = LevelDocument::new(
        StableId::new("level", "fnd-v2").unwrap(),
        "FND V2",
        128,
        128,
    );
    let layer = StableId::new("layer", "ground").unwrap();
    level
        .scene
        .layers
        .push(tile_layer(layer.clone(), "Ground", 16, 0));
    let mut history = CommandHistory::default();
    history
        .execute(
            Box::new(FillTilesCommand::new(
                layer.clone(),
                RectI {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                1,
                [0, 0],
                None,
            )),
            &mut level,
        )
        .unwrap();
    assert_eq!(level.layer(&layer).unwrap().tile_cells.len(), 4);

    let start = StableId::new("node", "start").unwrap();
    let end = StableId::new("node", "end").unwrap();
    let exec_out = PortDefinition {
        id: "out".into(),
        direction: PortDirection::Output,
        kind: PortKind::Execution,
        value_type: None,
    };
    let exec_in = PortDefinition {
        id: "in".into(),
        direction: PortDirection::Input,
        kind: PortKind::Execution,
        value_type: None,
    };
    let graph = TypedGraph {
        id: StableId::new("graph", "fnd-v2").unwrap(),
        entry: start.clone(),
        nodes: BTreeMap::from([
            (
                start.clone(),
                TypedNode {
                    id: start.clone(),
                    kind: TypedNodeKind::EventStart,
                    ports: vec![exec_out],
                },
            ),
            (
                end.clone(),
                TypedNode {
                    id: end.clone(),
                    kind: TypedNodeKind::End,
                    ports: vec![exec_in],
                },
            ),
        ]),
        connections: vec![GraphConnection {
            from_node: start,
            from_port: "out".into(),
            to_node: end,
            to_port: "in".into(),
            order: 0,
        }],
    };
    assert_eq!(graph.compile_behavior().unwrap().nodes.len(), 2);

    let audio = AudioEventDocument {
        id: StableId::new("audio_event", "click").unwrap(),
        layers: vec![AudioLayer {
            source: StableId::new("audio", "click").unwrap(),
            gain_db: 0.0,
            pitch: 1.0,
            looped: false,
            start_delay_ms: 0,
        }],
        parameters: BTreeMap::new(),
    };
    audio.validate().unwrap();

    let spatial = SpatialTransform::TwoPointFiveD(Transform2PointFiveD {
        planar: [4.0, 8.0],
        elevation: 2.0,
        yaw_degrees: 0.0,
        scale: [1.0, 1.0],
        depth_bias: 0.0,
    });
    assert_eq!(spatial.mode(), SpatialMode::TwoPointFiveD);

    assert_eq!(
        apply_text_edits(
            "hello world",
            &[TextEdit {
                start_byte: 6,
                end_byte: 11,
                replacement: "ember".into(),
            }],
        )
        .unwrap(),
        "hello ember"
    );

    let catalog = CapabilityCatalog {
        schema_version: 1,
        project_id: "ember".into(),
        capabilities: vec![CapabilityRecord {
            id: "project.status".into(),
            version: 1,
            state: CapabilityState::Certified,
            owner: "pcc".into(),
            mutation: false,
            requires_approval: false,
            platforms: vec![],
            dependencies: vec![],
            evidence: vec![],
        }],
    };
    let mut router = AdapterRouter::default();
    router.register(RouteRecord {
        tool: "ember.project.status".into(),
        capability: "project.status".into(),
        target: RouteTarget::ProjectControlCenter,
    });
    assert_eq!(
        router
            .resolve("ember.project.status", &catalog)
            .unwrap()
            .target,
        RouteTarget::ProjectControlCenter
    );

    let plan = CookPlan {
        schema_version: 1,
        project_id: StableId::new("project", "fnd-v2").unwrap(),
        target: BuildTarget::WindowsPortable,
        entries: vec![CookEntry {
            source: ProjectPath::parse("content/a").unwrap(),
            output: ProjectPath::parse("data/a").unwrap(),
            kind: CookKind::Asset,
            required: true,
        }],
    };
    assert_eq!(ReleaseManifest::from_plan(&plan).unwrap().files.len(), 1);

    let library_id = StableId::new("library", "starter").unwrap();
    let mut library = LibraryCatalog::default();
    library.insert(LibraryEntry {
        id: library_id.clone(),
        kind: LibraryKind::Template,
        version: "1.0.0".into(),
        locator: LibraryLocator::Vault {
            namespace: "ember".into(),
            key: "templates/starter".into(),
            version: "1.0.0".into(),
        },
        dependencies: vec![],
        tags: BTreeSet::new(),
        provenance: Some("Ember".into()),
    });
    library.validate().unwrap();
    assert!(library.entries.contains_key(&library_id));
}
