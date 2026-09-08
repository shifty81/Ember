use crate::{PixelAssetKind, PixelDocument, PixelGrid, PixelPreviewMode, PixelSelection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelDocumentKind {
    Tile,
    Tilesheet,
    SpriteSheet,
    AnimationSheet,
    UiTexture,
    ObjectSprite,
    CharacterLayer,
    FreeCanvas,
}

impl PixelDocumentKind {
    pub const ALL: [Self; 8] = [
        Self::Tile,
        Self::Tilesheet,
        Self::SpriteSheet,
        Self::AnimationSheet,
        Self::UiTexture,
        Self::ObjectSprite,
        Self::CharacterLayer,
        Self::FreeCanvas,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Single Tile",
            Self::Tilesheet => "Tilesheet",
            Self::SpriteSheet => "Sprite Sheet",
            Self::AnimationSheet => "Animation Sheet",
            Self::UiTexture => "UI Texture",
            Self::ObjectSprite => "Object Sprite",
            Self::CharacterLayer => "Character Layer",
            Self::FreeCanvas => "Free Canvas",
        }
    }

    pub fn default_dimensions(self) -> (u32, u32, u32, u32) {
        match self {
            Self::Tile => (32, 32, 32, 32),
            Self::Tilesheet => (512, 512, 32, 32),
            Self::SpriteSheet => (512, 768, 64, 96),
            Self::AnimationSheet => (512, 768, 64, 96),
            Self::UiTexture => (512, 256, 16, 16),
            Self::ObjectSprite => (96, 96, 32, 32),
            Self::CharacterLayer => (512, 768, 64, 96),
            Self::FreeCanvas => (256, 256, 32, 32),
        }
    }

    pub fn asset_kind(self) -> PixelAssetKind {
        match self {
            Self::Tile => PixelAssetKind::Tile,
            Self::Tilesheet => PixelAssetKind::Tilesheet,
            Self::SpriteSheet => PixelAssetKind::SpriteSheet,
            Self::AnimationSheet => PixelAssetKind::AnimationSheet,
            Self::UiTexture => PixelAssetKind::UiTexture,
            Self::ObjectSprite => PixelAssetKind::ObjectSprite,
            Self::CharacterLayer => PixelAssetKind::CharacterLayer,
            Self::FreeCanvas => PixelAssetKind::General,
        }
    }

    pub fn preview_mode(self) -> PixelPreviewMode {
        match self {
            Self::Tile | Self::Tilesheet => PixelPreviewMode::Repeat,
            Self::SpriteSheet | Self::AnimationSheet | Self::CharacterLayer => {
                PixelPreviewMode::Character
            }
            Self::ObjectSprite => PixelPreviewMode::Object,
            Self::UiTexture => PixelPreviewMode::Ui,
            Self::FreeCanvas => PixelPreviewMode::None,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Self::Tile => "tile",
            Self::Tilesheet => "tilesheet",
            Self::SpriteSheet => "sprite_sheet",
            Self::AnimationSheet => "animation_sheet",
            Self::UiTexture => "ui_texture",
            Self::ObjectSprite => "object_sprite",
            Self::CharacterLayer => "character_layer",
            Self::FreeCanvas => "free_canvas",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewPixelDocumentSpec {
    pub kind: PixelDocumentKind,
    pub display_name: String,
    pub width: u32,
    pub height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub spacing: u32,
    pub padding: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl NewPixelDocumentSpec {
    pub fn for_kind(kind: PixelDocumentKind) -> Self {
        let (width, height, cell_width, cell_height) = kind.default_dimensions();
        Self {
            kind,
            display_name: format!("New Havenwild {}", kind.label()),
            width,
            height,
            cell_width,
            cell_height,
            spacing: 0,
            padding: 0,
            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.display_name.trim().is_empty() {
            return Err("asset name cannot be empty".to_string());
        }
        if self.width == 0 || self.height == 0 {
            return Err("canvas dimensions must be at least 1x1".to_string());
        }
        if self.width > 8_192 || self.height > 8_192 {
            return Err("canvas dimensions cannot exceed 8192 pixels".to_string());
        }
        if u64::from(self.width) * u64::from(self.height) > 16_777_216 {
            return Err("canvas exceeds the 16,777,216 pixel editing limit".to_string());
        }
        if self.cell_width == 0 || self.cell_height == 0 {
            return Err("cell dimensions must be at least 1x1".to_string());
        }
        if self.cell_width > self.width || self.cell_height > self.height {
            return Err("cell dimensions cannot exceed the canvas".to_string());
        }
        Ok(())
    }

    pub fn create_document(&self) -> Result<PixelDocument, String> {
        self.validate()?;
        let mut document = PixelDocument::from_rgba(
            self.width,
            self.height,
            self.display_name.trim().to_string(),
        );
        document.metadata.asset_kind = self.kind.asset_kind();
        document.metadata.preview_mode = self.kind.preview_mode();
        document.metadata.grid = PixelGrid {
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            offset_x: self.offset_x + self.padding as i32,
            offset_y: self.offset_y + self.padding as i32,
            spacing_x: self.spacing,
            spacing_y: self.spacing,
            subgrid: self.cell_width.min(self.cell_height).clamp(1, 8),
        };
        document.metadata.selection = PixelSelection {
            x: document.metadata.grid.offset_x.max(0) as u32,
            y: document.metadata.grid.offset_y.max(0) as u32,
            width: self.cell_width.min(self.width),
            height: self.cell_height.min(self.height),
        };
        document.metadata.tags = vec![
            self.kind.tag().to_string(),
            format!("grid_spacing:{}", self.spacing),
            format!("grid_padding:{}", self.padding),
        ];
        document.metadata.pivot = [
            (self.cell_width / 2) as i32,
            self.cell_height.saturating_sub(4) as i32,
        ];
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilesheet_spec_creates_metadata_backed_document() {
        let mut spec = NewPixelDocumentSpec::for_kind(PixelDocumentKind::Tilesheet);
        spec.display_name = "Terrain Draft".to_string();
        spec.width = 256;
        spec.height = 128;
        spec.cell_width = 32;
        spec.cell_height = 32;
        spec.padding = 2;
        spec.spacing = 1;
        let document = spec.create_document().unwrap();
        assert_eq!(document.width(), 256);
        assert_eq!(document.height(), 128);
        assert_eq!(document.metadata.grid.cell_width, 32);
        assert_eq!(document.metadata.grid.offset_x, 2);
        assert!(document.metadata.tags.contains(&"tilesheet".to_string()));
    }

    #[test]
    fn invalid_oversized_document_is_rejected() {
        let mut spec = NewPixelDocumentSpec::for_kind(PixelDocumentKind::FreeCanvas);
        spec.width = 8192;
        spec.height = 8192;
        assert!(spec.create_document().is_err());
    }
}
