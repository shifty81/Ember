use haven_core::TileKind;
use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const TERRAIN_MATERIAL_BINDINGS_PATH: &str =
    "content/assets/terrain_material_bindings_v0_2.json";

static TERRAIN_MATERIAL_BINDINGS: OnceLock<Result<TerrainMaterialBindings, String>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerrainRenderMode {
    CornerTuple,
    GeneratedBand,
    Overlay,
    Structure,
    DerivedElevationStructure,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerrainMaterialBinding {
    pub tile_kind: String,
    pub material: Option<String>,
    pub geometry_family: String,
    pub render_mode: TerrainRenderMode,
    #[serde(default)]
    pub overlay: Option<String>,
    #[serde(default)]
    pub tuple_fallback_material: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TerrainMaterialBindingsFile {
    schema: String,
    version: String,
    bindings: Vec<TerrainMaterialBinding>,
}

#[derive(Clone, Debug)]
pub struct TerrainMaterialBindings {
    schema: String,
    version: String,
    bindings: Vec<TerrainMaterialBinding>,
}

impl TerrainMaterialBindings {
    pub fn load_default() -> Result<Self, String> {
        let path = repo_root_dir().join(TERRAIN_MATERIAL_BINDINGS_PATH);
        let payload =
            read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let file: TerrainMaterialBindingsFile = serde_json::from_str(&payload)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file.schema != "havenwild.terrain_material_bindings.v0_2" {
            return Err(format!(
                "{}: unsupported schema {}",
                path.display(),
                file.schema
            ));
        }
        let registry = Self {
            schema: file.schema,
            version: file.version,
            bindings: file.bindings,
        };
        registry.validate()?;
        Ok(registry)
    }

    pub fn binding(&self, tile: TileKind) -> Option<&TerrainMaterialBinding> {
        let name = tile_kind_name(tile);
        self.bindings
            .iter()
            .find(|binding| binding.tile_kind == name)
    }

    pub fn corner_tuple_material(&self, tile: TileKind) -> Option<&str> {
        let binding = self.binding(tile)?;
        match binding.render_mode {
            TerrainRenderMode::CornerTuple | TerrainRenderMode::GeneratedBand => {
                binding.material.as_deref()
            }
            TerrainRenderMode::Overlay
            | TerrainRenderMode::Structure
            | TerrainRenderMode::DerivedElevationStructure => {
                binding.tuple_fallback_material.as_deref()
            }
        }
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }
    pub fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<(), String> {
        let mut names = std::collections::BTreeSet::new();
        for binding in &self.bindings {
            if !names.insert(binding.tile_kind.as_str()) {
                return Err(format!(
                    "duplicate terrain binding for {}",
                    binding.tile_kind
                ));
            }
            let needs_material = matches!(
                binding.render_mode,
                TerrainRenderMode::CornerTuple | TerrainRenderMode::GeneratedBand
            );
            if needs_material && binding.material.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "terrain binding {} requires a canonical material",
                    binding.tile_kind
                ));
            }
            let needs_overlay = matches!(
                binding.render_mode,
                TerrainRenderMode::GeneratedBand | TerrainRenderMode::Overlay
            );
            if needs_overlay && binding.overlay.as_deref().is_none_or(str::is_empty) {
                return Err(format!(
                    "terrain binding {} requires an overlay recipe",
                    binding.tile_kind
                ));
            }
        }
        Ok(())
    }
}

pub fn terrain_material_bindings() -> Result<&'static TerrainMaterialBindings, &'static str> {
    TERRAIN_MATERIAL_BINDINGS
        .get_or_init(TerrainMaterialBindings::load_default)
        .as_ref()
        .map_err(|_| "terrain material bindings unavailable")
}

pub fn terrain_material_binding(tile: TileKind) -> Option<&'static TerrainMaterialBinding> {
    terrain_material_bindings().ok()?.binding(tile)
}

pub fn canonical_corner_tuple_material(tile: TileKind) -> Option<&'static str> {
    terrain_material_bindings()
        .ok()?
        .corner_tuple_material(tile)
}

