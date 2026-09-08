use ember_project::ProjectManifest;
use std::path::Path;
pub fn project_status(path: &Path) -> Result<String, String> {
    let project = ProjectManifest::load(path).map_err(|e| e.to_string())?;
    Ok(format!(
        "{} [{}] format {}",
        project.name, project.project_id, project.format_version
    ))
}
