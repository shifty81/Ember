//! Project-aware source workspace contracts for Ember's integrated IDE lane.

use ember_core::ProjectPath;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceLanguage {
    Rust,
    Json,
    Toml,
    Markdown,
    Text,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceFileRecord {
    pub path: ProjectPath,
    pub language: SourceLanguage,
    pub fingerprint: u64,
    pub line_count: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceIndex {
    pub files: BTreeMap<String, SourceFileRecord>,
}

impl SourceIndex {
    pub fn upsert(&mut self, path: ProjectPath, text: &str) -> &SourceFileRecord {
        let key = path.as_path().to_string_lossy().replace('\\', "/");
        let record = SourceFileRecord {
            language: detect_language(&key),
            path,
            fingerprint: fnv1a64(text.as_bytes()),
            line_count: text.lines().count(),
        };
        self.files.insert(key.clone(), record);
        self.files.get(&key).expect("record inserted")
    }
}

pub fn detect_language(path: &str) -> SourceLanguage {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
    {
        Some(extension) if extension == "rs" => SourceLanguage::Rust,
        Some(extension) if extension == "json" => SourceLanguage::Json,
        Some(extension) if extension == "toml" => SourceLanguage::Toml,
        Some(extension) if extension == "md" => SourceLanguage::Markdown,
        Some(extension) if matches!(extension.as_str(), "txt" | "ron") => SourceLanguage::Text,
        Some(extension) => SourceLanguage::Other(extension),
        None => SourceLanguage::Text,
    }
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextEdit {
    pub start_byte: usize,
    pub end_byte: usize,
    pub replacement: String,
}

pub fn apply_text_edits(source: &str, edits: &[TextEdit]) -> Result<String, String> {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| (edit.start_byte, edit.end_byte));
    for edit in &ordered {
        if edit.start_byte > edit.end_byte || edit.end_byte > source.len() {
            return Err("text edit range is invalid".into());
        }
        if !source.is_char_boundary(edit.start_byte) || !source.is_char_boundary(edit.end_byte) {
            return Err("text edit range splits a UTF-8 code point".into());
        }
    }
    for pair in ordered.windows(2) {
        if pair[0].end_byte > pair[1].start_byte {
            return Err("text edits overlap".into());
        }
    }
    let mut result = source.to_string();
    for edit in ordered.into_iter().rev() {
        result.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_text_edits_apply_without_offset_drift() {
        let source = "alpha beta gamma";
        let result = apply_text_edits(
            source,
            &[
                TextEdit {
                    start_byte: 0,
                    end_byte: 5,
                    replacement: "one".into(),
                },
                TextEdit {
                    start_byte: 11,
                    end_byte: 16,
                    replacement: "three".into(),
                },
            ],
        )
        .unwrap();
        assert_eq!(result, "one beta three");
    }
}
