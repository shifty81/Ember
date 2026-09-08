use crate::{PixelAssetGeometry, PixelDocument, PixelRectI32};
use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const PIXEL_ATLAS_DRAFT_ROOT: &str = "assets/generated/pixel_studio/atlas_drafts";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaTrimScan {
    pub trim_rect: PixelRectI32,
    pub render_offset: [i32; 2],
    pub opaque_pixels: u64,
    pub alpha_threshold: u8,
}

impl AlphaTrimScan {
    pub fn is_empty(self) -> bool {
        self.opaque_pixels == 0 || self.trim_rect.width <= 0 || self.trim_rect.height <= 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableAtlasEntry {
    pub stable_id: String,
    pub source_rect: PixelRectI32,
    pub logical_frame: PixelRectI32,
    pub trim_rect: PixelRectI32,
    pub atlas_rect: PixelRectI32,
    pub render_offset: [i32; 2],
    pub ground_anchor: [i32; 2],
    pub opaque_pixels: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableAtlasManifest {
    pub schema: String,
    pub atlas_path: String,
    pub comparison_path: String,
    pub width: u32,
    pub height: u32,
    pub padding: u32,
    pub entries: Vec<VariableAtlasEntry>,
}

#[derive(Clone, Debug)]
pub struct AtlasDraftSource {
    pub stable_id: String,
    pub image: RgbaImage,
    pub geometry: PixelAssetGeometry,
}

#[derive(Clone, Debug)]
pub struct VariableAtlasDraft {
    pub image: RgbaImage,
    pub entries: Vec<VariableAtlasEntry>,
    pub padding: u32,
}

pub fn pack_variable_atlas(
    mut sources: Vec<AtlasDraftSource>,
    max_width: u32,
    padding: u32,
) -> Result<VariableAtlasDraft, String> {
    if sources.is_empty() {
        return Err("atlas draft requires at least one source".to_string());
    }
    let padding = padding.clamp(0, 32);
    let max_width = max_width.clamp(32, 8192);
    sources.sort_by(|left, right| {
        right
            .geometry
            .logical_frame
            .height
            .cmp(&left.geometry.logical_frame.height)
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });

    struct Pending {
        source: AtlasDraftSource,
        scan: AlphaTrimScan,
        atlas_rect: PixelRectI32,
    }

    let mut pending = Vec::with_capacity(sources.len());
    let mut cursor_x = padding;
    let mut cursor_y = padding;
    let mut shelf_height = 0u32;
    let mut used_width = 1u32;

    for source in sources {
        let scan = scan_alpha_bounds(&source.image, source.geometry.logical_frame, 0);
        if scan.is_empty() {
            return Err(format!("{} contains no visible pixels", source.stable_id));
        }
        let width = scan.trim_rect.width as u32;
        let height = scan.trim_rect.height as u32;
        if width + padding * 2 > max_width {
            return Err(format!(
                "{} requires {} atlas pixels but max width is {}",
                source.stable_id,
                width + padding * 2,
                max_width
            ));
        }
        if cursor_x + width + padding > max_width && cursor_x > padding {
            cursor_x = padding;
            cursor_y = cursor_y.saturating_add(shelf_height + padding);
            shelf_height = 0;
        }
        let atlas_rect = PixelRectI32::new(
            cursor_x as i32,
            cursor_y as i32,
            width as i32,
            height as i32,
        );
        used_width = used_width.max(cursor_x + width + padding);
        cursor_x = cursor_x.saturating_add(width + padding);
        shelf_height = shelf_height.max(height);
        pending.push(Pending {
            source,
            scan,
            atlas_rect,
        });
    }

    let used_height = cursor_y.saturating_add(shelf_height + padding).max(1);
    let atlas_width = next_power_of_two(used_width.min(max_width));
    let atlas_height = next_power_of_two(used_height);
    let mut image = RgbaImage::from_pixel(atlas_width, atlas_height, Rgba([0, 0, 0, 0]));
    let mut entries = Vec::with_capacity(pending.len());

    for item in pending {
        let trimmed = crop_rect(&item.source.image, item.scan.trim_rect)?;
        imageops::replace(
            &mut image,
            &trimmed,
            item.atlas_rect.x as i64,
            item.atlas_rect.y as i64,
        );
        entries.push(VariableAtlasEntry {
            stable_id: item.source.stable_id,
            source_rect: item.source.geometry.source_rect,
            logical_frame: item.source.geometry.logical_frame,
            trim_rect: item.scan.trim_rect,
            atlas_rect: item.atlas_rect,
            render_offset: item.scan.render_offset,
            ground_anchor: item.source.geometry.ground_anchor,
            opaque_pixels: item.scan.opaque_pixels,
        });
    }

    Ok(VariableAtlasDraft {
        image,
        entries,
        padding,
    })
}

#[derive(Clone, Debug)]
pub struct AtlasBatchExport {
    pub atlas_path: PathBuf,
    pub manifest_path: PathBuf,
    pub review_board_path: PathBuf,
    pub entries: Vec<VariableAtlasEntry>,
}

/// Exports a multi-entry variable atlas and a review board that compares every
/// logical source frame against its reconstructed atlas entry. Source images are
/// never modified.
pub fn export_batch_atlas_review(
    sources: Vec<AtlasDraftSource>,
    repo_root: impl AsRef<Path>,
    batch_id: &str,
    max_width: u32,
    padding: u32,
) -> Result<AtlasBatchExport, String> {
    let repo_root = repo_root.as_ref();
    let output_dir = repo_root
        .join(PIXEL_ATLAS_DRAFT_ROOT)
        .join(sanitize_id(batch_id));
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("could not create {}: {error}", output_dir.display()))?;

    let source_copies = sources.clone();
    let draft = pack_variable_atlas(sources, max_width, padding)?;
    let atlas_path = output_dir.join("atlas.png");
    let manifest_path = output_dir.join("atlas_draft.json");
    let review_board_path = output_dir.join("atlas_review_board.png");
    draft
        .image
        .save(&atlas_path)
        .map_err(|error| format!("could not save {}: {error}", atlas_path.display()))?;

    let board = build_batch_review_board(&source_copies, &draft.image, &draft.entries)?;
    board
        .save(&review_board_path)
        .map_err(|error| format!("could not save {}: {error}", review_board_path.display()))?;

    let manifest = VariableAtlasManifest {
        schema: "havenwild.variable_atlas_draft.v2".to_string(),
        atlas_path: relative_string(repo_root, &atlas_path),
        comparison_path: relative_string(repo_root, &review_board_path),
        width: draft.image.width(),
        height: draft.image.height(),
        padding: draft.padding,
        entries: draft.entries.clone(),
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("could not serialize atlas review: {error}"))?;
    fs::write(&manifest_path, format!("{json}\n"))
        .map_err(|error| format!("could not save {}: {error}", manifest_path.display()))?;

    Ok(AtlasBatchExport {
        atlas_path,
        manifest_path,
        review_board_path,
        entries: draft.entries,
    })
}

fn build_batch_review_board(
    sources: &[AtlasDraftSource],
    atlas: &RgbaImage,
    entries: &[VariableAtlasEntry],
) -> Result<RgbaImage, String> {
    let gap = 8u32;
    let row_gap = 12u32;
    let row_width = sources
        .iter()
        .map(|source| {
            let w = source.geometry.logical_frame.width.max(1) as u32;
            w * 2 + gap
        })
        .max()
        .unwrap_or(1);
    let total_height = sources
        .iter()
        .map(|source| source.geometry.logical_frame.height.max(1) as u32 + row_gap)
        .sum::<u32>()
        .max(1);
    let mut board = RgbaImage::from_pixel(row_width, total_height, Rgba([0, 0, 0, 0]));
    let mut y = 0u32;
    for source in sources {
        let entry = entries
            .iter()
            .find(|entry| entry.stable_id == source.stable_id)
            .ok_or_else(|| format!("missing packed entry for {}", source.stable_id))?;
        let logical_w = entry.logical_frame.width.max(1) as u32;
        let logical_h = entry.logical_frame.height.max(1) as u32;
        let original = crop_rect(&source.image, entry.logical_frame)?;
        imageops::replace(&mut board, &original, 0, y as i64);
        let packed = crop_rect(atlas, entry.atlas_rect)?;
        imageops::replace(
            &mut board,
            &packed,
            (logical_w + gap) as i64 + entry.render_offset[0] as i64,
            y as i64 + entry.render_offset[1] as i64,
        );
        y = y.saturating_add(logical_h + row_gap);
    }
    Ok(board)
}

#[derive(Clone, Debug)]
pub struct AtlasDraftExport {
    pub atlas_path: PathBuf,
    pub manifest_path: PathBuf,
    pub comparison_path: PathBuf,
    pub entry: VariableAtlasEntry,
}

pub fn scan_alpha_bounds(
    image: &RgbaImage,
    bounds: PixelRectI32,
    alpha_threshold: u8,
) -> AlphaTrimScan {
    let left = bounds.x.max(0) as u32;
    let top = bounds.y.max(0) as u32;
    let right = (bounds.x + bounds.width)
        .max(bounds.x)
        .clamp(0, image.width() as i32) as u32;
    let bottom = (bounds.y + bounds.height)
        .max(bounds.y)
        .clamp(0, image.height() as i32) as u32;

    let mut min_x = right;
    let mut min_y = bottom;
    let mut max_x = left;
    let mut max_y = top;
    let mut opaque_pixels = 0u64;

    for y in top..bottom {
        for x in left..right {
            if image.get_pixel(x, y)[3] <= alpha_threshold {
                continue;
            }
            opaque_pixels += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if opaque_pixels == 0 {
        return AlphaTrimScan {
            trim_rect: PixelRectI32::new(bounds.x, bounds.y, 0, 0),
            render_offset: [0, 0],
            opaque_pixels,
            alpha_threshold,
        };
    }

    let trim_rect = PixelRectI32::new(
        min_x as i32,
        min_y as i32,
        (max_x - min_x + 1) as i32,
        (max_y - min_y + 1) as i32,
    );
    AlphaTrimScan {
        render_offset: [trim_rect.x - bounds.x, trim_rect.y - bounds.y],
        trim_rect,
        opaque_pixels,
        alpha_threshold,
    }
}

impl PixelDocument {
    /// Updates trim metadata without cropping, resizing, or rewriting source pixels.
    pub fn scan_alpha_trim(&mut self, alpha_threshold: u8) -> AlphaTrimScan {
        let logical = self.metadata.geometry.logical_frame;
        let scan = scan_alpha_bounds(&self.composite, logical, alpha_threshold);
        self.begin_edit();
        self.metadata.geometry.trim_rect = scan.trim_rect;
        self.metadata.geometry.render_offset = scan.render_offset;
        self.dirty = true;
        scan
    }
}

pub fn export_document_atlas_draft(
    document: &PixelDocument,
    repo_root: impl AsRef<Path>,
    padding: u32,
) -> Result<AtlasDraftExport, String> {
    let repo_root = repo_root.as_ref();
    let geometry = document.metadata.geometry;
    let scan = scan_alpha_bounds(&document.composite, geometry.logical_frame, 0);
    if scan.is_empty() {
        return Err("selected logical frame contains no visible pixels".to_string());
    }

    let slug = sanitize_id(&document.metadata.asset_id);
    let output_dir = repo_root.join(PIXEL_ATLAS_DRAFT_ROOT).join(&slug);
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("could not create {}: {error}", output_dir.display()))?;

    let draft = pack_variable_atlas(
        vec![AtlasDraftSource {
            stable_id: document.metadata.asset_id.clone(),
            image: document.composite.clone(),
            geometry,
        }],
        2048,
        padding,
    )?;
    let atlas = draft.image;
    let entry = draft
        .entries
        .into_iter()
        .next()
        .ok_or_else(|| "atlas draft produced no entries".to_string())?;

    let atlas_path = output_dir.join("atlas.png");
    let comparison_path = output_dir.join("source_vs_reconstructed.png");
    let manifest_path = output_dir.join("atlas_draft.json");
    atlas
        .save(&atlas_path)
        .map_err(|error| format!("could not save {}: {error}", atlas_path.display()))?;

    let comparison = build_comparison(document, &atlas, &entry)?;
    comparison
        .save(&comparison_path)
        .map_err(|error| format!("could not save {}: {error}", comparison_path.display()))?;

    let manifest = VariableAtlasManifest {
        schema: "havenwild.variable_atlas_draft.v1".to_string(),
        atlas_path: relative_string(repo_root, &atlas_path),
        comparison_path: relative_string(repo_root, &comparison_path),
        width: atlas.width(),
        height: atlas.height(),
        padding: padding.clamp(0, 32),
        entries: vec![entry.clone()],
    };
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("could not serialize atlas draft: {error}"))?;
    fs::write(&manifest_path, format!("{json}\n"))
        .map_err(|error| format!("could not save {}: {error}", manifest_path.display()))?;

    Ok(AtlasDraftExport {
        atlas_path,
        manifest_path,
        comparison_path,
        entry,
    })
}

fn build_comparison(
    document: &PixelDocument,
    atlas: &RgbaImage,
    entry: &VariableAtlasEntry,
) -> Result<RgbaImage, String> {
    let logical_w = entry.logical_frame.width.max(1) as u32;
    let logical_h = entry.logical_frame.height.max(1) as u32;
    let gap = 8u32;
    let mut comparison = RgbaImage::from_pixel(logical_w * 2 + gap, logical_h, Rgba([0, 0, 0, 0]));
    let source = crop_rect(&document.composite, entry.logical_frame)?;
    imageops::replace(&mut comparison, &source, 0, 0);

    let packed = crop_rect(atlas, entry.atlas_rect)?;
    imageops::replace(
        &mut comparison,
        &packed,
        (logical_w + gap) as i64 + entry.render_offset[0] as i64,
        entry.render_offset[1] as i64,
    );
    Ok(comparison)
}

fn crop_rect(image: &RgbaImage, rect: PixelRectI32) -> Result<RgbaImage, String> {
    if rect.width <= 0 || rect.height <= 0 || rect.x < 0 || rect.y < 0 {
        return Err(format!("invalid image rectangle {rect:?}"));
    }
    let x = rect.x as u32;
    let y = rect.y as u32;
    let width = rect.width as u32;
    let height = rect.height as u32;
    if x.saturating_add(width) > image.width() || y.saturating_add(height) > image.height() {
        return Err(format!(
            "rectangle {rect:?} exceeds image {}x{}",
            image.width(),
            image.height()
        ));
    }
    Ok(imageops::crop_imm(image, x, y, width, height).to_image())
}

fn next_power_of_two(value: u32) -> u32 {
    value.checked_next_power_of_two().unwrap_or(value).max(1)
}

fn sanitize_id(value: &str) -> String {
    let result = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    result.trim_matches('_').to_string()
}

fn relative_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_scan_is_non_destructive_and_returns_offset() {
        let mut image = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
        for y in 5..10 {
            for x in 3..8 {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let result = scan_alpha_bounds(&image, PixelRectI32::new(0, 0, 16, 16), 0);
        assert_eq!(result.trim_rect, PixelRectI32::new(3, 5, 5, 5));
        assert_eq!(result.render_offset, [3, 5]);
        assert_eq!(result.opaque_pixels, 25);
        assert_eq!(image.dimensions(), (16, 16));
    }

    #[test]
    fn variable_packer_keeps_different_trim_sizes() {
        let mut first = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
        first.put_pixel(2, 2, Rgba([255, 255, 255, 255]));
        let mut second = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
        for y in 1..5 {
            for x in 3..9 {
                second.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let geometry = PixelAssetGeometry {
            logical_frame: PixelRectI32::new(0, 0, 16, 16),
            source_rect: PixelRectI32::new(0, 0, 16, 16),
            ..PixelAssetGeometry::default()
        };
        let draft = pack_variable_atlas(
            vec![
                AtlasDraftSource {
                    stable_id: "one".into(),
                    image: first,
                    geometry,
                },
                AtlasDraftSource {
                    stable_id: "two".into(),
                    image: second,
                    geometry,
                },
            ],
            64,
            2,
        )
        .unwrap();
        assert_eq!(draft.entries.len(), 2);
        assert!(draft
            .entries
            .iter()
            .any(|entry| entry.atlas_rect.width == 1));
        assert!(draft
            .entries
            .iter()
            .any(|entry| entry.atlas_rect.width == 6));
    }

    #[test]
    fn batch_review_preserves_all_entries() {
        let mut one = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        one.put_pixel(1, 1, Rgba([255, 255, 255, 255]));
        let mut two = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        two.put_pixel(5, 6, Rgba([255, 255, 255, 255]));
        let geometry = PixelAssetGeometry {
            logical_frame: PixelRectI32::new(0, 0, 8, 8),
            source_rect: PixelRectI32::new(0, 0, 8, 8),
            ..PixelAssetGeometry::default()
        };
        let sources = vec![
            AtlasDraftSource {
                stable_id: "one".into(),
                image: one,
                geometry,
            },
            AtlasDraftSource {
                stable_id: "two".into(),
                image: two,
                geometry,
            },
        ];
        let draft = pack_variable_atlas(sources.clone(), 64, 2).unwrap();
        let board = build_batch_review_board(&sources, &draft.image, &draft.entries).unwrap();
        assert_eq!(draft.entries.len(), 2);
        assert!(board.width() >= 16);
        assert!(board.height() >= 16);
    }

    #[test]
    fn empty_scan_returns_zero_sized_trim() {
        let image = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        let result = scan_alpha_bounds(&image, PixelRectI32::new(0, 0, 8, 8), 0);
        assert!(result.is_empty());
        assert_eq!(result.trim_rect.width, 0);
        assert_eq!(result.trim_rect.height, 0);
    }
}
