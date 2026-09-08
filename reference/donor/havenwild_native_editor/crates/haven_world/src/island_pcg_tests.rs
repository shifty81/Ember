#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_rectangles::{
        load_scene_rectangle_manifest_from_path, SCENE_RECTANGLE_MANIFEST_PATH,
    };
    use haven_core::GameWorld;
    use std::path::Path;

    fn manifest() -> SceneRectangleManifest {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("haven_world crate should live under crates/<name>");
        let manifest_path = repo_root.join(SCENE_RECTANGLE_MANIFEST_PATH);
        load_scene_rectangle_manifest_from_path(&manifest_path.to_string_lossy()).expect("manifest")
    }

    #[test]
    fn default_geology_can_reach_the_mountain_rock_threshold() {
        let settings = IslandGenerationSettings::default();
        let theoretical_peak =
            48.0 + 88.0 + 148.0 * settings.mountain_radius.clamp(0.0, 1.0) as f64;

        assert!(
            theoretical_peak >= f64::from(crate::highland_generation::MOUNTAIN_ROCK_HEIGHT),
            "default geological peak {theoretical_peak:.2} must reach mountain threshold {}",
            crate::highland_generation::MOUNTAIN_ROCK_HEIGHT
        );
    }

    #[test]
    fn generated_open_surface_does_not_emit_synthetic_cliff_walls() {
        let manifest = manifest();
        let generated = generate_landmass(&manifest, 0, IslandGenerationSettings::default())
            .expect("mainland generation");

        assert!(generated.scenes.iter().all(|entry| !entry
            .scene
            .map
            .tiles
            .contains(&TileKind::Cliff)));
    }

    #[test]
    fn main_island_generation_spans_many_empty_editable_scenes_with_shared_coastline() {
        let generated = generate_landmass(&manifest(), 0, IslandGenerationSettings::default())
            .expect("main island");
        assert_eq!(generated.scenes.len(), 40);
        assert!(generated.land_cells > 10_000);
        assert!(generated.mountain_cells > 0);
        assert!(generated.shoreline_cells > 100);
        assert!(generated.natural_objects > 0);
        assert!(generated
            .scenes
            .iter()
            .any(
                |entry| entry.scene.map.objects.iter().any(|object| matches!(
                    object.kind,
                    ObjectKind::Tree | ObjectKind::Bush | ObjectKind::Mushroom | ObjectKind::Herb
                ))
            ));
        assert!(generated.mainland_features.road_tiles > 0);
        assert!(generated.mainland_features.road_partitions > 1);
        assert!(generated.mainland_features.city_plot_reservations > 0);
        assert!(generated.mainland_features.city_reserved_tiles > 0);
        assert!(generated.mainland_features.harbor_reserved_tiles > 0);
        assert!(generated.structural_landforms.level_one_cells > 0);
        assert!(generated.structural_landforms.cliff_boundaries > 0);
        assert!(generated.structural_landforms.generated_ramps > 0);
        assert!(generated.world_plan.is_mainland());
        assert_eq!(
            generated
                .world_plan
                .target_mainland
                .as_ref()
                .expect("mainland target plan")
                .cities
                .len(),
            3
        );
        assert_eq!(generated.landmass_name, "Alderreach");
        assert!(generated.scenes.iter().any(|entry| {
            entry
                .scene
                .map
                .objects
                .iter()
                .any(|object| matches!(object.kind, ObjectKind::Boulder | ObjectKind::OreNode))
        }));
        assert!(generated.scenes.iter().all(|entry| {
            (0..MAP_H as i32).all(|y| {
                (0..MAP_W as i32).all(|x| {
                    matches!(
                        entry.scene.map.get(x, y),
                        TileKind::DeepWater
                            | TileKind::ShallowWater
                            | TileKind::OceanDeep
                            | TileKind::OceanShallow
                            | TileKind::Sand
                            | TileKind::WetSand
                            | TileKind::Grass
                            | TileKind::Dirt
                            | TileKind::Road
                            | TileKind::StonePath
                            | TileKind::Cliff
                            | TileKind::MountainRock
                            | TileKind::MountainPath
                    )
                })
            })
        }));
        assert!(generated
            .scenes
            .iter()
            .any(|entry| entry.rectangle_id == generated.harbor_rectangle_id));
    }

    #[test]
    fn generated_exterior_partitions_have_no_scene_edge_transitions() {
        let generated = generate_landmass(&manifest(), 0, IslandGenerationSettings::default())
            .expect("main island");
        assert!(generated
            .scenes
            .iter()
            .all(|entry| entry.scene.transitions.is_empty()));
    }

    #[test]
    fn every_manifest_landmass_generates_as_one_scene_cell_assembly() {
        let manifest = manifest();
        let mut ids: Vec<i32> = manifest
            .scene_rectangles
            .iter()
            .filter_map(|entry| entry.grid_x.zip(entry.grid_y).map(|_| entry.landmass_id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        for landmass_id in ids {
            let generated = generate_landmass(
                &manifest,
                landmass_id,
                IslandGenerationSettings {
                    seed: IslandGenerationSettings::default().seed ^ landmass_id as u64,
                    ..IslandGenerationSettings::default()
                },
            )
            .expect("landmass");
            assert!(!generated.scenes.is_empty());
            assert!(generated.land_cells > 0);
        }
    }
    #[test]
    fn generated_scene_ids_encode_negative_partition_coordinates_losslessly() {
        let mut rectangle = manifest().scene_rectangles[0].clone();
        rectangle.grid_x = Some(-2);
        rectangle.grid_y = Some(7);

        let scene_id = generated_scene_id("Havenwild Mainland", &rectangle);
        assert_eq!(scene_id.code(), "pcg_havenwild_mainland_n2_7");
        assert_eq!(
            crate::continuous_surface::parse_pcg_surface_scene_id(&scene_id),
            Some((
                "havenwild_mainland".to_string(),
                crate::open_world::ChunkCoord::new(-2, 7),
            ))
        );
    }

    #[test]
    fn natural_object_population_is_deterministic_and_stays_on_grass() {
        let manifest = manifest();
        let first = generate_landmass(&manifest, 0, IslandGenerationSettings::default())
            .expect("first generation");
        let second = generate_landmass(&manifest, 0, IslandGenerationSettings::default())
            .expect("second generation");
        let first_objects = first
            .scenes
            .iter()
            .flat_map(|entry| entry.scene.map.objects.iter())
            .map(|object| (object.kind.code(), object.x, object.y))
            .collect::<Vec<_>>();
        let second_objects = second
            .scenes
            .iter()
            .flat_map(|entry| entry.scene.map.objects.iter())
            .map(|object| (object.kind.code(), object.x, object.y))
            .collect::<Vec<_>>();
        assert_eq!(first_objects, second_objects);
        for entry in &first.scenes {
            for object in &entry.scene.map.objects {
                let tile = entry.scene.map.get(object.x, object.y);
                match object.kind {
                    ObjectKind::Tree
                    | ObjectKind::Bush
                    | ObjectKind::Mushroom
                    | ObjectKind::Herb => assert_eq!(tile, TileKind::Grass),
                    ObjectKind::Boulder => assert!(matches!(
                        tile,
                        TileKind::Grass | TileKind::Dirt | TileKind::MountainRock
                    )),
                    ObjectKind::OreNode => assert_eq!(tile, TileKind::MountainRock),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn natural_object_restore_tops_up_partially_populated_pcg_partitions() {
        let manifest = manifest();
        let mut generated = generate_landmass(
            &manifest,
            0,
            IslandGenerationSettings {
                tree_density: 0.0,
                ..IslandGenerationSettings::default()
            },
        )
        .expect("mainland generation");
        let mut target = None;
        'scenes: for (scene_index, entry) in generated.scenes.iter().enumerate() {
            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    if entry.scene.map.get(x, y) == TileKind::Grass {
                        target = Some((scene_index, x, y));
                        break 'scenes;
                    }
                }
            }
        }
        let (scene_index, grass_x, grass_y) = target.expect("generated mainland grass cell");
        let _ = generated.scenes[scene_index].scene.map.place_object(
            ObjectKind::Herb,
            grass_x,
            grass_y,
        );

        let mut world = GameWorld::starter();
        for scene in generated.scenes {
            world.insert_scene(scene.scene).expect("insert PCG scene");
        }
        let before = world
            .scenes
            .iter()
            .map(|scene| scene.map.objects.len())
            .sum::<usize>();
        let report = populate_missing_pcg_natural_objects(&mut world, 4_242);
        let after = world
            .scenes
            .iter()
            .map(|scene| scene.map.objects.len())
            .sum::<usize>();

        assert!(report.total_objects() > 0);
        assert!(after > before);
    }

    #[test]
    fn moved_scene_cell_assemblies_grow_coast_against_empty_grid_slots() {
        let mut manifest = manifest();
        manifest.scene_rectangles.retain(|entry| {
            entry.landmass_id != 2 || entry.grid_x != Some(1) || entry.grid_y != Some(1)
        });
        manifest.scene_count = manifest
            .scene_rectangles
            .iter()
            .filter(|entry| entry.grid_x.is_some() && entry.grid_y.is_some())
            .count();
        let generated = generate_landmass(&manifest, 2, IslandGenerationSettings::default())
            .expect("L-shaped island assembly");
        let north_east = generated
            .scenes
            .iter()
            .find(|entry| entry.rectangle_id == "S023")
            .expect("north-east Gullmere scene cell");
        assert!((0..MAP_W as i32).all(|x| {
            matches!(
                north_east.scene.map.get(x, MAP_H as i32 - 1),
                TileKind::DeepWater
                    | TileKind::ShallowWater
                    | TileKind::OceanDeep
                    | TileKind::OceanShallow
                    | TileKind::Sand
                    | TileKind::WetSand
            )
        }));
    }
}
