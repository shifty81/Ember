//! Regression tests for Alderreach mainland feature materialization.

use super::*;
use haven_core::{SceneBiome, SceneKind};
use std::collections::BTreeMap;

fn four_scene_surface() -> (Vec<SceneMap>, Vec<ChunkCoord>) {
    let mut scenes = Vec::new();
    let mut chunks = Vec::new();
    for y in 0..2 {
        for x in 0..2 {
            let id = ProjectSceneId::new(format!("pcg_havenwild_mainland_{x}_{y}"));
            let mut scene = SceneMap::blank(
                id,
                format!("mainland {x},{y}"),
                SceneKind::Exterior,
                SceneBiome::Temperate,
            );
            for tile in &mut scene.map.tiles {
                *tile = TileKind::Grass;
            }
            for height in &mut scene.map.heights {
                *height = 48;
            }
            scenes.push(scene);
            chunks.push(ChunkCoord::new(x, y));
        }
    }
    (scenes, chunks)
}

#[test]
fn roads_cross_storage_partitions_without_scene_transitions() {
    let (mut scenes, chunks) = four_scene_surface();
    let harbor = scenes.last().map(|scene| scene.id.clone());
    let report = apply_mainland_surface_features(&mut scenes, &chunks, harbor.as_ref(), 83, false)
        .expect("mainland features");
    assert!(report.road_tiles > 0);
    assert!(report.road_partitions >= 2);
    assert!(scenes.iter().all(|scene| scene.transitions.is_empty()));
}

#[test]
fn preserve_mode_does_not_repaint_existing_generated_city_roads() {
    let (mut scenes, chunks) = four_scene_surface();
    for index in 0..300 {
        let x = (index % MAP_W) as i32;
        let y = (index / MAP_W) as i32;
        scenes[0].map.set(x, y, TileKind::Road);
    }
    let report = apply_mainland_surface_features(&mut scenes, &chunks, None, 83, true)
        .expect("preserved mainland features");
    assert_eq!(report.road_tiles, 0);
    assert_eq!(scenes[0].map.get(10, 0), TileKind::Road);
}

#[test]
fn one_player_road_does_not_suppress_missing_capital_generation() {
    let (mut scenes, chunks) = four_scene_surface();
    scenes[0].map.set(10, 10, TileKind::Road);
    let report = apply_mainland_surface_features(&mut scenes, &chunks, None, 84, true)
        .expect("missing capital repaired");
    assert!(report.road_tiles > 0);
}

#[test]
fn city_lots_are_metadata_not_giant_stone_path_rectangles() {
    let (mut scenes, chunks) = four_scene_surface();
    let harbor = scenes.last().map(|scene| scene.id.clone());
    let report = apply_mainland_surface_features(&mut scenes, &chunks, harbor.as_ref(), 84, false)
        .expect("mainland features");
    assert!(report.city_plot_reservations > 0);
    assert!(report.city_reserved_tiles > 0);
    let stone_paths = scenes
        .iter()
        .flat_map(|scene| scene.map.tiles.iter())
        .filter(|tile| **tile == TileKind::StonePath)
        .count();
    assert!(
        stone_paths <= 121,
        "only the compact civic plaza may use StonePath"
    );
    assert!(scenes.iter().all(|scene| {
        scene
            .map
            .objects
            .iter()
            .all(|object| object.kind != ObjectKind::Well)
    }));
}

#[test]
fn civic_plaza_isolated_from_unsupported_brown_dirt_contacts() {
    let (mut scenes, chunks) = four_scene_surface();
    for scene in &mut scenes {
        for tile in &mut scene.map.tiles {
            *tile = TileKind::Dirt;
        }
    }
    let harbor = scenes.last().map(|scene| scene.id.clone());
    let report = apply_mainland_surface_features(&mut scenes, &chunks, harbor.as_ref(), 86, false)
        .expect("mainland features");
    assert!(report.road_tiles > 0);

    let mut global_tiles = BTreeMap::new();
    for (scene, chunk) in scenes.iter().zip(&chunks) {
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                global_tiles.insert(
                    (chunk.x * MAP_W as i32 + x, chunk.y * MAP_H as i32 + y),
                    scene.map.get(x, y),
                );
            }
        }
    }

    let stone_cells = global_tiles
        .iter()
        .filter_map(|(cell, tile)| (*tile == TileKind::StonePath).then_some(*cell))
        .collect::<Vec<_>>();
    assert!(
        !stone_cells.is_empty() && stone_cells.len() <= 121,
        "the compact plaza may be crossed by Road cells but must retain StonePath surface"
    );
    for (x, y) in stone_cells {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            let Some(neighbor) = global_tiles.get(&(nx, ny)) else {
                continue;
            };
            assert!(
                matches!(*neighbor, TileKind::StonePath | TileKind::Road),
                "StonePath at {x},{y} touched unsupported {neighbor:?} at {nx},{ny}"
            );
        }
    }
}
