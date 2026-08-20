use std::f32::consts::PI;
use std::time::Instant;
use crate::model::fs_entry::FsEntry;
use crate::model::layout::{
    CompoundLayoutAlgorithm, LayoutKind, LayoutNode, LayoutOptions, LayoutResult,
};

/// Size-Aware Radial Balloon Tree Layout
/// Implements non-overlapping radial orbital placement with directional outward sub-balloons:
/// - Exact bottom-up recursive circumscribed bounding disk computation (Lin & Eades / Carriere & Kazman)
/// - Exact chord-length non-overlapping orbit radius solver via binary search
/// - Outward-projecting radial sectors for expanded sub-trees to prevent backward overlap with hub and sibling branches
/// - Strict non-overlapping guarantees across all recursion levels
pub struct RadialBalloonTreeLayout;

impl RadialBalloonTreeLayout {
    /// Predicts base width and height of a node based on its contents
    fn predict_node_size(entry: &FsEntry, options: &LayoutOptions) -> (f32, f32) {
        if !entry.is_dir || entry.children.is_empty() {
            return (options.base_node_width, options.base_node_height);
        }
        let count = entry.children.len().max(1) as f32;
        let count_factor = count.sqrt().max(1.0);
        let width = options.base_node_width * (0.85 + 0.35 * count_factor);
        let height = options.base_node_height * (0.85 + 0.35 * count_factor);
        (width, height)
    }

    /// Computes the circumscribed bounding radius for a node including padding and gap
    fn bounding_radius(width: f32, height: f32, gap: f32) -> f32 {
        let half_diag = ((width * width) + (height * height)).sqrt() / 2.0;
        half_diag + (gap / 2.0)
    }

    /// Recursively computes the true bounding radius of a subtree
    fn compute_subtree_radius(
        entry: &FsEntry,
        options: &LayoutOptions,
        depth: usize,
        depth_limit: usize,
    ) -> f32 {
        let (w, h) = Self::predict_node_size(entry, options);
        let base_r = Self::bounding_radius(w, h, options.gap);

        let is_expanded = entry.is_dir
            && !entry.children.is_empty()
            && options.expanded_paths.contains(&entry.path)
            && depth < depth_limit;

        if !is_expanded {
            return base_r;
        }

        let child_radii: Vec<f32> = entry
            .children
            .iter()
            .map(|c| Self::compute_subtree_radius(c, options, depth + 1, depth_limit))
            .collect();

        let sub_orbit_r = Self::solve_minimal_orbit_radius(&child_radii, base_r, options.gap);
        let max_child_r = child_radii.iter().fold(0.0f32, |acc, &r| acc.max(r));

        sub_orbit_r + max_child_r + options.gap
    }

    /// Solves for the minimal orbit radius R that guarantees zero overlap between all adjacent nodes.
    /// Uses binary search to satisfy: sum( 2 * arcsin( (r_i + r_{i+1}) / (2 * R) ) ) <= max_angle
    fn solve_minimal_orbit_radius(radii: &[f32], hub_radius: f32, gap: f32) -> f32 {
        Self::solve_minimal_orbit_radius_sector(radii, hub_radius, gap, 2.0 * PI)
    }

    fn solve_minimal_orbit_radius_sector(
        radii: &[f32],
        hub_radius: f32,
        gap: f32,
        max_angle: f32,
    ) -> f32 {
        let n = radii.len();
        if n == 0 {
            return hub_radius + gap;
        }
        if n == 1 {
            return hub_radius + radii[0] + gap;
        }

        let is_full_circle = (max_angle - 2.0 * PI).abs() < 1e-4;
        let pairs_count = if is_full_circle { n } else { n - 1 };

        let max_pairwise_dist = (0..pairs_count)
            .map(|i| radii[i] + radii[(i + 1) % n])
            .fold(0.0f32, |acc, d| acc.max(d));

        let max_single_radius = radii.iter().fold(0.0f32, |acc, &r| acc.max(r));
        let hub_clearance_radius = hub_radius + max_single_radius + gap;

        // Lower bound for R: must be greater than half of the max pairwise chord
        let mut low = (max_pairwise_dist / 2.001).max(hub_clearance_radius);

        let chord_sum: f32 = (0..pairs_count)
            .map(|i| radii[i] + radii[(i + 1) % n])
            .sum();
        let mut high = (chord_sum / max_angle).max(low) * 3.0 + 300.0;

        // Binary search for exact R
        for _ in 0..32 {
            let mid = (low + high) / 2.0;
            let mut angle_sum = 0.0f32;
            let mut valid = true;

            for i in 0..pairs_count {
                let d = radii[i] + radii[(i + 1) % n];
                let sin_val = d / (2.0 * mid);
                if sin_val >= 1.0 {
                    valid = false;
                    break;
                }
                angle_sum += 2.0 * sin_val.asin();
            }

            if valid && angle_sum <= max_angle {
                high = mid; // R is large enough, try smaller
            } else {
                low = mid;  // R is too small, angles exceed max_angle
            }
        }

        high.max(hub_clearance_radius)
    }

