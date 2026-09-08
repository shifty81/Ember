use crate::layers::{composite_pixel, flatten_pair};
use crate::{PixelBlendMode, PixelDocument, PixelLayer, PixelLayerMetadata, PixelSelection};
use image::{imageops, Rgba, RgbaImage};

impl PixelDocument {
    pub fn flip_selection_horizontal(&mut self) {
        self.flip_selection(true);
    }

    pub fn flip_selection_vertical(&mut self) {
        self.flip_selection(false);
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> usize {
        self.begin_edit();
        let id = self.next_layer_id();
        let width = self.width();
        let height = self.height();
        self.layers.push(PixelLayer {
            metadata: PixelLayerMetadata::new(&id, name),
            image: RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0])),
        });
        self.metadata.active_layer_id = id;
        self.sync_layer_metadata();
        self.dirty = true;
        self.layers.len() - 1
    }

    pub fn duplicate_active_layer(&mut self) -> usize {
        self.begin_edit();
        let source = self.active_layer().clone();
        let id = self.next_layer_id();
        let mut metadata = source.metadata;
        metadata.id = id.clone();
        metadata.name = format!("{} Copy", metadata.name);
        metadata.image_path.clear();
        let index = self.active_layer_index() + 1;
        self.layers.insert(
            index,
            PixelLayer {
                metadata,
                image: source.image,
            },
        );
        self.metadata.active_layer_id = id;
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
        index
    }

    pub fn delete_active_layer(&mut self) -> bool {
        if self.layers.len() <= 1 {
            return false;
        }
        self.begin_edit();
        let index = self.active_layer_index();
        self.layers.remove(index);
        let next = index.saturating_sub(1).min(self.layers.len() - 1);
        self.metadata.active_layer_id = self.layers[next].metadata.id.clone();
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
        true
    }

    pub fn move_active_layer(&mut self, delta: i32) -> bool {
        let index = self.active_layer_index();
        let target =
            (index as i32 + delta).clamp(0, self.layers.len().saturating_sub(1) as i32) as usize;
        if target == index {
            return false;
        }
        self.begin_edit();
        self.layers.swap(index, target);
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
        true
    }

    pub fn merge_active_down(&mut self) -> bool {
        let index = self.active_layer_index();
        if index == 0 {
            return false;
        }
        self.begin_edit();
        let top = self.layers.remove(index);
        let bottom = self.layers.remove(index - 1);
        let image = flatten_pair(&bottom, &top);
        let mut metadata = bottom.metadata;
        metadata.name = format!("{} + {}", metadata.name, top.metadata.name);
        metadata.opacity = u8::MAX;
        metadata.blend_mode = PixelBlendMode::Normal;
        metadata.visible = true;
        metadata.locked = false;
        metadata.image_path.clear();
        let id = metadata.id.clone();
        self.layers
            .insert(index - 1, PixelLayer { metadata, image });
        self.metadata.active_layer_id = id;
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
        true
    }

    pub fn rename_active_layer(&mut self, name: impl Into<String>) -> bool {
        let name = name.into().trim().to_string();
        if name.is_empty() || self.active_layer().metadata.name == name {
            return false;
        }
        self.begin_edit();
        self.active_layer_mut().metadata.name = name;
        self.sync_layer_metadata();
        self.dirty = true;
        true
    }

    pub fn toggle_active_layer_visibility(&mut self) {
        self.begin_edit();
        let layer = self.active_layer_mut();
        layer.metadata.visible = !layer.metadata.visible;
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
    }

    pub fn toggle_active_layer_lock(&mut self) {
        self.begin_edit();
        let layer = self.active_layer_mut();
        layer.metadata.locked = !layer.metadata.locked;
        self.sync_layer_metadata();
        self.dirty = true;
    }

    pub fn adjust_active_layer_opacity(&mut self, delta: i16) {
        self.begin_edit();
        let layer = self.active_layer_mut();
        layer.metadata.opacity = (layer.metadata.opacity as i16 + delta).clamp(0, 255) as u8;
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
    }

    pub fn cycle_active_layer_blend_mode(&mut self) {
        self.begin_edit();
        let layer = self.active_layer_mut();
        layer.metadata.blend_mode = layer.metadata.blend_mode.next();
        self.sync_layer_metadata();
        self.refresh_composite();
        self.dirty = true;
    }

    pub fn resize_canvas(&mut self, width: u32, height: u32) -> bool {
        let width = width.clamp(1, 8192);
        let height = height.clamp(1, 8192);
        if width == self.width() && height == self.height() {
            return false;
        }
        self.begin_edit();
        for layer in &mut self.layers {
            let mut resized = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
            imageops::replace(&mut resized, &layer.image, 0, 0);
            layer.image = resized;
        }
        self.metadata.width = width;
        self.metadata.height = height;
        self.clamp_document_metadata();
        self.refresh_composite();
        self.dirty = true;
        true
    }

    pub fn crop_to_selection(&mut self) -> bool {
        let selection = self.clamped_selection();
        if selection.is_empty()
            || (selection.x == 0
                && selection.y == 0
                && selection.width == self.width()
                && selection.height == self.height())
        {
            return false;
        }
        self.begin_edit();
        self.apply_crop(selection);
        true
    }

    pub fn trim_transparent_padding(&mut self) -> bool {
        let mut min_x = self.width();
        let mut min_y = self.height();
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;
        for (x, y, pixel) in self.composite.enumerate_pixels() {
            if pixel[3] == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        if !found {
            return false;
        }
        let selection = PixelSelection {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        };
        if selection.x == 0
            && selection.y == 0
            && selection.width == self.width()
            && selection.height == self.height()
        {
            return false;
        }
        self.begin_edit();
        self.apply_crop(selection);
        true
    }

    fn apply_crop(&mut self, selection: PixelSelection) {
        for layer in &mut self.layers {
            layer.image = imageops::crop_imm(
                &layer.image,
                selection.x,
                selection.y,
                selection.width,
                selection.height,
            )
            .to_image();
        }
        self.metadata.width = selection.width;
        self.metadata.height = selection.height;
        self.metadata.selection = PixelSelection {
            x: 0,
            y: 0,
            width: selection.width,
            height: selection.height,
        };
        self.metadata.pivot[0] -= selection.x as i32;
        self.metadata.pivot[1] -= selection.y as i32;
        self.metadata.grid.offset_x -= selection.x as i32;
        self.metadata.grid.offset_y -= selection.y as i32;
        self.clamp_document_metadata();
        self.refresh_composite();
        self.dirty = true;
    }

    fn flip_selection(&mut self, horizontal: bool) {
        let selection = self.clamped_selection();
        if selection.is_empty() || !self.can_edit_active_layer() {
            return;
        }
        self.begin_edit();
        let crop = imageops::crop_imm(
            &self.active_layer().image,
            selection.x,
            selection.y,
            selection.width,
            selection.height,
        )
        .to_image();
        let flipped = if horizontal {
            imageops::flip_horizontal(&crop)
        } else {
            imageops::flip_vertical(&crop)
        };
        imageops::replace(
            &mut self.active_layer_mut().image,
            &flipped,
            selection.x as i64,
            selection.y as i64,
        );
        self.refresh_composite();
        self.dirty = true;
    }

    pub(crate) fn refresh_composite_pixel(&mut self, x: u32, y: u32) {
        let pixel = composite_pixel(&self.layers, x, y);
        self.composite.put_pixel(x, y, Rgba(pixel));
    }

    fn clamped_selection(&self) -> PixelSelection {
        let mut selection = self.metadata.selection;
        selection.x = selection.x.min(self.width().saturating_sub(1));
        selection.y = selection.y.min(self.height().saturating_sub(1));
        selection.width = selection
            .width
            .min(self.width().saturating_sub(selection.x));
        selection.height = selection
            .height
            .min(self.height().saturating_sub(selection.y));
        selection
    }

    fn clamp_document_metadata(&mut self) {
        let width = self.width().max(1) as i32;
        let height = self.height().max(1) as i32;
        self.metadata.selection = self.clamped_selection();
        self.metadata.pivot[0] = self.metadata.pivot[0].clamp(0, width - 1);
        self.metadata.pivot[1] = self.metadata.pivot[1].clamp(0, height - 1);
    }

    fn next_layer_id(&self) -> String {
        let mut number = self.layers.len() + 1;
        loop {
            let id = format!("layer_{number:03}");
            if !self.layers.iter().any(|layer| layer.metadata.id == id) {
                return id;
            }
            number += 1;
        }
    }
}
