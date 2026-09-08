use haven_core::{SceneMap, TileKind, STRUCTURAL_LEVEL_AUTO};
use haven_world::{
    resolve_tavern_map_elevation_cliffs_v2, ElevationCliffSettingsV2, LegacyCliffBridgeResultV2,
};

/// Cheap editor-side cache for the canonical structural cliff bake.
///
/// The signature includes only inputs that can change structural topology:
/// explicit structural levels and the legacy MountainRock fallback bit. Raw
/// geological height and ordinary surface material changes intentionally do not
/// invalidate the cliff bake.
#[derive(Clone, Debug, Default)]
pub(crate) struct StructuralCliffPreviewCache {
    signature: u64,
    initialized: bool,
    bridge: Option<LegacyCliffBridgeResultV2>,
    last_error: Option<String>,
}

impl StructuralCliffPreviewCache {
    pub(crate) fn synchronize(&mut self, scene: &SceneMap) {
        let signature = structural_signature(scene);
        if self.initialized && signature == self.signature {
            return;
        }
        self.initialized = true;
        self.signature = signature;
        match resolve_tavern_map_elevation_cliffs_v2(
            &scene.map,
            ElevationCliffSettingsV2::default(),
        ) {
            Ok(bridge) => {
                self.bridge = Some(bridge);
                self.last_error = None;
            }
            Err(error) => {
                self.bridge = None;
                self.last_error = Some(error);
            }
        }
    }

    pub(crate) fn bridge(&self) -> Option<&LegacyCliffBridgeResultV2> {
        self.bridge.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

fn structural_signature(scene: &SceneMap) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    let len = scene
        .map
        .structural_levels
        .len()
        .max(scene.map.tiles.len());
    for index in 0..len {
        let level = scene
            .map
            .structural_levels
            .get(index)
            .copied()
            .unwrap_or(STRUCTURAL_LEVEL_AUTO);
        let legacy_raised = scene
            .map
            .tiles
            .get(index)
            .is_some_and(|tile| *tile == TileKind::MountainRock);
        hash ^= u64::from(level);
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= u64::from(legacy_raised);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_rebuilds_when_structural_level_changes() {
        let mut scene = SceneMap::blank(
            haven_core::ProjectSceneId::new("test"),
            "Test",
            haven_core::SceneKind::Exterior,
            haven_core::SceneBiome::Temperate,
        );
        let before = structural_signature(&scene);
        scene.map.set_structural_level(10, 10, Some(1));
        let after = structural_signature(&scene);
        assert_ne!(before, after);
    }

    #[test]
    fn editor_bridge_feeds_the_shared_visual_recipe() {
        let mut scene = SceneMap::blank(
            haven_core::ProjectSceneId::new("test"),
            "Test",
            haven_core::SceneKind::Exterior,
            haven_core::SceneBiome::Temperate,
        );
        scene.map.set_structural_level(10, 10, Some(1));
        let bridge = resolve_tavern_map_elevation_cliffs_v2(
            &scene.map,
            ElevationCliffSettingsV2::default(),
        )
        .expect("editor structural bridge");
        let center = bridge.structural_at(10, 10).expect("raised host");
        let recipe = haven_render::resolve_cliff_visual_recipe_v1(center, |dx, dy| {
            bridge.structural_at(10 + dx, 10 + dy)
        })
        .expect("shared visual recipe");

        assert_eq!(recipe.shape, haven_world::CliffShape15::Isolated);
        assert_eq!(recipe.south_face_segments, 1);
        assert_eq!(
            recipe.visual_shape,
            haven_render::CliffVisualShape::SouthAuthoredTerminal
        );
    }

    #[test]
    fn editor_bridge_preserves_discrete_cliff_height_one_through_four() {
        for level in 1_u8..=4 {
            let mut scene = SceneMap::blank(
                haven_core::ProjectSceneId::new(format!("height_{level}")),
                "Height",
                haven_core::SceneKind::Exterior,
                haven_core::SceneBiome::Temperate,
            );
            scene
                .map
                .set_structural_level(10, 10, Some(level));
            let bridge = resolve_tavern_map_elevation_cliffs_v2(
                &scene.map,
                ElevationCliffSettingsV2::default(),
            )
            .expect("editor structural bridge");
            let center = bridge.structural_at(10, 10).expect("raised host");
            let recipe = haven_render::resolve_cliff_visual_recipe_v1(center, |dx, dy| {
                bridge.structural_at(10 + dx, 10 + dy)
            })
            .expect("shared visual recipe");

            assert_eq!(recipe.south_face_segments, level);
            assert_eq!(
                haven_render::uniform_south_face_receiver_rows(recipe.south_face_segments),
                usize::from(level),
            );
        }
    }
}

