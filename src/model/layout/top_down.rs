use std::time::Instant;
use crate::model::fs_entry::FsEntry;
use crate::model::layout::{CompoundLayoutAlgorithm, LayoutKind, LayoutNode, LayoutOptions, LayoutResult};

pub struct NativeTopDownLayout;

impl NativeTopDownLayout {
    /// Node Count Size Approximator from Section 3.1.2:
    /// Estimates size based on base dimensions multiplied by square root of child count.
    fn predict_node_size(entry: &FsEntry, options: &LayoutOptions) -> (f32, f32) {
        if !entry.is_dir || entry.children.is_empty() {
            return (options.base_node_width, options.base_node_height);
        }

        let count = entry.children.len().max(1) as f32;
        let count_factor = count.sqrt().max(1.0);
        
        // Multiplier proportional to sqrt(count) with aspect ratio matching base size
        let width = options.base_node_width * (0.8 + 0.6 * count_factor);
        let height = options.base_node_height * (0.8 + 0.6 * count_factor);
        (width, height)
    }

    /// Topdownpacking algorithm from Section 3.1.2:
    /// Arranges N nodes in a balanced grid with aspect ratio preservation and incomplete row handling.
    fn pack_children(
        children: &[FsEntry],
        container_width: f32,
        options: &LayoutOptions,
        depth: usize,
        depth_limit: usize,
        min_scale: &mut f32,
        max_scale: &mut f32,
        total_nodes: &mut usize,
    ) -> (Vec<LayoutNode>, f32, f32) {
        let n = children.len();
        if n == 0 {
            return (Vec::new(), 0.0, 0.0);
        }

        // Calculate columns and rows for a balanced grid
        let target_aspect = (options.base_node_width / options.base_node_height).max(1.0);
        let cols = ((n as f32 * target_aspect).sqrt().ceil() as usize).clamp(1, 8);
        let rows = (n + cols - 1) / cols;

        let mut packed_nodes = Vec::with_capacity(n);
        let gap = options.gap;
        let padding = options.padding;

        // Calculate column width based on container width or defaults
        let cell_width = if container_width > 2.0 * padding + (cols as f32 * options.base_node_width) {
            (container_width - 2.0 * padding - ((cols - 1) as f32 * gap)) / cols as f32
        } else {
            options.base_node_width
        };

        let cell_height = options.base_node_height;
        let mut max_y: f32 = 0.0;
        let mut max_x: f32 = 0.0;

        for (idx, child) in children.iter().enumerate() {
            *total_nodes += 1;
            let row = idx / cols;
            let col = idx % cols;

            // Incomplete row adjustment (Section 3.1.2, Fig 2c):
            // If on the last row and it's incomplete, expand or balance
            let is_last_row = row == rows - 1;
            let items_in_last_row = n - (row * cols);
            let item_width = if is_last_row && items_in_last_row < cols && items_in_last_row > 0 {
                // Distribute remaining width evenly across items in the last row
                let total_row_w = (cols as f32 * cell_width) + ((cols - 1) as f32 * gap);
                (total_row_w - ((items_in_last_row - 1) as f32 * gap)) / items_in_last_row as f32
            } else {
                cell_width
            };

            let x = padding + (col as f32 * (cell_width + gap));
            let y = padding + (row as f32 * (cell_height + gap));

            // Nested child preview layout (Algorithm 2 recursion)
            let mut nested_children = Vec::new();
            let mut scale_factor = 1.0;

            if child.is_dir && !child.children.is_empty() && depth < depth_limit {
                let (predicted_w, _predicted_h) = Self::predict_node_size(child, options);
                let (sub_children, sub_w, sub_h) = Self::pack_children(
                    &child.children,
                    predicted_w,
                    options,
                    depth + 1,
                    depth_limit,
                    min_scale,
                    max_scale,
                    total_nodes,
                );

                if sub_w > 0.0 && sub_h > 0.0 {
                    // Top-Down scale factor s_r = parent_size / layout_size
                    let scale_x = (item_width - 16.0) / sub_w;
                    let scale_y = (cell_height - 24.0) / sub_h;
                    scale_factor = scale_x.min(scale_y).clamp(0.1, 1.0);
                }

                *min_scale = min_scale.min(scale_factor);
                *max_scale = max_scale.max(scale_factor);
                nested_children = sub_children;
            }

            let node = LayoutNode {
                id: format!("{}_{}", child.path.display(), idx),
                path: child.path.clone(),
                name: child.name.clone(),
                is_dir: child.is_dir,
                category: child.category.clone(),
                size_bytes: child.size_bytes,
                item_count: child.item_count,
                x,
                y,
                width: item_width,
                height: cell_height,
                scale: scale_factor,
                depth,
                children: nested_children,
            };

            max_x = max_x.max(x + item_width);
            max_y = max_y.max(y + cell_height);
            packed_nodes.push(node);
        }

        let total_w = max_x + padding;
        let total_h = max_y + padding;
        (packed_nodes, total_w, total_h)
    }
}

impl CompoundLayoutAlgorithm for NativeTopDownLayout {
    fn kind(&self) -> LayoutKind {
        LayoutKind::NativeTopDown
    }

    fn compute_layout(
        &self,
        root: &FsEntry,
        available_width: f32,
        available_height: f32,
        options: &LayoutOptions,
    ) -> LayoutResult {
        let start = Instant::now();
        let mut min_scale = 1.0f32;
        let mut max_scale = 1.0f32;
        let mut node_count = 1;

        let (children, total_w, total_h) = Self::pack_children(
            &root.children,
            available_width,
            options,
            1,
            options.max_preview_depth,
            &mut min_scale,
            &mut max_scale,
            &mut node_count,
        );

        let root_node = LayoutNode {
            id: format!("root_{}", root.path.display()),
            path: root.path.clone(),
            name: root.name.clone(),
            is_dir: root.is_dir,
            category: root.category.clone(),
            size_bytes: root.size_bytes,
            item_count: root.item_count,
            x: 0.0,
            y: 0.0,
            width: total_w.max(available_width),
            height: total_h.max(available_height),
            scale: 1.0,
            depth: 0,
            children,
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        LayoutResult {
            kind: LayoutKind::NativeTopDown,
            root_node,
            total_width: total_w,
            total_height: total_h,
            min_scale,
            max_scale,
            node_count,
            compute_time_ms: elapsed,
        }
    }
}
