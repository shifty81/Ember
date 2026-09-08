mod document;
mod publish;
mod types;

pub use document::AnimationDocument;
pub use publish::{
    AnimationPublishResult, ANIMATION_STUDIO_OUTPUT_ROOT, GENERATED_ANIMATION_IMAGE_ROOT,
    RUNTIME_ANIMATION_CATALOG_PATH, RUNTIME_ANIMATION_ROOT,
};
pub use types::{
    AnimationClip, AnimationDirection, AnimationDocumentMetadata, AnimationEvent,
    AnimationEventKind, AnimationFrame, AnimationLoopMode, AnimationSocket, AnimationSocketKind,
};
