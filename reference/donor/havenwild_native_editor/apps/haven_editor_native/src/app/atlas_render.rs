use super::*;
use haven_assets::{
    asset_palette::{AssetPaletteEntry, AssetPaletteKind},
    asset_registry::{
        object_asset_entry_for_cell, tile_asset_rect, GENERATED_BASE_TERRAIN_ATLAS_PATH,
        OBJECT_ATLAS_PATH,
    },
    autotile::TERRAIN_AUTOTILE_ATLAS_PATH,
    lpc_cliff_ramp_provider::{LpcCliffContourStampRole, LPC_CLIFF_RAMP_GRASS_SOURCE_PATH},
    lpc_mapped_terrain::{
        lpc_mapped_terrain_atlas_path, lpc_mapped_terrain_owner_fill_entry_for_map,
        lpc_mapped_terrain_transition_entry_for_map,
    },
    placeable_asset_registry::PublishedWorldAssetRegistry,
    runtime_asset_cache::RuntimeAssetSession,
    stamp_registry::StampRegistry,
    user_asset_registry::{load_user_asset_registry_default, UserAssetRegistry},
};
use haven_world::terrain_tuple_render_origin_tiles;
use haven_render::{
    object_foot_tiles,
    structural_cliff_visual::{
        authored_body_rows, resolve_cliff_visual_recipe_v1, CliffVisualShape,
        EAST_EDGE_CELL, ELIZAWY_SUMMER_CLIFF_SOURCE_PATH, NORTH_EAST_CORNER_CELL,
        NORTH_LIP_CELL, NORTH_WEST_CORNER_CELL, SOUTH_EAST_DIAGONAL_FACE,
        SOUTH_STRAIGHT_FACE, SOUTH_TERMINAL_BODY_ROW, SOUTH_TERMINAL_CREST_ROW,
        SOUTH_TERMINAL_FOOT_ROW, SOUTH_WEST_DIAGONAL_FACE, WEST_EDGE_CELL,
    },
};

pub(crate) struct EditorTextureSet {
    terrain: Option<Texture2D>,
    objects: Option<Texture2D>,
    lpc_mapped_terrain: Option<Texture2D>,
    lpc_cliff_source: Option<Texture2D>,
    lpc_cliff_ramp_source: Option<Texture2D>,
    transitions: Option<Texture2D>,
    user_assets: Option<Texture2D>,
    user_registry: UserAssetRegistry,
    intake_source: Option<Texture2D>,
    intake_source_path: String,
    stamp_textures: HashMap<String, Texture2D>,
    stamp_registry: StampRegistry,
    placeable_textures: HashMap<String, Texture2D>,
    placeable_registry: PublishedWorldAssetRegistry,
}

impl EditorTextureSet {
    pub(crate) fn empty() -> Self {
        Self {
            terrain: None,
            objects: None,
            lpc_mapped_terrain: None,
            lpc_cliff_source: None,
            lpc_cliff_ramp_source: None,
            transitions: None,
            user_assets: None,
            user_registry: UserAssetRegistry::empty(),
            intake_source: None,
            intake_source_path: String::new(),
            stamp_textures: HashMap::new(),
            stamp_registry: StampRegistry::default(),
            placeable_textures: HashMap::new(),
            placeable_registry: PublishedWorldAssetRegistry::default(),
        }
    }

