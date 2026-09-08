use super::autotile_render::draw_live_autotile_preview;
use super::*;

pub(crate) struct SceneTilemapDraw<'a> {
    pub scene: &'a SceneMap,
    pub cursor: Option<(i32, i32)>,
    pub layer_mode: SceneLayerMode,
    pub selection: &'a EditorSelection,
    pub layer_states: &'a [SceneLayerState; 4],
    pub autotile_cache: Option<&'a LiveAutotileCache>,
    pub structural_cliff_bridge: Option<&'a haven_world::LegacyCliffBridgeResultV2>,
    pub textures: &'a EditorTextureSet,
    pub autotile_preview_enabled: bool,
    pub autotile_dirty_overlay: bool,
    pub visible_cells: Option<GridRect>,
    pub zoom: f32,
}

pub(crate) fn draw_scene_tilemap(draw: SceneTilemapDraw<'_>) {
    let SceneTilemapDraw {
        scene,
        cursor,
        layer_mode,
        selection,
        layer_states,
        autotile_cache,
        structural_cliff_bridge,
        textures,
        autotile_preview_enabled,
        autotile_dirty_overlay,
        visible_cells,
        zoom,
    } = draw;
    let terrain_state = layer_states[SceneLayerMode::Terrain.index()];
    if terrain_state.visible {
        if let Some(visible_cells) = visible_cells {
            for y in visible_cells.min.y..=visible_cells.max.y {
                for x in visible_cells.min.x..=visible_cells.max.x {
                if !scene.is_renderable_cell(x, y) {
                    continue;
                }
                if !textures.draw_tile(scene, autotile_cache, x, y, terrain_state.opacity) {
                    let tile = scene.map.get(x, y);
                    let mut color = scene_tile_color(tile);
                    color.a *= terrain_state.opacity;
                    draw_rectangle(x as f32, y as f32, 1.01, 1.01, color);
                }
                }
            }
            for y in visible_cells.min.y..=visible_cells.max.y {
                for x in visible_cells.min.x..=visible_cells.max.x {
                if !scene.is_renderable_cell(x, y) {
                    continue;
                }
                let _ = textures.draw_terrain_tuple_overlay(scene, x, y, terrain_state.opacity);
                }
            }
        }
        if let Some(structural) = structural_cliff_bridge {
            let _ = textures.draw_structural_cliffs(
                scene,
                structural,
                visible_cells,
                terrain_state.opacity,
            );
        }
        if autotile_preview_enabled {
            if let Some(cache) = autotile_cache {
                draw_live_autotile_preview(
                    cache,
                    terrain_state.opacity,
                    autotile_dirty_overlay,
                    zoom,
                );
            }
        }
        if autotile_dirty_overlay {
            draw_terrain_tuple_issue_overlay(scene, visible_cells, zoom);
        }
    }

    let zone_state = layer_states[SceneLayerMode::Zones.index()];
    if zone_state.visible {
        if let Some(visible_cells) = visible_cells {
            for y in visible_cells.min.y..=visible_cells.max.y {
                for x in visible_cells.min.x..=visible_cells.max.x {
                if !scene.is_renderable_cell(x, y) {
                    continue;
                }
                let Some(mut color) = zone_preview_color(scene.zone_at(x, y)) else {
                    continue;
                };
                color.a *= zone_state.opacity;
                if layer_mode != SceneLayerMode::Zones {
                    color.a *= 0.45;
                }
                draw_rectangle(x as f32, y as f32, 1.01, 1.01, color);
                }
            }
        }
    }

    let transition_state = layer_states[SceneLayerMode::Transitions.index()];
    if transition_state.visible {
        for transition in &scene.transitions {
            let is_selected = selection
                .items
                .contains(&SelectionItem::Transition(transition.id));
            let mut color = if is_selected {
                TEXT
            } else if layer_mode == SceneLayerMode::Transitions {
                Color::new(1.0, 0.82, 0.25, 1.0)
            } else {
                Color::new(0.95, 0.68, 0.28, 0.45)
            };
            color.a *= transition_state.opacity;
            draw_rectangle(
                transition.x as f32,
                transition.y as f32,
                transition.w as f32,
                transition.h as f32,
                Color::new(color.r, color.g, color.b, color.a * 0.12),
            );
            draw_rectangle_lines(
                transition.x as f32,
                transition.y as f32,
                transition.w as f32,
                transition.h as f32,
                if is_selected { 0.20 } else { 0.10 },
                color,
            );
        }
    }

    let object_state = layer_states[SceneLayerMode::Objects.index()];
    if object_state.visible {
        enum ObjectDrawCommand<'a> {
            Stamp(&'a PlacedStamp),
            Object(&'a PlacedObject),
        }

        // Match runtime actor ordering: low/detail objects are laid down first,
        // then stamps and ordinary objects share one bottom-root sort stream.
        // This prevents tall LPC art from winning merely because it happened to
        // be stored later in the scene or belonged to a different collection.
        for object in &scene.map.objects {
            if matches!(object.kind, ObjectKind::Mushroom | ObjectKind::Herb) {
                draw_editor_object_visual(
                    textures,
                    scene,
                    object,
                    object_state.opacity,
                    layer_mode,
                    selection.items.contains(&SelectionItem::Object(object.id)),
                );
            }
        }

        let mut commands = Vec::with_capacity(scene.map.stamps.len() + scene.map.objects.len());
        for stamp in &scene.map.stamps {
            commands.push((stamp.sort_y(), 0_u8, ObjectDrawCommand::Stamp(stamp)));
        }
        for object in &scene.map.objects {
            if matches!(object.kind, ObjectKind::Mushroom | ObjectKind::Herb) {
                continue;
            }
            commands.push((object.sort_y(), 1_u8, ObjectDrawCommand::Object(object)));
        }
        commands.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.cmp(&right.1)));

        for (_, _, command) in commands {
            match command {
                ObjectDrawCommand::Stamp(stamp) => draw_editor_stamp_visual(
                    textures,
                    stamp,
                    object_state.opacity,
                    layer_mode,
                    selection.items.contains(&SelectionItem::Stamp(stamp.id)),
                ),
                ObjectDrawCommand::Object(object) => draw_editor_object_visual(
                    textures,
                    scene,
                    object,
                    object_state.opacity,
                    layer_mode,
                    selection.items.contains(&SelectionItem::Object(object.id)),
                ),
            }
        }

        // Bounds and canonical anchor diagnostics are a dedicated overlay pass
        // so a later depth-sorted sprite cannot hide them. Preserve the prior
        // editor convention of outlining every placeable, but move the anchor
        // dot from the raw anchor-cell center to the shared physical foot/root.
        for stamp in &scene.map.stamps {
            let is_selected = selection.items.contains(&SelectionItem::Stamp(stamp.id));
            let (x, y, w, h) = stamp.visual_rect();
            let mut color = if is_selected {
                TEXT
            } else if layer_mode == SceneLayerMode::Objects {
                Color::new(0.45, 0.86, 1.0, 1.0)
            } else {
                Color::new(0.45, 0.76, 0.92, 0.55)
            };
            color.a *= object_state.opacity;
            draw_rectangle_lines(
                x as f32,
                y as f32,
                w as f32,
                h as f32,
                if is_selected { 0.18 } else { 0.08 },
                color,
            );
            let foot = haven_render::stamp_foot_tiles(stamp);
            draw_circle(foot.x, foot.y, 0.12, color);
        }
        for object in &scene.map.objects {
            let is_selected = selection.items.contains(&SelectionItem::Object(object.id));
            let (x, y, w, h) = object.visual_rect();
            let mut color = if is_selected {
                TEXT
            } else if layer_mode == SceneLayerMode::Objects {
                Color::new(1.0, 0.86, 0.36, 1.0)
            } else {
                Color::new(0.96, 0.78, 0.42, 0.55)
            };
            color.a *= object_state.opacity;
            draw_rectangle_lines(
                x as f32,
                y as f32,
                w as f32,
                h as f32,
                if is_selected { 0.18 } else { 0.08 },
                color,
            );
            let foot = haven_render::object_foot_tiles(*object);
            draw_circle(foot.x, foot.y, 0.12, color);
        }
    }

    if selection.scene_id.as_ref() == Some(&scene.id) {
        draw_selected_cells(selection, layer_mode);
        if let Some(bounds) = selection.bounds {
            draw_rectangle_lines(
                bounds.min.x as f32,
                bounds.min.y as f32,
                bounds.width() as f32,
                bounds.height() as f32,
                0.12,
                TEXT,
            );
        }
    }

    if let Some((cursor_x, cursor_y)) = cursor {
        draw_rectangle_lines(cursor_x as f32, cursor_y as f32, 1.0, 1.0, 0.10, TEXT);
    }

    draw_circle_lines(
        scene.spawn_x as f32 + 0.5,
        scene.spawn_y as f32 + 0.5,
        0.55,
        0.10,
        TEXT,
    );
    if let Some((min_x, min_y, max_x, max_y)) = scene.renderable_bounds() {
        draw_rectangle_lines(
            min_x as f32,
            min_y as f32,
            (max_x - min_x + 1) as f32,
            (max_y - min_y + 1) as f32,
            0.10,
            PANEL_EDGE,
        );
    }
}

