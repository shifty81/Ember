//! Deterministic resolver for Havenwild Terrain Standard v1 four-corner tuples.
//!
//! Semantic terrain remains gameplay/collision authority. This resolver only
//! selects the exact authored visual tile for a `(top-left, top-right,
//! bottom-left, bottom-right)` signature.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::Deserialize;

pub const HAVENWILD_TERRAIN_TUPLE_CATALOG_PATH: &str =
    "content/terrain/havenwild_terrain_tuple_catalog_v1.json";
pub const HAVENWILD_TERRAIN_TUPLE_CATALOG_SCHEMA: &str = "havenwild.terrain_tuple_catalog.v1";

const EMBEDDED_TUPLE_CATALOG: &str =
    include_str!("../../../content/terrain/havenwild_terrain_tuple_catalog_v1.json");

static EMBEDDED_RESOLVER: OnceLock<Result<TerrainTupleResolver, String>> = OnceLock::new();

/// Process-wide cached resolver used by runtime and editor inspection.
pub fn embedded_terrain_tuple_resolver() -> Result<&'static TerrainTupleResolver, &'static str> {
    match EMBEDDED_RESOLVER.get_or_init(TerrainTupleResolver::embedded) {
        Ok(resolver) => Ok(resolver),
        Err(error) => Err(error.as_str()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerrainCornerTuple {
    pub top_left: Option<u16>,
    pub top_right: Option<u16>,
    pub bottom_left: Option<u16>,
    pub bottom_right: Option<u16>,
}

impl TerrainCornerTuple {
    pub const fn new(
        top_left: Option<u16>,
        top_right: Option<u16>,
        bottom_left: Option<u16>,
        bottom_right: Option<u16>,
    ) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    pub const fn uniform(terrain_ordinal: u16) -> Self {
        Self::new(
            Some(terrain_ordinal),
            Some(terrain_ordinal),
            Some(terrain_ordinal),
            Some(terrain_ordinal),
        )
    }

    pub fn signature(self) -> String {
        [
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        ]
        .into_iter()
        .map(|corner| corner.map_or_else(|| "_".to_string(), |value| value.to_string()))
        .collect::<Vec<_>>()
        .join(",")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainTupleResolutionStatus {
    Exact,
    ExactDuplicateCanonical,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTupleResolution {
    pub tuple: TerrainCornerTuple,
    pub signature: String,
    pub selected_tile_id: Option<u32>,
    pub duplicate_alternatives: Vec<u32>,
    pub status: TerrainTupleResolutionStatus,
}

impl TerrainTupleResolution {
    pub fn is_resolved(&self) -> bool {
        self.selected_tile_id.is_some()
    }

    pub fn status_line(&self) -> String {
        match self.selected_tile_id {
            Some(tile_id) if self.duplicate_alternatives.is_empty() => {
                format!("tuple {} -> tile {} (exact)", self.signature, tile_id)
            }
            Some(tile_id) => format!(
                "tuple {} -> tile {} (canonical duplicate; alternatives {:?})",
                self.signature, tile_id, self.duplicate_alternatives
            ),
            None => format!("tuple {} unresolved", self.signature),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTupleCatalogSummary {
    pub declared_tile_count: usize,
    pub mapped_tuple_count: usize,
    pub unique_signature_count: usize,
    pub duplicate_signature_count: usize,
    pub terrain_family_count: usize,
}

#[derive(Clone, Debug)]
pub struct TerrainTupleResolver {
    signature_to_tile_id: BTreeMap<String, u32>,
    duplicates: BTreeMap<String, Vec<u32>>,
    terrain_ordinal_to_code: BTreeMap<u16, String>,
    supported_material_pairs: BTreeSet<(u16, u16)>,
    summary: TerrainTupleCatalogSummary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerrainTupleCatalogDocument {
    schema: String,
    version: u32,
    declared_tile_count: usize,
    mapped_tuple_count: usize,
    unique_signature_count: usize,
    duplicate_signature_count: usize,
    terrain_ordinal_to_code: BTreeMap<String, String>,
    signature_to_tile_id: BTreeMap<String, u32>,
    duplicates: BTreeMap<String, Vec<u32>>,
}

fn ordered_material_pair(first: u16, second: u16) -> (u16, u16) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn collect_supported_material_pairs(
    signatures: &BTreeMap<String, u32>,
) -> Result<BTreeSet<(u16, u16)>, String> {
    let mut pairs = BTreeSet::new();
    for signature in signatures.keys() {
        let mut ordinals = Vec::with_capacity(4);
        for token in signature.split(',') {
            if token == "_" {
                continue;
            }
            let ordinal = token.parse::<u16>().map_err(|error| {
                format!("invalid terrain tuple signature {signature}: {error}")
            })?;
            if !ordinals.contains(&ordinal) {
                ordinals.push(ordinal);
            }
        }
        for first_index in 0..ordinals.len() {
            for second_index in first_index + 1..ordinals.len() {
                pairs.insert(ordered_material_pair(
                    ordinals[first_index],
                    ordinals[second_index],
                ));
            }
        }
    }
    Ok(pairs)
}

impl TerrainTupleResolver {
    pub fn embedded() -> Result<Self, String> {
        Self::from_json(EMBEDDED_TUPLE_CATALOG)
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let document: TerrainTupleCatalogDocument =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        if document.schema != HAVENWILD_TERRAIN_TUPLE_CATALOG_SCHEMA {
            return Err(format!(
                "unsupported terrain tuple schema {}",
                document.schema
            ));
        }
        if document.version != 1 {
            return Err(format!(
                "unsupported terrain tuple version {}",
                document.version
            ));
        }

        let terrain_ordinal_to_code = document
            .terrain_ordinal_to_code
            .into_iter()
            .map(|(ordinal, code)| {
                ordinal
                    .parse::<u16>()
                    .map(|ordinal| (ordinal, code))
                    .map_err(|error| format!("invalid terrain ordinal {ordinal}: {error}"))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        if document.signature_to_tile_id.len() != document.unique_signature_count {
            return Err("tuple catalog unique signature count does not match index".to_string());
        }
        if document.duplicates.len() != document.duplicate_signature_count {
            return Err("tuple catalog duplicate signature count does not match index".to_string());
        }
        let supported_material_pairs =
            collect_supported_material_pairs(&document.signature_to_tile_id)?;

        Ok(Self {
            summary: TerrainTupleCatalogSummary {
                declared_tile_count: document.declared_tile_count,
                mapped_tuple_count: document.mapped_tuple_count,
                unique_signature_count: document.unique_signature_count,
                duplicate_signature_count: document.duplicate_signature_count,
                terrain_family_count: terrain_ordinal_to_code.len(),
            },
            signature_to_tile_id: document.signature_to_tile_id,
            duplicates: document.duplicates,
            terrain_ordinal_to_code,
            supported_material_pairs,
        })
    }

    pub fn summary(&self) -> &TerrainTupleCatalogSummary {
        &self.summary
    }

    pub fn terrain_code(&self, ordinal: u16) -> Option<&str> {
        self.terrain_ordinal_to_code
            .get(&ordinal)
            .map(String::as_str)
    }

    /// Reports whether the authored tuple catalog demonstrates a valid
    /// contact between two terrain families. This is broader than exact
    /// four-corner resolution: a three-way junction can be visually valid
    /// even when no single atlas cell encodes that exact signature.
    pub fn supports_material_pair(&self, first: u16, second: u16) -> bool {
        first == second
            || self
                .supported_material_pairs
                .contains(&ordered_material_pair(first, second))
    }

    pub fn resolve(&self, tuple: TerrainCornerTuple) -> TerrainTupleResolution {
        let signature = tuple.signature();
        let selected_tile_id = self.signature_to_tile_id.get(&signature).copied();
        let all_duplicate_ids = self.duplicates.get(&signature).cloned().unwrap_or_default();
        let duplicate_alternatives = selected_tile_id.map_or_else(Vec::new, |canonical| {
            all_duplicate_ids
                .into_iter()
                .filter(|tile_id| *tile_id != canonical)
                .collect()
        });
        let status = match selected_tile_id {
            Some(_) if duplicate_alternatives.is_empty() => TerrainTupleResolutionStatus::Exact,
            Some(_) => TerrainTupleResolutionStatus::ExactDuplicateCanonical,
            None => TerrainTupleResolutionStatus::Unresolved,
        };
        TerrainTupleResolution {
            tuple,
            signature,
            selected_tile_id,
            duplicate_alternatives,
            status,
        }
    }

    pub fn audit<'a>(
        &self,
        tuples: impl IntoIterator<Item = &'a TerrainCornerTuple>,
    ) -> TerrainTupleAuditReport {
        let mut report = TerrainTupleAuditReport::default();
        for tuple in tuples {
            let resolution = self.resolve(*tuple);
            report.total += 1;
            match resolution.status {
                TerrainTupleResolutionStatus::Exact => report.exact += 1,
                TerrainTupleResolutionStatus::ExactDuplicateCanonical => {
                    report.duplicate_canonical += 1
                }
                TerrainTupleResolutionStatus::Unresolved => {
                    report.unresolved += 1;
                    report.unresolved_signatures.push(resolution.signature);
                }
            }
        }
        report.unresolved_signatures.sort();
        report.unresolved_signatures.dedup();
        report
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TerrainTupleAuditReport {
    pub total: usize,
    pub exact: usize,
    pub duplicate_canonical: usize,
    pub unresolved: usize,
    pub unresolved_signatures: Vec<String>,
}

impl TerrainTupleAuditReport {
    pub fn status_line(&self) -> String {
        format!(
            "terrain tuple audit: {} total, {} exact, {} duplicate canonical, {} unresolved",
            self.total, self.exact, self.duplicate_canonical, self.unresolved
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver() -> TerrainTupleResolver {
        TerrainTupleResolver::embedded().expect("embedded terrain tuple catalog")
    }

    #[test]
    fn embedded_catalog_matches_promoted_standard_counts() {
        let resolver = resolver();
        assert_eq!(resolver.summary().terrain_family_count, 34);
        assert_eq!(resolver.summary().unique_signature_count, 15_562);
        assert_eq!(resolver.summary().duplicate_signature_count, 31);
    }

    #[test]
    fn uniform_water_and_grass_are_exact() {
        let resolver = resolver();
        let water = resolver.resolve(TerrainCornerTuple::uniform(28));
        let grass = resolver.resolve(TerrainCornerTuple::uniform(5));
        assert_eq!(water.selected_tile_id, Some(28));
        assert_eq!(grass.selected_tile_id, Some(5));
        assert!(water.is_resolved());
        assert!(grass.is_resolved());
    }

    #[test]
    fn unresolved_tuple_never_silently_substitutes_an_unrelated_tile() {
        let resolver = resolver();
        let unresolved = resolver.resolve(TerrainCornerTuple::new(
            Some(u16::MAX),
            Some(u16::MAX),
            None,
            None,
        ));
        assert_eq!(unresolved.selected_tile_id, None);
        assert_eq!(unresolved.status, TerrainTupleResolutionStatus::Unresolved);
    }

    #[test]
    fn duplicate_signatures_select_the_catalog_canonical_tile() {
        let resolver = resolver();
        let (signature, ids) = resolver
            .duplicates
            .iter()
            .next()
            .expect("catalog duplicate signature");
        let corners = signature
            .split(',')
            .map(|part| {
                if part == "_" {
                    None
                } else {
                    part.parse::<u16>().ok()
                }
            })
            .collect::<Vec<_>>();
        let resolution = resolver.resolve(TerrainCornerTuple::new(
            corners[0], corners[1], corners[2], corners[3],
        ));
        assert_eq!(resolution.selected_tile_id, ids.first().copied());
        assert_eq!(
            resolution.status,
            TerrainTupleResolutionStatus::ExactDuplicateCanonical
        );
    }

    #[test]
    fn chunk_edge_cells_resolve_identically_when_their_corner_tuple_matches() {
        let resolver = resolver();
        let left_chunk_edge = TerrainCornerTuple::new(Some(5), Some(28), Some(5), Some(28));
        let right_chunk_edge = TerrainCornerTuple::new(Some(5), Some(28), Some(5), Some(28));
        assert_eq!(
            resolver.resolve(left_chunk_edge).selected_tile_id,
            resolver.resolve(right_chunk_edge).selected_tile_id
        );
    }

    #[test]
    fn wrapped_world_seam_uses_the_same_tuple_contract_as_an_internal_edge() {
        let resolver = resolver();
        let internal = TerrainCornerTuple::new(Some(28), Some(22), Some(28), Some(22));
        let wrapped = TerrainCornerTuple::new(Some(28), Some(22), Some(28), Some(22));
        assert_eq!(
            resolver.resolve(internal).selected_tile_id,
            resolver.resolve(wrapped).selected_tile_id
        );
    }
    #[test]
    fn catalog_pair_support_accepts_valid_multi_material_junction_contacts() {
        let resolver = resolver();
        assert!(resolver.supports_material_pair(26, 22));
        assert!(resolver.supports_material_pair(26, 5));
        assert!(resolver.supports_material_pair(22, 5));
        assert!(!resolver.supports_material_pair(14, 5));
    }

}
