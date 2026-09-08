use crate::PixelDocument;
use haven_assets::asset_intake::{
    AssetFootprintRecipe, AssetIntakeCatalog, AssetIntakeRecipe, AssetIntakeTarget,
    AssetIntakeTargetKind, AssetLicenseDeclaration, AssetLicenseStatus, AssetPivot,
    AssetPromotionState, AssetSliceRect,
};
use std::path::Path;

pub const PIXEL_STUDIO_OUTPUT_ROOT: &str = "assets/source/original/pixel_studio";

#[derive(Clone, Debug)]
pub struct PublishResult {
    pub image_path: String,
    pub metadata_path: String,
    pub intake_stable_id: String,
}

pub fn publish_working_copy(
    document: &mut PixelDocument,
    repo_root: impl AsRef<Path>,
    target_kind: AssetIntakeTargetKind,
    target_code: &str,
) -> Result<PublishResult, String> {
    let repo_root = repo_root.as_ref();
    document.save(repo_root)?;
    let mut catalog = AssetIntakeCatalog::load_default()?;
    let selection = document.metadata.selection;
    let stable_id = format!(
        "import/{}_{}_{}_{}x{}",
        document
            .metadata
            .asset_id
            .trim_start_matches("pixel/")
            .replace('/', "_"),
        selection.x,
        selection.y,
        selection.width.max(1),
        selection.height.max(1)
    );
    let status = match document.metadata.license.status.as_str() {
        "cc0" => AssetLicenseStatus::Cc0,
        "project_owned" => AssetLicenseStatus::ProjectOwned,
        "cc_by" => AssetLicenseStatus::CcBy,
        _ => AssetLicenseStatus::Unverified,
    };
    let recipe = AssetIntakeRecipe {
        stable_id: stable_id.clone(),
        display_name: format!(
            "{} [{}x{} @ {},{}]",
            document.metadata.display_name,
            selection.width.max(1),
            selection.height.max(1),
            selection.x,
            selection.y
        ),
        source_path: document.metadata.output_path.clone(),
        target: AssetIntakeTarget {
            kind: target_kind,
            code: target_code.to_string(),
        },
        slice: AssetSliceRect {
            x: selection.x,
            y: selection.y,
            width: selection.width.max(1),
            height: selection.height.max(1),
        },
        pivot: AssetPivot {
            x: document.metadata.pivot[0] - selection.x as i32,
            y: document.metadata.pivot[1] - selection.y as i32,
        },
        footprint: AssetFootprintRecipe {
            visual: document.metadata.visual_footprint,
            collision: document.metadata.collision_footprint,
            interaction: document.metadata.interaction_footprint,
        },
        license: AssetLicenseDeclaration {
            status,
            attribution: document.metadata.license.source_name.clone(),
            source_url: document.metadata.license.source_url.clone(),
            accepted: status.permits_promotion(),
        },
        promotion_state: AssetPromotionState::Draft,
    };
    if let Some(existing) = catalog
        .recipes
        .iter_mut()
        .find(|candidate| candidate.stable_id == stable_id)
    {
        *existing = recipe;
    } else {
        catalog.recipes.push(recipe);
        catalog
            .recipes
            .sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    }
    catalog.save_default()?;
    Ok(PublishResult {
        image_path: document.metadata.output_path.clone(),
        metadata_path: document.metadata.sidecar_path(),
        intake_stable_id: stable_id,
    })
}
