use super::{ObjectFootprint, ObjectKind, TileKind, TILE_SIZE};
use crate::{ObjectId, SceneReference, StampInstanceId, TransitionId, ZoneKind};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StablePlaceableAssetRef {
    pub pack_id: String,
    pub category: String,
    pub asset_id: String,
    pub source_id: String,
    pub variant_id: Option<String>,
}

impl StablePlaceableAssetRef {
    pub fn new(
        pack_id: impl Into<String>,
        category: impl Into<String>,
        asset_id: impl Into<String>,
        source_id: impl Into<String>,
        variant_id: Option<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            category: category.into(),
            asset_id: asset_id.into(),
            source_id: source_id.into(),
            variant_id,
        }
    }

    pub fn stable_key(&self) -> String {
        format!(
            "{}::{}::{}::{}::{}",
            self.pack_id,
            self.category,
            self.asset_id,
            self.source_id,
            self.variant_id.as_deref().unwrap_or("-")
        )
    }

    /// Temporary compatibility reference used while loading authored scene JSON.
    /// The haven_assets published registry resolves this alias and rewrites it to
    /// the canonical persistent ref before normal save/runtime ownership.
    pub fn from_scene_asset_alias(asset_id: impl Into<String>) -> Self {
        Self::new(
            "__scene_asset_alias__",
            "world_asset_alias",
            asset_id,
            "authored_scene",
            None,
        )
    }

    pub fn scene_asset_alias_id(&self) -> Option<&str> {
        (self.pack_id == "__scene_asset_alias__"
            && self.category == "world_asset_alias"
            && self.source_id == "authored_scene")
            .then_some(self.asset_id.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
    pub x: i32,
    pub y: i32,
    pub footprint: ObjectFootprint,
}

impl PlacedObject {
    pub fn new(kind: ObjectKind, x: i32, y: i32) -> Self {
        Self {
            id: ObjectId::from_raw(0),
            kind,
            x,
            y,
            footprint: kind.footprint_for_cell(x, y),
        }
    }

    pub fn with_id(id: ObjectId, kind: ObjectKind, x: i32, y: i32) -> Self {
        Self {
            id,
            kind,
            x,
            y,
            footprint: kind.footprint_for_cell(x, y),
        }
    }

    pub fn with_footprint(kind: ObjectKind, x: i32, y: i32, footprint: ObjectFootprint) -> Self {
        Self {
            id: ObjectId::from_raw(0),
            kind,
            x,
            y,
            footprint,
        }
    }

    pub fn with_id_and_footprint(
        id: ObjectId,
        kind: ObjectKind,
        x: i32,
        y: i32,
        footprint: ObjectFootprint,
    ) -> Self {
        Self {
            id,
            kind,
            x,
            y,
            footprint,
        }
    }

    pub fn visual_rect(self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.visual_offset_x,
            self.y + self.footprint.visual_offset_y,
            self.footprint.visual_w.max(1),
            self.footprint.visual_h.max(1),
        )
    }

    pub fn collision_rect(self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.collision_offset_x,
            self.y + self.footprint.collision_offset_y,
            self.footprint.collision_w.max(0),
            self.footprint.collision_h.max(0),
        )
    }

    pub fn interaction_rect(self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.interaction_offset_x,
            self.y + self.footprint.interaction_offset_y,
            self.footprint.interaction_w.max(0),
            self.footprint.interaction_h.max(0),
        )
    }

    pub fn contains_visual_tile(self, x: i32, y: i32) -> bool {
        rect_contains(self.visual_rect(), x, y)
    }

    pub fn contains_collision_tile(self, x: i32, y: i32) -> bool {
        rect_contains(self.collision_rect(), x, y)
    }

    pub fn contains_interaction_tile(self, x: i32, y: i32) -> bool {
        rect_contains(self.interaction_rect(), x, y)
    }

    pub fn contains_tile(self, x: i32, y: i32) -> bool {
        self.contains_visual_tile(x, y)
            || self.contains_collision_tile(x, y)
            || self.contains_interaction_tile(x, y)
    }

    pub fn blocks_tile(self, x: i32, y: i32) -> bool {
        self.footprint.blocks_movement && self.contains_collision_tile(x, y)
    }

    pub fn sort_y(self) -> f32 {
        let (_, y, _, h) = self.collision_rect();
        (y + h).max(self.y + 1) as f32 * TILE_SIZE
    }

    pub fn footprint_label(self) -> String {
        let (vx, vy, vw, vh) = self.visual_rect();
        let (cx, cy, cw, ch) = self.collision_rect();
        let (ix, iy, iw, ih) = self.interaction_rect();
        format!(
            "visual {}x{} at {},{}; collision {}x{} at {},{}; interaction {}x{} at {},{}",
            vw, vh, vx, vy, cw, ch, cx, cy, iw, ih, ix, iy
        )
    }

    pub fn placement_label(self) -> String {
        format!(
            "{} anchor {},{} | {}",
            self.kind.label(),
            self.x,
            self.y,
            self.footprint_label()
        )
    }
}

