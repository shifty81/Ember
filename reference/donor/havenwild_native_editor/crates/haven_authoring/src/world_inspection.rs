use haven_assets::autotile::autotile_mask_8way;
use haven_core::{SceneMap, TavernMap, TileKind};

use crate::InspectorReport;

pub fn inspect_cell(map: &TavernMap, x: i32, y: i32) -> InspectorReport {
    let tile = map.get(x, y);
    let mut lines = vec![
        format!("Cell: {}, {}", x, y),
        format!("Tile: {}", tile.label()),
        format!("Category: {}", tile.category().label()),
        format!("Height: {}", map.get_height(x, y)),
        format!(
            "Walkable: {}",
            if map.collision_at(x, y).blocked {
                "no"
            } else {
                "yes"
            }
        ),
        format!("Collision: {:?}", map.collision_at(x, y).reason),
    ];

    if let Some(index) = map.object_at(x, y) {
        let object = map.objects[index];
        lines.push(format!("Object: {}", object.kind.label()));
        lines.push(format!("Footprint: {}", object.footprint_label()));
        lines.push(format!(
            "Blocks cell: {}",
            if object.blocks_tile(x, y) {
                "yes"
            } else {
                "no"
            }
        ));
    } else {
        lines.push("Object: none".to_string());
    }
    if let Some(index) = map.blocking_object_at(x, y) {
        lines.push(format!(
            "Blocked by object: {}",
            map.objects[index].kind.label()
        ));
    }

    if let Some(group) = tile.autotile_group() {
        lines.push(format!("Autotile group: {}", group.label()));
        lines.push(format!(
            "Autotile mask: {:#04x}",
            autotile_mask_8way(map, tile, x, y)
        ));
    }

    if tile == TileKind::GreenhouseZone {
        lines.push("Zone: year-round crop candidate".to_string());
    }
    if tile == TileKind::TilledSoil {
        lines.push("Farm: ready for seed/water systems".to_string());
    }

    InspectorReport {
        title: "Inspector".to_string(),
        lines,
    }
}

pub fn inspect_scene_cell(scene: &SceneMap, x: i32, y: i32) -> InspectorReport {
    let mut report = inspect_cell(&scene.map, x, y);
    report.lines.insert(0, format!("Scene: {}", scene.name));
    report
        .lines
        .insert(1, format!("Biome: {}", scene.biome.label()));
    report
        .lines
        .push(format!("Zone: {}", scene.zone_at(x, y).label()));
    if let Some(transition) = scene.transition_at(x, y) {
        report.lines.push(format!(
            "Transition: {} -> {} ({}, {})",
            transition.label,
            transition.target.label(),
            transition.spawn_x,
            transition.spawn_y
        ));
    } else {
        report.lines.push("Transition: none".to_string());
    }
    report
}
