use super::scene_size_migration::{
    center_legacy_zone_grid, default_biome_for_project_scene, migrate_legacy_point,
    migrate_legacy_transition, scene_fill_for_kind,
};
use super::starter_scene_rules as starter_rules;
use super::*;
use crate::{ProjectSceneId, SceneDimensions, SceneReference, SceneRegistry, TileAutoGroup};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneMap {
    pub id: ProjectSceneId,
    pub kind: SceneKind,
    pub biome: SceneBiome,
    pub name: String,
    /// Logical scene bounds. Do not infer active bounds from MAP_W/MAP_H.
    pub dimensions: SceneDimensions,
    pub map: TavernMap,
    pub zones: Vec<ZoneKind>,
    pub transitions: Vec<Transition>,
    pub autotile_overrides: Vec<AutotileOverride>,
    pub spawn_x: i32,
    pub spawn_y: i32,
}

#[derive(Clone, Debug)]
pub struct GameWorld {
    pub scenes: SceneRegistry,
    pub active_scene: SceneReference,
    pub tile_rules: Vec<TileInteraction>,
}

impl SceneMap {
    pub fn blank(
        id: impl Into<ProjectSceneId>,
        name: impl Into<String>,
        kind: SceneKind,
        biome: SceneBiome,
    ) -> Self {
        let fill = match kind {
            SceneKind::Exterior => TileKind::Grass,
            SceneKind::Interior => TileKind::Wall,
            SceneKind::Cave => TileKind::CaveWall,
        };
        Self {
            id: id.into(),
            kind,
            biome,
            name: name.into(),
            dimensions: SceneDimensions::legacy_canvas(),
            map: TavernMap::empty_with(fill),
            zones: vec![ZoneKind::None; MAP_W * MAP_H],
            transitions: Vec::new(),
            autotile_overrides: Vec::new(),
            spawn_x: (MAP_W / 2) as i32,
            spawn_y: (MAP_H / 2) as i32,
        }
    }

    pub fn blank_sized(
        id: impl Into<ProjectSceneId>,
        name: impl Into<String>,
        kind: SceneKind,
        biome: SceneBiome,
        dimensions: SceneDimensions,
    ) -> Result<Self, String> {
        let dimensions = dimensions.validate()?;
        let mut scene = Self::blank(id, name, kind, biome);
        scene.dimensions = dimensions;
        scene.spawn_x = (dimensions.width / 2) as i32;
        scene.spawn_y = (dimensions.height / 2) as i32;
        Ok(scene)
    }

    pub fn contains_cell(&self, x: i32, y: i32) -> bool { self.dimensions.contains(x, y) }

    pub fn starter(id: SceneId, kind: SceneKind, spawn_x: i32, spawn_y: i32) -> Self {
        Self::starter_seeded(id, kind, spawn_x, spawn_y, scene_seed(id))
    }

    pub fn starter_seeded(
        id: SceneId,
        kind: SceneKind,
        spawn_x: i32,
        spawn_y: i32,
        seed: u32,
    ) -> Self {
        let mut map = TavernMap::starter_for_seed(id, seed);
        let mut zones = center_legacy_zone_grid(starter_rules::starter_zones(id));
        if let Some(layers) = starter_rules::layered_scene_for(id, seed) {
            layers.apply_to_map(&mut map);
            zones = layers.zones;
        }
        let transitions = starter_rules::starter_transitions(id)
            .into_iter()
            .map(migrate_legacy_transition)
            .collect::<Vec<_>>();
        let (spawn_x, spawn_y) = migrate_legacy_point(spawn_x, spawn_y);
        starter_rules::ensure_scene_access(
            &mut map,
            &mut zones,
            id,
            kind,
            spawn_x,
            spawn_y,
            &transitions,
        );

        Self {
            id: ProjectSceneId::from(id),
            kind,
            biome: starter_rules::starter_biome(id, kind),
            name: id.label().to_string(),
            dimensions: SceneDimensions::legacy_canvas(),
            map,
            zones,
            transitions,
            autotile_overrides: Vec::new(),
            spawn_x,
            spawn_y,
        }
    }

