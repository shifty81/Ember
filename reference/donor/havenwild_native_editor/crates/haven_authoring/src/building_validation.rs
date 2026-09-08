use haven_core::{BuildingDefinition, BuildingSpace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingValidationIssue { pub code: &'static str, pub message: String }

/// Consolidated validator for every building archetype. It validates semantic topology;
/// interior/exterior dimensions are explicitly allowed to differ.
pub fn validate_building(building: &BuildingDefinition) -> Vec<BuildingValidationIssue> {
    let mut out = Vec::new();
    let ext = &building.layout.exterior;
    let int = &building.layout.interior;
    if ext.width == 0 || ext.height == 0 { issue(&mut out, "building.exterior.empty", "exterior dimensions must be non-zero"); }
    if int.width == 0 || int.height == 0 { issue(&mut out, "building.interior.empty", "interior dimensions must be non-zero"); }
    for floor in &int.floors {
        for room in &floor.rooms {
            if room.width == 0 || room.height == 0 || room.x.saturating_add(room.width) > int.width || room.y.saturating_add(room.height) > int.height {
                issue(&mut out, "building.room.out_of_bounds", format!("room '{}' exceeds interior scene bounds", room.id));
            }
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    for transition in &building.layout.transitions {
        if transition.id.trim().is_empty() || !ids.insert(transition.id.as_str()) { issue(&mut out, "building.transition.id", "transition ids must be non-empty and unique"); }
        validate_anchor(&mut out, building, &transition.id, &transition.from);
        validate_anchor(&mut out, building, &transition.id, &transition.to);
    }
    out
}

fn validate_anchor(out: &mut Vec<BuildingValidationIssue>, b: &BuildingDefinition, id: &str, a: &haven_core::BuildingAnchor) {
    let (w,h) = match a.space { BuildingSpace::Exterior => (b.layout.exterior.width,b.layout.exterior.height), BuildingSpace::Interior => (b.layout.interior.width,b.layout.interior.height) };
    let floor_exists = match a.space {
        BuildingSpace::Exterior => a.floor >= 0 && (a.floor as u32) < b.layout.exterior.floors,
        BuildingSpace::Interior => b.layout.interior.floors.iter().any(|floor| floor.level == a.floor),
    };
    if !floor_exists { issue(out, "building.transition.floor_missing", format!("transition '{id}' references missing {:?} floor {}", a.space, a.floor)); }
    if a.x >= w || a.y >= h { issue(out, "building.transition.out_of_bounds", format!("transition '{id}' anchor is outside its {:?} bounds", a.space)); }
}
fn issue(out: &mut Vec<BuildingValidationIssue>, code: &'static str, message: impl Into<String>) { out.push(BuildingValidationIssue { code, message: message.into() }); }

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::*;
    #[test]
    fn expanded_interior_is_valid() {
        let b = BuildingDefinition { id: BuildingInstanceId::new("estate.house.1"), archetype: "residence".into(), layout: BuildingLayout { exterior: ExteriorLayout { width: 7, height: 6, floors: 1 }, interior: InteriorLayout { width: 12, height: 10, floors: vec![BuildingFloorLayout { level: 0, rooms: vec![] }] }, transitions: vec![BuildingTransition { id: "entrance.main".into(), from: BuildingAnchor { space: BuildingSpace::Exterior, floor: 0, x: 3, y: 5 }, to: BuildingAnchor { space: BuildingSpace::Interior, floor: 0, x: 5, y: 9 } }] } };
        assert!(validate_building(&b).is_empty());
    }

    #[test]
    fn transition_to_missing_floor_is_rejected() {
        let b = BuildingDefinition { id: BuildingInstanceId::new("estate.house.2"), archetype: "residence".into(), layout: BuildingLayout { exterior: ExteriorLayout { width: 7, height: 6, floors: 1 }, interior: InteriorLayout { width: 12, height: 10, floors: vec![BuildingFloorLayout { level: 0, rooms: vec![] }] }, transitions: vec![BuildingTransition { id: "stairs.invalid".into(), from: BuildingAnchor { space: BuildingSpace::Interior, floor: 0, x: 1, y: 1 }, to: BuildingAnchor { space: BuildingSpace::Interior, floor: 1, x: 1, y: 1 } }] } };
        let issues = validate_building(&b);
        assert!(issues.iter().any(|issue| issue.code == "building.transition.floor_missing"));
    }

    #[test]
    fn pcg_acceptance_buildings_use_the_same_consolidated_validator() {
        for request in haven_world::building_acceptance_requests() {
            let building = haven_world::generate_building_layout(&request).unwrap();
            let issues = validate_building(&building);
            assert!(issues.is_empty(), "{}: {:?}", building.id.as_str(), issues);
        }
    }
}
