use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::{compose_building_scene, BuildingDefinition, BuildingInstanceId, BuildingSceneKey, BuildingSpace, ComposedBuildingScene, SceneDimensions};

/// Stable runtime identity for a materialized building space. This is deliberately
/// string-backed so new building archetypes/floors never require SceneId enum growth.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BuildingRuntimeSceneId(String);
impl BuildingRuntimeSceneId {
    pub fn from_key(key: &BuildingSceneKey) -> Self { Self(key.stable_id()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Runtime materialization of one exterior/interior floor. Bounds come from the
/// composed layout, so an interior is free to be larger than its exterior.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingRuntimeScene {
    pub id: BuildingRuntimeSceneId,
    pub building: BuildingInstanceId,
    pub key: BuildingSceneKey,
    pub width: u32,
    pub height: u32,
    #[serde(default)] pub room_ids: Vec<String>,
    #[serde(default)] pub transition_ids: Vec<String>,
}

impl BuildingRuntimeScene {
    pub fn materialize(building: &BuildingDefinition, space: BuildingSpace, floor: i32) -> Self {
        let composed = compose_building_scene(building, space, floor);
        Self::from_composed(composed)
    }
    pub fn dimensions(&self) -> Result<SceneDimensions, String> { SceneDimensions::new(self.width as usize, self.height as usize).validate() }
    pub fn from_composed(scene: ComposedBuildingScene) -> Self {
        let transition_ids = scene.transition_anchors.iter().map(|a| a.transition_id.clone()).collect();
        Self { id: BuildingRuntimeSceneId::from_key(&scene.key), building: scene.key.building.clone(), key: scene.key, width: scene.width, height: scene.height, room_ids: scene.room_ids, transition_ids }
    }
}

/// Building-specific adapter over runtime scene materialization. The general
/// project scene registry can consume these instances without knowing tavern,
/// residence, shop, inn, workshop, or other archetype names.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildingRuntimeSceneRegistry {
    #[serde(default)] scenes: BTreeMap<String, BuildingRuntimeScene>,
}
impl BuildingRuntimeSceneRegistry {
    pub fn get(&self, key: &BuildingSceneKey) -> Option<&BuildingRuntimeScene> { self.scenes.get(&key.stable_id()) }
    pub fn get_or_materialize(&mut self, building: &BuildingDefinition, space: BuildingSpace, floor: i32) -> &BuildingRuntimeScene {
        let key = BuildingSceneKey { building: building.id.clone(), space, floor };
        let stable = key.stable_id();
        self.scenes.entry(stable).or_insert_with(|| BuildingRuntimeScene::materialize(building, space, floor))
    }
    pub fn remove_building(&mut self, building: &BuildingInstanceId) { self.scenes.retain(|_, scene| &scene.building != building); }
    pub fn len(&self) -> usize { self.scenes.len() }
    pub fn is_empty(&self) -> bool { self.scenes.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*; use crate::*;
    fn building() -> BuildingDefinition { BuildingDefinition { id: BuildingInstanceId::new("willowmere.inn.001"), archetype:"inn".into(), layout: BuildingLayout { exterior: ExteriorLayout { width:7,height:6,floors:2 }, interior: InteriorLayout { width:15,height:12,floors:vec![BuildingFloorLayout{level:0,rooms:vec![]},BuildingFloorLayout{level:1,rooms:vec![]}] }, transitions:vec![BuildingTransition{id:"entrance.main".into(),from:BuildingAnchor{space:BuildingSpace::Exterior,floor:0,x:3,y:5},to:BuildingAnchor{space:BuildingSpace::Interior,floor:0,x:7,y:11}}] } } }
    #[test] fn materialized_interior_uses_interior_bounds() { let b=building(); let s=BuildingRuntimeScene::materialize(&b,BuildingSpace::Interior,0); assert_eq!((s.width,s.height),(15,12)); assert_eq!(s.dimensions().unwrap(), SceneDimensions::new(15,12)); assert_eq!(s.id.as_str(),"building:willowmere.inn.001:interior:floor:0"); }
    #[test] fn registry_reuses_same_instance_identity() { let b=building(); let mut r=BuildingRuntimeSceneRegistry::default(); let a=r.get_or_materialize(&b,BuildingSpace::Interior,0).id.clone(); let c=r.get_or_materialize(&b,BuildingSpace::Interior,0).id.clone(); assert_eq!(a,c); assert_eq!(r.len(),1); }
    #[test] fn floors_do_not_require_scene_enum_variants() { let b=building(); let mut r=BuildingRuntimeSceneRegistry::default(); r.get_or_materialize(&b,BuildingSpace::Interior,0); r.get_or_materialize(&b,BuildingSpace::Interior,1); assert_eq!(r.len(),2); }
}
