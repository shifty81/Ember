use ember_app_shell::FoundryShell;
use ember_core::StableId;
use ember_documents::{DocumentEnvelope, DocumentKind};
use ember_editor_core::register_standard_workspaces;
use ember_project::ProjectManifest;
use serde_json::json;
use std::env;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("Ember Editor failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut shell = FoundryShell::default();
    register_standard_workspaces(&mut shell.workspaces);

    let args: Vec<String> = env::args().collect();
    if let Some(path) = args.get(1) {
        shell
            .load_project(Path::new(path))
            .map_err(|error| error.to_string())?;
    } else {
        let project = ProjectManifest::new(
            StableId::new("project", "untitled").map_err(|error| error.to_string())?,
            "Untitled Ember Project",
        );
        shell.editor.active_project = Some(project.project_id.clone());
        shell.project = Some(project);
    }

    let welcome_id = StableId::new("document", "welcome").map_err(|error| error.to_string())?;
    shell.insert_document(
        DocumentEnvelope {
            format_version: 1,
            id: welcome_id,
            kind: DocumentKind::Custom("welcome".into()),
            revision: 0,
            payload: json!({
                "title": "Ember Editor",
                "message": "Unified Ember 2D/3D game creation workspace"
            }),
        },
        "Welcome",
        None,
    );

    let project_name = shell
        .project
        .as_ref()
        .map(|project| project.name.as_str())
        .unwrap_or("No project");
    println!("Ember Editor O2D-R005 shell model");
    println!("Project: {project_name}");
    println!("Workspaces: {}", shell.workspaces.ids().count());
    println!("Open documents: {}", shell.tabs.len());
    println!("Native window/render backend integration continues after shell certification.");
    Ok(())
}
