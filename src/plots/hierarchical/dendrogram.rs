//! Dendrogram implementations
//!
//! Provides hierarchical clustering visualization.
//!
//! # Reaching it
//!
//! `Plot::dendrogram(&linkage)` adds one as an ordinary series.
//! [`DendrogramPlotData`] implements [`ComputedSeries`], so its geometry is
//! described once and drawn by both the raster and SVG backends; driving
//! [`compute_dendrogram`] and [`PlotRender::render`] yourself still works.
//!
//! [`dendrogram_lines`] remains public for callers laying the segments out
//! against axes of their own; [`DendrogramPlotData::segments`] is the same
//! geometry for the whole tree and is what the renderer draws.
//!
//! The renderer draws the tree's links. Leaf *labels* are computed into
//! [`DendrogramPlotData::labels`] but not drawn, which is why `Plot`'s
//! dendrogram builder exposes no label setter — see
//! [`DendrogramConfig::show_labels`]. The styling fields that still have
//! nothing behind them at all are marked `#[deprecated]`, so the compiler says
//! so at the call site rather than the caller discovering it from an unchanged
//! image.

use crate::core::Result;
use crate::plots::traits::{
    AxisScaleSupport, ComputedSeries, ComputedStyle, PlotArea, PlotConfig, PlotData, PlotPrimitive,
    PlotRender, draw_primitives,
};
use crate::render::{Color, LineStyle, SkiaRenderer, Theme};
use crate::stats::clustering::Linkage;

/// Configuration for dendrogram
///
/// [`DendrogramConfig::show_labels`] and [`DendrogramConfig::labels`] are
/// consumed by [`compute_dendrogram`]; [`DendrogramConfig::orientation`],
/// [`DendrogramConfig::color`] and [`DendrogramConfig::line_width`] are read by
/// the renderer. The remaining fields are inert — see the module docs.
#[allow(deprecated)] // the derives touch the deprecated fields below
#[derive(Debug, Clone)]
pub struct DendrogramConfig {
    /// Orientation
    ///
    /// Carried onto [`DendrogramPlotData`] by [`compute_dendrogram`], which is
    /// how the renderer and [`DendrogramPlotData::segments`] agree on it.
    pub orientation: DendrogramOrientation,
    /// Line color, or `None` to take the series colour
    pub color: Option<Color>,
    /// Line width, in **points**
    ///
    /// The renderer converts it to device pixels, so a link keeps its physical
    /// thickness at every DPI.
    pub line_width: f32,
    /// Show leaf labels
    pub show_labels: bool,
    /// Label font size
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. Dendrograms are compute-only — set the font size on whatever draws DendrogramPlotData::labels"
    )]
    pub label_size: f32,
    /// Truncate at this number of leaves
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. compute_dendrogram always returns the full tree"
    )]
    pub truncate_mode: Option<TruncateMode>,
    /// Distance threshold for color coding
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. compute_dendrogram does not colour clusters; compare DendrogramLink::join_y yourself"
    )]
    pub color_threshold: Option<f64>,
    /// Leaf labels
    pub labels: Vec<String>,
}

/// Orientation for dendrogram
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DendrogramOrientation {
    #[default]
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
    // The inert fields still have to be populated while they exist.
    #[allow(deprecated)]
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

    /// Set the link colour
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the link width, in points
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Show or hide the leaf labels
    ///
    /// When disabled, [`DendrogramPlotData::labels`] comes back empty.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set color threshold
    ///
    /// Currently inert; see [`DendrogramConfig::color_threshold`].
    #[deprecated(
        since = "0.6.0",
        note = "not yet implemented; tracked for a future release. compute_dendrogram does not colour clusters; compare DendrogramLink::join_y yourself"
    )]
    #[allow(deprecated)]
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
    /// Configuration used to compute this data
    ///
    /// Carried here — as every other computed plot data type carries its
    /// config — so the bounds and the renderer read the same orientation and
    /// styling the caller asked for.
    pub config: DendrogramConfig,
}

/// Compute dendrogram from linkage result
///
/// Reads the matrix in the SciPy convention [`Linkage`] documents: row `i`
/// defines cluster `n + i`, so an id below `n` is a leaf and sits at height
/// zero, and anything else is a merge whose height and centroid come from its
/// own row. Both facts are load-bearing — a matrix that reused leaf ids for
/// merges made every arm drop to the baseline and drew each merge over one of
/// its own leaves instead of between them, which is a different tree.
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
            config: config.clone(),
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
        config: config.clone(),
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for DendrogramConfig {}

