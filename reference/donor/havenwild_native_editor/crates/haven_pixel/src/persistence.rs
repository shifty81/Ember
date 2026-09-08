use crate::document::slugify;
use crate::{
    PixelAssetKind, PixelDocument, PixelDocumentMetadata, PixelLayer, PixelLayerMetadata,
    PixelLicense, PixelPreviewMode, PixelSelection,
};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DOCUMENT_SCHEMA_V1: &str = "havenwild.pixel_document.v0_1";
const DOCUMENT_SCHEMA_V2: &str = "havenwild.pixel_document.v0_2";
const DOCUMENT_SCHEMA_V3: &str = "havenwild.pixel_document.v0_3";
const RECOVERY_SCHEMA: &str = "havenwild.pixel_recovery.v0_1";
const RECOVERY_ROOT: &str = "WORKSPACE/recovery/pixel_studio";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PixelRecoveryManifest {
    schema: String,
    saved_unix_ms: u64,
    metadata: PixelDocumentMetadata,
    layers: Vec<PixelLayerMetadata>,
}

impl PixelDocument {
    pub fn load(
        source_path: impl AsRef<Path>,
        display_name: impl Into<String>,
        license: PixelLicense,
    ) -> Result<Self, String> {
        let source_path = source_path.as_ref();
        let flattened = image::open(source_path)
            .map_err(|error| format!("failed to load {}: {error}", source_path.display()))?
            .to_rgba8();
        let display_name = display_name.into();
        let stem = slugify(&display_name);
        let fallback = default_metadata(source_path, display_name, license, &flattened, &stem);
        let sidecar = source_path.with_extension("hhasset.json");
        let mut metadata = if sidecar.is_file() {
            load_metadata(&sidecar, fallback)?
        } else {
            fallback
        };
        metadata.width = flattened.width();
        metadata.height = flattened.height();
        clamp_selection(&mut metadata.selection, metadata.width, metadata.height);

        let mut layers = if (metadata.schema == DOCUMENT_SCHEMA_V2
            || metadata.schema == DOCUMENT_SCHEMA_V3)
            && !metadata.layers.is_empty()
        {
            load_layers_from_metadata(
                &metadata.layers,
                sidecar.parent().unwrap_or(Path::new(".")),
                &flattened,
            )?
        } else {
            vec![PixelLayer {
                metadata: PixelLayerMetadata::new("layer_001", "Base"),
                image: flattened,
            }]
        };
        normalize_layer_dimensions(&mut layers, metadata.width, metadata.height);
        metadata.schema = DOCUMENT_SCHEMA_V3.to_string();

        let mut document = PixelDocument::from_loaded_parts(metadata, layers, false, false);
        if let Some(repo_root) = find_repo_root(source_path) {
            if let Some(recovered) = load_recovery(&repo_root, &document.metadata.asset_id)? {
                document = recovered;
            }
        }
        Ok(document)
    }

