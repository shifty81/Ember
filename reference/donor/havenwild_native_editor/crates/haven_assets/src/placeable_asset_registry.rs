use crate::{
    asset_pack::{AssetCategory, StableAssetRef},
    runtime_asset_cache::RuntimeAssetSession,
};
use haven_core::{ObjectFootprint, ObjectKind, StablePlaceableAssetRef};
use serde::Deserialize;
pub use crate::published_world_asset_metadata::{
    PublishedStructureDefinition, PublishedStructureSocket, PublishedWorldAssetProvenance,
    PublishedWorldAssetSeasonalSource, PublishedWorldAssetSourceLayer,
};
use std::{collections::BTreeMap, fs::read_to_string, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceableVisualFrame {
    pub state: String,
    pub source_rect: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceableVisualDefinition {
    pub foot_anchor: [f32; 2],
    pub frames: Vec<PlaceableVisualFrame>,
}

impl PlaceableVisualDefinition {
    pub fn frame_for_state(&self, state: Option<&str>) -> Option<&PlaceableVisualFrame> {
        state
            .and_then(|wanted| self.frames.iter().find(|frame| frame.state == wanted))
            .or_else(|| self.frames.first())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceableInteractionAction {
    #[default]
    Inspect,
    Sit,
    Sleep,
    Harvest,
    Open,
    EnterScene,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceableMutationAuthority {
    #[default]
    Host,
    EditorPreview,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceableMutationContext {
    pub is_authoritative_host: bool,
    pub editor_preview: bool,
}

impl PlaceableMutationAuthority {
    pub fn allows(self, context: PlaceableMutationContext) -> bool {
        match self {
            Self::Host => context.is_authoritative_host,
            Self::EditorPreview => context.editor_preview,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceableStateTransition {
    pub from: String,
    pub trigger: String,
    pub to: String,
    pub authority: PlaceableMutationAuthority,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceableBehaviorBinding {
    pub node_id: Option<String>,
    pub interaction_trigger: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceableStateGeometry {
    pub state: String,
    pub footprint: ObjectFootprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceableAttachmentPoint {
    pub id: String,
    pub state: Option<String>,
    pub offset: [i32; 2],
    pub reservation: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceableInteractionDefinition {
    pub action: PlaceableInteractionAction,
    pub message: String,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishedWorldAssetRole {
    #[default]
    Placeable,
    StructureComponent,
    StructuralConnector,
    BuildingRecipe,
    TerrainDetail,
    Stamp,
    LegacyAdapter,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishedWorldAssetCertification {
    Certified,
    #[default]
    Candidate,
    Provisional,
    Placeholder,
    Missing,
    LegacyAlias,
    Rejected,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublishedWorldAssetDefinition {
    pub stable_id: String,
    pub entry_id: String,
    pub semantic_id: String,
    pub label: String,
    pub category: AssetCategory,
    pub catalog_ref: StableAssetRef,
    pub source_ref: Option<StableAssetRef>,
    pub source_path: Option<PathBuf>,
    pub aliases: Vec<String>,
    pub role: PublishedWorldAssetRole,
    pub certification: PublishedWorldAssetCertification,
    pub provenance: PublishedWorldAssetProvenance,
    pub structure: Option<PublishedStructureDefinition>,
    pub footprint: ObjectFootprint,
    pub anchor: [i32; 2],
    pub allowed_surfaces: Vec<String>,
    pub forbidden_surfaces: Vec<String>,
    pub placement_tags: Vec<String>,
    pub states: Vec<String>,
    pub visual: Option<PlaceableVisualDefinition>,
    pub interaction: PlaceableInteractionDefinition,
    pub transitions: Vec<PlaceableStateTransition>,
    pub behavior: PlaceableBehaviorBinding,
    pub state_geometry: Vec<PlaceableStateGeometry>,
    pub attachment_points: Vec<PlaceableAttachmentPoint>,
    pub legacy_object_kind: Option<ObjectKind>,
    pub legacy_object_kind_primary: bool,
}

impl PublishedWorldAssetDefinition {
    pub fn pack_qualified_id(&self) -> &str {
        &self.stable_id
    }

    pub fn supports_surface(&self, surface: &str) -> bool {
        !self.forbidden_surfaces.iter().any(|item| item == surface)
            && (self.allowed_surfaces.is_empty()
                || self.allowed_surfaces.iter().any(|item| item == surface))
    }

    pub fn persistent_ref(&self) -> StablePlaceableAssetRef {
        StablePlaceableAssetRef::new(
            self.catalog_ref.pack_id.0.clone(),
            format!("{:?}", self.category).to_ascii_lowercase(),
            self.entry_id.clone(),
            self.source_ref
                .as_ref()
                .map(|value| value.source_id.0.clone())
                .unwrap_or_else(|| self.catalog_ref.source_id.0.clone()),
            self.catalog_ref.variant_id.clone(),
        )
    }

    pub fn compatibility_kind(&self) -> ObjectKind {
        self.legacy_object_kind.unwrap_or(ObjectKind::Crate)
    }

    pub fn initial_state(&self) -> Option<&str> {
        self.states.first().map(String::as_str)
    }

    pub fn transition_for(
        &self,
        current: Option<&str>,
        trigger: &str,
    ) -> Option<&PlaceableStateTransition> {
        let current = current.or_else(|| self.initial_state())?;
        self.transitions
            .iter()
            .find(|transition| transition.from == current && transition.trigger == trigger)
    }

    pub fn supports_state(&self, state: &str) -> bool {
        self.states.iter().any(|candidate| candidate == state)
    }

    pub fn footprint_for_state(&self, state: Option<&str>) -> ObjectFootprint {
        state
            .and_then(|wanted| {
                self.state_geometry
                    .iter()
                    .find(|entry| entry.state == wanted)
            })
            .map(|entry| entry.footprint)
            .unwrap_or(self.footprint)
    }

    pub fn attachment_points_for_state<'a>(
        &'a self,
        state: Option<&'a str>,
    ) -> impl Iterator<Item = &'a PlaceableAttachmentPoint> + 'a {
        self.attachment_points.iter().filter(move |point| {
            point.state.as_deref().is_none() || point.state.as_deref() == state
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishedWorldAssetRegistry {
    entries: Vec<PublishedWorldAssetDefinition>,
    by_legacy_object: BTreeMap<String, usize>,
    by_persistent_key: BTreeMap<String, usize>,
    by_alias: BTreeMap<String, usize>,
}
/// Compatibility names retained while callers migrate to the W42 published-world-asset vocabulary.
/// Both aliases refer to the same registry/storage; there is no parallel placeable registry.
pub type PlaceableAssetDefinition = PublishedWorldAssetDefinition;
pub type PlaceableAssetRegistry = PublishedWorldAssetRegistry;
impl PublishedWorldAssetRegistry {
    pub fn load_discovered(session: &RuntimeAssetSession) -> Result<Self, String> {
        let mut entries = BTreeMap::<String, PublishedWorldAssetDefinition>::new();
        let mut failures = Vec::new();

        for pack in session.registry.mounted_packs() {
            for asset in pack.assets.iter().filter(|asset| {
                asset.category == AssetCategory::EditorTemplate
                    && (asset.semantic_id.starts_with("placeable.catalog.")
                        || asset.semantic_id.starts_with("world_asset.catalog."))
            }) {
                let catalog_ref = StableAssetRef {
                    pack_id: pack.id.clone(),
                    category: asset.category.clone(),
                    asset_id: asset.id.clone(),
                    source_id: asset.source_id.clone(),
                    variant_id: None,
                };
                let Some(source) = session.sources.get(&catalog_ref) else {
                    failures.push(format!(
                        "missing source for placeable catalog {}",
                        asset.semantic_id
                    ));
                    continue;
                };
                let text = match read_to_string(&source.source_path) {
                    Ok(text) => text,
                    Err(error) => {
                        failures.push(format!("{}: {error}", source.source_path.display()));
                        continue;
                    }
                };
                let catalog: PlaceableCatalogFile = match serde_json::from_str(&text) {
                    Ok(catalog) => catalog,
                    Err(error) => {
                        failures.push(format!("{}: {error}", source.source_path.display()));
                        continue;
                    }
                };
                if !matches!(
                    catalog.schema.as_str(),
                    "havenwild.placeable_catalog.v1" | "havenwild.published_world_asset_catalog.v1"
                ) {
                    failures.push(format!(
                        "{} uses unsupported schema {}",
                        source.source_path.display(),
                        catalog.schema
                    ));
                    continue;
                }
                for item in catalog.entries {
                    match normalize_entry(session, &catalog_ref, item) {
                        Ok(definition) => {
                            if entries
                                .insert(definition.stable_id.clone(), definition)
                                .is_some()
                            {
                                failures.push(
                                    "duplicate stable placeable identity discovered".to_string(),
                                );
                            }
                        }
                        Err(error) => failures.push(error),
                    }
                }
            }
        }

        if entries.is_empty() && !failures.is_empty() {
            return Err(failures.join("; "));
        }
        let entries = entries.into_values().collect::<Vec<_>>();
        let mut by_legacy_object = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate() {
            if entry.legacy_object_kind_primary {
                if let Some(kind) = entry.legacy_object_kind {
                    if by_legacy_object
                        .insert(kind.code().to_string(), index)
                        .is_some()
                    {
                        return Err(format!(
                            "duplicate primary legacy ObjectKind adapter {}",
                            kind.code()
                        ));
                    }
                }
            }
        }
        // v1 catalog compatibility: if no explicit primary exists for a kind,
        // retain the first discovered mapping rather than dropping old saves.
        for (index, entry) in entries.iter().enumerate() {
            if let Some(kind) = entry.legacy_object_kind {
                by_legacy_object.entry(kind.code().to_string()).or_insert(index);
            }
        }
        let by_persistent_key = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.persistent_ref().stable_key(), index))
            .collect();
        let mut by_alias = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate() {
            for alias in entry
                .aliases
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(entry.entry_id.as_str()))
                .chain(std::iter::once(entry.semantic_id.as_str()))
            {
                if by_alias.insert(alias.to_string(), index).is_some() {
                    return Err(format!("duplicate published world asset alias {alias}"));
                }
            }
        }
        Ok(Self {
            entries,
            by_legacy_object,
            by_persistent_key,
            by_alias,
        })
    }

    pub fn entries(&self) -> &[PublishedWorldAssetDefinition] {
        &self.entries
    }

    pub fn entry(&self, stable_id: &str) -> Option<&PublishedWorldAssetDefinition> {
        self.entries
            .iter()
            .find(|entry| entry.stable_id == stable_id)
    }

    pub fn resolve_persistent_ref(
        &self,
        asset_ref: &StablePlaceableAssetRef,
    ) -> Option<&PublishedWorldAssetDefinition> {
        if let Some(alias) = asset_ref.scene_asset_alias_id() {
            return self.resolve_alias(alias);
        }
        self.by_persistent_key
            .get(&asset_ref.stable_key())
            .and_then(|index| self.entries.get(*index))
    }
    pub fn resolve_alias(&self, alias: &str) -> Option<&PublishedWorldAssetDefinition> {
        self.by_alias
            .get(alias)
            .and_then(|index| self.entries.get(*index))
    }

    pub fn canonical_ref_for_alias(&self, alias: &str) -> Option<StablePlaceableAssetRef> {
        self.resolve_alias(alias).map(PublishedWorldAssetDefinition::persistent_ref)
    }

    pub fn canonicalize_scene_aliases(&self, map: &mut haven_core::TavernMap) -> usize {
        let replacements = map
            .object_asset_refs
            .iter()
            .filter_map(|(object_id, asset_ref)| {
                let alias = asset_ref.scene_asset_alias_id()?;
                self.canonical_ref_for_alias(alias)
                    .map(|canonical| (*object_id, canonical))
            })
            .collect::<Vec<_>>();
        let count = replacements.len();
        for (object_id, canonical) in replacements {
            map.object_asset_refs.insert(object_id, canonical);
        }
        count
    }

    pub fn canonicalize_world_aliases(&self, world: &mut haven_core::GameWorld) -> usize {
        world
            .scenes
            .iter_mut()
            .map(|scene| self.canonicalize_scene_aliases(&mut scene.map))
            .sum()
    }

    pub fn for_legacy_object(&self, object: ObjectKind) -> Option<&PublishedWorldAssetDefinition> {
        self.by_legacy_object
            .get(object.code())
            .and_then(|index| self.entries.get(*index))
    }

    pub fn footprint_for_legacy_object(&self, object: ObjectKind) -> ObjectFootprint {
        self.for_legacy_object(object)
            .map(|entry| entry.footprint)
            .unwrap_or_else(|| object.default_footprint())
    }
}

#[derive(Debug, Deserialize)]
struct PlaceableCatalogFile {
    schema: String,
    entries: Vec<PlaceableCatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct PlaceableCatalogEntry {
    id: String,
    semantic_id: String,
    label: String,
    category: AssetCategory,
    #[serde(default)]
    source_semantic_id: Option<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    role: PublishedWorldAssetRole,
    #[serde(default)]
    certification: PublishedWorldAssetCertification,
    #[serde(default)]
    provenance: PublishedWorldAssetProvenance,
    #[serde(default)]
    structure: Option<PublishedStructureDefinition>,
    footprint: PlaceableFootprint,
    #[serde(default)]
    anchor: [i32; 2],
    #[serde(default)]
    allowed_surfaces: Vec<String>,
    #[serde(default)]
    forbidden_surfaces: Vec<String>,
    #[serde(default)]
    placement_tags: Vec<String>,
    #[serde(default)]
    states: Vec<String>,
    #[serde(default)]
    visual: Option<PlaceableVisualFile>,
    #[serde(default)]
    interaction: PlaceableInteractionFile,
    #[serde(default)]
    transitions: Vec<PlaceableStateTransitionFile>,
    #[serde(default)]
    behavior: PlaceableBehaviorFile,
    #[serde(default)]
    state_geometry: Vec<PlaceableStateGeometryFile>,
    #[serde(default)]
    attachment_points: Vec<PlaceableAttachmentPointFile>,
    #[serde(default)]
    legacy_object_kind: Option<String>,
    #[serde(default)]
    legacy_object_kind_primary: bool,
}

#[derive(Debug, Default, Deserialize)]
struct PlaceableStateTransitionFile {
    from: String,
    trigger: String,
    to: String,
    #[serde(default)]
    authority: PlaceableMutationAuthority,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct PlaceableStateGeometryFile {
    state: String,
    footprint: PlaceableFootprint,
}

#[derive(Debug, Deserialize)]
struct PlaceableAttachmentPointFile {
    id: String,
    #[serde(default)]
    state: Option<String>,
    offset: [i32; 2],
    #[serde(default)]
    reservation: String,
}

#[derive(Debug, Default, Deserialize)]
struct PlaceableBehaviorFile {
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default = "default_interaction_trigger")]
    interaction_trigger: String,
}

fn default_interaction_trigger() -> String {
    "interact".to_string()
}

#[derive(Debug, Default, Deserialize)]
struct PlaceableInteractionFile {
    #[serde(default)]
    action: PlaceableInteractionAction,
    #[serde(default)]
    message: String,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaceableVisualFile {
    #[serde(default)]
    foot_anchor: [f32; 2],
    #[serde(default)]
    frames: Vec<PlaceableVisualFrameFile>,
}

#[derive(Debug, Deserialize)]
struct PlaceableVisualFrameFile {
    state: String,
    source_rect: [f32; 4],
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct PlaceableFootprint {
    #[serde(default)]
    visual_offset: [i32; 2],
    visual_size: [i32; 2],
    #[serde(default)]
    collision_offset: [i32; 2],
    collision_size: [i32; 2],
    #[serde(default)]
    interaction_offset: [i32; 2],
    interaction_size: [i32; 2],
    #[serde(default = "default_true")]
    blocks_movement: bool,
    #[serde(default)]
    occludes_player: bool,
    #[serde(default)]
    fade_when_player_behind: bool,
}

fn default_true() -> bool {
    true
}

impl PlaceableFootprint {
    fn into_runtime(self) -> ObjectFootprint {
        ObjectFootprint {
            visual_offset_x: self.visual_offset[0],
            visual_offset_y: self.visual_offset[1],
            visual_w: self.visual_size[0].max(1),
            visual_h: self.visual_size[1].max(1),
            collision_offset_x: self.collision_offset[0],
            collision_offset_y: self.collision_offset[1],
            collision_w: self.collision_size[0].max(0),
            collision_h: self.collision_size[1].max(0),
            interaction_offset_x: self.interaction_offset[0],
            interaction_offset_y: self.interaction_offset[1],
            interaction_w: self.interaction_size[0].max(0),
            interaction_h: self.interaction_size[1].max(0),
            blocks_movement: self.blocks_movement,
            occludes_player: self.occludes_player,
            fade_when_player_behind: self.fade_when_player_behind,
        }
    }
}

fn normalize_entry(
    session: &RuntimeAssetSession,
    catalog_ref: &StableAssetRef,
    entry: PlaceableCatalogEntry,
) -> Result<PublishedWorldAssetDefinition, String> {
    let stable_id = format!("{}::{}", catalog_ref.pack_id.0, entry.id);
    let source = entry.source_semantic_id.as_deref().and_then(|semantic_id| {
        session.resolve_source(
            semantic_id,
            entry.category.clone(),
            crate::semantic_asset_resolution::AssetResolutionContext {
                explicit_pack: Some(catalog_ref.pack_id.clone()),
                allow_reference_only: false,
                ..Default::default()
            },
        )
    });
    let legacy_object_kind = entry
        .legacy_object_kind
        .as_deref()
        .and_then(object_kind_from_code);
    if entry.legacy_object_kind.is_some() && legacy_object_kind.is_none() {
        return Err(format!("{} has unknown legacy_object_kind", stable_id));
    }
    let visual = entry.visual.map(|visual| PlaceableVisualDefinition {
        foot_anchor: visual.foot_anchor,
        frames: visual
            .frames
            .into_iter()
            .map(|frame| PlaceableVisualFrame {
                state: frame.state,
                source_rect: frame.source_rect,
            })
            .collect(),
    });
    if visual
        .as_ref()
        .is_some_and(|visual| visual.frames.is_empty())
    {
        return Err(format!("{} visual contract has no frames", stable_id));
    }
    for geometry in &entry.state_geometry {
        if !entry.states.iter().any(|state| state == &geometry.state) {
            return Err(format!(
                "{} state geometry references unknown state {}",
                stable_id, geometry.state
            ));
        }
    }
    for point in &entry.attachment_points {
        if let Some(state) = point.state.as_deref() {
            if !entry.states.iter().any(|candidate| candidate == state) {
                return Err(format!(
                    "{} attachment {} references unknown state {}",
                    stable_id, point.id, state
                ));
            }
        }
    }
    for transition in &entry.transitions {
        if !entry.states.iter().any(|state| state == &transition.from) {
            return Err(format!(
                "{} transition references unknown from state {}",
                stable_id, transition.from
            ));
        }
        if !entry.states.iter().any(|state| state == &transition.to) {
            return Err(format!(
                "{} transition references unknown to state {}",
                stable_id, transition.to
            ));
        }
    }
    Ok(PublishedWorldAssetDefinition {
        stable_id,
        entry_id: entry.id,
        semantic_id: entry.semantic_id,
        label: entry.label,
        category: entry.category,
        catalog_ref: catalog_ref.clone(),
        source_ref: source.as_ref().map(|source| source.stable_ref.clone()),
        source_path: source.map(|source| source.source_path.clone()),
        aliases: entry.aliases,
        role: entry.role,
        certification: entry.certification,
        provenance: entry.provenance,
        structure: entry.structure,
        footprint: entry.footprint.into_runtime(),
        anchor: entry.anchor,
        allowed_surfaces: entry.allowed_surfaces,
        forbidden_surfaces: entry.forbidden_surfaces,
        placement_tags: entry.placement_tags,
        states: entry.states,
        visual,
        interaction: PlaceableInteractionDefinition {
            action: entry.interaction.action,
            message: entry.interaction.message,
            target: entry.interaction.target,
        },
        transitions: entry
            .transitions
            .into_iter()
            .map(|transition| PlaceableStateTransition {
                from: transition.from,
                trigger: transition.trigger,
                to: transition.to,
                authority: transition.authority,
                message: transition.message,
            })
            .collect(),
        behavior: PlaceableBehaviorBinding {
            node_id: entry.behavior.node_id,
            interaction_trigger: entry.behavior.interaction_trigger,
        },
        state_geometry: entry
            .state_geometry
            .into_iter()
            .map(|geometry| PlaceableStateGeometry {
                state: geometry.state,
                footprint: geometry.footprint.into_runtime(),
            })
            .collect(),
        attachment_points: entry
            .attachment_points
            .into_iter()
            .map(|point| PlaceableAttachmentPoint {
                id: point.id,
                state: point.state,
                offset: point.offset,
                reservation: point.reservation,
            })
            .collect(),
        legacy_object_kind,
        legacy_object_kind_primary: entry.legacy_object_kind_primary,
    })
}

fn object_kind_from_code(code: &str) -> Option<ObjectKind> {
    ObjectKind::from_code(code)
}
