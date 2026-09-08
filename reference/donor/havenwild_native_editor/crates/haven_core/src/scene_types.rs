#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneId {
    Farmstead,
    TavernInterior,
    Cellar,
    GuestFloor,
    NorthRoad,
    SouthField,
    EastWoods,
    CaveMouth,
    CaveDepths,
}

impl SceneId {
    pub const ALL: [SceneId; 9] = [
        SceneId::Farmstead,
        SceneId::TavernInterior,
        SceneId::Cellar,
        SceneId::GuestFloor,
        SceneId::NorthRoad,
        SceneId::SouthField,
        SceneId::EastWoods,
        SceneId::CaveMouth,
        SceneId::CaveDepths,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SceneId::Farmstead => "Estate",
            SceneId::TavernInterior => "Tavern Interior",
            SceneId::Cellar => "Cellar",
            SceneId::GuestFloor => "Guest Floor",
            SceneId::NorthRoad => "North Road",
            SceneId::SouthField => "South Field",
            SceneId::EastWoods => "East Woods",
            SceneId::CaveMouth => "Cave Mouth",
            SceneId::CaveDepths => "Cave Depths",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SceneId::Farmstead => "farmstead",
            SceneId::TavernInterior => "tavern_interior",
            SceneId::Cellar => "cellar",
            SceneId::GuestFloor => "guest_floor",
            SceneId::NorthRoad => "north_road",
            SceneId::SouthField => "south_field",
            SceneId::EastWoods => "east_woods",
            SceneId::CaveMouth => "cave_mouth",
            SceneId::CaveDepths => "cave_depths",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "farmstead" => Some(SceneId::Farmstead),
            "tavern_interior" => Some(SceneId::TavernInterior),
            "cellar" => Some(SceneId::Cellar),
            "guest_floor" => Some(SceneId::GuestFloor),
            "north_road" => Some(SceneId::NorthRoad),
            "south_field" => Some(SceneId::SouthField),
            "east_woods" => Some(SceneId::EastWoods),
            "cave_mouth" => Some(SceneId::CaveMouth),
            "cave_depths" => Some(SceneId::CaveDepths),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Exterior,
    Interior,
    Cave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneBiome {
    Temperate,
    Coastal,
    Highlands,
    Cave,
}

impl SceneBiome {
    pub fn label(self) -> &'static str {
        match self {
            SceneBiome::Temperate => "Temperate",
            SceneBiome::Coastal => "Coastal",
            SceneBiome::Highlands => "Highlands",
            SceneBiome::Cave => "Cave",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            SceneBiome::Temperate => "temperate",
            SceneBiome::Coastal => "coastal",
            SceneBiome::Highlands => "highlands",
            SceneBiome::Cave => "cave",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "temperate" => Some(SceneBiome::Temperate),
            "coastal" => Some(SceneBiome::Coastal),
            "highlands" => Some(SceneBiome::Highlands),
            "cave" => Some(SceneBiome::Cave),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneKind {
    None,
    Tavern,
    Kitchen,
    GuestRoom,
    Cellar,
    Greenhouse,
    Field,
    Cave,
    StaffOnly,
    PublicPath,
    TavernExterior,
    Bar,
    CivicLot,
    MarketLot,
    ResidentialLot,
    ArtisanLot,
    HarborLot,
    AgriculturalLot,
}

impl ZoneKind {
    pub fn label(self) -> &'static str {
        match self {
            ZoneKind::None => "None",
            ZoneKind::Tavern => "Tavern",
            ZoneKind::Kitchen => "Kitchen",
            ZoneKind::GuestRoom => "Guest Room",
            ZoneKind::Cellar => "Cellar",
            ZoneKind::Greenhouse => "Greenhouse",
            ZoneKind::Field => "Field",
            ZoneKind::Cave => "Cave",
            ZoneKind::StaffOnly => "Staff Only",
            ZoneKind::PublicPath => "Public Path",
            ZoneKind::TavernExterior => "Tavern Exterior",
            ZoneKind::Bar => "Bar",
            ZoneKind::CivicLot => "Civic Lot",
            ZoneKind::MarketLot => "Market Lot",
            ZoneKind::ResidentialLot => "Residential Lot",
            ZoneKind::ArtisanLot => "Artisan Lot",
            ZoneKind::HarborLot => "Harbor Lot",
            ZoneKind::AgriculturalLot => "Agricultural Lot",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            ZoneKind::None => "none",
            ZoneKind::Tavern => "tavern",
            ZoneKind::Kitchen => "kitchen",
            ZoneKind::GuestRoom => "guest_room",
            ZoneKind::Cellar => "cellar",
            ZoneKind::Greenhouse => "greenhouse",
            ZoneKind::Field => "field",
            ZoneKind::Cave => "cave",
            ZoneKind::StaffOnly => "staff_only",
            ZoneKind::PublicPath => "public_path",
            ZoneKind::TavernExterior => "tavern_exterior",
            ZoneKind::Bar => "bar",
            ZoneKind::CivicLot => "civic_lot",
            ZoneKind::MarketLot => "market_lot",
            ZoneKind::ResidentialLot => "residential_lot",
            ZoneKind::ArtisanLot => "artisan_lot",
            ZoneKind::HarborLot => "harbor_lot",
            ZoneKind::AgriculturalLot => "agricultural_lot",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "none" => Some(ZoneKind::None),
            "tavern" => Some(ZoneKind::Tavern),
            "kitchen" => Some(ZoneKind::Kitchen),
            "guest_room" => Some(ZoneKind::GuestRoom),
            "cellar" => Some(ZoneKind::Cellar),
            "greenhouse" => Some(ZoneKind::Greenhouse),
            "field" => Some(ZoneKind::Field),
            "cave" => Some(ZoneKind::Cave),
            "staff_only" => Some(ZoneKind::StaffOnly),
            "public_path" => Some(ZoneKind::PublicPath),
            "tavern_exterior" => Some(ZoneKind::TavernExterior),
            "bar" => Some(ZoneKind::Bar),
            "civic_lot" => Some(ZoneKind::CivicLot),
            "market_lot" => Some(ZoneKind::MarketLot),
            "residential_lot" => Some(ZoneKind::ResidentialLot),
            "artisan_lot" => Some(ZoneKind::ArtisanLot),
            "harbor_lot" => Some(ZoneKind::HarborLot),
            "agricultural_lot" => Some(ZoneKind::AgriculturalLot),
            _ => None,
        }
    }
}

impl SceneKind {
    pub fn code(self) -> &'static str {
        match self {
            SceneKind::Exterior => "exterior",
            SceneKind::Interior => "interior",
            SceneKind::Cave => "cave",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "exterior" => Some(SceneKind::Exterior),
            "interior" => Some(SceneKind::Interior),
            "cave" => Some(SceneKind::Cave),
            _ => None,
        }
    }
}
