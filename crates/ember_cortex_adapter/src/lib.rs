use ember_capabilities::CapabilityCatalog;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const ADAPTER_PROTOCOL: &str = "ember.cortex.adapter";
pub const ADAPTER_PROTOCOL_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolStatus {
    Ok,
    Failed,
    Cancelled,
    ApprovalRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub protocol: String,
    pub protocol_version: String,
    pub project_schema: String,
    pub pcc_contract: String,
    pub capabilities: Vec<String>,
}

impl Handshake {
    pub fn ember(catalog: &CapabilityCatalog) -> Self {
        Self {
            protocol: ADAPTER_PROTOCOL.to_owned(),
            protocol_version: ADAPTER_PROTOCOL_VERSION.to_owned(),
            project_schema: "1".to_owned(),
            pcc_contract: "1".to_owned(),
            capabilities: catalog.certified_ids(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: String,
    pub tool: String,
    pub project_root: String,
    #[serde(default)]
    pub arguments: BTreeMap<String, Value>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub approval_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub status: ToolStatus,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
}

// No dependency on Cortex implementation crates. Executable transports remain
// unavailable until the standalone Cortex plugin SDK is certified.
