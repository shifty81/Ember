use super::pixel_new_document::NewPixelDialogState;
use super::sprite_canvas_authority::*;
use super::*;
use haven_assets::asset_intake::{repo_root_dir, AssetIntakeTargetKind};
use haven_pixel::{
    record_recent_pixel_document, scan_pixel_library, AnimationSocket, PixelAssetKind,
    PixelDocument, PixelDocumentKind, PixelLibraryEntry, PixelPreviewMode, PixelSelection,
    PixelTool,
};
use std::path::Path;

const ZOOM_LEVELS: [f32; 18] = [
    0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 40.0, 48.0, 56.0,
    64.0, 72.0,
];
const MAX_PIXEL_EDIT_DIMENSION: u32 = 8_192;
const MAX_PIXEL_EDIT_PIXELS: u64 = 16_777_216;
pub(crate) const PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 0],
    [24, 27, 34, 255],
    [65, 49, 45, 255],
    [111, 78, 55, 255],
    [177, 117, 69, 255],
    [226, 172, 91, 255],
    [242, 222, 166, 255],
    [239, 239, 224, 255],
    [34, 74, 52, 255],
    [57, 111, 62, 255],
    [104, 157, 75, 255],
    [166, 195, 94, 255],
    [26, 72, 104, 255],
    [43, 117, 155, 255],
    [79, 171, 190, 255],
    [169, 220, 209, 255],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelInspectorTab {
    Layers,
    Asset,
    Animation,
    Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelSelectionMode {
    Pixels,
    Frame,
}

impl PixelSelectionMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Pixels => "Pixels",
            Self::Frame => "Frame",
        }
    }

    pub(crate) fn cycle(self) -> Self {
        match self {
            Self::Pixels => Self::Frame,
            Self::Frame => Self::Pixels,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelAssetRole {
    RepeatTexture,
    Character,
    Object,
    General,
}

impl PixelAssetRole {
    pub(crate) fn from_document(document: &PixelDocument) -> Self {
        match document.metadata.preview_mode {
            PixelPreviewMode::Repeat => Self::RepeatTexture,
            PixelPreviewMode::Character => Self::Character,
            PixelPreviewMode::Object => Self::Object,
            PixelPreviewMode::Ui | PixelPreviewMode::None => match document.metadata.asset_kind {
                PixelAssetKind::Tile | PixelAssetKind::Tilesheet => Self::RepeatTexture,
                PixelAssetKind::SpriteSheet
                | PixelAssetKind::AnimationSheet
                | PixelAssetKind::CharacterLayer => Self::Character,
                PixelAssetKind::ObjectSprite => Self::Object,
                PixelAssetKind::UiTexture | PixelAssetKind::General => Self::General,
            },
        }
    }

    pub(crate) fn supports_repeat_preview(self) -> bool {
        matches!(self, Self::RepeatTexture)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct WorldAssetEditContext {
    pub origin_scene_id: ProjectSceneId,
    pub origin_cell: [i32; 2],
    pub origin_camera: CanvasCameraState,
    pub semantic_id: String,
    pub generated_output: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PixelAnimationEditContext {
    pub animation_asset_id: String,
    pub animation_display_name: String,
    pub clip_index: usize,
    pub frame_index: usize,
    pub clip_label: String,
    pub direction_label: String,
    pub source_path_before_edit: String,
    pub frame_source: PixelSelection,
    pub previous_source: Option<PixelSelection>,
    pub next_source: Option<PixelSelection>,
    pub shadow_offset: [i32; 2],
    pub sockets: Vec<AnimationSocket>,
    pub onion_skin: bool,
}

pub(crate) struct PixelStudioState {
    pub library: Vec<PixelLibraryEntry>,
    pub library_loaded: bool,
    pub library_filter: String,
    pub selected_entry: usize,
    pub library_offset: usize,
    pub document: Option<PixelDocument>,
    pub texture: Option<Texture2D>,
    pub tool: PixelTool,
    pub selected_color: [u8; 4],
    pub background_color: [u8; 4],
    pub secondary_tool: PixelTool,
    pub brush_opacity: u8,
    pub zoom_index: usize,
    pub pan: Vec2,
    pub needs_frame: bool,
    pub show_pixel_grid: bool,
    pub show_atlas_grid: bool,
    pub show_repeat_preview: bool,
    pub selection_mode: PixelSelectionMode,
    pub grid_realign_armed: bool,
    pub grid_realign_enabled: bool,
    pub grid_drag_origin: Option<(i32, i32)>,
    pub drag_start: Option<(u32, u32)>,
    pub drag_current: Option<(u32, u32)>,
    pub stroke_started: bool,
    pub pan_drag: Option<Vec2>,
    pub target_kind: AssetIntakeTargetKind,
    pub target_index: usize,
    pub inspector_tab: PixelInspectorTab,
    pub layer_offset: usize,
    pub layer_rename_buffer: Option<String>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub next_autosave_at: f64,
    pub autosave_status: String,
    pub animation_context: Option<PixelAnimationEditContext>,
    pub world_asset_context: Option<WorldAssetEditContext>,
    pub new_dialog: Option<NewPixelDialogState>,
}

impl PixelStudioState {
    pub(crate) fn new() -> Self {
        Self {
            // The LPC source library can contain tens of thousands of sheets.
            // Do not recursively scan it while the editor process is starting.
            // The Pixel Studio library is populated only after the user presses
            // Rescan, using the bounded project-library policy in haven_pixel.
            library: Vec::new(),
            library_loaded: false,
            library_filter: String::new(),
            selected_entry: 0,
            library_offset: 0,
            document: None,
            texture: None,
            tool: PixelTool::Pencil,
            selected_color: PALETTE[7],
            background_color: PALETTE[1],
            secondary_tool: PixelTool::Eraser,
            brush_opacity: 255,
            zoom_index: 7,
            pan: Vec2::ZERO,
            needs_frame: true,
            show_pixel_grid: true,
            show_atlas_grid: true,
            show_repeat_preview: false,
            selection_mode: PixelSelectionMode::Frame,
            grid_realign_armed: false,
            grid_realign_enabled: false,
            grid_drag_origin: None,
            drag_start: None,
            drag_current: None,
            stroke_started: false,
            pan_drag: None,
            target_kind: AssetIntakeTargetKind::Object,
            target_index: 0,
            inspector_tab: PixelInspectorTab::Layers,
            layer_offset: 0,
            layer_rename_buffer: None,
            resize_width: 32,
            resize_height: 32,
            next_autosave_at: get_time() + 10.0,
            autosave_status: "Autosave ready".to_string(),
            animation_context: None,
            world_asset_context: None,
            new_dialog: None,
        }
    }

    pub(crate) fn filtered_library_indices(&self) -> Vec<usize> {
        let query = self.library_filter.trim().to_ascii_lowercase();
        self.library
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let matches = query.is_empty()
                    || entry.display_name.to_ascii_lowercase().contains(&query)
                    || entry.relative_path.to_ascii_lowercase().contains(&query)
                    || entry.source.label().to_ascii_lowercase().contains(&query)
                    || entry
                        .license
                        .source_name
                        .to_ascii_lowercase()
                        .contains(&query);
                matches.then_some(index)
            })
            .collect()
    }

    pub(crate) fn refresh_library(&mut self) -> Result<usize, String> {
        self.library = scan_pixel_library(repo_root_dir())?;
        self.library_loaded = true;
        self.selected_entry = self
            .selected_entry
            .min(self.library.len().saturating_sub(1));
        self.library_offset = self.library_offset.min(self.selected_entry);
        Ok(self.library.len())
    }

    pub(crate) fn load_selected(&mut self) -> Result<String, String> {
        let entry = self
            .library
            .get(self.selected_entry)
            .ok_or_else(|| "Pixel library is empty".to_string())?
            .clone();
        validate_editable_image(&entry.path)?;

        let document = PixelDocument::load(&entry.path, &entry.display_name, entry.license)?;
        let texture = texture_from_document(&document)?;

        if let Some(current) = self.document.as_mut() {
            if current.dirty {
                let _ = current.autosave(repo_root_dir());
            }
        }
        let _ = record_recent_pixel_document(
            repo_root_dir(),
            &entry.relative_path,
            &entry.display_name,
        );
        self.resize_width = document.width();
        self.resize_height = document.height();
        self.layer_offset = 0;
        self.layer_rename_buffer = None;
        let recovered = document.recovered_from_autosave;
        self.document = Some(document);
        self.texture = Some(texture);
        self.animation_context = None;
        self.world_asset_context = None;
        if let Some(document) = self.document.as_mut() {
            if document.metadata.palette.is_empty() {
                document.metadata.palette = PALETTE.to_vec();
            }
            let role = PixelAssetRole::from_document(document);
            self.show_repeat_preview = role.supports_repeat_preview();
            self.selection_mode = if matches!(
                role,
                PixelAssetRole::Character | PixelAssetRole::RepeatTexture
            ) {
                PixelSelectionMode::Frame
            } else {
                PixelSelectionMode::Pixels
            };
            if matches!(role, PixelAssetRole::Character) {
                let grid = document.metadata.grid;
                let selection = document.metadata.selection;
                if selection.width != grid.cell_width || selection.height != grid.cell_height {
                    document.metadata.selection = PixelSelection {
                        x: grid.offset_x.max(0) as u32,
                        y: grid.offset_y.max(0) as u32,
                        width: grid.cell_width.min(document.width()).max(1),
                        height: grid.cell_height.min(document.height()).max(1),
                    };
                }
            }
        }
        self.needs_frame = true;
        self.next_autosave_at = get_time() + 10.0;
        self.autosave_status = if recovered {
            "Recovered autosaved changes; Save to keep them".to_string()
        } else {
            "Autosave ready".to_string()
        };
        Ok(format!(
            "Opened {} from {}{}",
            entry.display_name,
            entry.source.label(),
            if recovered {
                " (recovered autosave)"
            } else {
                ""
            }
        ))
    }

    pub(crate) fn open_new_dialog(&mut self) {
        self.new_dialog = Some(NewPixelDialogState::new());
    }

    pub(crate) fn create_blank(&mut self) -> String {
        self.new_dialog = Some(NewPixelDialogState::new());
        "Choose the new Pixel Studio asset type and dimensions".to_string()
    }

    pub(crate) fn create_from_dialog(&mut self) -> Result<String, String> {
        let dialog = self
            .new_dialog
            .as_ref()
            .ok_or_else(|| "new-document dialog is not open".to_string())?;
        let document = dialog.spec.create_document()?;
        let label = dialog.spec.kind.label().to_string();
        let width = document.width();
        let height = document.height();
        self.document = Some(document);
        if let Some(document) = self.document.as_mut() {
            if document.metadata.palette.is_empty() {
                document.metadata.palette = PALETTE.to_vec();
            }
        }
        self.animation_context = None;
        self.world_asset_context = None;
        self.resize_width = width;
        self.resize_height = height;
        self.layer_offset = 0;
        self.layer_rename_buffer = None;
        self.autosave_status = "New document; autosave pending".to_string();
        self.next_autosave_at = get_time() + 10.0;
        self.refresh_texture();
        self.needs_frame = true;
        self.new_dialog = None;
        Ok(format!("Created {label} at {width}x{height}"))
    }

    pub(crate) fn cycle_new_kind(&mut self, direction: i32) {
        let Some(dialog) = self.new_dialog.as_mut() else {
            return;
        };
        let current = PixelDocumentKind::ALL
            .iter()
            .position(|kind| *kind == dialog.spec.kind)
            .unwrap_or(0);
        let len = PixelDocumentKind::ALL.len() as i32;
        let next = (current as i32 + direction).rem_euclid(len) as usize;
        dialog.select_kind(PixelDocumentKind::ALL[next]);
    }

    pub(crate) fn refresh_texture(&mut self) {
        let Some(document) = &self.document else {
            self.texture = None;
            return;
        };
        let pixel_count = u64::from(document.width()) * u64::from(document.height());
        if pixel_count > 4_194_304 {
            self.autosave_status = format!(
                "Live preview deferred for {}x{} document; save or frame a smaller asset",
                document.width(),
                document.height()
            );
            return;
        }
        match texture_from_document(document) {
            Ok(texture) => self.texture = Some(texture),
            Err(error) => {
                self.texture = None;
                self.autosave_status = format!("Pixel preview unavailable: {error}");
            }
        }
    }

    pub(crate) fn zoom(&self) -> f32 {
        ZOOM_LEVELS[self.zoom_index]
    }

    pub(crate) fn zoom_in(&mut self) {
        self.zoom_index = (self.zoom_index + 1).min(ZOOM_LEVELS.len() - 1);
    }

    pub(crate) fn zoom_out(&mut self) {
        self.zoom_index = self.zoom_index.saturating_sub(1);
    }

    pub(crate) fn frame_document(&mut self, canvas: Rect) {
        let Some(document) = &self.document else {
            return;
        };
        let fit = (canvas.w / document.width().max(1) as f32)
            .min(canvas.h / document.height().max(1) as f32)
            .clamp(ZOOM_LEVELS[0], ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);
        self.zoom_index = ZOOM_LEVELS
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                (*left - fit)
                    .abs()
                    .partial_cmp(&(*right - fit).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or(3);
        self.pan = Vec2::ZERO;
        self.needs_frame = false;
    }

    pub(crate) fn frame_selection(&mut self, canvas: Rect) {
        let Some(document) = &self.document else {
            return;
        };
        let selection = document.metadata.selection;
        if selection.is_empty() {
            self.frame_document(canvas);
            return;
        }
        let fit = ((canvas.w * 0.72) / selection.width.max(1) as f32)
            .min((canvas.h * 0.72) / selection.height.max(1) as f32)
            .clamp(ZOOM_LEVELS[0], ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);
        self.zoom_index = ZOOM_LEVELS
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                (*left - fit)
                    .abs()
                    .partial_cmp(&(*right - fit).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index)
            .unwrap_or(3);
        let zoom = self.zoom();
        let base_x = canvas.x + (canvas.w - document.width() as f32 * zoom) * 0.5;
        let base_y = canvas.y + (canvas.h - document.height() as f32 * zoom) * 0.5;
        let selection_center_x = (selection.x as f32 + selection.width as f32 * 0.5) * zoom;
        let selection_center_y = (selection.y as f32 + selection.height as f32 * 0.5) * zoom;
        self.pan = vec2(
            canvas.x + canvas.w * 0.5 - (base_x + selection_center_x),
            canvas.y + canvas.h * 0.5 - (base_y + selection_center_y),
        );
        self.needs_frame = false;
    }

    pub(crate) fn canvas_transform(&self, canvas: Rect) -> Option<SpriteCanvasTransform> {
        let document = self.document.as_ref()?;
        Some(SpriteCanvasTransform::new(
            canvas,
            document.width(),
            document.height(),
            self.zoom(),
            self.pan,
        ))
    }

    pub(crate) fn image_rect(&self, canvas: Rect) -> Option<Rect> {
        self.canvas_transform(canvas)
            .map(|transform| transform.image)
    }

    pub(crate) fn screen_to_pixel(&self, canvas: Rect, point: Vec2) -> Option<(u32, u32)> {
        self.canvas_transform(canvas)?.screen_to_pixel(point)
    }

    pub(crate) fn update_autosave(&mut self) -> Option<Result<String, String>> {
        if get_time() < self.next_autosave_at {
            return None;
        }
        self.next_autosave_at = get_time() + 10.0;
        let result = {
            let document = self.document.as_mut()?;
            if !document.dirty {
                return None;
            }
            let pixel_count = u64::from(document.width()) * u64::from(document.height());
            if pixel_count > 4_194_304 {
                self.autosave_status =
                    "Autosave deferred for large document; use Ctrl+S".to_string();
                return None;
            }
            document.autosave(repo_root_dir())
        };
        Some(match result {
            Ok(path) => {
                let message = format!("Autosaved recovery to {}", path.display());
                self.autosave_status = message.clone();
                Ok(message)
            }
            Err(error) => Err(error),
        })
    }

    pub(crate) fn target_code(&self) -> &'static str {
        match self.target_kind {
            AssetIntakeTargetKind::Tile => {
                TileKind::ALL[self.target_index % TileKind::ALL.len()].code()
            }
            AssetIntakeTargetKind::Object => {
                OBJECT_BRUSHES[self.target_index % OBJECT_BRUSHES.len()].code()
            }
        }
    }

    pub(crate) fn target_label(&self) -> &'static str {
        match self.target_kind {
            AssetIntakeTargetKind::Tile => {
                TileKind::ALL[self.target_index % TileKind::ALL.len()].label()
            }
            AssetIntakeTargetKind::Object => {
                OBJECT_BRUSHES[self.target_index % OBJECT_BRUSHES.len()].label()
            }
        }
    }
}

fn validate_editable_image(path: &Path) -> Result<(), String> {
    let (width, height) = image::image_dimensions(path)
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    validate_editable_dimensions(width, height)
}

fn validate_editable_dimensions(width: u32, height: u32) -> Result<(), String> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 {
        return Err("image has zero width or height".to_string());
    }
    if width > MAX_PIXEL_EDIT_DIMENSION || height > MAX_PIXEL_EDIT_DIMENSION {
        return Err(format!(
            "image is {width}x{height}; Pixel Studio currently supports up to {MAX_PIXEL_EDIT_DIMENSION}px per side"
        ));
    }
    if pixels > MAX_PIXEL_EDIT_PIXELS {
        return Err(format!(
            "image is {width}x{height} ({pixels} pixels); create a cropped working copy before editing"
        ));
    }
    Ok(())
}

fn texture_from_document(document: &PixelDocument) -> Result<Texture2D, String> {
    validate_editable_dimensions(document.width(), document.height())?;
    let width = u16::try_from(document.width())
        .map_err(|_| "pixel document width exceeds texture limits".to_string())?;
    let height = u16::try_from(document.height())
        .map_err(|_| "pixel document height exceeds texture limits".to_string())?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Texture2D::from_rgba8(width, height, document.rgba_bytes())
    }));
    let texture = result.map_err(|_| {
        format!(
            "GPU rejected the {}x{} preview texture; the document was not opened",
            document.width(),
            document.height()
        )
    })?;
    texture.set_filter(FilterMode::Nearest);
    Ok(texture)
}