    pub fn autotile_override_at(&self, x: i32, y: i32) -> Option<AutotileOverride> {
        self.autotile_overrides
            .iter()
            .copied()
            .find(|entry| entry.x == x && entry.y == y)
    }

    pub fn set_autotile_override(&mut self, override_value: AutotileOverride) {
        if let Some(index) = self
            .autotile_overrides
            .iter()
            .position(|entry| entry.x == override_value.x && entry.y == override_value.y)
        {
            self.autotile_overrides[index] = override_value;
        } else {
            self.autotile_overrides.push(override_value);
        }
    }

    pub fn clear_autotile_override(&mut self, x: i32, y: i32) -> Option<AutotileOverride> {
        let index = self
            .autotile_overrides
            .iter()
            .position(|entry| entry.x == x && entry.y == y)?;
        Some(self.autotile_overrides.remove(index))
    }

    pub fn prune_autotile_overrides(&mut self) -> usize {
        let before = self.autotile_overrides.len();
        let map = &self.map;
        self.autotile_overrides.retain(|entry| {
            TavernMap::idx(entry.x, entry.y).is_some()
                && map.get(entry.x, entry.y).autotile_group() == Some(entry.group)
        });
        before.saturating_sub(self.autotile_overrides.len())
    }

    pub fn transition_at(&self, x: i32, y: i32) -> Option<&Transition> {
        self.transitions
            .iter()
            .find(|transition| transition.contains(x, y))
    }

    pub fn transition_id_at(&self, x: i32, y: i32) -> Option<TransitionId> {
        self.transition_at(x, y).map(|transition| transition.id)
    }

    pub fn transition(&self, id: TransitionId) -> Option<&Transition> {
        self.transitions
            .iter()
            .find(|transition| transition.id == id)
    }

    pub fn transition_mut(&mut self, id: TransitionId) -> Option<&mut Transition> {
        self.transitions
            .iter_mut()
            .find(|transition| transition.id == id)
    }

    pub fn next_transition_id(&self) -> TransitionId {
        TransitionId::next_after(self.transitions.iter().map(|transition| transition.id))
    }

    pub fn insert_transition(&mut self, mut transition: Transition) -> TransitionId {
        if !transition.id.is_assigned() || self.transition(transition.id).is_some() {
            transition.id = self.next_transition_id();
        }
        let id = transition.id;
        self.transitions.push(transition);
        id
    }

    pub fn remove_transition(&mut self, id: TransitionId) -> Option<Transition> {
        let index = self
            .transitions
            .iter()
            .position(|transition| transition.id == id)?;
        Some(self.transitions.remove(index))
    }

    pub fn zone_at(&self, x: i32, y: i32) -> ZoneKind {
        if !self.contains_cell(x, y) { return ZoneKind::None; }
        TavernMap::idx(x, y).map_or(ZoneKind::None, |idx| self.zones[idx])
    }

    pub fn set_zone(&mut self, x: i32, y: i32, zone: ZoneKind) {
        if !self.contains_cell(x, y) { return; }
        if let Some(idx) = TavernMap::idx(x, y) {
            self.zones[idx] = zone;
        }
    }

    pub fn is_cell_walkable(&self, x: i32, y: i32) -> bool {
        self.contains_cell(x, y) && self.map.is_cell_walkable(x, y)
    }

    pub fn transition_at_mut(&mut self, x: i32, y: i32) -> Option<&mut Transition> {
        self.transitions
            .iter_mut()
            .find(|transition| transition.contains(x, y))
    }
}

