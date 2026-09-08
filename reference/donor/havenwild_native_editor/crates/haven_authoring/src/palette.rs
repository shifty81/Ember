use haven_core::{BuildTool, ObjectKind, TileKind, ZoneKind};

/// Canonical world-authoring palette shared by developer and player-facing
/// authoring frontends. Frontends may filter these tools through an
/// [`crate::AuthoringProfile`].
pub struct AuthoringPalette {
    pub tools: Vec<BuildTool>,
}

impl Default for AuthoringPalette {
    fn default() -> Self {
        Self {
            tools: vec![
                BuildTool::Inspect,
                BuildTool::Floor(TileKind::Grass),
                BuildTool::Floor(TileKind::Dirt),
                BuildTool::Floor(TileKind::Sand),
                BuildTool::Floor(TileKind::PebbleShore),
                BuildTool::Floor(TileKind::Road),
                BuildTool::Floor(TileKind::StonePath),
                BuildTool::Floor(TileKind::MountainPath),
                BuildTool::Floor(TileKind::MountainRock),
                BuildTool::Floor(TileKind::CaveFloor),
                BuildTool::Floor(TileKind::TilledSoil),
                BuildTool::Floor(TileKind::MudBank),
                BuildTool::Floor(TileKind::Water),
                BuildTool::Floor(TileKind::ShallowWater),
                BuildTool::Floor(TileKind::DeepWater),
                BuildTool::Object(ObjectKind::Table),
                BuildTool::Object(ObjectKind::Chair),
                BuildTool::Object(ObjectKind::Keg),
                BuildTool::Object(ObjectKind::Bed),
                BuildTool::Object(ObjectKind::Bar),
                BuildTool::Object(ObjectKind::Fireplace),
                BuildTool::Object(ObjectKind::Tree),
                BuildTool::Object(ObjectKind::Bush),
                BuildTool::Object(ObjectKind::Boulder),
                BuildTool::Object(ObjectKind::OreNode),
                BuildTool::Object(ObjectKind::Mushroom),
                BuildTool::Object(ObjectKind::Herb),
                BuildTool::Object(ObjectKind::Crate),
                BuildTool::Object(ObjectKind::Barrel),
                BuildTool::Object(ObjectKind::Scarecrow),
                BuildTool::Object(ObjectKind::Fence),
                BuildTool::Object(ObjectKind::Lamp),
                BuildTool::Object(ObjectKind::Bench),
                BuildTool::Object(ObjectKind::Stump),
                BuildTool::Object(ObjectKind::Log),
                BuildTool::Object(ObjectKind::Sign),
                BuildTool::Object(ObjectKind::Door),
                BuildTool::Object(ObjectKind::Stairs),
                BuildTool::Object(ObjectKind::CaveEntrance),
                BuildTool::Zone(ZoneKind::Tavern),
                BuildTool::Zone(ZoneKind::Kitchen),
                BuildTool::Zone(ZoneKind::GuestRoom),
                BuildTool::Zone(ZoneKind::Cellar),
                BuildTool::Zone(ZoneKind::Greenhouse),
                BuildTool::Zone(ZoneKind::Field),
                BuildTool::Zone(ZoneKind::Cave),
                BuildTool::Zone(ZoneKind::StaffOnly),
                BuildTool::Zone(ZoneKind::PublicPath),
                BuildTool::Zone(ZoneKind::TavernExterior),
                BuildTool::Zone(ZoneKind::Bar),
                BuildTool::Zone(ZoneKind::CivicLot),
                BuildTool::Zone(ZoneKind::MarketLot),
                BuildTool::Zone(ZoneKind::ResidentialLot),
                BuildTool::Zone(ZoneKind::ArtisanLot),
                BuildTool::Zone(ZoneKind::HarborLot),
                BuildTool::Zone(ZoneKind::AgriculturalLot),
                BuildTool::Transition,
                BuildTool::Erase,
            ],
        }
    }
}
