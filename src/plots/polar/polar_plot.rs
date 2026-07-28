//! Polar plot implementations
//!
//! Provides polar scatter, line, and bar plots with configurable axis labels.
//!
//! # Axis Labels
//!
//! Polar plots support angular (theta) and radial (r) axis labels:
//!
//! ```rust,ignore
//! use ruviz::plots::polar::PolarPlotConfig;
//!
//! // Default: show all labels
//! let config = PolarPlotConfig::default();
//!
//! // Customize label appearance
//! let config = PolarPlotConfig::new()
//!     .show_theta_labels(true)      // Show 0°, 30°, 60°, etc.
//!     .show_r_labels(true)          // Show radial scale
//!     .r_label_position(22.5)       // Position at 22.5° from right
//!     .label_font_size(10.0);       // Font size in points
//!
//! // Hide labels for cleaner appearance
//! let minimal = PolarPlotConfig::new()
//!     .show_theta_labels(false)
//!     .show_r_labels(false);
//! ```
//!
//! # Grid
//!
//! The polar grid is a set of concentric rings at the radial ticks plus one
//! spoke per angular tick. Both halves are switchable, and both are drawn from
//! the plot's own [`GridStyle`](crate::core::GridStyle), so a polar grid looks
//! like every other grid in the figure:
//!
//! ```rust,ignore
//! use ruviz::plots::polar::PolarPlotConfig;
//!
//! let config = PolarPlotConfig::new()
//!     .show_rgrid(true)         // Concentric rings
//!     .show_thetagrid(true)     // Spokes
//!     .rgrid_count(4)           // Rings, and radial labels, at four radii
//!     .thetagrid_count(8);      // Spokes, and angular labels, every 45°
//! ```
//!
//! # Trait-Based API
//!
//! Polar plots implement the core plot traits:
//! - [`PlotConfig`] for `PolarPlotConfig`
//! - [`PlotCompute`] for `PolarPlot` marker struct
//! - [`PlotData`] for `PolarPlotData`
//! - [`PlotRender`] for `PolarPlotData`

use crate::core::Result;
use crate::plots::polar::radar::{RADAR_BOUNDS_RADIUS, RADAR_LABEL_RADIUS};
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, LineStyle, MarkerStyle, Theme};

/// Radius, as a multiple of `r_max`, at which the angular labels ring the plot.
///
/// Polar plots and radar charts are one family of radial charts, so they place
/// their outside labels on the same ring and reserve the same pad around it.
/// Pointing both at one constant is the point: polar used to label at
/// `1.12 · r_max` inside a `1.5 · r_max` box, so a cardioid filled ~63% of its
/// square while a radar polygon filled ~80%.
pub(crate) const POLAR_LABEL_RADIUS: f64 = RADAR_LABEL_RADIUS;

/// Half-extent of the data range a polar plot needs in x and y, as a multiple
/// of `r_max`.
///
/// Every arm that has to reserve room for a polar plot — [`PlotData::data_bounds`]
/// here and the raster bounds arm — derives it from this, so the backends agree
/// by construction rather than by two matching literals.
pub(crate) const POLAR_BOUNDS_RADIUS: f64 = RADAR_BOUNDS_RADIUS;

/// Segments used to approximate one grid ring.
///
/// 72 is a 5° step: smooth at any figure size a plot is saved at, and cheap
/// enough that the default five rings cost well under a thousand points.
const POLAR_RING_SEGMENTS: usize = 72;

/// Configuration for polar plots
#[derive(Debug, Clone)]
pub struct PolarPlotConfig {
    /// Start angle in radians (0 = right, counter-clockwise)
    pub theta_offset: f64,
    /// Direction of theta (true = counter-clockwise)
    pub theta_direction: bool,
    /// Draw the concentric radial grid rings
    pub show_rgrid: bool,
    /// Draw the angular grid spokes
    pub show_thetagrid: bool,
    /// Number of radial grid rings
    pub rgrid_count: usize,
    /// Number of angular grid spokes
    pub thetagrid_count: usize,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Marker size
    pub marker_size: f32,
    /// Fill area under curve
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Show angular axis labels (0°, 45°, 90°, etc.)
    pub show_theta_labels: bool,
    /// Show radial axis labels
    pub show_r_labels: bool,
    /// Position of radial labels in degrees from right (default: 22.5)
    pub r_label_position: f64,
    /// Font size for axis labels
    pub label_font_size: f32,
}

impl Default for PolarPlotConfig {
    fn default() -> Self {
        Self {
            theta_offset: 0.0,
            theta_direction: true,
            show_rgrid: true,
            show_thetagrid: true,
            rgrid_count: 5,
            thetagrid_count: 12,
            color: None,
            line_width: 1.5,
            marker_size: 0.0,
            fill: false,
            fill_alpha: 0.3,
            show_theta_labels: true,
            show_r_labels: true,
            r_label_position: 22.5, // degrees
            label_font_size: 10.0,
        }
    }
}