fn draw_terrain_tuple_issue_overlay(
    scene: &SceneMap,
    visible_cells: Option<GridRect>,
    zoom: f32,
) {
    let Some(visible_cells) = visible_cells else {
        return;
    };
    let Ok(resolver) = haven_world::embedded_terrain_tuple_resolver() else {
        return;
    };
    let line = 0.075 / zoom.max(0.20);
    for y in visible_cells.min.y..=visible_cells.max.y {
        for x in visible_cells.min.x..=visible_cells.max.x {
            if !scene.is_renderable_cell(x, y) {
                continue;
            }
            let resolution = haven_world::resolve_semantic_tuple_at(resolver, &scene.map, x, y);
            let color = match resolution.status {
                haven_world::TerrainTupleResolutionStatus::Exact => continue,
                haven_world::TerrainTupleResolutionStatus::ExactDuplicateCanonical => WARN,
                haven_world::TerrainTupleResolutionStatus::Unresolved
                    if matches!(
                        haven_world::terrain_tuple_compatibility(resolution.tuple),
                        haven_world::TerrainTupleCompatibility::Compatible
                    ) => continue,
                haven_world::TerrainTupleResolutionStatus::Unresolved => {
                    Color::new(1.0, 0.26, 0.42, 1.0)
                }
            };
            draw_rectangle(
                x as f32,
                y as f32,
                1.0,
                1.0,
                Color::new(color.r, color.g, color.b, 0.13),
            );
            draw_rectangle_lines(x as f32, y as f32, 1.0, 1.0, line, color);
        }
    }
}

