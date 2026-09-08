use crate::{
    donor_reference_catalog::DonorReferenceAssetRecord,
    prototype_bake::{
        PrototypeAssetBakeDryRunReport, PrototypeAssetBakeStatus,
        PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, path::Path};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSourceAvailabilityState {
    Ready,
    MissingSource,
    RefusedBlockedSource,
    PlanError,
    Unknown,
}

impl AssetSourceAvailabilityState {
    pub fn label(self) -> &'static str {
        match self {
            AssetSourceAvailabilityState::Ready => "ready",
            AssetSourceAvailabilityState::MissingSource => "missing source",
            AssetSourceAvailabilityState::RefusedBlockedSource => "blocked/refused",
            AssetSourceAvailabilityState::PlanError => "plan error",
            AssetSourceAvailabilityState::Unknown => "not in bake plan",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            AssetSourceAvailabilityState::Ready => "ready",
            AssetSourceAvailabilityState::MissingSource => "missing",
            AssetSourceAvailabilityState::RefusedBlockedSource => "blocked",
            AssetSourceAvailabilityState::PlanError => "error",
            AssetSourceAvailabilityState::Unknown => "n/a",
        }
    }

    fn rank(self) -> u8 {
        match self {
            AssetSourceAvailabilityState::PlanError => 5,
            AssetSourceAvailabilityState::RefusedBlockedSource => 4,
            AssetSourceAvailabilityState::MissingSource => 3,
            AssetSourceAvailabilityState::Unknown => 2,
            AssetSourceAvailabilityState::Ready => 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetSourceAvailabilityRecord {
    pub external_source_id: String,
    pub state: AssetSourceAvailabilityState,
    pub inputs_found: usize,
    pub inputs_total: usize,
    pub dry_run_entry_count: usize,
    pub report_path: String,
    pub reason: String,
}

pub fn load_asset_source_availability_report(
    repo_root: impl AsRef<Path>,
) -> Result<PrototypeAssetBakeDryRunReport, String> {
    let path = repo_root
        .as_ref()
        .join(PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH);
    let contents =
        read_to_string(&path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn build_asset_source_availability_index(
    report: &PrototypeAssetBakeDryRunReport,
) -> BTreeMap<String, AssetSourceAvailabilityRecord> {
    let mut index: BTreeMap<String, AssetSourceAvailabilityRecord> = BTreeMap::new();
    for entry in &report.entries {
        let state = availability_state_from_bake_status(&entry.status);
        let inputs_total = entry.inputs.len();
        let inputs_found = entry.inputs.iter().filter(|input| input.found).count();
        merge_availability_record(
            &mut index,
            AssetSourceAvailabilityRecord {
                external_source_id: entry.external_source_id.clone(),
                state,
                inputs_found,
                inputs_total,
                dry_run_entry_count: 1,
                report_path: report.report_path.clone(),
                reason: if entry.notes.is_empty() {
                    format!("dry-run bake entry {} reported {}", entry.id, state.label())
                } else {
                    entry.notes.join("; ")
                },
            },
        );
    }

    for probe in &report.blocked_source_probes {
        merge_availability_record(
            &mut index,
            AssetSourceAvailabilityRecord {
                external_source_id: probe.source_id.clone(),
                state: AssetSourceAvailabilityState::RefusedBlockedSource,
                inputs_found: 0,
                inputs_total: 0,
                dry_run_entry_count: 1,
                report_path: report.report_path.clone(),
                reason: probe.reason.clone(),
            },
        );
    }
    index
}

pub fn availability_for_donor_record(
    record: &DonorReferenceAssetRecord,
    index: &BTreeMap<String, AssetSourceAvailabilityRecord>,
) -> AssetSourceAvailabilityRecord {
    index
        .get(record.external_source_id.as_str())
        .cloned()
        .unwrap_or_else(|| AssetSourceAvailabilityRecord {
            external_source_id: record.external_source_id.clone(),
            state: AssetSourceAvailabilityState::Unknown,
            inputs_found: 0,
            inputs_total: 0,
            dry_run_entry_count: 0,
            report_path: PROTOTYPE_ASSET_LOCAL_BAKE_REPORT_PATH.to_string(),
            reason: "source is cataloged for reference but has no prototype bake adapter yet"
                .to_string(),
        })
}

pub fn asset_source_availability_summary_line(report: &PrototypeAssetBakeDryRunReport) -> String {
    let index = build_asset_source_availability_index(report);
    let ready = index
        .values()
        .filter(|record| record.state == AssetSourceAvailabilityState::Ready)
        .count();
    let missing = index
        .values()
        .filter(|record| record.state == AssetSourceAvailabilityState::MissingSource)
        .count();
    let blocked = index
        .values()
        .filter(|record| record.state == AssetSourceAvailabilityState::RefusedBlockedSource)
        .count();
    let error = index
        .values()
        .filter(|record| record.state == AssetSourceAvailabilityState::PlanError)
        .count();
    format!(
        "source availability: {ready} ready | {missing} missing source | {blocked} blocked/refused | {error} error"
    )
}

fn availability_state_from_bake_status(
    status: &PrototypeAssetBakeStatus,
) -> AssetSourceAvailabilityState {
    match status {
        PrototypeAssetBakeStatus::Ready => AssetSourceAvailabilityState::Ready,
        PrototypeAssetBakeStatus::MissingSource => AssetSourceAvailabilityState::MissingSource,
        PrototypeAssetBakeStatus::RefusedBlockedSource => {
            AssetSourceAvailabilityState::RefusedBlockedSource
        }
        PrototypeAssetBakeStatus::PlanError => AssetSourceAvailabilityState::PlanError,
    }
}

fn merge_availability_record(
    index: &mut BTreeMap<String, AssetSourceAvailabilityRecord>,
    next: AssetSourceAvailabilityRecord,
) {
    let Some(current) = index.get_mut(next.external_source_id.as_str()) else {
        index.insert(next.external_source_id.clone(), next);
        return;
    };
    current.inputs_found += next.inputs_found;
    current.inputs_total += next.inputs_total;
    current.dry_run_entry_count += next.dry_run_entry_count;
    if next.state.rank() > current.state.rank() {
        current.state = next.state;
        current.reason = next.reason;
    } else if !next.reason.is_empty() && !current.reason.contains(&next.reason) {
        current.reason = format!("{}; {}", current.reason, next.reason);
    }
}
