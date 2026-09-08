use serde::{Deserialize, Serialize};

/// Presentation-neutral inspection result shared by runtime, renderer, native editor,
/// validation tools, and future automation surfaces.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectorReport {
    pub title: String,
    pub lines: Vec<String>,
}

impl InspectorReport {
    pub fn new(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.lines.is_empty()
    }
}
