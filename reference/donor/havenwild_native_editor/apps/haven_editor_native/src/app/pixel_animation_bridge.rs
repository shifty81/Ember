use super::pixel_studio::{PixelAnimationEditContext, PixelInspectorTab};
use super::*;
use haven_assets::asset_intake::repo_root_dir;
use haven_pixel::PixelDocument;
use std::path::{Path, PathBuf};

impl EditorApp {
    pub(crate) fn open_selected_animation_frame_in_pixel_studio(&mut self) {
        let Some(animation) = self.animation_studio.document.as_ref() else {
            self.status_message = "Open an animation source before editing pixels".to_string();
            return;
        };
        let Some(clip) = animation.clip() else {
            self.status_message = "The selected animation has no active clip".to_string();
            return;
        };
        let Some(frame) = animation.frame() else {
            self.status_message =
                "Add or select an animation frame before editing pixels".to_string();
            return;
        };

        let frame_count = clip.frames.len();
        let previous_source = (frame_count > 1).then(|| {
            let index = if animation.selected_frame == 0 {
                frame_count - 1
            } else {
                animation.selected_frame - 1
            };
            clip.frames[index].source
        });
        let next_source = (frame_count > 1).then(|| {
            let index = (animation.selected_frame + 1) % frame_count;
            clip.frames[index].source
        });
        let animation_display_name = animation.metadata.display_name.clone();
        let clip_label = clip.display_name.clone();
        let frame_number = animation.selected_frame + 1;
        let context = PixelAnimationEditContext {
            animation_asset_id: animation.metadata.asset_id.clone(),
            animation_display_name: animation_display_name.clone(),
            clip_index: animation.selected_clip,
            frame_index: animation.selected_frame,
            clip_label: clip_label.clone(),
            direction_label: clip.direction.label().to_string(),
            source_path_before_edit: animation.metadata.source_path.clone(),
            frame_source: frame.source,
            previous_source,
            next_source,
            shadow_offset: frame.shadow_offset,
            sockets: frame.sockets.clone(),
            onion_skin: self.animation_studio.onion_skin,
        };
        let source_path = resolve_project_path(&animation.metadata.source_path);
        let mut document = match PixelDocument::load(
            &source_path,
            &animation.metadata.display_name,
            animation.metadata.license.clone(),
        ) {
            Ok(document) => document,
            Err(error) => {
                self.status_message =
                    format!("Unable to open animation source in Pixel Studio: {error}");
                return;
            }
        };
        document.metadata.grid.cell_width = animation.metadata.frame_width.max(1);
        document.metadata.grid.cell_height = animation.metadata.frame_height.max(1);
        document.metadata.grid.offset_x = animation.metadata.grid_offset[0];
        document.metadata.grid.offset_y = animation.metadata.grid_offset[1];
        document.metadata.selection = frame.source;
        document.metadata.pivot = [
            frame.source.x as i32 + frame.pivot[0],
            frame.source.y as i32 + frame.pivot[1],
        ];

        self.pixel_studio.document = Some(document);
        self.pixel_studio.animation_context = Some(context);
        self.pixel_studio.inspector_tab = PixelInspectorTab::Animation;
        self.pixel_studio.layer_offset = 0;
        self.pixel_studio.layer_rename_buffer = None;
        self.pixel_studio.refresh_texture();
        let canvas = self.pixel_canvas_rect();
        self.pixel_studio.frame_selection(canvas);
        self.pixel_studio.next_autosave_at = get_time() + 10.0;
        self.pixel_studio.autosave_status =
            "Animation frame bridge active; edits autosave to recovery".to_string();
        self.animation_studio.playing = false;
        self.viewport_mode = EditorViewportMode::PixelStudio;
        self.status_message = format!(
            "Editing {animation_display_name} / {clip_label} frame {frame_number} in Pixel Studio"
        );
    }