impl PolarPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set theta offset
    pub fn theta_offset(mut self, offset: f64) -> Self {
        self.theta_offset = offset;
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

    /// Set marker size
    pub fn marker_size(mut self, size: f32) -> Self {
        self.marker_size = size.max(0.0);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Show/hide the concentric radial grid rings
    pub fn show_rgrid(mut self, show: bool) -> Self {
        self.show_rgrid = show;
        self
    }

    /// Show/hide the angular grid spokes
    pub fn show_thetagrid(mut self, show: bool) -> Self {
        self.show_thetagrid = show;
        self
    }

    /// Set the number of radial grid rings (and radial tick labels)
    pub fn rgrid_count(mut self, count: usize) -> Self {
        self.rgrid_count = count;
        self
    }

    /// Set the number of angular grid spokes (and angular tick labels)
    pub fn thetagrid_count(mut self, count: usize) -> Self {
        self.thetagrid_count = count;
        self
    }

    /// Show/hide angular labels (0°, 45°, 90°, etc.)
    pub fn show_theta_labels(mut self, show: bool) -> Self {
        self.show_theta_labels = show;
        self
    }

    /// Show/hide radial labels
    pub fn show_r_labels(mut self, show: bool) -> Self {
        self.show_r_labels = show;
        self
    }

    /// Set position of radial labels (degrees from right)
    pub fn r_label_position(mut self, degrees: f64) -> Self {
        self.r_label_position = degrees;
        self
    }

    /// Set label font size
    pub fn label_font_size(mut self, size: f32) -> Self {
        self.label_font_size = size.max(1.0);
        self
    }
}

// Implement PlotConfig marker trait
impl PlotConfig for PolarPlotConfig {}

/// Marker struct for Polar plot type
pub struct PolarPlot;

/// A point in polar coordinates
#[derive(Debug, Clone, Copy)]
pub struct PolarPoint {
    /// Radius (distance from center)
    pub r: f64,
    /// Theta (angle in radians)
    pub theta: f64,
    /// Cartesian x
    pub x: f64,
    /// Cartesian y
    pub y: f64,
}

impl PolarPoint {
    /// Create from polar coordinates
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            r,
            theta,
            x: r * theta.cos(),
            y: r * theta.sin(),
        }
    }

    /// Create from cartesian coordinates
    pub fn from_cartesian(x: f64, y: f64) -> Self {
        Self {
            r: (x * x + y * y).sqrt(),
            theta: y.atan2(x),
            x,
            y,
        }
    }
}

/// A label with position and text
#[derive(Debug, Clone)]
pub struct PositionedLabel {
    /// Screen-relative x position (in data coordinates)
    pub x: f64,
    /// Screen-relative y position (in data coordinates)
    pub y: f64,
    /// Label text
    pub text: String,
}

/// Computed polar plot data
#[derive(Debug, Clone)]
pub struct PolarPlotData {
    /// Points in the plot
    pub points: Vec<PolarPoint>,
    /// Maximum radius
    pub r_max: f64,
    /// Polygon vertices for fill (closed path)
    pub fill_polygon: Vec<(f64, f64)>,
    /// Whether the sweep is a full turn, so the curve closes on itself instead of
    /// through the origin
    pub closed: bool,
    /// Concentric grid rings, in data coordinates, outermost last
    ///
    /// Empty when [`PolarPlotConfig::show_rgrid`] is off. Each ring is a closed
    /// polyline whose last point repeats its first.
    pub grid_rings: Vec<Vec<(f64, f64)>>,
    /// Angular grid spokes, in data coordinates, as `(centre, rim)` pairs
    ///
    /// Empty when [`PolarPlotConfig::show_thetagrid`] is off.
    pub grid_spokes: Vec<((f64, f64), (f64, f64))>,
    /// Angular axis labels (0°, 90°, etc.)
    pub theta_labels: Vec<PositionedLabel>,
    /// Radial axis labels
    pub r_labels: Vec<PositionedLabel>,
    /// Configuration used
    pub(crate) config: PolarPlotConfig,
}

/// Relative gap, as a fraction of `r_max`, below which a full-turn outline is
/// already closed by its own samples.
const CLOSING_SEGMENT_EPSILON: f64 = 1e-9;

impl PolarPlotData {
    /// Segment that closes the outline of a full-turn curve, if one is needed.
    ///
    /// Returns `None` for partial arcs (which close through the origin instead)
    /// and for samplings that already repeat the first point at the end.
    pub fn closing_segment(&self) -> Option<((f64, f64), (f64, f64))> {
        if !self.closed {
            return None;
        }

        let first = self.points.first()?;
        let last = self.points.last()?;
        let gap = ((last.x - first.x).powi(2) + (last.y - first.y).powi(2)).sqrt();

        (gap > CLOSING_SEGMENT_EPSILON * self.r_max)
            .then_some(((last.x, last.y), (first.x, first.y)))
    }

