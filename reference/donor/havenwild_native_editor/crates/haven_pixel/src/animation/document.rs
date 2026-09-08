use super::{
    publish::publish_runtime, types::default_frame_duration_ms, AnimationClip, AnimationDirection,
    AnimationDocumentMetadata, AnimationEventKind, AnimationFrame, AnimationPublishResult,
};
use crate::{PixelDocument, PixelLicense, PixelSelection};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AnimationDocument {
    pub metadata: AnimationDocumentMetadata,
    pub selected_clip: usize,
    pub selected_frame: usize,
    pub dirty: bool,
}

impl AnimationDocument {
    pub fn load_or_create(
        source_path: impl AsRef<Path>,
        display_name: impl Into<String>,
        license: PixelLicense,
        image_width: u32,
        image_height: u32,
    ) -> Result<Self, String> {
        let source_path = source_path.as_ref();
        let display_name = display_name.into();
        let stem = slugify(&display_name);
        let output_path = format!(
            "{}/{}.hhanim.json",
            super::ANIMATION_STUDIO_OUTPUT_ROOT,
            stem
        );
        let workspace_root = find_workspace_root(source_path);
        let candidate = workspace_root.as_deref().map_or_else(
            || PathBuf::from(&output_path),
            |root| root.join(&output_path),
        );
        let source_reference = workspace_root
            .as_deref()
            .and_then(|root| source_path.strip_prefix(root).ok())
            .unwrap_or(source_path)
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = if candidate.is_file() {
            let text = fs::read_to_string(&candidate)
                .map_err(|error| format!("failed to read {}: {error}", candidate.display()))?;
            serde_json::from_str::<AnimationDocumentMetadata>(&text)
                .map_err(|error| format!("failed to parse {}: {error}", candidate.display()))?
        } else {
            let (frame_width, frame_height) =
                recommended_frame_size(source_path, image_width, image_height);
            AnimationDocumentMetadata {
                schema: "havenwild.animation_document.v0_1".to_string(),
                asset_id: format!("animation/{stem}"),
                display_name,
                source_path: source_reference,
                output_path,
                image_width,
                image_height,
                frame_width,
                frame_height,
                grid_offset: [0, 0],
                clips: vec![AnimationClip::new(1)],
                license,
            }
        };
        let mut document = Self {
            metadata,
            selected_clip: 0,
            selected_frame: 0,
            dirty: false,
        };
        document.clamp_selection();
        Ok(document)
    }

    pub fn from_pixel_document(document: &PixelDocument) -> Self {
        let mut animation = Self {
            metadata: AnimationDocumentMetadata {
                schema: "havenwild.animation_document.v0_1".to_string(),
                asset_id: format!("animation/{}", slugify(&document.metadata.display_name)),
                display_name: document.metadata.display_name.clone(),
                source_path: document.metadata.output_path.clone(),
                output_path: format!(
                    "{}/{}.hhanim.json",
                    super::ANIMATION_STUDIO_OUTPUT_ROOT,
                    slugify(&document.metadata.display_name)
                ),
                image_width: document.width(),
                image_height: document.height(),
                frame_width: document.metadata.grid.cell_width.max(1),
                frame_height: document.metadata.grid.cell_height.max(1),
                grid_offset: [
                    document.metadata.grid.offset_x,
                    document.metadata.grid.offset_y,
                ],
                clips: vec![AnimationClip::new(1)],
                license: document.metadata.license.clone(),
            },
            selected_clip: 0,
            selected_frame: 0,
            dirty: true,
        };
        animation.add_frame(document.metadata.selection);
        animation
    }

    pub fn clip(&self) -> Option<&AnimationClip> {
        self.metadata.clips.get(self.selected_clip)
    }

    pub fn clip_mut(&mut self) -> Option<&mut AnimationClip> {
        self.metadata.clips.get_mut(self.selected_clip)
    }

    pub fn frame(&self) -> Option<&AnimationFrame> {
        self.clip()?.frames.get(self.selected_frame)
    }

    pub fn frame_mut(&mut self) -> Option<&mut AnimationFrame> {
        let frame_index = self.selected_frame;
        self.clip_mut()?.frames.get_mut(frame_index)
    }