fn draw_editor_stamp_visual(
    textures: &EditorTextureSet,
    stamp: &PlacedStamp,
    opacity: f32,
    layer_mode: SceneLayerMode,
    is_selected: bool,
) {
    let (x, y, w, h) = stamp.visual_rect();
    let mut color = if is_selected {
        TEXT
    } else if layer_mode == SceneLayerMode::Objects {
        Color::new(0.45, 0.86, 1.0, 1.0)
    } else {
        Color::new(0.45, 0.76, 0.92, 0.55)
    };
    color.a *= opacity;
    if !textures.draw_stamp(stamp, opacity) {
        draw_rectangle(
            x as f32,
            y as f32,
            w as f32,
            h as f32,
            Color::new(color.r, color.g, color.b, color.a * 0.22),
        );
    } else if is_selected {
        draw_rectangle(
            x as f32,
            y as f32,
            w as f32,
            h as f32,
            Color::new(color.r, color.g, color.b, color.a * 0.10),
        );
    }
}

fn draw_editor_object_visual(
    textures: &EditorTextureSet,
    scene: &SceneMap,
    object: &PlacedObject,
    opacity: f32,
    layer_mode: SceneLayerMode,
    is_selected: bool,
) {
    let (x, y, w, h) = object.visual_rect();
    let mut color = if is_selected {
        TEXT
    } else if layer_mode == SceneLayerMode::Objects {
        Color::new(1.0, 0.86, 0.36, 1.0)
    } else {
        Color::new(0.96, 0.78, 0.42, 0.55)
    };
    color.a *= opacity;
    if !textures.draw_object(scene, object, opacity) {
        draw_rectangle(
            x as f32,
            y as f32,
            w as f32,
            h as f32,
            Color::new(color.r, color.g, color.b, color.a * 0.22),
        );
    } else if is_selected {
        draw_rectangle(
            x as f32,
            y as f32,
            w as f32,
            h as f32,
            Color::new(color.r, color.g, color.b, color.a * 0.10),
        );
    }
}

fn draw_selected_cells(selection: &EditorSelection, layer_mode: SceneLayerMode) {
    for item in &selection.items {
        let cell = match (layer_mode, item) {
            (SceneLayerMode::Terrain, SelectionItem::Tile(cell)) => Some(*cell),
            (SceneLayerMode::Zones, SelectionItem::ZoneCell { cell, .. }) => Some(*cell),
            _ => None,
        };
        if let Some(cell) = cell {
            draw_rectangle(
                cell.x as f32,
                cell.y as f32,
                1.0,
                1.0,
                Color::new(TEXT.r, TEXT.g, TEXT.b, 0.12),
            );
            draw_rectangle_lines(cell.x as f32, cell.y as f32, 1.0, 1.0, 0.08, TEXT);
        }
    }
}

