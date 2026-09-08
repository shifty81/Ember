use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

pub const HARBOR_ROUTE_CATALOG_PATH: &str = "content/worldgen/harbor_routes_v0_1.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarborRouteCatalog {
    pub schema: String,
    pub routes: Vec<HarborRouteSpec>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HarborRouteSpec {
    pub from_landmass_id: i32,
    pub to_landmass_id: i32,
}

impl Default for HarborRouteCatalog {
    fn default() -> Self {
        Self {
            schema: "havenwild.harbor_routes.v001".to_string(),
            routes: Vec::new(),
        }
    }
}

impl HarborRouteCatalog {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize {path}: {error}"))?;
        std::fs::write(path, data).map_err(|error| format!("failed to write {path}: {error}"))
    }

    pub fn connect(&mut self, from_landmass_id: i32, to_landmass_id: i32) -> bool {
        if from_landmass_id == to_landmass_id {
            return false;
        }
        let (from_landmass_id, to_landmass_id) = ordered_pair(from_landmass_id, to_landmass_id);
        if self.routes.iter().any(|route| {
            route.from_landmass_id == from_landmass_id && route.to_landmass_id == to_landmass_id
        }) {
            return false;
        }
        self.routes.push(HarborRouteSpec {
            from_landmass_id,
            to_landmass_id,
        });
        self.routes
            .sort_by_key(|route| (route.from_landmass_id, route.to_landmass_id));
        true
    }

    pub fn validate(&self, known_landmass_ids: &[i32]) -> Vec<String> {
        let mut warnings = Vec::new();
        for route in &self.routes {
            if route.from_landmass_id == route.to_landmass_id {
                warnings.push(format!(
                    "harbor route {} -> {} connects an island to itself",
                    route.from_landmass_id, route.to_landmass_id
                ));
            }
            for id in [route.from_landmass_id, route.to_landmass_id] {
                if !known_landmass_ids.contains(&id) {
                    warnings.push(format!("harbor route references unknown landmass {id}"));
                }
            }
        }
        warnings
    }
}

fn ordered_pair(left: i32, right: i32) -> (i32, i32) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harbor_route_pairs_are_unique_and_order_independent() {
        let mut catalog = HarborRouteCatalog::default();
        assert!(catalog.connect(4, 2));
        assert!(!catalog.connect(2, 4));
        assert_eq!(catalog.routes.len(), 1);
        assert_eq!(catalog.routes[0].from_landmass_id, 2);
        assert_eq!(catalog.routes[0].to_landmass_id, 4);
    }
}