fn rect_contains(rect: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    let (rx, ry, rw, rh) = rect;
    rw > 0 && rh > 0 && x >= rx && y >= ry && x < rx + rw && y < ry + rh
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedStamp {
    pub id: StampInstanceId,
    pub stamp_key: String,
    pub x: i32,
    pub y: i32,
    pub footprint: ObjectFootprint,
}

impl PlacedStamp {
    pub fn new(stamp_key: impl Into<String>, x: i32, y: i32, footprint: ObjectFootprint) -> Self {
        Self {
            id: StampInstanceId::from_raw(0),
            stamp_key: normalize_stamp_key(stamp_key.into()),
            x,
            y,
            footprint,
        }
    }

    pub fn with_id(
        id: StampInstanceId,
        stamp_key: impl Into<String>,
        x: i32,
        y: i32,
        footprint: ObjectFootprint,
    ) -> Self {
        Self {
            id,
            stamp_key: normalize_stamp_key(stamp_key.into()),
            x,
            y,
            footprint,
        }
    }

    pub fn visual_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.visual_offset_x,
            self.y + self.footprint.visual_offset_y,
            self.footprint.visual_w.max(1),
            self.footprint.visual_h.max(1),
        )
    }

    pub fn collision_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.collision_offset_x,
            self.y + self.footprint.collision_offset_y,
            self.footprint.collision_w.max(0),
            self.footprint.collision_h.max(0),
        )
    }

    pub fn interaction_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.x + self.footprint.interaction_offset_x,
            self.y + self.footprint.interaction_offset_y,
            self.footprint.interaction_w.max(0),
            self.footprint.interaction_h.max(0),
        )
    }

    pub fn contains_visual_tile(&self, x: i32, y: i32) -> bool {
        rect_contains(self.visual_rect(), x, y)
    }

    pub fn contains_collision_tile(&self, x: i32, y: i32) -> bool {
        rect_contains(self.collision_rect(), x, y)
    }

    pub fn contains_interaction_tile(&self, x: i32, y: i32) -> bool {
        rect_contains(self.interaction_rect(), x, y)
    }

    pub fn contains_tile(&self, x: i32, y: i32) -> bool {
        self.contains_visual_tile(x, y)
            || self.contains_collision_tile(x, y)
            || self.contains_interaction_tile(x, y)
    }

    pub fn blocks_tile(&self, x: i32, y: i32) -> bool {
        self.footprint.blocks_movement && self.contains_collision_tile(x, y)
    }

    pub fn sort_y(&self) -> f32 {
        let (_, y, _, h) = self.collision_rect();
        (y + h).max(self.y + 1) as f32 * TILE_SIZE
    }

    pub fn placement_label(&self) -> String {
        format!(
            "{} ({}) anchor {},{}",
            self.stamp_key, self.id, self.x, self.y
        )
    }
}

fn normalize_stamp_key(value: String) -> String {
    let normalized = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        "stamp_asset".to_string()
    } else {
        normalized
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildTool {
    Inspect,
    Floor(TileKind),
    Object(ObjectKind),
    Erase,
    Hoe,
    GreenhouseZone,
    Zone(ZoneKind),
    Transition,
}

impl BuildTool {
    pub fn label(self) -> &'static str {
        match self {
            BuildTool::Inspect => "Inspect",
            BuildTool::Floor(tile) => tile.label(),
            BuildTool::Object(kind) => kind.label(),
            BuildTool::Erase => "Erase",
            BuildTool::Hoe => "Hoe",
            BuildTool::GreenhouseZone => "Greenhouse Zone",
            BuildTool::Zone(zone) => zone.label(),
            BuildTool::Transition => "Transition",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub id: TransitionId,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub target: SceneReference,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub label: String,
}

impl Transition {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
