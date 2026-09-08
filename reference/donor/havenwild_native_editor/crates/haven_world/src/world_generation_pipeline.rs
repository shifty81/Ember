use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

pub const WORLD_GENERATION_PIPELINE_SCHEMA: &str = "havenwild.world_generation_pipeline.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldGenerationPipeline {
    pub schema: String,
    pub version: u32,
    pub policy: PipelinePolicy,
    pub layers: Vec<PipelineLayer>,
    pub stages: Vec<PipelineStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelinePolicy {
    pub deterministic_by_default: bool,
    pub undeclared_mutation: String,
    pub stage_order: String,
    pub preview_must_be_reversible: bool,
    pub authored_overrides_survive_regeneration: bool,
    pub player_deltas_survive_regeneration: bool,
    pub future_stage_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineLayer {
    pub id: String,
    pub owner: String,
    pub persistence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    pub id: String,
    pub order: u32,
    pub status: String,
    pub seed_domain: String,
    pub deterministic: bool,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub may_mutate: Vec<String>,
    pub must_not_mutate: Vec<String>,
    pub preview: String,
    pub runtime_regeneration: String,
    pub persistence_interaction: String,
    pub validator: String,
}

impl WorldGenerationPipeline {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read world-generation pipeline: {e}"))?;
        let pipeline: Self = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse world-generation pipeline: {e}"))?;
        pipeline.validate()?;
        Ok(pipeline)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_GENERATION_PIPELINE_SCHEMA || self.version != 1 {
            return Err("unsupported world-generation pipeline schema/version".into());
        }
        let layer_ids: HashSet<_> = self.layers.iter().map(|x| x.id.as_str()).collect();
        if layer_ids.len() != self.layers.len() {
            return Err("duplicate pipeline layer id".into());
        }
        let mut ids = HashSet::new();
        let mut orders = HashSet::new();
        let mut seeds = HashSet::new();
        let owners: HashMap<_, _> = self
            .layers
            .iter()
            .map(|x| (x.id.as_str(), x.owner.as_str()))
            .collect();
        for stage in &self.stages {
            if !ids.insert(stage.id.as_str()) {
                return Err(format!("duplicate stage id {}", stage.id));
            }
            if !orders.insert(stage.order) {
                return Err(format!("duplicate stage order {}", stage.order));
            }
            if !seeds.insert(stage.seed_domain.as_str()) {
                return Err(format!("duplicate seed domain {}", stage.seed_domain));
            }
            if stage.deterministic && stage.seed_domain.is_empty() {
                return Err(format!(
                    "deterministic stage {} has no seed domain",
                    stage.id
                ));
            }
            if stage.outputs.is_empty() || stage.may_mutate.is_empty() {
                return Err(format!(
                    "stage {} must declare outputs and mutation rights",
                    stage.id
                ));
            }
            for output in &stage.outputs {
                if !layer_ids.contains(output.as_str()) {
                    return Err(format!(
                        "stage {} outputs unknown layer {}",
                        stage.id, output
                    ));
                }
            }
            for layer in &stage.may_mutate {
                if !layer_ids.contains(layer.as_str()) {
                    return Err(format!(
                        "stage {} mutates unknown layer {}",
                        stage.id, layer
                    ));
                }
            }
            for layer in &stage.may_mutate {
                if stage.must_not_mutate.contains(layer) {
                    return Err(format!(
                        "stage {} both may and must-not mutate {}",
                        stage.id, layer
                    ));
                }
            }
            for output in &stage.outputs {
                if owners.get(output.as_str()).copied() != Some(stage.id.as_str()) {
                    return Err(format!(
                        "layer {} owner does not match stage {}",
                        output, stage.id
                    ));
                }
            }
        }
        let mut sorted: Vec<_> = self.stages.iter().map(|x| x.order).collect();
        sorted.sort_unstable();
        if sorted != (1..=self.stages.len() as u32).collect::<Vec<_>>() {
            return Err("stage order must be contiguous and append-only".into());
        }
        Ok(())
    }

    pub fn stage(&self, id: &str) -> Option<&PipelineStage> {
        self.stages.iter().find(|stage| stage.id == id)
    }
}
