use crate::PixelLicense;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const RECENT_DOCUMENTS_PATH: &str = "WORKSPACE/pixel_studio/recent_documents.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelLibrarySource {
    Intake,
    ProjectOriginal,
    UserCc0,
    Generated,
    External,
}

impl PixelLibrarySource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Intake => "Intake",
            Self::ProjectOriginal => "Project",
            Self::UserCc0 => "CC0 Upload",
            Self::Generated => "Generated",
            Self::External => "External",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PixelLibraryEntry {
    pub display_name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub source: PixelLibrarySource,
    pub license: PixelLicense,
    pub recent_rank: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentPixelDocument {
    relative_path: String,
    display_name: String,
    opened_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalPixelRoots {
    #[serde(default)]
    roots: Vec<ExternalPixelRoot>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalPixelRoot {
    id: String,
    #[serde(default)]
    display_name: String,
    path: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_external_license")]
    license_status: String,
    #[serde(default)]
    attribution: String,
    #[serde(default)]
    source_url: String,
    #[serde(default)]
    notes: String,
}

fn default_true() -> bool {
    true
}

fn default_external_license() -> String {
    "unknown_requires_review".to_string()
}

pub fn scan_pixel_library(repo_root: impl AsRef<Path>) -> Result<Vec<PixelLibraryEntry>, String> {
    let repo_root = repo_root.as_ref();
    let recent = load_recent_documents(repo_root)?;
    let rank_by_path: HashMap<&str, usize> = recent
        .iter()
        .enumerate()
        .map(|(index, item)| (item.relative_path.as_str(), index))
        .collect();
    let roots = [
        ("assets/source/intake", PixelLibrarySource::Intake),
        (
            "assets/source/original",
            PixelLibrarySource::ProjectOriginal,
        ),
        (
            "assets/source/cc0/user_uploads",
            PixelLibrarySource::UserCc0,
        ),
        ("assets/generated", PixelLibrarySource::Generated),
    ];
    let mut entries = Vec::new();
    for (relative, source) in roots {
        let root = repo_root.join(relative);
        if !root.is_dir() {
            continue;
        }
        scan_root(repo_root, &root, &root, source, None, None, &mut entries)?;
    }
    let scan_external = std::env::var("HAVENWILD_PIXEL_SCAN_EXTERNAL")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    if scan_external {
        for external in load_external_roots(repo_root)? {
            if !external.enabled {
                continue;
            }
            let root = PathBuf::from(&external.path);
            if !root.is_dir() {
                continue;
            }
            let license = PixelLicense {
                status: external.license_status.clone(),
                source_name: if external.display_name.is_empty() {
                    external.id.clone()
                } else {
                    external.display_name.clone()
                },
                source_url: external.source_url.clone(),
                notes: if external.notes.is_empty() {
                    format!(
                        "External source library. Attribution: {}. Create a project working copy before editing or publishing.",
                        external.attribution
                    )
                } else {
                    external.notes.clone()
                },
            };
            scan_root(
                repo_root,
                &root,
                &root,
                PixelLibrarySource::External,
                Some(&license),
                Some(&external.id),
                &mut entries,
            )?;
        }
    }
    for entry in &mut entries {
        entry.recent_rank = rank_by_path.get(entry.relative_path.as_str()).copied();
    }
    entries.sort_by(|left, right| match (left.recent_rank, right.recent_rank) {
        (Some(left_rank), Some(right_rank)) => left_rank.cmp(&right_rank),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left
            .source
            .label()
            .cmp(right.source.label())
            .then_with(|| left.display_name.cmp(&right.display_name)),
    });
    Ok(entries)
}

pub fn record_recent_pixel_document(
    repo_root: impl AsRef<Path>,
    relative_path: &str,
    display_name: &str,
) -> Result<(), String> {
    let repo_root = repo_root.as_ref();
    let mut recent = load_recent_documents(repo_root)?;
    recent.retain(|item| item.relative_path != relative_path);
    recent.insert(
        0,
        RecentPixelDocument {
            relative_path: relative_path.to_string(),
            display_name: display_name.to_string(),
            opened_unix_ms: unix_millis(),
        },
    );
    recent.truncate(16);
    let path = repo_root.join(RECENT_DOCUMENTS_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(&recent)
        .map_err(|error| format!("failed to serialize recent pixel documents: {error}"))?;
    fs::write(&path, format!("{raw}\n"))
        .map_err(|error| format!("failed to save {}: {error}", path.display()))
}

fn scan_root(
    repo_root: &Path,
    directory: &Path,
    source_root: &Path,
    source: PixelLibrarySource,
    license_override: Option<&PixelLicense>,
    external_root_id: Option<&str>,
    output: &mut Vec<PixelLibraryEntry>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to scan {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("failed to read pixel library entry: {error}"))?
            .path();
        if path.is_dir() {
            if path.extension().and_then(|value| value.to_str()) == Some("hhpixel") {
                continue;
            }
            scan_root(
                repo_root,
                &path,
                source_root,
                source,
                license_override,
                external_root_id,
                output,
            )?;
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(
            extension.as_str(),
            "png" | "gif" | "jpg" | "jpeg" | "bmp" | "webp"
        ) {
            continue;
        }
        let relative_path = if let Some(root_id) = external_root_id {
            let external_relative = path
                .strip_prefix(source_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            format!("external://{root_id}/{external_relative}")
        } else {
            path.strip_prefix(repo_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/")
        };
        let display_name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("image")
            .replace(['_', '-'], " ");
        let license = if let Some(license) = license_override {
            license.clone()
        } else {
            match source {
                PixelLibrarySource::UserCc0 => PixelLicense {
                    status: "cc0".to_string(),
                    source_name: "User-uploaded CC0 asset library".to_string(),
                    source_url: String::new(),
                    notes: "License declared CC0 by the project owner; retain source manifest."
                        .to_string(),
                },
                PixelLibrarySource::ProjectOriginal => PixelLicense {
                    status: "project_owned".to_string(),
                    source_name: "Havenwild project source".to_string(),
                    source_url: String::new(),
                    notes: String::new(),
                },
                PixelLibrarySource::Generated => PixelLicense {
                    status: "project_owned".to_string(),
                    source_name: "Havenwild generated output".to_string(),
                    source_url: String::new(),
                    notes: "Generated output; edit a working copy rather than the generated file."
                        .to_string(),
                },
                PixelLibrarySource::Intake | PixelLibrarySource::External => {
                    PixelLicense::default()
                }
            }
        };
        output.push(PixelLibraryEntry {
            display_name,
            path,
            relative_path,
            source,
            license,
            recent_rank: None,
        });
    }
    Ok(())
}

fn load_external_roots(repo_root: &Path) -> Result<Vec<ExternalPixelRoot>, String> {
    let paths = [
        repo_root.join("content/assets/intake/external_asset_roots_v0_1.json"),
        repo_root.join(".local/havenwild_external_asset_roots.json"),
    ];
    let mut roots_by_id = std::collections::BTreeMap::new();
    for path in paths {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let payload: ExternalPixelRoots = serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        for root in payload.roots {
            roots_by_id.insert(root.id.clone(), root);
        }
    }
    Ok(roots_by_id.into_values().collect())
}

fn load_recent_documents(repo_root: &Path) -> Result<Vec<RecentPixelDocument>, String> {
    let path = repo_root.join(RECENT_DOCUMENTS_PATH);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
