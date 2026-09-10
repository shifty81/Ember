//! Public tool routing from certified Ember capabilities to external clients.

use ember_capabilities::{CapabilityCatalog, CapabilityState};
use ember_cortex_adapter::Handshake;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteTarget {
    ProjectControlCenter,
    EditorSession,
    AssetService,
    DocumentService,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteRecord {
    pub tool: String,
    pub capability: String,
    pub target: RouteTarget,
}

#[derive(Clone, Debug, Default)]
pub struct AdapterRouter {
    routes: BTreeMap<String, RouteRecord>,
}

impl AdapterRouter {
    pub fn register(&mut self, record: RouteRecord) -> Option<RouteRecord> {
        self.routes.insert(record.tool.clone(), record)
    }

    pub fn resolve(
        &self,
        tool: &str,
        capabilities: &CapabilityCatalog,
    ) -> Result<&RouteRecord, String> {
        let route = self
            .routes
            .get(tool)
            .ok_or_else(|| format!("tool {tool} is not registered"))?;
        let capability = capabilities
            .capabilities
            .iter()
            .find(|candidate| candidate.id == route.capability)
            .ok_or_else(|| {
                format!(
                    "tool {tool} requires missing capability {}",
                    route.capability
                )
            })?;
        if capability.state != CapabilityState::Certified {
            return Err(format!(
                "tool {tool} capability {} is not certified",
                route.capability
            ));
        }
        Ok(route)
    }

    pub fn handshake(&self, capabilities: &CapabilityCatalog) -> Handshake {
        Handshake::ember(capabilities.certified_ids())
    }

    pub fn registered_tools(&self) -> impl Iterator<Item = &str> {
        self.routes.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_capabilities::{CapabilityRecord, CapabilityState};

    #[test]
    fn router_refuses_candidate_capabilities() {
        let catalog = CapabilityCatalog {
            schema_version: 1,
            project_id: "ember".into(),
            capabilities: vec![CapabilityRecord {
                id: "editor.play_test".into(),
                version: 1,
                state: CapabilityState::Candidate,
                owner: "ember_session".into(),
                mutation: true,
                requires_approval: false,
                platforms: vec![],
                dependencies: vec![],
                evidence: vec![],
            }],
        };
        let mut router = AdapterRouter::default();
        router.register(RouteRecord {
            tool: "ember.editor.play_test".into(),
            capability: "editor.play_test".into(),
            target: RouteTarget::EditorSession,
        });
        assert!(router.resolve("ember.editor.play_test", &catalog).is_err());
    }
}
