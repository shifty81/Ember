use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const DEFAULT_UNIVERSAL_LPC_EQUIPMENT_CATALOG_PATH: &str =
    "content/assets/lpc/universal_lpc_equipment_action_catalog_v0_1.json";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalLpcEquipmentCatalog {
    pub schema: String,
    pub source_commit: String,
    pub source_mount: String,
    pub equipment_count: usize,
    pub tool_definition_count: usize,
    pub weapon_definition_count: usize,
    pub shield_definition_count: usize,
    #[serde(default)]
    pub equipment: Vec<UniversalLpcEquipmentDefinition>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalLpcEquipmentDefinition {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub subcategory: String,
    pub definition_path: String,
    pub type_name: String,
    #[serde(default)]
    pub animations: Vec<String>,
    pub gameplay_action: String,
    #[serde(default)]
    pub layers: Vec<UniversalLpcEquipmentLayer>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    pub creator_allowed: bool,
    #[serde(default)]
    pub gameplay_acquisition: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniversalLpcEquipmentLayer {
    pub layer: String,
    pub z_pos: Option<i32>,
    pub custom_animation: Option<String>,
    #[serde(default)]
    pub sources: HashMap<String, String>,
}

impl UniversalLpcEquipmentCatalog {
    pub fn load_default() -> Result<Self, String> {
        Self::load_from_path(DEFAULT_UNIVERSAL_LPC_EQUIPMENT_CATALOG_PATH)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let catalog: Self = serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "havenwild.universal_lpc_equipment_action_catalog.v0_1" {
            return Err(format!(
                "unsupported Universal LPC equipment schema: {}",
                self.schema
            ));
        }
        if self.source_mount != "assets/source/licensed/universal_lpc_generator" {
            return Err(
                "Universal LPC equipment catalog must use the complete source mount".into(),
            );
        }
        if self.equipment_count != self.equipment.len() {
            return Err(format!(
                "Universal LPC equipment count mismatch: declared {}, loaded {}",
                self.equipment_count,
                self.equipment.len()
            ));
        }
        if self.tool_definition_count + self.weapon_definition_count != self.equipment_count {
            return Err("Universal LPC tool/weapon counts do not cover the catalog".into());
        }
        if self.shield_definition_count > self.weapon_definition_count {
            return Err("Universal LPC shield count cannot exceed weapon definitions".into());
        }
        for item in &self.equipment {
            item.validate()?;
        }
        Ok(())
    }

    pub fn find(&self, id: &str) -> Option<&UniversalLpcEquipmentDefinition> {
        self.equipment.iter().find(|item| item.id == id)
    }

    pub fn by_gameplay_action<'a>(
        &'a self,
        action: &'a str,
    ) -> impl Iterator<Item = &'a UniversalLpcEquipmentDefinition> + 'a {
        self.equipment
            .iter()
            .filter(move |item| item.gameplay_action.eq_ignore_ascii_case(action))
    }
}

impl UniversalLpcEquipmentDefinition {
    fn validate(&self) -> Result<(), String> {
        if self.creator_allowed {
            return Err(format!(
                "Universal LPC gameplay equipment cannot enter character creation: {}",
                self.id
            ));
        }
        if !self.definition_path.starts_with("sheet_definitions/tools/")
            && !self
                .definition_path
                .starts_with("sheet_definitions/weapons/")
        {
            return Err(format!(
                "Universal LPC equipment definition is outside the character repository equipment roots: {}",
                self.definition_path
            ));
        }
        if self.layers.is_empty() {
            return Err(format!(
                "Universal LPC equipment has no layer definitions: {}",
                self.id
            ));
        }
        if self.animations.is_empty() {
            return Err(format!(
                "Universal LPC equipment has no animation contract: {}",
                self.id
            ));
        }
        if self.gameplay_acquisition.is_empty() {
            return Err(format!(
                "Universal LPC equipment has no gameplay acquisition route: {}",
                self.id
            ));
        }
        Ok(())
    }
}
