use haven_core::{
    BuildingAnchor, BuildingDefinition, BuildingFloorLayout, BuildingInstanceId, BuildingLayout,
    BuildingRoom, BuildingRoomKind, BuildingSpace, BuildingTransition, ExteriorLayout,
    InteriorLayout,
};
use serde::{Deserialize, Serialize};

/// Deterministic PCG request that resolves directly to the canonical BuildingLayout authority.
/// No generated-building runtime type is introduced: after generation, authored and PCG
/// buildings are both plain BuildingDefinition values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingGenerationRequest {
    pub instance_id: String,
    pub archetype: String,
    pub seed: u64,
    pub exterior_min: [u32; 2],
    pub exterior_max: [u32; 2],
    pub interior_min: [u32; 2],
    pub interior_max: [u32; 2],
    #[serde(default = "default_floor_count")]
    pub floors: u32,
    #[serde(default)]
    pub room_program: Vec<BuildingRoomKind>,
}

impl BuildingGenerationRequest {
    pub fn validate(&self) -> Result<(), String> {
        validate_range("exterior", self.exterior_min, self.exterior_max)?;
        validate_range("interior", self.interior_min, self.interior_max)?;
        if self.instance_id.trim().is_empty() {
            return Err("building generation instance_id must not be empty".to_string());
        }
        if self.archetype.trim().is_empty() {
            return Err("building generation archetype must not be empty".to_string());
        }
        if self.floors == 0 {
            return Err("building generation floors must be at least 1".to_string());
        }
        Ok(())
    }
}

/// Generate one canonical building definition from deterministic constraints.
/// The exterior and interior ranges are intentionally independent; callers may request
/// a compact overworld footprint and a substantially larger playable interior scene.
pub fn generate_building_layout(request: &BuildingGenerationRequest) -> Result<BuildingDefinition, String> {
    request.validate()?;

    let exterior_width = choose_dimension(request.seed, 0x31, request.exterior_min[0], request.exterior_max[0]);
    let exterior_height = choose_dimension(request.seed, 0x47, request.exterior_min[1], request.exterior_max[1]);
    let interior_width = choose_dimension(request.seed, 0x59, request.interior_min[0], request.interior_max[0]);
    let interior_height = choose_dimension(request.seed, 0x6d, request.interior_min[1], request.interior_max[1]);

    let floors = (0..request.floors)
        .map(|level| BuildingFloorLayout {
            level: level as i32,
            rooms: build_rooms_for_floor(
                &request.room_program,
                level,
                interior_width,
                interior_height,
            ),
        })
        .collect::<Vec<_>>();

    let mut transitions = vec![BuildingTransition {
        id: "entrance.main".to_string(),
        from: BuildingAnchor {
            space: BuildingSpace::Exterior,
            floor: 0,
            x: exterior_width / 2,
            y: exterior_height.saturating_sub(1),
        },
        to: BuildingAnchor {
            space: BuildingSpace::Interior,
            floor: 0,
            x: interior_width / 2,
            y: interior_height.saturating_sub(1),
        },
    }];

    for level in 0..request.floors.saturating_sub(1) {
        let x = 1.min(interior_width.saturating_sub(1));
        let y = 1.min(interior_height.saturating_sub(1));
        transitions.push(BuildingTransition {
            id: format!("stairs.{}_to_{}", level, level + 1),
            from: BuildingAnchor {
                space: BuildingSpace::Interior,
                floor: level as i32,
                x,
                y,
            },
            to: BuildingAnchor {
                space: BuildingSpace::Interior,
                floor: level as i32 + 1,
                x,
                y,
            },
        });
    }

    Ok(BuildingDefinition {
        id: BuildingInstanceId::new(&request.instance_id),
        archetype: request.archetype.trim().to_ascii_lowercase(),
        layout: BuildingLayout {
            exterior: ExteriorLayout {
                width: exterior_width,
                height: exterior_height,
                floors: request.floors,
            },
            interior: InteriorLayout {
                width: interior_width,
                height: interior_height,
                floors,
            },
            transitions,
        },
    })
}

/// Permanent W55R9 engineering fixtures. Exact dimensions make failures obvious, but they
/// are acceptance data only; production building archetypes remain variable-size.
pub fn building_acceptance_requests() -> Vec<BuildingGenerationRequest> {
    vec![
        fixture("acceptance.cottage.001", "residence", [7, 6], [11, 9], 1, vec![BuildingRoomKind::Public, BuildingRoomKind::Bedroom]),
        fixture("acceptance.residence.001", "residence", [11, 8], [14, 12], 2, vec![BuildingRoomKind::Public, BuildingRoomKind::Bedroom, BuildingRoomKind::Private]),
        fixture("acceptance.shop.001", "shop", [9, 7], [13, 10], 1, vec![BuildingRoomKind::Shop, BuildingRoomKind::Storage]),
        fixture("acceptance.workshop.001", "workshop", [12, 8], [15, 11], 1, vec![BuildingRoomKind::Workshop, BuildingRoomKind::Storage]),
        fixture("acceptance.tavern.001", "tavern", [15, 10], [21, 16], 1, vec![BuildingRoomKind::Public, BuildingRoomKind::Bar, BuildingRoomKind::Dining, BuildingRoomKind::Kitchen]),
        fixture("acceptance.inn.001", "inn", [16, 11], [22, 18], 3, vec![BuildingRoomKind::Public, BuildingRoomKind::Bedroom, BuildingRoomKind::Hall]),
        fixture("acceptance.mixed_use.001", "mixed_use", [12, 9], [16, 13], 2, vec![BuildingRoomKind::Shop, BuildingRoomKind::Private, BuildingRoomKind::Bedroom]),
    ]
}