pub(crate) fn draw_scene_grid_overlay(scene: &SceneMap, zoom: f32) {
    let Some((min_x, min_y, max_x, max_y)) = scene.renderable_bounds() else {
        return;
    };
    let minor = Color::new(0.10, 0.12, 0.13, 0.46);
    let major = Color::new(0.82, 0.68, 0.43, 0.58);
    let thin = 0.035 / zoom.max(0.20);
    let thick = 0.075 / zoom.max(0.20);
    for x in min_x..=max_x + 1 {
        let is_major = x.rem_euclid(8) == 0;
        draw_line(
            x as f32,
            min_y as f32,
            x as f32,
            (max_y + 1) as f32,
            if is_major { thick } else { thin },
            if is_major { major } else { minor },
        );
    }
    for y in min_y..=max_y + 1 {
        let is_major = y.rem_euclid(8) == 0;
        draw_line(
            min_x as f32,
            y as f32,
            (max_x + 1) as f32,
            y as f32,
            if is_major { thick } else { thin },
            if is_major { major } else { minor },
        );
    }
}

pub(crate) fn zone_preview_color(zone: ZoneKind) -> Option<Color> {
    match zone {
        ZoneKind::None => None,
        ZoneKind::Tavern => Some(Color::new(0.80, 0.56, 0.30, 0.45)),
        ZoneKind::Kitchen => Some(Color::new(0.90, 0.34, 0.22, 0.45)),
        ZoneKind::GuestRoom => Some(Color::new(0.56, 0.42, 0.84, 0.45)),
        ZoneKind::Cellar => Some(Color::new(0.37, 0.32, 0.45, 0.50)),
        ZoneKind::Greenhouse => Some(Color::new(0.25, 0.72, 0.40, 0.45)),
        ZoneKind::Field => Some(Color::new(0.72, 0.62, 0.22, 0.45)),
        ZoneKind::Cave => Some(Color::new(0.34, 0.36, 0.40, 0.50)),
        ZoneKind::StaffOnly => Some(Color::new(0.78, 0.22, 0.28, 0.45)),
        ZoneKind::PublicPath => Some(Color::new(0.86, 0.76, 0.46, 0.40)),
        ZoneKind::TavernExterior => Some(Color::new(0.42, 0.64, 0.34, 0.42)),
        ZoneKind::Bar => Some(Color::new(0.86, 0.48, 0.20, 0.45)),
        ZoneKind::CivicLot => Some(Color::new(0.72, 0.64, 1.00, 0.24)),
        ZoneKind::MarketLot => Some(Color::new(1.00, 0.58, 0.18, 0.24)),
        ZoneKind::ResidentialLot => Some(Color::new(0.42, 0.72, 1.00, 0.24)),
        ZoneKind::ArtisanLot => Some(Color::new(0.86, 0.48, 0.22, 0.24)),
        ZoneKind::HarborLot => Some(Color::new(0.22, 0.68, 0.88, 0.26)),
        ZoneKind::AgriculturalLot => Some(Color::new(0.52, 0.82, 0.30, 0.24)),
    }
}

pub(crate) fn scene_tile_color(tile: TileKind) -> Color {
    match tile {
        TileKind::Grass => Color::new(0.30, 0.55, 0.28, 1.0),
        TileKind::TallGrass => Color::new(0.22, 0.48, 0.23, 1.0),
        TileKind::Sand => Color::new(0.77, 0.68, 0.42, 1.0),
        TileKind::WetSand => Color::new(0.58, 0.54, 0.39, 1.0),
        TileKind::PebbleShore => Color::new(0.55, 0.55, 0.50, 1.0),
        TileKind::Road | TileKind::StonePath | TileKind::MountainPath => {
            Color::new(0.46, 0.39, 0.29, 1.0)
        }
        TileKind::WoodFloor | TileKind::PlankFloor | TileKind::Bridge => {
            Color::new(0.50, 0.30, 0.16, 1.0)
        }
        TileKind::StoneFloor | TileKind::BrickFloor => Color::new(0.48, 0.50, 0.51, 1.0),
        TileKind::Wall | TileKind::Cliff | TileKind::MountainRock | TileKind::CaveWall => {
            Color::new(0.22, 0.22, 0.24, 1.0)
        }
        TileKind::Dirt | TileKind::TilledSoil | TileKind::WateredSoil | TileKind::MudBank => {
            Color::new(0.39, 0.24, 0.14, 1.0)
        }
        TileKind::CaveFloor => Color::new(0.30, 0.28, 0.25, 1.0),
        TileKind::Crop | TileKind::GreenhouseZone => Color::new(0.22, 0.62, 0.30, 1.0),
        TileKind::Water
        | TileKind::ShallowWater
        | TileKind::DeepWater
        | TileKind::OceanDeep
        | TileKind::OceanShallow
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend
        | TileKind::ShoreFoam => Color::new(0.15, 0.33, 0.55, 1.0),
    }
}
