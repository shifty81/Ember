use super::*;

impl TavernMap {
    pub fn serialize_lines(&self) -> String {
        let mut out = format!("tavern_map 1 {} {}\n", MAP_W, MAP_H);
        for y in 0..MAP_H as i32 {
            out.push_str("row");
            for x in 0..MAP_W as i32 {
                out.push(' ');
                out.push_str(self.get(x, y).code());
            }
            out.push('\n');
        }
        out.push_str("heights\n");
        for y in 0..MAP_H as i32 {
            out.push_str("height_row");
            for x in 0..MAP_W as i32 {
                out.push(' ');
                out.push_str(&self.get_height(x, y).to_string());
            }
            out.push('\n');
        }
        out.push_str("structural_levels\n");
        for y in 0..MAP_H as i32 {
            out.push_str("structural_level_row");
            for x in 0..MAP_W as i32 {
                out.push(' ');
                match self.get_structural_level(x, y) {
                    Some(level) => out.push_str(&level.to_string()),
                    None => out.push_str("auto"),
                }
            }
            out.push('\n');
        }
        for object in &self.objects {
            let fp = object.footprint;
            out.push_str(&format!(
                "object {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
                object.id.code(),
                object.kind.code(),
                object.x,
                object.y,
                fp.visual_offset_x,
                fp.visual_offset_y,
                fp.visual_w,
                fp.visual_h,
                fp.collision_offset_x,
                fp.collision_offset_y,
                fp.collision_w,
                fp.collision_h,
                fp.interaction_offset_x,
                fp.interaction_offset_y,
                fp.interaction_w,
                fp.interaction_h,
                fp.blocks_movement as u8,
                fp.occludes_player as u8,
                fp.fade_when_player_behind as u8
            ));
        }
        for (object_id, asset_ref) in &self.object_asset_refs {
            out.push_str(&format!(
                "object_asset {} {} {} {} {} {}\n",
                object_id.code(),
                asset_ref.pack_id,
                asset_ref.category,
                asset_ref.asset_id,
                asset_ref.source_id,
                asset_ref.variant_id.as_deref().unwrap_or("-")
            ));
        }
        for (object_id, state) in &self.object_states {
            out.push_str(&format!("object_state {} {}\n", object_id.code(), state));
        }
        for stamp in &self.stamps {
            let fp = stamp.footprint;
            out.push_str(&format!(
                "stamp {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
                stamp.id.code(),
                stamp.stamp_key,
                stamp.x,
                stamp.y,
                fp.visual_offset_x,
                fp.visual_offset_y,
                fp.visual_w,
                fp.visual_h,
                fp.collision_offset_x,
                fp.collision_offset_y,
                fp.collision_w,
                fp.collision_h,
                fp.interaction_offset_x,
                fp.interaction_offset_y,
                fp.interaction_w,
                fp.interaction_h,
                fp.blocks_movement as u8,
                fp.occludes_player as u8,
                fp.fade_when_player_behind as u8
            ));
        }
        out
    }

    pub fn deserialize_lines(input: &str) -> Result<Self, String> {
        Self::deserialize_lines_with_fill(input, TileKind::Grass)
    }

