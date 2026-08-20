use crate::model::fs_entry::FsEntry;
use crate::model::layout::{CompoundLayoutAlgorithm, LayoutKind, LayoutOptions, LayoutResult};
use crate::model::layout::top_down::NativeTopDownLayout;

/// Planned FCoSE (Fast Compound Spring Embedder) Layout
/// (Force-directed layout with compound cluster constraints and spectral placement)
pub struct FCoSELayout;

impl CompoundLayoutAlgorithm for FCoSELayout {
    fn kind(&self) -> LayoutKind {
        LayoutKind::FCoSE
    }

    fn compute_layout(
        &self,
        root: &FsEntry,
        available_width: f32,
        available_height: f32,
        options: &LayoutOptions,
    ) -> LayoutResult {
        // Fallback to Native Top-Down with marker until fully implemented
        let mut result = NativeTopDownLayout.compute_layout(root, available_width, available_height, options);
        result.kind = LayoutKind::FCoSE;
        result
    }
}
