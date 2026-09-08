use macroquad::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    /// A dedicated editor UI font atlas. Macroquad's built-in default font is
    /// intentionally not used once this is initialized: on the affected
    /// Windows path its glyph atlas can render as opaque black while all other
    /// UI colors remain correct. Keeping a separate atlas also prevents asset
    /// preview texture work from sharing the editor's text authority.
    static EDITOR_UI_FONT: RefCell<Option<Font>> = const { RefCell::new(None) };
}

pub(crate) fn initialize_editor_font() -> String {
    let candidates = editor_font_candidates();
    for path in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        match load_ttf_font_from_bytes(&bytes) {
            Ok(font) => {
                EDITOR_UI_FONT.with(|slot| *slot.borrow_mut() = Some(font));
                return format!("Editor UI font atlas loaded from {}", path.display());
            }
            Err(_) => continue,
        }
    }

    EDITOR_UI_FONT.with(|slot| *slot.borrow_mut() = None);
    "Editor UI font atlas unavailable; Macroquad fallback font active".to_string()
}

fn editor_font_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        let fonts = Path::new(&windir).join("Fonts");
        // Segoe UI is the native Windows UI face. Arial is retained only as a
        // compatibility fallback for stripped-down Windows installations.
        candidates.push(fonts.join("segoeui.ttf"));
        candidates.push(fonts.join("segoeuisl.ttf"));
        candidates.push(fonts.join("arial.ttf"));
    }
    candidates
}

pub(crate) fn draw_editor_text(
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
) -> TextDimensions {
    EDITOR_UI_FONT.with(|slot| {
        let font = slot.borrow();
        if let Some(font) = font.as_ref() {
            draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: font_size.max(1.0).round() as u16,
                    color,
                    ..Default::default()
                },
            )
        } else {
            draw_text(text, x, y, font_size, color)
        }
    })
}

pub(crate) fn measure_editor_text(
    text: &str,
    _font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions {
    EDITOR_UI_FONT.with(|slot| {
        let font = slot.borrow();
        measure_text(text, font.as_ref(), font_size, font_scale)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_font_candidates_never_depend_on_repository_font_files() {
        for path in editor_font_candidates() {
            let normalized = path
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            assert!(normalized.contains("/fonts/"));
            assert!(!normalized.contains("/content/"));
            assert!(!normalized.contains("/assets/"));
        }
    }
}