impl DendrogramPlotData {
    /// Every link's segments, in **data** coordinates.
    ///
    /// This is the only place whole-tree geometry is derived:
    /// [`PlotRender::render`] projects exactly these segments, so a caller that
    /// draws the tree itself and a rendered one cannot disagree.
    pub fn segments(&self) -> Vec<((f64, f64), (f64, f64))> {
        self.links
            .iter()
            .flat_map(|link| dendrogram_lines(link, self.config.orientation))
            .collect()
    }
}

impl ComputedSeries for DendrogramPlotData {
    fn kind(&self) -> &'static str {
        "dendrogram"
    }

    fn point_count(&self) -> usize {
        self.links.len()
    }

    /// One slot per leaf, so the leaf axis prints the labels the tree carries
    /// rather than repeating the slot numbers underneath them.
    fn category_slots(&self) -> Vec<(String, f64)> {
        self.labels
            .iter()
            .map(|(position, label)| (label.clone(), *position))
            .collect()
    }

    fn category_orientation(&self) -> crate::core::Orientation {
        match self.config.orientation {
            DendrogramOrientation::Top | DendrogramOrientation::Bottom => {
                crate::core::Orientation::Vertical
            }
            DendrogramOrientation::Left | DendrogramOrientation::Right => {
                crate::core::Orientation::Horizontal
            }
        }
    }

    /// The leaf axis carries ordinal slots, so it has no quantitative spacing
    /// to take a logarithm of; the height axis is projected and scales freely.
    fn axis_scale_support(&self) -> (AxisScaleSupport, AxisScaleSupport) {
        match self.config.orientation {
            DendrogramOrientation::Top | DendrogramOrientation::Bottom => {
                (AxisScaleSupport::ORDINAL, AxisScaleSupport::Scaled)
            }
            DendrogramOrientation::Left | DendrogramOrientation::Right => {
                (AxisScaleSupport::Scaled, AxisScaleSupport::ORDINAL)
            }
        }
    }

    /// One stroked segment per link arm, from the tree geometry
    /// [`DendrogramPlotData::segments`] derives.
    fn primitives(&self, area: &PlotArea, style: &ComputedStyle) -> Vec<PlotPrimitive> {
        let base = self.config.color.unwrap_or(style.color);
        let color = style.tinted(base);
        // The width is authored in points; the render scale is what makes the
        // link keep its physical thickness at every DPI.
        let width_px = style.stroke_px(self.config.line_width);

        self.segments()
            .into_iter()
            .filter_map(|(start, end)| {
                // A segment with an endpoint the axis cannot place has no
                // position at all, so it is dropped rather than stroked to a
                // NaN pixel.
                let from = area.try_data_to_screen(start.0, start.1)?;
                let to = area.try_data_to_screen(end.0, end.1)?;
                Some(PlotPrimitive::Line {
                    from,
                    to,
                    color,
                    width_px,
                    style: LineStyle::Solid,
                })
            })
            .collect()
    }
}

impl PlotData for DendrogramPlotData {
    /// The leaf axis spans the ordinal slots with the usual half-slot margin;
    /// the height axis runs from the leaves at zero up to the root.
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        if self.leaf_positions.is_empty() {
            return ((0.0, 1.0), (0.0, 1.0));
        }

        let leaves = (-0.5, self.leaf_positions.len() as f64 - 0.5);
        let heights = (0.0, self.max_height);

        match self.config.orientation {
            DendrogramOrientation::Top | DendrogramOrientation::Bottom => (leaves, heights),
            DendrogramOrientation::Left | DendrogramOrientation::Right => (heights, leaves),
        }
    }

    fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

impl PlotRender for DendrogramPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        let style = ComputedStyle::opaque(renderer.render_scale(), color);
        draw_primitives(renderer, &self.primitives(area, &style))
    }

    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        let style = ComputedStyle {
            scale: renderer.render_scale(),
            color,
            alpha,
            line_width,
        };
        draw_primitives(renderer, &self.primitives(area, &style))
    }
}