    pub(crate) async fn load() -> Self {
        let user_registry =
            load_user_asset_registry_default().unwrap_or_else(|_| UserAssetRegistry::empty());
        let user_assets = if user_registry.atlas_path().is_empty() {
            None
        } else {
            load_nearest(user_registry.atlas_path()).await
        };
        let root = std::path::Path::new(".");
        let asset_session = RuntimeAssetSession::discover_tolerant(root);
        let stamp_registry = StampRegistry::load_discovered(&asset_session)
            .or_else(|_| StampRegistry::load_default())
            .unwrap_or_default();
        let mut stamp_textures = HashMap::new();
        for sheet in stamp_registry.sheets() {
            if let Some(texture) = load_nearest(&sheet).await {
                stamp_textures.insert(sheet, texture);
            }
        }
        let placeable_registry = PublishedWorldAssetRegistry::load_discovered(&asset_session)
            .unwrap_or_default();
        let mut placeable_textures = HashMap::new();
        for definition in placeable_registry.entries() {
            if let Some(path) = &definition.source_path {
                if let Some(texture) = load_nearest(path.to_string_lossy().as_ref()).await {
                    placeable_textures.insert(definition.stable_id.clone(), texture);
                }
            }
        }
        Self {
            terrain: load_nearest(GENERATED_BASE_TERRAIN_ATLAS_PATH).await,
            objects: load_nearest(OBJECT_ATLAS_PATH).await,
            lpc_mapped_terrain: match lpc_mapped_terrain_atlas_path() {
                Ok(path) => load_nearest(path).await,
                Err(error) => {
                    println!("Could not resolve mapped LPC terrain atlas: {error}");
                    None
                }
            },
            lpc_cliff_source: load_nearest(ELIZAWY_SUMMER_CLIFF_SOURCE_PATH).await,
            lpc_cliff_ramp_source: load_nearest(LPC_CLIFF_RAMP_GRASS_SOURCE_PATH).await,
            transitions: load_nearest(TERRAIN_AUTOTILE_ATLAS_PATH).await,
            user_assets,
            user_registry,
            intake_source: None,
            intake_source_path: String::new(),
            stamp_textures,
            stamp_registry,
            placeable_textures,
            placeable_registry,
        }
    }

    pub(crate) async fn reload_user_assets(&mut self) -> Result<String, String> {
        let registry = load_user_asset_registry_default()?;
        let texture = if registry.atlas_path().is_empty() {
            None
        } else {
            load_nearest(registry.atlas_path()).await
        };
        if !registry.atlas_path().is_empty() && texture.is_none() {
            return Err(format!("failed to reload {}", registry.atlas_path()));
        }
        let summary = registry.coverage_summary();
        self.user_registry = registry;
        self.user_assets = texture;
        Ok(summary)
    }

    pub(crate) fn user_registry(&self) -> &UserAssetRegistry {
        &self.user_registry
    }

    pub(crate) async fn reload_intake_source(&mut self, path: &str) -> Result<(), String> {
        let texture = load_nearest(path)
            .await
            .ok_or_else(|| format!("failed to load intake source {path}"))?;
        self.intake_source = Some(texture);
        self.intake_source_path = path.to_string();
        Ok(())
    }

