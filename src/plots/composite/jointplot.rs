//! Joint plots: a bivariate panel with a marginal distribution on each axis.
//!
//! [`jointplot`] assembles the figure; [`joint_plot_layout`] is the geometry it
//! uses, exposed on its own for anyone placing the three panels by hand with
//! [`SubplotFigure::add_axes`].

use crate::core::subplot::{FigureRect, SubplotFigure, figure};
use crate::core::{
    DEFAULT_AUTOSCALE_MARGIN, IntoPlot, MarginConfig, Plot, PlottingError, Result, ShapeStyle,
};
use crate::data::NumericData1D;
use crate::plots::distribution::RugAxis;
use crate::render::{Color, LineStyle, Theme};
use crate::stats::kde::kde_1d;

/// Configuration for joint plot
#[derive(Debug, Clone)]
pub struct JointPlotConfig {
    /// Type of central plot
    ///
    /// [`JointKind::Scatter`] and [`JointKind::Hex`] are drawable; the other
    /// three have no renderer behind them yet and [`jointplot_with`] reports
    /// that rather than quietly substituting a scatter.
    pub kind: JointKind,
    /// Show marginal histograms
    pub marginal_hist: bool,
    /// Show marginal KDE
    pub marginal_kde: bool,
    /// Show rugplot on margins
    pub rugplot: bool,
    /// Scatter point size
    pub scatter_size: f32,
    /// Scatter alpha
    pub scatter_alpha: f32,
    /// Scatter color (also colours the marginals, so the figure reads as one plot)
    pub color: Option<Color>,
    /// Number of histogram bins
    pub bins: usize,
    /// Ratio of marginal plot size to main plot
    pub marginal_ratio: f64,
}

/// Type of central plot in joint plot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JointKind {
    /// Scatter plot
    #[default]
    Scatter,
    /// Regression plot
    Reg,
    /// Hexbin density
    Hex,
    /// KDE density
    Kde,
    /// Residual plot
    Resid,
}

impl Default for JointPlotConfig {
    fn default() -> Self {
        Self {
            kind: JointKind::Scatter,
            marginal_hist: true,
            marginal_kde: true,
            rugplot: false,
            scatter_size: 5.0,
            scatter_alpha: 0.7,
            color: None,
            bins: 30,
            marginal_ratio: 0.2,
        }
    }
}

impl JointPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set joint plot kind
    pub fn kind(mut self, kind: JointKind) -> Self {
        self.kind = kind;
        self
    }

    /// Enable marginal histograms
    pub fn marginal_hist(mut self, show: bool) -> Self {
        self.marginal_hist = show;
        self
    }

    /// Enable marginal KDE
    pub fn marginal_kde(mut self, show: bool) -> Self {
        self.marginal_kde = show;
        self
    }

    /// Enable rugplot
    ///
    /// Draws one tick per observation against the value axis of each marginal
    /// panel, using the same [`rug`](crate::core::Plot::rug) plot type the
    /// builder exposes.
    pub fn rugplot(mut self, show: bool) -> Self {
        self.rugplot = show;
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set number of bins
    pub fn bins(mut self, bins: usize) -> Self {
        self.bins = bins.max(2);
        self
    }

    /// Set the marginal panel thickness as a fraction of the figure
    ///
    /// Clamped to `0.1..=0.4` by [`joint_plot_layout`].
    pub fn marginal_ratio(mut self, ratio: f64) -> Self {
        self.marginal_ratio = ratio;
        self
    }
}

/// Layout for joint plot
///
/// All three rectangles are in figure-relative coordinates with the origin at
/// the lower-left corner, so each can be handed straight to
/// [`SubplotFigure::add_axes`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointPlotLayout {
    /// Main plot bounds
    pub main_bounds: FigureRect,
    /// X marginal bounds (top)
    pub x_marginal_bounds: FigureRect,
    /// Y marginal bounds (right)
    pub y_marginal_bounds: FigureRect,
}

