use super::animation_studio_render::*;
use super::render_helpers::*;
use super::sprite_workspace::*;
use super::*;
use haven_assets::asset_intake::repo_root_dir;
use haven_pixel::{
    scan_pixel_library, AnimationDocument, AnimationEventKind, AnimationLoopMode,
    AnimationSocketKind, PixelLibraryEntry, PixelSelection,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationPlacementMode {
    None,
    Pivot,
    Shadow,
    Socket,
}

impl AnimationPlacementMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::None => "Select",
            Self::Pivot => "Place Pivot",
            Self::Shadow => "Place Shadow",
            Self::Socket => "Place Socket",
        }
    }
}

pub(crate) struct AnimationStudioState {
    pub library: Vec<PixelLibraryEntry>,
    pub library_loaded: bool,
    pub selected_entry: usize,
    pub library_offset: usize,
    pub document: Option<AnimationDocument>,
    pub texture: Option<Texture2D>,
    pub source_cell: [u32; 2],
    pub playing: bool,
    pub playback_elapsed_ms: f32,
    pub playback_direction: i32,
    pub onion_skin: bool,
    pub show_source_grid: bool,
    pub placement_mode: AnimationPlacementMode,
    pub socket_kind_index: usize,
    pub event_kind_index: usize,
}

impl AnimationStudioState {
    pub(crate) fn new() -> Self {
        Self {
            // Keep editor startup independent from filesystem-wide animation
            // discovery. The user explicitly requests a scan from this tab.
            library: Vec::new(),
            library_loaded: false,
            selected_entry: 0,
            library_offset: 0,
            document: None,
            texture: None,
            source_cell: [0, 0],
            playing: false,
            playback_elapsed_ms: 0.0,
            playback_direction: 1,
            onion_skin: true,
            show_source_grid: true,
            placement_mode: AnimationPlacementMode::None,
            socket_kind_index: 0,
            event_kind_index: 0,
        }
    }

    pub(crate) fn refresh_library(&mut self) -> usize {
        self.library = animation_library();
        self.library_loaded = true;
        self.selected_entry = self
            .selected_entry
            .min(self.library.len().saturating_sub(1));
        self.library_offset = self.library_offset.min(self.selected_entry);
        self.library.len()
    }

    pub(crate) fn load_selected(&mut self) -> Result<String, String> {
        let entry = self
            .library
            .get(self.selected_entry)
            .ok_or_else(|| "Animation source library is empty".to_string())?
            .clone();
        let image = image::open(&entry.path)
            .map_err(|error| format!("failed to load {}: {error}", entry.path.display()))?
            .to_rgba8();
        if image.width() > u16::MAX as u32 || image.height() > u16::MAX as u32 {
            return Err("source image exceeds Macroquad texture limits".to_string());
        }
        let texture =
            Texture2D::from_rgba8(image.width() as u16, image.height() as u16, image.as_raw());
        texture.set_filter(FilterMode::Nearest);
        let document = AnimationDocument::load_or_create(
            &entry.path,
            &entry.display_name,
            entry.license,
            image.width(),
            image.height(),
        )?;
        self.document = Some(document);
        self.texture = Some(texture);
        self.source_cell = [0, 0];
        self.playing = false;
        self.playback_elapsed_ms = 0.0;
        Ok(format!(
            "Opened {} for animation authoring",
            entry.display_name
        ))
    }

    pub(crate) fn selected_socket_kind(&self) -> AnimationSocketKind {
        AnimationSocketKind::ALL[self.socket_kind_index % AnimationSocketKind::ALL.len()]
    }

    pub(crate) fn selected_event_kind(&self) -> AnimationEventKind {
        AnimationEventKind::ALL[self.event_kind_index % AnimationEventKind::ALL.len()]
    }

    pub(crate) fn selected_source(&self) -> Option<PixelSelection> {
        let document = self.document.as_ref()?;
        let frame_width = document.metadata.frame_width.max(1);
        let frame_height = document.metadata.frame_height.max(1);
        let x = document.metadata.grid_offset[0].max(0) as u32
            + self.source_cell[0].saturating_mul(frame_width);
        let y = document.metadata.grid_offset[1].max(0) as u32
            + self.source_cell[1].saturating_mul(frame_height);
        (x.saturating_add(frame_width) <= document.metadata.image_width
            && y.saturating_add(frame_height) <= document.metadata.image_height)
            .then_some(PixelSelection {
                x,
                y,
                width: frame_width,
                height: frame_height,
            })
    }