    /// Half-extent of the square this plot needs, labels included.
    ///
    /// The one answer to "how much room does this polar plot want?", shared by
    /// [`PlotData::data_bounds`] and the bounds arm the render pipeline uses.
    pub fn bounds_radius(&self) -> f64 {
        self.r_max * POLAR_BOUNDS_RADIUS
    }
}

/// Input for polar plot computation
pub struct PolarPlotInput<'a> {
    /// Radius values
    pub r: &'a [f64],
    /// Theta values (in radians)
    pub theta: &'a [f64],
}

impl<'a> PolarPlotInput<'a> {
    /// Create new polar plot input
    pub fn new(r: &'a [f64], theta: &'a [f64]) -> Self {
        Self { r, theta }
    }
}

/// Compute polar plot points
///
/// # Arguments
/// * `r` - Radius values
/// * `theta` - Theta values (in radians)
/// * `config` - Polar plot configuration
///
/// # Returns
/// PolarPlotData with converted points
pub fn compute_polar_plot(r: &[f64], theta: &[f64], config: &PolarPlotConfig) -> PolarPlotData {
    use std::f64::consts::PI;

    let n = r.len().min(theta.len());
    if n == 0 {
        return PolarPlotData {
            points: vec![],
            r_max: 1.0,
            fill_polygon: vec![],
            closed: false,
            grid_rings: vec![],
            grid_spokes: vec![],
            theta_labels: vec![],
            r_labels: vec![],
            config: config.clone(),
        };
    }

    // Drop samples that cannot be plotted. A non-finite `r` or `theta` would
    // otherwise become a NaN Cartesian point, and tiny-skia rejects any path
    // containing one — the whole series would fail to render. Filtering here
    // (rather than only inside `is_full_turn`) keeps the sweep classification
    // and the geometry looking at exactly the same samples.
    let finite_theta: Vec<f64> = (0..n)
        .filter(|&i| r[i].is_finite() && theta[i].is_finite())
        .map(|i| theta[i])
        .collect();

    let mut points = Vec::with_capacity(finite_theta.len());
    let mut r_max = 0.0_f64;

    for i in 0..n {
        if !r[i].is_finite() || !theta[i].is_finite() {
            continue;
        }

        // Apply theta offset and direction
        let adjusted_theta = if config.theta_direction {
            theta[i] + config.theta_offset
        } else {
            -theta[i] + config.theta_offset
        };

        let point = PolarPoint::from_polar(r[i], adjusted_theta);
        r_max = r_max.max(r[i].abs());
        points.push(point);
    }

    // Ensure we have a valid r_max
    let r_max = if r_max > 0.0 { r_max } else { 1.0 };

    let closed = is_full_turn(&finite_theta);

    // Generate fill polygon if enabled
    let fill_polygon = if config.fill && !points.is_empty() {
        let mut polygon: Vec<(f64, f64)> = points.iter().map(|p| (p.x, p.y)).collect();
        // A full sweep already closes on itself; adding the origin would cut the
        // fill with a degenerate out-and-back spike at the seam. Only a partial
        // arc has to be closed through the centre.
        if !closed {
            polygon.push((0.0, 0.0));
        }
        polygon
    } else {
        vec![]
    };

    // Grid geometry, from the same `polar_grid` locator the radial labels use,
    // so a ring and its label can never end up at different radii. Both arms are
    // gated here rather than at draw time: the renderers then draw what was
    // computed, and the SVG and raster backends cannot disagree about whether a
    // ring exists.
    let (ring_radii, spokes) = polar_grid(r_max, config.rgrid_count, config.thetagrid_count);
    let grid_rings = if config.show_rgrid {
        ring_radii
            .iter()
            .map(|&radius| circle_vertices(0.0, 0.0, radius, POLAR_RING_SEGMENTS))
            .collect()
    } else {
        vec![]
    };
    let grid_spokes = if config.show_thetagrid {
        spokes
    } else {
        Vec::new()
    };

    // Compute theta labels (0°, 45°, 90°, etc.) positioned at edge of plot
    let theta_labels = if config.show_theta_labels {
        let label_radius = r_max * POLAR_LABEL_RADIUS;
        (0..config.thetagrid_count)
            .map(|i| {
                let angle = 2.0 * PI * i as f64 / config.thetagrid_count as f64;
                let degrees = (angle * 180.0 / PI).round() as i32;
                PositionedLabel {
                    x: label_radius * angle.cos(),
                    y: label_radius * angle.sin(),
                    text: format!("{}°", degrees),
                }
            })
            .collect()
    } else {
        vec![]
    };

    // Compute radial labels positioned along r_label_position angle
    let r_labels = if config.show_r_labels {
        let label_angle = config.r_label_position * PI / 180.0; // Convert to radians
        (1..=config.rgrid_count)
            .map(|i| {
                let radius = r_max * i as f64 / config.rgrid_count as f64;
                PositionedLabel {
                    x: radius * label_angle.cos(),
                    y: radius * label_angle.sin(),
                    text: format!("{:.1}", radius),
                }
            })
            .collect()
    } else {
        vec![]
    };

    PolarPlotData {
        points,
        r_max,
        fill_polygon,
        closed,
        grid_rings,
        grid_spokes,
        theta_labels,
        r_labels,
        config: config.clone(),
    }
}

