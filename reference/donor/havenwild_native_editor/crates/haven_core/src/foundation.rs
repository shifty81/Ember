use std::collections::BTreeMap;

use crate::{
    collision_class, terrain_gameplay_profile, AutotileOverride, BuildingClass, BuildingSupport,
    CollisionClass, CollisionReason, CollisionResult, FarmingClass, FootstepSurface, ObjectId,
    SceneBiome, SceneId, SceneKind, StampInstanceId, TransitionId, ZoneKind,
};

mod authored_entities;
mod map_core;
mod map_serialization;
mod object_footprint;
mod scene_size_migration;
mod scene_visual_bounds;
mod scene_world;
mod seed_transitions;
mod spatial_validation;
mod starter_generation;
mod starter_scene_rules;
mod tile_object_catalog;
mod ui_layout;

pub use authored_entities::*;
pub use object_footprint::*;
use seed_transitions::transition;
use spatial_validation::{objects_overlap, spatial_footprints_overlap, validate_footprint_bounds};

pub use scene_world::{GameWorld, SceneMap};
pub use tile_object_catalog::*;
pub use ui_layout::*;

pub const TILE_SIZE: f32 = 32.0;
pub const LEGACY_MAP_W: usize = 48;
pub const LEGACY_MAP_H: usize = 32;
pub const MAP_W: usize = 96;
pub const MAP_H: usize = 64;

pub(crate) use scene_size_migration::parse_tavern_map_dimensions;
pub use scene_size_migration::scene_dimension_offset;

#[path = "worldgen_exporter.rs"]
pub mod worldgen_exporter;
#[path = "worldgen_loader.rs"]
pub mod worldgen_loader;
pub use worldgen_exporter::{export_worldgen_pack_to_path, WorldgenExportReport};
pub use worldgen_loader::{load_worldgen_pack_from_path, WorldgenLoadReport};

#[path = "addons/grid_2p5d.rs"]
pub mod grid_2p5d_impl;
#[path = "addons/worldgen_layered.rs"]
pub mod worldgen_layered_impl;

pub mod addons {
    pub use super::grid_2p5d_impl as grid_2p5d;
    pub use super::worldgen_layered_impl as worldgen_layered;
}

pub const STRUCTURAL_LEVEL_AUTO: u8 = u8::MAX;
pub const MAX_STRUCTURAL_LEVEL: u8 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TavernMap {
    pub tiles: Vec<TileKind>,
    /// Smooth geological/hydrology height. This is not editor-authored platform height.
    pub heights: Vec<u8>,
    /// Explicit tile-authored structural levels. `STRUCTURAL_LEVEL_AUTO` preserves
    /// legacy/generated fallback behavior; 0..=MAX_STRUCTURAL_LEVEL are authored.
    pub structural_levels: Vec<u8>,
    pub objects: Vec<PlacedObject>,
    pub stamps: Vec<PlacedStamp>,
    pub object_asset_refs: BTreeMap<ObjectId, StablePlaceableAssetRef>,
    pub object_states: BTreeMap<ObjectId, String>,
}

pub struct AssetRecord {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub output: String,
    pub license: String,
    pub author: String,
    pub tags: Vec<String>,
}

impl AssetRecord {
    pub fn missing_license(&self) -> bool {
        self.license.trim().is_empty() || self.license.eq_ignore_ascii_case("unknown")
    }
}

#[derive(Clone, Debug)]
pub struct PlacementIssue {
    pub x: i32,
    pub y: i32,
    pub reason: String,
}

impl PlacementIssue {
    pub fn new(x: i32, y: i32, reason: impl Into<String>) -> Self {
        Self {
            x,
            y,
            reason: reason.into(),
        }
    }

    pub fn label(&self) -> String {
        format!("{},{}: {}", self.x, self.y, self.reason)
    }
}

impl TavernMap {
    pub fn object_at(&self, x: i32, y: i32) -> Option<usize> {
        self.objects
            .iter()
            .rposition(|object| object.contains_tile(x, y))
    }

    pub fn object_id_at(&self, x: i32, y: i32) -> Option<ObjectId> {
        self.object_at(x, y)
            .and_then(|index| self.objects.get(index))
            .map(|object| object.id)
    }

