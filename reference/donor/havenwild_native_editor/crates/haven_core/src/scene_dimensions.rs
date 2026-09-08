use crate::{MAP_H, MAP_W};

/// Logical dimensions of a scene. The current tile backing store remains capped by
/// MAP_W x MAP_H during migration, but consumers must use these bounds rather than
/// assuming every scene occupies the full legacy canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SceneDimensions {
    pub width: usize,
    pub height: usize,
}

impl SceneDimensions {
    pub const fn new(width: usize, height: usize) -> Self { Self { width, height } }
    pub const fn legacy_canvas() -> Self { Self::new(MAP_W, MAP_H) }
    pub fn validate(self) -> Result<Self, String> {
        if self.width == 0 || self.height == 0 { return Err("scene dimensions must be non-zero".into()); }
        if self.width > MAP_W || self.height > MAP_H {
            return Err(format!("scene dimensions {}x{} exceed transitional backing store {}x{}", self.width, self.height, MAP_W, MAP_H));
        }
        Ok(self)
    }
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }
    pub const fn cell_count(self) -> usize { self.width * self.height }
}

impl Default for SceneDimensions { fn default() -> Self { Self::legacy_canvas() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn logical_bounds_are_independent_of_legacy_canvas() {
        let d = SceneDimensions::new(14, 11).validate().unwrap();
        assert!(d.contains(13, 10));
        assert!(!d.contains(14, 10));
        assert_ne!(d, SceneDimensions::legacy_canvas());
    }
}
