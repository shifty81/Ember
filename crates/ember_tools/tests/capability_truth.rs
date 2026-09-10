use ember_capabilities::{CapabilityCatalog, CapabilityState};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("ember_tools lives under <root>/crates")
        .to_path_buf()
}

#[test]
fn certified_capabilities_have_evidence_and_certified_dependencies() {
    let root = workspace_root();
    let catalog = CapabilityCatalog::load(&root.join("config/ember/capabilities.json"))
        .expect("capability catalog must load");

    let states: BTreeMap<&str, CapabilityState> = catalog
        .capabilities
        .iter()
        .map(|entry| (entry.id.as_str(), entry.state))
        .collect();

    let mut violations = Vec::new();
    for capability in &catalog.capabilities {
        if capability.state != CapabilityState::Certified {
            continue;
        }

        if capability.evidence.is_empty() {
            violations.push(format!(
                "{} is certified without executable/documented evidence",
                capability.id
            ));
        }

        for dependency in &capability.dependencies {
            match states.get(dependency.as_str()) {
                Some(CapabilityState::Certified) => {}
                Some(state) => violations.push(format!(
                    "{} is certified but depends on {} in state {:?}",
                    capability.id, dependency, state
                )),
                None => violations.push(format!(
                    "{} is certified but dependency {} is absent",
                    capability.id, dependency
                )),
            }
        }
    }

    assert!(
        violations.is_empty(),
        "capability truth violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn visual_editor_and_pie_capabilities_are_not_prematurely_certified() {
    let root = workspace_root();
    let catalog = CapabilityCatalog::load(&root.join("config/ember/capabilities.json"))
        .expect("capability catalog must load");
    let by_id = catalog.by_id();

    assert_ne!(
        by_id["editor.native_host"].state,
        CapabilityState::Certified,
        "native editor interaction has not been certified yet"
    );
    assert_ne!(
        by_id["editor.play_test"].state,
        CapabilityState::Certified,
        "PIE has not been certified yet"
    );
    assert_eq!(
        by_id["runtime.session.stdio"].state,
        CapabilityState::Certified,
        "runtime stdio service now has process-level certification"
    );
    assert_eq!(
        by_id["editor.save"].state,
        CapabilityState::Certified,
        "path-backed save now has integration-level certification"
    );
}
