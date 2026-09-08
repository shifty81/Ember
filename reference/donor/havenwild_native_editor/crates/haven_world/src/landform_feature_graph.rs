//! Seed-stable landform and mainland feature planning contracts.
//!
//! This module intentionally describes *what must exist* before local tiles are
//! materialized. It prevents large-scale geography from degenerating into
//! chunk-local blobs and gives cities, trade routes, caves, coastal cliffs,
//! hydrology, ecology, and later terrain constraint synthesis one shared macro
//! authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const LANDFORM_FEATURE_GRAPH_SCHEMA: &str = "havenwild.landform_feature_graph.v0_1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldFeatureId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldGridPoint {
    pub x: i32,
    pub y: i32,
}

impl WorldGridPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Discrete structural tier. Current Havenwild surface profiles cap normal
/// generated terrain at tier 2, but using a numeric wrapper avoids baking that
/// limit into every consumer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructuralTier(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LandformFeatureKind {
    MountainMassif,
    Highland,
    Plateau,
    Valley,
    Basin,
    Canyon,
    CoastalCliff,
    Island,
    Wetland,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralTerracePlan {
    pub tier: StructuralTier,
    /// Ordered control points for an irregular terrace/ridge footprint. Local
    /// rasterization may distort and erode this shape, but must preserve the
    /// feature identity and tier relationship.
    pub contour_controls: Vec<WorldGridPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaveEntranceCandidate {
    pub anchor: WorldGridPoint,
    pub host_tier: StructuralTier,
    pub preferred_facing: CardinalFacing,
    pub cave_system_id: WorldFeatureId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardinalFacing {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountainMassifPlan {
    pub id: WorldFeatureId,
    pub ridge_spine: Vec<WorldGridPoint>,
    pub terraces: Vec<StructuralTerracePlan>,
    pub cave_candidates: Vec<CaveEntranceCandidate>,
    pub drainage_notches: Vec<WorldGridPoint>,
    pub pass_candidates: Vec<WorldGridPoint>,
}

impl MountainMassifPlan {
    pub fn validate(&self, max_surface_tier: StructuralTier) -> Result<(), String> {
        if self.ridge_spine.len() < 2 {
            return Err(format!(
                "mountain massif {} requires a ridge spine with at least two controls",
                self.id.0
            ));
        }
        if self.terraces.is_empty() {
            return Err(format!("mountain massif {} has no terraces", self.id.0));
        }
        let mut tiers = BTreeSet::new();
        for terrace in &self.terraces {
            if terrace.tier.0 == 0 || terrace.tier > max_surface_tier {
                return Err(format!(
                    "mountain massif {} terrace tier {} outside supported surface range",
                    self.id.0, terrace.tier.0
                ));
            }
            if terrace.contour_controls.len() < 3 {
                return Err(format!(
                    "mountain massif {} tier {} needs at least three contour controls",
                    self.id.0, terrace.tier.0
                ));
            }
            tiers.insert(terrace.tier);
        }
        if !tiers.contains(&StructuralTier(1)) {
            return Err(format!(
                "mountain massif {} must contain a Level/Tier 1 base terrace",
                self.id.0
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoastalToeMaterial {
    None,
    Sand,
    WetSand,
    Pebble,
    Talus,
    WetRock,
    Mud,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoastalCliffPlan {
    pub id: WorldFeatureId,
    pub contour_controls: Vec<WorldGridPoint>,
    pub host_tier: StructuralTier,
    pub toe_materials: Vec<CoastalToeMaterial>,
    pub minimum_toe_width: u8,
    pub maximum_toe_width: u8,
    pub permits_direct_deep_water: bool,
    pub wave_exposure: u8,
}

impl CoastalCliffPlan {
    pub fn validate(&self) -> Result<(), String> {
        if self.contour_controls.len() < 2 {
            return Err(format!("coastal cliff {} has no usable contour", self.id.0));
        }
        if self.host_tier.0 == 0 {
            return Err(format!(
                "coastal cliff {} must descend from a raised structural tier",
                self.id.0
            ));
        }
        if self.minimum_toe_width > self.maximum_toe_width {
            return Err(format!(
                "coastal cliff {} toe width range is inverted",
                self.id.0
            ));
        }
        if self.toe_materials.is_empty() && !self.permits_direct_deep_water {
            return Err(format!(
                "coastal cliff {} needs a toe material or direct-water permission",
                self.id.0
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MainlandCityRole {
    WesternHarborCapital,
    MountainCity,
    SouthernSandyCity,
}

impl MainlandCityRole {
    pub const ALL: [Self; 3] = [
        Self::WesternHarborCapital,
        Self::MountainCity,
        Self::SouthernSandyCity,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainlandCityAnchor {
    pub feature_id: WorldFeatureId,
    pub role: MainlandCityRole,
    pub anchor: WorldGridPoint,
    pub province_id: WorldFeatureId,
    pub route_node_id: WorldFeatureId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TradeRouteClass {
    PrimaryRoad,
    RiverFreight,
    CoastalShipping,
    MountainPassRoad,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainlandTradeRoutePlan {
    pub feature_id: WorldFeatureId,
    pub from: MainlandCityRole,
    pub to: MainlandCityRole,
    pub class: TradeRouteClass,
    pub scheduled_npc_traffic: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainlandPlanV1 {
    pub feature_id: WorldFeatureId,
    pub max_surface_tier: StructuralTier,
    pub cities: Vec<MainlandCityAnchor>,
    pub trade_routes: Vec<MainlandTradeRoutePlan>,
    pub massifs: Vec<MountainMassifPlan>,
    pub coastal_cliffs: Vec<CoastalCliffPlan>,
    /// The western capital/harbor is the mandatory gateway to mission/extraction
    /// islands and scheduled inter-island transport.
    pub expedition_gateway: MainlandCityRole,
}

impl MainlandPlanV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_surface_tier.0 < 2 {
            return Err("normal mainland plan must support structural tiers 0/1/2".into());
        }
        if self.cities.len() != MainlandCityRole::ALL.len() {
            return Err(format!(
                "mainland requires exactly three primary cities; found {}",
                self.cities.len()
            ));
        }
        let roles = self.cities.iter().map(|city| city.role).collect::<BTreeSet<_>>();
        if roles != MainlandCityRole::ALL.into_iter().collect::<BTreeSet<_>>() {
            return Err("mainland primary city roles are incomplete or duplicated".into());
        }
        if self.expedition_gateway != MainlandCityRole::WesternHarborCapital {
            return Err("western harbor capital must own the expedition gateway".into());
        }
        for massif in &self.massifs {
            massif.validate(self.max_surface_tier)?;
        }
        if self.massifs.is_empty() {
            return Err("mainland requires at least one mountain/highland massif".into());
        }
        if !self
            .massifs
            .iter()
            .any(|massif| !massif.cave_candidates.is_empty())
        {
            return Err("mainland requires at least one cave-bearing massif".into());
        }
        for coastal in &self.coastal_cliffs {
            coastal.validate()?;
        }
        self.validate_trade_network()
    }

    fn validate_trade_network(&self) -> Result<(), String> {
        let mut graph: BTreeMap<MainlandCityRole, BTreeSet<MainlandCityRole>> =
            MainlandCityRole::ALL
                .into_iter()
                .map(|role| (role, BTreeSet::new()))
                .collect();
        let mut direct_pairs = BTreeSet::new();
        for route in &self.trade_routes {
            if route.from == route.to {
                return Err(format!("trade route {} connects a city to itself", route.feature_id.0));
            }
            if !route.scheduled_npc_traffic {
                return Err(format!(
                    "primary mainland route {} lacks scheduled NPC traffic",
                    route.feature_id.0
                ));
            }
            graph.entry(route.from).or_default().insert(route.to);
            graph.entry(route.to).or_default().insert(route.from);
            direct_pairs.insert(ordered_city_pair(route.from, route.to));
        }

        // The primary trade triangle is a gameplay invariant. Additional river
        // or shipping services may duplicate these connections, but each city
        // pair must have at least one direct scheduled route.
        for (a, b) in [
            (
                MainlandCityRole::WesternHarborCapital,
                MainlandCityRole::MountainCity,
            ),
            (
                MainlandCityRole::WesternHarborCapital,
                MainlandCityRole::SouthernSandyCity,
            ),
            (
                MainlandCityRole::MountainCity,
                MainlandCityRole::SouthernSandyCity,
            ),
        ] {
            if !direct_pairs.contains(&ordered_city_pair(a, b)) {
                return Err(format!("mainland trade triangle missing {:?} <-> {:?}", a, b));
            }
        }

        let start = MainlandCityRole::WesternHarborCapital;
        let mut visited = BTreeSet::new();
        visited.insert(start);
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(role) = queue.pop_front() {
            for neighbor in graph.get(&role).into_iter().flatten() {
                if visited.insert(*neighbor) {
                    queue.push_back(*neighbor);
                }
            }
        }
        if visited.len() != MainlandCityRole::ALL.len() {
            return Err("mainland scheduled trade network is disconnected".into());
        }
        Ok(())
    }
}

fn ordered_city_pair(
    a: MainlandCityRole,
    b: MainlandCityRole,
) -> (MainlandCityRole, MainlandCityRole) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpeditionTravelMode {
    MissionContract,
    Stowaway,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpeditionNetworkPlan {
    pub gateway_city: MainlandCityRole,
    pub allows_mission_contracts: bool,
    pub allows_stowaway: bool,
    /// Normal stowaway flow promises the transport will return to the same
    /// extraction anchor after this many in-game days. Current design target is
    /// one day.
    pub stowaway_return_days: u8,
    pub missed_extraction_is_recoverable: bool,
}

impl ExpeditionNetworkPlan {
    pub fn validate(&self) -> Result<(), String> {
        if self.gateway_city != MainlandCityRole::WesternHarborCapital {
            return Err("expedition network must originate at the western harbor capital".into());
        }
        if !self.allows_mission_contracts || !self.allows_stowaway {
            return Err("expedition network must support mission and stowaway travel lanes".into());
        }
        if self.stowaway_return_days == 0 {
            return Err("stowaway pickup must have a non-zero return window".into());
        }
        if !self.missed_extraction_is_recoverable {
            return Err("missed expedition extraction may not hard-lock a world".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_plan() -> MainlandPlanV1 {
        let city = |id, role, x, y| MainlandCityAnchor {
            feature_id: WorldFeatureId(id),
            role,
            anchor: WorldGridPoint::new(x, y),
            province_id: WorldFeatureId(id + 10),
            route_node_id: WorldFeatureId(id + 20),
        };
        let route = |id, from, to| MainlandTradeRoutePlan {
            feature_id: WorldFeatureId(id),
            from,
            to,
            class: TradeRouteClass::PrimaryRoad,
            scheduled_npc_traffic: true,
        };
        MainlandPlanV1 {
            feature_id: WorldFeatureId(1),
            max_surface_tier: StructuralTier(2),
            cities: vec![
                city(100, MainlandCityRole::WesternHarborCapital, -100, 0),
                city(101, MainlandCityRole::MountainCity, 30, -90),
                city(102, MainlandCityRole::SouthernSandyCity, 80, 120),
            ],
            trade_routes: vec![
                route(
                    200,
                    MainlandCityRole::WesternHarborCapital,
                    MainlandCityRole::MountainCity,
                ),
                route(
                    201,
                    MainlandCityRole::WesternHarborCapital,
                    MainlandCityRole::SouthernSandyCity,
                ),
                route(
                    202,
                    MainlandCityRole::MountainCity,
                    MainlandCityRole::SouthernSandyCity,
                ),
            ],
            massifs: vec![MountainMassifPlan {
                id: WorldFeatureId(300),
                ridge_spine: vec![WorldGridPoint::new(0, 0), WorldGridPoint::new(40, -30)],
                terraces: vec![
                    StructuralTerracePlan {
                        tier: StructuralTier(1),
                        contour_controls: vec![
                            WorldGridPoint::new(-10, 0),
                            WorldGridPoint::new(20, -20),
                            WorldGridPoint::new(40, 10),
                        ],
                    },
                    StructuralTerracePlan {
                        tier: StructuralTier(2),
                        contour_controls: vec![
                            WorldGridPoint::new(5, -5),
                            WorldGridPoint::new(20, -15),
                            WorldGridPoint::new(30, 0),
                        ],
                    },
                ],
                cave_candidates: vec![CaveEntranceCandidate {
                    anchor: WorldGridPoint::new(8, 4),
                    host_tier: StructuralTier(1),
                    preferred_facing: CardinalFacing::South,
                    cave_system_id: WorldFeatureId(400),
                }],
                drainage_notches: vec![],
                pass_candidates: vec![],
            }],
            coastal_cliffs: vec![],
            expedition_gateway: MainlandCityRole::WesternHarborCapital,
        }
    }

    #[test]
    fn mainland_requires_three_role_specific_cities_and_trade_triangle() {
        valid_plan().validate().expect("valid mainland plan");
    }

    #[test]
    fn missing_direct_trade_leg_is_rejected() {
        let mut plan = valid_plan();
        plan.trade_routes.pop();
        assert!(plan.validate().is_err());
    }

    #[test]
    fn coastal_cliff_allows_zero_width_toe_when_direct_water_is_permitted() {
        CoastalCliffPlan {
            id: WorldFeatureId(9),
            contour_controls: vec![WorldGridPoint::new(0, 0), WorldGridPoint::new(8, 0)],
            host_tier: StructuralTier(1),
            toe_materials: vec![],
            minimum_toe_width: 0,
            maximum_toe_width: 0,
            permits_direct_deep_water: true,
            wave_exposure: 255,
        }
        .validate()
        .expect("sheer sea cliff");
    }

    #[test]
    fn expedition_plan_locks_western_gateway_and_recoverable_pickup() {
        ExpeditionNetworkPlan {
            gateway_city: MainlandCityRole::WesternHarborCapital,
            allows_mission_contracts: true,
            allows_stowaway: true,
            stowaway_return_days: 1,
            missed_extraction_is_recoverable: true,
        }
        .validate()
        .expect("expedition plan");
    }
}