    /// Open one exact upstream source region as a derived, editable Pixel Studio
    /// document. The upstream sheet remains immutable: Save writes to the normal
    /// Havenwild Pixel Studio output path, while source_path/source_region retain
    /// provenance for reset/rebase.
    pub fn load_source_region(
        source_path: impl AsRef<Path>,
        source_region: PixelSelection,
        display_name: impl Into<String>,
        license: PixelLicense,
    ) -> Result<Self, String> {
        let source_path = source_path.as_ref();
        let display_name = display_name.into();
        if source_region.is_empty() {
            return Err("source region is empty".to_string());
        }
        let upstream = image::open(source_path)
            .map_err(|error| format!("failed to load {}: {error}", source_path.display()))?
            .to_rgba8();
        let right = source_region.x.saturating_add(source_region.width);
        let bottom = source_region.y.saturating_add(source_region.height);
        if right > upstream.width() || bottom > upstream.height() {
            return Err(format!(
                "source region {},{} {}x{} exceeds {}x{} image {}",
                source_region.x,
                source_region.y,
                source_region.width,
                source_region.height,
                upstream.width(),
                upstream.height(),
                source_path.display()
            ));
        }
        let stem = slugify(&display_name);
        let mut metadata = default_metadata(
            source_path,
            display_name.clone(),
            license.clone(),
            &upstream,
            &stem,
        );
        let repo_root = find_repo_root(source_path);
        if let Some(root) = repo_root.as_ref() {
            if let Ok(relative) = source_path.strip_prefix(root) {
                metadata.source_path = relative.to_string_lossy().replace('\\', "/");
            }
        }
        let derived_path = repo_root
            .as_ref()
            .map(|root| root.join(&metadata.output_path));

        // Prefer an existing derived working copy. Its sidecar preserves the
        // original source_path/source_region, so reopening never discards edits.
        if let Some(path) = derived_path.as_ref().filter(|path| path.is_file()) {
            let mut document = Self::load(path, display_name, license)?;
            if document.metadata.source_region.is_none() {
                document.metadata.source_path = source_path.to_string_lossy().replace('\\', "/");
                document.metadata.source_region = Some(source_region);
            }
            document.metadata.selection = PixelSelection {
                x: 0,
                y: 0,
                width: document.width(),
                height: document.height(),
            };
            return Ok(document);
        }

        let cropped = image::imageops::crop_imm(
            &upstream,
            source_region.x,
            source_region.y,
            source_region.width,
            source_region.height,
        )
        .to_image();
        metadata.width = cropped.width();
        metadata.height = cropped.height();
        metadata.selection = PixelSelection {
            x: 0,
            y: 0,
            width: cropped.width(),
            height: cropped.height(),
        };
        metadata.source_region = Some(source_region);
        metadata.pivot = [
            (cropped.width() / 2) as i32,
            cropped.height().saturating_sub(1) as i32,
        ];
        if !metadata.tags.iter().any(|tag| tag == "derived_source_region") {
            metadata.tags.push("derived_source_region".to_string());
        }
        Ok(PixelDocument::from_loaded_parts(
            metadata,
            vec![PixelLayer {
                metadata: PixelLayerMetadata::new("layer_001", "Base"),
                image: cropped,
            }],
            false,
            false,
        ))
    }

    /// Reset a derived exact-region document back to its immutable upstream
    /// source rectangle without changing the configured Havenwild output path.
    pub fn reset_to_source_region(&mut self, repo_root: impl AsRef<Path>) -> Result<(), String> {
        let Some(region) = self.metadata.source_region else {
            return Err("document has no upstream source region".to_string());
        };
        let source_path = PathBuf::from(&self.metadata.source_path);
        let resolved_source = if source_path.is_absolute() {
            source_path
        } else {
            repo_root.as_ref().join(source_path)
        };
        let upstream = image::open(&resolved_source)
            .map_err(|error| format!("failed to load {}: {error}", resolved_source.display()))?
            .to_rgba8();
        let right = region.x.saturating_add(region.width);
        let bottom = region.y.saturating_add(region.height);
        if right > upstream.width() || bottom > upstream.height() {
            return Err("saved upstream source region no longer fits source image".to_string());
        }
        let cropped = image::imageops::crop_imm(
            &upstream,
            region.x,
            region.y,
            region.width,
            region.height,
        )
        .to_image();
        let mut metadata = self.metadata.clone();
        metadata.width = cropped.width();
        metadata.height = cropped.height();
        metadata.selection = PixelSelection {
            x: 0,
            y: 0,
            width: cropped.width(),
            height: cropped.height(),
        };
        *self = PixelDocument::from_loaded_parts(
            metadata,
            vec![PixelLayer {
                metadata: PixelLayerMetadata::new("layer_001", "Base"),
                image: cropped,
            }],
            true,
            false,
        );
        Ok(())
    }

