use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct StableId(String);

impl StableId {
    pub fn new(namespace: &str, value: &str) -> Result<Self, CoreError> {
        let namespace = normalize_token(namespace)?;
        let value = normalize_token(value)?;
        Ok(Self(format!("{namespace}:{value}")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for StableId {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (namespace, value) = s.split_once(':').ok_or(CoreError::InvalidStableId)?;
        Self::new(namespace, value)
    }
}

fn normalize_token(value: &str) -> Result<String, CoreError> {
    let value = value.trim().to_ascii_lowercase().replace([' ', '\\'], "_");

    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
    {
        return Err(CoreError::InvalidStableId);
    }

    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectPath(PathBuf);

impl ProjectPath {
    pub fn parse(value: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = value.as_ref();

        if path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(CoreError::UnsafeProjectPath);
        }

        Ok(Self(path.to_path_buf()))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub subject: Option<StableId>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            subject: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidStableId,
    UnsafeProjectPath,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId => f.write_str("invalid stable id"),
            Self::UnsafeProjectPath => f.write_str("unsafe project-relative path"),
        }
    }
}

impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_normalize() {
        assert_eq!(
            StableId::new("Entity", "Player One").unwrap().as_str(),
            "entity:player_one"
        );
    }

    #[test]
    fn paths_reject_parent() {
        assert!(ProjectPath::parse("../x").is_err());
    }
}
