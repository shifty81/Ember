use std::collections::HashMap;
use std::ops::{Deref, Index};

use crate::{ProjectSceneId, SceneMap};

/// Ordered project scene storage with stable identifier lookup.
///
/// The editor still presents scenes in insertion order, while runtime and authoring
/// systems can resolve a scene directly by `ProjectSceneId`. Scene identifiers must
/// be changed through `rename` so the index remains valid.
#[derive(Clone, Debug, Default)]
pub struct SceneRegistry {
    ordered: Vec<SceneMap>,
    index_by_id: HashMap<ProjectSceneId, usize>,
}

impl SceneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_scenes(scenes: Vec<SceneMap>) -> Result<Self, String> {
        let mut registry = Self {
            ordered: scenes,
            index_by_id: HashMap::new(),
        };
        registry.rebuild_index()?;
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    pub fn as_slice(&self) -> &[SceneMap] {
        &self.ordered
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SceneMap> {
        self.ordered.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, SceneMap> {
        self.ordered.iter_mut()
    }

    pub fn get_at(&self, index: usize) -> Option<&SceneMap> {
        self.ordered.get(index)
    }

    pub fn get_at_mut(&mut self, index: usize) -> Option<&mut SceneMap> {
        self.ordered.get_mut(index)
    }

    pub fn first(&self) -> Option<&SceneMap> {
        self.ordered.first()
    }

    pub fn first_mut(&mut self) -> Option<&mut SceneMap> {
        self.ordered.first_mut()
    }

    pub fn by_id(&self, id: &ProjectSceneId) -> Option<&SceneMap> {
        self.index_by_id
            .get(id)
            .and_then(|index| self.ordered.get(*index))
            .filter(|scene| &scene.id == id)
            .or_else(|| self.ordered.iter().find(|scene| &scene.id == id))
    }

    pub fn by_id_mut(&mut self, id: &ProjectSceneId) -> Option<&mut SceneMap> {
        let indexed = self.index_by_id.get(id).copied().and_then(|index| {
            self.ordered
                .get(index)
                .filter(|scene| &scene.id == id)
                .map(|_| index)
        });
        let index = indexed.or_else(|| self.ordered.iter().position(|scene| &scene.id == id))?;
        self.ordered.get_mut(index)
    }

    pub fn contains(&self, id: &ProjectSceneId) -> bool {
        self.by_id(id).is_some()
    }

    pub fn position(&self, id: &ProjectSceneId) -> Option<usize> {
        self.index_by_id
            .get(id)
            .copied()
            .filter(|index| {
                self.ordered
                    .get(*index)
                    .is_some_and(|scene| &scene.id == id)
            })
            .or_else(|| self.ordered.iter().position(|scene| &scene.id == id))
    }

    pub fn ordered_ids(&self) -> impl Iterator<Item = &ProjectSceneId> {
        self.ordered.iter().map(|scene| &scene.id)
    }

    pub fn insert(&mut self, scene: SceneMap) -> Result<usize, String> {
        let index = self.ordered.len();
        self.insert_at(index, scene)
    }

    pub fn insert_at(&mut self, index: usize, scene: SceneMap) -> Result<usize, String> {
        if self.contains(&scene.id) {
            return Err(format!("scene '{}' is already registered", scene.id));
        }
        let index = index.min(self.ordered.len());
        self.ordered.insert(index, scene);
        self.rebuild_index()?;
        Ok(index)
    }

    pub fn remove(&mut self, id: &ProjectSceneId) -> Option<SceneMap> {
        let index = self.position(id)?;
        let removed = self.ordered.remove(index);
        self.rebuild_index()
            .expect("removing a scene cannot introduce duplicate scene identifiers");
        Some(removed)
    }

    pub fn rename(
        &mut self,
        current: &ProjectSceneId,
        replacement: ProjectSceneId,
    ) -> Result<(), String> {
        if current == &replacement {
            return Ok(());
        }
        if self.contains(&replacement) {
            return Err(format!("scene '{}' is already registered", replacement));
        }
        let index = self
            .position(current)
            .ok_or_else(|| format!("scene '{}' is not registered", current))?;
        self.ordered[index].id = replacement;
        self.rebuild_index()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.ordered.len() != self.index_by_id.len() {
            return Err(format!(
                "scene registry index has {} entries for {} scenes",
                self.index_by_id.len(),
                self.ordered.len()
            ));
        }
        for (index, scene) in self.ordered.iter().enumerate() {
            match self.index_by_id.get(&scene.id) {
                Some(indexed) if *indexed == index => {}
                Some(indexed) => {
                    return Err(format!(
                        "scene '{}' is indexed at {}, expected {}",
                        scene.id, indexed, index
                    ));
                }
                None => return Err(format!("scene '{}' is missing from the index", scene.id)),
            }
        }
        Ok(())
    }

    pub fn rebuild_index(&mut self) -> Result<(), String> {
        let mut index_by_id = HashMap::with_capacity(self.ordered.len());
        for (index, scene) in self.ordered.iter().enumerate() {
            if index_by_id.insert(scene.id.clone(), index).is_some() {
                return Err(format!("duplicate scene identifier '{}'", scene.id));
            }
        }
        self.index_by_id = index_by_id;
        Ok(())
    }
}

impl From<Vec<SceneMap>> for SceneRegistry {
    fn from(scenes: Vec<SceneMap>) -> Self {
        Self::from_scenes(scenes).expect("scene registry requires unique scene identifiers")
    }
}

impl Deref for SceneRegistry {
    type Target = [SceneMap];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Index<usize> for SceneRegistry {
    type Output = SceneMap;

    fn index(&self, index: usize) -> &Self::Output {
        &self.ordered[index]
    }
}

impl<'a> IntoIterator for &'a SceneRegistry {
    type Item = &'a SceneMap;
    type IntoIter = std::slice::Iter<'a, SceneMap>;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.iter()
    }
}

impl<'a> IntoIterator for &'a mut SceneRegistry {
    type Item = &'a mut SceneMap;
    type IntoIter = std::slice::IterMut<'a, SceneMap>;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.iter_mut()
    }
}

impl IntoIterator for SceneRegistry {
    type Item = SceneMap;
    type IntoIter = std::vec::IntoIter<SceneMap>;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SceneId, SceneKind};