    pub fn save(&mut self, repo_root: impl AsRef<Path>) -> Result<(), String> {
        let repo_root = repo_root.as_ref();
        self.metadata.schema = DOCUMENT_SCHEMA_V3.to_string();
        self.metadata.width = self.width();
        self.metadata.height = self.height();

        let output = repo_root.join(&self.metadata.output_path);
        let output_parent = output
            .parent()
            .ok_or_else(|| format!("invalid pixel output path {}", output.display()))?;
        fs::create_dir_all(output_parent)
            .map_err(|error| format!("failed to create {}: {error}", output_parent.display()))?;

        let package = repo_root.join(self.metadata.package_directory());
        if package.exists() {
            fs::remove_dir_all(&package)
                .map_err(|error| format!("failed to replace {}: {error}", package.display()))?;
        }
        let layer_directory = package.join("layers");
        fs::create_dir_all(&layer_directory)
            .map_err(|error| format!("failed to create {}: {error}", layer_directory.display()))?;

        let package_name = package
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid pixel package path {}", package.display()))?;
        for layer in &mut self.layers {
            let filename = format!("{}.png", slugify(&layer.metadata.id));
            let layer_path = layer_directory.join(&filename);
            layer
                .image
                .save(&layer_path)
                .map_err(|error| format!("failed to save {}: {error}", layer_path.display()))?;
            layer.metadata.image_path = format!("{package_name}/layers/{filename}");
        }
        self.sync_layer_metadata();
        self.composite
            .save(&output)
            .map_err(|error| format!("failed to save {}: {error}", output.display()))?;

        let sidecar = repo_root.join(self.metadata.sidecar_path());
        let raw = serde_json::to_string_pretty(&self.metadata)
            .map_err(|error| format!("failed to serialize pixel metadata: {error}"))?;
        fs::write(&sidecar, format!("{raw}\n"))
            .map_err(|error| format!("failed to save {}: {error}", sidecar.display()))?;
        self.discard_autosave(repo_root)?;
        self.set_clean();
        Ok(())
    }

    pub fn autosave(&mut self, repo_root: impl AsRef<Path>) -> Result<PathBuf, String> {
        let repo_root = repo_root.as_ref();
        self.metadata.schema = DOCUMENT_SCHEMA_V3.to_string();
        self.metadata.width = self.width();
        self.metadata.height = self.height();
        self.sync_layer_metadata();

        let recovery = recovery_directory(repo_root, &self.metadata.asset_id);
        if recovery.exists() {
            fs::remove_dir_all(&recovery)
                .map_err(|error| format!("failed to replace {}: {error}", recovery.display()))?;
        }
        let layer_directory = recovery.join("layers");
        fs::create_dir_all(&layer_directory)
            .map_err(|error| format!("failed to create {}: {error}", layer_directory.display()))?;

        let mut layer_metadata = Vec::with_capacity(self.layers.len());
        for layer in &self.layers {
            let filename = format!("{}.png", slugify(&layer.metadata.id));
            let layer_path = layer_directory.join(&filename);
            layer
                .image
                .save(&layer_path)
                .map_err(|error| format!("failed to autosave {}: {error}", layer_path.display()))?;
            let mut metadata = layer.metadata.clone();
            metadata.image_path = format!("layers/{filename}");
            layer_metadata.push(metadata);
        }
        self.composite
            .save(recovery.join("composite.png"))
            .map_err(|error| format!("failed to autosave composite: {error}"))?;
        let manifest = PixelRecoveryManifest {
            schema: RECOVERY_SCHEMA.to_string(),
            saved_unix_ms: unix_millis(),
            metadata: self.metadata.clone(),
            layers: layer_metadata,
        };
        let raw = serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("failed to serialize pixel recovery: {error}"))?;
        let manifest_path = recovery.join("document.json");
        fs::write(&manifest_path, format!("{raw}\n"))
            .map_err(|error| format!("failed to save {}: {error}", manifest_path.display()))?;
        Ok(manifest_path)
    }

    pub fn discard_autosave(&self, repo_root: impl AsRef<Path>) -> Result<(), String> {
        let recovery = recovery_directory(repo_root.as_ref(), &self.metadata.asset_id);
        if recovery.exists() {
            fs::remove_dir_all(&recovery)
                .map_err(|error| format!("failed to remove {}: {error}", recovery.display()))?;
        }
        Ok(())
    }

    pub fn recovery_path(&self, repo_root: impl AsRef<Path>) -> PathBuf {
        recovery_directory(repo_root.as_ref(), &self.metadata.asset_id).join("document.json")
    }
}

