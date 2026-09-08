use crate::category_metadata::{attach_category_metadata, AnimationClip, AnimationFrame, AnimationMetadata, CategoryMetadata, CollisionShape, TerrainMatchMode, TerrainMetadata, TerrainPeers, CATEGORY_METADATA_SCHEMA};
use crate::asset_pack::{
    AssetCategory, AssetDefinition, AssetId, AssetPackId, AssetPackManifest, AssetSource,
    AssetSourceId, AssetSourceKind, AtlasRegion, LicenseRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ImportConfidence { Unsupported, Low, Medium, High, Exact }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportState { Detected, ManualOnly, PartiallyConfigured, RuntimeReady, ProductionVerified, ReferenceOnly, Rejected }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSource {
    pub path: PathBuf,
    #[serde(default)]
    pub category_hint: Option<AssetCategory>,
    #[serde(default)]
    pub tile_size: Option<[u32; 2]>,
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportDiagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectedSource {
    pub provider_id: String,
    pub confidence: ImportConfidence,
    pub kind: AssetSourceKind,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportPreview {
    pub provider_id: String,
    pub state: ImportState,
    pub detected_sources: Vec<DetectedSource>,
    pub asset_count: usize,
    pub source_count: usize,
    #[serde(default)]
    pub categories: BTreeSet<AssetCategory>,
    #[serde(default)]
    pub unresolved: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportRequest {
    pub source: ImportSource,
    pub pack_id: AssetPackId,
    pub display_name: String,
    pub license: LicenseRecord,
    #[serde(default)]
    pub approval_state: Option<ImportState>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportProfile {
    pub schema: String,
    pub id: String,
    pub provider: String,
    #[serde(default)]
    pub tile_size: Option<[u32; 2]>,
    #[serde(default)]
    pub margin: u32,
    #[serde(default)]
    pub spacing: u32,
    #[serde(default)]
    pub properties: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedAssetPack {
    pub manifest: AssetPackManifest,
    pub state: ImportState,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
    #[serde(default)]
    pub generated_profiles: Vec<ImportProfile>,
}

pub trait AssetImportProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn can_import(&self, source: &ImportSource) -> ImportConfidence;
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String>;
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String>;
}

#[derive(Default)]
pub struct AssetImportRegistry { providers: Vec<Box<dyn AssetImportProvider>> }

impl AssetImportRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        registry.register(RawSheetImportProvider);
        registry.register(JsonSidecarImportProvider);
        registry.register(TiledTsxImportProvider);
        registry.register(AudioFolderImportProvider);
        registry.register(SpriteAnimationSheetImportProvider);
        registry
    }
    pub fn register<P: AssetImportProvider + 'static>(&mut self, provider: P) { self.providers.push(Box::new(provider)); }
    pub fn detect(&self, source: &ImportSource) -> Vec<(&dyn AssetImportProvider, ImportConfidence)> {
        let mut matches: Vec<_> = self.providers.iter().map(|p| (p.as_ref(), p.can_import(source))).filter(|(_, c)| *c != ImportConfidence::Unsupported).collect();
        matches.sort_by_key(|(_, confidence)| std::cmp::Reverse(*confidence));
        matches
    }
    pub fn best(&self, source: &ImportSource) -> Option<&dyn AssetImportProvider> { self.detect(source).first().map(|(provider, _)| *provider) }
}

fn extension(path: &Path) -> String { path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase() }
fn default_category(source: &ImportSource) -> AssetCategory { source.category_hint.clone().unwrap_or(AssetCategory::Other) }
fn source_path(path: &Path) -> String { path.to_string_lossy().replace('\\', "/") }

fn png_dimensions(path: &Path) -> Result<[u32; 2], String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" { return Err("raw-sheet prototype currently requires PNG input".to_string()); }
    Ok([u32::from_be_bytes(bytes[16..20].try_into().unwrap()), u32::from_be_bytes(bytes[20..24].try_into().unwrap())])
}

fn base_manifest(request: &ImportRequest, sources: Vec<AssetSource>, assets: Vec<AssetDefinition>) -> AssetPackManifest {
    AssetPackManifest { schema: AssetPackManifest::SCHEMA.to_string(), id: request.pack_id.clone(), display_name: request.display_name.clone(), version: "1".to_string(), license: request.license.clone(), production_enabled: request.approval_state == Some(ImportState::ProductionVerified), dependencies: vec![], sources, assets, priority: 0 }
}

pub struct RawSheetImportProvider;
impl AssetImportProvider for RawSheetImportProvider {
    fn id(&self) -> &'static str { "raw_image_sheet" }
    fn can_import(&self, source: &ImportSource) -> ImportConfidence { if matches!(extension(&source.path).as_str(), "png" | "jpg" | "jpeg" | "webp") { ImportConfidence::Medium } else { ImportConfidence::Unsupported } }
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String> {
        let dimensions = png_dimensions(&source.path)?;
        let tile = source.tile_size.unwrap_or([32, 32]);
        let count = usize::try_from((dimensions[0] / tile[0]) * (dimensions[1] / tile[1])).unwrap_or(0);
        Ok(ImportPreview { provider_id: self.id().to_string(), state: ImportState::ManualOnly, detected_sources: vec![DetectedSource { provider_id: self.id().to_string(), confidence: self.can_import(source), kind: AssetSourceKind::RawSheet, path: source_path(&source.path) }], asset_count: count, source_count: 1, categories: [default_category(source)].into_iter().collect(), unresolved: vec!["semantic IDs, terrain patterns, animation groups, collision, and production approval require review".to_string()], diagnostics: vec![] })
    }
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String> {
        let dimensions = png_dimensions(&request.source.path)?;
        let tile = request.source.tile_size.unwrap_or([32, 32]);
        if tile[0] == 0 || tile[1] == 0 || dimensions[0] % tile[0] != 0 || dimensions[1] % tile[1] != 0 { return Err("sheet dimensions must divide evenly by tile size".to_string()); }
        let source_id = AssetSourceId("sheet".to_string());
        let mut assets = Vec::new();
        for row in 0..dimensions[1] / tile[1] { for column in 0..dimensions[0] / tile[0] {
            let id = format!("cell_{column}_{row}");
            assets.push(AssetDefinition { id: AssetId(id.clone()), category: default_category(&request.source), semantic_id: format!("manual.{id}"), source_id: source_id.clone(), atlas_region: Some(AtlasRegion { x: column * tile[0], y: row * tile[1], width: tile[0], height: tile[1] }), variants: vec![], tags: ["manual_only".to_string()].into_iter().collect(), metadata: BTreeMap::new() });
        }}
        let source = AssetSource { id: source_id, kind: AssetSourceKind::RawSheet, path: source_path(&request.source.path), tile_size: Some(tile), margin: 0, spacing: 0 };
        let profile = ImportProfile { schema: "havenwild.import_profile.v1".to_string(), id: format!("{}_raw_sheet", request.pack_id.0), provider: self.id().to_string(), tile_size: Some(tile), margin: 0, spacing: 0, properties: BTreeMap::new() };
        Ok(ImportedAssetPack { manifest: base_manifest(request, vec![source], assets), state: request.approval_state.unwrap_or(ImportState::ManualOnly), diagnostics: vec![], generated_profiles: vec![profile] })
    }
}

pub struct JsonSidecarImportProvider;
impl AssetImportProvider for JsonSidecarImportProvider {
    fn id(&self) -> &'static str { "json_sidecar" }
    fn can_import(&self, source: &ImportSource) -> ImportConfidence { if extension(&source.path) == "json" { ImportConfidence::High } else { ImportConfidence::Unsupported } }
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String> {
        let text = fs::read_to_string(&source.path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let assets = value.get("assets").and_then(|v| v.as_array()).map_or(0, Vec::len);
        Ok(ImportPreview { provider_id: self.id().to_string(), state: ImportState::PartiallyConfigured, detected_sources: vec![DetectedSource { provider_id: self.id().to_string(), confidence: ImportConfidence::High, kind: AssetSourceKind::Sidecar, path: source_path(&source.path) }], asset_count: assets, source_count: value.get("sources").and_then(|v| v.as_array()).map_or(0, Vec::len), categories: BTreeSet::new(), unresolved: vec![], diagnostics: vec![] })
    }
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String> {
        let text = fs::read_to_string(&request.source.path).map_err(|e| e.to_string())?;
        let mut manifest: AssetPackManifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        manifest.id = request.pack_id.clone(); manifest.display_name = request.display_name.clone(); manifest.license = request.license.clone();
        manifest.production_enabled = request.approval_state == Some(ImportState::ProductionVerified);
        manifest.validate().map_err(|e| e.join("; "))?;
        Ok(ImportedAssetPack { manifest, state: request.approval_state.unwrap_or(ImportState::PartiallyConfigured), diagnostics: vec![], generated_profiles: vec![] })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct TiledTileMetadata {
    pub tile_id: u32,
    #[serde(default)] pub class_name: Option<String>,
    #[serde(default)] pub probability: Option<f32>,
    #[serde(default)] pub properties: BTreeMap<String, serde_json::Value>,
    #[serde(default)] pub animation: Vec<AnimationFrame>,
    #[serde(default)] pub collision_shapes: Vec<CollisionShape>,
    #[serde(default)] pub wang_assignments: Vec<TiledWangAssignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TiledWangAssignment {
    pub wang_set: String,
    pub wang_id: Vec<u32>,
}

pub struct TiledTsxImportProvider;
include!("asset_import_tiled.rs");

impl AssetImportProvider for AudioFolderImportProvider {
    fn id(&self) -> &'static str { "audio_folder" }
    fn can_import(&self, source: &ImportSource) -> ImportConfidence { if source.path.is_dir() && fs::read_dir(&source.path).ok().into_iter().flatten().flatten().any(|e| matches!(extension(&e.path()).as_str(), "wav" | "ogg" | "mp3" | "flac")) { ImportConfidence::High } else { ImportConfidence::Unsupported } }
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String> { let count = audio_files(&source.path)?.len(); Ok(ImportPreview { provider_id: self.id().to_string(), state: ImportState::RuntimeReady, detected_sources: vec![DetectedSource { provider_id: self.id().to_string(), confidence: ImportConfidence::High, kind: AssetSourceKind::Audio, path: source_path(&source.path) }], asset_count: count, source_count: count, categories: [AssetCategory::Audio].into_iter().collect(), unresolved: vec![], diagnostics: vec![] }) }
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String> { let files = audio_files(&request.source.path)?; let mut sources = Vec::new(); let mut assets = Vec::new(); for path in files { let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("audio").to_string(); let id = AssetSourceId(stem.clone()); sources.push(AssetSource { id: id.clone(), kind: AssetSourceKind::Audio, path: source_path(&path), tile_size: None, margin: 0, spacing: 0 }); assets.push(AssetDefinition { id: AssetId(stem.clone()), category: AssetCategory::Audio, semantic_id: format!("audio.{stem}"), source_id: id, atlas_region: None, variants: vec![], tags: BTreeSet::new(), metadata: BTreeMap::new() }); } Ok(ImportedAssetPack { manifest: base_manifest(request, sources, assets), state: request.approval_state.unwrap_or(ImportState::RuntimeReady), diagnostics: vec![], generated_profiles: vec![] }) }
}
fn audio_files(path: &Path) -> Result<Vec<PathBuf>, String> { let mut files: Vec<_> = fs::read_dir(path).map_err(|e| e.to_string())?.filter_map(Result::ok).map(|e| e.path()).filter(|p| matches!(extension(p).as_str(), "wav" | "ogg" | "mp3" | "flac")).collect(); files.sort(); Ok(files) }

pub struct SpriteAnimationSheetImportProvider;
impl AssetImportProvider for SpriteAnimationSheetImportProvider {
    fn id(&self) -> &'static str { "sprite_animation_sheet" }
    fn can_import(&self, source: &ImportSource) -> ImportConfidence { if extension(&source.path) == "png" && matches!(source.category_hint, Some(AssetCategory::Animation)) { ImportConfidence::High } else { ImportConfidence::Unsupported } }
    fn inspect(&self, source: &ImportSource) -> Result<ImportPreview, String> { RawSheetImportProvider.inspect(source).map(|mut preview| { preview.provider_id = self.id().to_string(); preview.state = ImportState::PartiallyConfigured; preview.unresolved = vec!["animation clip rows, directions, frame timing, and anchors require an import profile".to_string()]; preview }) }
    fn import(&self, request: &ImportRequest) -> Result<ImportedAssetPack, String> { let mut imported = RawSheetImportProvider.import(request)?; imported.state = request.approval_state.unwrap_or(ImportState::PartiallyConfigured); for asset in &mut imported.manifest.assets { asset.category = AssetCategory::Animation; asset.tags.insert("animation_frame".to_string()); } imported.generated_profiles[0].provider = self.id().to_string(); Ok(imported) }
}

#[cfg(test)]
mod tests {
    include!("asset_import_tests.rs");
}