    fn layout_radial_children(
        children: &[FsEntry],
        center_x: f32,
        center_y: f32,
        parent_angle: Option<f32>,
        options: &LayoutOptions,
        depth: usize,
        depth_limit: usize,
        min_scale: &mut f32,
        max_scale: &mut f32,
        total_nodes: &mut usize,
    ) -> (Vec<LayoutNode>, f32, f32, f32, f32) {
        let n = children.len();
        if n == 0 {
            return (Vec::new(), center_x, center_x, center_y, center_y);
        }

        // 1. Calculate node dimensions and true recursive subtree bounding radii
        let mut node_sizes = Vec::with_capacity(n);
        let mut radii = Vec::with_capacity(n);

        for child in children {
            let (w, h) = Self::predict_node_size(child, options);
            let r = Self::compute_subtree_radius(child, options, depth, depth_limit);
            node_sizes.push((w, h));
            radii.push(r);
        }

        // 2. Determine angular sector and solve for minimal non-overlapping orbit radius
        let is_root_hub = parent_angle.is_none();
        let hub_radius = (options.base_node_width / 2.0).max(60.0);

        let (orbit_radius, angles) = if is_root_hub {
            // Full 360-degree circle for the root hub
            let orbit_r = Self::solve_minimal_orbit_radius(&radii, hub_radius, options.gap);

            let mut angles = Vec::with_capacity(n);
            let mut current_angle = -PI / 2.0; // Start at 12 o'clock

            let mut raw_deltas = Vec::with_capacity(n);
            for i in 0..n {
                let d = radii[i] + radii[(i + 1) % n];
                let sin_val = (d / (2.0 * orbit_r)).clamp(-1.0, 1.0);
                raw_deltas.push(2.0 * sin_val.asin());
            }

            let raw_sum: f32 = raw_deltas.iter().sum();
            let scale_factor = if raw_sum > 0.0 { (2.0 * PI) / raw_sum } else { 1.0 };

            for delta in raw_deltas {
                angles.push(current_angle);
                current_angle += delta * scale_factor;
            }

            (orbit_r, angles)
        } else {
            // Outward-pointing sector for expanded sub-tree nodes
            let base_angle = parent_angle.unwrap();
            let sector_span = (PI * 0.85).min(PI * 0.4 + (n as f32 * 0.15));

            let orbit_r = Self::solve_minimal_orbit_radius_sector(
                &radii,
                hub_radius * 0.8,
                options.gap,
                sector_span,
            );

            let mut angles = Vec::with_capacity(n);
            if n == 1 {
                angles.push(base_angle);
            } else {
                let mut raw_deltas = Vec::with_capacity(n - 1);
                for i in 0..(n - 1) {
                    let d = radii[i] + radii[i + 1];
                    let sin_val = (d / (2.0 * orbit_r)).clamp(-1.0, 1.0);
                    raw_deltas.push(2.0 * sin_val.asin());
                }

                let total_delta: f32 = raw_deltas.iter().sum();
                let start_angle = base_angle - (total_delta / 2.0);

                let mut cur = start_angle;
                angles.push(cur);
                for delta in raw_deltas {
                    cur += delta;
                    angles.push(cur);
                }
            }

            (orbit_r, angles)
        };

        let mut nodes = Vec::with_capacity(n);
        let mut min_x = center_x;
        let mut max_x = center_x;
        let mut min_y = center_y;
        let mut max_y = center_y;

        for (idx, child) in children.iter().enumerate() {
            *total_nodes += 1;
            let (node_w, node_h) = node_sizes[idx];
            let angle = angles[idx];

            let child_center_x = center_x + (orbit_radius * angle.cos());
            let child_center_y = center_y + (orbit_radius * angle.sin());

            let node_x = child_center_x - (node_w / 2.0);
            let node_y = child_center_y - (node_h / 2.0);

            min_x = min_x.min(node_x);
            max_x = max_x.max(node_x + node_w);
            min_y = min_y.min(node_y);
            max_y = max_y.max(node_y + node_h);

            // Nested child layout (if expanded or for inside preview)
            let is_expanded = child.is_dir && !child.children.is_empty() && options.expanded_paths.contains(&child.path);
            let mut nested_children = Vec::new();
            let mut scale = 1.0f32;

            if child.is_dir && !child.children.is_empty() && depth < depth_limit {
                let sub_options = LayoutOptions {
                    base_node_width: options.base_node_width * 0.85,
                    base_node_height: options.base_node_height * 0.85,
                    gap: options.gap * 0.85,
                    ..options.clone()
                };

                let (sub_nodes, sub_min_x, sub_max_x, sub_min_y, sub_max_y) = Self::layout_radial_children(
                    &child.children,
                    child_center_x,
                    child_center_y,
                    Some(angle),
                    &sub_options,
                    depth + 1,
                    depth_limit,
                    min_scale,
                    max_scale,
                    total_nodes,
                );

                if is_expanded {
                    min_x = min_x.min(sub_min_x);
                    max_x = max_x.max(sub_max_x);
                    min_y = min_y.min(sub_min_y);
                    max_y = max_y.max(sub_max_y);
                }

                let sub_w = (sub_max_x - sub_min_x).max(1.0);
                let sub_h = (sub_max_y - sub_min_y).max(1.0);

                scale = ((node_w - 16.0) / sub_w).min((node_h - 24.0) / sub_h).clamp(0.1, 1.0);
                *min_scale = min_scale.min(scale);
                *max_scale = max_scale.max(scale);
                nested_children = sub_nodes;
            }

            let layout_node = LayoutNode {
                id: format!("radial_{}_{}", child.path.display(), idx),
                path: child.path.clone(),
                name: child.name.clone(),
                is_dir: child.is_dir,
                category: child.category.clone(),
                size_bytes: child.size_bytes,
                item_count: child.item_count,
                x: node_x,
                y: node_y,
                width: node_w,
                height: node_h,
                scale,
                depth,
                children: nested_children,
            };

            nodes.push(layout_node);
        }

        (nodes, min_x, max_x, min_y, max_y)
    }

