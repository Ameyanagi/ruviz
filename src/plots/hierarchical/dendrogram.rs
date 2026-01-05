//! Dendrogram implementations
//!
//! Provides hierarchical clustering visualization.

use crate::render::Color;
use crate::stats::clustering::Linkage;

/// Configuration for dendrogram
#[derive(Debug, Clone)]
pub struct DendrogramConfig {
    /// Orientation
    pub orientation: DendrogramOrientation,
    /// Line color
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Show leaf labels
    pub show_labels: bool,
    /// Label font size
    pub label_size: f32,
    /// Truncate at this number of leaves
    pub truncate_mode: Option<TruncateMode>,
    /// Distance threshold for color coding
    pub color_threshold: Option<f64>,
    /// Leaf labels
    pub labels: Vec<String>,
}

/// Orientation for dendrogram
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DendrogramOrientation {
    Top,
    Bottom,
    Left,
    Right,
}

/// Truncation mode for large dendrograms
#[derive(Debug, Clone, Copy)]
pub enum TruncateMode {
    /// Show only last n clusters
    LastN(usize),
    /// Cut at level
    Level(usize),
}

impl Default for DendrogramConfig {
    fn default() -> Self {
        Self {
            orientation: DendrogramOrientation::Top,
            color: None,
            line_width: 1.0,
            show_labels: true,
            label_size: 10.0,
            truncate_mode: None,
            color_threshold: None,
            labels: vec![],
        }
    }
}

impl DendrogramConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set orientation
    pub fn orientation(mut self, orient: DendrogramOrientation) -> Self {
        self.orientation = orient;
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set color threshold
    pub fn color_threshold(mut self, threshold: f64) -> Self {
        self.color_threshold = Some(threshold);
        self
    }
}

/// A link in the dendrogram
#[derive(Debug, Clone)]
pub struct DendrogramLink {
    /// Left child x position
    pub left_x: f64,
    /// Right child x position
    pub right_x: f64,
    /// Left child y position (height/distance)
    pub left_y: f64,
    /// Right child y position
    pub right_y: f64,
    /// Join y position (this cluster's height)
    pub join_y: f64,
    /// Cluster index
    pub cluster_idx: usize,
}

/// Computed dendrogram data
#[derive(Debug, Clone)]
pub struct DendrogramPlotData {
    /// All links
    pub links: Vec<DendrogramLink>,
    /// Leaf positions (x coordinates)
    pub leaf_positions: Vec<f64>,
    /// Leaf order (indices into original data)
    pub leaf_order: Vec<usize>,
    /// Max height
    pub max_height: f64,
    /// Label positions and text
    pub labels: Vec<(f64, String)>,
}

/// Compute dendrogram from linkage result
///
/// # Arguments
/// * `linkage` - Hierarchical clustering linkage result
/// * `config` - Dendrogram configuration
///
/// # Returns
/// DendrogramPlotData for rendering
pub fn compute_dendrogram(linkage: &Linkage, config: &DendrogramConfig) -> DendrogramPlotData {
    if linkage.matrix.is_empty() {
        return DendrogramPlotData {
            links: vec![],
            leaf_positions: vec![],
            leaf_order: linkage.leaves.clone(),
            max_height: 1.0,
            labels: vec![],
        };
    }

    let n_leaves = linkage.leaves.len();
    let leaf_order = linkage.leaves.clone();

    // Create position map for leaves
    let mut leaf_pos = vec![0.0; n_leaves];
    for (i, &leaf) in leaf_order.iter().enumerate() {
        if leaf < n_leaves {
            leaf_pos[leaf] = i as f64;
        }
    }

    // Track positions of each cluster (leaves + merged clusters)
    let mut cluster_pos: Vec<f64> = leaf_pos.clone();
    let mut links = Vec::new();
    let mut max_height = 0.0_f64;

    for (i, row) in linkage.matrix.iter().enumerate() {
        let left = row[0] as usize;
        let right = row[1] as usize;
        let dist = row[2];

        let left_pos = cluster_pos.get(left).copied().unwrap_or(0.0);
        let right_pos = cluster_pos.get(right).copied().unwrap_or(0.0);

        // Get heights of children
        let left_height = if left < n_leaves {
            0.0
        } else {
            linkage
                .matrix
                .get(left - n_leaves)
                .map(|r| r[2])
                .unwrap_or(0.0)
        };
        let right_height = if right < n_leaves {
            0.0
        } else {
            linkage
                .matrix
                .get(right - n_leaves)
                .map(|r| r[2])
                .unwrap_or(0.0)
        };

        links.push(DendrogramLink {
            left_x: left_pos,
            right_x: right_pos,
            left_y: left_height,
            right_y: right_height,
            join_y: dist,
            cluster_idx: n_leaves + i,
        });

        max_height = max_height.max(dist);

        // New cluster position is average of children
        cluster_pos.push((left_pos + right_pos) / 2.0);
    }

    // Generate labels
    let labels: Vec<(f64, String)> = if config.show_labels {
        leaf_order
            .iter()
            .enumerate()
            .map(|(i, &leaf)| {
                let label = config
                    .labels
                    .get(leaf)
                    .cloned()
                    .unwrap_or_else(|| format!("{}", leaf));
                (i as f64, label)
            })
            .collect()
    } else {
        vec![]
    };

    DendrogramPlotData {
        links,
        leaf_positions: (0..n_leaves).map(|i| i as f64).collect(),
        leaf_order,
        max_height: if max_height > 0.0 { max_height } else { 1.0 },
        labels,
    }
}

/// Generate line segments for dendrogram links
pub fn dendrogram_lines(
    link: &DendrogramLink,
    orientation: DendrogramOrientation,
) -> Vec<((f64, f64), (f64, f64))> {
    match orientation {
        DendrogramOrientation::Top | DendrogramOrientation::Bottom => {
            // Horizontal layout
            vec![
                // Left vertical
                ((link.left_x, link.left_y), (link.left_x, link.join_y)),
                // Horizontal connector
                ((link.left_x, link.join_y), (link.right_x, link.join_y)),
                // Right vertical
                ((link.right_x, link.right_y), (link.right_x, link.join_y)),
            ]
        }
        DendrogramOrientation::Left | DendrogramOrientation::Right => {
            // Vertical layout (swap x and y)
            vec![
                // Left horizontal
                ((link.left_y, link.left_x), (link.join_y, link.left_x)),
                // Vertical connector
                ((link.join_y, link.left_x), (link.join_y, link.right_x)),
                // Right horizontal
                ((link.right_y, link.right_x), (link.join_y, link.right_x)),
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::clustering::{LinkageMethod, linkage, pdist_euclidean};

    #[test]
    fn test_dendrogram_basic() {
        // Create simple distance matrix
        let points = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![5.0, 0.0],
            vec![6.0, 0.0],
        ];
        let distances = pdist_euclidean(&points);
        let linkage_result = linkage(&distances, LinkageMethod::Single);
        let config = DendrogramConfig::default();
        let data = compute_dendrogram(&linkage_result, &config);

        // Should have n-1 links for n leaves
        assert_eq!(data.links.len(), 3);
        assert_eq!(data.leaf_order.len(), 4);
    }

    #[test]
    fn test_dendrogram_lines() {
        let link = DendrogramLink {
            left_x: 0.0,
            right_x: 1.0,
            left_y: 0.0,
            right_y: 0.0,
            join_y: 1.0,
            cluster_idx: 2,
        };

        let lines = dendrogram_lines(&link, DendrogramOrientation::Top);
        assert_eq!(lines.len(), 3);
    }
}
