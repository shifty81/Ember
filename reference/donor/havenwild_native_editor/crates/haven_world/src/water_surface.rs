use haven_core::TileKind;

use crate::water_render_mask::WaterRenderMask;

/// Renderer-neutral water optics produced from the semantic V7 water tile,
/// world position, time, and resolved topology. GPU shaders, editor previews,
/// and the current CPU bridge consume the same values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterSurfaceSample {
    pub depth01: f32,
    pub reflection_strength: f32,
    pub refraction_strength: f32,
    pub caustic_strength: f32,
    pub wave_phase: f32,
    pub foam_phase: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterDetailProfile {
    pub blend_layers: u8,
    pub draw_diagonal_corners: bool,
}

impl WaterDetailProfile {
    pub fn for_camera_zoom(camera_zoom: f32) -> Self {
        if camera_zoom < 0.55 {
            Self {
                blend_layers: 1,
                draw_diagonal_corners: false,
            }
        } else if camera_zoom < 0.95 {
            Self {
                blend_layers: 2,
                draw_diagonal_corners: false,
            }
        } else {
            Self {
                blend_layers: 2,
                draw_diagonal_corners: true,
            }
        }
    }

    /// Apply a conservative pressure budget at every zoom. V7 topology remains
    /// unchanged; only decorative water subdivision and corner detail are reduced.
    pub const fn with_frame_pressure(self, _camera_zoom: f32, pressured: bool) -> Self {
        if !pressured {
            return self;
        }
        Self {
            blend_layers: 1,
            draw_diagonal_corners: false,
        }
    }

    pub const fn label(self) -> &'static str {
        match self.blend_layers {
            0..=1 => "ultra-wide",
            2 => "wide",
            3..=4 => "balanced",
            _ => "full",
        }
    }
}

impl WaterSurfaceSample {
    pub const fn is_deep(self) -> bool {
        self.depth01 >= 0.70
    }
}

/// A deterministic world-space disturbance. This is deliberately independent
/// from Macroquad so footsteps, fishing floats, rain, boats, and swimming can
/// all feed one future ripple texture or compute pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterRippleEvent {
    pub world_x: f32,
    pub world_y: f32,
    pub radius: f32,
    pub strength: f32,
    pub started_at: f32,
}

impl WaterRippleEvent {
    pub fn amplitude_at(self, now: f32) -> f32 {
        let age = (now - self.started_at).max(0.0);
        (self.strength * (1.0 - age / 2.4)).max(0.0)
    }

    pub fn is_alive(self, now: f32) -> bool {
        self.amplitude_at(now) > 0.0
    }
}

pub fn resolve_water_surface_sample(
    tile: TileKind,
    world_x: i32,
    world_y: i32,
    time: f32,
    mask: WaterRenderMask,
) -> Option<WaterSurfaceSample> {
    let depth01 = water_depth01(tile)?;
    let phase_seed = world_x as f32 * 0.173 + world_y as f32 * 0.291;
    let wave_phase = time * (0.58 + depth01 * 0.22) + phase_seed;
    let foam_phase = time * 1.08 + phase_seed * 1.7;
    let shore_factor = if mask.shoreline_edges != 0 { 1.0 } else { 0.0 };
    let depth_edge_factor = if mask.has_depth_transition() {
        1.0
    } else {
        0.0
    };

    Some(WaterSurfaceSample {
        depth01,
        reflection_strength: 0.14 + depth01 * 0.42,
        refraction_strength: 0.46 - depth01 * 0.22,
        caustic_strength: (0.62 - depth01 * 0.56).max(0.04),
        wave_phase,
        foam_phase: foam_phase + shore_factor * 0.35 + depth_edge_factor * 0.12,
    })
}

fn water_depth01(tile: TileKind) -> Option<f32> {
    match tile {
        TileKind::ShoreFoam => Some(0.08),
        TileKind::Water | TileKind::RiverWater | TileKind::RiverMouthBlend => Some(0.24),
        TileKind::ShallowWater | TileKind::OceanShallow => Some(0.36),
        TileKind::DeepWater => Some(0.78),
        TileKind::OceanDeep => Some(0.92),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shallow_water_refracts_more_and_caustics_are_stronger() {
        let mask = WaterRenderMask::default();
        let shallow = resolve_water_surface_sample(TileKind::ShallowWater, 4, 7, 2.0, mask)
            .expect("shallow water sample");
        let deep = resolve_water_surface_sample(TileKind::DeepWater, 4, 7, 2.0, mask)
            .expect("deep water sample");
        assert!(shallow.refraction_strength > deep.refraction_strength);
        assert!(shallow.caustic_strength > deep.caustic_strength);
        assert!(deep.reflection_strength > shallow.reflection_strength);
    }

    #[test]
    fn phase_is_world_space_stable_and_time_animated() {
        let mask = WaterRenderMask::default();
        let a = resolve_water_surface_sample(TileKind::ShallowWater, 10, 12, 1.0, mask).unwrap();
        let b = resolve_water_surface_sample(TileKind::ShallowWater, 10, 12, 2.0, mask).unwrap();
        let c = resolve_water_surface_sample(TileKind::ShallowWater, 11, 12, 1.0, mask).unwrap();
        assert_ne!(a.wave_phase, b.wave_phase);
        assert_ne!(a.wave_phase, c.wave_phase);
    }

    #[test]
    fn pressure_budget_reduces_normal_zoom_to_single_layer() {
        let normal = WaterDetailProfile::for_camera_zoom(1.0).with_frame_pressure(1.0, true);
        assert_eq!(normal.blend_layers, 1);
        assert!(!normal.draw_diagonal_corners);
    }

    #[test]
    fn pressure_budget_reduces_all_zoom_levels_to_single_layer() {
        let balanced = WaterDetailProfile::for_camera_zoom(0.80).with_frame_pressure(0.80, true);
        let wide = WaterDetailProfile::for_camera_zoom(0.60).with_frame_pressure(0.60, true);
        assert_eq!(balanced.blend_layers, 1);
        assert!(!balanced.draw_diagonal_corners);
        assert_eq!(wide.blend_layers, 1);
    }

    #[test]
    fn water_detail_lod_reduces_work_at_wide_zoom() {
        let ultra_wide = WaterDetailProfile::for_camera_zoom(0.45);
        let wide = WaterDetailProfile::for_camera_zoom(0.60);
        let normal = WaterDetailProfile::for_camera_zoom(1.0);
        assert_eq!(ultra_wide.blend_layers, 1);
        assert!(!ultra_wide.draw_diagonal_corners);
        assert_eq!(wide.blend_layers, 2);
        assert!(!wide.draw_diagonal_corners);
        assert_eq!(normal.blend_layers, 2);
        assert!(normal.draw_diagonal_corners);
    }

    #[test]
    fn ripple_events_decay_without_becoming_negative() {
        let ripple = WaterRippleEvent {
            world_x: 1.0,
            world_y: 2.0,
            radius: 8.0,
            strength: 1.0,
            started_at: 5.0,
        };
        assert!(ripple.is_alive(5.5));
        assert_eq!(ripple.amplitude_at(8.0), 0.0);
        assert!(!ripple.is_alive(8.0));
    }
}
