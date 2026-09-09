//! Native Ember level-authoring model.
//!
//! This crate owns the editable level semantics used by Ember Editor. LDtk is an
//! interchange/reference source only; the authoring model here is Ember-owned.

use ember_commands::{Command, CommandError};
use ember_core::StableId;
use ember_scene::{
    EntityInstance, SceneDocument, SceneLayer, SceneLayerKind, SemanticBinding, SemanticCell,
    TileCell,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LevelDocument {
    pub scene: SceneDocument,
    pub definitions: LevelDefinitions,
    pub settings: LevelSettings,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct LevelDefinitions {
    pub semantic_values: Vec<SemanticValueDefinition>,
    pub entity_definitions: Vec<EntityDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SemanticValueDefinition {
    pub id: StableId,
    pub value: i32,
    pub name: String,
    pub display_color_rgba: [u8; 4],
    pub binding: SemanticBinding,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityDefinition {
    pub id: StableId,
    pub name: String,
    pub size_pixels: [i32; 2],
    pub tags: Vec<StableId>,
    pub default_properties: BTreeMap<String, Value>,
    pub default_behavior_graphs: Vec<StableId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LevelSettings {
    pub default_grid_size: i32,
    pub background_rgba: [u8; 4],
    pub elevation_step_pixels: i32,
}

impl Default for LevelSettings {
    fn default() -> Self {
        Self {
            default_grid_size: 16,
            background_rgba: [32, 32, 32, 255],
            elevation_step_pixels: 16,
        }
    }
}

impl LevelDocument {
    pub fn new(
        id: StableId,
        name: impl Into<String>,
        width_pixels: i32,
        height_pixels: i32,
    ) -> Self {
        Self {
            scene: SceneDocument {
                id,
                name: name.into(),
                width_pixels,
                height_pixels,
                world_origin: [0, 0, 0],
                layers: Vec::new(),
                entities: Vec::new(),
                properties: BTreeMap::new(),
            },
            definitions: LevelDefinitions::default(),
            settings: LevelSettings::default(),
        }
    }

    pub fn layer(&self, id: &StableId) -> Option<&SceneLayer> {
        self.scene.layers.iter().find(|layer| &layer.id == id)
    }

    pub fn layer_mut(&mut self, id: &StableId) -> Option<&mut SceneLayer> {
        self.scene.layers.iter_mut().find(|layer| &layer.id == id)
    }

    pub fn semantic_definition(&self, value: i32) -> Option<&SemanticValueDefinition> {
        self.definitions
            .semantic_values
            .iter()
            .find(|definition| definition.value == value)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = self.scene.validate().err().unwrap_or_default();
        if self.settings.default_grid_size <= 0 {
            errors.push("default grid size must be positive".to_string());
        }
        if self.settings.elevation_step_pixels <= 0 {
            errors.push("elevation step must be positive".to_string());
        }
        let mut semantic_values = std::collections::BTreeSet::new();
        for definition in &self.definitions.semantic_values {
            if !semantic_values.insert(definition.value) {
                errors.push(format!("duplicate semantic value {}", definition.value));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Clone, Debug)]
pub struct AddLayerCommand {
    pub layer: SceneLayer,
    inserted_index: Option<usize>,
}

impl AddLayerCommand {
    pub fn new(layer: SceneLayer) -> Self {
        Self {
            layer,
            inserted_index: None,
        }
    }
}

impl Command<LevelDocument> for AddLayerCommand {
    fn label(&self) -> &str {
        "Add Level Layer"
    }
    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        if context.layer(&self.layer.id).is_some() {
            return Err(CommandError(format!(
                "layer {} already exists",
                self.layer.id
            )));
        }
        let index = self
            .inserted_index
            .unwrap_or(context.scene.layers.len())
            .min(context.scene.layers.len());
        context.scene.layers.insert(index, self.layer.clone());
        self.inserted_index = Some(index);
        Ok(())
    }
    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let index = context
            .scene
            .layers
            .iter()
            .position(|layer| layer.id == self.layer.id)
            .ok_or_else(|| CommandError("layer to undo was not found".into()))?;
        context.scene.layers.remove(index);
        self.inserted_index = Some(index);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PaintTileCommand {
    pub layer: StableId,
    pub cell: TileCell,
    previous: Option<TileCell>,
    had_previous: bool,
}

impl PaintTileCommand {
    pub fn new(layer: StableId, cell: TileCell) -> Self {
        Self {
            layer,
            cell,
            previous: None,
            had_previous: false,
        }
    }
}

impl Command<LevelDocument> for PaintTileCommand {
    fn label(&self) -> &str {
        "Paint Tile"
    }
    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let layer = context
            .layer_mut(&self.layer)
            .ok_or_else(|| CommandError("tile layer not found".into()))?;
        if !matches!(
            layer.kind,
            SceneLayerKind::Tiles | SceneLayerKind::AutoTiles
        ) {
            return Err(CommandError("target layer does not accept tiles".into()));
        }
        if let Some(index) = layer
            .tile_cells
            .iter()
            .position(|existing| existing.grid == self.cell.grid)
        {
            if !self.had_previous {
                self.previous = Some(layer.tile_cells[index].clone());
                self.had_previous = true;
            }
            layer.tile_cells[index] = self.cell.clone();
        } else {
            layer.tile_cells.push(self.cell.clone());
        }
        Ok(())
    }
    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let layer = context
            .layer_mut(&self.layer)
            .ok_or_else(|| CommandError("tile layer not found".into()))?;
        let index = layer
            .tile_cells
            .iter()
            .position(|existing| existing.grid == self.cell.grid)
            .ok_or_else(|| CommandError("painted tile no longer exists".into()))?;
        if self.had_previous {
            layer.tile_cells[index] = self
                .previous
                .clone()
                .ok_or_else(|| CommandError("missing previous tile".into()))?;
        } else {
            layer.tile_cells.remove(index);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PaintSemanticCommand {
    pub layer: StableId,
    pub grid: [i32; 3],
    pub value: i32,
    previous: Option<SemanticCell>,
    had_previous: bool,
}

impl PaintSemanticCommand {
    pub fn new(layer: StableId, grid: [i32; 3], value: i32) -> Self {
        Self {
            layer,
            grid,
            value,
            previous: None,
            had_previous: false,
        }
    }
}

impl Command<LevelDocument> for PaintSemanticCommand {
    fn label(&self) -> &str {
        "Paint Semantic Cell"
    }
    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let binding = context
            .semantic_definition(self.value)
            .ok_or_else(|| CommandError(format!("semantic value {} is not defined", self.value)))?
            .binding
            .clone();
        let layer = context
            .layer_mut(&self.layer)
            .ok_or_else(|| CommandError("semantic layer not found".into()))?;
        if layer.kind != SceneLayerKind::SemanticGrid {
            return Err(CommandError("target layer is not a semantic grid".into()));
        }
        let cell = SemanticCell {
            grid: self.grid,
            value: self.value,
            semantic: binding,
        };
        if let Some(index) = layer
            .semantic_cells
            .iter()
            .position(|existing| existing.grid == self.grid)
        {
            if !self.had_previous {
                self.previous = Some(layer.semantic_cells[index].clone());
                self.had_previous = true;
            }
            layer.semantic_cells[index] = cell;
        } else {
            layer.semantic_cells.push(cell);
        }
        Ok(())
    }
    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let layer = context
            .layer_mut(&self.layer)
            .ok_or_else(|| CommandError("semantic layer not found".into()))?;
        let index = layer
            .semantic_cells
            .iter()
            .position(|existing| existing.grid == self.grid)
            .ok_or_else(|| CommandError("semantic cell no longer exists".into()))?;
        if self.had_previous {
            layer.semantic_cells[index] = self
                .previous
                .clone()
                .ok_or_else(|| CommandError("missing previous semantic cell".into()))?;
        } else {
            layer.semantic_cells.remove(index);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PlaceEntityCommand {
    pub entity: EntityInstance,
    inserted_index: Option<usize>,
}

impl PlaceEntityCommand {
    pub fn new(entity: EntityInstance) -> Self {
        Self {
            entity,
            inserted_index: None,
        }
    }
}

impl Command<LevelDocument> for PlaceEntityCommand {
    fn label(&self) -> &str {
        "Place Entity"
    }
    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        if context
            .scene
            .entities
            .iter()
            .any(|entity| entity.id == self.entity.id)
        {
            return Err(CommandError(format!(
                "entity {} already exists",
                self.entity.id
            )));
        }
        let index = self
            .inserted_index
            .unwrap_or(context.scene.entities.len())
            .min(context.scene.entities.len());
        context.scene.entities.insert(index, self.entity.clone());
        self.inserted_index = Some(index);
        Ok(())
    }
    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let index = context
            .scene
            .entities
            .iter()
            .position(|entity| entity.id == self.entity.id)
            .ok_or_else(|| CommandError("entity to undo was not found".into()))?;
        context.scene.entities.remove(index);
        self.inserted_index = Some(index);
        Ok(())
    }
}

pub fn tile_layer(
    id: StableId,
    name: impl Into<String>,
    grid_size: i32,
    z_index: i32,
) -> SceneLayer {
    empty_layer(id, name, SceneLayerKind::Tiles, grid_size, z_index)
}

pub fn semantic_layer(
    id: StableId,
    name: impl Into<String>,
    grid_size: i32,
    z_index: i32,
) -> SceneLayer {
    empty_layer(id, name, SceneLayerKind::SemanticGrid, grid_size, z_index)
}

fn empty_layer(
    id: StableId,
    name: impl Into<String>,
    kind: SceneLayerKind,
    grid_size: i32,
    z_index: i32,
) -> SceneLayer {
    SceneLayer {
        id,
        name: name.into(),
        kind,
        grid_size,
        visible: true,
        opacity: 1.0,
        z_index,
        tile_cells: Vec::new(),
        semantic_cells: Vec::new(),
        properties: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_commands::CommandHistory;
    use ember_scene::{CollisionSemantic, TileTransform};

    fn id(namespace: &str, value: &str) -> StableId {
        StableId::new(namespace, value).unwrap()
    }

    #[test]
    fn tile_paint_round_trips_through_undo_redo() {
        let mut level = LevelDocument::new(id("level", "test"), "Test", 320, 180);
        let layer_id = id("layer", "ground");
        level
            .scene
            .layers
            .push(tile_layer(layer_id.clone(), "Ground", 16, 0));
        let cell = TileCell {
            grid: [2, 3, 0],
            source_pixels: [16, 0],
            tile_id: 1,
            transform: TileTransform::default(),
            tileset: None,
        };
        let mut history = CommandHistory::default();
        history
            .execute(
                Box::new(PaintTileCommand::new(layer_id.clone(), cell)),
                &mut level,
            )
            .unwrap();
        assert_eq!(level.layer(&layer_id).unwrap().tile_cells.len(), 1);
        history.undo(&mut level).unwrap();
        assert!(level.layer(&layer_id).unwrap().tile_cells.is_empty());
        history.redo(&mut level).unwrap();
        assert_eq!(level.layer(&layer_id).unwrap().tile_cells.len(), 1);
    }

    #[test]
    fn semantic_paint_uses_definition_binding() {
        let mut level = LevelDocument::new(id("level", "test"), "Test", 320, 180);
        let layer_id = id("layer", "semantic");
        level
            .scene
            .layers
            .push(semantic_layer(layer_id.clone(), "Semantic", 16, 0));
        level
            .definitions
            .semantic_values
            .push(SemanticValueDefinition {
                id: id("semantic", "solid"),
                value: 1,
                name: "Solid".into(),
                display_color_rgba: [255, 0, 0, 180],
                binding: SemanticBinding {
                    material: None,
                    collision: Some(CollisionSemantic::Solid),
                    navigation_cost: None,
                    tags: vec![],
                    behavior_graphs: vec![],
                    properties: BTreeMap::new(),
                },
            });
        let mut command = PaintSemanticCommand::new(layer_id.clone(), [1, 1, 0], 1);
        command.execute(&mut level).unwrap();
        assert_eq!(
            level.layer(&layer_id).unwrap().semantic_cells[0]
                .semantic
                .collision,
            Some(CollisionSemantic::Solid)
        );
    }
}

#[derive(Debug)]
pub enum LevelIoError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(Vec<String>),
}

impl std::fmt::Display for LevelIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "level I/O failed: {error}"),
            Self::Json(error) => write!(f, "level JSON failed: {error}"),
            Self::Invalid(errors) => write!(f, "level validation failed: {}", errors.join("; ")),
        }
    }
}
impl std::error::Error for LevelIoError {}
impl From<std::io::Error> for LevelIoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for LevelIoError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn save_level_json(
    path: impl AsRef<std::path::Path>,
    level: &LevelDocument,
) -> Result<(), LevelIoError> {
    if let Err(errors) = level.validate() {
        return Err(LevelIoError::Invalid(errors));
    }
    let bytes = serde_json::to_vec_pretty(level)?;
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("level");
    let temp = path.with_file_name(format!(".{file_name}.ember-level.tmp"));
    std::fs::write(&temp, bytes)?;
    if path.exists() {
        #[cfg(windows)]
        {
            let backup = path.with_extension("ember-level.bak");
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(path, &backup)?;
            match std::fs::rename(&temp, path) {
                Ok(()) => {
                    let _ = std::fs::remove_file(backup);
                }
                Err(error) => {
                    let _ = std::fs::rename(backup, path);
                    return Err(LevelIoError::Io(error));
                }
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(&temp, path)?;
    } else {
        std::fs::rename(&temp, path)?;
    }
    Ok(())
}

pub fn load_level_json(path: impl AsRef<std::path::Path>) -> Result<LevelDocument, LevelIoError> {
    let bytes = std::fs::read(path)?;
    let level: LevelDocument = serde_json::from_slice(&bytes)?;
    if let Err(errors) = level.validate() {
        return Err(LevelIoError::Invalid(errors));
    }
    Ok(level)
}
