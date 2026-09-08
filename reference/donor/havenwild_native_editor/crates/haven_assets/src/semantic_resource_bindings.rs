use crate::asset_pack::{AssetCategory, AssetSourceKind, StableAssetRef};
use crate::runtime_asset_cache::RuntimeAssetSession;
use crate::semantic_asset_resolution::AssetResolutionContext;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticResourceBinding {
    pub semantic_id: String,
    pub category: AssetCategory,
    pub stable_ref: StableAssetRef,
    pub source_kind: AssetSourceKind,
    pub source_path: PathBuf,
}

#[derive(Default)]
pub struct SemanticResourceRegistry {
    bindings: BTreeMap<(String, AssetCategory), SemanticResourceBinding>,
    diagnostics: Vec<String>,
}

impl SemanticResourceRegistry {
    pub fn resolve(
        &mut self,
        session: Option<&RuntimeAssetSession>,
        semantic_id: &str,
        category: AssetCategory,
        fallback: Option<&Path>,
    ) -> Option<PathBuf> {
        if let Some(session) = session {
            if let Some(source) = session.resolve_source(
                semantic_id,
                category.clone(),
                AssetResolutionContext::default(),
            ) {
                let binding = SemanticResourceBinding {
                    semantic_id: semantic_id.to_string(),
                    category: category.clone(),
                    stable_ref: source.stable_ref.clone(),
                    source_kind: source.source_kind.clone(),
                    source_path: source.source_path.clone(),
                };
                let path = binding.source_path.clone();
                self.bindings
                    .insert((semantic_id.to_string(), category), binding);
                return Some(path);
            }
        }
        if let Some(path) = fallback {
            self.diagnostics.push(format!(
                "No stable non-texture provider for {semantic_id}; using legacy path {}",
                path.display()
            ));
            return Some(path.to_path_buf());
        }
        self.diagnostics
            .push(format!("No stable non-texture provider for {semantic_id}"));
        None
    }

    pub fn binding(
        &self,
        semantic_id: &str,
        category: AssetCategory,
    ) -> Option<&SemanticResourceBinding> {
        self.bindings.get(&(semantic_id.to_string(), category))
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_session_uses_explicit_fallback() {
        let mut registry = SemanticResourceRegistry::default();
        let fallback = Path::new("content/example.json");
        assert_eq!(
            registry.resolve(
                None,
                "ui.contract.default",
                AssetCategory::Ui,
                Some(fallback)
            ),
            Some(fallback.to_path_buf())
        );
        assert_eq!(registry.diagnostics().len(), 1);
    }
}