/// Generate line segments for one dendrogram link
///
/// [`DendrogramPlotData::segments`] is the whole-tree wrapper the renderer
/// uses, and it is what you want unless you are laying links out against an
/// axis of your own.
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
    fn test_show_labels_gates_the_only_label_output() {
        let points = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![5.0, 0.0]];
        let distances = pdist_euclidean(&points);
        let linkage_result = linkage(&distances, LinkageMethod::Single);

        let with = compute_dendrogram(
            &linkage_result,
            &DendrogramConfig::new().labels(vec!["a".into(), "b".into(), "c".into()]),
        );
        let without = compute_dendrogram(
            &linkage_result,
            &DendrogramConfig::new()
                .labels(vec!["a".into(), "b".into(), "c".into()])
                .show_labels(false),
        );

        assert_eq!(with.labels.len(), 3);
        assert!(without.labels.is_empty());
    }

    fn four_leaf_tree(config: DendrogramConfig) -> DendrogramPlotData {
        let points = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![5.0, 0.0],
            vec![6.0, 0.0],
        ];
        let distances = pdist_euclidean(&points);
        compute_dendrogram(&linkage(&distances, LinkageMethod::Single), &config)
    }

    fn render(data: &DendrogramPlotData, dpi_scale: f32) -> crate::core::plot::Image {
        let mut renderer =
            SkiaRenderer::new(200, 200, Theme::default()).expect("renderer for a 200x200 canvas");
        renderer.set_dpi_scale(dpi_scale);
        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        let area = PlotArea::new(10.0, 10.0, 180.0, 180.0, x_min, x_max, y_min, y_max);
        data.render(
            &mut renderer,
            &area,
            &Theme::default(),
            Color::from_rgb(200, 0, 0),
        )
        .expect("dendrogram render");
        renderer.into_image()
    }

    /// Total ink coverage, proportional to stroked area and so to stroke width.
    fn coverage(image: &crate::core::plot::Image) -> u64 {
        image
            .pixels
            .chunks_exact(4)
            .map(|p| u64::from(255 - p[1]))
            .sum()
    }

    #[test]
    fn test_dendrogram_renders_its_links() {
        let drawn = coverage(&render(&four_leaf_tree(DendrogramConfig::new()), 1.0));
        assert!(drawn > 0, "the dendrogram renderer left the canvas blank");
    }

    #[test]
    fn test_dendrogram_segments_cover_every_link() {
        let data = four_leaf_tree(DendrogramConfig::new());
        // Three segments per link, in the configured orientation.
        assert_eq!(data.segments().len(), data.links.len() * 3);
        assert_eq!(
            data.segments(),
            data.links
                .iter()
                .flat_map(|link| dendrogram_lines(link, DendrogramOrientation::Top))
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_dendrogram_bounds_follow_the_orientation() {
        let upright = four_leaf_tree(DendrogramConfig::new());
        let sideways =
            four_leaf_tree(DendrogramConfig::new().orientation(DendrogramOrientation::Left));

        let ((x_min, x_max), (y_min, y_max)) = upright.data_bounds();
        assert!((x_min - -0.5).abs() < 1e-9 && (x_max - 3.5).abs() < 1e-9);
        assert!((y_min - 0.0).abs() < 1e-9 && y_max > 0.0);

        // A left-facing tree puts the heights on x and the leaves on y.
        assert_eq!(sideways.data_bounds(), ((y_min, y_max), (x_min, x_max)));
    }

    #[test]
    fn test_dendrogram_links_keep_their_physical_width_at_higher_dpi() {
        // `line_width` is in points, so doubling the render scale must double
        // the stroked area.
        let data = four_leaf_tree(DendrogramConfig::new());
        let single = coverage(&render(&data, 1.0));
        let double = coverage(&render(&data, 2.0));

        assert!(single > 0, "the dendrogram drew nothing at all");
        assert!(
            double > single + single / 2,
            "dendrogram links did not thicken with DPI ({double} vs {single} ink coverage)"
        );
    }

    #[test]
    fn test_dendrogram_config_color_is_honoured() {
        // `color` used to be #[deprecated] as "not yet implemented"; the
        // renderer reads it now, so the two images must differ.
        let themed = render(&four_leaf_tree(DendrogramConfig::new()), 1.0);
        let explicit = render(
            &four_leaf_tree(DendrogramConfig::new().color(Color::from_rgb(0, 0, 255))),
            1.0,
        );

        assert_ne!(
            themed.pixels, explicit.pixels,
            "DendrogramConfig::color left the image unchanged"
        );
    }

    #[test]
    fn test_dendrogram_config_line_width_is_honoured() {
        let thin = coverage(&render(&four_leaf_tree(DendrogramConfig::new()), 1.0));
        let thick = coverage(&render(
            &four_leaf_tree(DendrogramConfig::new().line_width(4.0)),
            1.0,
        ));

        assert!(
            thick > thin,
            "DendrogramConfig::line_width drew no more ink ({thick} vs {thin})"
        );
    }

    #[test]
    fn test_dendrogram_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<DendrogramConfig>();
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
