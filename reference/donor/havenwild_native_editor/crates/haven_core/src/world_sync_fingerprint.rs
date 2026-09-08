use crate::{GameWorld, ProjectSceneId, SceneMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneSyncFingerprint {
    pub scene_id: ProjectSceneId,
    pub hash: String,
    pub tiles: usize,
    pub objects: usize,
    pub stamps: usize,
    pub transitions: usize,
    pub autotile_overrides: usize,
    pub zones: usize,
    pub tile_kinds: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSyncFingerprint {
    pub hash: String,
    pub active_scene: ProjectSceneId,
    pub scenes: usize,
    pub tile_rules: usize,
    pub scene_fingerprints: Vec<SceneSyncFingerprint>,
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hash_debug(value: &impl std::fmt::Debug) -> String {
    format!("{:016x}", fnv1a64(format!("{value:?}").as_bytes()))
}

pub fn fingerprint_scene(scene: &SceneMap) -> SceneSyncFingerprint {
    SceneSyncFingerprint {
        scene_id: scene.id.clone(),
        hash: hash_debug(scene),
        tiles: scene.map.tiles.len(),
        objects: scene.map.objects.len(),
        stamps: scene.map.stamps.len(),
        transitions: scene.transitions.len(),
        autotile_overrides: scene.autotile_overrides.len(),
        zones: scene.zones.len(),
        tile_kinds: {
            let mut counts = std::collections::BTreeMap::<String, usize>::new();
            for tile in &scene.map.tiles {
                *counts.entry(format!("{:?}", tile)).or_default() += 1;
            }
            counts.into_iter()
                .map(|(kind, count)| format!("{kind}:{count}"))
                .collect::<Vec<_>>()
                .join(",")
        },
    }
}

pub fn fingerprint_world(world: &GameWorld) -> WorldSyncFingerprint {
    let scene_fingerprints = world.scenes.iter().map(fingerprint_scene).collect::<Vec<_>>();
    // Hash the ordered semantic scene hashes plus the world-owned fields. This
    // intentionally excludes SceneRegistry's HashMap lookup index.
    let canonical = (
        world.active_scene.code().to_string(),
        format!("{:?}", world.tile_rules),
        scene_fingerprints
            .iter()
            .map(|scene| (scene.scene_id.to_string(), scene.hash.clone()))
            .collect::<Vec<_>>(),
    );
    WorldSyncFingerprint {
        hash: hash_debug(&canonical),
        active_scene: world.active_scene_id().clone(),
        scenes: world.scenes.len(),
        tile_rules: world.tile_rules.len(),
        scene_fingerprints,
    }
}

pub fn format_world_sync(label: &str, world: &GameWorld, focus_scene: Option<&ProjectSceneId>) -> String {
    let fingerprint = fingerprint_world(world);
    let mut text = format!(
        "[DEV-SYNC:{label}] world={} active={} scenes={} tile_rules={}",
        fingerprint.hash,
        fingerprint.active_scene,
        fingerprint.scenes,
        fingerprint.tile_rules
    );
    if let Some(scene_id) = focus_scene {
        if let Some(scene) = fingerprint
            .scene_fingerprints
            .iter()
            .find(|scene| &scene.scene_id == scene_id)
        {
            text.push_str(&format!(
                " scene={} scene_hash={} tiles={} objects={} stamps={} transitions={} overrides={} zones={} tile_kinds=[{}]",
                scene.scene_id,
                scene.hash,
                scene.tiles,
                scene.objects,
                scene.stamps,
                scene.transitions,
                scene.autotile_overrides,
                scene.zones,
                scene.tile_kinds
            ));
        } else {
            text.push_str(&format!(" scene={} MISSING", scene_id));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_clone_and_changes_with_scene_content() {
        let world = GameWorld::starter();
        let cloned = world.clone();
        assert_eq!(fingerprint_world(&world), fingerprint_world(&cloned));

        let mut changed = cloned;
        changed.active_mut().spawn_x += 1;
        assert_ne!(fingerprint_world(&world), fingerprint_world(&changed));
    }
}