    pub fn deserialize_lines_with_fill(input: &str, fill: TileKind) -> Result<Self, String> {
        let mut lines = input.lines();
        let header = lines.next().ok_or("missing header")?;
        let (source_w, source_h) = parse_tavern_map_dimensions(header)?;
        let (offset_x, offset_y) = scene_dimension_offset(source_w, source_h);

        let mut map = Self {
            tiles: vec![fill; MAP_W * MAP_H],
            heights: vec![48; MAP_W * MAP_H],
            structural_levels: vec![STRUCTURAL_LEVEL_AUTO; MAP_W * MAP_H],
            objects: Vec::new(),
            stamps: Vec::new(),
            object_asset_refs: BTreeMap::new(),
            object_states: BTreeMap::new(),
        };

        for y in 0..source_h {
            let line = lines.next().ok_or("missing tile row")?;
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() != source_w + 1 || parts[0] != "row" {
                return Err(format!("invalid row {}", y + 1));
            }
            for x in 0..source_w {
                let tile = TileKind::from_code(parts[x + 1])
                    .ok_or_else(|| format!("unknown tile code {}", parts[x + 1]))?;
                map.set(x as i32 + offset_x, y as i32 + offset_y, tile);
            }
        }

        if lines.clone().next() == Some("heights") {
            lines.next();
            for y in 0..source_h {
                let row = lines.next().ok_or("missing height row")?;
                let parts: Vec<_> = row.split_whitespace().collect();
                if parts.len() != source_w + 1 || parts[0] != "height_row" {
                    return Err(format!("invalid height row {}", y + 1));
                }
                for x in 0..source_w {
                    let height = parts[x + 1]
                        .parse::<u8>()
                        .map_err(|_| format!("bad height {}", parts[x + 1]))?;
                    map.set_height(x as i32 + offset_x, y as i32 + offset_y, height);
                }
            }
        }

        if lines.clone().next() == Some("structural_levels") {
            lines.next();
            for y in 0..source_h {
                let row = lines.next().ok_or("missing structural level row")?;
                let parts: Vec<_> = row.split_whitespace().collect();
                if parts.len() != source_w + 1 || parts[0] != "structural_level_row" {
                    return Err(format!("invalid structural level row {}", y + 1));
                }
                for x in 0..source_w {
                    let level = match parts[x + 1] {
                        "auto" => None,
                        value => {
                            let parsed = value
                                .parse::<u8>()
                                .map_err(|_| format!("bad structural level {value}"))?;
                            if parsed > MAX_STRUCTURAL_LEVEL {
                                return Err(format!(
                                    "structural level {parsed} exceeds maximum {MAX_STRUCTURAL_LEVEL}"
                                ));
                            }
                            Some(parsed)
                        }
                    };
                    map.set_structural_level(x as i32 + offset_x, y as i32 + offset_y, level);
                }
            }
        }

        for line in lines {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            if parts[0] == "object_asset" {
                if parts.len() != 7 {
                    return Err(format!("bad object asset record: {line}"));
                }
                let id = ObjectId::parse_code(parts[1])
                    .ok_or_else(|| format!("invalid object asset id {}", parts[1]))?;
                let variant_id = (parts[6] != "-").then(|| parts[6].to_string());
                map.object_asset_refs.insert(
                    id,
                    StablePlaceableAssetRef::new(
                        parts[2], parts[3], parts[4], parts[5], variant_id,
                    ),
                );
                continue;
            }
            if parts[0] == "object_state" {
                if parts.len() != 3 {
                    return Err(format!("bad object state record: {line}"));
                }
                let id = ObjectId::parse_code(parts[1])
                    .ok_or_else(|| format!("invalid object state id {}", parts[1]))?;
                map.object_states.insert(id, parts[2].to_string());
                continue;
            }
            if parts[0] == "stamp" {
                if parts.len() != 20 {
                    return Err(format!("bad stamp record: {line}"));
                }
                let mut id = StampInstanceId::parse_code(parts[1])
                    .ok_or_else(|| format!("invalid stamp id {}", parts[1]))?;
                if !id.is_assigned() || map.stamp(id).is_some() {
                    id = map.next_stamp_id();
                }
                let stamp_key = parts[2].to_string();
                let x = parts[3].parse::<i32>().map_err(|_| "bad stamp x")? + offset_x;
                let y = parts[4].parse::<i32>().map_err(|_| "bad stamp y")? + offset_y;
                let mut fp = ObjectFootprint::single_tile();
                fp.visual_offset_x = parts[5]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp visual offset x")?;
                fp.visual_offset_y = parts[6]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp visual offset y")?;
                fp.visual_w = parts[7].parse::<i32>().map_err(|_| "bad stamp visual w")?;
                fp.visual_h = parts[8].parse::<i32>().map_err(|_| "bad stamp visual h")?;
                fp.collision_offset_x = parts[9]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp collision offset x")?;
                fp.collision_offset_y = parts[10]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp collision offset y")?;
                fp.collision_w = parts[11]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp collision w")?;
                fp.collision_h = parts[12]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp collision h")?;
                fp.interaction_offset_x = parts[13]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp interaction offset x")?;
                fp.interaction_offset_y = parts[14]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp interaction offset y")?;
                fp.interaction_w = parts[15]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp interaction w")?;
                fp.interaction_h = parts[16]
                    .parse::<i32>()
                    .map_err(|_| "bad stamp interaction h")?;
                fp.blocks_movement = parts[17] != "0";
                fp.occludes_player = parts[18] != "0";
                fp.fade_when_player_behind = parts[19] != "0";
                map.stamps
                    .push(PlacedStamp::with_id(id, stamp_key, x, y, fp));
                continue;
            }
            if !matches!(parts.len(), 4 | 19 | 20) || parts[0] != "object" {
                continue;
            }

            let has_stable_id = parts.len() == 20;
            let id = if has_stable_id {
                ObjectId::parse_code(parts[1])
                    .ok_or_else(|| format!("invalid object id {}", parts[1]))?
            } else {
                map.next_object_id()
            };
            let kind_index = if has_stable_id { 2 } else { 1 };
            let x_index = kind_index + 1;
            let y_index = kind_index + 2;
            let footprint_index = kind_index + 3;
            let kind = ObjectKind::from_code(parts[kind_index])
                .ok_or_else(|| format!("unknown object code {}", parts[kind_index]))?;
            let x = parts[x_index].parse::<i32>().map_err(|_| "bad object x")? + offset_x;
            let y = parts[y_index].parse::<i32>().map_err(|_| "bad object y")? + offset_y;

            if parts.len() == 19 || parts.len() == 20 {
                let mut fp = kind.default_footprint();
                fp.visual_offset_x = parts[footprint_index]
                    .parse::<i32>()
                    .map_err(|_| "bad object visual offset x")?;
                fp.visual_offset_y = parts[footprint_index + 1]
                    .parse::<i32>()
                    .map_err(|_| "bad object visual offset y")?;
                fp.visual_w = parts[footprint_index + 2]
                    .parse::<i32>()
                    .map_err(|_| "bad object visual w")?;
                fp.visual_h = parts[footprint_index + 3]
                    .parse::<i32>()
                    .map_err(|_| "bad object visual h")?;
                fp.collision_offset_x = parts[footprint_index + 4]
                    .parse::<i32>()
                    .map_err(|_| "bad object collision offset x")?;
                fp.collision_offset_y = parts[footprint_index + 5]
                    .parse::<i32>()
                    .map_err(|_| "bad object collision offset y")?;
                fp.collision_w = parts[footprint_index + 6]
                    .parse::<i32>()
                    .map_err(|_| "bad object collision w")?;
                fp.collision_h = parts[footprint_index + 7]
                    .parse::<i32>()
                    .map_err(|_| "bad object collision h")?;
                fp.interaction_offset_x = parts[footprint_index + 8]
                    .parse::<i32>()
                    .map_err(|_| "bad object interaction offset x")?;
                fp.interaction_offset_y = parts[footprint_index + 9]
                    .parse::<i32>()
                    .map_err(|_| "bad object interaction offset y")?;
                fp.interaction_w = parts[footprint_index + 10]
                    .parse::<i32>()
                    .map_err(|_| "bad object interaction w")?;
                fp.interaction_h = parts[footprint_index + 11]
                    .parse::<i32>()
                    .map_err(|_| "bad object interaction h")?;
                fp.blocks_movement = parts[footprint_index + 12] != "0";
                fp.occludes_player = parts[footprint_index + 13] != "0";
                fp.fade_when_player_behind = parts[footprint_index + 14] != "0";
                let _ = map
                    .place_custom_object(PlacedObject::with_id_and_footprint(id, kind, x, y, fp));
            } else {
                let _ = map.place_custom_object(PlacedObject::with_id(id, kind, x, y));
            }
        }

        Ok(map)
    }
}
