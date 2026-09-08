use haven_core::{BuildingDefinition, BuildingRoomKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingFloorInspectorSummary {
    pub level: i32,
    pub scene_dimensions: [u32; 2],
    pub room_count: usize,
    pub rooms: Vec<(String, BuildingRoomKind)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingLayoutInspectorReport {
    pub instance_id: String,
    pub archetype: String,
    pub exterior_dimensions: [u32; 2],
    pub interior_dimensions: [u32; 2],
    pub interior_is_expanded: bool,
    pub exterior_floor_count: u32,
    pub floors: Vec<BuildingFloorInspectorSummary>,
    pub transition_ids: Vec<String>,
    pub validation_issues: Vec<String>,
}

/// Editor-facing read model over the same canonical `BuildingDefinition` consumed by runtime.
/// It never reconstructs or scales interior dimensions from the exterior footprint.
pub fn inspect_building_layout(building: &BuildingDefinition) -> BuildingLayoutInspectorReport {
    let validation_issues = haven_authoring::validate_building(building)
        .into_iter()
        .map(|issue| format!("{}: {}", issue.code, issue.message))
        .collect();
    BuildingLayoutInspectorReport {
        instance_id: building.id.as_str().to_string(),
        archetype: building.archetype.clone(),
        exterior_dimensions: [building.layout.exterior.width, building.layout.exterior.height],
        interior_dimensions: [building.layout.interior.width, building.layout.interior.height],
        interior_is_expanded: building.layout.interior.width > building.layout.exterior.width
            || building.layout.interior.height > building.layout.exterior.height,
        exterior_floor_count: building.layout.exterior.floors,
        floors: building
            .layout
            .interior
            .floors
            .iter()
            .map(|floor| BuildingFloorInspectorSummary {
                level: floor.level,
                scene_dimensions: [building.layout.interior.width, building.layout.interior.height],
                room_count: floor.rooms.len(),
                rooms: floor
                    .rooms
                    .iter()
                    .map(|room| (room.id.clone(), room.kind.clone()))
                    .collect(),
            })
            .collect(),
        transition_ids: building
            .layout
            .transitions
            .iter()
            .map(|transition| transition.id.clone())
            .collect(),
        validation_issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_inspector_reports_canonical_expanded_multifloor_layout() {
        let request = haven_world::building_acceptance_requests()
            .into_iter()
            .find(|request| request.archetype == "inn")
            .unwrap();
        let building = haven_world::generate_building_layout(&request).unwrap();
        let report = inspect_building_layout(&building);
        assert_eq!(report.instance_id, "acceptance.inn.001");
        assert_eq!(report.exterior_dimensions, [16, 11]);
        assert_eq!(report.interior_dimensions, [22, 18]);
        assert!(report.interior_is_expanded);
        assert_eq!(report.floors.len(), 3);
        assert!(report.validation_issues.is_empty());
        assert!(report.transition_ids.iter().any(|id| id == "stairs.1_to_2"));
    }
}