fn default_metadata(
    source_path: &Path,
    display_name: String,
    license: PixelLicense,
    image: &RgbaImage,
    stem: &str,
) -> PixelDocumentMetadata {
    PixelDocumentMetadata {
        schema: DOCUMENT_SCHEMA_V3.to_string(),
        asset_id: format!("pixel/{stem}"),
        display_name,
        source_path: source_path.to_string_lossy().replace('\\', "/"),
        output_path: format!("assets/source/original/pixel_studio/{stem}.png"),
        width: image.width(),
        height: image.height(),
        grid: infer_grid(source_path, image),
        asset_kind: infer_asset_kind(source_path),
        preview_mode: infer_preview_mode(source_path),
        palette: Vec::new(),
        selection: PixelSelection {
            x: 0,
            y: 0,
            width: image.width().min(32),
            height: image.height().min(32),
        },
        source_region: None,
        pivot: [16, 28],
        visual_footprint: [0, 0, 1, 1],
        collision_footprint: [0, 0, 1, 1],
        interaction_footprint: [0, 0, 1, 1],
        tags: Vec::new(),
        license,
        active_layer_id: "layer_001".to_string(),
        layers: Vec::new(),
    }
}

fn infer_asset_kind(source_path: &Path) -> PixelAssetKind {
    let normalized = source_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if normalized.contains("terrain")
        || normalized.contains("tile")
        || normalized.contains("floor")
        || normalized.contains("wall")
    {
        PixelAssetKind::Tilesheet
    } else if normalized.contains("animation")
        || normalized.contains("walk")
        || normalized.contains("idle")
    {
        PixelAssetKind::AnimationSheet
    } else if normalized.contains("character")
        || normalized.contains("player_")
        || normalized.contains("body_")
        || normalized.contains("hair")
        || normalized.contains("eyes")
    {
        PixelAssetKind::CharacterLayer
    } else if normalized.contains("object")
        || normalized.contains("prop")
        || normalized.contains("furniture")
    {
        PixelAssetKind::ObjectSprite
    } else if normalized.contains("ui") || normalized.contains("icon") {
        PixelAssetKind::UiTexture
    } else {
        PixelAssetKind::General
    }
}

fn infer_preview_mode(source_path: &Path) -> PixelPreviewMode {
    preview_for_kind(infer_asset_kind(source_path), PixelPreviewMode::None)
}

fn infer_asset_kind_from_metadata(
    metadata: &PixelDocumentMetadata,
    fallback: PixelAssetKind,
) -> PixelAssetKind {
    for tag in &metadata.tags {
        match tag.as_str() {
            "tile" => return PixelAssetKind::Tile,
            "tilesheet" => return PixelAssetKind::Tilesheet,
            "sprite_sheet" => return PixelAssetKind::SpriteSheet,
            "animation_sheet" => return PixelAssetKind::AnimationSheet,
            "ui_texture" => return PixelAssetKind::UiTexture,
            "object_sprite" => return PixelAssetKind::ObjectSprite,
            "character_layer" => return PixelAssetKind::CharacterLayer,
            _ => {}
        }
    }
    fallback
}

fn preview_for_kind(kind: PixelAssetKind, fallback: PixelPreviewMode) -> PixelPreviewMode {
    match kind {
        PixelAssetKind::Tile | PixelAssetKind::Tilesheet => PixelPreviewMode::Repeat,
        PixelAssetKind::SpriteSheet
        | PixelAssetKind::AnimationSheet
        | PixelAssetKind::CharacterLayer => PixelPreviewMode::Character,
        PixelAssetKind::ObjectSprite => PixelPreviewMode::Object,
        PixelAssetKind::UiTexture => PixelPreviewMode::Ui,
        PixelAssetKind::General => fallback,
    }
}