/// Compute joint plot layout
///
/// The main panel and the top marginal are given the same width, and the main
/// panel and the right marginal the same height, so that panels rendered with
/// [`panel_config`] have their shared axis in the same place.
pub fn joint_plot_layout(marginal_ratio: f64) -> JointPlotLayout {
    let ratio = if marginal_ratio.is_finite() {
        marginal_ratio.clamp(0.1, 0.4)
    } else {
        0.2
    };
    let gap = 0.02;

    JointPlotLayout {
        main_bounds: FigureRect::new(0.0, 0.0, 1.0 - ratio - gap, 1.0 - ratio - gap),
        x_marginal_bounds: FigureRect::new(0.0, 1.0 - ratio, 1.0 - ratio - gap, ratio),
        y_marginal_bounds: FigureRect::new(1.0 - ratio, 0.0, ratio, 1.0 - ratio - gap),
    }
}

/// Computed marginal histogram data
#[derive(Debug, Clone)]
pub struct MarginalHistogram {
    /// Bin edges
    pub edges: Vec<f64>,
    /// Bin counts
    pub counts: Vec<usize>,
    /// Bin centers
    pub centers: Vec<f64>,
}

impl MarginalHistogram {
    /// Bin heights normalised to a probability density.
    ///
    /// A count and a kernel density estimate of the same data are not on the
    /// same scale, and a marginal panel draws both. Normalising here is what
    /// lets them share one axis.
    pub fn density(&self) -> Vec<f64> {
        let total: usize = self.counts.iter().sum();
        let bin_width = match (self.edges.first(), self.edges.get(1)) {
            (Some(low), Some(high)) => high - low,
            _ => return Vec::new(),
        };
        if total == 0 || bin_width <= 0.0 {
            return vec![0.0; self.counts.len()];
        }
        let scale = 1.0 / (total as f64 * bin_width);
        self.counts.iter().map(|&c| c as f64 * scale).collect()
    }

    /// The staircase outline of the histogram as `(value, density)` columns,
    /// starting and ending on the baseline.
    pub fn staircase(&self) -> (Vec<f64>, Vec<f64>) {
        let density = self.density();
        if density.is_empty() {
            return (Vec::new(), Vec::new());
        }
        let points = 2 * (density.len() + 1);
        let mut values = Vec::with_capacity(points);
        let mut densities = Vec::with_capacity(points);

        values.push(self.edges[0]);
        densities.push(0.0);
        for (bin, height) in density.iter().enumerate() {
            values.push(self.edges[bin]);
            densities.push(*height);
            values.push(self.edges[bin + 1]);
            densities.push(*height);
        }
        values.push(self.edges[density.len()]);
        densities.push(0.0);

        (values, densities)
    }
}

/// Compute marginal histogram
pub fn compute_marginal_histogram(data: &[f64], bins: usize) -> MarginalHistogram {
    if data.is_empty() || bins == 0 {
        return MarginalHistogram {
            edges: vec![],
            counts: vec![],
            centers: vec![],
        };
    }

    let min_val = data.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max_val - min_val).abs() < 1e-10 {
        1.0
    } else {
        max_val - min_val
    };
    let bin_width = range / bins as f64;

    // Create edges
    let edges: Vec<f64> = (0..=bins).map(|i| min_val + i as f64 * bin_width).collect();

    // Count points in each bin
    let mut counts = vec![0_usize; bins];
    for &val in data {
        let bin = ((val - min_val) / bin_width).floor() as usize;
        let bin = bin.min(bins - 1); // Handle edge case
        counts[bin] += 1;
    }

    // Compute centers
    let centers: Vec<f64> = (0..bins)
        .map(|i| min_val + (i as f64 + 0.5) * bin_width)
        .collect();

    MarginalHistogram {
        edges,
        counts,
        centers,
    }
}

// ===========================================================================
// Shared panel machinery
//
// Everything a composite figure assembles — the joint plot's three panels and
// the pair plot's n² cells — is built here, so a composite cannot grow a panel
// that lays out differently from its neighbours.
// ===========================================================================

