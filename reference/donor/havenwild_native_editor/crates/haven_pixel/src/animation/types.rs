use crate::{PixelLicense, PixelSelection};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationDirection {
    South,
    SouthEast,
    East,
    NorthEast,
    North,
    NorthWest,
    West,
    SouthWest,
    None,
}

impl AnimationDirection {
    pub const ALL: [Self; 9] = [
        Self::South,
        Self::SouthEast,
        Self::East,
        Self::NorthEast,
        Self::North,
        Self::NorthWest,
        Self::West,
        Self::SouthWest,
        Self::None,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::South => "S",
            Self::SouthEast => "SE",
            Self::East => "E",
            Self::NorthEast => "NE",
            Self::North => "N",
            Self::NorthWest => "NW",
            Self::West => "W",
            Self::SouthWest => "SW",
            Self::None => "None",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationLoopMode {
    Loop,
    Once,
    PingPong,
}

impl AnimationLoopMode {
    pub const ALL: [Self; 3] = [Self::Loop, Self::Once, Self::PingPong];

    pub fn label(self) -> &'static str {
        match self {
            Self::Loop => "Loop",
            Self::Once => "Once",
            Self::PingPong => "Ping Pong",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationSocketKind {
    MainHand,
    OffHand,
    Tool,
    Head,
    Back,
    Mouth,
    Ground,
    Interaction,
}

impl AnimationSocketKind {
    pub const ALL: [Self; 8] = [
        Self::MainHand,
        Self::OffHand,
        Self::Tool,
        Self::Head,
        Self::Back,
        Self::Mouth,
        Self::Ground,
        Self::Interaction,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::MainHand => "main_hand",
            Self::OffHand => "off_hand",
            Self::Tool => "tool",
            Self::Head => "head",
            Self::Back => "back",
            Self::Mouth => "mouth",
            Self::Ground => "ground",
            Self::Interaction => "interaction",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::MainHand => "Main Hand",
            Self::OffHand => "Off Hand",
            Self::Tool => "Tool",
            Self::Head => "Head",
            Self::Back => "Back",
            Self::Mouth => "Mouth",
            Self::Ground => "Ground",
            Self::Interaction => "Interaction",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationEventKind {
    Footstep,
    Sound,
    Impact,
    Eat,
    Interaction,
    SpawnEffect,
    Custom,
}

impl AnimationEventKind {
    pub const ALL: [Self; 7] = [
        Self::Footstep,
        Self::Sound,
        Self::Impact,
        Self::Eat,
        Self::Interaction,
        Self::SpawnEffect,
        Self::Custom,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Footstep => "footstep",
            Self::Sound => "sound",
            Self::Impact => "impact",
            Self::Eat => "eat",
            Self::Interaction => "interaction",
            Self::SpawnEffect => "spawn_effect",
            Self::Custom => "custom",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Footstep => "Footstep",
            Self::Sound => "Sound",
            Self::Impact => "Impact",
            Self::Eat => "Eat",
            Self::Interaction => "Interaction",
            Self::SpawnEffect => "Spawn Effect",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSocket {
    pub kind: AnimationSocketKind,
    pub position: [i32; 2],
    #[serde(default)]
    pub rotation_degrees: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationEvent {
    pub kind: AnimationEventKind,
    #[serde(default)]
    pub payload: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationFrame {
    pub source: PixelSelection,
    pub duration_ms: u32,
    pub pivot: [i32; 2],
    #[serde(default)]
    pub shadow_offset: [i32; 2],
    #[serde(default)]
    pub sockets: Vec<AnimationSocket>,
    #[serde(default)]
    pub events: Vec<AnimationEvent>,
}

impl AnimationFrame {
    pub fn new(source: PixelSelection, duration_ms: u32) -> Self {
        Self {
            source,
            duration_ms: duration_ms.max(1),
            pivot: [
                source.width as i32 / 2,
                source.height.saturating_sub(4) as i32,
            ],
            shadow_offset: [0, 0],
            sockets: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn socket(&self, kind: AnimationSocketKind) -> Option<&AnimationSocket> {
        self.sockets.iter().find(|socket| socket.kind == kind)
    }

    pub fn set_socket(&mut self, kind: AnimationSocketKind, position: [i32; 2]) {
        if let Some(socket) = self.sockets.iter_mut().find(|socket| socket.kind == kind) {
            socket.position = position;
        } else {
            self.sockets.push(AnimationSocket {
                kind,
                position,
                rotation_degrees: 0,
            });
        }
    }

    pub fn toggle_event(&mut self, kind: AnimationEventKind) -> bool {
        if let Some(index) = self.events.iter().position(|event| event.kind == kind) {
            self.events.remove(index);
            false
        } else {
            self.events.push(AnimationEvent {
                kind,
                payload: String::new(),
            });
            true
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationClip {
    pub id: String,
    pub display_name: String,
    pub direction: AnimationDirection,
    pub loop_mode: AnimationLoopMode,
    #[serde(default = "default_frame_duration_ms")]
    pub default_frame_duration_ms: u32,
    #[serde(default)]
    pub frames: Vec<AnimationFrame>,
}

pub fn default_frame_duration_ms() -> u32 {
    125
}

impl AnimationClip {
    pub fn new(index: usize) -> Self {
        Self {
            id: format!("clip_{index:02}"),
            display_name: format!("Clip {index}"),
            direction: AnimationDirection::South,
            loop_mode: AnimationLoopMode::Loop,
            default_frame_duration_ms: default_frame_duration_ms(),
            frames: Vec::new(),
        }
    }

    pub fn duration_ms(&self) -> u32 {
        self.frames
            .iter()
            .map(|frame| frame.duration_ms.max(1))
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationDocumentMetadata {
    pub schema: String,
    pub asset_id: String,
    pub display_name: String,
    pub source_path: String,
    pub output_path: String,
    pub image_width: u32,
    pub image_height: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub grid_offset: [i32; 2],
    #[serde(default)]
    pub clips: Vec<AnimationClip>,
    pub license: PixelLicense,
}