impl GameWorld {
    pub fn starter() -> Self {
        Self {
            active_scene: SceneId::Farmstead.into(),
            tile_rules: starter_rules::default_tile_rules(),
            scenes: SceneRegistry::from(vec![
                SceneMap::starter(SceneId::Farmstead, SceneKind::Exterior, 23, 13),
                SceneMap::starter(SceneId::TavernInterior, SceneKind::Interior, 23, 23),
                SceneMap::starter(SceneId::Cellar, SceneKind::Cave, 10, 21),
                SceneMap::starter(SceneId::GuestFloor, SceneKind::Interior, 34, 20),
                SceneMap::starter(SceneId::NorthRoad, SceneKind::Exterior, 24, 16),
                SceneMap::starter(SceneId::SouthField, SceneKind::Exterior, 24, 3),
                SceneMap::starter(SceneId::EastWoods, SceneKind::Exterior, 2, 15),
                SceneMap::starter(SceneId::CaveMouth, SceneKind::Cave, 8, 22),
                SceneMap::starter(SceneId::CaveDepths, SceneKind::Cave, 6, 23),
            ]),
        }
    }

    pub fn active(&self) -> &SceneMap {
        self.scene_by_reference(&self.active_scene)
            .expect("active scene reference must resolve to a loaded scene")
    }

    pub fn active_mut(&mut self) -> &mut SceneMap {
        let active_scene = self.active_scene.clone();
        self.scene_mut_by_reference(&active_scene)
            .expect("active scene reference must resolve to a loaded scene")
    }

    pub fn active_scene_id(&self) -> &ProjectSceneId {
        self.active_scene.project_id()
    }

    pub fn active_legacy_scene_id(&self) -> Option<SceneId> {
        self.active_scene.legacy_scene_id()
    }

    pub fn scene(&self, id: impl Into<ProjectSceneId>) -> Option<&SceneMap> {
        let id = id.into();
        self.scene_by_id(&id)
    }

    pub fn scene_mut(&mut self, id: impl Into<ProjectSceneId>) -> Option<&mut SceneMap> {
        let id = id.into();
        self.scene_mut_by_id(&id)
    }

    pub fn scene_by_id(&self, id: &ProjectSceneId) -> Option<&SceneMap> {
        self.scenes.by_id(id)
    }

    pub fn scene_mut_by_id(&mut self, id: &ProjectSceneId) -> Option<&mut SceneMap> {
        self.scenes.by_id_mut(id)
    }

    pub fn scene_by_reference(&self, reference: &SceneReference) -> Option<&SceneMap> {
        self.scene_by_id(reference.project_id())
    }

    pub fn scene_mut_by_reference(&mut self, reference: &SceneReference) -> Option<&mut SceneMap> {
        self.scene_mut_by_id(reference.project_id())
    }

    pub fn set_active_scene(&mut self, reference: impl Into<SceneReference>) -> Result<(), String> {
        let reference = reference.into();
        if self.scene_by_reference(&reference).is_none() {
            return Err(format!(
                "scene reference '{}' does not resolve to a loaded scene",
                reference.code()
            ));
        }
        self.active_scene = reference;
        Ok(())
    }

    pub fn insert_scene(&mut self, scene: SceneMap) -> Result<usize, String> {
        self.scenes.insert(scene)
    }

    pub fn insert_scene_at(&mut self, index: usize, scene: SceneMap) -> Result<usize, String> {
        self.scenes.insert_at(index, scene)
    }

    pub fn duplicate_scene(
        &mut self,
        source: &ProjectSceneId,
        replacement: ProjectSceneId,
        display_name: impl Into<String>,
    ) -> Result<usize, String> {
        let mut duplicate = self
            .scene_by_id(source)
            .cloned()
            .ok_or_else(|| format!("scene '{}' is not registered", source))?;
        duplicate.id = replacement;
        duplicate.name = display_name.into();
        self.scenes.insert(duplicate)
    }

