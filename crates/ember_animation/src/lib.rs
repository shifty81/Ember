//! Animation publication/runtime metadata built on Ember pixel documents.

use ember_core::StableId;
use ember_pixel::{FrameId, PixelDocument, PlaybackDirection, RepeatMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClipPlayback {
    Once,
    Loop,
    Count(u32),
    PingPong,
    Reverse,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationEvent {
    pub frame: FrameId,
    pub name: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SocketKeyframe {
    pub frame: FrameId,
    pub socket: StableId,
    pub position: [f32; 2],
    pub rotation_degrees: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HitboxKeyframe {
    pub frame: FrameId,
    pub id: StableId,
    pub rect: [i32; 4],
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationClip {
    pub id: StableId,
    pub name: String,
    pub frames: Vec<FrameId>,
    pub durations_ms: Vec<u32>,
    pub playback: ClipPlayback,
    #[serde(default)]
    pub events: Vec<AnimationEvent>,
    #[serde(default)]
    pub sockets: Vec<SocketKeyframe>,
    #[serde(default)]
    pub hitboxes: Vec<HitboxKeyframe>,
}

impl AnimationClip {
    pub fn validate(&self) -> Result<(), String> {
        if self.frames.is_empty() {
            return Err("animation clip has no frames".into());
        }
        if self.frames.len() != self.durations_ms.len() {
            return Err("animation frame and duration counts differ".into());
        }
        if self.durations_ms.contains(&0) {
            return Err("animation frame duration cannot be zero".into());
        }
        for event in &self.events {
            if !self.frames.contains(&event.frame) {
                return Err(format!(
                    "animation event {} references frame outside clip",
                    event.name
                ));
            }
        }
        for socket in &self.sockets {
            if !self.frames.contains(&socket.frame) {
                return Err(format!(
                    "socket {} references frame outside clip",
                    socket.socket
                ));
            }
        }
        for hitbox in &self.hitboxes {
            if !self.frames.contains(&hitbox.frame) {
                return Err(format!(
                    "hitbox {} references frame outside clip",
                    hitbox.id
                ));
            }
            if hitbox.rect[2] < 0 || hitbox.rect[3] < 0 {
                return Err(format!("hitbox {} has negative size", hitbox.id));
            }
        }
        Ok(())
    }
}

pub fn clip_from_tag(
    document: &PixelDocument,
    tag_name: &str,
    clip_id: StableId,
) -> Result<AnimationClip, String> {
    document.validate()?;
    let tag = document
        .frame_tags
        .iter()
        .find(|tag| tag.name == tag_name)
        .ok_or_else(|| format!("frame tag {tag_name} was not found"))?;
    let start = document
        .frames
        .iter()
        .position(|frame| frame.id == tag.from_frame)
        .ok_or_else(|| "tag start frame was not found".to_string())?;
    let end = document
        .frames
        .iter()
        .position(|frame| frame.id == tag.to_frame)
        .ok_or_else(|| "tag end frame was not found".to_string())?;
    if start > end {
        return Err("frame tag start occurs after end".into());
    }
    let selected = &document.frames[start..=end];
    let frames = selected.iter().map(|frame| frame.id).collect::<Vec<_>>();
    let durations_ms = selected
        .iter()
        .map(|frame| frame.duration_ms.max(1))
        .collect::<Vec<_>>();
    let playback = match (tag.direction, tag.repeat) {
        (PlaybackDirection::PingPong, _) => ClipPlayback::PingPong,
        (PlaybackDirection::Reverse, _) => ClipPlayback::Reverse,
        (PlaybackDirection::Forward, RepeatMode::Loop) => ClipPlayback::Loop,
        (PlaybackDirection::Forward, RepeatMode::Count(count)) => ClipPlayback::Count(count),
        (PlaybackDirection::Forward, RepeatMode::Once) => ClipPlayback::Once,
    };
    let clip = AnimationClip {
        id: clip_id,
        name: tag.name.clone(),
        frames,
        durations_ms,
        playback,
        events: Vec::new(),
        sockets: Vec::new(),
        hitboxes: Vec::new(),
    };
    clip.validate()?;
    Ok(clip)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteSheetLayout {
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub padding: u32,
}

impl SpriteSheetLayout {
    pub fn validate(self) -> Result<(), String> {
        if self.frame_width == 0 || self.frame_height == 0 {
            return Err("sprite-sheet frame dimensions must be non-zero".into());
        }
        if self.columns == 0 || self.rows == 0 {
            return Err("sprite-sheet rows/columns must be non-zero".into());
        }
        Ok(())
    }

    pub fn sheet_size(self) -> Result<[u32; 2], String> {
        self.validate()?;
        let width = self
            .frame_width
            .checked_mul(self.columns)
            .and_then(|value| {
                value.checked_add(self.padding.saturating_mul(self.columns.saturating_sub(1)))
            })
            .ok_or_else(|| "sprite-sheet width overflow".to_string())?;
        let height = self
            .frame_height
            .checked_mul(self.rows)
            .and_then(|value| {
                value.checked_add(self.padding.saturating_mul(self.rows.saturating_sub(1)))
            })
            .ok_or_else(|| "sprite-sheet height overflow".to_string())?;
        Ok([width, height])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_pixel::{FrameTag, PixelDocument};

    #[test]
    fn frame_tag_publishes_clip() {
        let mut document = PixelDocument::new(16, 16);
        let second = document.add_frame(120);
        document.frame_tags.push(FrameTag {
            name: "walk".into(),
            from_frame: 1,
            to_frame: second,
            direction: PlaybackDirection::Forward,
            repeat: RepeatMode::Loop,
        });
        let clip = clip_from_tag(
            &document,
            "walk",
            StableId::new("animation", "walk").unwrap(),
        )
        .unwrap();
        assert_eq!(clip.frames.len(), 2);
        assert_eq!(clip.playback, ClipPlayback::Loop);
    }
}