    pub(crate) fn draw_intake_source_preview(
        &self,
        recipe: &haven_assets::asset_intake::AssetIntakeRecipe,
        rect: Rect,
    ) -> bool {
        if self.intake_source_path != recipe.source_path {
            return false;
        }
        let Some(texture) = self.intake_source.as_ref() else {
            return false;
        };
        let source = Rect::new(
            recipe.slice.x as f32,
            recipe.slice.y as f32,
            recipe.slice.width as f32,
            recipe.slice.height as f32,
        );
        draw_texture_ex(
            texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                source: Some(source),
                ..Default::default()
            },
        );
        let pivot_x = rect.x + (recipe.pivot.x as f32 / recipe.slice.width.max(1) as f32) * rect.w;
        let pivot_y = rect.y + (recipe.pivot.y as f32 / recipe.slice.height.max(1) as f32) * rect.h;
        draw_line(pivot_x - 5.0, pivot_y, pivot_x + 5.0, pivot_y, 1.0, WARN);
        draw_line(pivot_x, pivot_y - 5.0, pivot_x, pivot_y + 5.0, 1.0, WARN);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, PANEL_EDGE);
        true
    }

    pub(crate) fn readiness_summary(&self) -> String {
        let loaded = [
            self.terrain.is_some(),
            self.objects.is_some(),
            self.lpc_mapped_terrain.is_some(),
            self.lpc_cliff_source.is_some(),
            self.lpc_cliff_ramp_source.is_some(),
            self.transitions.is_some(),
            self.user_assets.is_some(),
            !self.stamp_textures.is_empty(),
        ]
        .into_iter()
        .filter(|ready| *ready)
        .count();
        format!("{loaded}/8 editor atlas texture groups loaded")
    }

    pub(crate) fn draw_tile(
        &self,
        scene: &SceneMap,
        cache: Option<&LiveAutotileCache>,
        x: i32,
        y: i32,
        opacity: f32,
    ) -> bool {
        let tile = scene.map.get(x, y);
        let mut drew = false;

        // Semantic cells remain the gameplay and brush authority. Draw their
        // pure owner fill at the exact cell origin; mixed corner tuples are
        // submitted later in a dedicated intersection-aligned overlay pass.
        if let (Some(texture), Some(entry)) = (
            self.lpc_mapped_terrain.as_ref(),
            lpc_mapped_terrain_owner_fill_entry_for_map(&scene.map, x, y),
        ) {
            draw_atlas_rect(
                texture,
                entry.rect,
                Rect::new(x as f32, y as f32, 1.01, 1.01),
                Color::new(1.0, 1.0, 1.0, opacity),
            );
            drew = true;
        }

        // W43C fail-closed production rule: live_autotile_16_32 is a legacy
        // connectivity/debug atlas, not Havenwild production artwork. Do not
        // render it as a base-cell fallback when an authored LPC material is
        // unavailable. The cache remains available to the explicit Auto/dirty
        // preview overlay, while ordinary scene rendering continues through
        // user assets or the generated base diagnostic lane below.
        let _ = cache;
        if !drew {
            if let (Some(texture), Some(binding)) = (
                self.user_assets.as_ref(),
                self.user_registry.binding_for_tile(tile),
            ) {
                draw_atlas_rect(
                    texture,
                    binding.rect,
                    Rect::new(x as f32, y as f32, 1.01, 1.01),
                    Color::new(1.0, 1.0, 1.0, opacity),
                );
                drew = true;
            }
        }
        if !drew {
            if let Some(texture) = self.terrain.as_ref() {
                draw_atlas_rect(
                    texture,
                    tile_asset_rect(tile, x, y),
                    Rect::new(x as f32, y as f32, 1.01, 1.01),
                    Color::new(1.0, 1.0, 1.0, opacity),
                );
                drew = true;
            }
        }

        // The V7 certification lane is fail-closed. Exact tuples win; reviewed
        // V7-only connectors may bridge Gravel, Rock Ground and Mud while the
        // source-pure owner fill remains unchanged. No generic or ElizaWy
        // transition pixels are borrowed.
        drew
    }

    pub(crate) fn draw_terrain_tuple_overlay(
        &self,
        scene: &SceneMap,
        x: i32,
        y: i32,
        opacity: f32,
    ) -> bool {
        let (Some(texture), Some(entry)) = (
            self.lpc_mapped_terrain.as_ref(),
            lpc_mapped_terrain_transition_entry_for_map(&scene.map, x, y),
        ) else {
            return false;
        };
        let (draw_x, draw_y) = terrain_tuple_render_origin_tiles(x, y);
        draw_atlas_rect(
            texture,
            entry.rect,
            Rect::new(draw_x, draw_y, 1.01, 1.01),
            Color::new(1.0, 1.0, 1.0, opacity),
        );
        true
    }

    /// Draw structural cliffs in the native scene editor from the same W40
    /// semantic visual recipe consumed by the game runtime.
    ///
    /// GPU submission remains editor-specific, but source cells, corner family,
    /// south-face tier count and terminal/diagonal selection are shared.
    pub(crate) fn draw_structural_cliffs(
        &self,
        scene: &SceneMap,
        structural: &haven_world::LegacyCliffBridgeResultV2,
        visible_cells: Option<GridRect>,
        opacity: f32,
    ) -> bool {
        let Some(texture) = self.lpc_cliff_source.as_ref() else {
            return false;
        };
        let Some(visible) = visible_cells else {
            return false;
        };
        let min_x = (visible.min.x - 2).max(0);
        let max_x = (visible.max.x + 2).min(MAP_W as i32 - 1);
        // South faces can project several receiver rows downward, so include
        // hosts above the visible window as well as one row below it.
        let min_y = (visible.min.y - 4).max(0);
        let max_y = (visible.max.y + 1).min(MAP_H as i32 - 1);

        let draw_cell = |source: Rect, x: i32, y: i32| {
            draw_texture_ex(
                texture,
                x as f32,
                y as f32,
                Color::new(1.0, 1.0, 1.0, opacity),
                DrawTextureParams {
                    dest_size: Some(vec2(1.0, 1.0)),
                    source: Some(source),
                    ..Default::default()
                },
            );
        };

        let level_at = |x: i32, y: i32| -> Option<u8> {
            if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
                return None;
            }
            Some(haven_world::structural_level_for_surface_recipe_v1(
                &scene.map, x, y,
            ))
        };

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if !scene.is_renderable_cell(x, y) {
                    continue;
                }
                let Some(center) = structural.structural_at(x, y) else {
                    continue;
                };
                let Some(recipe) = resolve_cliff_visual_recipe_v1(center, |dx, dy| {
                    structural.structural_at(x + dx, y + dy)
                }) else {
                    continue;
                };
                let host_level = haven_world::structural_level_for_surface_recipe_v1(
                    &scene.map, x, y,
                );
                let projection_visible = |target_x: i32, target_y: i32| {
                    level_at(target_x, target_y).is_none_or(|level| level < host_level)
                };

                if !recipe.south_exposed {
                    match (
                        recipe.north_exposed,
                        recipe.east_exposed,
                        recipe.west_exposed,
                    ) {
                        (false, false, false) => {}
                        (true, false, false) => draw_cell(NORTH_LIP_CELL, x, y),
                        (false, true, false) => draw_cell(EAST_EDGE_CELL, x, y),
                        (false, false, true) => draw_cell(WEST_EDGE_CELL, x, y),
                        (true, true, false) => draw_cell(NORTH_EAST_CORNER_CELL, x, y),
                        (true, false, true) => draw_cell(NORTH_WEST_CORNER_CELL, x, y),
                        (false, true, true) => {
                            if let Some(ridge_texture) = self.lpc_cliff_ramp_source.as_ref() {
                                let role = LpcCliffContourStampRole::VerticalRidgeMiddle;
                                let stamp = role.source_stamp();
                                let (anchor_x, anchor_y) = role.host_anchor_offset();
                                draw_texture_ex(
                                    ridge_texture,
                                    (x + i32::from(anchor_x)) as f32,
                                    (y + i32::from(anchor_y)) as f32,
                                    Color::new(1.0, 1.0, 1.0, opacity),
                                    DrawTextureParams {
                                        dest_size: Some(vec2(
                                            stamp.width_cells as f32,
                                            stamp.height_cells as f32,
                                        )),
                                        source: Some(Rect::new(
                                            stamp.column as f32 * haven_core::TILE_SIZE,
                                            stamp.row as f32 * haven_core::TILE_SIZE,
                                            stamp.width_cells as f32 * haven_core::TILE_SIZE,
                                            stamp.height_cells as f32 * haven_core::TILE_SIZE,
                                        )),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                        // Three-sided thin caps are invalid fresh-PCG topology;
                        // match runtime and leave them for diagnostics instead
                        // of inventing an authored tile.
                        (true, true, true) => {}
                    }
                    continue;
                }

                if recipe.north_exposed {
                    draw_cell(NORTH_LIP_CELL, x, y);
                }

                let face_segments = recipe.south_face_segments.max(1);
                let body_rows = authored_body_rows(face_segments);
                match recipe.visual_shape {
                    CliffVisualShape::SouthWestDiagonal if recipe.west_exposed => {
                        draw_cell(SOUTH_WEST_DIAGONAL_FACE.crest, x, y);
                        for row in 0..body_rows {
                            let target_y = y + 1 + row as i32;
                            if projection_visible(x - 1, target_y) {
                                draw_cell(SOUTH_WEST_DIAGONAL_FACE.body, x, target_y);
                            }
                        }
                        let foot_y = y + 1 + body_rows as i32;
                        if projection_visible(x - 1, foot_y) {
                            draw_cell(SOUTH_WEST_DIAGONAL_FACE.foot, x, foot_y);
                        }
                    }
                    CliffVisualShape::SouthEastDiagonal if recipe.east_exposed => {
                        draw_cell(SOUTH_EAST_DIAGONAL_FACE.crest, x, y);
                        for row in 0..body_rows {
                            let target_y = y + 1 + row as i32;
                            if projection_visible(x + 1, target_y) {
                                draw_cell(SOUTH_EAST_DIAGONAL_FACE.body, x, target_y);
                            }
                        }
                        let foot_y = y + 1 + body_rows as i32;
                        if projection_visible(x + 1, foot_y) {
                            draw_cell(SOUTH_EAST_DIAGONAL_FACE.foot, x, foot_y);
                        }
                    }
                    CliffVisualShape::SouthAuthoredTerminal
                        if recipe.east_exposed && recipe.west_exposed =>
                    {
                        let draw_row = |row: &[Rect; 3], target_y: i32| {
                            for (column, source) in row.iter().copied().enumerate() {
                                let target_x = x + column as i32 - 1;
                                if target_y == y || projection_visible(target_x, target_y) {
                                    draw_cell(source, target_x, target_y);
                                }
                            }
                        };
                        draw_row(&SOUTH_TERMINAL_CREST_ROW, y);
                        for row in 0..body_rows {
                            draw_row(&SOUTH_TERMINAL_BODY_ROW, y + 1 + row as i32);
                        }
                        draw_row(&SOUTH_TERMINAL_FOOT_ROW, y + 1 + body_rows as i32);
                    }
                    _ => {
                        if let Some(crest) = SOUTH_STRAIGHT_FACE.leading {
                            draw_cell(crest, x, y);
                        }
                        for row in 0..body_rows {
                            let target_y = y + 1 + row as i32;
                            if projection_visible(x, target_y) {
                                draw_cell(SOUTH_STRAIGHT_FACE.body, x, target_y);
                            }
                        }
                        let foot_y = y + 1 + body_rows as i32;
                        if projection_visible(x, foot_y) {
                            draw_cell(SOUTH_STRAIGHT_FACE.foot, x, foot_y);
                        }
                    }
                }
            }
        }
        true
    }

    pub(crate) fn draw_published_asset_at_tile(&self, asset_id: &str, state: Option<&str>, tile: [i32; 2], opacity: f32) -> bool {
        let Some(definition) = self.placeable_registry.resolve_alias(asset_id) else { return false; };
        let (Some(texture), Some(visual)) = (self.placeable_textures.get(definition.pack_qualified_id()), definition.visual.as_ref()) else { return false; };
        let Some(frame) = visual.frame_for_state(state) else { return false; };
        let tile_size = haven_core::TILE_SIZE;
        let destination = Rect::new(
            tile[0] as f32 + 0.5 - visual.foot_anchor[0] / tile_size,
            tile[1] as f32 + 1.0 - visual.foot_anchor[1] / tile_size,
            frame.source_rect[2] / tile_size, frame.source_rect[3] / tile_size,
        );
        let [x, y, w, h] = frame.source_rect;
        draw_atlas_rect(texture, haven_assets::asset_registry::AtlasRect { x, y, w, h }, destination, Color::new(1.0, 1.0, 1.0, opacity));
        true
    }

    pub(crate) fn draw_object(
        &self,
        scene: &SceneMap,
        object: &PlacedObject,
        opacity: f32,
    ) -> bool {
        let published = scene
            .map
            .object_asset_ref(object.id)
            .and_then(|asset_ref| self.placeable_registry.resolve_persistent_ref(asset_ref))
            .or_else(|| self.placeable_registry.for_legacy_object(object.kind));
        if let Some(definition) = published {
            if let (Some(texture), Some(visual)) = (
                self.placeable_textures.get(definition.pack_qualified_id()),
                definition.visual.as_ref(),
            ) {
                let state = scene.map.object_state(object.id);
                if let Some(frame) = visual.frame_for_state(state) {
                    let foot = object_foot_tiles(*object);
                    let tile_size = haven_core::TILE_SIZE;
                    let destination = Rect::new(
                        foot.x - visual.foot_anchor[0] / tile_size,
                        foot.y - visual.foot_anchor[1] / tile_size,
                        frame.source_rect[2] / tile_size,
                        frame.source_rect[3] / tile_size,
                    );
                    draw_atlas_rect(
                        texture,
                        haven_assets::asset_registry::AtlasRect {
                            x: frame.source_rect[0],
                            y: frame.source_rect[1],
                            w: frame.source_rect[2],
                            h: frame.source_rect[3],
                        },
                        destination,
                        Color::new(1.0, 1.0, 1.0, opacity),
                    );
                    return true;
                }
            }
        }
        if let (Some(texture), Some(binding)) = (
            self.user_assets.as_ref(),
            self.user_registry.binding_for_object(object.kind),
        ) {
            let tile_size = haven_core::TILE_SIZE;
            let foot = object_foot_tiles(*object);
            let destination = Rect::new(
                foot.x - binding.pivot.x as f32 / tile_size,
                foot.y - binding.pivot.y as f32 / tile_size,
                binding.rect.w / tile_size,
                binding.rect.h / tile_size,
            );
            draw_atlas_rect(
                texture,
                binding.rect,
                destination,
                Color::new(1.0, 1.0, 1.0, opacity),
            );
            return true;
        }
        let Some(texture) = self.objects.as_ref() else {
            return false;
        };
        let Some(entry) = object_asset_entry_for_cell(object.kind, object.x, object.y) else {
            return false;
        };
        let tile_size = haven_core::TILE_SIZE;
        let foot = object_foot_tiles(*object);
        let destination = Rect::new(
            foot.x - entry.foot_anchor.0 / tile_size,
            foot.y - entry.foot_anchor.1 / tile_size,
            entry.rect.w / tile_size,
            entry.rect.h / tile_size,
        );
        draw_atlas_rect(
            texture,
            entry.rect,
            destination,
            Color::new(1.0, 1.0, 1.0, opacity),
        );
        true
    }

    pub(crate) fn draw_stamp(&self, stamp: &haven_core::PlacedStamp, opacity: f32) -> bool {
        let Some(definition) = self.stamp_registry.entry(&stamp.stamp_key) else {
            return false;
        };
        let Some(texture) = self.stamp_textures.get(&definition.sheet) else {
            return false;
        };
        let (x, y, w, h) = stamp.visual_rect();
        if let Some(expandable) = definition.expandable {
            draw_expandable_stamp_rect(
                texture,
                expandable,
                Rect::new(x as f32, y as f32, w as f32, h as f32),
                opacity,
            );
        } else {
            draw_atlas_rect(
                texture,
                definition.rect,
                Rect::new(x as f32, y as f32, w as f32, h as f32),
                Color::new(1.0, 1.0, 1.0, opacity),
            );
        }
        true
    }

    pub(crate) fn draw_stamp_ghost(&self, stable_id: &str, x: i32, y: i32, opacity: f32) -> bool {
        let Some(definition) = self.stamp_registry.entry(stable_id) else {
            return false;
        };
        let Some(texture) = self.stamp_textures.get(&definition.sheet) else {
            return false;
        };
        let placed = definition.placed_at(x, y);
        let (vx, vy, vw, vh) = placed.visual_rect();
        if let Some(expandable) = definition.expandable {
            draw_expandable_stamp_rect(
                texture,
                expandable,
                Rect::new(vx as f32, vy as f32, vw as f32, vh as f32),
                opacity,
            );
        } else {
            draw_atlas_rect(
                texture,
                definition.rect,
                Rect::new(vx as f32, vy as f32, vw as f32, vh as f32),
                Color::new(1.0, 1.0, 1.0, opacity),
            );
        }
        true
    }

    pub(crate) fn draw_palette_thumbnail(&self, entry: &AssetPaletteEntry, rect: Rect) -> bool {
        let Some(source) = entry.rect else {
            return false;
        };
        let texture = match entry.sheet.as_deref() {
            Some(sheet)
                if lpc_mapped_terrain_atlas_path()
                    .ok()
                    .is_some_and(|mapped| sheet == mapped) =>
            {
                self.lpc_mapped_terrain.as_ref()
            }
            Some(sheet) if sheet == self.user_registry.atlas_path() => self.user_assets.as_ref(),
            Some(sheet) if matches!(entry.kind, AssetPaletteKind::Stamp) => {
                self.stamp_textures.get(sheet)
            }
            _ => match entry.kind {
                AssetPaletteKind::Tile(_) => self.terrain.as_ref(),
                AssetPaletteKind::Object(_) => self.objects.as_ref(),
                AssetPaletteKind::Stamp => None,
            },
        };
        let Some(texture) = texture else {
            return false;
        };
        draw_atlas_rect(texture, source, rect, WHITE);
        true
    }
}