/// The margin rule every panel of a composite figure shares.
///
/// Panels are rendered into separate canvases and composited, so their data
/// areas line up only if their margins are the same *fraction* of the same
/// dimension. The default content-driven margins measure each panel's own tick
/// labels, so a main panel that draws y tick labels and a marginal panel that
/// does not would disagree by tens of pixels and the marginal would visibly
/// slide off its own axis. Fixed proportions make the alignment structural:
/// [`joint_plot_layout`] hands the main panel and its top marginal the same
/// width, and the same left/right fractions of that width put their x axes in
/// exactly the same place.
pub fn panel_config() -> crate::core::PlotConfig {
    crate::core::PlotConfig {
        margins: MarginConfig::proportional_custom(0.17, 0.05, 0.08, 0.17),
        ..crate::core::PlotConfig::default()
    }
}

/// A panel with the composite margin rule already applied.
pub(crate) fn panel() -> Plot {
    Plot::with_config(panel_config())
}

/// The data range a panel and everything aligned with it must share.
///
/// Padded exactly as the crate's autoscale pads an axis, so an explicitly
/// limited composite panel frames its data the way a standalone plot would.
pub(crate) fn padded_range(values: &[f64]) -> (f64, f64) {
    let (low, high) = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| {
            (low.min(v), high.max(v))
        });
    if !low.is_finite() || !high.is_finite() {
        return (0.0, 1.0);
    }

    let span = high - low;
    let pad = if span > 0.0 {
        span * DEFAULT_AUTOSCALE_MARGIN
    } else {
        low.abs().max(1.0) * DEFAULT_AUTOSCALE_MARGIN
    };
    (low - pad, high + pad)
}

/// Which of a joint plot's two variables a marginal panel describes.
///
/// Spelled like [`RugAxis`], the other "which axis do the marks stand on"
/// knob in the crate: `X` is the panel above the main one, whose values run
/// left to right; `Y` is the panel to its right, whose values run bottom to
/// top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarginalAxis {
    /// Values on the x axis, density on the y axis.
    X,
    /// Values on the y axis, density on the x axis.
    Y,
}

impl MarginalAxis {
    /// Map a `(value, density)` pair of columns onto `(x, y)` for this axis.
    fn place(self, values: Vec<f64>, density: Vec<f64>) -> (Vec<f64>, Vec<f64>) {
        match self {
            MarginalAxis::X => (values, density),
            MarginalAxis::Y => (density, values),
        }
    }

    fn rug_axis(self) -> RugAxis {
        match self {
            MarginalAxis::X => RugAxis::X,
            MarginalAxis::Y => RugAxis::Y,
        }
    }
}

/// One marginal distribution panel, described once and drawn the same way in
/// both orientations.
///
/// The orientation is a parameter rather than a second code path, so a joint
/// plot's top and right marginals — and a pair plot's diagonal — cannot drift
/// apart in binning, normalisation, colour or limits.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Marginal<'a> {
    /// The observations this panel summarises.
    pub values: &'a [f64],
    /// The shared limits of the value axis.
    pub range: (f64, f64),
    /// Which way the panel runs.
    pub axis: MarginalAxis,
    /// Draw a density-normalised histogram.
    pub hist: bool,
    /// Draw a kernel density estimate.
    pub kde: bool,
    /// Draw one tick per observation against the value axis.
    pub rug: bool,
    /// Histogram bin count.
    pub bins: usize,
    /// The colour the whole composite figure is drawn in.
    pub color: Color,
}

