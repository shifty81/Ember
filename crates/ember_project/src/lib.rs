use ember_core::{ProjectPath, StableId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const PROJECT_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub project_id: StableId,
    pub name: String,
    pub startup_scene: Option<StableId>,
    pub content_roots: Vec<ProjectPath>,
    #[serde(default)]
    pub enabled_packages: Vec<StableId>,
    #[serde(default)]
    pub package_roots: Vec<ProjectPath>,
    #[serde(default)]
    pub asset_catalog: Option<ProjectPath>,
    #[serde(default)]
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
            package_roots: vec![ProjectPath::parse("packages").expect("literal")],
            asset_catalog: Some(ProjectPath::parse("content/assets.json").expect("literal")),
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
        atomic_write(path, &bytes)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.format_version != PROJECT_FORMAT_VERSION {
            return Err(ProjectError::UnsupportedVersion(self.format_version));
        }
        if self.name.trim().is_empty() {
            return Err(ProjectError::EmptyName);
        }
        if self.content_roots.is_empty() {
            return Err(ProjectError::Invalid("project has no content roots".into()));
        }
        let mut profiles = BTreeSet::new();
        for profile in &self.build_profiles {
            if !profiles.insert(profile.id.clone()) {
                return Err(ProjectError::Invalid(format!(
                    "duplicate build profile {}",
                    profile.id
                )));
            }
        }
        let mut packages = BTreeSet::new();
        for package in &self.enabled_packages {
            if !packages.insert(package.clone()) {
                return Err(ProjectError::Invalid(format!(
                    "duplicate enabled package {package}"
                )));
            }
        }
        Ok(())
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");
    let temp: PathBuf = path.with_file_name(format!(".{file_name}.ember-project.tmp"));
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.flush()?;
        let _ = file.sync_all();
    }
    if path.exists() {
        #[cfg(windows)]
        {
            let backup = path.with_extension("ember-project.bak");
            let _ = fs::remove_file(&backup);
            fs::rename(path, &backup)?;
            match fs::rename(&temp, path) {
                Ok(()) => {
                    let _ = fs::remove_file(backup);
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(backup, path);
                    Err(error)
                }
            }
        }
        #[cfg(not(windows))]
        {
            fs::rename(&temp, path)
        }
    } else {
        fs::rename(&temp, path)
    }
}

#[derive(Debug)]
pub enum ProjectError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    EmptyName,
    Invalid(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "project I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "project JSON failed: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported project version {version}")
            }
            Self::EmptyName => formatter.write_str("project name is empty"),
            Self::Invalid(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for ProjectError {}
impl From<io::Error> for ProjectError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for ProjectError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}
