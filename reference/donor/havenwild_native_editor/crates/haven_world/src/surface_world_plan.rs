//! Behavior-preserving bridge between the current scene-rectangle PCG lane and
//! Havenwild's feature-first WorldPlan direction.
//!
//! Pass167Z109T deliberately does not replace the visible generator. It creates
//! one plan object before materialization, routes the existing terrain/feature/
//! structural/ecology stages through that plan's compatibility seeds, and
//! carries the target three-city mainland feature contract alongside the legacy
//! materializer. Later passes can replace individual materialization stages
//! without changing their callers or reintroducing scene rectangles as gameplay
//! authority.

use std::collections::BTreeSet;

use haven_core::{ProjectSceneId, SceneMap, MAP_H, MAP_W};

use crate::{
    mainland_features::{apply_mainland_surface_features, MainlandFeatureReport},
    open_world::ChunkCoord,
    CardinalFacing, CaveEntranceCandidate, MainlandCityAnchor, MainlandCityRole,
    MainlandPlanV1, MainlandTradeRoutePlan, MountainMassifPlan, StructuralTerracePlan,
    StructuralTier, TradeRouteClass, WorldFeatureId, WorldGridPoint,
};

pub const SURFACE_WORLD_PLAN_SCHEMA: &str = "havenwild.surface_world_plan.v0_1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceLandmassRole {
    Mainland,
    ExpeditionIsland,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfacePlanMaterializationMode {
    /// Preserve the current Z109S visible output while the new WorldPlan route
    /// becomes the call-site authority.
    LegacyVisualParity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceWorldBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl SurfaceWorldBounds {
    pub fn from_chunks(chunks: &[ChunkCoord]) -> Result<Self, String> {
        let min_chunk_x = chunks
            .iter()
            .map(|chunk| chunk.x)
            .min()
            .ok_or_else(|| "surface WorldPlan requires at least one chunk".to_string())?;
        let min_chunk_y = chunks.iter().map(|chunk| chunk.y).min().unwrap_or(0);
        let max_chunk_x = chunks.iter().map(|chunk| chunk.x).max().unwrap_or(0);
        let max_chunk_y = chunks.iter().map(|chunk| chunk.y).max().unwrap_or(0);
        Ok(Self {
            min_x: min_chunk_x * MAP_W as i32,
            min_y: min_chunk_y * MAP_H as i32,
            max_x: (max_chunk_x + 1) * MAP_W as i32 - 1,
            max_y: (max_chunk_y + 1) * MAP_H as i32 - 1,
        })
    }

    pub fn contains(self, point: WorldGridPoint) -> bool {
        point.x >= self.min_x
            && point.x <= self.max_x
            && point.y >= self.min_y
            && point.y <= self.max_y
    }

    fn point(self, x_num: i32, x_den: i32, y_num: i32, y_den: i32) -> WorldGridPoint {
        let width = (self.max_x - self.min_x).max(1);
        let height = (self.max_y - self.min_y).max(1);
        WorldGridPoint::new(
            self.min_x + width * x_num / x_den.max(1),
            self.min_y + height * y_num / y_den.max(1),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWorldPlanV1 {
    pub landmass_id: i32,
    pub landmass_name: String,
    pub region_id: String,
    /// Generation seed supplied to the existing landmass generator. For the
    /// mainland this is the actual world seed; minor islands may already use a
    /// caller-derived island seed. Keeping this exact value preserves output.
    pub generation_seed: u64,
    pub role: SurfaceLandmassRole,
    pub materialization_mode: SurfacePlanMaterializationMode,
    pub chunks: Vec<ChunkCoord>,
    pub bounds: SurfaceWorldBounds,
    pub harbor_scene_id: ProjectSceneId,
    /// Desired feature-first mainland contract. During Z109T it is validated
    /// and exposed for authoring/diagnostics but does not yet replace the legacy
    /// visible city/road/cliff materializer.
    pub target_mainland: Option<MainlandPlanV1>,
}

impl SurfaceWorldPlanV1 {
    pub fn build(
        landmass_id: i32,
        landmass_name: impl Into<String>,
        region_id: impl Into<String>,
        generation_seed: u64,
        chunks: Vec<ChunkCoord>,
        harbor_scene_id: ProjectSceneId,
    ) -> Result<Self, String> {
        let bounds = SurfaceWorldBounds::from_chunks(&chunks)?;
        let role = if landmass_id == 0 {
            SurfaceLandmassRole::Mainland
        } else {
            SurfaceLandmassRole::ExpeditionIsland
        };
        let target_mainland = (role == SurfaceLandmassRole::Mainland)
            .then(|| build_target_mainland_plan(generation_seed, bounds));
        let plan = Self {
            landmass_id,
            landmass_name: landmass_name.into(),
            region_id: region_id.into(),
            generation_seed,
            role,
            materialization_mode: SurfacePlanMaterializationMode::LegacyVisualParity,
            chunks,
            bounds,
            harbor_scene_id,
            target_mainland,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.region_id.trim().is_empty() {
            return Err("surface WorldPlan region id may not be empty".into());
        }
        if self.chunks.is_empty() {
            return Err("surface WorldPlan requires at least one partition".into());
        }
        let unique = self.chunks.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != self.chunks.len() {
            return Err("surface WorldPlan contains duplicate chunk coordinates".into());
        }
        match self.role {
            SurfaceLandmassRole::Mainland => {
                let target = self
                    .target_mainland
                    .as_ref()
                    .ok_or_else(|| "mainland WorldPlan is missing its target feature contract".to_string())?;
                target.validate()?;
                for city in &target.cities {
                    if !self.bounds.contains(city.anchor) {
                        return Err(format!(
                            "mainland city {:?} anchor lies outside the compatibility surface",
                            city.role
                        ));
                    }
                }
            }
            SurfaceLandmassRole::ExpeditionIsland => {
                if self.target_mainland.is_some() {
                    return Err("expedition island may not carry a mainland feature contract".into());
                }
            }
        }
        Ok(())
    }

    /// Exact seed previously used for highland, structural-landform, and
    /// natural-object materialization. This method intentionally does not adopt
    /// the future domain-separated seed scheme yet because T is a parity pass.
    pub const fn compatibility_landmass_seed(&self) -> u64 {
        self.generation_seed ^ self.landmass_id as i64 as u64
    }

    pub const fn is_mainland(&self) -> bool {
        matches!(self.role, SurfaceLandmassRole::Mainland)
    }
}

/// Routes current mainland feature materialization through the WorldPlan object
/// while preserving the exact legacy implementation and output in Z109T.
pub fn materialize_mainland_world_plan_compatibility(
    plan: &SurfaceWorldPlanV1,
    scenes: &mut [SceneMap],
    preserve_existing_infrastructure: bool,
) -> Result<MainlandFeatureReport, String> {
    if !plan.is_mainland() {
        return Ok(MainlandFeatureReport::default());
    }
    if scenes.len() != plan.chunks.len() {
        return Err(format!(
            "mainland WorldPlan materialization requires matching scene/chunk arrays ({} scenes, {} chunks)",
            scenes.len(),
            plan.chunks.len()
        ));
    }
    apply_mainland_surface_features(
        scenes,
        &plan.chunks,
        Some(&plan.harbor_scene_id),
        plan.generation_seed,
        preserve_existing_infrastructure,
    )
}

fn build_target_mainland_plan(seed: u64, bounds: SurfaceWorldBounds) -> MainlandPlanV1 {
    let western = bounds.point(1, 8, 3, 5);
    let mountain = bounds.point(3, 5, 1, 3);
    let southern = bounds.point(2, 3, 4, 5);
    let massif_west = bounds.point(9, 20, 1, 5);
    let massif_north = bounds.point(3, 5, 1, 6);
    let massif_east = bounds.point(3, 4, 2, 5);
    let massif_south = bounds.point(11, 20, 1, 2);
    let tier_two_west = bounds.point(1, 2, 1, 4);
    let tier_two_north = bounds.point(3, 5, 1, 5);
    let tier_two_east = bounds.point(7, 10, 7, 20);
    let tier_two_south = bounds.point(3, 5, 9, 20);
    let cave = bounds.point(11, 20, 9, 20);

    let city = |salt: u64, role, anchor| MainlandCityAnchor {
        feature_id: feature_id(seed, 0x4349_5459, salt),
        role,
        anchor,
        province_id: feature_id(seed, 0x5052_4f56, salt),
        route_node_id: feature_id(seed, 0x524f_5554, salt),
    };
    let route = |salt: u64, from, to, class| MainlandTradeRoutePlan {
        feature_id: feature_id(seed, 0x5452_4144, salt),
        from,
        to,
        class,
        scheduled_npc_traffic: true,
    };

    MainlandPlanV1 {
        feature_id: feature_id(seed, 0x4d41_494e, 0),
        max_surface_tier: StructuralTier(2),
        cities: vec![
            city(1, MainlandCityRole::WesternHarborCapital, western),
            city(2, MainlandCityRole::MountainCity, mountain),
            city(3, MainlandCityRole::SouthernSandyCity, southern),
        ],
        trade_routes: vec![
            route(
                1,
                MainlandCityRole::WesternHarborCapital,
                MainlandCityRole::MountainCity,
                TradeRouteClass::MountainPassRoad,
            ),
            route(
                2,
                MainlandCityRole::WesternHarborCapital,
                MainlandCityRole::SouthernSandyCity,
                TradeRouteClass::PrimaryRoad,
            ),
            route(
                3,
                MainlandCityRole::MountainCity,
                MainlandCityRole::SouthernSandyCity,
                TradeRouteClass::PrimaryRoad,
            ),
        ],
        massifs: vec![MountainMassifPlan {
            id: feature_id(seed, 0x4d41_5353, 1),
            ridge_spine: vec![massif_west, mountain, massif_east],
            terraces: vec![
                StructuralTerracePlan {
                    tier: StructuralTier(1),
                    contour_controls: vec![
                        massif_west,
                        massif_north,
                        massif_east,
                        massif_south,
                    ],
                },
                StructuralTerracePlan {
                    tier: StructuralTier(2),
                    contour_controls: vec![
                        tier_two_west,
                        tier_two_north,
                        tier_two_east,
                        tier_two_south,
                    ],
                },
            ],
            cave_candidates: vec![CaveEntranceCandidate {
                anchor: cave,
                host_tier: StructuralTier(1),
                preferred_facing: CardinalFacing::South,
                cave_system_id: feature_id(seed, 0x4341_5645, 1),
            }],
            drainage_notches: vec![massif_south],
            pass_candidates: vec![bounds.point(1, 2, 2, 5)],
        }],
        coastal_cliffs: vec![],
        expedition_gateway: MainlandCityRole::WesternHarborCapital,
    }
}

fn feature_id(seed: u64, domain: u64, salt: u64) -> WorldFeatureId {
    let mut value = seed ^ domain.rotate_left(17) ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    WorldFeatureId(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunks() -> Vec<ChunkCoord> {
        (0..5)
            .flat_map(|y| (0..8).map(move |x| ChunkCoord::new(x, y)))
            .collect()
    }

    #[test]
    fn mainland_bridge_carries_valid_three_city_target_without_changing_materializer_mode() {
        let plan = SurfaceWorldPlanV1::build(
            0,
            "Alderreach",
            "havenwild_mainland",
            1_337,
            chunks(),
            ProjectSceneId::new("pcg_havenwild_mainland_0_4"),
        )
        .expect("mainland compatibility plan");
        assert_eq!(plan.materialization_mode, SurfacePlanMaterializationMode::LegacyVisualParity);
        assert_eq!(plan.target_mainland.as_ref().expect("target").cities.len(), 3);
        plan.validate().expect("valid target feature plan");
    }

    #[test]
    fn compatibility_seed_matches_pre_bridge_landmass_seed_expression() {
        let mainland = SurfaceWorldPlanV1::build(
            0,
            "Alderreach",
            "havenwild_mainland",
            9_001,
            chunks(),
            ProjectSceneId::new("pcg_havenwild_mainland_0_4"),
        )
        .expect("mainland");
        assert_eq!(mainland.compatibility_landmass_seed(), 9_001);

        let island = SurfaceWorldPlanV1::build(
            7,
            "Outer Isle",
            "outer_isle",
            42,
            vec![ChunkCoord::new(0, 0)],
            ProjectSceneId::new("pcg_outer_isle_0_0"),
        )
        .expect("island");
        assert_eq!(island.compatibility_landmass_seed(), 42 ^ 7);
        assert!(island.target_mainland.is_none());
    }
}