fn infer_grid(source_path: &Path, image: &RgbaImage) -> crate::PixelGrid {
    let normalized = source_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let preferred_cell = if normalized.contains("character")
        || normalized.contains("player_")
        || normalized.contains("body_")
        || normalized.contains("animation")
    {
        64
    } else {
        32
    };

    for cell in [preferred_cell, 32, 64] {
        for spacing in 0..=4u32 {
            let stride = cell + spacing;
            for margin in 0..=4u32 {
                let usable_w = image.width().saturating_sub(margin.saturating_mul(2));
                let usable_h = image.height().saturating_sub(margin.saturating_mul(2));
                if usable_w < cell || usable_h < cell {
                    continue;
                }
                let cols_exact = usable_w.saturating_add(spacing) % stride == 0;
                let rows_exact = usable_h.saturating_add(spacing) % stride == 0;
                if cols_exact && rows_exact {
                    return crate::PixelGrid {
                        cell_width: cell,
                        cell_height: cell,
                        offset_x: margin as i32,
                        offset_y: margin as i32,
                        spacing_x: spacing,
                        spacing_y: spacing,
                        subgrid: if cell >= 64 { 8 } else { 4 },
                    };
                }
            }
        }
    }

    crate::PixelGrid {
        cell_width: preferred_cell,
        cell_height: preferred_cell,
        ..Default::default()
    }
}

fn load_metadata(
    sidecar: &Path,
    fallback: PixelDocumentMetadata,
) -> Result<PixelDocumentMetadata, String> {
    let raw = fs::read_to_string(sidecar)
        .map_err(|error| format!("failed to read {}: {error}", sidecar.display()))?;
    let mut metadata: PixelDocumentMetadata = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", sidecar.display()))?;
    if metadata.schema != DOCUMENT_SCHEMA_V1
        && metadata.schema != DOCUMENT_SCHEMA_V2
        && metadata.schema != DOCUMENT_SCHEMA_V3
    {
        return Err(format!(
            "unsupported pixel document schema {} in {}",
            metadata.schema,
            sidecar.display()
        ));
    }
    if metadata.asset_id.is_empty() {
        metadata.asset_id = fallback.asset_id;
    }
    if metadata.output_path.is_empty() {
        metadata.output_path = fallback.output_path;
    }
    if metadata.display_name.is_empty() {
        metadata.display_name = fallback.display_name;
    }
    if metadata.source_path.is_empty() {
        metadata.source_path = fallback.source_path;
    }
    if metadata.asset_kind == PixelAssetKind::General {
        metadata.asset_kind = infer_asset_kind_from_metadata(&metadata, fallback.asset_kind);
    }
    if metadata.preview_mode == PixelPreviewMode::None {
        metadata.preview_mode = preview_for_kind(metadata.asset_kind, fallback.preview_mode);
    }
    Ok(metadata)
}

fn load_layers_from_metadata(
    metadata: &[PixelLayerMetadata],
    parent: &Path,
    fallback: &RgbaImage,
) -> Result<Vec<PixelLayer>, String> {
    let mut layers = Vec::with_capacity(metadata.len());
    for (index, layer_metadata) in metadata.iter().enumerate() {
        let image = if layer_metadata.image_path.is_empty() {
            if index == 0 {
                fallback.clone()
            } else {
                RgbaImage::from_pixel(fallback.width(), fallback.height(), Rgba([0, 0, 0, 0]))
            }
        } else {
            let path = parent.join(&layer_metadata.image_path);
            image::open(&path)
                .map_err(|error| format!("failed to load layer {}: {error}", path.display()))?
                .to_rgba8()
        };
        layers.push(PixelLayer {
            metadata: layer_metadata.clone(),
            image,
        });
    }
    Ok(layers)
}

fn normalize_layer_dimensions(layers: &mut [PixelLayer], width: u32, height: u32) {
    for layer in layers {
        if layer.image.width() == width && layer.image.height() == height {
            continue;
        }
        let mut normalized = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        image::imageops::replace(&mut normalized, &layer.image, 0, 0);
        layer.image = normalized;
    }
}