    pub fn add_clip(&mut self) {
        let index = self.metadata.clips.len() + 1;
        self.metadata.clips.push(AnimationClip::new(index));
        self.selected_clip = self.metadata.clips.len() - 1;
        self.selected_frame = 0;
        self.dirty = true;
    }

    pub fn delete_selected_clip(&mut self) -> bool {
        if self.metadata.clips.len() <= 1 {
            return false;
        }
        self.metadata.clips.remove(self.selected_clip);
        self.selected_clip = self.selected_clip.min(self.metadata.clips.len() - 1);
        self.selected_frame = 0;
        self.dirty = true;
        true
    }

    pub fn cycle_clip(&mut self, delta: i32) {
        self.selected_clip = cycle_index(self.selected_clip, self.metadata.clips.len(), delta);
        self.selected_frame = 0;
    }

    pub fn cycle_frame(&mut self, delta: i32) {
        let len = self.clip().map_or(0, |clip| clip.frames.len());
        self.selected_frame = cycle_index(self.selected_frame, len, delta);
    }

    pub fn add_frame(&mut self, source: PixelSelection) {
        if source.is_empty() {
            return;
        }
        let duration = self.clip().map_or(default_frame_duration_ms(), |clip| {
            clip.default_frame_duration_ms
        });
        let selected_frame = {
            let Some(clip) = self.clip_mut() else {
                return;
            };
            clip.frames.push(AnimationFrame::new(source, duration));
            clip.frames.len() - 1
        };
        self.selected_frame = selected_frame;
        self.dirty = true;
    }

    pub fn duplicate_selected_frame(&mut self) -> bool {
        let Some(frame) = self.frame().cloned() else {
            return false;
        };
        let insert_at = self.selected_frame + 1;
        {
            let Some(clip) = self.clip_mut() else {
                return false;
            };
            clip.frames.insert(insert_at, frame);
        }
        self.selected_frame = insert_at;
        self.dirty = true;
        true
    }

    pub fn delete_selected_frame(&mut self) -> bool {
        let selected = self.selected_frame;
        let remaining = {
            let Some(clip) = self.clip_mut() else {
                return false;
            };
            if selected >= clip.frames.len() {
                return false;
            }
            clip.frames.remove(selected);
            clip.frames.len()
        };
        self.selected_frame = selected.min(remaining.saturating_sub(1));
        self.dirty = true;
        true
    }

    pub fn move_selected_frame(&mut self, delta: i32) -> bool {
        let selected = self.selected_frame;
        let len = self.clip().map_or(0, |clip| clip.frames.len());
        if len < 2 {
            return false;
        }
        let target = if delta < 0 {
            selected.saturating_sub((-delta) as usize)
        } else {
            (selected + delta as usize).min(len - 1)
        };
        if target == selected {
            return false;
        }
        {
            let Some(clip) = self.clip_mut() else {
                return false;
            };
            clip.frames.swap(selected, target);
        }
        self.selected_frame = target;
        self.dirty = true;
        true
    }

    pub fn auto_slice_selected_row(&mut self, row: u32) -> usize {
        let frame_width = self.metadata.frame_width.max(1);
        let frame_height = self.metadata.frame_height.max(1);
        let offset_x = self.metadata.grid_offset[0].max(0) as u32;
        let offset_y = self.metadata.grid_offset[1].max(0) as u32;
        let y = offset_y.saturating_add(row.saturating_mul(frame_height));
        if y.saturating_add(frame_height) > self.metadata.image_height {
            return 0;
        }
        let columns = self.metadata.image_width.saturating_sub(offset_x) / frame_width;
        let duration = self.clip().map_or(default_frame_duration_ms(), |clip| {
            clip.default_frame_duration_ms
        });
        let frames: Vec<_> = (0..columns)
            .map(|column| {
                AnimationFrame::new(
                    PixelSelection {
                        x: offset_x + column * frame_width,
                        y,
                        width: frame_width,
                        height: frame_height,
                    },
                    duration,
                )
            })
            .collect();
        let count = frames.len();
        if let Some(clip) = self.clip_mut() {
            clip.frames = frames;
        }
        self.selected_frame = 0;
        self.dirty = true;
        count
    }