    pub fn object_index(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|object| object.id == id)
    }

    pub fn object(&self, id: ObjectId) -> Option<&PlacedObject> {
        self.object_index(id)
            .and_then(|index| self.objects.get(index))
    }

    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut PlacedObject> {
        let index = self.object_index(id)?;
        self.objects.get_mut(index)
    }

    pub fn next_object_id(&self) -> ObjectId {
        ObjectId::next_after(self.objects.iter().map(|object| object.id))
    }

    pub fn stamp_at(&self, x: i32, y: i32) -> Option<usize> {
        self.stamps
            .iter()
            .rposition(|stamp| stamp.contains_tile(x, y))
    }

    pub fn stamp_id_at(&self, x: i32, y: i32) -> Option<StampInstanceId> {
        self.stamp_at(x, y)
            .and_then(|index| self.stamps.get(index))
            .map(|stamp| stamp.id)
    }

    pub fn stamp_index(&self, id: StampInstanceId) -> Option<usize> {
        self.stamps.iter().position(|stamp| stamp.id == id)
    }

    pub fn stamp(&self, id: StampInstanceId) -> Option<&PlacedStamp> {
        self.stamp_index(id)
            .and_then(|index| self.stamps.get(index))
    }

    pub fn stamp_mut(&mut self, id: StampInstanceId) -> Option<&mut PlacedStamp> {
        let index = self.stamp_index(id)?;
        self.stamps.get_mut(index)
    }

    pub fn next_stamp_id(&self) -> StampInstanceId {
        StampInstanceId::next_after(self.stamps.iter().map(|stamp| stamp.id))
    }

    pub fn blocking_stamp_at(&self, x: i32, y: i32) -> Option<usize> {
        self.stamps
            .iter()
            .rposition(|stamp| stamp.blocks_tile(x, y))
    }

    pub fn placement_issues_for_stamp(&self, stamp: &PlacedStamp) -> Vec<PlacementIssue> {
        self.placement_issues_for_stamp_excluding(stamp, None)
    }

    pub fn placement_issues_for_stamp_excluding(
        &self,
        stamp: &PlacedStamp,
        ignore_index: Option<usize>,
    ) -> Vec<PlacementIssue> {
        let mut issues = Vec::new();
        if Self::idx(stamp.x, stamp.y).is_none() {
            issues.push(PlacementIssue::new(
                stamp.x,
                stamp.y,
                "stamp anchor outside scene",
            ));
            return issues;
        }
        validate_footprint_bounds(self, stamp.x, stamp.y, stamp.footprint, &mut issues);

        for object in &self.objects {
            if spatial_footprints_overlap(
                stamp.footprint,
                stamp.x,
                stamp.y,
                object.footprint,
                object.x,
                object.y,
            ) {
                issues.push(PlacementIssue::new(
                    stamp.x,
                    stamp.y,
                    format!("overlaps existing {}", object.kind.label()),
                ));
            }
        }
        for (index, other) in self.stamps.iter().enumerate() {
            if Some(index) == ignore_index {
                continue;
            }
            if spatial_footprints_overlap(
                stamp.footprint,
                stamp.x,
                stamp.y,
                other.footprint,
                other.x,
                other.y,
            ) {
                issues.push(PlacementIssue::new(
                    stamp.x,
                    stamp.y,
                    format!("overlaps existing stamp {}", other.stamp_key),
                ));
            }
        }
        issues
    }

    pub fn place_stamp(
        &mut self,
        mut stamp: PlacedStamp,
    ) -> Result<StampInstanceId, Vec<PlacementIssue>> {
        let issues = self.placement_issues_for_stamp(&stamp);
        if !issues.is_empty() {
            return Err(issues);
        }
        if !stamp.id.is_assigned() || self.stamp(stamp.id).is_some() {
            stamp.id = self.next_stamp_id();
        }
        let id = stamp.id;
        self.stamps.push(stamp);
        Ok(id)
    }

    pub fn remove_stamp(&mut self, id: StampInstanceId) -> Option<PlacedStamp> {
        let index = self.stamp_index(id)?;
        Some(self.stamps.remove(index))
    }

    pub fn remove_stamp_at(&mut self, x: i32, y: i32) -> Option<PlacedStamp> {
        let index = self.stamp_at(x, y)?;
        Some(self.stamps.remove(index))
    }

    pub fn move_stamp(
        &mut self,
        id: StampInstanceId,
        x: i32,
        y: i32,
    ) -> Result<(), Vec<PlacementIssue>> {
        let Some(index) = self.stamp_index(id) else {
            return Err(vec![PlacementIssue::new(
                x,
                y,
                format!("stamp {} missing", id),
            )]);
        };
        let mut moved = self.stamps[index].clone();
        moved.x = x;
        moved.y = y;
        let issues = self.placement_issues_for_stamp_excluding(&moved, Some(index));
        if issues.is_empty() {
            self.stamps[index] = moved;
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub fn object_anchor_at(&self, x: i32, y: i32) -> Option<usize> {
        self.objects
            .iter()
            .position(|object| object.x == x && object.y == y)
    }

    pub fn blocking_object_at(&self, x: i32, y: i32) -> Option<usize> {
        self.objects
            .iter()
            .rposition(|object| object.blocks_tile(x, y))
    }

    pub fn placement_issues_for_object(&self, object: PlacedObject) -> Vec<PlacementIssue> {
        self.placement_issues_for_object_excluding(object, None)
    }

    pub fn placement_issues_for_object_excluding(
        &self,
        object: PlacedObject,
        ignore_index: Option<usize>,
    ) -> Vec<PlacementIssue> {
        let mut issues = Vec::new();

        if Self::idx(object.x, object.y).is_none() {
            issues.push(PlacementIssue::new(
                object.x,
                object.y,
                "anchor outside scene",
            ));
            return issues;
        }

        let (vx, vy, vw, vh) = object.visual_rect();
        if vw <= 0 || vh <= 0 {
            issues.push(PlacementIssue::new(
                object.x,
                object.y,
                "visual footprint has no area",
            ));
        } else {
            for y in vy..vy + vh {
                for x in vx..vx + vw {
                    if Self::idx(x, y).is_none() {
                        issues.push(PlacementIssue::new(x, y, "visual footprint outside scene"));
                    }
                }
            }
        }

        let (cx, cy, cw, ch) = object.collision_rect();
        if cw < 0 || ch < 0 {
            issues.push(PlacementIssue::new(
                object.x,
                object.y,
                "negative collision footprint",
            ));
        }
        if object.footprint.blocks_movement {
            for y in cy..cy + ch.max(0) {
                for x in cx..cx + cw.max(0) {
                    if Self::idx(x, y).is_none() {
                        issues.push(PlacementIssue::new(
                            x,
                            y,
                            "collision footprint outside scene",
                        ));
                    } else if !self.terrain_allows_standard_building_at(x, y) {
                        issues.push(PlacementIssue::new(
                            x,
                            y,
                            "collision footprint on terrain that forbids standard building",
                        ));
                    }
                }
            }
        }

        let (ix, iy, iw, ih) = object.interaction_rect();
        if iw < 0 || ih < 0 {
            issues.push(PlacementIssue::new(
                object.x,
                object.y,
                "negative interaction footprint",
            ));
        }
        for y in iy..iy + ih.max(0) {
            for x in ix..ix + iw.max(0) {
                if Self::idx(x, y).is_none() {
                    issues.push(PlacementIssue::new(
                        x,
                        y,
                        "interaction footprint outside scene",
                    ));
                }
            }
        }

        for (index, other) in self.objects.iter().enumerate() {
            if Some(index) == ignore_index {
                continue;
            }
            if objects_overlap(*other, object) {
                issues.push(PlacementIssue::new(
                    object.x,
                    object.y,
                    format!("overlaps existing {}", other.kind.label()),
                ));
            }
        }
        for stamp in &self.stamps {
            if spatial_footprints_overlap(
                object.footprint,
                object.x,
                object.y,
                stamp.footprint,
                stamp.x,
                stamp.y,
            ) {
                issues.push(PlacementIssue::new(
                    object.x,
                    object.y,
                    format!("overlaps existing stamp {}", stamp.stamp_key),
                ));
            }
        }

        issues
    }

    pub fn can_place_custom_object(&self, object: PlacedObject) -> bool {
        self.placement_issues_for_object(object).is_empty()
    }

    pub fn can_place_custom_object_excluding(
        &self,
        object: PlacedObject,
        ignore_index: Option<usize>,
    ) -> bool {
        self.placement_issues_for_object_excluding(object, ignore_index)
            .is_empty()
    }

    pub fn interaction_object_at(&self, x: i32, y: i32) -> Option<usize> {
        self.objects
            .iter()
            .rposition(|object| object.contains_interaction_tile(x, y))
            .or_else(|| {
                self.objects
                    .iter()
                    .rposition(|object| object.contains_collision_tile(x, y))
            })
    }

    pub fn collision_at(&self, x: i32, y: i32) -> CollisionResult {
        if Self::idx(x, y).is_none() {
            return CollisionResult::blocked(CollisionReason::OutOfBounds);
        }
        let tile = self.get(x, y);
        let class = collision_class(tile);
        if class != CollisionClass::WalkableGround {
            return CollisionResult::blocked(CollisionReason::Terrain { tile, class });
        }
        if let Some(index) = self.blocking_object_at(x, y) {
            return CollisionResult::blocked(CollisionReason::Object { index });
        }
        if let Some(index) = self.blocking_stamp_at(x, y) {
            return CollisionResult::blocked(CollisionReason::Stamp { index });
        }
        CollisionResult::open()
    }

    pub fn is_cell_walkable(&self, x: i32, y: i32) -> bool {
        !self.collision_at(x, y).blocked
    }

    pub fn terrain_movement_cost_at(&self, x: i32, y: i32) -> Option<u16> {
        Self::idx(x, y)?;
        Some(terrain_gameplay_profile(self.get(x, y)).movement_cost)
    }

    pub fn terrain_building_class_at(&self, x: i32, y: i32) -> Option<BuildingClass> {
        Self::idx(x, y)?;
        Some(terrain_gameplay_profile(self.get(x, y)).building)
    }

    pub fn terrain_farming_class_at(&self, x: i32, y: i32) -> Option<FarmingClass> {
        Self::idx(x, y)?;
        Some(terrain_gameplay_profile(self.get(x, y)).farming)
    }

    pub fn terrain_footstep_surface_at(&self, x: i32, y: i32) -> Option<FootstepSurface> {
        Self::idx(x, y)?;
        Some(terrain_gameplay_profile(self.get(x, y)).footstep)
    }

    pub fn terrain_is_fishable_at(&self, x: i32, y: i32) -> bool {
        Self::idx(x, y)
            .map(|_| terrain_gameplay_profile(self.get(x, y)).fishable)
            .unwrap_or(false)
    }

    pub fn terrain_allows_building_support_at(
        &self,
        x: i32,
        y: i32,
        support: BuildingSupport,
    ) -> bool {
        self.terrain_building_class_at(x, y)
            .map(|class| class.allows(support))
            .unwrap_or(false)
    }

    pub fn terrain_allows_standard_building_at(&self, x: i32, y: i32) -> bool {
        self.terrain_allows_building_support_at(x, y, BuildingSupport::Standard)
    }

    pub fn terrain_allows_foundation_at(&self, x: i32, y: i32) -> bool {
        self.terrain_allows_building_support_at(x, y, BuildingSupport::Foundation)
    }

    pub fn terrain_allows_bridge_at(&self, x: i32, y: i32) -> bool {
        self.terrain_allows_building_support_at(x, y, BuildingSupport::Bridge)
    }

    pub fn collision_at_with_water_access(
        &self,
        x: i32,
        y: i32,
        allow_shallow: bool,
        allow_deep: bool,
    ) -> CollisionResult {
        if Self::idx(x, y).is_none() {
            return CollisionResult::blocked(CollisionReason::OutOfBounds);
        }
        let tile = self.get(x, y);
        let profile = terrain_gameplay_profile(tile);
        let terrain_open = match profile.water_depth {
            crate::WaterDepthClass::Shallow => allow_shallow,
            crate::WaterDepthClass::Deep => allow_deep,
            crate::WaterDepthClass::None => profile.walkable(),
        };
        if !terrain_open {
            return CollisionResult::blocked(CollisionReason::Terrain {
                tile,
                class: profile.collision,
            });
        }
        if let Some(index) = self.blocking_object_at(x, y) {
            return CollisionResult::blocked(CollisionReason::Object { index });
        }
        if let Some(index) = self.blocking_stamp_at(x, y) {
            return CollisionResult::blocked(CollisionReason::Stamp { index });
        }
        CollisionResult::open()
    }

    pub fn remove_object_at(&mut self, x: i32, y: i32) {
        self.objects.retain(|object| !object.contains_tile(x, y));
    }

    pub fn place_object(&mut self, kind: ObjectKind, x: i32, y: i32) -> Option<ObjectId> {
        Self::idx(x, y)?;
        let placed = PlacedObject::with_id(self.next_object_id(), kind, x, y);
        if self.can_place_custom_object(placed) {
            let id = placed.id;
            self.objects.push(placed);
            Some(id)
        } else {
            None
        }
    }

    pub fn place_custom_object(&mut self, mut object: PlacedObject) -> Option<ObjectId> {
        Self::idx(object.x, object.y)?;
        if !object.id.is_assigned() || self.object(object.id).is_some() {
            object.id = self.next_object_id();
        }
        let removed_ids = self
            .objects
            .iter()
            .filter(|other| objects_overlap(**other, object))
            .map(|other| other.id)
            .collect::<Vec<_>>();
        self.objects
            .retain(|other| !removed_ids.contains(&other.id));
        for removed_id in removed_ids {
            self.object_asset_refs.remove(&removed_id);
            self.object_states.remove(&removed_id);
        }
        let id = object.id;
        self.objects.push(object);
        Some(id)
    }

    pub fn remove_object_index(&mut self, index: usize) -> Option<PlacedObject> {
        if index < self.objects.len() {
            let removed = self.objects.remove(index);
            self.object_asset_refs.remove(&removed.id);
            self.object_states.remove(&removed.id);
            Some(removed)
        } else {
            None
        }
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<PlacedObject> {
        let index = self.object_index(id)?;
        self.remove_object_index(index)
    }

    pub fn move_object(&mut self, id: ObjectId, x: i32, y: i32) -> Result<(), Vec<PlacementIssue>> {
        let Some(index) = self.object_index(id) else {
            return Err(vec![PlacementIssue::new(
                x,
                y,
                format!("object {} missing", id),
            )]);
        };
        self.move_object_to(index, x, y)
    }

    pub fn move_object_to(
        &mut self,
        index: usize,
        x: i32,
        y: i32,
    ) -> Result<(), Vec<PlacementIssue>> {
        if index >= self.objects.len() {
            return Err(vec![PlacementIssue::new(x, y, "object index missing")]);
        }
        let mut moved = self.objects[index];
        moved.x = x;
        moved.y = y;
        let issues = self.placement_issues_for_object_excluding(moved, Some(index));
        if issues.is_empty() {
            self.objects[index] = moved;
            Ok(())
        } else {
            Err(issues)
        }
    }

    pub fn object_asset_ref(&self, id: ObjectId) -> Option<&StablePlaceableAssetRef> {
        self.object_asset_refs.get(&id)
    }

    pub fn set_object_asset_ref(&mut self, id: ObjectId, asset_ref: StablePlaceableAssetRef) {
        if self.object(id).is_some() {
            self.object_asset_refs.insert(id, asset_ref);
        }
    }

    pub fn object_state(&self, id: ObjectId) -> Option<&str> {
        self.object_states.get(&id).map(String::as_str)
    }

    pub fn set_object_state(&mut self, id: ObjectId, state: impl Into<String>) {
        if self.object(id).is_some() {
            self.object_states.insert(id, state.into());
        }
    }

    pub fn place_pack_defined_object(
        &mut self,
        object: PlacedObject,
        asset_ref: StablePlaceableAssetRef,
    ) -> Option<ObjectId> {
        self.place_pack_defined_object_with_state(object, asset_ref, None)
    }

    pub fn place_pack_defined_object_with_state(
        &mut self,
        object: PlacedObject,
        asset_ref: StablePlaceableAssetRef,
        initial_state: Option<&str>,
    ) -> Option<ObjectId> {
        let id = self.place_custom_object(object)?;
        self.object_asset_refs.insert(id, asset_ref);
        if let Some(state) = initial_state.filter(|state| !state.is_empty()) {
            self.object_states.insert(id, state.to_string());
        }
        Some(id)
    }
}

fn scene_seed(id: SceneId) -> u32 {
    match id {
        SceneId::Farmstead => 11,
        SceneId::TavernInterior => 23,
        SceneId::Cellar => 37,
        SceneId::GuestFloor => 41,
        SceneId::NorthRoad => 53,
        SceneId::SouthField => 67,
        SceneId::EastWoods => 79,
        SceneId::CaveMouth => 83,
        SceneId::CaveDepths => 97,
    }
}

fn hash_cell(seed: u32, x: i32, y: i32) -> u32 {
    let mut n = seed ^ (x as u32).wrapping_mul(374_761_393);
    n = n.wrapping_add((y as u32).wrapping_mul(668_265_263));
    n ^= n >> 13;
    n = n.wrapping_mul(1_274_126_177);
    n ^ (n >> 16)
}

#[cfg(test)]
mod foundation_tests;
