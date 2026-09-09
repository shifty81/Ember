use ember_assets::AssetRegistry;
use ember_capabilities::{CapabilityCatalog, CapabilityState};
use ember_core::{Diagnostic, Severity};
use ember_documents::DocumentRegistry;
use ember_packages::PackageRegistry;
use ember_project::ProjectManifest;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.severity, Severity::Error))
    }
}

pub struct ProjectValidationContext<'a> {
    pub project: &'a ProjectManifest,
    pub documents: &'a DocumentRegistry,
    pub assets: Option<&'a AssetRegistry>,
    pub packages: Option<&'a PackageRegistry>,
}

pub fn validate_project(
    project: &ProjectManifest,
    documents: &DocumentRegistry,
) -> ValidationReport {
    validate_project_context(ProjectValidationContext {
        project,
        documents,
        assets: None,
        packages: None,
    })
}

pub fn validate_project_context(context: ProjectValidationContext<'_>) -> ValidationReport {
    let mut diagnostics = context.documents.validate();
    if let Err(error) = context.project.validate() {
        diagnostics.push(Diagnostic::error("EMBER-PROJECT-001", error.to_string()));
    }
    if let Some(assets) = context.assets {
        if let Err(error) = assets.validate() {
            diagnostics.push(Diagnostic::error("EMBER-ASSET-001", error.to_string()));
        }
    }
    if let Some(packages) = context.packages {
        if let Err(error) = packages.resolve(&context.project.enabled_packages) {
            diagnostics.push(Diagnostic::error("EMBER-PACKAGE-001", error.to_string()));
        }
    }
    ValidationReport { diagnostics }
}

pub fn validate_architecture(root: &Path) -> ValidationReport {
    let mut diagnostics = Vec::new();
    let required = [
        "Cargo.toml",
        "forge.project.json",
        "project-control-center.profile.json",
        "config/ember/capabilities.json",
        "integrations/cortex/adapter.json",
        "PROJECT_CONTROL_CENTER.cmd",
        "crates/ember_packages/Cargo.toml",
        "crates/ember_jobs/Cargo.toml",
        "crates/ember_session/Cargo.toml",
        "certification/ember_smoke/ember.project.json",
    ];
    for relative in required {
        if !root.join(relative).exists() {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-001",
                format!("required architecture path is missing: {relative}"),
            ));
        }
    }

    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    if cargo.contains("reference/donor") || cargo.contains("haven_editor_native") {
        diagnostics.push(Diagnostic::error(
            "EMBER-ARCH-002",
            "donor editor is present in the active Cargo workspace",
        ));
    }
    for member in [
        "ember_packages",
        "ember_jobs",
        "ember_session",
        "ember_capabilities",
    ] {
        if !cargo.contains(member) {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-003",
                format!("workspace is missing required foundation crate {member}"),
            ));
        }
    }

    match CapabilityCatalog::load(&root.join("config/ember/capabilities.json")) {
        Ok(catalog) => {
            if let Ok(adapter) = read_json(&root.join("integrations/cortex/adapter.json")) {
                let advertised: BTreeSet<String> = adapter
                    .get("capabilities")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                let certified: BTreeSet<String> = catalog
                    .capabilities
                    .iter()
                    .filter(|capability| capability.state == CapabilityState::Certified)
                    .map(|capability| capability.id.clone())
                    .collect();
                for id in advertised.difference(&certified) {
                    diagnostics.push(Diagnostic::error(
                        "EMBER-ARCH-004",
                        format!("Cortex adapter advertises non-certified capability {id}"),
                    ));
                }
                if adapter
                    .get("authority")
                    .and_then(|value| value.get("project_operations"))
                    .and_then(Value::as_str)
                    != Some("pcc")
                {
                    diagnostics.push(Diagnostic::error(
                        "EMBER-ARCH-005",
                        "Cortex adapter project operations authority must be pcc",
                    ));
                }
            }
        }
        Err(error) => diagnostics.push(Diagnostic::error("EMBER-ARCH-006", error)),
    }

    if let Ok(project) = read_json(&root.join("forge.project.json")) {
        if project.get("id").and_then(Value::as_str) != Some("ember") {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-007",
                "forge.project.json id is not ember",
            ));
        }
        let external = project
            .get("project_control_center")
            .and_then(|value| value.get("external_runtime_required"))
            .and_then(Value::as_bool);
        if external != Some(false) {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-008",
                "Ember PCC must not require an external runtime",
            ));
        }
    }

    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed.contains("cortex")
            && trimmed.contains("crates/")
            && !trimmed.contains("crates/ember_cortex_adapter")
        {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-009",
                format!("unexpected Cortex workspace member: {trimmed}"),
            ));
        }
    }

    for source_root in [root.join("apps"), root.join("crates")] {
        scan_active_source_names(&source_root, &mut diagnostics);
    }

    ValidationReport { diagnostics }
}

fn scan_active_source_names(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_active_source_names(&path, diagnostics);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        if path.ends_with(Path::new("crates/ember_validation/src/lib.rs")) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        for token in ["Open2D", "Open2d", "O2D-", "Havenwild"] {
            if text.contains(token) {
                diagnostics.push(Diagnostic::error(
                    "EMBER-ARCH-010",
                    format!(
                        "active source {} contains legacy product token {token}",
                        path.display()
                    ),
                ));
            }
        }
        if text.contains("FoundryShell")
            && !path.ends_with(Path::new("crates/ember_app_shell/src/lib.rs"))
        {
            diagnostics.push(Diagnostic::error(
                "EMBER-ARCH-011",
                format!(
                    "active source {} uses FoundryShell outside its compatibility alias",
                    path.display()
                ),
            ));
        }
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