fn load_recovery(repo_root: &Path, asset_id: &str) -> Result<Option<PixelDocument>, String> {
    let recovery = recovery_directory(repo_root, asset_id);
    let manifest_path = recovery.join("document.json");
    if !manifest_path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest: PixelRecoveryManifest = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    if manifest.schema != RECOVERY_SCHEMA || manifest.metadata.asset_id != asset_id {
        return Ok(None);
    }
    let fallback = RgbaImage::from_pixel(
        manifest.metadata.width.max(1),
        manifest.metadata.height.max(1),
        Rgba([0, 0, 0, 0]),
    );
    let layers = load_layers_from_metadata(&manifest.layers, &recovery, &fallback)?;
    Ok(Some(PixelDocument::from_loaded_parts(
        manifest.metadata,
        layers,
        true,
        true,
    )))
}

fn recovery_directory(repo_root: &Path, asset_id: &str) -> PathBuf {
    repo_root.join(RECOVERY_ROOT).join(slugify(asset_id))
}

fn find_repo_root(source_path: &Path) -> Option<PathBuf> {
    let mut current = source_path.parent();
    while let Some(directory) = current {
        if directory.join("Cargo.toml").is_file()
            && directory.join("crates").is_dir()
            && directory.join("assets").is_dir()
        {
            return Some(directory.to_path_buf());
        }
        current = directory.parent();
    }
    None
}

fn clamp_selection(selection: &mut PixelSelection, width: u32, height: u32) {
    selection.x = selection.x.min(width.saturating_sub(1));
    selection.y = selection.y.min(height.saturating_sub(1));
    selection.width = selection.width.min(width.saturating_sub(selection.x));
    selection.height = selection.height.min(height.saturating_sub(selection.y));
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "havenwild_pixel_{name}_{}_{}",
            std::process::id(),
            unix_millis()
        ))
    }

    #[test]
    fn generated_strict_32_atlas_infers_margin_and_spacing() {
        let image = RgbaImage::new(274, 138);
        let grid = infer_grid(
            Path::new("assets/generated/worldgen_v0_1/terrain/common_base_terrain_32.png"),
            &image,
        );
        assert_eq!(grid.cell_width, 32);
        assert_eq!(grid.cell_height, 32);
        assert_eq!(grid.offset_x, 2);
        assert_eq!(grid.offset_y, 2);
        assert_eq!(grid.spacing_x, 2);
        assert_eq!(grid.spacing_y, 2);
    }

    #[test]
    fn layered_document_round_trips() {
        let root = temporary_root("layers");
        fs::create_dir_all(root.join("assets/source/original/pixel_studio")).unwrap();
        let mut document = PixelDocument::from_rgba(8, 8, "round trip");
        document.add_layer("Highlights");
        document.begin_edit();
        document.set_pixel(3, 4, [200, 180, 90, 255]);
        document.save(&root).unwrap();
        let output = root.join(&document.metadata.output_path);
        let loaded = PixelDocument::load(output, "round trip", PixelLicense::default()).unwrap();
        assert_eq!(loaded.layer_count(), 2);
        assert_eq!(loaded.color_at(3, 4), [200, 180, 90, 255]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn autosave_is_restored_until_normal_save() {
        let root = temporary_root("recovery");
        fs::create_dir_all(root.join("assets/source/original/pixel_studio")).unwrap();
        let source = root.join("assets/source/original/pixel_studio/recovery.png");
        RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]))
            .save(&source)
            .unwrap();
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::create_dir_all(root.join("crates")).unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();

        let mut document =
            PixelDocument::load(&source, "recovery", PixelLicense::default()).unwrap();
        document.begin_edit();
        document.set_pixel(1, 1, [10, 20, 30, 255]);
        document.autosave(&root).unwrap();
        let recovered = PixelDocument::load(&source, "recovery", PixelLicense::default()).unwrap();
        assert!(recovered.recovered_from_autosave);
        assert_eq!(recovered.color_at(1, 1), [10, 20, 30, 255]);
        let _ = fs::remove_dir_all(root);
    }
}
