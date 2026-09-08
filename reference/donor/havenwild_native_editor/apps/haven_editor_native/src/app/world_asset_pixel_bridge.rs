use super::pixel_studio::{PixelInspectorTab, WorldAssetEditContext};
use super::scene_asset_context::{action_at, SceneAssetContextAction, SceneAssetContextMenu};
use super::*;
use haven_assets::asset_intake::repo_root_dir;
use haven_pixel::{PixelDocument, PixelSelection};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct ResolvedWorldAsset {
    semantic_id: String,
    source_path: String,
    source_rect: PixelSelection,
    generated_output: bool,
}

impl EditorApp {
    pub(crate) fn open_scene_asset_context_menu(&mut self) -> bool {
        if self.viewport_mode != EditorViewportMode::SceneMap {
            return false;
        }
        let Some((x, y)) = self.scene_cell_at_mouse() else {
            return false;
        };
        self.scene_cursor_x = x;
        self.scene_cursor_y = y;
        let object_hit = self
            .model
            .world
            .scenes
            .get(self.selected_scene)
            .and_then(|scene| hit_test_scene_cell(
                scene,
                GridPos { x, y },
                SceneAuthoringLayer::Objects,
            ));
        let object_id = object_hit.as_ref().and_then(|hit| match hit.selection_item() {
            SelectionItem::Object(id) => Some(id),
            _ => None,
        });
        if let Some(hit) = object_hit {
            self.select_scene_hit(hit);
        }
        let (mx, my) = mouse_position();
        self.scene_asset_context_menu = Some(SceneAssetContextMenu {
            screen_position: vec2(mx, my),
            cell: [x, y],
            object_id,
        });
        self.status_message = if let Some(id) = object_id {
            format!("Opened exact-source asset actions for object {:?} at {x}, {y}", id)
        } else {
            format!("Opened asset actions for terrain tile {x}, {y}")
        };
        true
    }

    pub(crate) fn handle_scene_asset_context_click(&mut self, mx: f32, my: f32) -> bool {
        let Some(menu) = self.scene_asset_context_menu else {
            return false;
        };
        let selected = action_at(menu, vec2(mx, my));
        self.scene_asset_context_menu = None;
        let Some(action) = selected else {
            return false;
        };
        self.scene_cursor_x = menu.cell[0];
        self.scene_cursor_y = menu.cell[1];
        match action {
            SceneAssetContextAction::EditSource => {
                if let Some(object_id) = menu.object_id {
                    self.open_scene_object_source_in_pixel_studio(object_id);
                } else {
                    self.open_scene_tile_source_in_pixel_studio();
                }
            }
            SceneAssetContextAction::InspectBinding => {
                if let Some(object_id) = menu.object_id {
                    self.inspect_scene_object_asset_binding(object_id);
                } else {
                    self.inspect_scene_tile_asset_binding();
                }
            }
            SceneAssetContextAction::RebuildNeighborhood => {
                self.autotile_caches.remove(
                    &self
                        .active_scene_id()
                        .unwrap_or_else(|| ProjectSceneId::new("none")),
                );
                self.status_message = format!(
                    "Invalidated terrain neighborhood around {}, {}",
                    menu.cell[0], menu.cell[1]
                );
            }
            SceneAssetContextAction::RevealSource => {
                if let Some(object_id) = menu.object_id {
                    self.reveal_scene_object_source(object_id);
                } else {
                    self.reveal_scene_tile_source();
                }
            }
        }
        true
    }

