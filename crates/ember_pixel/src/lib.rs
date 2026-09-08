//! Ember-owned pixel and animation document model.
//!
//! This model is intentionally independent from LibreSprite/Aseprite source.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type LayerId = u64;
pub type FrameId = u64;
pub type CelId = u64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PixelDocument {
    pub width: u32,
    pub height: u32,
    pub color_mode: PixelColorMode,
    pub layers: Vec<PixelLayer>,
    pub frames: Vec<PixelFrame>,
    pub cels: Vec<PixelCel>,
    pub frame_tags: Vec<FrameTag>,
    pub palettes: Vec<PixelPalette>,
    pub active_palette: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PixelColorMode {
    Rgba,
    Indexed,
    Grayscale,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PixelLayer {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: u8,
    pub blend_mode: BlendMode,
    pub parent: Option<LayerId>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Add,
    Subtract,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PixelFrame {
    pub id: FrameId,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PixelCel {
    pub id: CelId,
    pub layer: LayerId,
    pub frame: FrameId,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub linked_to: Option<CelId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrameTag {
    pub name: String,
    pub from_frame: FrameId,
    pub to_frame: FrameId,
    pub direction: PlaybackDirection,
    pub repeat: RepeatMode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlaybackDirection {
    Forward,
    Reverse,
    PingPong,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepeatMode {
    Once,
    Count(u32),
    Loop,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PixelPalette {
    pub id: u64,
    pub name: String,
    pub colors: Vec<[u8; 4]>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionMask {
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub coverage: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TiledDrawingMode {
    #[default]
    None,
    WrapX,
    WrapY,
    WrapBoth,
}

impl PixelDocument {
    pub fn new(width: u32, height: u32) -> Self {
        let layer = PixelLayer {
            id: 1,
            name: "Layer 1".into(),
            visible: true,
            locked: false,
            opacity: 255,
            blend_mode: BlendMode::Normal,
            parent: None,
        };
        let frame = PixelFrame {
            id: 1,
            duration_ms: 100,
        };
        let cel = PixelCel {
            id: 1,
            layer: layer.id,
            frame: frame.id,
            origin_x: 0,
            origin_y: 0,
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
            linked_to: None,
        };
        Self {
            width,
            height,
            color_mode: PixelColorMode::Rgba,
            layers: vec![layer],
            frames: vec![frame],
            cels: vec![cel],
            frame_tags: Vec::new(),
            palettes: vec![PixelPalette {
                id: 1,
                name: "Default".into(),
                colors: Vec::new(),
            }],
            active_palette: Some(1),
            metadata: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("pixel document dimensions must be non-zero".into());
        }
        for cel in &self.cels {
            let expected = (cel.width * cel.height * 4) as usize;
            if cel.rgba.len() != expected {
                return Err(format!("cel {} pixel buffer length mismatch", cel.id));
            }
            if !self.layers.iter().any(|layer| layer.id == cel.layer) {
                return Err(format!("cel {} references missing layer", cel.id));
            }
            if !self.frames.iter().any(|frame| frame.id == cel.frame) {
                return Err(format!("cel {} references missing frame", cel.id));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_document_is_valid() {
        PixelDocument::new(32, 32)
            .validate()
            .expect("new document validates");
    }

    #[test]
    fn tiled_drawing_mode_defaults_to_none() {
        assert_eq!(TiledDrawingMode::default(), TiledDrawingMode::None);
    }
}

#[derive(Default)]
pub struct PixelEditContext {
    pub document: Option<PixelDocument>,
    pub selection: Option<SelectionMask>,
    pub tiled_mode: TiledDrawingMode,
}

impl PixelDocument {
    pub fn layer(&self, id: LayerId) -> Option<&PixelLayer> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub fn cel(&self, layer: LayerId, frame: FrameId) -> Option<&PixelCel> {
        self.cels
            .iter()
            .find(|cel| cel.layer == layer && cel.frame == frame)
    }

    pub fn cel_mut(&mut self, layer: LayerId, frame: FrameId) -> Option<&mut PixelCel> {
        self.cels
            .iter_mut()
            .find(|cel| cel.layer == layer && cel.frame == frame)
    }

    pub fn next_layer_id(&self) -> LayerId {
        self.layers.iter().map(|layer| layer.id).max().unwrap_or(0) + 1
    }

    pub fn next_frame_id(&self) -> FrameId {
        self.frames.iter().map(|frame| frame.id).max().unwrap_or(0) + 1
    }

    pub fn next_cel_id(&self) -> CelId {
        self.cels.iter().map(|cel| cel.id).max().unwrap_or(0) + 1
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> LayerId {
        let id = self.next_layer_id();
        self.layers.push(PixelLayer {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 255,
            blend_mode: BlendMode::Normal,
            parent: None,
        });
        for frame in self.frames.clone() {
            self.cels.push(PixelCel {
                id: self.next_cel_id(),
                layer: id,
                frame: frame.id,
                origin_x: 0,
                origin_y: 0,
                width: self.width,
                height: self.height,
                rgba: vec![0; (self.width * self.height * 4) as usize],
                linked_to: None,
            });
        }
        id
    }

    pub fn add_frame(&mut self, duration_ms: u32) -> FrameId {
        let id = self.next_frame_id();
        self.frames.push(PixelFrame {
            id,
            duration_ms: duration_ms.max(1),
        });
        for layer in self.layers.clone() {
            self.cels.push(PixelCel {
                id: self.next_cel_id(),
                layer: layer.id,
                frame: id,
                origin_x: 0,
                origin_y: 0,
                width: self.width,
                height: self.height,
                rgba: vec![0; (self.width * self.height * 4) as usize],
                linked_to: None,
            });
        }
        id
    }

    pub fn set_pixel(
        &mut self,
        layer: LayerId,
        frame: FrameId,
        x: i32,
        y: i32,
        color: [u8; 4],
        tiled_mode: TiledDrawingMode,
    ) -> Result<[u8; 4], String> {
        let width = self.width as i32;
        let height = self.height as i32;
        let x = wrap_coordinate(
            x,
            width,
            matches!(
                tiled_mode,
                TiledDrawingMode::WrapX | TiledDrawingMode::WrapBoth
            ),
        )?;
        let y = wrap_coordinate(
            y,
            height,
            matches!(
                tiled_mode,
                TiledDrawingMode::WrapY | TiledDrawingMode::WrapBoth
            ),
        )?;
        let cel = self
            .cel_mut(layer, frame)
            .ok_or_else(|| "target cel does not exist".to_string())?;
        if cel.linked_to.is_some() {
            return Err("linked cels must be unlinked before direct pixel edits".into());
        }
        let local_x = x - cel.origin_x;
        let local_y = y - cel.origin_y;
        if local_x < 0 || local_y < 0 || local_x >= cel.width as i32 || local_y >= cel.height as i32
        {
            return Err("pixel falls outside cel bounds".into());
        }
        let index = ((local_y as u32 * cel.width + local_x as u32) * 4) as usize;
        let old = [
            cel.rgba[index],
            cel.rgba[index + 1],
            cel.rgba[index + 2],
            cel.rgba[index + 3],
        ];
        cel.rgba[index..index + 4].copy_from_slice(&color);
        Ok(old)
    }
}

fn wrap_coordinate(value: i32, size: i32, wrap: bool) -> Result<i32, String> {
    if size <= 0 {
        return Err("invalid document dimension".into());
    }
    if wrap {
        Ok(value.rem_euclid(size))
    } else if (0..size).contains(&value) {
        Ok(value)
    } else {
        Err("pixel coordinate outside document".into())
    }
}

pub struct PaintPixelCommand {
    pub layer: LayerId,
    pub frame: FrameId,
    pub x: i32,
    pub y: i32,
    pub color: [u8; 4],
    previous: Option<[u8; 4]>,
}

impl PaintPixelCommand {
    pub fn new(layer: LayerId, frame: FrameId, x: i32, y: i32, color: [u8; 4]) -> Self {
        Self {
            layer,
            frame,
            x,
            y,
            color,
            previous: None,
        }
    }
}

impl ember_commands::Command<PixelEditContext> for PaintPixelCommand {
    fn label(&self) -> &str {
        "Paint Pixel"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        let previous = document
            .set_pixel(
                self.layer,
                self.frame,
                self.x,
                self.y,
                self.color,
                context.tiled_mode,
            )
            .map_err(ember_commands::CommandError)?;
        self.previous = Some(previous);
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let previous = self
            .previous
            .ok_or_else(|| ember_commands::CommandError("paint command has not executed".into()))?;
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        document
            .set_pixel(
                self.layer,
                self.frame,
                self.x,
                self.y,
                previous,
                context.tiled_mode,
            )
            .map_err(ember_commands::CommandError)?;
        Ok(())
    }
}

pub struct AddLayerCommand {
    pub name: String,
    inserted_layer: Option<PixelLayer>,
    inserted_cels: Vec<PixelCel>,
}

impl AddLayerCommand {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inserted_layer: None,
            inserted_cels: Vec::new(),
        }
    }
}

impl ember_commands::Command<PixelEditContext> for AddLayerCommand {
    fn label(&self) -> &str {
        "Add Layer"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        if let Some(layer) = self.inserted_layer.clone() {
            document.layers.push(layer);
            document.cels.extend(self.inserted_cels.clone());
        } else {
            let before = document.cels.len();
            let id = document.add_layer(self.name.clone());
            self.inserted_layer = document.layer(id).cloned();
            self.inserted_cels = document.cels[before..].to_vec();
        }
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let layer_id = self
            .inserted_layer
            .as_ref()
            .ok_or_else(|| ember_commands::CommandError("layer command has not executed".into()))?
            .id;
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        document.layers.retain(|layer| layer.id != layer_id);
        document.cels.retain(|cel| cel.layer != layer_id);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelChange {
    pub x: i32,
    pub y: i32,
    pub before: [u8; 4],
    pub after: [u8; 4],
}

pub struct PaintStrokeCommand {
    pub layer: LayerId,
    pub frame: FrameId,
    pub points: Vec<(i32, i32)>,
    pub color: [u8; 4],
    changes: Vec<PixelChange>,
}

impl PaintStrokeCommand {
    pub fn new(
        layer: LayerId,
        frame: FrameId,
        points: impl IntoIterator<Item = (i32, i32)>,
        color: [u8; 4],
    ) -> Self {
        Self {
            layer,
            frame,
            points: points.into_iter().collect(),
            color,
            changes: Vec::new(),
        }
    }
}

impl ember_commands::Command<PixelEditContext> for PaintStrokeCommand {
    fn label(&self) -> &str {
        "Paint Stroke"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        self.changes.clear();
        for &(x, y) in &self.points {
            let before = document
                .set_pixel(self.layer, self.frame, x, y, self.color, context.tiled_mode)
                .map_err(ember_commands::CommandError)?;
            self.changes.push(PixelChange {
                x,
                y,
                before,
                after: self.color,
            });
        }
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        for change in self.changes.iter().rev() {
            document
                .set_pixel(
                    self.layer,
                    self.frame,
                    change.x,
                    change.y,
                    change.before,
                    context.tiled_mode,
                )
                .map_err(ember_commands::CommandError)?;
        }
        Ok(())
    }
}

pub struct AddFrameCommand {
    pub duration_ms: u32,
    inserted_frame: Option<PixelFrame>,
    inserted_cels: Vec<PixelCel>,
}

impl AddFrameCommand {
    pub fn new(duration_ms: u32) -> Self {
        Self {
            duration_ms,
            inserted_frame: None,
            inserted_cels: Vec::new(),
        }
    }
}

impl ember_commands::Command<PixelEditContext> for AddFrameCommand {
    fn label(&self) -> &str {
        "Add Frame"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        if let Some(frame) = self.inserted_frame.clone() {
            document.frames.push(frame);
            document.cels.extend(self.inserted_cels.clone());
        } else {
            let before = document.cels.len();
            let id = document.add_frame(self.duration_ms);
            self.inserted_frame = document.frames.iter().find(|frame| frame.id == id).cloned();
            self.inserted_cels = document.cels[before..].to_vec();
        }
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let frame_id = self
            .inserted_frame
            .as_ref()
            .ok_or_else(|| ember_commands::CommandError("frame command has not executed".into()))?
            .id;
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        document.frames.retain(|frame| frame.id != frame_id);
        document.cels.retain(|cel| cel.frame != frame_id);
        Ok(())
    }
}

pub struct SetFrameDurationCommand {
    pub frame: FrameId,
    pub duration_ms: u32,
    previous: Option<u32>,
}

impl SetFrameDurationCommand {
    pub fn new(frame: FrameId, duration_ms: u32) -> Self {
        Self {
            frame,
            duration_ms: duration_ms.max(1),
            previous: None,
        }
    }
}

impl ember_commands::Command<PixelEditContext> for SetFrameDurationCommand {
    fn label(&self) -> &str {
        "Set Frame Duration"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        let frame = document
            .frames
            .iter_mut()
            .find(|candidate| candidate.id == self.frame)
            .ok_or_else(|| ember_commands::CommandError("frame does not exist".into()))?;
        self.previous = Some(frame.duration_ms);
        frame.duration_ms = self.duration_ms;
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let previous = self.previous.ok_or_else(|| {
            ember_commands::CommandError("duration command has not executed".into())
        })?;
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        let frame = document
            .frames
            .iter_mut()
            .find(|candidate| candidate.id == self.frame)
            .ok_or_else(|| ember_commands::CommandError("frame does not exist".into()))?;
        frame.duration_ms = previous;
        Ok(())
    }
}

pub struct AddPaletteColorCommand {
    pub palette: u64,
    pub color: [u8; 4],
    inserted_index: Option<usize>,
}

impl AddPaletteColorCommand {
    pub fn new(palette: u64, color: [u8; 4]) -> Self {
        Self {
            palette,
            color,
            inserted_index: None,
        }
    }
}

impl ember_commands::Command<PixelEditContext> for AddPaletteColorCommand {
    fn label(&self) -> &str {
        "Add Palette Color"
    }

    fn execute(
        &mut self,
        context: &mut PixelEditContext,
    ) -> Result<(), ember_commands::CommandError> {
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        let palette = document
            .palettes
            .iter_mut()
            .find(|palette| palette.id == self.palette)
            .ok_or_else(|| ember_commands::CommandError("palette does not exist".into()))?;
        let index = self.inserted_index.unwrap_or(palette.colors.len());
        if index > palette.colors.len() {
            return Err(ember_commands::CommandError(
                "palette insertion index is invalid".into(),
            ));
        }
        palette.colors.insert(index, self.color);
        self.inserted_index = Some(index);
        Ok(())
    }

    fn undo(&mut self, context: &mut PixelEditContext) -> Result<(), ember_commands::CommandError> {
        let index = self.inserted_index.ok_or_else(|| {
            ember_commands::CommandError("palette command has not executed".into())
        })?;
        let document = context
            .document
            .as_mut()
            .ok_or_else(|| ember_commands::CommandError("no pixel document is open".into()))?;
        let palette = document
            .palettes
            .iter_mut()
            .find(|palette| palette.id == self.palette)
            .ok_or_else(|| ember_commands::CommandError("palette does not exist".into()))?;
        if index >= palette.colors.len() {
            return Err(ember_commands::CommandError(
                "palette color is missing".into(),
            ));
        }
        palette.colors.remove(index);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineSnapshot {
    pub frame_order: Vec<FrameId>,
    pub durations_ms: Vec<u32>,
    pub layer_order: Vec<LayerId>,
}

impl PixelDocument {
    pub fn timeline_snapshot(&self) -> TimelineSnapshot {
        TimelineSnapshot {
            frame_order: self.frames.iter().map(|frame| frame.id).collect(),
            durations_ms: self.frames.iter().map(|frame| frame.duration_ms).collect(),
            layer_order: self.layers.iter().map(|layer| layer.id).collect(),
        }
    }
}
