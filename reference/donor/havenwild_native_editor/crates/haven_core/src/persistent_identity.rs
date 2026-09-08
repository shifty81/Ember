use serde::{Deserialize, Serialize};

/// Stable gameplay identity that may cross save/load or replication boundaries.
///
/// This is intentionally separate from `haven_ecs::EntityId`, which is a
/// runtime-local handle and must never be treated as durable identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersistentEntityId(pub String);

impl PersistentEntityId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        Self::validate_value(&value)?;
        Ok(Self(value))
    }

    /// Wrap a stable ID that was already validated by the owning domain.
    /// Save/profile adapters use this when preserving an existing on-disk ID.
    pub fn from_domain_id(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_value(&self.0)
    }

    fn validate_value(value: &str) -> Result<(), String> {
        if value.is_empty()
            || value.len() > 96
            || !value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        {
            return Err(
                "persistent entity id must use 1-96 ASCII letters, digits, '_' or '-'"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_identity_rejects_path_or_schema_unsafe_text() {
        assert!(PersistentEntityId::new("character_123").is_ok());
        assert!(PersistentEntityId::new("../character").is_err());
        assert!(PersistentEntityId::new("").is_err());
    }
}
