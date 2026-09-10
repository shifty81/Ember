use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("ember_tools lives under <root>/crates")
        .to_path_buf()
}

#[test]
fn third_party_dependency_metadata_has_governed_origin_and_license() {
    let root = workspace_root();
    assert!(root.join("Cargo.lock").is_file(), "Cargo.lock is required");

    let policy: Value = serde_json::from_slice(
        &std::fs::read(root.join("config/ember/dependency-policy.json"))
            .expect("read dependency policy"),
    )
    .expect("parse dependency policy");

    let allowed: Vec<&str> = policy["rules"]["allowed_registry_prefixes"]
        .as_array()
        .expect("allowed_registry_prefixes array")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        !allowed.is_empty(),
        "dependency policy must declare registry sources"
    );

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(&root)
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata JSON");
    let workspace_members: BTreeSet<String> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();

    let mut violations = Vec::new();
    for package in metadata["packages"].as_array().expect("packages") {
        let package_id = package["id"].as_str().unwrap_or_default();
        if workspace_members.contains(package_id) {
            continue;
        }

        let name = package["name"].as_str().unwrap_or("<unknown>");
        let version = package["version"].as_str().unwrap_or("<unknown>");
        let source = package["source"].as_str().unwrap_or_default();

        if source.starts_with("git+") {
            violations.push(format!(
                "{name} {version}: git dependency source is prohibited"
            ));
        } else if source.is_empty() {
            violations.push(format!(
                "{name} {version}: external path dependency is outside workspace authority"
            ));
        } else if !allowed.iter().any(|prefix| source.starts_with(prefix)) {
            violations.push(format!(
                "{name} {version}: dependency source is not approved: {source}"
            ));
        }

        let has_license = package["license"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty());
        let has_license_file = package["license_file"]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty());
        if !has_license && !has_license_file {
            violations.push(format!(
                "{name} {version}: no license metadata or license file declared"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "dependency policy violations:\n{}",
        violations.join("\n")
    );
}