impl Marginal<'_> {
    /// Number of points in the kernel density curve.
    const KDE_POINTS: usize = 128;
    /// Headroom above the tallest bin, so the peak is not clipped by the frame.
    const DENSITY_HEADROOM: f64 = 1.08;

    /// Build the panel, or `None` when it would be blank.
    pub(crate) fn axes(self) -> Option<Plot> {
        if self.values.is_empty() || !(self.hist || self.kde || self.rug) {
            return None;
        }

        let histogram = self
            .hist
            .then(|| compute_marginal_histogram(self.values, self.bins));
        let kde = self
            .kde
            .then(|| kde_1d(self.values, None, Some(Self::KDE_POINTS)));

        let peak = |values: &[f64]| values.iter().copied().fold(0.0_f64, f64::max);
        let mut density_max = 0.0_f64;
        if let Some(histogram) = &histogram {
            density_max = density_max.max(peak(&histogram.density()));
        }
        if let Some(kde) = &kde {
            density_max = density_max.max(peak(&kde.density));
        }
        if density_max <= 0.0 || !density_max.is_finite() {
            density_max = 1.0;
        }

        let mut plot = panel().grid(false).ticks(false);

        // Filled bins as rectangle annotations: one geometry expression that
        // works in both orientations, where `area`/`fill_to_baseline` only
        // fill downwards to a y baseline.
        if let Some(histogram) = &histogram {
            let fill = ShapeStyle {
                fill_color: Some(self.color),
                fill_alpha: 0.35,
                edge_color: None,
                edge_width: 0.0,
                edge_style: LineStyle::Solid,
            };
            for (bin, density) in histogram.density().iter().enumerate() {
                if *density <= 0.0 {
                    continue;
                }
                let (low, high) = (histogram.edges[bin], histogram.edges[bin + 1]);
                let (x, y, width, height) = match self.axis {
                    MarginalAxis::X => (low, 0.0, high - low, *density),
                    MarginalAxis::Y => (0.0, low, *density, high - low),
                };
                plot = plot.rect_styled(x, y, width, height, fill.clone());
            }

            let (values, density) = histogram.staircase();
            let (x, y) = self.axis.place(values, density);
            plot = plot
                .line(&x, &y)
                .color(self.color)
                .line_width(1.0)
                .into_plot();
        }

        if let Some(kde) = kde {
            let (x, y) = self.axis.place(kde.x, kde.density);
            plot = plot
                .line(&x, &y)
                .color(self.color)
                .line_width(1.5)
                .into_plot();
        }

        if self.rug {
            let values = self.values.to_vec();
            plot = plot
                .rug(&values)
                .axis(self.axis.rug_axis())
                .color(self.color)
                .into_plot();
        }

        let density_limit = density_max * Self::DENSITY_HEADROOM;
        Some(match self.axis {
            MarginalAxis::X => plot
                .xlim(self.range.0, self.range.1)
                .ylim(0.0, density_limit),
            MarginalAxis::Y => plot
                .xlim(0.0, density_limit)
                .ylim(self.range.0, self.range.1),
        })
    }
}

/// The colour a composite figure draws itself in when the config names none.
pub(crate) fn resolved_color(color: Option<Color>) -> Color {
    color.unwrap_or_else(|| Theme::default().get_color(0))
}

// ===========================================================================
// The figure
// ===========================================================================

/// Draw a joint plot: a bivariate main panel with a marginal distribution
/// above it and another to its right.
///
/// Returns the same [`SubplotFigure`] that
/// [`subplots`](crate::core::subplot::subplots) returns, so the figure-level
/// chain is the one you already know — `.suptitle(..)`, `.theme(..)`,
/// `.save(..)` — and you can [`add_axes`](SubplotFigure::add_axes) more panels
/// onto it.
///
/// The three panels share one set of limits: the marginals are drawn against
/// the *same* x (respectively y) range as the main panel, so a peak in a
/// marginal lines up with the points that produced it.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::plots::composite::jointplot;
///
/// let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
/// let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();
///
/// jointplot(&x, &y, 800, 800)?
///     .suptitle("x vs y")
///     .save("jointplot.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn jointplot<X, Y>(x: &X, y: &Y, width: u32, height: u32) -> Result<SubplotFigure>
where
    X: NumericData1D,
    Y: NumericData1D,
{
    jointplot_with(x, y, width, height, JointPlotConfig::default())
}