    pub fn apply_recommended_profile(&mut self) -> usize {
        let source = Path::new(&self.metadata.source_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if source == "base" || source == "base_1" || source.contains("cat") {
            return 0;
        }
        let rows = self.metadata.image_height / self.metadata.frame_height.max(1);
        let direction_order = [
            AnimationDirection::North,
            AnimationDirection::West,
            AnimationDirection::South,
            AnimationDirection::East,
        ];
        let clip_prefix = if source.contains("shadow") {
            "shadow"
        } else if source.contains("eat") {
            "eat"
        } else {
            "walk"
        };
        let suggested_rows = rows.min(4);
        if suggested_rows == 0 {
            return 0;
        }
        self.metadata.clips.clear();
        for row in 0..suggested_rows {
            let direction = direction_order[row as usize];
            let mut clip = AnimationClip::new(row as usize + 1);
            clip.id = format!("{}_{}", clip_prefix, direction.label().to_ascii_lowercase());
            clip.display_name = format!("{} {}", title_case(clip_prefix), direction.label());
            clip.direction = direction;
            self.metadata.clips.push(clip);
            self.selected_clip = self.metadata.clips.len() - 1;
            self.auto_slice_selected_row(row);
        }
        if source.contains("horse") && rows >= 8 {
            for row in 4..8 {
                let direction = direction_order[(row - 4) as usize];
                let mut clip = AnimationClip::new(self.metadata.clips.len() + 1);
                clip.id = format!("idle_{}", direction.label().to_ascii_lowercase());
                clip.display_name = format!("Idle {}", direction.label());
                clip.direction = direction;
                self.metadata.clips.push(clip);
                self.selected_clip = self.metadata.clips.len() - 1;
                self.auto_slice_selected_row(row);
            }
        }
        self.selected_clip = 0;
        self.selected_frame = 0;
        self.dirty = true;
        self.metadata.clips.len()
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();
        if self.metadata.frame_width == 0 || self.metadata.frame_height == 0 {
            issues.push("frame size must be non-zero".to_string());
        }
        if self.metadata.clips.is_empty() {
            issues.push("at least one animation clip is required".to_string());
        }
        let mut clip_ids = HashSet::new();
        for clip in &self.metadata.clips {
            if clip.id.trim().is_empty() {
                issues.push("clip id cannot be empty".to_string());
            } else if !clip_ids.insert(clip.id.clone()) {
                issues.push(format!("duplicate clip id {}", clip.id));
            }
            if clip.frames.is_empty() {
                issues.push(format!("clip {} has no frames", clip.id));
            }
            for (frame_index, frame) in clip.frames.iter().enumerate() {
                if frame.duration_ms == 0 {
                    issues.push(format!(
                        "clip {} frame {} has zero duration",
                        clip.id, frame_index
                    ));
                }
                if frame.source.is_empty()
                    || frame.source.x.saturating_add(frame.source.width) > self.metadata.image_width
                    || frame.source.y.saturating_add(frame.source.height)
                        > self.metadata.image_height
                {
                    issues.push(format!(
                        "clip {} frame {} is outside the source image",
                        clip.id, frame_index
                    ));
                }
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub fn save(&mut self, repo_root: impl AsRef<Path>) -> Result<(), String> {
        let output = super::publish::resolve_path(repo_root.as_ref(), &self.metadata.output_path);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(&self.metadata)
            .map_err(|error| format!("failed to serialize animation metadata: {error}"))?;
        fs::write(&output, format!("{text}\n"))
            .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
        self.dirty = false;
        Ok(())
    }

    pub fn publish_runtime(
        &mut self,
        repo_root: impl AsRef<Path>,
    ) -> Result<AnimationPublishResult, String> {
        publish_runtime(self, repo_root.as_ref())
    }

    pub fn toggle_selected_event(&mut self, kind: AnimationEventKind) -> Option<bool> {
        let toggled = self.frame_mut()?.toggle_event(kind);
        self.dirty = true;
        Some(toggled)
    }

    fn clamp_selection(&mut self) {
        self.selected_clip = self
            .selected_clip
            .min(self.metadata.clips.len().saturating_sub(1));
        self.selected_frame = self.selected_frame.min(
            self.clip()
                .map_or(0, |clip| clip.frames.len().saturating_sub(1)),
        );
    }
}

fn find_workspace_root(source_path: &Path) -> Option<PathBuf> {
    source_path
        .ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

fn recommended_frame_size(path: &Path, image_width: u32, image_height: u32) -> (u32, u32) {
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("llama") || name.contains("pig") || name.contains("horse") {
        (128, 128)
    } else if name.contains("character")
        || name.contains("player")
        || name.contains("body")
        || name.contains("walk")
        || name.contains("idle")
    {
        (64.min(image_width.max(1)), 64.min(image_height.max(1)))
    } else if image_width.is_multiple_of(64) && image_height.is_multiple_of(96) {
        (64, 96)
    } else if name.contains("chicken") || name.contains("cat") {
        (32, 32)
    } else if image_width.is_multiple_of(64) && image_height.is_multiple_of(64) {
        (64, 64)
    } else if image_width.is_multiple_of(32) && image_height.is_multiple_of(32) {
        (32, 32)
    } else {
        (image_width.clamp(1, 64), image_height.clamp(1, 64))
    }
}

fn cycle_index(current: usize, len: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    if delta >= 0 {
        (current + delta as usize) % len
    } else {
        (current + len - ((-delta) as usize % len)) % len
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_underscore = false;
        } else if !previous_underscore && !output.is_empty() {
            output.push('_');
            previous_underscore = true;
        }
    }
    output.trim_matches('_').to_string()
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_document() -> AnimationDocument {
        AnimationDocument {
            metadata: AnimationDocumentMetadata {
                schema: "havenwild.animation_document.v0_1".to_string(),
                asset_id: "animation/test".to_string(),
                display_name: "Test".to_string(),
                source_path: "test.png".to_string(),
                output_path: "test.hhanim.json".to_string(),
                image_width: 128,
                image_height: 128,
                frame_width: 32,
                frame_height: 32,
                grid_offset: [0, 0],
                clips: vec![AnimationClip::new(1)],
                license: PixelLicense::default(),
            },
            selected_clip: 0,
            selected_frame: 0,
            dirty: false,
        }
    }

    #[test]
    fn auto_slice_creates_complete_grid_row() {
        let mut document = test_document();
        assert_eq!(document.auto_slice_selected_row(2), 4);
        let clip = document.clip().expect("clip");
        assert_eq!(clip.frames.len(), 4);
        assert_eq!(clip.frames[0].source.y, 64);
        assert_eq!(clip.frames[3].source.x, 96);
        assert!(document.validate().is_ok());
    }

    #[test]
    fn recommended_profile_preserves_mixed_cat_sheets_for_manual_authoring() {
        let mut document = test_document();
        document.metadata.source_path = "assets/source/cc0/cat_1.png".to_string();
        assert_eq!(document.apply_recommended_profile(), 0);
        assert_eq!(document.metadata.clips.len(), 1);
    }

    #[test]
    fn recommended_profile_names_shadow_clips_correctly() {
        let mut document = test_document();
        document.metadata.source_path = "assets/source/cc0/chicken_shadow_1.png".to_string();
        document.metadata.image_width = 32;
        document.metadata.image_height = 128;
        assert_eq!(document.apply_recommended_profile(), 4);
        assert_eq!(document.metadata.clips[0].id, "shadow_n");
        assert_eq!(document.metadata.clips[0].frames.len(), 1);
    }

    #[test]
    fn workspace_sources_are_saved_as_portable_relative_paths() {
        let root = std::env::temp_dir().join(format!(
            "havenwild_animation_document_{}",
            std::process::id()
        ));
        let source = root.join("assets/source/cc0/test_walk.png");
        fs::create_dir_all(source.parent().expect("source parent")).expect("create source parent");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("write workspace marker");
        fs::write(&source, b"").expect("write source marker");

        let document = AnimationDocument::load_or_create(
            &source,
            "Test Walk",
            PixelLicense::default(),
            128,
            128,
        )
        .expect("create animation document");

        assert_eq!(
            document.metadata.source_path,
            "assets/source/cc0/test_walk.png"
        );
        let _ = fs::remove_dir_all(root);
    }
}
