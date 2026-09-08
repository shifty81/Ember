use ember_core::{ProjectPath, StableId};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

pub const PROJECT_FORMAT_VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub project_id: StableId,
    pub name: String,
    pub startup_scene: Option<StableId>,
    pub content_roots: Vec<ProjectPath>,
    pub enabled_packages: Vec<StableId>,
    pub build_profiles: Vec<BuildProfile>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildProfile {
    pub id: StableId,
    pub target: BuildTarget,
    pub development: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BuildTarget {
    WindowsPortable,
    WindowsInstaller,
    LinuxPortable,
    DedicatedServer,
}
impl ProjectManifest {
    pub fn new(project_id: StableId, name: impl Into<String>) -> Self {
        Self {
            format_version: PROJECT_FORMAT_VERSION,
            project_id,
            name: name.into(),
            startup_scene: None,
            content_roots: vec![ProjectPath::parse("content").expect("literal")],
            enabled_packages: vec![],
            build_profiles: vec![],
        }
    }
    pub fn load(path: &Path) -> Result<Self, ProjectError> {
        let bytes = fs::read(path)?;
        let value: Self = serde_json::from_slice(&bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        Ok(())
    }
    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.format_version != PROJECT_FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion(self.format_version));
        }
        if self.name.trim().is_empty() {
            return Err(ProjectError::EmptyName);
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum ProjectError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    EmptyName,
}
impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProjectError {}
impl From<io::Error> for ProjectError {
    fn from(v: io::Error) -> Self {
        Self::Io(v)
    }
}
impl From<serde_json::Error> for ProjectError {
    fn from(v: serde_json::Error) -> Self {
        Self::Json(v)
    }
}
