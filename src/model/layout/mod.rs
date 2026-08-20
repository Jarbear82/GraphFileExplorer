pub mod fcose;
pub mod radial;
pub mod sugiyama;
pub mod top_down;

use std::path::PathBuf;
use crate::model::fs_entry::{FileCategory, FsEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutKind {
    #[default]
    NativeTopDown,
    RadialBalloonTree,
    SugiyamaLayered,
    FCoSE,
}

impl LayoutKind {
    pub fn name(&self) -> &'static str {
        match self {
            LayoutKind::NativeTopDown => "Native Top-Down",
            LayoutKind::RadialBalloonTree => "Radial Balloon Tree",
            LayoutKind::SugiyamaLayered => "Sugiyama Layered",
            LayoutKind::FCoSE => "FCoSE Compound Force",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            LayoutKind::NativeTopDown => "Algorithm 2: Topdownpacking with node count size approximation",
            LayoutKind::RadialBalloonTree => "Concentric radial balloon tree layout with compound sub-orbits (Paper Appendix)",
            LayoutKind::SugiyamaLayered => "Hierarchical layered directed compound layout (Planned)",
            LayoutKind::FCoSE => "Fast Compound Spring Embedder force layout (Planned)",
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, LayoutKind::NativeTopDown | LayoutKind::RadialBalloonTree)
    }

    pub fn all() -> &'static [LayoutKind] {
        &[
            LayoutKind::NativeTopDown,
            LayoutKind::RadialBalloonTree,
            LayoutKind::SugiyamaLayered,
            LayoutKind::FCoSE,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub category: FileCategory,
    pub size_bytes: u64,
    pub item_count: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub depth: usize,
    pub children: Vec<LayoutNode>,
}

#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub base_node_width: f32,
    pub base_node_height: f32,
    pub padding: f32,
    pub gap: f32,
    pub max_preview_depth: usize,
    pub show_hidden: bool,
    pub expanded_paths: std::collections::HashSet<PathBuf>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            base_node_width: 140.0,
            base_node_height: 90.0,
            padding: 16.0,
            gap: 12.0,
            max_preview_depth: 10,
            show_hidden: false,
            expanded_paths: std::collections::HashSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub kind: LayoutKind,
    pub root_node: LayoutNode,
    pub total_width: f32,
    pub total_height: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub node_count: usize,
    pub compute_time_ms: f64,
}

pub trait CompoundLayoutAlgorithm: Send + Sync {
    fn kind(&self) -> LayoutKind;
    fn compute_layout(
        &self,
        root: &FsEntry,
        available_width: f32,
        available_height: f32,
        options: &LayoutOptions,
    ) -> LayoutResult;
}

pub fn create_layout_engine(kind: LayoutKind) -> Box<dyn CompoundLayoutAlgorithm> {
    match kind {
        LayoutKind::NativeTopDown => Box::new(top_down::NativeTopDownLayout),
        LayoutKind::RadialBalloonTree => Box::new(radial::RadialBalloonTreeLayout),
        LayoutKind::SugiyamaLayered => Box::new(sugiyama::SugiyamaLayeredLayout),
        LayoutKind::FCoSE => Box::new(fcose::FCoSELayout),
    }
}