/// Angular slack, in radians, within which a sweep still counts as a full turn.
const FULL_TURN_EPSILON: f64 = 1e-6;

/// Minimum number of finite samples before a sweep can be called a full turn.
/// Below this the mean spacing is too coarse to tell a closed curve from a wedge.
const FULL_TURN_MIN_SAMPLES: usize = 3;

/// Whether the sampled `theta` values sweep a complete turn, i.e. the curve
/// closes on itself and needs no closing segment through the origin.
///
/// The sweep is the *ordered, unwrapped* angular travel: each consecutive step is
/// folded into `[-π, π]` before being accumulated, so the running total follows the
/// curve instead of jumping a turn at the 0/2π seam. Global extrema would not do:
/// a narrow wedge straddling the seam, such as `[6.1, 6.2, 0.0, 0.1]`, spans 6.2 rad
/// between its extremes but only sweeps ~0.28 rad, and must stay a partial arc.
/// Clockwise sweeps accumulate negatively and are compared by magnitude; multi-turn
/// spirals exceed a turn and count as closed.
///
/// Both common samplings count as closed: endpoint-inclusive (`0..=2π`, where the
/// sweep is already a full turn) and endpoint-exclusive (`i * 2π / n`, where the
/// sweep falls one sample short). Non-finite samples are ignored.
fn is_full_turn(theta: &[f64]) -> bool {
    use std::f64::consts::TAU;

    let mut count = 0usize;
    let mut previous = 0.0_f64;
    let mut cumulative = 0.0_f64;
    let mut lowest = 0.0_f64;
    let mut highest = 0.0_f64;

    for t in theta.iter().copied().filter(|t| t.is_finite()) {
        if count > 0 {
            let step = t - previous;
            cumulative += step - TAU * (step / TAU).round();
            lowest = lowest.min(cumulative);
            highest = highest.max(cumulative);
        }
        count += 1;
        previous = t;
    }

    if count < FULL_TURN_MIN_SAMPLES {
        return false;
    }

    // The widest excursion of the running phase, not its final value: a curve
    // that completes a turn and then backtracks — `[0, π/2, π, 3π/2, 2π, 3π/2]`
    // — is still closed, but its final sweep is only 3π/2 and would read as a
    // partial arc, reintroducing the origin seam this function exists to avoid.
    // Taking `highest - lowest` also keeps clockwise sweeps (which accumulate
    // negatively) and multi-turn spirals correct.
    let sweep = highest - lowest;
    // The residual gap back to the first sample closes the curve when it is no
    // wider than one sampling step.
    let mean_step = sweep / (count - 1) as f64;
    sweep + mean_step >= TAU - FULL_TURN_EPSILON
}

/// Generate polar grid lines
///
/// # Arguments
/// * `r_max` - Maximum radius
/// * `r_count` - Number of radial circles
/// * `theta_count` - Number of angular divisions
///
/// # Returns
/// (radial_circles, angular_lines) where circles are radii and lines are (start, end) points
#[allow(clippy::type_complexity)]
pub fn polar_grid(
    r_max: f64,
    r_count: usize,
    theta_count: usize,
) -> (Vec<f64>, Vec<((f64, f64), (f64, f64))>) {
    // Radial circles
    let radii: Vec<f64> = (1..=r_count)
        .map(|i| r_max * i as f64 / r_count as f64)
        .collect();

    // Angular lines (from center to edge)
    let angular_step = 2.0 * std::f64::consts::PI / theta_count as f64;
    let angular_lines: Vec<((f64, f64), (f64, f64))> = (0..theta_count)
        .map(|i| {
            let theta = i as f64 * angular_step;
            ((0.0, 0.0), (r_max * theta.cos(), r_max * theta.sin()))
        })
        .collect();

    (radii, angular_lines)
}

/// Generate circle vertices for rendering
pub fn circle_vertices(cx: f64, cy: f64, radius: f64, n_segments: usize) -> Vec<(f64, f64)> {
    let step = 2.0 * std::f64::consts::PI / n_segments as f64;
    (0..=n_segments)
        .map(|i| {
            let theta = i as f64 * step;
            (cx + radius * theta.cos(), cy + radius * theta.sin())
        })
        .collect()
}

// ============================================================================
// Trait-Based API
// ============================================================================

impl PlotCompute for PolarPlot {
    type Input<'a> = PolarPlotInput<'a>;
    type Config = PolarPlotConfig;
    type Output = PolarPlotData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        if input.r.is_empty() || input.theta.is_empty() {
            return Err(crate::core::PlottingError::EmptyDataSet);
        }

        Ok(compute_polar_plot(input.r, input.theta, config))
    }
}