    pub(crate) fn advance_playback(&mut self, delta_seconds: f32) {
        if !self.playing {
            return;
        }
        let Some(document) = self.document.as_mut() else {
            self.playing = false;
            return;
        };
        let Some(clip) = document.clip() else {
            self.playing = false;
            return;
        };
        if clip.frames.is_empty() {
            self.playing = false;
            return;
        }
        self.playback_elapsed_ms += delta_seconds.max(0.0) * 1000.0;
        loop {
            let duration = document
                .frame()
                .map_or(125.0, |frame| frame.duration_ms.max(1) as f32);
            if self.playback_elapsed_ms < duration {
                break;
            }
            self.playback_elapsed_ms -= duration;
            let frame_count = document.clip().map_or(0, |clip| clip.frames.len());
            if frame_count == 0 {
                self.playing = false;
                break;
            }
            let mode = document
                .clip()
                .map_or(AnimationLoopMode::Loop, |clip| clip.loop_mode);
            match mode {
                AnimationLoopMode::Loop => {
                    document.selected_frame = (document.selected_frame + 1) % frame_count;
                }
                AnimationLoopMode::Once => {
                    if document.selected_frame + 1 >= frame_count {
                        document.selected_frame = frame_count - 1;
                        self.playing = false;
                        break;
                    }
                    document.selected_frame += 1;
                }
                AnimationLoopMode::PingPong => {
                    if frame_count == 1 {
                        document.selected_frame = 0;
                    } else if self.playback_direction > 0 {
                        if document.selected_frame + 1 >= frame_count {
                            self.playback_direction = -1;
                            document.selected_frame = frame_count - 2;
                        } else {
                            document.selected_frame += 1;
                        }
                    } else if document.selected_frame == 0 {
                        self.playback_direction = 1;
                        document.selected_frame = 1;
                    } else {
                        document.selected_frame -= 1;
                    }
                }
            }
        }
    }
}

fn animation_library() -> Vec<PixelLibraryEntry> {
    scan_pixel_library(repo_root_dir())
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            let relative = entry.relative_path.to_ascii_lowercase();
            if relative.starts_with("assets/generated/animations/") {
                return false;
            }
            let stem = entry
                .path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            [
                "walk",
                "eat",
                "shadow",
                "character",
                "player",
                "cat",
                "horse",
                "llama",
                "pig",
                "chicken",
            ]
            .iter()
            .any(|needle| stem.contains(needle))
                || matches!(stem.as_str(), "base" | "base_1")
                || stem.contains("base_character")
        })
        .collect()
}

impl EditorApp {
    pub(crate) fn draw_animation_library(&self, rect: Rect) {
        draw_editor_text("Animation Sources", rect.x, rect.y + 20.0, 18.0, TEXT);
        draw_editor_widget(animation_refresh_rect(rect), "Rescan", false);
        draw_editor_widget(animation_open_rect(rect), "Open", false);
        if self.animation_studio.library.is_empty() {
            let message = if self.animation_studio.library_loaded {
                "No project character or animal sheets matched the animation filter."
            } else {
                "Animation source scan deferred for fast startup. Click Rescan when this workspace is needed."
            };
            draw_wrapped(message, rect.x, rect.y + 86.0, rect.w, 15.0, MUTED);
            return;
        }
        let visible = ((rect.h - 92.0) / 44.0).floor().max(1.0) as usize;
        for row in 0..visible {
            let index = self.animation_studio.library_offset + row;
            let Some(entry) = self.animation_studio.library.get(index) else {
                break;
            };
            let row_rect = animation_library_row_rect(rect, row);
            draw_rectangle(
                row_rect.x,
                row_rect.y,
                row_rect.w,
                row_rect.h,
                if index == self.animation_studio.selected_entry {
                    ACCENT
                } else {
                    CONTROL_BG
                },
            );
            draw_rectangle_lines(
                row_rect.x, row_rect.y, row_rect.w, row_rect.h, 1.0, PANEL_EDGE,
            );
            draw_scissored_text(
                &entry.display_name,
                row_rect.x + 8.0,
                row_rect.y + 17.0,
                row_rect.w - 16.0,
                15.0,
                TEXT,
            );
            draw_scissored_text(
                entry.source.label(),
                row_rect.x + 8.0,
                row_rect.y + 34.0,
                row_rect.w - 16.0,
                12.0,
                MUTED,
            );
        }
    }