    fn resolve_scene_object_asset(&self, object_id: haven_editor::ObjectId) -> Option<ResolvedWorldAsset> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        let object = scene.map.object(object_id)?;
        let definition = scene
            .map
            .object_asset_ref(object_id)
            .and_then(|asset_ref| self.placeable_registry.resolve_persistent_ref(asset_ref))
            .or_else(|| self.placeable_registry.for_legacy_object(object.kind))?;
        let source_path = definition.provenance.source_path.as_ref()?.clone();
        let source_rect = definition.provenance.source_rect?;
        let [x, y, width, height] = source_rect;
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return None;
        }
        Some(ResolvedWorldAsset {
            semantic_id: definition.semantic_id.clone(),
            source_path,
            source_rect: PixelSelection {
                x: x as u32,
                y: y as u32,
                width: width as u32,
                height: height as u32,
            },
            generated_output: false,
        })
    }

    fn inspect_scene_object_asset_binding(&mut self, object_id: haven_editor::ObjectId) {
        self.status_message = self.resolve_scene_object_asset(object_id).map_or_else(
            || format!("Object {:?} has no reviewed exact-source binding yet", object_id),
            |binding| format!(
                "Object {:?} -> {} [{} x={}, y={}, {}x{}]",
                object_id,
                binding.semantic_id,
                binding.source_path,
                binding.source_rect.x,
                binding.source_rect.y,
                binding.source_rect.width,
                binding.source_rect.height,
            ),
        );
    }

    fn reveal_scene_object_source(&mut self, object_id: haven_editor::ObjectId) {
        self.status_message = self.resolve_scene_object_asset(object_id).map_or_else(
            || format!("Object {:?} has no reviewed exact-source binding", object_id),
            |binding| format!("Source: {} [{} ,{} {}x{}]", binding.source_path, binding.source_rect.x, binding.source_rect.y, binding.source_rect.width, binding.source_rect.height),
        );
    }

    pub(crate) fn open_scene_object_source_in_pixel_studio(&mut self, object_id: haven_editor::ObjectId) {
        let Some(scene_id) = self.active_scene_id() else { return; };
        let Some(binding) = self.resolve_scene_object_asset(object_id) else {
            self.status_message = format!("Object {:?} has no reviewed exact source; W43/W45 disposition remains authoritative", object_id);
            return;
        };
        self.open_resolved_world_asset_source(scene_id, binding, Some(object_id));
    }

    fn selected_scene_tile(&self) -> Option<TileKind> {
        self.model
            .world
            .scenes
            .get(self.selected_scene)
            .map(|scene| scene.map.get(self.scene_cursor_x, self.scene_cursor_y))
    }

    fn resolve_selected_world_asset(&self) -> Option<ResolvedWorldAsset> {
        self.resolve_selected_structural_cliff_asset()
            .or_else(|| resolve_tile_asset(self.selected_scene_tile()?))
    }

    /// Mirrors the deliberately narrow runtime cliff lane certified in Z105/Z106.
    /// A structural cliff is not a 32x32 ground tile: the only runtime-visible
    /// recipe currently certified is the complete ElizaWy 1x3 straight south
    /// face. Keep editor inspection on that same authority instead of inventing
    /// corner/diagonal bindings that runtime cannot yet draw.
    fn resolve_selected_structural_cliff_asset(&self) -> Option<ResolvedWorldAsset> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        let x = self.scene_cursor_x;
        let y = self.scene_cursor_y;
        if x == 0 || x + 1 >= MAP_W as i32 || y + 1 >= MAP_H as i32 {
            return None;
        }
        let level = |cx: i32, cy: i32| scene.map.get_structural_level(cx, cy).unwrap_or(0);
        let south_exposed = |cx: i32| level(cx, y) > level(cx, y + 1);
        if !south_exposed(x) || !south_exposed(x - 1) || !south_exposed(x + 1) {
            return None;
        }
        Some(ResolvedWorldAsset {
            semantic_id: "terrain.cliff.elizawy.south_repeat_a_1x3".to_string(),
            source_path: "assets/source/licensed/lpc_revised/Terrain/cliff_summer.png".to_string(),
            source_rect: PixelSelection { x: 320, y: 288, width: 32, height: 96 },
            generated_output: false,
        })
    }

    fn inspect_scene_tile_asset_binding(&mut self) {
        let Some(tile) = self.selected_scene_tile() else {
            return;
        };
        let structural = self.selected_structural_cliff_diagnostic();
        self.status_message = match self.resolve_selected_world_asset() {
            Some(binding) => format!(
                "{:?} -> {} [{} x={}, y={}, {}x{}]{}",
                tile,
                binding.semantic_id,
                binding.source_path,
                binding.source_rect.x,
                binding.source_rect.y,
                binding.source_rect.width,
                binding.source_rect.height,
                structural.as_deref().map(|value| format!(" | {value}")).unwrap_or_default()
            ),
            None => format!("{:?} has no reviewed editable source binding yet", tile),
        };
    }

    fn selected_structural_cliff_diagnostic(&self) -> Option<String> {
        let scene = self.model.world.scenes.get(self.selected_scene)?;
        let x = self.scene_cursor_x;
        let y = self.scene_cursor_y;
        if y + 1 >= MAP_H as i32 { return None; }
        let current = scene.map.get_structural_level(x, y).unwrap_or(0);
        let south = scene.map.get_structural_level(x, y + 1).unwrap_or(0);
        if current <= south { return None; }
        let exposed = current - south;
        let certified = self.resolve_selected_structural_cliff_asset().is_some();
        Some(format!(
            "structural south drop {current}->{south} (height {exposed}); visual recipe {}",
            if certified { "Z105-certified ElizaWy 1x3 south face" } else { "not runtime-certified for this topology" }
        ))
    }

    fn reveal_scene_tile_source(&mut self) {
        self.status_message = self.resolve_selected_world_asset().map_or_else(
            || "The selected tile has no reviewed source binding".to_string(),
            |binding| format!("Source: {}", binding.source_path),
        );
    }

    pub(crate) fn open_scene_tile_source_in_pixel_studio(&mut self) {
        let Some(scene_id) = self.active_scene_id() else { return; };
        let Some(tile) = self.selected_scene_tile() else { return; };
        let Some(binding) = self.resolve_selected_world_asset() else {
            self.status_message = format!("{:?} is not yet mapped to an editable production source", tile);
            return;
        };
        self.open_resolved_world_asset_source(scene_id, binding, None);
    }

    fn open_resolved_world_asset_source(
        &mut self,
        scene_id: ProjectSceneId,
        binding: ResolvedWorldAsset,
        object_id: Option<haven_editor::ObjectId>,
    ) {
        let source_path = resolve_project_path(&binding.source_path);
        let mut document = match PixelDocument::load_source_region(
            &source_path,
            binding.source_rect,
            binding.semantic_id.clone(),
            haven_pixel::PixelLicense::default(),
        ) {
            Ok(document) => document,
            Err(error) => {
                self.status_message = format!("Unable to open exact source region: {error}");
                return;
            }
        };
        document.metadata.grid.cell_width = document.metadata.width.min(32).max(1);
        document.metadata.grid.cell_height = document.metadata.height.min(32).max(1);
        document.metadata.selection = PixelSelection {
            x: 0,
            y: 0,
            width: document.metadata.width,
            height: document.metadata.height,
        };
        let context = WorldAssetEditContext {
            origin_scene_id: scene_id,
            origin_cell: [self.scene_cursor_x, self.scene_cursor_y],
            origin_camera: self.scene_canvas,
            semantic_id: binding.semantic_id.clone(),
            generated_output: binding.generated_output,
        };
        self.pixel_studio.document = Some(document);
        self.pixel_studio.world_asset_context = Some(context);
        self.pixel_studio.animation_context = None;
        self.pixel_studio.inspector_tab = PixelInspectorTab::Asset;
        self.pixel_studio.show_atlas_grid = false;
        self.pixel_studio.refresh_texture();
        self.pixel_studio.frame_document(self.pixel_canvas_rect());
        self.viewport_mode = EditorViewportMode::PixelStudio;
        self.status_message = match object_id {
            Some(id) => format!("Editing exact source region for object {:?}: {}", id, binding.semantic_id),
            None => format!("Editing exact source region for {} at {}, {}", binding.semantic_id, self.scene_cursor_x, self.scene_cursor_y),
        };
    }

    pub(crate) fn save_world_asset_pixels_and_return(&mut self) {
        let Some(context) = self.pixel_studio.world_asset_context.clone() else {
            self.status_message = "No world-asset Pixel Studio bridge is active".to_string();
            return;
        };
        if context.generated_output {
            self.status_message =
                "Generated atlas output is read-only; edit its reviewed source or override"
                    .to_string();
            return;
        }
        let Some(document) = self.pixel_studio.document.as_mut() else {
            return;
        };
        if let Err(error) = document.save(repo_root_dir()) {
            self.status_message = format!("World asset save failed: {error}");
            return;
        }
        if let Some(index) = self.model.world.scenes.position(&context.origin_scene_id) {
            self.selected_scene = index;
        }
        self.scene_cursor_x = context.origin_cell[0];
        self.scene_cursor_y = context.origin_cell[1];
        self.scene_canvas = context.origin_camera;
        self.pixel_studio.world_asset_context = None;
        self.viewport_mode = EditorViewportMode::SceneMap;
        self.status_message = format!(
            "Saved derived working copy for {}. Runtime binding is unchanged until Publish Slice Draft -> approve -> Bake + Reload.",
            context.semantic_id
        );
    }
}