fn tile_kind_name(tile: TileKind) -> &'static str {
    match tile {
        TileKind::Grass => "Grass",
        TileKind::TallGrass => "TallGrass",
        TileKind::Sand => "Sand",
        TileKind::WetSand => "WetSand",
        TileKind::PebbleShore => "PebbleShore",
        TileKind::Road => "Road",
        TileKind::StonePath => "StonePath",
        TileKind::MountainPath => "MountainPath",
        TileKind::WoodFloor => "WoodFloor",
        TileKind::PlankFloor => "PlankFloor",
        TileKind::StoneFloor => "StoneFloor",
        TileKind::BrickFloor => "BrickFloor",
        TileKind::Wall => "Wall",
        TileKind::Cliff => "Cliff",
        TileKind::MountainRock => "MountainRock",
        TileKind::Dirt => "Dirt",
        TileKind::Bridge => "Bridge",
        TileKind::CaveFloor => "CaveFloor",
        TileKind::CaveWall => "CaveWall",
        TileKind::TilledSoil => "TilledSoil",
        TileKind::WateredSoil => "WateredSoil",
        TileKind::Crop => "Crop",
        TileKind::GreenhouseZone => "GreenhouseZone",
        TileKind::Water => "Water",
        TileKind::ShallowWater => "ShallowWater",
        TileKind::DeepWater => "DeepWater",
        TileKind::OceanDeep => "OceanDeep",
        TileKind::OceanShallow => "OceanShallow",
        TileKind::RiverWater => "RiverWater",
        TileKind::RiverMouthBlend => "RiverMouthBlend",
        TileKind::ShoreFoam => "ShoreFoam",
        TileKind::MudBank => "MudBank",
    }
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_path_and_shore_semantics_keep_distinct_materials() {
        assert_eq!(
            canonical_corner_tuple_material(TileKind::PebbleShore),
            Some("Gravel_1")
        );
        assert_eq!(
            canonical_corner_tuple_material(TileKind::StonePath),
            Some("Stone_Tan")
        );
        assert_ne!(
            canonical_corner_tuple_material(TileKind::PebbleShore),
            canonical_corner_tuple_material(TileKind::StonePath)
        );
    }

    #[test]
    fn generated_and_overlay_roles_are_explicit() {
        let wet_sand = terrain_material_binding(TileKind::WetSand).expect("wet sand binding");
        assert_eq!(wet_sand.render_mode, TerrainRenderMode::CornerTuple);
        assert_eq!(wet_sand.material.as_deref(), Some("Sand"));
        assert_eq!(wet_sand.overlay, None);
        let foam = terrain_material_binding(TileKind::ShoreFoam).expect("foam binding");
        assert_eq!(foam.render_mode, TerrainRenderMode::Overlay);
        assert_eq!(foam.material, None);
        assert_eq!(
            foam.tuple_fallback_material.as_deref(),
            Some("Water_Shallows_Sand")
        );
        assert_eq!(
            canonical_corner_tuple_material(TileKind::ShoreFoam),
            Some("Water_Shallows_Sand")
        );
    }

    #[test]
    fn derived_cliff_mode_loads_without_disabling_the_material_registry() {
        let bindings = terrain_material_bindings().expect("terrain material bindings");
        let cliff = bindings.binding(TileKind::Cliff).expect("cliff binding");

        assert_eq!(
            cliff.render_mode,
            TerrainRenderMode::DerivedElevationStructure
        );
        assert_eq!(canonical_corner_tuple_material(TileKind::Cliff), None);
        assert_eq!(
            canonical_corner_tuple_material(TileKind::Grass),
            Some("Grass")
        );
    }

    #[test]
    fn structures_do_not_alias_ground_materials() {
        let bridge = terrain_material_binding(TileKind::Bridge).expect("bridge binding");
        assert_eq!(bridge.render_mode, TerrainRenderMode::Structure);
        assert_eq!(bridge.material, None);
        assert_eq!(bridge.tuple_fallback_material.as_deref(), Some("Dirt_Tan"));
    }
}
