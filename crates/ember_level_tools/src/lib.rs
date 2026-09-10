//! Higher-level editor commands for native Ember level authoring.

use ember_commands::{Command, CommandError};
use ember_core::StableId;
use ember_level::LevelDocument;
use ember_scene::{SceneLayerKind, TileCell, TileTransform};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl RectI {
    pub fn cells(self) -> impl Iterator<Item = [i32; 3]> {
        let x = self.x;
        let y = self.y;
        let width = self.width;
        let height = self.height;
        (0..height).flat_map(move |dy| (0..width).map(move |dx| [x + dx as i32, y + dy as i32, 0]))
    }
}

pub struct FillTilesCommand {
    pub layer: StableId,
    pub rect: RectI,
    pub tile_id: i32,
    pub source_pixels: [i32; 2],
    pub tileset: Option<StableId>,
    before: Option<Vec<TileCell>>,
}

impl FillTilesCommand {
    pub fn new(
        layer: StableId,
        rect: RectI,
        tile_id: i32,
        source_pixels: [i32; 2],
        tileset: Option<StableId>,
    ) -> Self {
        Self {
            layer,
            rect,
            tile_id,
            source_pixels,
            tileset,
            before: None,
        }
    }
}

impl Command<LevelDocument> for FillTilesCommand {
    fn label(&self) -> &str {
        "Fill Tile Rectangle"
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
        if self.before.is_none() {
            self.before = Some(layer.tile_cells.clone());
        }
        for grid in self.rect.cells() {
            let replacement = TileCell {
                grid,
                source_pixels: self.source_pixels,
                tile_id: self.tile_id,
                transform: TileTransform::default(),
                tileset: self.tileset.clone(),
            };
            if let Some(index) = layer
                .tile_cells
                .iter()
                .position(|existing| existing.grid == grid)
            {
                layer.tile_cells[index] = replacement;
            } else {
                layer.tile_cells.push(replacement);
            }
        }
        Ok(())
    }

    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let before = self
            .before
            .clone()
            .ok_or_else(|| CommandError("fill command has not executed".into()))?;
        let layer = context
            .layer_mut(&self.layer)
            .ok_or_else(|| CommandError("tile layer not found".into()))?;
        layer.tile_cells = before;
        Ok(())
    }
}

pub struct ReorderLayerCommand {
    pub layer: StableId,
    pub target_index: usize,
    previous_index: Option<usize>,
}

impl ReorderLayerCommand {
    pub fn new(layer: StableId, target_index: usize) -> Self {
        Self {
            layer,
            target_index,
            previous_index: None,
        }
    }

    fn move_layer(
        context: &mut LevelDocument,
        layer: &StableId,
        target: usize,
    ) -> Result<(), CommandError> {
        let source = context
            .scene
            .layers
            .iter()
            .position(|candidate| &candidate.id == layer)
            .ok_or_else(|| CommandError("layer to reorder was not found".into()))?;
        let value = context.scene.layers.remove(source);
        let target = target.min(context.scene.layers.len());
        context.scene.layers.insert(target, value);
        Ok(())
    }
}

impl Command<LevelDocument> for ReorderLayerCommand {
    fn label(&self) -> &str {
        "Reorder Level Layer"
    }

    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        if self.previous_index.is_none() {
            self.previous_index = context
                .scene
                .layers
                .iter()
                .position(|candidate| candidate.id == self.layer);
        }
        Self::move_layer(context, &self.layer, self.target_index)
    }

    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let previous = self
            .previous_index
            .ok_or_else(|| CommandError("reorder command has not executed".into()))?;
        Self::move_layer(context, &self.layer, previous)
    }
}

pub struct SetEntityPropertyCommand {
    pub entity: StableId,
    pub key: String,
    pub value: Value,
    previous: Option<Value>,
    had_previous: bool,
}

impl SetEntityPropertyCommand {
    pub fn new(entity: StableId, key: impl Into<String>, value: Value) -> Self {
        Self {
            entity,
            key: key.into(),
            value,
            previous: None,
            had_previous: false,
        }
    }
}

impl Command<LevelDocument> for SetEntityPropertyCommand {
    fn label(&self) -> &str {
        "Set Entity Property"
    }

    fn execute(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let entity = context
            .scene
            .entities
            .iter_mut()
            .find(|candidate| candidate.id == self.entity)
            .ok_or_else(|| CommandError("entity was not found".into()))?;
        if !self.had_previous {
            self.previous = entity.properties.get(&self.key).cloned();
            self.had_previous = true;
        }
        entity
            .properties
            .insert(self.key.clone(), self.value.clone());
        Ok(())
    }

    fn undo(&mut self, context: &mut LevelDocument) -> Result<(), CommandError> {
        let entity = context
            .scene
            .entities
            .iter_mut()
            .find(|candidate| candidate.id == self.entity)
            .ok_or_else(|| CommandError("entity was not found".into()))?;
        match self.previous.clone() {
            Some(value) => {
                entity.properties.insert(self.key.clone(), value);
            }
            None => {
                entity.properties.remove(&self.key);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_commands::CommandHistory;
    use ember_level::{tile_layer, LevelDocument};

    #[test]
    fn fill_rectangle_round_trips_through_undo() {
        let mut level = LevelDocument::new(
            StableId::new("level", "fill-test").unwrap(),
            "Fill Test",
            320,
            180,
        );
        let layer_id = StableId::new("layer", "ground").unwrap();
        level
            .scene
            .layers
            .push(tile_layer(layer_id.clone(), "Ground", 16, 0));
        let mut history = CommandHistory::default();
        history
            .execute(
                Box::new(FillTilesCommand::new(
                    layer_id.clone(),
                    RectI {
                        x: 2,
                        y: 3,
                        width: 3,
                        height: 2,
                    },
                    7,
                    [0, 0],
                    None,
                )),
                &mut level,
            )
            .unwrap();
        assert_eq!(level.layer(&layer_id).unwrap().tile_cells.len(), 6);
        history.undo(&mut level).unwrap();
        assert!(level.layer(&layer_id).unwrap().tile_cells.is_empty());
    }
}
