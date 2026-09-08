use crate::{TavernMap, MAX_STRUCTURAL_LEVEL};
use serde_json::Value;

pub(super) fn parse_structural_levels(
    layers: &Value,
    source_path: &str,
    source_w: usize,
    source_h: usize,
    offset_x: i32,
    offset_y: i32,
    map: &mut TavernMap,
) -> Result<(), String> {
    let Some(rows) = layers.get("structuralLevels").and_then(Value::as_array) else {
        return Ok(());
    };
    if rows.len() != source_h {
        return Err(format!(
            "{source_path}: structuralLevels layer has {} rows, expected {}",
            rows.len(), source_h
        ));
    }
    for (y, row_value) in rows.iter().enumerate() {
        let row = row_value
            .as_array()
            .ok_or_else(|| format!("{source_path}: structuralLevels row {y} is not an array"))?;
        if row.len() != source_w {
            return Err(format!(
                "{source_path}: structuralLevels row {y} is {} wide, expected {}",
                row.len(), source_w
            ));
        }
        for (x, cell) in row.iter().enumerate() {
            let level = if cell.is_null()
                || cell
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case("auto"))
            {
                None
            } else {
                let raw = cell.as_u64().ok_or_else(|| {
                    format!(
                        "{source_path}: structuralLevels cell {x},{y} must be 0..={MAX_STRUCTURAL_LEVEL}, null, or 'auto'"
                    )
                })?;
                if raw > u64::from(MAX_STRUCTURAL_LEVEL) {
                    return Err(format!(
                        "{source_path}: structural level {raw} at {x},{y} exceeds maximum {MAX_STRUCTURAL_LEVEL}"
                    ));
                }
                Some(raw as u8)
            };
            map.set_structural_level(x as i32 + offset_x, y as i32 + offset_y, level);
        }
    }
    Ok(())
}