    pub(crate) fn draw_animation_workspace(&self, host: Rect) {
        let toolbar = animation_toolbar_rect(host);
        draw_animation_toolbar(self, toolbar);
        let source_rect = animation_source_rect(host);
        let preview_rect = animation_preview_rect(host);
        let timeline_rect = animation_timeline_rect(host);
        draw_sprite_panel(source_rect, "Source Sheet", Some("Frame source"));
        draw_sprite_panel(preview_rect, "Clip Preview", Some("Shared sprite canvas"));
        draw_sprite_panel(timeline_rect, "Timeline", Some("Frames / events / sockets"));
        let (Some(document), Some(texture)) = (
            self.animation_studio.document.as_ref(),
            self.animation_studio.texture.as_ref(),
        ) else {
            draw_wrapped(
                "Open a character or animal sprite sheet. Animation Studio will preserve the source image while authoring clip, frame, pivot, event, shadow, and socket metadata.",
                source_rect.x + 18.0,
                source_rect.y + 48.0,
                source_rect.w - 36.0,
                17.0,
                TEXT,
            );
            return;
        };
        draw_animation_source_sheet(self, document, texture, source_rect);
        draw_animation_preview(self, document, texture, preview_rect);
        draw_animation_timeline(self, document, texture, timeline_rect);
    }

