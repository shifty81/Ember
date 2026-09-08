pub mod live_autotile;
pub mod neighbor_mask;
mod shore_water_lifecycle;
pub mod shoreline_resolver;
pub mod terrain_debug;
pub mod terrain_family;
pub mod transition_atlas;
pub mod transition_atlas_groups;
pub mod transition_pair_registry;
pub mod transition_resolver;
pub mod transition_rule_catalog;
pub mod transition_rule_draft;
pub mod transition_rule_inspector;
pub mod transition_rule_preview;
pub mod transition_rules_manifest;

pub use live_autotile::{
    adjacency_mask, normalize_mask, resolve_autotile_cell, AutotileShape, AutotileSource,
    AutotileSyncReport, LiveAutotileCache, ResolvedAutotileCell,
};
pub use neighbor_mask::{
    family_at, family_neighbors, CardinalDirection, DiagonalDirection, FamilyNeighbors, E, N, NE,
    NW, S, SE, SW, W,
};
pub use shoreline_resolver::{
    analyze_shoreline_cell, apply_coastline_tile_pass, apply_coastline_tile_pass_with_profile,
    normalize_shore_water_lifecycle_region, CoastlineCleanupReport, CoastlineGenerationProfile,
    ShoreWaterLifecycleReport, ShorelineCell,
};
pub use terrain_debug::{mask_code, terrain_debug_cell, TerrainDebugCell};
pub use terrain_family::{terrain_family, tile_is_land, tile_is_water, TerrainFamily};
pub use transition_atlas::{
    resolve_transition_atlas_requests, resolve_transition_inner_corner_requests,
    transition_material_atlas_group, transition_pair_atlas_group, TerrainTransitionAtlasRequest,
    TerrainTransitionInnerCornerRequest,
};
pub use transition_pair_registry::{
    transition_pair_policy, TerrainTransitionPairPolicy, TransitionCollisionPolicy,
    TransitionOwnership,
};
pub use transition_resolver::{
    corner_material, edge_material, resolve_terrain_transitions,
    resolve_terrain_transitions_from_neighbors, ResolvedTerrainTransitions,
    TerrainCornerTransition, TerrainEdgeTransition, TerrainOuterCornerTransition,
    TransitionMaterial,
};
pub use transition_rule_inspector::{
    inspect_terrain_transition_rules, TerrainTransitionRuleHit, TerrainTransitionRuleInspection,
};
pub use transition_rules_manifest::{
    reload_terrain_transition_rule_manifest, terrain_transition_rule_manifest,
    terrain_transition_rule_manifest_cache_summary, transition_rule_atlas_group_for_material,
    transition_rule_material, TerrainFamilySelector, TerrainTransitionRule,
    TerrainTransitionRuleManifest, TerrainTransitionRuleManifestReloadReport, TransitionRulePhase,
    TERRAIN_TRANSITION_RULE_MANIFEST_PATH,
};

pub use transition_rule_preview::{
    preview_transition_rule, preview_transition_rules, TerrainTransitionRulePreview,
    TransitionPreviewCell, TransitionPreviewCellRole, TransitionPreviewSample,
    TransitionPreviewSampleKind,
};

pub use transition_rule_catalog::{
    transition_rule_catalog_rows, transition_rule_catalog_summary, TransitionRuleCatalogFilter,
    TransitionRuleCatalogRow,
};

pub use transition_rule_draft::{
    apply_transition_rule_draft_auto_fixes, apply_transition_rule_draft_edit,
    compare_transition_rule_draft_to_live, export_transition_rule_draft_from_manifest,
    promote_transition_rule_draft_to_manifest, reset_transition_rule_draft_from_live_manifest,
    restore_transition_rule_draft_from_undo, transition_rule_draft_auto_fix_lines,
    transition_rule_draft_auto_fix_status, transition_rule_draft_compare_lines,
    transition_rule_draft_compare_status, transition_rule_draft_selected_summary,
    transition_rule_draft_selected_validation_lines, transition_rule_draft_status,
    transition_rule_draft_undo_lines, transition_rule_draft_undo_status,
    transition_rule_draft_validation_lines, transition_rule_draft_validation_status,
    validate_transition_rule_draft_for_editor, TransitionRuleDraftAutoFixEntry,
    TransitionRuleDraftAutoFixReport, TransitionRuleDraftCompareReport,
    TransitionRuleDraftDiagnostic, TransitionRuleDraftDiagnosticSeverity,
    TransitionRuleDraftDiffEntry, TransitionRuleDraftDiffKind, TransitionRuleDraftEdit,
    TransitionRuleDraftPromotionReport, TransitionRuleDraftReport,
    TransitionRuleDraftRestoreReport, TransitionRuleDraftValidationReport,
    TERRAIN_TRANSITION_RULE_DRAFT_PATH, TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH,
    TERRAIN_TRANSITION_RULE_LIVE_BACKUP_PATH,
};

pub mod terrain_pattern_v2;

pub mod terrain_pattern_trace_v2;