/// Draw a joint plot with an explicit [`JointPlotConfig`].
///
/// Spelled like the rest of the crate's `_with` entry points
/// ([`histogram_with`](crate::core::Plot::histogram_with),
/// [`boxplot_with`](crate::core::Plot::boxplot_with)): the bare name takes the
/// defaults, the `_with` name takes the config.
///
/// # Errors
///
/// Returns [`PlottingError::InvalidInput`] for a [`JointKind`] that has no
/// renderer yet — `Reg`, `Kde` and `Resid`. Substituting a scatter for them
/// would be the silent no-op this crate refuses to ship.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::plots::composite::{JointKind, JointPlotConfig, jointplot_with};
///
/// let x: Vec<f64> = (0..500).map(|i| (i % 37) as f64).collect();
/// let y: Vec<f64> = (0..500).map(|i| (i % 23) as f64).collect();
///
/// jointplot_with(
///     &x,
///     &y,
///     800,
///     800,
///     JointPlotConfig::new()
///         .kind(JointKind::Hex)
///         .rugplot(true)
///         .bins(24),
/// )?
/// .save("jointplot_hex.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn jointplot_with<X, Y>(
    x: &X,
    y: &Y,
    width: u32,
    height: u32,
    config: JointPlotConfig,
) -> Result<SubplotFigure>
where
    X: NumericData1D,
    Y: NumericData1D,
{
    let xs = x.try_collect_f64()?;
    let ys = y.try_collect_f64()?;
    if xs.len() != ys.len() {
        return Err(PlottingError::DataLengthMismatch {
            x_len: xs.len(),
            y_len: ys.len(),
            series_index: None,
        });
    }
    if xs.is_empty() {
        return Err(PlottingError::EmptyDataSet);
    }

    let color = resolved_color(config.color);
    let x_range = padded_range(&xs);
    let y_range = padded_range(&ys);
    let layout = joint_plot_layout(config.marginal_ratio);

    let mut assembled = figure(width, height)?.add_axes(
        layout.main_bounds,
        main_panel(&xs, &ys, x_range, y_range, &config, color)?,
    )?;

    let marginal = |values: &[f64], range, axis| {
        Marginal {
            values,
            range,
            axis,
            hist: config.marginal_hist,
            kde: config.marginal_kde,
            rug: config.rugplot,
            bins: config.bins,
            color,
        }
        .axes()
    };

    if let Some(top) = marginal(&xs, x_range, MarginalAxis::X) {
        assembled = assembled.add_axes(layout.x_marginal_bounds, top)?;
    }
    if let Some(right) = marginal(&ys, y_range, MarginalAxis::Y) {
        assembled = assembled.add_axes(layout.y_marginal_bounds, right)?;
    }

    Ok(assembled)
}

