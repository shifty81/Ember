use std::fs::create_dir_all;
use std::path::Path;

use haven_core::{GameWorld, SceneMap, TileKind, MAP_H, MAP_W};
use image::{Rgba, RgbaImage};

use crate::island_pcg::generated_scene_id;
use crate::scene_rectangles::{SceneRectangleManifest, SceneRectangleSpec};

pub const CLIENT_ARCHIPELAGO_PREVIEW_FILENAME: &str = "archipelago.png";
const PREVIEW_SCALE_DIVISOR: i32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchipelagoPreviewReport {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub rendered_scene_count: usize,
}

pub fn export_archipelago_preview(
    manifest: &SceneRectangleManifest,
    world: &GameWorld,
    path: &str,
) -> Result<ArchipelagoPreviewReport, String> {
    let [canvas_width, canvas_height] = manifest.archipelago_generation.canvas_size_px;
    let width = (canvas_width / PREVIEW_SCALE_DIVISOR).max(1) as u32;
    let height = (canvas_height / PREVIEW_SCALE_DIVISOR).max(1) as u32;
    let mut image = RgbaImage::from_pixel(width, height, Rgba([14, 48, 66, 255]));
    paint_ocean_bands(&mut image);

    let mut rendered_scene_count = 0usize;
    for rectangle in manifest
        .scene_rectangles
        .iter()
        .filter(|rectangle| rectangle.grid_x.is_some() && rectangle.grid_y.is_some())
    {
        let scene_id = generated_scene_id(&rectangle.landmass_name, rectangle);
        let Some(scene) = world.scene_by_id(&scene_id) else {
            continue;
        };
        paint_scene_rectangle(&mut image, rectangle, scene);
        rendered_scene_count += 1;
    }

    let output = Path::new(path);
    if let Some(parent) = output.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    image.save(output).map_err(|error| error.to_string())?;
    Ok(ArchipelagoPreviewReport {
        path: path.to_string(),
        width,
        height,
        rendered_scene_count,
    })
}

fn paint_ocean_bands(image: &mut RgbaImage) {
    let height = image.height().max(1);
    for y in 0..height {
        let amount = y as f32 / height as f32;
        let color = Rgba([
            12,
            (43.0 + amount * 13.0) as u8,
            (61.0 + amount * 18.0) as u8,
            255,
        ]);
        for x in 0..image.width() {
            image.put_pixel(x, y, color);
        }
    }
}

fn paint_scene_rectangle(output: &mut RgbaImage, rectangle: &SceneRectangleSpec, scene: &SceneMap) {
    let [preview_x, preview_y, preview_width, preview_height] = rectangle.world_rect_preview_px;
    let x0 = (preview_x / PREVIEW_SCALE_DIVISOR).max(0);
    let y0 = (preview_y / PREVIEW_SCALE_DIVISOR).max(0);
    let width = (preview_width / PREVIEW_SCALE_DIVISOR).max(1);
    let height = (preview_height / PREVIEW_SCALE_DIVISOR).max(1);

    for py in 0..height {
        for px in 0..width {
            let tile_x = ((px as f32 / width as f32) * MAP_W as f32)
                .floor()
                .clamp(0.0, (MAP_W - 1) as f32) as i32;
            let tile_y = ((py as f32 / height as f32) * MAP_H as f32)
                .floor()
                .clamp(0.0, (MAP_H - 1) as f32) as i32;
            let out_x = x0 + px;
            let out_y = y0 + py;
            if out_x < 0
                || out_y < 0
                || out_x >= output.width() as i32
                || out_y >= output.height() as i32
            {
                continue;
            }
            output.put_pixel(
                out_x as u32,
                out_y as u32,
                tile_preview_color(scene.map.get(tile_x, tile_y)),
            );
        }
    }
}

fn tile_preview_color(tile: TileKind) -> Rgba<u8> {
    match tile {
        TileKind::OceanDeep | TileKind::DeepWater => Rgba([16, 56, 83, 255]),
        TileKind::Water => Rgba([24, 76, 104, 255]),
        TileKind::OceanShallow
        | TileKind::ShallowWater
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend
        | TileKind::ShoreFoam => Rgba([48, 112, 130, 255]),
        TileKind::WetSand | TileKind::MudBank => Rgba([157, 140, 91, 255]),
        TileKind::Sand | TileKind::PebbleShore => Rgba([205, 187, 123, 255]),
        TileKind::Grass | TileKind::TallGrass => Rgba([77, 126, 73, 255]),
        TileKind::Dirt | TileKind::TilledSoil | TileKind::WateredSoil => Rgba([113, 78, 50, 255]),
        TileKind::Road | TileKind::StonePath | TileKind::MountainPath => Rgba([132, 119, 93, 255]),
        TileKind::Cliff | TileKind::MountainRock | TileKind::CaveWall => Rgba([91, 91, 85, 255]),
        _ => Rgba([99, 119, 87, 255]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archipelago_layout::generate_archipelago_layout;
    use crate::island_pcg::{generate_landmass, IslandGenerationSettings};
    use crate::scene_rectangles::{
        load_scene_rectangle_manifest_from_path, SCENE_RECTANGLE_MANIFEST_PATH,
    };
    use std::path::PathBuf;

    fn manifest() -> SceneRectangleManifest {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("haven_world crate should live under crates/<name>");
        let manifest_path = repo_root.join(SCENE_RECTANGLE_MANIFEST_PATH);
        load_scene_rectangle_manifest_from_path(&manifest_path.to_string_lossy()).expect("manifest")
    }

    #[test]
    fn exports_seeded_archipelago_from_actual_generated_scene_tiles() {
        let mut manifest = manifest();
        generate_archipelago_layout(&mut manifest, 4_242).expect("layout");
        let mut world = GameWorld::starter();
        for landmass_id in 0..10 {
            let generated = generate_landmass(
                &manifest,
                landmass_id,
                IslandGenerationSettings {
                    seed: 4_242 ^ landmass_id as u64,
                    mountain_radius: 0.4,
                    shoreline_width: 0.12,
                    tree_density: 0.0,
                    geography: crate::GeographicGenerationProfile::default(),
                },
            )
            .expect("landmass");
            for scene in generated.scenes {
                world
                    .insert_scene(scene.scene)
                    .expect("insert generated scene");
            }
        }
        let path: PathBuf = std::env::temp_dir().join("havenwild-client-preview-test.png");
        let report = export_archipelago_preview(&manifest, &world, &path.to_string_lossy())
            .expect("export preview");
        assert_eq!(report.rendered_scene_count, 63);
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }
}