    pub(crate) fn draw_animation_inspector(&self, rect: Rect) {
        let Some(document) = self.animation_studio.document.as_ref() else {
            draw_editor_text(
                "No animation source open",
                rect.x,
                rect.y + 22.0,
                18.0,
                MUTED,
            );
            return;
        };
        draw_scissored_text(
            &document.metadata.display_name,
            rect.x,
            rect.y + 20.0,
            rect.w,
            21.0,
            TEXT,
        );
        draw_scissored_text(
            &format!(
                "{}x{} | frame {}x{} | {}",
                document.metadata.image_width,
                document.metadata.image_height,
                document.metadata.frame_width,
                document.metadata.frame_height,
                if document.dirty { "dirty" } else { "saved" }
            ),
            rect.x,
            rect.y + 44.0,
            rect.w,
            14.0,
            MUTED,
        );

        draw_editor_text("Clips", rect.x, rect.y + 76.0, 18.0, TEXT);
        draw_editor_widget(animation_clip_prev_rect(rect), "< Clip", false);
        draw_editor_widget(animation_clip_next_rect(rect), "Clip >", false);
        draw_editor_widget(animation_clip_add_rect(rect), "+ Clip", false);
        draw_editor_widget(animation_clip_delete_rect(rect), "Delete", false);
        if let Some(clip) = document.clip() {
            draw_scissored_text(
                &format!(
                    "{} | {} | {} | {} frames",
                    clip.display_name,
                    clip.direction.label(),
                    clip.loop_mode.label(),
                    clip.frames.len()
                ),
                rect.x,
                rect.y + 146.0,
                rect.w,
                15.0,
                TEXT,
            );
        }
        draw_editor_widget(animation_direction_rect(rect), "Direction", false);
        draw_editor_widget(animation_loop_rect(rect), "Loop Mode", false);
        draw_editor_widget(animation_profile_rect(rect), "Apply Sheet Preset", false);
        draw_editor_widget(animation_slice_row_rect(rect), "Slice Selected Row", false);
        draw_editor_widget(animation_frame_width_down_rect(rect), "W -", false);
        draw_editor_widget(animation_frame_width_up_rect(rect), "W +", false);
        draw_editor_widget(animation_frame_height_down_rect(rect), "H -", false);
        draw_editor_widget(animation_frame_height_up_rect(rect), "H +", false);

        draw_editor_text("Frame", rect.x, rect.y + 264.0, 18.0, TEXT);
        let frame_summary = document.frame().map_or_else(
            || "No frame selected".to_string(),
            |frame| {
                format!(
                    "{} / {} | {} ms | src {},{} {}x{}",
                    document.selected_frame + 1,
                    document.clip().map_or(0, |clip| clip.frames.len()),
                    frame.duration_ms,
                    frame.source.x,
                    frame.source.y,
                    frame.source.width,
                    frame.source.height
                )
            },
        );
        draw_scissored_text(&frame_summary, rect.x, rect.y + 288.0, rect.w, 14.0, TEXT);
        draw_editor_widget(animation_duration_down_rect(rect), "- Duration", false);
        draw_editor_widget(animation_duration_up_rect(rect), "+ Duration", false);
        draw_editor_widget(
            animation_pivot_mode_rect(rect),
            AnimationPlacementMode::Pivot.label(),
            self.animation_studio.placement_mode == AnimationPlacementMode::Pivot,
        );
        draw_editor_widget(animation_pivot_bottom_rect(rect), "Pivot Bottom", false);
        draw_editor_widget(
            animation_shadow_mode_rect(rect),
            "Place Shadow",
            self.animation_studio.placement_mode == AnimationPlacementMode::Shadow,
        );
        draw_editor_widget(animation_shadow_reset_rect(rect), "Reset Shadow", false);

        draw_editor_text("Sockets and Events", rect.x, rect.y + 420.0, 18.0, TEXT);
        let socket = self.animation_studio.selected_socket_kind();
        draw_editor_widget(animation_socket_prev_rect(rect), "<", false);
        draw_editor_widget(animation_socket_next_rect(rect), ">", false);
        draw_scissored_text(
            socket.label(),
            rect.x + 42.0,
            rect.y + 456.0,
            152.0,
            15.0,
            TEXT,
        );
        draw_editor_widget(
            animation_socket_mode_rect(rect),
            "Place Socket",
            self.animation_studio.placement_mode == AnimationPlacementMode::Socket,
        );
        let event = self.animation_studio.selected_event_kind();
        draw_editor_widget(animation_event_prev_rect(rect), "<", false);
        draw_editor_widget(animation_event_next_rect(rect), ">", false);
        draw_scissored_text(
            event.label(),
            rect.x + 42.0,
            rect.y + 500.0,
            152.0,
            15.0,
            TEXT,
        );
        draw_editor_widget(animation_event_toggle_rect(rect), "Toggle Event", false);
        draw_editor_widget(
            animation_edit_pixels_rect(rect),
            "Edit Selected Frame in Pixel Studio",
            false,
        );

        draw_editor_text("Output", rect.x, rect.y + 552.0, 18.0, TEXT);
        draw_editor_widget(animation_save_rect(rect), "Save Animation", false);
        draw_editor_widget(animation_publish_rect(rect), "Publish Runtime", false);
        let validation = match document.validate() {
            Ok(()) => "Ready to publish".to_string(),
            Err(issues) => format!("{} issue(s): {}", issues.len(), issues[0]),
        };
        draw_wrapped(
            &validation,
            rect.x,
            rect.y + 612.0,
            rect.w,
            14.0,
            if document.validate().is_ok() {
                GOOD
            } else {
                WARN
            },
        );
        draw_wrapped(
            "Workflow: choose a source cell, add it to the clip, order frames, set pivot/sockets/events, press E to edit the selected frame in Pixel Studio, then save and publish runtime metadata.",
            rect.x,
            rect.y + 668.0,
            rect.w,
            13.0,
            MUTED,
        );
    }

    pub(crate) fn save_animation_document(&mut self) {
        let Some(document) = self.animation_studio.document.as_mut() else {
            self.status_message = "No animation document is open".to_string();
            return;
        };
        self.status_message = match document.save(repo_root_dir()) {
            Ok(()) => format!("Saved {}", document.metadata.output_path),
            Err(error) => format!("Animation save failed: {error}"),
        };
    }

    pub(crate) fn publish_animation_document(&mut self) {
        let Some(document) = self.animation_studio.document.as_mut() else {
            self.status_message = "No animation document is open".to_string();
            return;
        };
        self.status_message = match document.publish_runtime(repo_root_dir()) {
            Ok(result) => format!(
                "Published {} clips to {} using {}",
                result.clip_count, result.runtime_metadata_path, result.generated_image_path
            ),
            Err(error) => format!("Animation publish failed: {error}"),
        };
    }
}