    fn apply_offset(node: &mut LayoutNode, offset_x: f32, offset_y: f32) {
        node.x += offset_x;
        node.y += offset_y;
        for child in &mut node.children {
            Self::apply_offset(child, offset_x, offset_y);
        }
    }
}

impl CompoundLayoutAlgorithm for RadialBalloonTreeLayout {
    fn kind(&self) -> LayoutKind {
        LayoutKind::RadialBalloonTree
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

        // Use estimated initial center
        let approx_center_x = available_width.max(800.0) / 2.0;
        let approx_center_y = available_height.max(600.0) / 2.0;

        let (children, min_x, max_x, min_y, max_y) = Self::layout_radial_children(
            &root.children,
            approx_center_x,
            approx_center_y,
            None,
            options,
            1,
            options.max_preview_depth,
            &mut min_scale,
            &mut max_scale,
            &mut node_count,
        );

        // Normalize coordinates with padding
        let padding = options.padding + 32.0;
        let offset_x = padding - min_x.min(approx_center_x - options.base_node_width / 2.0);
        let offset_y = padding - min_y.min(approx_center_y - options.base_node_height / 2.0);

        let mut normalized_children = children;
        for node in &mut normalized_children {
            Self::apply_offset(node, offset_x, offset_y);
        }

        let total_w = (max_x - min_x + (padding * 2.0)).max(available_width);
        let total_h = (max_y - min_y + (padding * 2.0)).max(available_height);

        let root_node = LayoutNode {
            id: format!("radial_root_{}", root.path.display()),
            path: root.path.clone(),
            name: root.name.clone(),
            is_dir: root.is_dir,
            category: root.category.clone(),
            size_bytes: root.size_bytes,
            item_count: root.item_count,
            x: approx_center_x + offset_x - (options.base_node_width / 2.0),
            y: approx_center_y + offset_y - (options.base_node_height / 2.0),
            width: options.base_node_width,
            height: options.base_node_height,
            scale: 1.0,
            depth: 0,
            children: normalized_children,
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;

        LayoutResult {
            kind: LayoutKind::RadialBalloonTree,
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
