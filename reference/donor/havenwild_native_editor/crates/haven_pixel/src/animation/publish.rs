use super::AnimationDocument;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const ANIMATION_STUDIO_OUTPUT_ROOT: &str = "assets/source/original/animation_studio";
pub const RUNTIME_ANIMATION_ROOT: &str = "content/animations";
pub const GENERATED_ANIMATION_IMAGE_ROOT: &str = "assets/generated/animations";
pub const RUNTIME_ANIMATION_CATALOG_PATH: &str = "content/animations/animation_catalog_v0_1.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationPublishResult {
    pub runtime_metadata_path: String,
    pub generated_image_path: String,
    pub clip_count: usize,
}

pub(super) fn publish_runtime(
    document: &mut AnimationDocument,
    root: &Path,
) -> Result<AnimationPublishResult, String> {
    if let Err(issues) = document.validate() {
        return Err(format!(
            "animation validation failed: {}",
            issues.join("; ")
        ));
    }
    document.save(root)?;
    let stem = slugify(&document.metadata.display_name);
    let source = resolve_path(root, &document.metadata.source_path);
    if !source.is_file() {
        return Err(format!("animation source is missing: {}", source.display()));
    }
    let generated_relative = format!("{GENERATED_ANIMATION_IMAGE_ROOT}/{stem}.png");
    let generated_path = resolve_path(root, &generated_relative);
    if let Some(parent) = generated_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::copy(&source, &generated_path).map_err(|error| {
        format!(
            "failed to copy {} to {}: {error}",
            source.display(),
            generated_path.display()
        )
    })?;

    let runtime_relative = format!("{RUNTIME_ANIMATION_ROOT}/{stem}.animation.json");
    let runtime_path = resolve_path(root, &runtime_relative);
    if let Some(parent) = runtime_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let mut runtime_metadata = document.metadata.clone();
    runtime_metadata.schema = "havenwild.runtime_animation.v0_1".to_string();
    runtime_metadata.source_path = generated_relative.clone();
    runtime_metadata.output_path = runtime_relative.clone();
    let runtime_text = serde_json::to_string_pretty(&runtime_metadata)
        .map_err(|error| format!("failed to serialize runtime animation: {error}"))?;
    fs::write(&runtime_path, format!("{runtime_text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", runtime_path.display()))?;
    update_runtime_catalog(
        root,
        &runtime_metadata.asset_id,
        &runtime_relative,
        &generated_relative,
    )?;
    Ok(AnimationPublishResult {
        runtime_metadata_path: runtime_relative,
        generated_image_path: generated_relative,
        clip_count: runtime_metadata.clips.len(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeAnimationCatalog {
    schema: String,
    animations: Vec<RuntimeAnimationCatalogEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeAnimationCatalogEntry {
    asset_id: String,
    metadata_path: String,
    image_path: String,
}

fn update_runtime_catalog(
    root: &Path,
    asset_id: &str,
    metadata_path: &str,
    image_path: &str,
) -> Result<(), String> {
    let path = resolve_path(root, RUNTIME_ANIMATION_CATALOG_PATH);
    let mut catalog = if path.is_file() {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        serde_json::from_str::<RuntimeAnimationCatalog>(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
    } else {
        RuntimeAnimationCatalog {
            schema: "havenwild.runtime_animation_catalog.v0_1".to_string(),
            animations: Vec::new(),
        }
    };
    let entry = RuntimeAnimationCatalogEntry {
        asset_id: asset_id.to_string(),
        metadata_path: metadata_path.to_string(),
        image_path: image_path.to_string(),
    };
    if let Some(existing) = catalog
        .animations
        .iter_mut()
        .find(|existing| existing.asset_id == asset_id)
    {
        *existing = entry;
    } else {
        catalog.animations.push(entry);
        catalog
            .animations
            .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&catalog)
        .map_err(|error| format!("failed to serialize animation catalog: {error}"))?;
    fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(super) fn resolve_path(root: &Path, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut previous_underscore = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_underscore = false;
        } else if !previous_underscore && !output.is_empty() {
            output.push('_');
            previous_underscore = true;
        }
    }
    output.trim_matches('_').to_string()
}