fn fixture(
    instance_id: &str,
    archetype: &str,
    exterior: [u32; 2],
    interior: [u32; 2],
    floors: u32,
    room_program: Vec<BuildingRoomKind>,
) -> BuildingGenerationRequest {
    BuildingGenerationRequest {
        instance_id: instance_id.to_string(),
        archetype: archetype.to_string(),
        seed: fnv1a64(instance_id.as_bytes()),
        exterior_min: exterior,
        exterior_max: exterior,
        interior_min: interior,
        interior_max: interior,
        floors,
        room_program,
    }
}

fn validate_range(label: &str, min: [u32; 2], max: [u32; 2]) -> Result<(), String> {
    if min[0] == 0 || min[1] == 0 || max[0] == 0 || max[1] == 0 {
        return Err(format!("{label} dimensions must be non-zero"));
    }
    if min[0] > max[0] || min[1] > max[1] {
        return Err(format!("{label} minimum dimensions exceed maximum dimensions"));
    }
    Ok(())
}

fn choose_dimension(seed: u64, salt: u64, min: u32, max: u32) -> u32 {
    if min == max {
        return min;
    }
    let span = u64::from(max - min) + 1;
    min + ((mix64(seed ^ salt) % span) as u32)
}

fn build_rooms_for_floor(
    program: &[BuildingRoomKind],
    level: u32,
    width: u32,
    height: u32,
) -> Vec<BuildingRoom> {
    if program.is_empty() || width < 3 || height < 3 {
        return Vec::new();
    }

    let count = program.len().min(width.saturating_sub(2) as usize).max(1);
    let usable_width = width.saturating_sub(2);
    let base = (usable_width / count as u32).max(1);
    let mut x = 1u32;
    let mut rooms = Vec::with_capacity(count);

    for (index, kind) in program.iter().take(count).enumerate() {
        let remaining = width.saturating_sub(1).saturating_sub(x);
        let room_width = if index + 1 == count { remaining.max(1) } else { base.min(remaining).max(1) };
        rooms.push(BuildingRoom {
            id: format!("floor{level}.room{index}"),
            kind: kind.clone(),
            x,
            y: 1,
            width: room_width,
            height: height.saturating_sub(2).max(1),
        });
        x = x.saturating_add(room_width);
    }
    rooms
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

const fn default_floor_count() -> u32 { 1 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_request_is_deterministic() {
        let mut request = building_acceptance_requests().remove(0);
        request.exterior_min = [7, 6];
        request.exterior_max = [10, 9];
        request.interior_min = [11, 9];
        request.interior_max = [16, 14];
        let a = generate_building_layout(&request).unwrap();
        let b = generate_building_layout(&request).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn acceptance_catalog_proves_variable_and_expanded_interiors() {
        let buildings = building_acceptance_requests()
            .iter()
            .map(generate_building_layout)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(buildings.len(), 7);
        assert!(buildings.iter().all(|building| {
            building.layout.interior.width > building.layout.exterior.width
                && building.layout.interior.height > building.layout.exterior.height
        }));
        assert!(buildings.iter().any(|building| building.layout.exterior.floors >= 3));
    }

    #[test]
    fn generated_multifloor_layout_contains_generic_stair_transitions() {
        let request = building_acceptance_requests().into_iter().find(|request| request.archetype == "inn").unwrap();
        let building = generate_building_layout(&request).unwrap();
        assert!(building.layout.transitions.iter().any(|transition| transition.id == "stairs.0_to_1"));
        assert!(building.layout.transitions.iter().any(|transition| transition.id == "stairs.1_to_2"));
    }

    #[test]
    fn generated_building_materializes_and_round_trips_through_runtime_navigation() {
        let request = building_acceptance_requests().into_iter().next().unwrap();
        let building = generate_building_layout(&request).unwrap();
        let expected_return = building.layout.transitions[0].from.clone();
        let expected_interior = building.layout.transitions[0].to.clone();
        let mut travel = haven_core::BuildingTravelSession::default();
        let mut scenes = haven_core::BuildingRuntimeSceneRegistry::default();
        let entered = haven_core::enter_building_runtime(
            &building,
            "entrance.main",
            &mut travel,
            &mut scenes,
        )
        .unwrap();
        assert_eq!((entered.spawn_x, entered.spawn_y), (expected_interior.x, expected_interior.y));
        assert_eq!(
            scenes.get(&entered.building_scene).unwrap().dimensions().unwrap(),
            haven_core::SceneDimensions::new(
                building.layout.interior.width as usize,
                building.layout.interior.height as usize,
            ),
        );
        let exited = haven_core::exit_building_runtime(&mut travel, &mut scenes, &building).unwrap();
        assert_eq!((exited.spawn_x, exited.spawn_y), (expected_return.x, expected_return.y));
    }
}
