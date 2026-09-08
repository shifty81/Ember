use haven_core::TileKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainSetV2 {
    NaturalGround,
    Path,
    Water,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainIdV2 {
    Grass,
    Sand,
    Road,
    StonePath,
    ShallowWater,
    DeepWater,
}

impl TerrainIdV2 {
    pub fn from_tile(tile: TileKind) -> Option<Self> {
        match tile {
            TileKind::Grass | TileKind::TallGrass => Some(Self::Grass),
            TileKind::Sand | TileKind::WetSand => Some(Self::Sand),
            TileKind::Road => Some(Self::Road),
            TileKind::StonePath | TileKind::PebbleShore => Some(Self::StonePath),
            TileKind::ShallowWater | TileKind::OceanShallow | TileKind::ShoreFoam => {
                Some(Self::ShallowWater)
            }
            TileKind::DeepWater | TileKind::OceanDeep => Some(Self::DeepWater),
            _ => None,
        }
    }

    pub fn terrain_set(self) -> TerrainSetV2 {
        match self {
            Self::Grass | Self::Sand => TerrainSetV2::NaturalGround,
            Self::Road | Self::StonePath => TerrainSetV2::Path,
            Self::ShallowWater | Self::DeepWater => TerrainSetV2::Water,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainPeerV2 {
    Empty,
    Terrain(TerrainIdV2),
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainPatternV2 {
    pub center: TerrainIdV2,
    pub north: TerrainPeerV2,
    pub north_east: TerrainPeerV2,
    pub east: TerrainPeerV2,
    pub south_east: TerrainPeerV2,
    pub south: TerrainPeerV2,
    pub south_west: TerrainPeerV2,
    pub west: TerrainPeerV2,
    pub north_west: TerrainPeerV2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainPatternCandidateV2 {
    pub id: String,
    pub pattern: TerrainPatternV2,
    pub verified: bool,
    pub weight: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainPatternRequestV2 {
    pub pattern: TerrainPatternV2,
    pub world_seed: u64,
    pub x: i32,
    pub y: i32,
}

pub fn resolve_pattern_v2<'a>(
    request: &TerrainPatternRequestV2,
    candidates: &'a [TerrainPatternCandidateV2],
) -> Option<&'a TerrainPatternCandidateV2> {
    let mut best: Option<(&TerrainPatternCandidateV2, u16, u64)> = None;
    for candidate in candidates {
        if candidate.pattern.center != request.pattern.center
            || candidate.pattern.center.terrain_set() != request.pattern.center.terrain_set()
        {
            continue;
        }
        let Some(score) = pattern_score(&request.pattern, &candidate.pattern) else {
            continue;
        };
        let verified_bonus = u16::from(candidate.verified) * 100;
        let total = score + verified_bonus;
        let tie = stable_tie_break(request, candidate);
        if best.as_ref().is_none_or(|(_, best_score, best_tie)| {
            total > *best_score || (total == *best_score && tie < *best_tie)
        }) {
            best = Some((candidate, total, tie));
        }
    }
    best.map(|(candidate, _, _)| candidate)
}

fn pattern_score(request: &TerrainPatternV2, candidate: &TerrainPatternV2) -> Option<u16> {
    let requested = [
        request.north,
        request.north_east,
        request.east,
        request.south_east,
        request.south,
        request.south_west,
        request.west,
        request.north_west,
    ];
    let authored = [
        candidate.north,
        candidate.north_east,
        candidate.east,
        candidate.south_east,
        candidate.south,
        candidate.south_west,
        candidate.west,
        candidate.north_west,
    ];
    let mut score = 0u16;
    for (actual, expected) in requested.into_iter().zip(authored) {
        match expected {
            TerrainPeerV2::Any => score += 1,
            _ if expected == actual => score += 10,
            _ => return None,
        }
    }
    Some(score)
}

fn stable_tie_break(
    request: &TerrainPatternRequestV2,
    candidate: &TerrainPatternCandidateV2,
) -> u64 {
    let mut value = request.world_seed
        ^ (request.x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (request.y as u64).rotate_left(31);
    for byte in candidate.id.as_bytes() {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x100_0000_01b3);
    }
    value ^ u64::from(candidate.weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(center: TerrainIdV2, peer: TerrainPeerV2) -> TerrainPatternV2 {
        TerrainPatternV2 {
            center,
            north: peer,
            north_east: peer,
            east: peer,
            south_east: peer,
            south: peer,
            south_west: peer,
            west: peer,
            north_west: peer,
        }
    }

    #[test]
    fn road_and_stone_path_keep_distinct_identity() {
        assert_ne!(TerrainIdV2::Road, TerrainIdV2::StonePath);
        assert_eq!(TerrainIdV2::Road.terrain_set(), TerrainSetV2::Path);
        assert_eq!(TerrainIdV2::StonePath.terrain_set(), TerrainSetV2::Path);
    }

    #[test]
    fn exact_pattern_beats_wildcard_pattern() {
        let request = TerrainPatternRequestV2 {
            pattern: all(TerrainIdV2::Road, TerrainPeerV2::Terrain(TerrainIdV2::Road)),
            world_seed: 7,
            x: 3,
            y: 9,
        };
        let candidates = vec![
            TerrainPatternCandidateV2 {
                id: "wildcard".into(),
                pattern: all(TerrainIdV2::Road, TerrainPeerV2::Any),
                verified: true,
                weight: 1,
            },
            TerrainPatternCandidateV2 {
                id: "exact".into(),
                pattern: request.pattern,
                verified: true,
                weight: 1,
            },
        ];
        assert_eq!(
            resolve_pattern_v2(&request, &candidates).unwrap().id,
            "exact"
        );
    }

    #[test]
    fn road_candidate_cannot_satisfy_stone_path_request() {
        let request = TerrainPatternRequestV2 {
            pattern: all(TerrainIdV2::StonePath, TerrainPeerV2::Any),
            world_seed: 1,
            x: 0,
            y: 0,
        };
        let candidates = vec![TerrainPatternCandidateV2 {
            id: "road".into(),
            pattern: all(TerrainIdV2::Road, TerrainPeerV2::Any),
            verified: true,
            weight: 1,
        }];
        assert!(resolve_pattern_v2(&request, &candidates).is_none());
    }
}
