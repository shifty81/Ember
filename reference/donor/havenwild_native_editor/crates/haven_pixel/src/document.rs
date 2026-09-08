use crate::layers::composite_layers;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelTool {
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    Selection,
    Line,
    Rectangle,
}

impl PixelTool {
    pub const ALL: [Self; 7] = [
        Self::Pencil,
        Self::Eraser,
        Self::Fill,
        Self::Eyedropper,
        Self::Selection,
        Self::Line,
        Self::Rectangle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
            Self::Eyedropper => "Pick",
            Self::Selection => "Select",
            Self::Line => "Line",
            Self::Rectangle => "Rect",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelBlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Add,
    Erase,
}

impl PixelBlendMode {
    pub const ALL: [Self; 5] = [
        Self::Normal,
        Self::Multiply,
        Self::Screen,
        Self::Add,
        Self::Erase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Add => "Add",
            Self::Erase => "Erase",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelAssetKind {
    Tile,
    Tilesheet,
    SpriteSheet,
    AnimationSheet,
    UiTexture,
    ObjectSprite,
    CharacterLayer,
    #[default]
    General,
}

impl PixelAssetKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Tile",
            Self::Tilesheet => "Tilesheet",
            Self::SpriteSheet => "Sprite Sheet",
            Self::AnimationSheet => "Animation Sheet",
            Self::UiTexture => "UI Texture",
            Self::ObjectSprite => "Object Sprite",
            Self::CharacterLayer => "Character Layer",
            Self::General => "General",
        }
    }