fn main_panel<D: NumericData1D>(
    xs: &D,
    ys: &D,
    x_range: (f64, f64),
    y_range: (f64, f64),
    config: &JointPlotConfig,
    color: Color,
) -> Result<Plot> {
    let drawn: Plot = match config.kind {
        JointKind::Scatter => panel()
            .scatter(xs, ys)
            .marker_size(config.scatter_size)
            .alpha(config.scatter_alpha)
            .color(color)
            .into_plot(),
        JointKind::Hex => panel().hexbin(xs, ys).into_plot(),
        unsupported => {
            return Err(PlottingError::InvalidInput(format!(
                "JointKind::{unsupported:?} has no renderer yet, so a joint plot \
                 cannot draw it. Use JointKind::Scatter or JointKind::Hex."
            )));
        }
    };

    Ok(drawn.xlim(x_range.0, x_range.1).ylim(y_range.0, y_range.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..120).map(|i| (i as f64) * 0.1).collect();
        let y: Vec<f64> = x.iter().map(|v| (v * 0.7).sin() * 3.0).collect();
        (x, y)
    }

    #[test]
    fn test_joint_plot_layout() {
        let layout = joint_plot_layout(0.2);

        // Main plot should take most space
        assert!(layout.main_bounds.width > 0.5);
        assert!(layout.main_bounds.height > 0.5);

        // Marginals should be smaller
        assert!(layout.x_marginal_bounds.height < 0.3);
        assert!(layout.y_marginal_bounds.width < 0.3);
    }

    #[test]
    fn the_three_panels_line_up_and_stay_inside_the_figure() {
        let layout = joint_plot_layout(0.2);
        for rect in [
            layout.main_bounds,
            layout.x_marginal_bounds,
            layout.y_marginal_bounds,
        ] {
            rect.validate().expect("every panel must be drawable");
        }

        // The top marginal shares the main panel's x extent, the right
        // marginal its y extent — which is what makes their shared axes land
        // in the same place once `panel_config` gives them equal margins.
        assert_eq!(layout.main_bounds.x, layout.x_marginal_bounds.x);
        assert_eq!(layout.main_bounds.width, layout.x_marginal_bounds.width);
        assert_eq!(layout.main_bounds.y, layout.y_marginal_bounds.y);
        assert_eq!(layout.main_bounds.height, layout.y_marginal_bounds.height);
    }

    #[test]
    fn test_marginal_histogram() {
        let data = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
        let hist = compute_marginal_histogram(&data, 3);

        assert_eq!(hist.counts.len(), 3);
        assert_eq!(hist.edges.len(), 4);
        assert_eq!(hist.centers.len(), 3);

        // Total count should equal data length
        let total: usize = hist.counts.iter().sum();
        assert_eq!(total, 7);
    }

    #[test]
    fn test_marginal_histogram_empty() {
        let data: Vec<f64> = vec![];
        let hist = compute_marginal_histogram(&data, 10);

        assert!(hist.edges.is_empty());
        assert!(hist.counts.is_empty());
    }

    #[test]
    fn marginal_histogram_density_integrates_to_one() {
        let data: Vec<f64> = (0..100).map(|i| i as f64 * 0.25).collect();
        let hist = compute_marginal_histogram(&data, 10);
        let bin_width = hist.edges[1] - hist.edges[0];
        let mass: f64 = hist.density().iter().map(|d| d * bin_width).sum();

        assert!(
            (mass - 1.0).abs() < 1e-9,
            "a density-normalised histogram must integrate to 1, got {mass}"
        );
    }

    #[test]
    fn the_staircase_starts_and_ends_on_the_baseline() {
        let data = vec![0.0, 1.0, 1.0, 2.0];
        let hist = compute_marginal_histogram(&data, 2);
        let (values, density) = hist.staircase();

        assert_eq!(values.len(), density.len());
        assert_eq!(values.len(), 2 * (hist.counts.len() + 1));
        assert_eq!(density.first(), Some(&0.0));
        assert_eq!(density.last(), Some(&0.0));
        assert_eq!(values.first(), hist.edges.first());
        assert_eq!(values.last(), hist.edges.last());
    }

    #[test]
    fn a_marginal_places_the_same_geometry_on_whichever_axis_it_runs() {
        let (x, _) = sample();
        let spec = |axis| Marginal {
            values: &x,
            range: padded_range(&x),
            axis,
            hist: true,
            kde: false,
            rug: false,
            bins: 12,
            color: Color::from_rgb(1, 2, 3),
        };

        assert!(spec(MarginalAxis::X).axes().is_some());
        assert!(spec(MarginalAxis::Y).axes().is_some());

        // The mapping is the only difference between the two.
        let values = vec![1.0, 2.0];
        let density = vec![10.0, 20.0];
        assert_eq!(
            MarginalAxis::X.place(values.clone(), density.clone()),
            (values.clone(), density.clone())
        );
        assert_eq!(
            MarginalAxis::Y.place(values.clone(), density.clone()),
            (density, values)
        );
    }

    #[test]
    fn a_marginal_with_nothing_switched_on_is_no_panel_at_all() {
        let (x, _) = sample();
        let blank = Marginal {
            values: &x,
            range: padded_range(&x),
            axis: MarginalAxis::X,
            hist: false,
            kde: false,
            rug: false,
            bins: 12,
            color: Color::from_rgb(1, 2, 3),
        };
        assert!(blank.axes().is_none());
        assert!(
            Marginal { rug: true, ..blank }.axes().is_some(),
            "rugplot alone must still produce a panel"
        );
        let no_observations: &[f64] = &[];
        assert!(
            Marginal {
                values: no_observations,
                hist: true,
                ..blank
            }
            .axes()
            .is_none()
        );
    }

    #[test]
    fn jointplot_assembles_a_main_panel_and_two_marginals() {
        let (x, y) = sample();
        let figure = jointplot(&x, &y, 600, 600).unwrap();

        assert_eq!(figure.axes_count(), 3);
        assert_eq!(figure.subplot_count(), 0);
        assert_eq!(figure.grid_spec().total_subplots(), 0);
    }

    #[test]
    fn switching_the_marginals_off_drops_their_panels() {
        let (x, y) = sample();
        let config = JointPlotConfig::new()
            .marginal_hist(false)
            .marginal_kde(false);
        let figure = jointplot_with(&x, &y, 600, 600, config).unwrap();

        assert_eq!(
            figure.axes_count(),
            1,
            "with no histogram, no KDE and no rug there is nothing to draw in a marginal"
        );
    }

    #[test]
    fn rugplot_is_no_longer_inert() {
        let (x, y) = sample();
        let quiet = JointPlotConfig::new()
            .marginal_hist(false)
            .marginal_kde(false);
        let loud = quiet.clone().rugplot(true);

        assert_eq!(
            jointplot_with(&x, &y, 400, 400, quiet)
                .unwrap()
                .axes_count(),
            1
        );
        assert_eq!(
            jointplot_with(&x, &y, 400, 400, loud).unwrap().axes_count(),
            3,
            "`rugplot(true)` must bring both marginal panels back"
        );
    }

    #[test]
    fn jointplot_rejects_input_it_cannot_draw() {
        let (x, y) = sample();

        assert!(matches!(
            jointplot(&x, &y[..10].to_vec(), 400, 400),
            Err(PlottingError::DataLengthMismatch { .. })
        ));
        assert!(matches!(
            jointplot(&Vec::<f64>::new(), &Vec::<f64>::new(), 400, 400),
            Err(PlottingError::EmptyDataSet)
        ));
        for kind in [JointKind::Reg, JointKind::Kde, JointKind::Resid] {
            let err = jointplot_with(&x, &y, 400, 400, JointPlotConfig::new().kind(kind))
                .expect_err("an unrendered kind must be reported, not substituted");
            assert!(matches!(err, PlottingError::InvalidInput(_)), "{kind:?}");
        }
    }

    #[test]
    fn jointplot_renders_all_three_panels() {
        fn ink_rows(image: &image::RgbaImage) -> (usize, usize) {
            let mut top = 0;
            let mut bottom = 0;
            for (_, y, pixel) in image.enumerate_pixels() {
                if pixel.0[..3].iter().all(|channel| *channel > 245) {
                    continue;
                }
                if y < image.height() / 4 {
                    top += 1;
                } else {
                    bottom += 1;
                }
            }
            (top, bottom)
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jointplot.png");
        let (x, y) = sample();

        jointplot_with(
            &x,
            &y,
            600,
            600,
            JointPlotConfig::new().rugplot(true).bins(15),
        )
        .unwrap()
        .save(&path)
        .unwrap();

        let image = image::open(&path).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (600, 600));
        let (top, bottom) = ink_rows(&image);
        assert!(top > 200, "the top marginal must draw something, ink={top}");
        assert!(
            bottom > 2000,
            "the main and right panels must draw something, ink={bottom}"
        );
    }
}
