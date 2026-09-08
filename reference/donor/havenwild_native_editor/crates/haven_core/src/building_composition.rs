use serde::{Deserialize, Serialize};
use crate::{BuildingDefinition, BuildingInstanceId, BuildingSpace};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSceneKey { pub building: BuildingInstanceId, pub space: BuildingSpace, pub floor: i32 }

impl BuildingSceneKey {
    pub fn stable_id(&self) -> String {
        format!("building:{}:{}:floor:{}", self.building.as_str(), match self.space { BuildingSpace::Exterior => "exterior", BuildingSpace::Interior => "interior" }, self.floor)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposedBuildingScene {
    pub key: BuildingSceneKey,
    pub width: u32,
    pub height: u32,
    #[serde(default)] pub room_ids: Vec<String>,
    #[serde(default)] pub transition_anchors: Vec<ComposedTransitionAnchor>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComposedTransitionAnchor { pub transition_id: String, pub is_destination: bool, pub x: u32, pub y: u32 }

pub fn compose_building_scene(building: &BuildingDefinition, space: BuildingSpace, floor: i32) -> ComposedBuildingScene {
    let (width, height, room_ids) = match space {
        BuildingSpace::Exterior => (building.layout.exterior.width, building.layout.exterior.height, Vec::new()),
        BuildingSpace::Interior => {
            let rooms = building.layout.interior.floors.iter().find(|f| f.level == floor)
                .map(|f| f.rooms.iter().map(|r| r.id.clone()).collect()).unwrap_or_default();
            (building.layout.interior.width, building.layout.interior.height, rooms)
        }
    };
    let mut transition_anchors = Vec::new();
    for t in &building.layout.transitions {
        if t.from.space == space && t.from.floor == floor { transition_anchors.push(ComposedTransitionAnchor { transition_id: t.id.clone(), is_destination: false, x: t.from.x, y: t.from.y }); }
        if t.to.space == space && t.to.floor == floor { transition_anchors.push(ComposedTransitionAnchor { transition_id: t.id.clone(), is_destination: true, x: t.to.x, y: t.to.y }); }
    }
    ComposedBuildingScene { key: BuildingSceneKey { building: building.id.clone(), space, floor }, width, height, room_ids, transition_anchors }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingTransitionDestination { pub scene: BuildingSceneKey, pub x: u32, pub y: u32 }

pub fn resolve_building_transition(building: &BuildingDefinition, transition_id: &str, from_space: BuildingSpace, from_floor: i32) -> Option<BuildingTransitionDestination> {
    let t = building.layout.transitions.iter().find(|t| t.id == transition_id && t.from.space == from_space && t.from.floor == from_floor)?;
    Some(BuildingTransitionDestination { scene: BuildingSceneKey { building: building.id.clone(), space: t.to.space, floor: t.to.floor }, x: t.to.x, y: t.to.y })
}

#[cfg(test)]
mod tests {
    use super::*; use crate::*;
    fn sample() -> BuildingDefinition { BuildingDefinition { id: BuildingInstanceId::new("willowmere.tavern.001"), archetype: "tavern".into(), layout: BuildingLayout { exterior: ExteriorLayout { width: 7, height: 6, floors: 1 }, interior: InteriorLayout { width: 14, height: 11, floors: vec![BuildingFloorLayout { level: 0, rooms: vec![BuildingRoom { id:"public_hall".into(), kind: BuildingRoomKind::Public, x:1,y:1,width:10,height:8 }] }] }, transitions: vec![BuildingTransition { id:"entrance.main".into(), from: BuildingAnchor { space:BuildingSpace::Exterior,floor:0,x:3,y:5 }, to:BuildingAnchor { space:BuildingSpace::Interior,floor:0,x:6,y:10 } }] } } }
    #[test] fn interior_scene_keeps_independent_dimensions() { let b=sample(); let s=compose_building_scene(&b,BuildingSpace::Interior,0); assert_eq!((s.width,s.height),(14,11)); assert_eq!(s.room_ids,vec!["public_hall"]); }
    #[test] fn transition_resolves_to_semantic_interior_scene() { let b=sample(); let d=resolve_building_transition(&b,"entrance.main",BuildingSpace::Exterior,0).unwrap(); assert_eq!(d.scene.stable_id(),"building:willowmere.tavern.001:interior:floor:0"); assert_eq!((d.x,d.y),(6,10)); }
}
