use super::ObjectKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectFootprint {
    pub visual_offset_x: i32,
    pub visual_offset_y: i32,
    pub visual_w: i32,
    pub visual_h: i32,
    pub collision_offset_x: i32,
    pub collision_offset_y: i32,
    pub collision_w: i32,
    pub collision_h: i32,
    pub interaction_offset_x: i32,
    pub interaction_offset_y: i32,
    pub interaction_w: i32,
    pub interaction_h: i32,
    pub blocks_movement: bool,
    pub occludes_player: bool,
    pub fade_when_player_behind: bool,
}

impl ObjectFootprint {
    pub const fn single_tile() -> Self {
        Self {
            visual_offset_x: 0,
            visual_offset_y: 0,
            visual_w: 1,
            visual_h: 1,
            collision_offset_x: 0,
            collision_offset_y: 0,
            collision_w: 1,
            collision_h: 1,
            interaction_offset_x: 0,
            interaction_offset_y: 0,
            interaction_w: 1,
            interaction_h: 1,
            blocks_movement: true,
            occludes_player: false,
            fade_when_player_behind: false,
        }
    }
}

impl ObjectKind {
    /// Resolves the authored footprint for the same deterministic visual variant
    /// selected by `haven_assets::object_asset_entry_for_cell`.
    pub fn footprint_for_cell(self, x: i32, y: i32) -> ObjectFootprint {
        if self != ObjectKind::Boulder {
            return self.default_footprint();
        }

        match deterministic_object_variant("boulder", x, y, 4) {
            0 => ObjectFootprint::single_tile(),
            1 => ObjectFootprint {
                visual_w: 2,
                visual_h: 1,
                collision_w: 2,
                collision_h: 1,
                interaction_w: 2,
                interaction_h: 1,
                ..ObjectFootprint::single_tile()
            },
            2 => ObjectFootprint {
                visual_offset_y: -1,
                visual_w: 2,
                visual_h: 2,
                collision_w: 2,
                collision_h: 1,
                interaction_w: 2,
                interaction_h: 1,
                ..ObjectFootprint::single_tile()
            },
            _ => ObjectFootprint {
                visual_offset_y: -1,
                visual_w: 1,
                visual_h: 2,
                collision_w: 1,
                collision_h: 1,
                interaction_w: 1,
                interaction_h: 1,
                ..ObjectFootprint::single_tile()
            },
        }
    }

    pub fn default_footprint(self) -> ObjectFootprint {
        match self {
            ObjectKind::Table => ObjectFootprint {
                visual_w: 2,
                visual_h: 2,
                collision_w: 2,
                collision_h: 2,
                interaction_w: 2,
                interaction_h: 2,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Chair => ObjectFootprint {
                blocks_movement: true,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Bar => ObjectFootprint {
                visual_w: 4,
                visual_h: 2,
                collision_w: 4,
                collision_h: 1,
                collision_offset_y: 1,
                interaction_w: 4,
                interaction_h: 1,
                interaction_offset_y: 2,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Keg => ObjectFootprint {
                visual_w: 1,
                visual_h: 2,
                collision_w: 1,
                collision_h: 1,
                collision_offset_y: 1,
                interaction_offset_y: 1,
                occludes_player: true,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Bed => ObjectFootprint {
                visual_w: 3,
                visual_h: 2,
                collision_w: 3,
                collision_h: 1,
                collision_offset_y: 1,
                interaction_w: 3,
                interaction_h: 1,
                interaction_offset_y: 1,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Fireplace => ObjectFootprint {
                visual_w: 2,
                visual_h: 2,
                collision_w: 2,
                collision_h: 1,
                collision_offset_y: 1,
                interaction_w: 2,
                interaction_h: 1,
                interaction_offset_y: 1,
                occludes_player: true,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::GreenhouseMarker => ObjectFootprint {
                visual_w: 5,
                visual_h: 5,
                collision_w: 5,
                collision_h: 1,
                collision_offset_y: 4,
                interaction_w: 3,
                interaction_h: 1,
                interaction_offset_x: 1,
                interaction_offset_y: 4,
                blocks_movement: false,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Tree => ObjectFootprint {
                visual_offset_x: -1,
                visual_offset_y: -3,
                visual_w: 3,
                visual_h: 4,
                collision_w: 1,
                collision_h: 1,
                interaction_w: 1,
                interaction_h: 1,
                blocks_movement: true,
                occludes_player: true,
                fade_when_player_behind: true,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Bush => ObjectFootprint {
                // Bushes are full one-tile obstacles. Their prior nonblocking
                // footprint let the player walk beneath dense bush artwork.
                blocks_movement: true,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Mushroom | ObjectKind::Herb | ObjectKind::Lamp | ObjectKind::Sign => {
                ObjectFootprint {
                    blocks_movement: false,
                    ..ObjectFootprint::single_tile()
                }
            }
            ObjectKind::Boulder
            | ObjectKind::OreNode
            | ObjectKind::Crate
            | ObjectKind::Barrel
            | ObjectKind::Well
            | ObjectKind::Scarecrow
            | ObjectKind::Fence
            | ObjectKind::Bench
            | ObjectKind::Stump
            | ObjectKind::Log => ObjectFootprint {
                visual_w: 1,
                visual_h: 1,
                collision_w: 1,
                collision_h: 1,
                interaction_w: 1,
                interaction_h: 1,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::Door | ObjectKind::Stairs => ObjectFootprint {
                blocks_movement: false,
                ..ObjectFootprint::single_tile()
            },
            ObjectKind::CaveEntrance => ObjectFootprint {
                visual_w: 4,
                visual_h: 4,
                collision_w: 4,
                collision_h: 1,
                collision_offset_y: 3,
                interaction_w: 2,
                interaction_h: 1,
                interaction_offset_x: 1,
                interaction_offset_y: 2,
                blocks_movement: true,
                occludes_player: true,
                fade_when_player_behind: true,
                ..ObjectFootprint::single_tile()
            },
        }
    }
}

fn deterministic_object_variant(stable_id: &str, x: i32, y: i32, count: usize) -> usize {
    let mut hash = 2_166_136_261u32;
    for byte in stable_id.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash ^= (x as u32).wrapping_mul(0x9E37_79B9);
    hash = hash.rotate_left(13);
    hash ^= (y as u32).wrapping_mul(0x85EB_CA6B);
    (hash as usize) % count.max(1)
}

#[cfg(test)]
mod variant_footprint_tests {
    use super::*;

    #[test]
    fn boulder_variants_include_single_and_multi_cell_footprints() {
        let mut saw_single = false;
        let mut saw_multi = false;
        for y in 0..32 {
            for x in 0..32 {
                let footprint = ObjectKind::Boulder.footprint_for_cell(x, y);
                saw_single |= footprint.visual_w == 1 && footprint.visual_h == 1;
                saw_multi |= footprint.visual_w > 1 || footprint.visual_h > 1;
            }
        }
        assert!(saw_single);
        assert!(saw_multi);
    }
}
