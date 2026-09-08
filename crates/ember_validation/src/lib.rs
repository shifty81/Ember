use ember_core::{Diagnostic, Severity};
use ember_documents::DocumentRegistry;
use ember_project::ProjectManifest;
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}
impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.severity, Severity::Error))
    }
}
pub fn validate_project(
    project: &ProjectManifest,
    documents: &DocumentRegistry,
) -> ValidationReport {
    let mut d = documents.validate();
    if project.content_roots.is_empty() {
        d.push(Diagnostic::error(
            "O2D-PROJECT-001",
            "project has no content roots",
        ));
    }
    ValidationReport { diagnostics: d }
}