    pub fn rename_scene(
        &mut self,
        current: &ProjectSceneId,
        replacement: ProjectSceneId,
    ) -> Result<(), String> {
        self.scenes.rename(current, replacement.clone())?;
        if self.active_scene.project_id() == current {
            self.active_scene = SceneReference::from(replacement.clone());
        }
        for scene in self.scenes.iter_mut() {
            for transition in &mut scene.transitions {
                if transition.target.project_id() == current {
                    transition.target = SceneReference::from(replacement.clone());
                }
            }
        }
        Ok(())
    }

    pub fn remove_scene(&mut self, id: &ProjectSceneId) -> Result<SceneMap, String> {
        if self.active_scene.project_id() == id {
            return Err(format!("cannot remove active scene '{}'", id));
        }
        let inbound = self
            .scenes
            .iter()
            .filter(|scene| {
                scene
                    .transitions
                    .iter()
                    .any(|transition| transition.target.project_id() == id)
            })
            .map(|scene| scene.id.code().to_string())
            .collect::<Vec<_>>();
        if !inbound.is_empty() {
            return Err(format!(
                "cannot remove scene '{}'; referenced by {}",
                id,
                inbound.join(", ")
            ));
        }
        self.scenes
            .remove(id)
            .ok_or_else(|| format!("scene '{}' is not registered", id))
    }

    pub fn cycle_scene(&mut self, direction: i32) -> (i32, i32) {
        if self.scenes.is_empty() {
            return (0, 0);
        }
        let index = self
            .scenes
            .position(self.active_scene.project_id())
            .unwrap_or(0);
        let next = (index as i32 + direction).rem_euclid(self.scenes.len() as i32) as usize;
        let next_id = self.scenes[next].id.clone();
        self.active_scene = SceneReference::from(next_id);
        let scene = self.active();
        (scene.spawn_x, scene.spawn_y)
    }

    pub fn tile_rule(&self, tile: TileKind) -> TileInteraction {
        self.tile_rules
            .get(starter_rules::tile_index(tile))
            .copied()
            .unwrap_or_else(|| starter_rules::default_tile_rule(tile))
    }

    pub fn set_tile_rule(&mut self, tile: TileKind, interaction: TileInteraction) {
        if self.tile_rules.len() != TileKind::ALL.len() {
            self.tile_rules = starter_rules::default_tile_rules();
        }
        self.tile_rules[starter_rules::tile_index(tile)] = interaction;
    }

    pub fn serialize_lines(&self) -> String {
        let mut out = format!("tavern_world 1 active {}\n", self.active_scene.code());
        out.push_str("tile_rules\n");
        for tile in TileKind::ALL {
            out.push_str(&format!(
                "tile_rule {} {}\n",
                tile.code(),
                self.tile_rule(tile).code()
            ));
        }
        for scene in &self.scenes {
            out.push_str(&format!(
                "scene {} {} {} {} {} {} {} {}\n",
                scene.id.code(),
                scene.kind.code(),
                scene.biome.code(),
                scene.dimensions.width,
                scene.dimensions.height,
                scene.spawn_x,
                scene.spawn_y,
                scene.name.replace(' ', "_")
            ));
            out.push_str(&scene.map.serialize_lines());
            out.push_str("zones\n");
            for y in 0..MAP_H as i32 {
                out.push_str("zone_row");
                for x in 0..MAP_W as i32 {
                    out.push(' ');
                    out.push_str(scene.zone_at(x, y).code());
                }
                out.push('\n');
            }
            for override_value in &scene.autotile_overrides {
                out.push_str(&format!(
                    "autotile_override {} {} {} {}\n",
                    override_value.x,
                    override_value.y,
                    override_value.group.code(),
                    override_value.mask
                ));
            }
            for transition in &scene.transitions {
                out.push_str(&format!(
                    "transition {} {} {} {} {} {} {} {} {}\n",
                    transition.id.code(),
                    transition.x,
                    transition.y,
                    transition.w,
                    transition.h,
                    transition.target.code(),
                    transition.spawn_x,
                    transition.spawn_y,
                    transition.label.replace(' ', "_")
                ));
            }
            out.push_str("end_scene\n");
        }
        out
    }