    pub(crate) fn save_animation_pixels_and_return(&mut self) {
        let Some(context) = self.pixel_studio.animation_context.clone() else {
            self.status_message = "No Animation Studio frame bridge is active".to_string();
            return;
        };
        let Some(document) = self.pixel_studio.document.as_mut() else {
            self.status_message = "No Pixel Studio document is open".to_string();
            return;
        };
        let root = repo_root_dir();
        let frame_right = context
            .frame_source
            .x
            .saturating_add(context.frame_source.width);
        let frame_bottom = context
            .frame_source
            .y
            .saturating_add(context.frame_source.height);
        if frame_right > document.width() || frame_bottom > document.height() {
            self.status_message =
                "Animation frame source no longer fits inside the Pixel Studio document"
                    .to_string();
            return;
        }
        if let Err(error) = document.save(&root) {
            self.status_message = format!("Animation frame pixel save failed: {error}");
            return;
        }
        let output_path = document.metadata.output_path.clone();
        let source_dimensions = [document.width(), document.height()];
        let pixel_pivot = document.metadata.pivot;

        let Some(animation) = self.animation_studio.document.as_mut() else {
            self.status_message =
                "Animation document was closed while Pixel Studio was active".to_string();
            return;
        };
        if animation.metadata.asset_id != context.animation_asset_id {
            self.status_message =
                "Animation context changed; refusing to write into another animation".to_string();
            return;
        }
        let clip_max = animation.metadata.clips.len().saturating_sub(1);
        animation.selected_clip = context.clip_index.min(clip_max);
        let frame_max = animation
            .metadata
            .clips
            .get(animation.selected_clip)
            .map_or(0, |clip| clip.frames.len().saturating_sub(1));
        animation.selected_frame = context.frame_index.min(frame_max);
        animation.metadata.source_path = output_path.clone();
        animation.metadata.image_width = source_dimensions[0];
        animation.metadata.image_height = source_dimensions[1];
        if let Some(frame) = animation.frame_mut() {
            frame.pivot = [
                (pixel_pivot[0] - frame.source.x as i32)
                    .clamp(0, frame.source.width.saturating_sub(1) as i32),
                (pixel_pivot[1] - frame.source.y as i32)
                    .clamp(0, frame.source.height.saturating_sub(1) as i32),
            ];
        }
        animation.dirty = true;
        if let Err(error) = animation.save(&root) {
            self.status_message =
                format!("Animation metadata save failed after pixel save: {error}");
            return;
        }
        if let Err(error) = self.reload_animation_texture_from_path(&output_path) {
            self.status_message =
                format!("Saved pixels, but animation preview reload failed: {error}");
            return;
        }
        self.pixel_studio.animation_context = None;
        self.viewport_mode = EditorViewportMode::AnimationStudio;
        self.status_message = format!(
            "Saved Pixel Studio changes and returned to {} frame {}",
            context.clip_label,
            context.frame_index + 1
        );
    }

    pub(crate) fn return_to_animation_without_pixel_save(&mut self) {
        let Some(context) = self.pixel_studio.animation_context.take() else {
            self.status_message = "No Animation Studio frame bridge is active".to_string();
            return;
        };
        if let Some(document) = self.animation_studio.document.as_mut() {
            document.selected_clip = context
                .clip_index
                .min(document.metadata.clips.len().saturating_sub(1));
            let frame_max = document
                .metadata
                .clips
                .get(document.selected_clip)
                .map_or(0, |clip| clip.frames.len().saturating_sub(1));
            document.selected_frame = context.frame_index.min(frame_max);
        }
        self.viewport_mode = EditorViewportMode::AnimationStudio;
        self.status_message = format!(
            "Returned to {} frame {} without saving Pixel Studio changes",
            context.clip_label,
            context.frame_index + 1
        );
    }

    pub(crate) fn reload_animation_texture_from_path(
        &mut self,
        source_path: &str,
    ) -> Result<(), String> {
        let path = resolve_project_path(source_path);
        let image = image::open(&path)
            .map_err(|error| format!("failed to load {}: {error}", path.display()))?
            .to_rgba8();
        if image.width() > u16::MAX as u32 || image.height() > u16::MAX as u32 {
            return Err("source image exceeds Macroquad texture limits".to_string());
        }
        let texture =
            Texture2D::from_rgba8(image.width() as u16, image.height() as u16, image.as_raw());
        texture.set_filter(FilterMode::Nearest);
        self.animation_studio.texture = Some(texture);
        Ok(())
    }
}

fn resolve_project_path(path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        repo_root_dir().join(candidate)
    }
}