    #[test]
    fn registry_preserves_order_and_resolves_project_ids() {
        let farmstead = SceneMap::starter(SceneId::Farmstead, SceneKind::Exterior, 1, 1);
        let cellar = SceneMap::starter(SceneId::Cellar, SceneKind::Cave, 2, 2);
        let expected_cellar_spawn_x = cellar.spawn_x;
        let registry = SceneRegistry::from(vec![farmstead, cellar]);

        assert_eq!(registry.len(), 2);
        assert_eq!(registry[0].id.code(), "farmstead");
        assert_eq!(
            registry
                .by_id(&ProjectSceneId::new("cellar"))
                .unwrap()
                .spawn_x,
            expected_cellar_spawn_x
        );
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn registry_rejects_duplicate_ids() {
        let scene = SceneMap::starter(SceneId::Farmstead, SceneKind::Exterior, 1, 1);
        let duplicate = scene.clone();
        assert!(SceneRegistry::from_scenes(vec![scene, duplicate]).is_err());
    }

    #[test]
    fn insert_at_preserves_requested_authoring_order() {
        let mut registry = SceneRegistry::from(vec![
            SceneMap::starter(SceneId::Farmstead, SceneKind::Exterior, 1, 1),
            SceneMap::starter(SceneId::Cellar, SceneKind::Cave, 2, 2),
        ]);
        let inserted = SceneMap::blank(
            ProjectSceneId::new("mid_scene"),
            "Mid Scene",
            SceneKind::Interior,
            crate::SceneBiome::Temperate,
        );
        assert_eq!(registry.insert_at(1, inserted).expect("insert scene"), 1);
        assert_eq!(registry[0].id.code(), "farmstead");
        assert_eq!(registry[1].id.code(), "mid_scene");
        assert_eq!(registry[2].id.code(), "cellar");
        assert!(registry.validate().is_ok());
    }
}