fn resolve_tile_asset(tile: TileKind) -> Option<ResolvedWorldAsset> {
    // A cliff cell is structural elevation. The visible face can consist of
    // several source cells, so it must be inspected through the cliff
    // composition resolver rather than pretending it is one ground tile.
    if tile == TileKind::Cliff {
        return None;
    }

    // World -> Pixel Studio must follow the same semantic terrain authority as
    // runtime/worldgen. Do not grow another TileKind -> atlas match table here.
    let semantic_id = haven_assets::runtime_asset_adapters::terrain_semantic_id(tile);
    resolve_reviewed_edit_binding(tile, semantic_id)
        .or_else(|| resolve_semantic_terrain_source(tile, semantic_id))
}

fn resolve_reviewed_edit_binding(
    tile: TileKind,
    semantic_id: &'static str,
) -> Option<ResolvedWorldAsset> {
    let reviewed_semantic = match tile {
        TileKind::Grass | TileKind::TallGrass => "terrain.lpc_v7.grass.pure_fill",
        TileKind::Sand => "terrain.lpc_v7.sand.pure_fill",
        TileKind::WetSand => "terrain.lpc_v7.water.shallows.sand.pure_fill",
        TileKind::Dirt => "terrain.lpc_v7.dirt.brown.pure_fill",
        TileKind::MountainRock => "terrain.lpc_v7.rock.dark.pure_fill",
        TileKind::MountainPath | TileKind::Road | TileKind::StonePath => {
            "terrain.lpc_v7.dirt.roots.pure_fill"
        }
        TileKind::CaveFloor => "terrain.lpc_v7.mudstone.brown.pure_fill",
        TileKind::TilledSoil | TileKind::WateredSoil => "terrain.lpc_v7.soil.pure_fill",
        TileKind::MudBank => "terrain.lpc_v7.mud.brown.pure_fill",
        TileKind::Water | TileKind::ShallowWater | TileKind::OceanShallow | TileKind::RiverWater => {
            "terrain.lpc_v7.water.pure_fill"
        }
        TileKind::DeepWater | TileKind::OceanDeep => "terrain.lpc_v7.water.deep.pure_fill",
        _ => return None,
    };
    let bindings_path = repo_root_dir().join("content/editor/world_asset_edit_bindings_v0_2.json");
    let bytes = std::fs::read(bindings_path).ok()?;
    let catalog: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let binding = catalog.get("bindings")?.as_array()?.iter().find(|binding| {
        binding.get("semantic_id").and_then(serde_json::Value::as_str) == Some(reviewed_semantic)
    })?;
    let source_path = binding.get("source_path")?.as_str()?;
    let rect = binding.get("source_rect")?.as_array()?;
    if rect.len() != 4 { return None; }
    let component = |index: usize| -> Option<u32> {
        u32::try_from(rect.get(index)?.as_u64()?).ok()
    };
    Some(ResolvedWorldAsset {
        semantic_id: semantic_id.to_string(),
        source_path: source_path.to_string(),
        source_rect: PixelSelection {
            x: component(0)?, y: component(1)?, width: component(2)?, height: component(3)?,
        },
        generated_output: false,
    })
}