    pub fn is_character(self) -> bool {
        matches!(
            self,
            Self::CharacterLayer | Self::SpriteSheet | Self::AnimationSheet
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelPreviewMode {
    Repeat,
    Character,
    Object,
    Ui,
    #[default]
    None,
}

impl PixelPreviewMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Repeat => "Repeat",
            Self::Character => "Character",
            Self::Object => "Object",
            Self::Ui => "UI",
            Self::None => "None",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelGrid {
    pub cell_width: u32,
    pub cell_height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    #[serde(default)]
    pub spacing_x: u32,
    #[serde(default)]
    pub spacing_y: u32,
    pub subgrid: u32,
}

impl Default for PixelGrid {
    fn default() -> Self {
        Self {
            cell_width: 32,
            cell_height: 32,
            offset_x: 0,
            offset_y: 0,
            spacing_x: 0,
            spacing_y: 0,
            subgrid: 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelSelection {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelSelection {
    pub fn from_points(ax: u32, ay: u32, bx: u32, by: u32) -> Self {
        let x = ax.min(bx);
        let y = ay.min(by);
        Self {
            x,
            y,
            width: ax.max(bx) - x + 1,
            height: ay.max(by) - y + 1,
        }
    }

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelLicense {
    pub status: String,
    pub source_name: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub notes: String,
}

impl Default for PixelLicense {
    fn default() -> Self {
        Self {
            status: "unverified".to_string(),
            source_name: String::new(),
            source_url: String::new(),
            notes: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelLayerMetadata {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_opacity")]
    pub opacity: u8,
    #[serde(default)]
    pub blend_mode: PixelBlendMode,
    #[serde(default)]
    pub image_path: String,
}

impl PixelLayerMetadata {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            visible: true,
            locked: false,
            opacity: u8::MAX,
            blend_mode: PixelBlendMode::Normal,
            image_path: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PixelLayer {
    pub metadata: PixelLayerMetadata,
    pub image: RgbaImage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelDocumentMetadata {
    pub schema: String,
    pub asset_id: String,
    pub display_name: String,
    pub source_path: String,
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub grid: PixelGrid,
    #[serde(default)]
    pub asset_kind: PixelAssetKind,
    #[serde(default)]
    pub preview_mode: PixelPreviewMode,
    #[serde(default)]
    pub palette: Vec<[u8; 4]>,
    pub selection: PixelSelection,
    /// Exact immutable upstream source region used to seed a derived Pixel Studio
    /// working copy. Coordinates are in the original source image, not the
    /// cropped working document. Older documents leave this unset.
    #[serde(default)]
    pub source_region: Option<PixelSelection>,
    pub pivot: [i32; 2],
    pub visual_footprint: [i32; 4],
    pub collision_footprint: [i32; 4],
    pub interaction_footprint: [i32; 4],
    #[serde(default)]
    pub tags: Vec<String>,
    pub license: PixelLicense,
    #[serde(default)]
    pub active_layer_id: String,
    #[serde(default)]
    pub layers: Vec<PixelLayerMetadata>,
}

impl PixelDocumentMetadata {
    pub fn sidecar_path(&self) -> String {
        replace_extension(&self.output_path, "hhasset.json")
    }

    pub fn package_directory(&self) -> String {
        replace_extension(&self.output_path, "hhpixel")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PixelDocumentSnapshot {
    metadata: PixelDocumentMetadata,
    layers: Vec<PixelLayer>,
}

#[derive(Clone)]
pub struct PixelDocument {
    pub metadata: PixelDocumentMetadata,
    pub(crate) layers: Vec<PixelLayer>,
    pub(crate) composite: RgbaImage,
    undo: VecDeque<PixelDocumentSnapshot>,
    redo: VecDeque<PixelDocumentSnapshot>,
    undo_limit: usize,
    pub dirty: bool,
    pub recovered_from_autosave: bool,
}

impl PixelDocument {
    pub(crate) fn from_loaded_parts(
        mut metadata: PixelDocumentMetadata,
        mut layers: Vec<PixelLayer>,
        dirty: bool,
        recovered_from_autosave: bool,
    ) -> Self {
        if layers.is_empty() {
            layers.push(PixelLayer {
                metadata: PixelLayerMetadata::new("layer_001", "Base"),
                image: RgbaImage::from_pixel(
                    metadata.width.max(1),
                    metadata.height.max(1),
                    Rgba([0, 0, 0, 0]),
                ),
            });
        }
        metadata.width = layers[0].image.width();
        metadata.height = layers[0].image.height();
        if metadata.active_layer_id.is_empty()
            || !layers
                .iter()
                .any(|layer| layer.metadata.id == metadata.active_layer_id)
        {
            metadata.active_layer_id = layers.last().unwrap().metadata.id.clone();
        }
        metadata.layers = layers.iter().map(|layer| layer.metadata.clone()).collect();
        let composite = composite_layers(&layers, metadata.width, metadata.height);
        Self {
            metadata,
            layers,
            composite,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            undo_limit: 64,
            dirty,
            recovered_from_autosave,
        }
    }

    pub fn from_rgba(width: u32, height: u32, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        let stem = slugify(&display_name);
        let metadata = PixelDocumentMetadata {
            schema: "havenwild.pixel_document.v0_3".to_string(),
            asset_id: format!("pixel/{stem}"),
            display_name,
            source_path: String::new(),
            output_path: format!("assets/source/original/pixel_studio/{stem}.png"),
            width: width.max(1),
            height: height.max(1),
            grid: PixelGrid::default(),
            asset_kind: PixelAssetKind::General,
            preview_mode: PixelPreviewMode::None,
            palette: Vec::new(),
            selection: PixelSelection {
                x: 0,
                y: 0,
                width: width.clamp(1, 32),
                height: height.clamp(1, 32),
            },
            source_region: None,
            pivot: [16, 28],
            visual_footprint: [0, 0, 1, 1],
            collision_footprint: [0, 0, 1, 1],
            interaction_footprint: [0, 0, 1, 1],
            tags: Vec::new(),
            license: PixelLicense {
                status: "project_owned".to_string(),
                source_name: "Havenwild Pixel Studio".to_string(),
                source_url: String::new(),
                notes: "Created in project".to_string(),
            },
            active_layer_id: "layer_001".to_string(),
            layers: Vec::new(),
        };
        Self::from_loaded_parts(
            metadata,
            vec![PixelLayer {
                metadata: PixelLayerMetadata::new("layer_001", "Base"),
                image: RgbaImage::from_pixel(width.max(1), height.max(1), Rgba([0, 0, 0, 0])),
            }],
            true,
            false,
        )
    }

    pub fn width(&self) -> u32 {
        self.composite.width()
    }

    pub fn height(&self) -> u32 {
        self.composite.height()
    }

    pub fn rgba_bytes(&self) -> &[u8] {
        self.composite.as_raw()
    }

    pub fn layers(&self) -> &[PixelLayer] {
        &self.layers
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn active_layer_index(&self) -> usize {
        self.layers
            .iter()
            .position(|layer| layer.metadata.id == self.metadata.active_layer_id)
            .unwrap_or_else(|| self.layers.len().saturating_sub(1))
    }

    pub fn active_layer(&self) -> &PixelLayer {
        &self.layers[self.active_layer_index()]
    }

    pub fn active_layer_mut(&mut self) -> &mut PixelLayer {
        let index = self.active_layer_index();
        &mut self.layers[index]
    }

    pub fn select_layer(&mut self, index: usize) -> bool {
        let Some(layer) = self.layers.get(index) else {
            return false;
        };
        self.metadata.active_layer_id = layer.metadata.id.clone();
        true
    }

    pub fn can_edit_active_layer(&self) -> bool {
        !self.active_layer().metadata.locked
    }

    pub fn begin_edit(&mut self) {
        let snapshot = self.snapshot();
        if self.undo.back().is_some_and(|last| last == &snapshot) {
            return;
        }
        self.undo.push_back(snapshot);
        while self.undo.len() > self.undo_limit {
            self.undo.pop_front();
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push_back(self.snapshot());
        self.restore(previous);
        self.dirty = true;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop_back() else {
            return false;
        };
        self.undo.push_back(self.snapshot());
        self.restore(next);
        self.dirty = true;
        true
    }

    pub fn color_at(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width() && y < self.height() {
            self.composite.get_pixel(x, y).0
        } else {
            [0, 0, 0, 0]
        }
    }

    pub fn active_color_at(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width() && y < self.height() {
            self.active_layer().image.get_pixel(x, y).0
        } else {
            [0, 0, 0, 0]
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) -> bool {
        if !self.can_edit_active_layer()
            || x >= self.width()
            || y >= self.height()
            || self.active_color_at(x, y) == color
        {
            return false;
        }
        self.active_layer_mut().image.put_pixel(x, y, Rgba(color));
        self.refresh_composite_pixel(x, y);
        self.dirty = true;
        true
    }

    pub fn flood_fill(&mut self, x: u32, y: u32, replacement: [u8; 4]) -> usize {
        if !self.can_edit_active_layer() || x >= self.width() || y >= self.height() {
            return 0;
        }
        let target = self.active_color_at(x, y);
        if target == replacement {
            return 0;
        }
        let width = self.width();
        let height = self.height();
        let mut queue = VecDeque::from([(x, y)]);
        let mut changed = 0usize;
        while let Some((px, py)) = queue.pop_front() {
            if self.active_color_at(px, py) != target {
                continue;
            }
            self.active_layer_mut()
                .image
                .put_pixel(px, py, Rgba(replacement));
            changed += 1;
            if px > 0 {
                queue.push_back((px - 1, py));
            }
            if py > 0 {
                queue.push_back((px, py - 1));
            }
            if px + 1 < width {
                queue.push_back((px + 1, py));
            }
            if py + 1 < height {
                queue.push_back((px, py + 1));
            }
        }
        if changed > 0 {
            self.refresh_composite();
            self.dirty = true;
        }
        changed
    }

    pub fn draw_line(&mut self, start: (u32, u32), end: (u32, u32), color: [u8; 4]) {
        let (mut x0, mut y0) = (start.0 as i32, start.1 as i32);
        let (x1, y1) = (end.0 as i32, end.1 as i32);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as u32, y0 as u32, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled = error * 2;
            if doubled >= dy {
                error += dy;
                x0 += sx;
            }
            if doubled <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_rectangle(&mut self, selection: PixelSelection, color: [u8; 4], filled: bool) {
        if selection.is_empty() || !self.can_edit_active_layer() {
            return;
        }
        let right = selection.x + selection.width - 1;
        let bottom = selection.y + selection.height - 1;
        for y in selection.y..=bottom.min(self.height().saturating_sub(1)) {
            for x in selection.x..=right.min(self.width().saturating_sub(1)) {
                if filled || x == selection.x || x == right || y == selection.y || y == bottom {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    pub fn refresh_composite(&mut self) {
        self.composite = composite_layers(&self.layers, self.metadata.width, self.metadata.height);
    }

    pub(crate) fn sync_layer_metadata(&mut self) {
        self.metadata.layers = self
            .layers
            .iter()
            .map(|layer| layer.metadata.clone())
            .collect();
    }

    pub(crate) fn set_clean(&mut self) {
        self.dirty = false;
        self.recovered_from_autosave = false;
    }

    fn snapshot(&self) -> PixelDocumentSnapshot {
        PixelDocumentSnapshot {
            metadata: self.metadata.clone(),
            layers: self.layers.clone(),
        }
    }

    fn restore(&mut self, snapshot: PixelDocumentSnapshot) {
        self.metadata = snapshot.metadata;
        self.layers = snapshot.layers;
        self.sync_layer_metadata();
        self.refresh_composite();
    }
}

pub(crate) fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut underscore = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            underscore = false;
        } else if !underscore && !output.is_empty() {
            output.push('_');
            underscore = true;
        }
    }
    let value = output.trim_matches('_');
    if value.is_empty() {
        "pixel_asset".to_string()
    } else {
        value.to_string()
    }
}

fn replace_extension(path: &str, extension: &str) -> String {
    let path = std::path::Path::new(path);
    path.with_extension(extension)
        .to_string_lossy()
        .replace('\\', "/")
}

fn default_true() -> bool {
    true
}

fn default_opacity() -> u8 {
    u8::MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_and_undo_are_deterministic() {
        let mut document = PixelDocument::from_rgba(4, 4, "test");
        document.begin_edit();
        assert_eq!(document.flood_fill(0, 0, [12, 34, 56, 255]), 16);
        assert_eq!(document.color_at(3, 3), [12, 34, 56, 255]);
        assert!(document.undo());
        assert_eq!(document.color_at(3, 3), [0, 0, 0, 0]);
        assert!(document.redo());
        assert_eq!(document.color_at(3, 3), [12, 34, 56, 255]);
    }

    #[test]
    fn layer_operations_are_part_of_document_history() {
        let mut document = PixelDocument::from_rgba(4, 4, "layers");
        document.add_layer("Shading");
        assert_eq!(document.layer_count(), 2);
        document.begin_edit();
        document.set_pixel(1, 1, [20, 40, 60, 255]);
        assert!(document.undo());
        assert_eq!(document.color_at(1, 1), [0, 0, 0, 0]);
        assert!(document.undo());
        assert_eq!(document.layer_count(), 1);
    }

    #[test]
    fn sidecar_and_package_paths_replace_png_extension() {
        let document = PixelDocument::from_rgba(32, 32, "test tile");
        assert_eq!(
            document.metadata.sidecar_path(),
            "assets/source/original/pixel_studio/test_tile.hhasset.json"
        );
        assert_eq!(
            document.metadata.package_directory(),
            "assets/source/original/pixel_studio/test_tile.hhpixel"
        );
    }

    #[test]
    fn selection_normalizes_drag_direction() {
        assert_eq!(
            PixelSelection::from_points(8, 9, 2, 3),
            PixelSelection {
                x: 2,
                y: 3,
                width: 7,
                height: 7,
            }
        );
    }
}
