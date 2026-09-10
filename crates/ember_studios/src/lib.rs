//! Shared typed document models for Ember GUI, Data, and Audio studios.

use ember_core::StableId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuiDocument {
    pub id: StableId,
    pub canvas_size: [u32; 2],
    pub root: StableId,
    pub widgets: BTreeMap<StableId, GuiWidget>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuiWidget {
    pub id: StableId,
    pub parent: Option<StableId>,
    pub kind: GuiWidgetKind,
    pub rect: [f32; 4],
    #[serde(default)]
    pub children: Vec<StableId>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GuiWidgetKind {
    Panel,
    Text,
    Image,
    Button,
    List,
    Input,
    Custom(StableId),
}

impl GuiDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.canvas_size.contains(&0) {
            return Err("GUI canvas dimensions must be non-zero".into());
        }
        if !self.widgets.contains_key(&self.root) {
            return Err("GUI root widget is missing".into());
        }
        for widget in self.widgets.values() {
            if widget.rect.iter().any(|value| !value.is_finite()) {
                return Err(format!("GUI widget {} has non-finite geometry", widget.id));
            }
            for child in &widget.children {
                let child_widget = self.widgets.get(child).ok_or_else(|| {
                    format!("GUI widget {} references missing child {child}", widget.id)
                })?;
                if child_widget.parent.as_ref() != Some(&widget.id) {
                    return Err(format!("GUI child {child} has inconsistent parent"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Number,
    Boolean,
    Text,
    StableId,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataValue {
    Integer(i64),
    Number(f64),
    Boolean(bool),
    Text(String),
    StableId(StableId),
}

impl DataValue {
    fn matches_type(&self, kind: &DataType) -> bool {
        matches!(
            (self, kind),
            (Self::Integer(_), DataType::Integer)
                | (Self::Number(_), DataType::Number)
                | (Self::Boolean(_), DataType::Boolean)
                | (Self::Text(_), DataType::Text)
                | (Self::StableId(_), DataType::StableId)
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataColumn {
    pub id: String,
    pub kind: DataType,
    pub required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataRow {
    pub id: StableId,
    pub values: BTreeMap<String, DataValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataTableDocument {
    pub id: StableId,
    pub columns: Vec<DataColumn>,
    pub rows: Vec<DataRow>,
}

impl DataTableDocument {
    pub fn validate(&self) -> Result<(), String> {
        let mut columns = BTreeMap::new();
        for column in &self.columns {
            if column.id.trim().is_empty() {
                return Err("data column id is empty".into());
            }
            if columns.insert(column.id.as_str(), column).is_some() {
                return Err(format!("duplicate data column {}", column.id));
            }
        }
        let mut row_ids = BTreeSet::new();
        for row in &self.rows {
            if !row_ids.insert(row.id.clone()) {
                return Err(format!("duplicate data row {}", row.id));
            }
            for column in &self.columns {
                match row.values.get(&column.id) {
                    Some(value) if value.matches_type(&column.kind) => {}
                    Some(_) => {
                        return Err(format!(
                            "row {} value {} has the wrong type",
                            row.id, column.id
                        ));
                    }
                    None if column.required => {
                        return Err(format!(
                            "row {} is missing required value {}",
                            row.id, column.id
                        ));
                    }
                    None => {}
                }
            }
            for key in row.values.keys() {
                if !columns.contains_key(key.as_str()) {
                    return Err(format!("row {} contains unknown column {key}", row.id));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioEventDocument {
    pub id: StableId,
    pub layers: Vec<AudioLayer>,
    #[serde(default)]
    pub parameters: BTreeMap<String, f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioLayer {
    pub source: StableId,
    pub gain_db: f32,
    pub pitch: f32,
    pub looped: bool,
    pub start_delay_ms: u32,
}

impl AudioEventDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.layers.is_empty() {
            return Err("audio event has no layers".into());
        }
        for layer in &self.layers {
            if !layer.gain_db.is_finite() || !layer.pitch.is_finite() || layer.pitch <= 0.0 {
                return Err(format!(
                    "audio layer {} has invalid gain/pitch",
                    layer.source
                ));
            }
        }
        if self.parameters.values().any(|value| !value.is_finite()) {
            return Err("audio event has a non-finite parameter".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_table_rejects_wrong_value_type() {
        let table = DataTableDocument {
            id: StableId::new("data_table", "items").unwrap(),
            columns: vec![DataColumn {
                id: "value".into(),
                kind: DataType::Integer,
                required: true,
            }],
            rows: vec![DataRow {
                id: StableId::new("row", "one").unwrap(),
                values: BTreeMap::from([("value".into(), DataValue::Text("bad".into()))]),
            }],
        };
        assert!(table.validate().is_err());
    }
}