    pub fn deserialize_lines(input: &str) -> Result<Self, String> {
        let mut lines = input.lines().peekable();
        let header = lines.next().ok_or("missing world header")?;
        let header_parts: Vec<_> = header.split_whitespace().collect();
        if header_parts.len() != 4
            || header_parts[0] != "tavern_world"
            || header_parts[1] != "1"
            || header_parts[2] != "active"
        {
            return Err(format!("unsupported world header: {header}"));
        }
        let active_scene = SceneReference::from(header_parts[3]);
        let mut scenes = Vec::new();
        let mut tile_rules = starter_rules::default_tile_rules();

        if lines.peek() == Some(&"tile_rules") {
            lines.next();
            while let Some(line) = lines.peek() {
                if line.starts_with("scene ") {
                    break;
                }
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() == 3 && parts[0] == "tile_rule" {
                    if let (Some(tile), Some(rule)) = (
                        TileKind::from_code(parts[1]),
                        TileInteraction::from_code(parts[2]),
                    ) {
                        tile_rules[starter_rules::tile_index(tile)] = rule;
                    }
                }
                lines.next();
            }
        }

        while let Some(line) = lines.next() {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            if (parts.len() != 6 && parts.len() != 7 && parts.len() != 9) || parts[0] != "scene" {
                return Err(format!("expected scene header, got {line}"));
            }
            let id = ProjectSceneId::new(parts[1]);
            let kind = SceneKind::from_code(parts[2]).ok_or("bad scene kind")?;
            let (biome, dimensions, spawn_x_index, spawn_y_index, name_index) = if parts.len() == 9 {
                let dimensions = SceneDimensions::new(
                    parts[4].parse::<usize>().map_err(|_| "bad scene width")?,
                    parts[5].parse::<usize>().map_err(|_| "bad scene height")?,
                ).validate()?;
                (SceneBiome::from_code(parts[3]).ok_or("bad scene biome")?, dimensions, 6, 7, 8)
            } else if parts.len() == 7 {
                (SceneBiome::from_code(parts[3]).ok_or("bad scene biome")?, SceneDimensions::legacy_canvas(), 4, 5, 6)
            } else {
                (default_biome_for_project_scene(&id, kind), SceneDimensions::legacy_canvas(), 3, 4, 5)
            };
            let spawn_x = parts[spawn_x_index]
                .parse::<i32>()
                .map_err(|_| "bad spawn x")?;
            let spawn_y = parts[spawn_y_index]
                .parse::<i32>()
                .map_err(|_| "bad spawn y")?;
            let name = parts[name_index].replace('_', " ");

            let map_header = lines.next().ok_or("missing map header")?;
            let (source_w, source_h) = parse_tavern_map_dimensions(map_header)?;
            let (offset_x, offset_y) = scene_dimension_offset(source_w, source_h);
            let spawn_x = spawn_x + offset_x;
            let spawn_y = spawn_y + offset_y;
            let mut map_blob = String::new();
            map_blob.push_str(map_header);
            map_blob.push('\n');
            for _ in 0..source_h {
                map_blob.push_str(lines.next().ok_or("missing map row")?);
                map_blob.push('\n');
            }

            let mut object_lines = Vec::new();
            let mut transitions: Vec<Transition> = Vec::new();
            let mut autotile_overrides: Vec<AutotileOverride> = Vec::new();
            let mut zones = vec![ZoneKind::None; MAP_W * MAP_H];
            while let Some(next) = lines.next() {
                let next_parts: Vec<_> = next.split_whitespace().collect();
                if next_parts.first() == Some(&"end_scene") {
                    break;
                }
                if next_parts.first() == Some(&"zones") {
                    for y in 0..source_h {
                        let row = lines.next().ok_or("missing zone row")?;
                        let row_parts: Vec<_> = row.split_whitespace().collect();
                        if row_parts.len() != source_w + 1 || row_parts[0] != "zone_row" {
                            return Err(format!("invalid zone row {}", y + 1));
                        }
                        for x in 0..source_w {
                            let target_x = x as i32 + offset_x;
                            let target_y = y as i32 + offset_y;
                            if let Some(index) = TavernMap::idx(target_x, target_y) {
                                zones[index] = ZoneKind::from_code(row_parts[x + 1])
                                    .ok_or_else(|| format!("unknown zone {}", row_parts[x + 1]))?;
                            }
                        }
                    }
                    continue;
                }
                if next_parts.first() == Some(&"autotile_override") {
                    if next_parts.len() != 5 {
                        return Err(format!("bad autotile override: {next}"));
                    }
                    let x = next_parts[1]
                        .parse::<i32>()
                        .map_err(|_| "bad autotile override x")?;
                    let y = next_parts[2]
                        .parse::<i32>()
                        .map_err(|_| "bad autotile override y")?;
                    let group = TileAutoGroup::from_code(next_parts[3])
                        .ok_or_else(|| format!("bad autotile group {}", next_parts[3]))?;
                    let mask = next_parts[4]
                        .parse::<u8>()
                        .map_err(|_| "bad autotile override mask")?;
                    autotile_overrides.push(AutotileOverride::new(
                        x + offset_x,
                        y + offset_y,
                        group,
                        mask,
                    ));
                    continue;
                }
                if next_parts.first() == Some(&"transition") {
                    if !matches!(next_parts.len(), 9 | 10) {
                        return Err(format!("bad transition: {next}"));
                    }
                    let has_stable_id = next_parts.len() == 10;
                    let mut id = if has_stable_id {
                        TransitionId::parse_code(next_parts[1])
                            .ok_or_else(|| format!("invalid transition id {}", next_parts[1]))?
                    } else {
                        TransitionId::from_ordinal(transitions.len())
                    };
                    if !id.is_assigned() || transitions.iter().any(|transition| transition.id == id)
                    {
                        id = TransitionId::next_after(
                            transitions.iter().map(|transition| transition.id),
                        );
                    }
                    let x_index = if has_stable_id { 2 } else { 1 };
                    transitions.push(Transition {
                        id,
                        x: next_parts[x_index]
                            .parse::<i32>()
                            .map_err(|_| "bad transition x")?
                            + offset_x,
                        y: next_parts[x_index + 1]
                            .parse::<i32>()
                            .map_err(|_| "bad transition y")?
                            + offset_y,
                        w: next_parts[x_index + 2]
                            .parse::<i32>()
                            .map_err(|_| "bad transition w")?,
                        h: next_parts[x_index + 3]
                            .parse::<i32>()
                            .map_err(|_| "bad transition h")?,
                        target: SceneReference::from(next_parts[x_index + 4]),
                        spawn_x: next_parts[x_index + 5]
                            .parse::<i32>()
                            .map_err(|_| "bad transition spawn x")?
                            + offset_x,
                        spawn_y: next_parts[x_index + 6]
                            .parse::<i32>()
                            .map_err(|_| "bad transition spawn y")?
                            + offset_y,
                        label: next_parts[x_index + 7].replace('_', " "),
                    });
                } else {
                    object_lines.push(next.to_string());
                }
            }
            for object in object_lines {
                map_blob.push_str(&object);
                map_blob.push('\n');
            }
            let map = TavernMap::deserialize_lines_with_fill(&map_blob, scene_fill_for_kind(kind))?;
            scenes.push(SceneMap {
                id,
                kind,
                biome,
                name,
                dimensions,
                map,
                zones,
                transitions,
                autotile_overrides,
                spawn_x,
                spawn_y,
            });
        }

        let scenes = SceneRegistry::from_scenes(scenes)?;
        let world = Self {
            scenes,
            active_scene,
            tile_rules,
        };
        if world.scene_by_reference(&world.active_scene).is_none() {
            return Err(format!(
                "active scene reference '{}' does not resolve to a loaded scene",
                world.active_scene.code()
            ));
        }
        Ok(world)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_project_scene_round_trips_through_line_save() {
        let mut world = GameWorld::starter();
        let source = ProjectSceneId::from(SceneId::Farmstead);
        let custom = ProjectSceneId::new("harbor_annex_02");
        world
            .duplicate_scene(&source, custom.clone(), "Harbor Annex 02")
            .expect("duplicate custom scene");
        world
            .set_active_scene(custom.clone())
            .expect("activate custom scene");

        let decoded = GameWorld::deserialize_lines(&world.serialize_lines())
            .expect("round-trip arbitrary scene");
        assert_eq!(decoded.active_scene.project_id(), &custom);
        assert_eq!(
            decoded.scene_by_id(&custom).unwrap().name,
            "Harbor Annex 02"
        );
        assert!(decoded.scenes.validate().is_ok());
        for source_scene in &world.scenes {
            let decoded_scene = decoded
                .scene_by_id(&source_scene.id)
                .expect("decoded scene");
            let source_ids = source_scene
                .transitions
                .iter()
                .map(|transition| transition.id)
                .collect::<Vec<_>>();
            let decoded_ids = decoded_scene
                .transitions
                .iter()
                .map(|transition| transition.id)
                .collect::<Vec<_>>();
            assert_eq!(decoded_ids, source_ids);
        }
    }

    #[test]
    fn manual_autotile_override_round_trips_through_line_save() {
        let mut world = GameWorld::starter();
        let scene_id = ProjectSceneId::from(SceneId::Farmstead);
        let scene = world.scene_mut_by_id(&scene_id).expect("farmstead");
        scene.map.set(7, 7, TileKind::Road);
        scene.set_autotile_override(AutotileOverride::new(7, 7, TileAutoGroup::Road, 5));

        let decoded = GameWorld::deserialize_lines(&world.serialize_lines())
            .expect("round-trip autotile override");
        assert_eq!(
            decoded
                .scene_by_id(&scene_id)
                .expect("farmstead")
                .autotile_override_at(7, 7),
            Some(AutotileOverride::new(7, 7, TileAutoGroup::Road, 5))
        );
    }

    #[test]
    fn project_scene_can_be_removed_when_it_is_not_active_or_referenced() {
        let mut world = GameWorld::starter();
        let source = ProjectSceneId::from(SceneId::Farmstead);
        let custom = ProjectSceneId::new("temporary_harbor_test");
        world
            .duplicate_scene(&source, custom.clone(), "Temporary Harbor Test")
            .expect("duplicate project scene");

        let removed = world.remove_scene(&custom).expect("remove project scene");
        assert_eq!(removed.id, custom);
        assert!(world
            .scene_by_id(&ProjectSceneId::new("temporary_harbor_test"))
            .is_none());
        assert!(world.scenes.validate().is_ok());
    }

    #[test]
    fn scene_removal_rejects_inbound_transition_references() {
        let mut world = GameWorld::starter();
        let tavern = ProjectSceneId::from(SceneId::TavernInterior);
        let error = world
            .remove_scene(&tavern)
            .expect_err("starter tavern has inbound references");
        assert!(error.contains("referenced by"));
        assert!(world.scene_by_id(&tavern).is_some());
    }

    #[test]
    fn rename_scene_repairs_active_and_transition_references() {
        let mut world = GameWorld::starter();
        let current = ProjectSceneId::from(SceneId::Farmstead);
        let replacement = ProjectSceneId::new("starter_homestead");
        world
            .rename_scene(&current, replacement.clone())
            .expect("rename scene");
        assert_eq!(world.active_scene.project_id(), &replacement);
        assert!(world.scene_by_id(&replacement).is_some());
        assert!(world
            .scenes
            .iter()
            .flat_map(|scene| &scene.transitions)
            .all(|transition| { transition.target.project_id() != &current }));
    }
}
