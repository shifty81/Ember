use serde::{Deserialize, Serialize};

macro_rules! numeric_id {
    ($name:ident, $prefix:literal) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const fn from_raw(value: u64) -> Self {
                Self(value)
            }

            pub const fn raw(self) -> u64 {
                self.0
            }

            pub fn from_ordinal(index: usize) -> Self {
                Self(index as u64 + 1)
            }

            pub fn from_stable_key(value: &str) -> Self {
                let mut hash = 0xcbf29ce484222325u64;
                for byte in value.as_bytes() {
                    hash ^= *byte as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                Self(hash.max(1))
            }

            pub fn is_assigned(self) -> bool {
                self.0 != 0
            }

            pub fn next_after<I>(ids: I) -> Self
            where
                I: IntoIterator<Item = Self>,
            {
                let occupied = ids
                    .into_iter()
                    .map(Self::raw)
                    .collect::<std::collections::BTreeSet<_>>();
                let mut candidate = 1u64;
                while occupied.contains(&candidate) {
                    candidate = candidate.saturating_add(1);
                    if candidate == u64::MAX && occupied.contains(&candidate) {
                        return Self::from_stable_key(concat!($prefix, "_overflow"));
                    }
                }
                Self(candidate)
            }

            pub fn code(self) -> String {
                format!(concat!($prefix, "_{:016x}"), self.0)
            }

            pub fn parse_code(value: &str) -> Option<Self> {
                let encoded = value.strip_prefix(concat!($prefix, "_"))?;
                u64::from_str_radix(encoded, 16).ok().map(Self)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.code())
            }
        }
    };
}

numeric_id!(ObjectId, "obj");
numeric_id!(StampInstanceId, "stamp");
numeric_id!(TransitionId, "transition");
numeric_id!(ZoneId, "zone");

impl ZoneId {
    /// Stable identity for the current cell-backed zone model. When zones become
    /// first-class regions, this same type remains the selection/persistence key.
    pub fn for_cell(x: i32, y: i32, map_width: usize) -> Self {
        let ordinal = y.max(0) as usize * map_width + x.max(0) as usize;
        Self::from_ordinal(ordinal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegionNodeId(String);

impl RegionNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let mut normalized = String::with_capacity(value.len());
        let mut previous_separator = false;
        for character in value.trim().chars() {
            let mapped = if character.is_ascii_alphanumeric() {
                previous_separator = false;
                character.to_ascii_lowercase()
            } else if !previous_separator {
                previous_separator = true;
                '_'
            } else {
                continue;
            };
            normalized.push(mapped);
        }
        let normalized = normalized.trim_matches('_').to_string();
        Self(if normalized.is_empty() {
            "region_node".to_string()
        } else {
            normalized
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RegionNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RegionNodeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RegionNodeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_ids_round_trip_through_text_codes() {
        let id = ObjectId::from_raw(42);
        assert_eq!(ObjectId::parse_code(&id.code()), Some(id));
        let stamp = StampInstanceId::from_raw(64);
        assert_eq!(StampInstanceId::parse_code(&stamp.code()), Some(stamp));
        let transition = TransitionId::from_raw(77);
        assert_eq!(
            TransitionId::parse_code(&transition.code()),
            Some(transition)
        );
    }

    #[test]
    fn region_node_ids_are_normalized() {
        assert_eq!(RegionNodeId::new(" North Harbor ").as_str(), "north_harbor");
    }
}
