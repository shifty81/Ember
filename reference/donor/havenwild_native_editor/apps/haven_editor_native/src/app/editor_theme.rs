use macroquad::prelude::Color;

/// Shared native-editor visual language.
///
/// Keep editor chrome, controls, overlays, and workspace panels on these tokens
/// instead of introducing workspace-local color literals. Gameplay rendering
/// colors remain owned by their rendering/domain modules.
#[allow(dead_code)]
pub(crate) mod colors {
    use super::Color;

    pub const WINDOW_BG: Color = Color::new(0.070, 0.070, 0.070, 1.0);
    pub const TOP_BAR_BG: Color = Color::new(0.055, 0.055, 0.055, 1.0);
    pub const PANEL_BG: Color = Color::new(0.090, 0.090, 0.090, 0.985);
    pub const PANEL_RAISED: Color = Color::new(0.120, 0.120, 0.120, 1.0);
    pub const PANEL_HEADER: Color = Color::new(0.105, 0.105, 0.105, 1.0);
    pub const CANVAS_SURROUND: Color = Color::new(0.125, 0.125, 0.125, 1.0);
    pub const CONTROL_BG: Color = Color::new(0.145, 0.145, 0.145, 1.0);
    pub const CONTROL_HOVER: Color = Color::new(0.205, 0.205, 0.205, 1.0);
    pub const CONTROL_PRESSED: Color = Color::new(0.095, 0.095, 0.095, 1.0);
    pub const BORDER_SUBTLE: Color = Color::new(0.210, 0.210, 0.210, 1.0);
    pub const BORDER_STRONG: Color = Color::new(0.315, 0.315, 0.315, 1.0);

    pub const TEXT_PRIMARY: Color = Color::new(0.875, 0.875, 0.875, 1.0);
    pub const TEXT_SECONDARY: Color = Color::new(0.585, 0.585, 0.585, 1.0);
    pub const TEXT_DISABLED: Color = Color::new(0.390, 0.390, 0.390, 1.0);

    // Havenwild editor identity: warm amber-orange, distinct from gameplay state colors.
    pub const ACCENT: Color = Color::new(0.965, 0.365, 0.105, 1.0);
    pub const ACCENT_HOVER: Color = Color::new(1.000, 0.445, 0.145, 1.0);
    pub const ACCENT_PRESSED: Color = Color::new(0.790, 0.255, 0.055, 1.0);
    pub const SELECTION_FILL: Color = Color::new(0.965, 0.365, 0.105, 0.22);
    pub const SELECTION_OUTLINE: Color = Color::new(1.000, 0.585, 0.245, 1.0);

    pub const GOOD: Color = Color::new(0.340, 0.760, 0.480, 1.0);
    pub const WARN: Color = Color::new(0.965, 0.650, 0.245, 1.0);
    pub const DANGER: Color = Color::new(0.850, 0.265, 0.245, 1.0);

    pub const GRID_PIXEL: Color = Color::new(0.760, 0.760, 0.760, 0.16);
    pub const GRID_FRAME: Color = Color::new(0.965, 0.365, 0.105, 0.68);
    pub const GRID_WORLD: Color = Color::new(0.280, 0.660, 0.860, 0.52);
    pub const CHECKER_LIGHT: Color = Color::new(0.235, 0.235, 0.235, 1.0);
    pub const CHECKER_DARK: Color = Color::new(0.180, 0.180, 0.180, 1.0);
}

#[allow(dead_code)]
pub(crate) mod metrics {
    pub const MENU_BAR_H: f32 = 40.0;
    pub const DOCUMENT_BAR_H: f32 = 34.0;
    pub const TOP_BAR_H: f32 = MENU_BAR_H + DOCUMENT_BAR_H;
    pub const OUTER_GAP: f32 = 7.0;
    pub const PANEL_HEADER_H: f32 = 28.0;
    pub const STATUS_BAR_H: f32 = 28.0;
    pub const CONTROL_H: f32 = 28.0;
    pub const TOOL_BUTTON: f32 = 28.0;
    pub const DOCK_TAB_H: f32 = 26.0;
    pub const SPLITTER: f32 = 4.0;
    pub const PANEL_PAD: f32 = 8.0;
    pub const CORNER_RADIUS: f32 = 2.0;

    // Responsive shell authority. Persisted panel preferences are desired
    // widths; the live layout may compress them to keep every panel on-screen.
    pub const MIN_CANVAS_W: f32 = 320.0;
    pub const MIN_WINDOW_LAYOUT_W: f32 = 480.0;
    pub const MIN_WINDOW_LAYOUT_H: f32 = 360.0;
    pub const MIN_BOTTOM_DOCK_H: f32 = 96.0;
}