impl PlotData for PolarPlotData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        // A symmetric square around the origin, wide enough for the label ring
        // at `POLAR_LABEL_RADIUS` plus its pad. Sized from the same constant a
        // radar chart uses, so the two radial plot types fill the same share of
        // their square.
        let radius = self.bounds_radius();
        ((-radius, radius), (-radius, radius))
    }

    fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl PlotRender for PolarPlotData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
    ) -> Result<()> {
        self.render_styled(renderer, area, theme, color, 1.0, None)
    }

    fn render_styled(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        self.render_styled_with_grid(renderer, area, theme, color, alpha, line_width, None)
    }

    fn render_styled_with_grid(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        theme: &Theme,
        color: Color,
        alpha: f32,
        line_width: Option<f32>,
        grid_style: Option<&crate::core::GridStyle>,
    ) -> Result<()> {
        if self.points.is_empty() {
            return Ok(());
        }

        let config = &self.config;
        let base_color = config.color.unwrap_or(color);
        let line_color =
            base_color.with_alpha((f32::from(base_color.a) / 255.0) * alpha.clamp(0.0, 1.0));
        let render_scale = renderer.render_scale();
        let line_width_px = render_scale.points_to_pixels(line_width.unwrap_or(config.line_width));
        let marker_size_px = render_scale.points_to_pixels(config.marker_size);
        let label_font_size_px = render_scale.points_to_pixels(config.label_font_size);

        // Grid first, underneath everything: rings and spokes are the frame the
        // radial tick numbers label. Resolved exactly the way the radar grid is,
        // from the plot's own `GridStyle`, so a polar grid and a radar grid in
        // the same theme are the same lines. Falling back to the theme when no
        // style is passed keeps the trait's simpler entry points honest.
        if grid_style.is_none_or(|style| style.visible) {
            let grid_color = grid_style.map_or(theme.grid_color, |style| {
                style.color.with_alpha(style.alpha)
            });
            let grid_line_width =
                render_scale.points_to_pixels(grid_style.map_or(0.5, |style| style.line_width));
            let grid_line_style = grid_style
                .map(|style| style.line_style.clone())
                .unwrap_or(LineStyle::Solid);

            for ring in &self.grid_rings {
                if ring.len() < 2 {
                    continue;
                }
                let screen_ring: Vec<(f32, f32)> = ring
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                renderer.draw_polyline(
                    &screen_ring,
                    grid_color,
                    grid_line_width,
                    grid_line_style.clone(),
                )?;
            }

            for &((x1, y1), (x2, y2)) in &self.grid_spokes {
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    grid_color,
                    grid_line_width,
                    grid_line_style.clone(),
                )?;
            }
        }

        // Draw fill if enabled
        if config.fill && !self.fill_polygon.is_empty() {
            let fill_color = base_color.with_alpha(
                (f32::from(base_color.a) / 255.0) * config.fill_alpha * alpha.clamp(0.0, 1.0),
            );
            let screen_polygon: Vec<(f32, f32)> = self
                .fill_polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();
            renderer.draw_filled_polygon(&screen_polygon, fill_color)?;
        }

        // Draw lines connecting points
        if self.points.len() > 1 {
            for i in 0..self.points.len() - 1 {
                let p1 = &self.points[i];
                let p2 = &self.points[i + 1];
                let (sx1, sy1) = area.data_to_screen(p1.x, p1.y);
                let (sx2, sy2) = area.data_to_screen(p2.x, p2.y);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    line_color,
                    line_width_px,
                    LineStyle::Solid,
                )?;
            }

            // Endpoint-exclusive sampling of a full turn stops one step short of
            // the start; without this segment the outline gapes at the seam.
            if let Some(segment) = self.closing_segment() {
                let ((x1, y1), (x2, y2)) = segment;
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                renderer.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    line_color,
                    line_width_px,
                    LineStyle::Solid,
                )?;
            }
        }

        // Draw markers if configured
        if config.marker_size > 0.0 {
            for point in &self.points {
                let (sx, sy) = area.data_to_screen(point.x, point.y);
                renderer.draw_marker(sx, sy, marker_size_px, MarkerStyle::Circle, line_color)?;
            }
        }

        // Draw theta labels (angular axis labels: 0°, 30°, 60°, etc.)
        let label_color = theme.foreground;
        for label in &self.theta_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            renderer.draw_text_centered(&label.text, sx, sy, label_font_size_px, label_color)?;
        }

        // Draw radial labels
        for label in &self.r_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            renderer.draw_text_centered(&label.text, sx, sy, label_font_size_px, label_color)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, TAU};

    #[test]
    fn test_polar_point_from_polar() {
        let point = PolarPoint::from_polar(1.0, 0.0);
        assert!((point.x - 1.0).abs() < 1e-10);
        assert!((point.y - 0.0).abs() < 1e-10);

        let point = PolarPoint::from_polar(1.0, PI / 2.0);
        assert!((point.x - 0.0).abs() < 1e-10);
        assert!((point.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_point_from_cartesian() {
        let point = PolarPoint::from_cartesian(1.0, 0.0);
        assert!((point.r - 1.0).abs() < 1e-10);
        assert!((point.theta - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_polar_plot() {
        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.points.len(), 3);
        assert!((data.r_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_full_turn_fill_has_no_origin_seam() {
        // Endpoint-exclusive sampling of a limaçon: the curve closes on itself,
        // so no origin vertex may be appended (it would cut a seam at 0°).
        let n = 200;
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
        let r: Vec<f64> = theta.iter().map(|&t| 2.0 + t.cos()).collect();
        let config = PolarPlotConfig::default().fill(true);
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.fill_polygon.len(), n);
        assert!(
            !data
                .fill_polygon
                .iter()
                .any(|&(x, y)| x.abs() < 1e-12 && y.abs() < 1e-12),
            "full-turn fill must not close through the origin"
        );

        // Endpoint-inclusive sampling closes too.
        let theta: Vec<f64> = (0..=n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
        let r: Vec<f64> = theta.iter().map(|&t| 2.0 + t.cos()).collect();
        let data = compute_polar_plot(&r, &theta, &config);
        assert_eq!(data.fill_polygon.len(), n + 1);
    }

    #[test]
    fn test_partial_arc_fill_closes_through_origin() {
        // A half turn is a wedge: closing through the centre is correct.
        let n = 50;
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * PI / (n - 1) as f64).collect();
        let r: Vec<f64> = vec![1.0; n];
        let config = PolarPlotConfig::default().fill(true);
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.fill_polygon.len(), n + 1);
        let last = data.fill_polygon[n];
        assert!(last.0.abs() < 1e-12 && last.1.abs() < 1e-12);
    }

    #[test]
    fn test_is_full_turn() {
        assert!(is_full_turn(&[0.0, PI, 2.0 * PI]));
        assert!(is_full_turn(&[0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0]));
        assert!(!is_full_turn(&[0.0, PI / 2.0, PI]));
        // Too few samples to distinguish a closed curve from a wedge.
        assert!(!is_full_turn(&[0.0, PI]));
        assert!(!is_full_turn(&[]));
        // Non-finite samples are ignored.
        assert!(is_full_turn(&[0.0, f64::NAN, PI, 2.0 * PI]));
    }

    /// Normalized angles wrapped into `[0, 2π)`, sampled endpoint-exclusively.
    fn wrapped_sweep(start: f64, sweep: f64, n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (start + sweep * i as f64 / n as f64).rem_euclid(TAU))
            .collect()
    }

    #[test]
    fn test_is_full_turn_partial_arc_crossing_zero() {
        // A ~0.28 rad wedge straddling the 0/2π seam. Its extremes are 6.2 rad
        // apart, but it sweeps almost nothing: it must stay partial.
        assert!(!is_full_turn(&[6.1, 6.2, 0.0, 0.1]));
        // Same wedge swept clockwise.
        assert!(!is_full_turn(&[0.1, 0.0, 6.2, 6.1]));
        // A denser wedge across the seam, still partial.
        assert!(!is_full_turn(&wrapped_sweep(6.0, 0.5, 32)));
    }

    #[test]
    fn test_wrapping_partial_arc_fills_as_a_wedge() {
        // The seam-crossing wedge must close through the origin; joining its
        // endpoints directly would render a lens/chord instead.
        let theta = vec![6.1, 6.2, 0.0, 0.1];
        let r = vec![1.0; theta.len()];
        let config = PolarPlotConfig::default().fill(true);
        let data = compute_polar_plot(&r, &theta, &config);

        assert!(!data.closed, "a 0.28 rad wedge is not a full turn");
        assert!(data.closing_segment().is_none());
        assert_eq!(data.fill_polygon.len(), theta.len() + 1);
        let last = data.fill_polygon[theta.len()];
        assert!(last.0.abs() < 1e-12 && last.1.abs() < 1e-12);
    }

    #[test]
    fn test_is_full_turn_not_starting_at_zero() {
        // A genuine full turn whose samples start mid-circle and wrap at the seam.
        let theta = wrapped_sweep(1.0, TAU, 180);
        assert!(is_full_turn(&theta));

        let r = vec![1.0; theta.len()];
        let config = PolarPlotConfig::default().fill(true);
        let data = compute_polar_plot(&r, &theta, &config);
        assert!(data.closed);
        // No origin vertex: a closed curve would show a seam through the centre.
        assert_eq!(data.fill_polygon.len(), theta.len());
        assert!(
            !data
                .fill_polygon
                .iter()
                .any(|&(x, y)| x.abs() < 1e-12 && y.abs() < 1e-12)
        );
    }

    #[test]
    fn test_is_full_turn_clockwise() {
        // Decreasing angles sweep the same circle the other way round.
        assert!(is_full_turn(&wrapped_sweep(0.0, -TAU, 180)));
        assert!(is_full_turn(&wrapped_sweep(2.4, -TAU, 180)));
        assert!(is_full_turn(&[0.0, -2.0 * PI / 3.0, -4.0 * PI / 3.0]));
        // A clockwise quarter turn is still a wedge.
        assert!(!is_full_turn(&wrapped_sweep(0.3, -PI / 2.0, 16)));
    }

    #[test]
    fn test_is_full_turn_half_circle() {
        let n = 50;
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * PI / (n - 1) as f64).collect();
        assert!(!is_full_turn(&theta));
        // Shifting the same half circle across the seam does not change it.
        assert!(!is_full_turn(&wrapped_sweep(5.5, PI, n)));
    }

    #[test]
    fn test_is_full_turn_multi_turn_spiral() {
        // Two turns still close on themselves as far as the fill is concerned.
        let n = 100;
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * TAU / n as f64).collect();
        assert!(is_full_turn(&theta));
        assert!(is_full_turn(&wrapped_sweep(0.7, 2.0 * TAU, n)));
    }

    #[test]
    fn test_is_full_turn_survives_backtracking() {
        // A completed turn that then reverses is still closed. Classifying on the
        // final signed sweep would see only 3π/2 here and reintroduce the seam.
        assert!(is_full_turn(&[
            0.0,
            PI / 2.0,
            PI,
            3.0 * PI / 2.0,
            TAU,
            3.0 * PI / 2.0,
        ]));

        // ... but an out-and-back that never completes a turn stays partial.
        assert!(!is_full_turn(&[0.0, PI / 2.0, PI, PI / 2.0, 0.0]));

        // Same rule clockwise.
        assert!(is_full_turn(&[
            0.0,
            -PI / 2.0,
            -PI,
            -3.0 * PI / 2.0,
            -TAU,
            -3.0 * PI / 2.0,
        ]));
    }

    #[test]
    fn test_compute_polar_plot_drops_non_finite_samples() {
        // A NaN in either array must not reach the geometry: tiny-skia rejects a
        // path containing a NaN, which would fail the whole series.
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(
            &[1.0, 2.0, f64::NAN, 3.0, 4.0],
            &[0.0, 1.0, 2.0, f64::INFINITY, 3.0],
            &config,
        );

        assert_eq!(data.points.len(), 3, "non-finite pairs should be dropped");
        assert!(
            data.points
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite()),
            "no NaN or infinite coordinates may survive"
        );
        assert!(data.r_max.is_finite() && data.r_max > 0.0);
    }

    #[test]
    fn test_closing_segment() {
        let n = 200;
        // Endpoint-exclusive: outline stops one step short, so it needs closing.
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
        let r: Vec<f64> = vec![1.0; n];
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(&r, &theta, &config);
        let (from, to) = data.closing_segment().expect("full turn needs closing");
        assert!((from.0 - data.points[n - 1].x).abs() < 1e-12);
        assert!((to.0 - data.points[0].x).abs() < 1e-12);

        // Endpoint-inclusive: the last sample already coincides with the first.
        let theta: Vec<f64> = (0..=n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
        let r: Vec<f64> = vec![1.0; n + 1];
        let data = compute_polar_plot(&r, &theta, &config);
        assert!(data.closing_segment().is_none());

        // Partial arcs close through the origin, not back to the first point.
        let theta: Vec<f64> = (0..n).map(|i| i as f64 * PI / n as f64).collect();
        let r: Vec<f64> = vec![1.0; n];
        let data = compute_polar_plot(&r, &theta, &config);
        assert!(!data.closed);
        assert!(data.closing_segment().is_none());
    }

    #[test]
    fn test_fill_disabled_leaves_polygon_empty() {
        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let data = compute_polar_plot(&r, &theta, &PolarPlotConfig::default());
        assert!(data.fill_polygon.is_empty());
    }

    #[test]
    fn test_polar_grid() {
        let (radii, lines) = polar_grid(10.0, 5, 8);

        assert_eq!(radii.len(), 5);
        assert_eq!(lines.len(), 8);
        assert!((radii[4] - 10.0).abs() < 1e-10);
    }

    /// The grid used to be computed by a `polar_grid` nothing called, so a
    /// rendered polar plot had no rings and no spokes and its radial tick
    /// numbers floated in blank space. The geometry now travels on the data, so
    /// every backend draws the same rings.
    #[test]
    fn computed_polar_data_carries_its_grid() {
        let r = vec![1.0, 2.0, 4.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.grid_rings.len(), config.rgrid_count);
        assert_eq!(data.grid_spokes.len(), config.thetagrid_count);

        // The outer ring sits exactly on `r_max`, and every ring is closed.
        for ring in &data.grid_rings {
            assert_eq!(ring.len(), POLAR_RING_SEGMENTS + 1);
            assert!((ring[0].0 - ring[ring.len() - 1].0).abs() < 1e-9);
            assert!((ring[0].1 - ring[ring.len() - 1].1).abs() < 1e-9);
        }
        let outer = data.grid_rings.last().expect("outer ring");
        for &(x, y) in outer {
            assert!((x.hypot(y) - data.r_max).abs() < 1e-9);
        }

        // Spokes run from the centre out to the rim.
        for &((x1, y1), (x2, y2)) in &data.grid_spokes {
            assert!(x1.abs() < 1e-12 && y1.abs() < 1e-12);
            assert!((x2.hypot(y2) - data.r_max).abs() < 1e-9);
        }
    }

    /// Each half of the grid is switchable on its own, and the switch is
    /// applied once — at compute time — so no backend can honour it and another
    /// ignore it.
    #[test]
    fn grid_visibility_flags_are_live() {
        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let grid_of = |config: PolarPlotConfig| compute_polar_plot(&r, &theta, &config);

        let no_rings = grid_of(PolarPlotConfig::default().show_rgrid(false));
        assert!(no_rings.grid_rings.is_empty());
        assert!(!no_rings.grid_spokes.is_empty());

        let no_spokes = grid_of(PolarPlotConfig::default().show_thetagrid(false));
        assert!(!no_spokes.grid_rings.is_empty());
        assert!(no_spokes.grid_spokes.is_empty());

        let counted = grid_of(PolarPlotConfig::default().rgrid_count(3).thetagrid_count(6));
        assert_eq!(counted.grid_rings.len(), 3);
        assert_eq!(counted.grid_spokes.len(), 6);
        // The radial labels count the same rings they annotate.
        assert_eq!(counted.r_labels.len(), 3);
        assert_eq!(counted.theta_labels.len(), 6);
    }

    /// A ring radius and the radial label beside it must be the same number.
    #[test]
    fn radial_labels_sit_on_their_rings() {
        let r = vec![0.5, 2.0, 3.0];
        let theta = vec![0.0, 1.0, 2.0];
        let data = compute_polar_plot(&r, &theta, &PolarPlotConfig::default());

        for (ring, label) in data.grid_rings.iter().zip(&data.r_labels) {
            let ring_radius = ring[0].0.hypot(ring[0].1);
            let label_radius = label.x.hypot(label.y);
            assert!((ring_radius - label_radius).abs() < 1e-9);
        }
    }

    /// Polar and radar are one family: the same label ring, the same pad, so
    /// the same share of the square is filled. Polar used to reserve
    /// `1.5 · r_max` for labels drawn at `1.12 · r_max`, wasting a quarter of
    /// the frame that radar used.
    #[test]
    fn polar_reserves_the_same_square_as_radar() {
        assert_eq!(POLAR_LABEL_RADIUS, RADAR_LABEL_RADIUS);
        assert_eq!(POLAR_BOUNDS_RADIUS, RADAR_BOUNDS_RADIUS);
        const { assert!(POLAR_BOUNDS_RADIUS > POLAR_LABEL_RADIUS) };

        let r = vec![1.0, 2.0, 4.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let data = compute_polar_plot(&r, &theta, &PolarPlotConfig::default());
        assert!((data.bounds_radius() - data.r_max * POLAR_BOUNDS_RADIUS).abs() < 1e-12);

        let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
        assert!((x_max - data.bounds_radius()).abs() < 1e-12);
        assert!((x_min + data.bounds_radius()).abs() < 1e-12);
        assert!((y_max - data.bounds_radius()).abs() < 1e-12);
        assert!((y_min + data.bounds_radius()).abs() < 1e-12);

        // Every label the plot draws fits inside the square it asks for.
        for label in data.theta_labels.iter().chain(&data.r_labels) {
            assert!(label.x.abs() <= data.bounds_radius());
            assert!(label.y.abs() <= data.bounds_radius());
        }
        // ... and the drawn area is a clear majority of it.
        assert!(data.r_max / data.bounds_radius() > 0.75);
    }

    #[test]
    fn test_circle_vertices() {
        let vertices = circle_vertices(0.0, 0.0, 1.0, 4);
        assert_eq!(vertices.len(), 5); // 4 segments + closing point
    }

    #[test]
    fn test_polar_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<PolarPlotConfig>();
    }

    #[test]
    fn test_polar_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let input = PolarPlotInput::new(&r, &theta);
        let result = PolarPlot::compute(input, &config);

        assert!(result.is_ok());
        let polar_data = result.unwrap();
        assert_eq!(polar_data.points.len(), 3);
    }

    #[test]
    fn test_polar_plot_data_trait() {
        use crate::plots::traits::{PlotCompute, PlotData};

        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let input = PolarPlotInput::new(&r, &theta);
        let polar_data = PolarPlot::compute(input, &config).unwrap();

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = polar_data.data_bounds();
        assert!(x_min < 0.0);
        assert!(x_max > 0.0);
        assert!(y_min < 0.0);
        assert!(y_max > 0.0);

        // Test is_empty
        assert!(!polar_data.is_empty());
    }
}
