use super::render_helpers::*;
use super::*;
use haven_pixel::{NewPixelDocumentSpec, PixelDocumentKind};

const MODAL_W: f32 = 620.0;
const MODAL_H: f32 = 520.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NewPixelField {
    Name,
    Width,
    Height,
    CellWidth,
    CellHeight,
    Spacing,
    Padding,
    OffsetX,
    OffsetY,
}

#[derive(Clone, Debug)]
pub(crate) struct NewPixelDialogState {
    pub spec: NewPixelDocumentSpec,
    pub focused: NewPixelField,
    pub error: Option<String>,
}

impl NewPixelDialogState {
    pub(crate) fn new() -> Self {
        Self {
            spec: NewPixelDocumentSpec::for_kind(PixelDocumentKind::Tile),
            focused: NewPixelField::Name,
            error: None,
        }
    }

    pub(crate) fn select_kind(&mut self, kind: PixelDocumentKind) {
        let current_name = self.spec.display_name.clone();
        self.spec = NewPixelDocumentSpec::for_kind(kind);
        if !current_name.trim().is_empty() && !current_name.starts_with("New Havenwild") {
            self.spec.display_name = current_name;
        }
        self.error = None;
    }
}

pub(crate) fn new_pixel_modal_rect() -> Rect {
    Rect::new(
        (screen_width() - MODAL_W) * 0.5,
        (screen_height() - MODAL_H) * 0.5,
        MODAL_W,
        MODAL_H,
    )
}

pub(crate) fn draw_new_pixel_dialog(dialog: &NewPixelDialogState) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.65),
    );
    let rect = new_pixel_modal_rect();
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL_BG);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, PANEL_EDGE);
    draw_editor_text(
        "Create Pixel Asset",
        rect.x + 22.0,
        rect.y + 34.0,
        24.0,
        TEXT,
    );
    draw_editor_text(
        "Choose an asset preset, then adjust its canvas and atlas grid.",
        rect.x + 22.0,
        rect.y + 58.0,
        15.0,
        MUTED,
    );

    for (index, kind) in PixelDocumentKind::ALL.iter().copied().enumerate() {
        let button = kind_rect(rect, index);
        draw_editor_widget(button, kind.label(), dialog.spec.kind == kind);
    }

    draw_field(
        rect,
        dialog,
        NewPixelField::Name,
        "Asset name",
        &dialog.spec.display_name,
        0,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::Width,
        "Canvas width",
        &dialog.spec.width.to_string(),
        1,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::Height,
        "Canvas height",
        &dialog.spec.height.to_string(),
        2,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::CellWidth,
        "Cell width",
        &dialog.spec.cell_width.to_string(),
        3,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::CellHeight,
        "Cell height",
        &dialog.spec.cell_height.to_string(),
        4,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::Spacing,
        "Spacing",
        &dialog.spec.spacing.to_string(),
        5,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::Padding,
        "Padding",
        &dialog.spec.padding.to_string(),
        6,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::OffsetX,
        "Grid offset X",
        &dialog.spec.offset_x.to_string(),
        7,
    );
    draw_field(
        rect,
        dialog,
        NewPixelField::OffsetY,
        "Grid offset Y",
        &dialog.spec.offset_y.to_string(),
        8,
    );

    if let Some(error) = &dialog.error {
        draw_scissored_text(
            error,
            rect.x + 22.0,
            rect.y + rect.h - 70.0,
            rect.w - 260.0,
            15.0,
            WARN,
        );
    } else {
        draw_editor_text(
            "Enter creates the document. Escape cancels.",
            rect.x + 22.0,
            rect.y + rect.h - 48.0,
            14.0,
            MUTED,
        );
    }
    draw_editor_widget_tone(cancel_rect(rect), "Cancel", false, WidgetTone::Quiet);
    draw_editor_widget_tone(
        create_rect(rect),
        "Create & Open",
        true,
        WidgetTone::Primary,
    );
}

fn draw_field(
    rect: Rect,
    dialog: &NewPixelDialogState,
    field: NewPixelField,
    label: &str,
    value: &str,
    row: usize,
) {
    draw_text_field(field_rect(rect, row), label, value, dialog.focused == field);
}

pub(crate) fn kind_rect(rect: Rect, index: usize) -> Rect {
    let column = index % 4;
    let row = index / 4;
    Rect::new(
        rect.x + 22.0 + column as f32 * 143.0,
        rect.y + 78.0 + row as f32 * 38.0,
        134.0,
        30.0,
    )
}

pub(crate) fn field_rect(rect: Rect, row: usize) -> Rect {
    let column = row % 3;
    let grid_row = row / 3;
    Rect::new(
        rect.x + 22.0 + column as f32 * 194.0,
        rect.y + 188.0 + grid_row as f32 * 72.0,
        176.0,
        32.0,
    )
}

pub(crate) fn cancel_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 250.0, rect.y + rect.h - 58.0, 102.0, 34.0)
}

pub(crate) fn create_rect(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 138.0, rect.y + rect.h - 58.0, 116.0, 34.0)
}

pub(crate) fn field_at(rect: Rect, point: Vec2) -> Option<NewPixelField> {
    const FIELDS: [NewPixelField; 9] = [
        NewPixelField::Name,
        NewPixelField::Width,
        NewPixelField::Height,
        NewPixelField::CellWidth,
        NewPixelField::CellHeight,
        NewPixelField::Spacing,
        NewPixelField::Padding,
        NewPixelField::OffsetX,
        NewPixelField::OffsetY,
    ];
    FIELDS
        .into_iter()
        .enumerate()
        .find_map(|(index, field)| field_rect(rect, index).contains(point).then_some(field))
}