fn draw_expandable_stamp_rect(
    texture: &Texture2D,
    definition: haven_assets::stamp_registry::ExpandableStampDefinition,
    destination: Rect,
    opacity: f32,
) {
    let width = destination.w.round().max(definition.minimum_w as f32) as i32;
    let height = destination.h.round().max(definition.minimum_h as f32) as i32;
    for cell_y in 0..height {
        for cell_x in 0..width {
            let cell_destination = Rect::new(
                destination.x + cell_x as f32,
                destination.y + cell_y as f32,
                1.0,
                1.0,
            );
            draw_atlas_rect(
                texture,
                definition.base_fill_tile,
                cell_destination,
                Color::new(1.0, 1.0, 1.0, opacity),
            );
            draw_atlas_rect(
                texture,
                definition.source_rect_for_rect_cell(cell_x, cell_y, width, height),
                cell_destination,
                Color::new(1.0, 1.0, 1.0, opacity),
            );
        }
    }
}

pub(crate) fn draw_atlas_rect(
    texture: &Texture2D,
    source: haven_assets::asset_registry::AtlasRect,
    destination: Rect,
    tint: Color,
) {
    draw_texture_ex(
        texture,
        destination.x,
        destination.y,
        tint,
        DrawTextureParams {
            dest_size: Some(vec2(destination.w, destination.h)),
            source: Some(Rect::new(source.x, source.y, source.w, source.h)),
            ..Default::default()
        },
    );
}

async fn load_nearest(path: &str) -> Option<Texture2D> {
    let resolved = if std::path::Path::new(path).exists() {
        path.to_string()
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
            .to_string_lossy()
            .into_owned()
    };
    match load_texture(&resolved).await {
        Ok(texture) => {
            texture.set_filter(FilterMode::Nearest);
            Some(texture)
        }
        Err(error) => {
            println!("Could not load editor atlas {resolved}: {error}");
            None
        }
    }
}
