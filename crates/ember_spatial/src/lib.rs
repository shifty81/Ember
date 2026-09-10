//! Explicit 2D, 2.5D, and 3D spatial contracts for Ember authoring/runtime.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpatialMode {
    TwoD,
    TwoPointFiveD,
    ThreeD,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform2D {
    pub translation: [f32; 2],
    pub rotation_degrees: f32,
    pub scale: [f32; 2],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform2PointFiveD {
    pub planar: [f32; 2],
    pub elevation: f32,
    pub yaw_degrees: f32,
    pub scale: [f32; 2],
    pub depth_bias: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform3D {
    pub translation: [f32; 3],
    pub rotation_euler_degrees: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum SpatialTransform {
    TwoD(Transform2D),
    TwoPointFiveD(Transform2PointFiveD),
    ThreeD(Transform3D),
}

impl SpatialTransform {
    pub fn mode(self) -> SpatialMode {
        match self {
            Self::TwoD(_) => SpatialMode::TwoD,
            Self::TwoPointFiveD(_) => SpatialMode::TwoPointFiveD,
            Self::ThreeD(_) => SpatialMode::ThreeD,
        }
    }

    pub fn position3(self) -> [f32; 3] {
        match self {
            Self::TwoD(transform) => [transform.translation[0], transform.translation[1], 0.0],
            Self::TwoPointFiveD(transform) => [
                transform.planar[0],
                transform.planar[1],
                transform.elevation,
            ],
            Self::ThreeD(transform) => transform.translation,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        let finite = match self {
            Self::TwoD(transform) => transform
                .translation
                .into_iter()
                .chain([transform.rotation_degrees])
                .chain(transform.scale)
                .all(f32::is_finite),
            Self::TwoPointFiveD(transform) => transform
                .planar
                .into_iter()
                .chain([
                    transform.elevation,
                    transform.yaw_degrees,
                    transform.depth_bias,
                ])
                .chain(transform.scale)
                .all(f32::is_finite),
            Self::ThreeD(transform) => transform
                .translation
                .into_iter()
                .chain(transform.rotation_euler_degrees)
                .chain(transform.scale)
                .all(f32::is_finite),
        };
        if !finite {
            return Err("spatial transform contains non-finite values".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrthographicDepthProfile {
    pub elevation_pixels_per_unit: f32,
    pub depth_pixels_per_unit: f32,
}

impl OrthographicDepthProfile {
    pub fn project(self, world: [f32; 3]) -> [f32; 2] {
        [
            world[0],
            world[1] - world[2] * self.elevation_pixels_per_unit
                + (world[0] + world[1]) * self.depth_pixels_per_unit,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_point_five_d_keeps_explicit_elevation() {
        let transform = SpatialTransform::TwoPointFiveD(Transform2PointFiveD {
            planar: [10.0, 20.0],
            elevation: 3.0,
            yaw_degrees: 0.0,
            scale: [1.0, 1.0],
            depth_bias: 0.0,
        });
        assert_eq!(transform.mode(), SpatialMode::TwoPointFiveD);
        assert_eq!(transform.position3(), [10.0, 20.0, 3.0]);
    }
}
