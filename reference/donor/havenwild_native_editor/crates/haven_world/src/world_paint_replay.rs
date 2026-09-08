use haven_core::{GameWorld, SceneReference, TavernMap};

use crate::{
    apply_world_paint_brush, load_world_paint_delta_document, WorldPaintBrushSettings,
    WorldPaintDeltaDocument, WorldPaintDeltaRecord, WorldPaintFamily, WorldPaintLayer,
    WorldPaintSubcellMode,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldPaintReplayIssue {
    pub sequence: u64,
    pub scene_id: String,
    pub reason: String,
}

impl WorldPaintReplayIssue {
    pub fn new(sequence: u64, scene_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            sequence,
            scene_id: scene_id.into(),
            reason: reason.into(),
        }
    }

    pub fn status_line(&self) -> String {
        format!(
            "paint replay #{} {} skipped: {}",
            self.sequence, self.scene_id, self.reason
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldPaintReplayReport {
    pub source_path: Option<String>,
    pub requested_scene: Option<String>,
    pub records_seen: usize,
    pub records_replayed: usize,
    pub tiles_repainted: usize,
    pub affected_subcells: usize,
    pub cleanup_mutations: usize,
    pub skipped_records: usize,
    pub first_sequence: Option<u64>,
    pub last_sequence: Option<u64>,
    pub issues: Vec<WorldPaintReplayIssue>,
}

impl WorldPaintReplayReport {
    pub fn empty() -> Self {
        Self {
            source_path: None,
            requested_scene: None,
            records_seen: 0,
            records_replayed: 0,
            tiles_repainted: 0,
            affected_subcells: 0,
            cleanup_mutations: 0,
            skipped_records: 0,
            first_sequence: None,
            last_sequence: None,
            issues: Vec::new(),
        }
    }

    pub fn status_line(&self) -> String {
        let scene = self.requested_scene.as_deref().unwrap_or("all scenes");
        let range = match (self.first_sequence, self.last_sequence) {
            (Some(first), Some(last)) if first == last => format!("#{}", first),
            (Some(first), Some(last)) => format!("#{}..#{}", first, last),
            _ => "no sequence".to_string(),
        };
        format!(
            "Replayed {}/{} paint delta(s) for {} ({}); {} tile writes, {} subcells, {} cleanup mutation(s), {} skipped",
            self.records_replayed,
            self.records_seen,
            scene,
            range,
            self.tiles_repainted,
            self.affected_subcells,
            self.cleanup_mutations,
            self.skipped_records
        )
    }

    fn record_success(
        &mut self,
        sequence: u64,
        painted_tiles: usize,
        affected_subcells: usize,
        cleanup_mutations: usize,
    ) {
        self.records_replayed += 1;
        self.tiles_repainted += painted_tiles;
        self.affected_subcells += affected_subcells;
        self.cleanup_mutations += cleanup_mutations;
        self.first_sequence = Some(
            self.first_sequence
                .map_or(sequence, |current| current.min(sequence)),
        );
        self.last_sequence = Some(
            self.last_sequence
                .map_or(sequence, |current| current.max(sequence)),
        );
    }

    fn record_issue(&mut self, issue: WorldPaintReplayIssue) {
        self.skipped_records += 1;
        self.issues.push(issue);
    }
}

pub fn replay_world_paint_delta_document_onto_world(
    world: &mut GameWorld,
    doc: &WorldPaintDeltaDocument,
    scene_filter: Option<SceneReference>,
) -> WorldPaintReplayReport {
    let mut report = WorldPaintReplayReport::empty();
    report.records_seen = doc.records.len();
    report.requested_scene = scene_filter.as_ref().map(|scene| scene.code().to_string());

    for record in &doc.records {
        if let Some(scene_filter) = scene_filter.as_ref() {
            if record.scene_id != scene_filter.code() {
                continue;
            }
        }
        let scene_reference = SceneReference::from(record.scene_id.clone());
        let Some(scene) = world.scene_mut_by_reference(&scene_reference) else {
            report.record_issue(WorldPaintReplayIssue::new(
                record.sequence,
                &record.scene_id,
                "scene not present in world",
            ));
            continue;
        };
        match replay_world_paint_delta_record_onto_map(&mut scene.map, scene.biome, record) {
            Ok(record_report) => report.record_success(
                record.sequence,
                record_report.painted_tiles,
                record_report.affected_subcells,
                record_report.cleanup_mutations,
            ),
            Err(reason) => report.record_issue(WorldPaintReplayIssue::new(
                record.sequence,
                &record.scene_id,
                reason,
            )),
        }
    }

    report
}

pub fn replay_world_paint_delta_record_onto_map(
    map: &mut TavernMap,
    biome: haven_core::SceneBiome,
    record: &WorldPaintDeltaRecord,
) -> Result<crate::WorldPaintReport, String> {
    let family = WorldPaintFamily::from_code(&record.family)
        .ok_or_else(|| format!("unsupported paint family {}", record.family))?;
    let layer = WorldPaintLayer::from_code(&record.layer).unwrap_or_else(|| family.default_layer());
    let subcell_mode = WorldPaintSubcellMode::from_code(&record.subcell_mode)
        .ok_or_else(|| format!("unsupported subcell mode {}", record.subcell_mode))?;
    let settings = WorldPaintBrushSettings {
        family,
        layer,
        radius_tiles: record.radius_tiles,
        strength: record.strength,
        subcell_mode,
        autotile_refresh: record.autotile_refresh_requested,
        mirror_horizontal: record.mirror_horizontal,
        mirror_vertical: record.mirror_vertical,
    };
    Ok(apply_world_paint_brush(
        map,
        record.center[0],
        record.center[1],
        biome,
        settings,
    ))
}

pub fn load_and_replay_world_paint_deltas(
    world: &mut GameWorld,
    path: impl AsRef<std::path::Path>,
    scene_filter: Option<SceneReference>,
) -> std::io::Result<WorldPaintReplayReport> {
    let path_ref = path.as_ref();
    let doc = load_world_paint_delta_document(path_ref)?;
    let mut report = replay_world_paint_delta_document_onto_world(world, &doc, scene_filter);
    report.source_path = Some(path_ref.display().to_string());
    Ok(report)
}