fn resolve_semantic_terrain_source(
    tile: TileKind,
    semantic_id: &'static str,
) -> Option<ResolvedWorldAsset> {
    let registry_path = repo_root_dir()
        .join("content/worldgen/terrain_world_semantic_registry_v1.json");
    let bytes = std::fs::read(&registry_path).ok()?;
    let registry: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let tile_kind = format!("{tile:?}");
    let material = registry
        .get("materials")?
        .as_array()?
        .iter()
        .find(|material| {
            material.get("id").and_then(serde_json::Value::as_str) == Some(semantic_id)
                || material.get("tileKind").and_then(serde_json::Value::as_str)
                    == Some(tile_kind.as_str())
        })?;
    let source = material.get("source")?;
    let source_path = source.get("image")?.as_str()?;
    let rect = source.get("rect")?.as_array()?;
    if rect.len() != 4 {
        return None;
    }
    let component = |index: usize| -> Option<u32> {
        u32::try_from(rect.get(index)?.as_u64()?).ok()
    };
    let lifecycle = material
        .get("ownership")
        .and_then(|ownership| ownership.get("lifecycle"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("authored");
    let generated_output = lifecycle.eq_ignore_ascii_case("generated")
        || source_path.contains("/generated/")
        || source_path.contains("\\generated\\");

    // Semantic ids originate from the static runtime adapter and therefore
    // remain valid for the lifetime of this binding.
    Some(ResolvedWorldAsset {
        semantic_id: semantic_id.to_string(),
        source_path: source_path.to_string(),
        source_rect: PixelSelection {
            x: component(0)?,
            y: component(1)?,
            width: component(2)?,
            height: component(3)?,
        },
        generated_output,
    })
}
fn resolve_project_path(path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        repo_root_dir().join(candidate)
    }
}
