//! LDtk interchange and conversion support for Ember Editor.
//!
//! This crate parses the public LDtk JSON format and converts it into
//! Ember-owned editable scene documents. It does not embed the LDtk editor.

use ember_core::StableId;
use ember_scene::{
    CollisionSemantic, ComponentInstance, EntityInstance, EntityTransform, SceneDocument,
    SceneLayer, SceneLayerKind, SemanticBinding, SemanticCell, TileCell, TileTransform,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkProject {
    #[serde(rename = "jsonVersion")]
    pub json_version: String,
    #[serde(default)]
    pub defs: LdtkDefinitions,
    #[serde(default)]
    pub worlds: Vec<LdtkWorld>,
    #[serde(default)]
    pub levels: Vec<LdtkLevel>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct LdtkDefinitions {
    #[serde(default)]
    pub layers: Vec<LdtkLayerDefinition>,
    #[serde(default)]
    pub entities: Vec<LdtkEntityDefinition>,
    #[serde(default)]
    pub tilesets: Vec<LdtkTilesetDefinition>,
    #[serde(default, rename = "enums")]
    pub enum_definitions: Vec<Value>,
    #[serde(default, rename = "externalEnums")]
    pub external_enum_definitions: Vec<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkWorld {
    pub identifier: String,
    pub iid: String,
    #[serde(default)]
    pub levels: Vec<LdtkLevel>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkLevel {
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "worldX", default)]
    pub world_x: i32,
    #[serde(rename = "worldY", default)]
    pub world_y: i32,
    #[serde(rename = "pxWid", default)]
    pub pixel_width: i32,
    #[serde(rename = "pxHei", default)]
    pub pixel_height: i32,
    #[serde(rename = "layerInstances", default)]
    pub layer_instances: Option<Vec<LdtkLayerInstance>>,
    #[serde(rename = "externalRelPath", default)]
    pub external_relative_path: Option<String>,
    #[serde(rename = "fieldInstances", default)]
    pub fields: Vec<LdtkFieldInstance>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkLayerDefinition {
    pub identifier: String,
    pub uid: i64,
    #[serde(rename = "type")]
    pub layer_type: String,
    #[serde(rename = "gridSize", default)]
    pub grid_size: i32,
    #[serde(rename = "autoRuleGroups", default)]
    pub auto_rule_groups: Vec<Value>,
    #[serde(rename = "intGridValues", default)]
    pub int_grid_values: Vec<LdtkIntGridValueDefinition>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkIntGridValueDefinition {
    pub value: i32,
    pub identifier: Option<String>,
    #[serde(default)]
    pub color: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkEntityDefinition {
    pub identifier: String,
    pub uid: i64,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(rename = "fieldDefs", default)]
    pub field_definitions: Vec<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkTilesetDefinition {
    pub identifier: String,
    pub uid: i64,
    #[serde(rename = "relPath", default)]
    pub relative_path: Option<String>,
    #[serde(rename = "tileGridSize", default)]
    pub tile_grid_size: i32,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkLayerInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__type")]
    pub layer_type: String,
    #[serde(rename = "__gridSize", default)]
    pub grid_size: i32,
    #[serde(rename = "__cWid", default)]
    pub cell_width: i32,
    #[serde(rename = "__cHei", default)]
    pub cell_height: i32,
    #[serde(rename = "__tilesetDefUid", default)]
    pub tileset_definition_uid: Option<i64>,
    #[serde(rename = "intGridCsv", default)]
    pub int_grid_csv: Vec<i32>,
    #[serde(rename = "gridTiles", default)]
    pub grid_tiles: Vec<LdtkTileInstance>,
    #[serde(rename = "autoLayerTiles", default)]
    pub auto_layer_tiles: Vec<LdtkTileInstance>,
    #[serde(rename = "entityInstances", default)]
    pub entity_instances: Vec<LdtkEntityInstance>,
    #[serde(rename = "pxTotalOffsetX", default)]
    pub pixel_offset_x: i32,
    #[serde(rename = "pxTotalOffsetY", default)]
    pub pixel_offset_y: i32,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkTileInstance {
    #[serde(rename = "px")]
    pub pixel: [i32; 2],
    #[serde(rename = "src")]
    pub source: [i32; 2],
    #[serde(rename = "t")]
    pub tile_id: i32,
    #[serde(rename = "f", default)]
    pub flip_bits: i32,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkEntityInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    pub iid: String,
    #[serde(rename = "px")]
    pub pixel: [i32; 2],
    #[serde(rename = "__grid")]
    pub grid: [i32; 2],
    #[serde(rename = "width", default)]
    pub width: i32,
    #[serde(rename = "height", default)]
    pub height: i32,
    #[serde(rename = "fieldInstances", default)]
    pub fields: Vec<LdtkFieldInstance>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LdtkFieldInstance {
    #[serde(rename = "__identifier")]
    pub identifier: String,
    #[serde(rename = "__type")]
    pub field_type: String,
    #[serde(rename = "__value")]
    pub value: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct LdtkImportProfile {
    pub int_grid: Vec<IntGridSemanticRule>,
    pub entity_components: BTreeMap<String, Vec<EntityComponentRule>>,
    pub entity_tags: BTreeMap<String, Vec<StableId>>,
    pub layer_kinds: BTreeMap<String, SceneLayerKind>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IntGridSemanticRule {
    pub layer: String,
    pub value: i32,
    pub semantic: SemanticBinding,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EntityComponentRule {
    pub component_type: StableId,
    pub field_map: BTreeMap<String, String>,
    pub fixed_properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ImportedEmberProject {
    pub source_json_version: String,
    pub scenes: Vec<SceneDocument>,
    pub external_levels: Vec<ExternalLevelReference>,
    pub diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalLevelReference {
    pub level_identifier: String,
    pub level_iid: String,
    pub relative_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportDiagnostic {
    pub severity: ImportSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug)]
pub enum LdtkImportError {
    Json(serde_json::Error),
    UnsupportedVersion(String),
    InvalidIdentifier(String),
}

impl From<serde_json::Error> for LdtkImportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn parse_project(input: &str) -> Result<LdtkProject, LdtkImportError> {
    let project: LdtkProject = serde_json::from_str(input)?;
    if project.json_version.trim().is_empty() {
        return Err(LdtkImportError::UnsupportedVersion(
            "missing jsonVersion".into(),
        ));
    }
    Ok(project)
}

pub fn level_count(project: &LdtkProject) -> usize {
    project.levels.len()
        + project
            .worlds
            .iter()
            .map(|world| world.levels.len())
            .sum::<usize>()
}

pub fn convert_project(
    project: &LdtkProject,
    profile: &LdtkImportProfile,
) -> Result<ImportedEmberProject, LdtkImportError> {
    let mut scenes = Vec::new();
    let mut external_levels = Vec::new();
    let mut diagnostics = Vec::new();

    for level in project
        .levels
        .iter()
        .chain(project.worlds.iter().flat_map(|world| world.levels.iter()))
    {
        if let Some(path) = &level.external_relative_path {
            external_levels.push(ExternalLevelReference {
                level_identifier: level.identifier.clone(),
                level_iid: level.iid.clone(),
                relative_path: path.clone(),
            });
            diagnostics.push(ImportDiagnostic {
                severity: ImportSeverity::Info,
                code: "EMBER-LDTK-EXT-001".into(),
                message: format!(
                    "level {} is external and requires a second-stage document load",
                    level.identifier
                ),
            });
            continue;
        }
        scenes.push(convert_level(project, level, profile, &mut diagnostics)?);
    }

    Ok(ImportedEmberProject {
        source_json_version: project.json_version.clone(),
        scenes,
        external_levels,
        diagnostics,
    })
}

fn convert_level(
    project: &LdtkProject,
    level: &LdtkLevel,
    profile: &LdtkImportProfile,
    diagnostics: &mut Vec<ImportDiagnostic>,
) -> Result<SceneDocument, LdtkImportError> {
    let scene_id = stable("scene", &level.iid)?;
    let mut scene = SceneDocument {
        id: scene_id,
        name: level.identifier.clone(),
        width_pixels: level.pixel_width,
        height_pixels: level.pixel_height,
        world_origin: [level.world_x, level.world_y, 0],
        layers: Vec::new(),
        entities: Vec::new(),
        properties: field_properties(&level.fields),
    };

    let Some(layer_instances) = &level.layer_instances else {
        diagnostics.push(ImportDiagnostic {
            severity: ImportSeverity::Warning,
            code: "EMBER-LDTK-LAYER-001".into(),
            message: format!("level {} has no embedded layer instances", level.identifier),
        });
        return Ok(scene);
    };

    for (z_index, layer) in layer_instances.iter().enumerate() {
        let definition = project
            .defs
            .layers
            .iter()
            .find(|candidate| candidate.identifier == layer.identifier);
        let mut converted = convert_layer(layer, definition, profile, z_index as i32)?;
        scene.entities.extend(convert_entities(layer, profile)?);
        scene.layers.append(&mut converted);
    }

    Ok(scene)
}

fn convert_layer(
    layer: &LdtkLayerInstance,
    definition: Option<&LdtkLayerDefinition>,
    profile: &LdtkImportProfile,
    z_index: i32,
) -> Result<Vec<SceneLayer>, LdtkImportError> {
    let layer_id = stable("layer", &layer.identifier)?;
    let kind = profile
        .layer_kinds
        .get(&layer.identifier)
        .cloned()
        .unwrap_or_else(|| default_layer_kind(&layer.layer_type));
    let tileset = layer
        .tileset_definition_uid
        .map(|uid| stable("ldtk_tileset", &uid.to_string()))
        .transpose()?;

    let mut tile_cells = Vec::new();
    for tile in layer.grid_tiles.iter().chain(layer.auto_layer_tiles.iter()) {
        tile_cells.push(TileCell {
            grid: [
                (tile.pixel[0] + layer.pixel_offset_x) / layer.grid_size.max(1),
                (tile.pixel[1] + layer.pixel_offset_y) / layer.grid_size.max(1),
                0,
            ],
            source_pixels: tile.source,
            tile_id: tile.tile_id,
            transform: decode_flip_bits(tile.flip_bits),
            tileset: tileset.clone(),
        });
    }

    let mut semantic_cells = Vec::new();
    if !layer.int_grid_csv.is_empty() && layer.cell_width > 0 {
        for (index, value) in layer.int_grid_csv.iter().copied().enumerate() {
            if value == 0 {
                continue;
            }
            let x = index as i32 % layer.cell_width;
            let y = index as i32 / layer.cell_width;
            let semantic = profile
                .int_grid
                .iter()
                .find(|rule| rule.layer == layer.identifier && rule.value == value)
                .map(|rule| rule.semantic.clone())
                .unwrap_or_else(|| fallback_semantic(definition, value));
            semantic_cells.push(SemanticCell {
                grid: [x, y, 0],
                value,
                semantic,
            });
        }
    }

    Ok(vec![SceneLayer {
        id: layer_id,
        name: layer.identifier.clone(),
        kind,
        grid_size: layer.grid_size.max(1),
        visible: true,
        opacity: 1.0,
        z_index,
        tile_cells,
        semantic_cells,
        properties: layer.extra.clone(),
    }])
}

fn convert_entities(
    layer: &LdtkLayerInstance,
    profile: &LdtkImportProfile,
) -> Result<Vec<EntityInstance>, LdtkImportError> {
    layer
        .entity_instances
        .iter()
        .map(|entity| {
            let definition = stable("entity_definition", &entity.identifier)?;
            let id = stable("entity", &entity.iid)?;
            let mut components = Vec::new();
            if let Some(rules) = profile.entity_components.get(&entity.identifier) {
                for rule in rules {
                    let mut properties = rule.fixed_properties.clone();
                    for field in &entity.fields {
                        if let Some(target_name) = rule.field_map.get(&field.identifier) {
                            properties.insert(target_name.clone(), field.value.clone());
                        }
                    }
                    components.push(ComponentInstance {
                        component_type: rule.component_type.clone(),
                        enabled: true,
                        properties,
                    });
                }
            }
            if components.is_empty() {
                components.push(ComponentInstance {
                    component_type: stable("component", "ldtk_fields")?,
                    enabled: true,
                    properties: field_properties(&entity.fields),
                });
            }
            Ok(EntityInstance {
                id,
                definition,
                name: entity.identifier.clone(),
                transform: EntityTransform {
                    position: [entity.pixel[0] as f32, entity.pixel[1] as f32, 0.0],
                    rotation_degrees: 0.0,
                    scale: [1.0, 1.0],
                },
                components,
                behavior_graphs: Vec::new(),
                tags: profile
                    .entity_tags
                    .get(&entity.identifier)
                    .cloned()
                    .unwrap_or_default(),
                properties: entity.extra.clone(),
            })
        })
        .collect()
}

fn field_properties(fields: &[LdtkFieldInstance]) -> BTreeMap<String, Value> {
    fields
        .iter()
        .map(|field| (field.identifier.clone(), field.value.clone()))
        .collect()
}

fn fallback_semantic(definition: Option<&LdtkLayerDefinition>, value: i32) -> SemanticBinding {
    let identifier = definition
        .and_then(|definition| {
            definition
                .int_grid_values
                .iter()
                .find(|candidate| candidate.value == value)
                .and_then(|candidate| candidate.identifier.clone())
        })
        .unwrap_or_else(|| format!("value_{value}"));
    let material = stable("terrain_material", &identifier).ok();
    SemanticBinding {
        material,
        collision: None,
        navigation_cost: None,
        tags: Vec::new(),
        behavior_graphs: Vec::new(),
        properties: BTreeMap::new(),
    }
}

fn default_layer_kind(value: &str) -> SceneLayerKind {
    match value {
        "Tiles" => SceneLayerKind::Tiles,
        "AutoLayer" => SceneLayerKind::AutoTiles,
        "IntGrid" => SceneLayerKind::SemanticGrid,
        "Entities" => SceneLayerKind::Entities,
        other => SceneLayerKind::Custom(other.to_string()),
    }
}

fn decode_flip_bits(bits: i32) -> TileTransform {
    TileTransform {
        flip_x: bits & 1 != 0,
        flip_y: bits & 2 != 0,
        transpose: false,
    }
}

fn stable(namespace: &str, value: &str) -> Result<StableId, LdtkImportError> {
    StableId::new(namespace, value)
        .map_err(|_| LdtkImportError::InvalidIdentifier(format!("{namespace}:{value}")))
}

pub fn collision_binding(material: StableId) -> SemanticBinding {
    SemanticBinding {
        material: Some(material),
        collision: Some(CollisionSemantic::Solid),
        navigation_cost: None,
        tags: Vec::new(),
        behavior_graphs: Vec::new(),
        properties: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_project() {
        let source = r#"{"jsonVersion":"1.5.3","defs":{},"worlds":[],"levels":[]}"#;
        let project = parse_project(source).expect("minimal project parses");
        assert_eq!(project.json_version, "1.5.3");
        assert_eq!(level_count(&project), 0);
    }

    #[test]
    fn converts_embedded_level() {
        let source = r#"{
          "jsonVersion":"1.5.3",
          "defs":{"layers":[],"entities":[],"tilesets":[]},
          "worlds":[],
          "levels":[{
            "identifier":"StageOne","iid":"stage-one","worldX":64,"worldY":32,
            "pxWid":320,"pxHei":180,
            "layerInstances":[]
          }]
        }"#;
        let project = parse_project(source).unwrap();
        let imported = convert_project(&project, &LdtkImportProfile::default()).unwrap();
        assert_eq!(imported.scenes.len(), 1);
        assert_eq!(imported.scenes[0].world_origin, [64, 32, 0]);
    }
}
