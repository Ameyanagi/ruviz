//! The crate's single 2D data-bounds routine.
//!
//! Every 2D axis range comes out of [`BoundsAccumulator`] and the `impl Plot`
//! block below. This module was split out of the former `parallel_render.rs`
//! when the unreachable series-parallel 2D renderer was deleted; the bounds
//! code was the only live tenant of that file.

use super::*;
use crate::{core::Orientation, plots::BarOrientation};

// ===========================================================================
// Data bounds — one accumulator, one routine, one annotation pass
// ===========================================================================
//
// Every 2D axis range in the crate comes out of [`BoundsAccumulator`]. There is
// exactly one implementation per plot type, exactly one place where
// annotations are folded in, and exactly one place where a plot type declares
// its sticky edges. A new plot type therefore cannot end up with bounds that
// differ between the raw-series path, the resolved-frame path and the SVG
// path — which is how the three previous near-clones of this code drifted.

/// The axis edges a plot type pins by construction (matplotlib `sticky_edges`).
///
/// The autoscale margin (`Plot::apply_autoscale_margins`) is the only consumer:
/// it asks the series what they pin instead of re-deriving the rule from
/// `matches!` chains of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StickyEdges {
    /// The `x = 0` baseline must keep touching whichever x edge it lands on.
    pub(super) x_zero_baseline: bool,
    /// The `y = 0` baseline must keep touching whichever y edge it lands on.
    pub(super) y_zero_baseline: bool,
    /// Every edge is pinned: the series fills its axes edge to edge.
    pub(super) all_edges: bool,
    /// Bounds are chosen by construction, so no autoscale margin applies.
    pub(super) by_construction: bool,
}

impl StickyEdges {
    /// An ordinary Cartesian series: nothing pinned, margin on all four sides.
    pub(super) const NONE: Self = Self {
        x_zero_baseline: false,
        y_zero_baseline: false,
        all_edges: false,
        by_construction: false,
    };

    /// Bars and histograms: the zero baseline they are drawn from is sticky.
    const Y_ZERO_BASELINE: Self = Self {
        x_zero_baseline: false,
        y_zero_baseline: true,
        all_edges: false,
        by_construction: false,
    };

    /// Horizontal bars pin their value-axis baseline at `x = 0`.
    const X_ZERO_BASELINE: Self = Self {
        x_zero_baseline: true,
        y_zero_baseline: false,
        all_edges: false,
        by_construction: false,
    };

    /// Grid-sampled fields: `imshow`/`ContourSet` reach the spines exactly.
    const ALL_EDGES: Self = Self {
        x_zero_baseline: false,
        y_zero_baseline: false,
        all_edges: true,
        by_construction: false,
    };

    /// Pie/radar/polar: the bounds already include their own label ring.
    ///
    /// Also the identity of [`Self::union`]'s `by_construction` fold, so a plot
    /// with no series at all keeps its default axes untouched.
    pub(super) const BY_CONSTRUCTION: Self = Self {
        x_zero_baseline: false,
        y_zero_baseline: false,
        all_edges: false,
        by_construction: true,
    };

    /// Combine the edges pinned by two series sharing one pair of axes.
    ///
    /// Pinning is contagious (`||`) because one sticky series is enough to
    /// forbid a margin band on that edge, but "the bounds are entirely
    /// self-determined" only survives if *every* series says so (`&&`).
    fn union(self, other: Self) -> Self {
        Self {
            x_zero_baseline: self.x_zero_baseline || other.x_zero_baseline,
            y_zero_baseline: self.y_zero_baseline || other.y_zero_baseline,
            all_edges: self.all_edges || other.all_edges,
            by_construction: self.by_construction && other.by_construction,
        }
    }
}

/// The sticky edges of one plot type — the single source of truth for the rule.
///
/// This is the trait-shaped hook the audit asked for; it lives next to the
/// bounds code so that adding a plot type means answering "what does it pin?"
/// in the same match that answers "what is its extent?".
fn sticky_edges_of(series_type: &SeriesType) -> StickyEdges {
    match series_type {
        // Bars and histograms include the zero baseline in their extent and
        // must keep sitting exactly on it.
        SeriesType::Bar { config, .. } => match config.orientation {
            BarOrientation::Vertical => StickyEdges::Y_ZERO_BASELINE,
            BarOrientation::Horizontal => StickyEdges::X_ZERO_BASELINE,
        },
        SeriesType::Histogram { .. } => StickyEdges::Y_ZERO_BASELINE,
        // Grid-sampled fields fill the axes by construction: `imshow` marks all
        // four edges sticky and `ContourSet` calls `autoscale_view(tight=True)`.
        // Without this a filled contour floats inside a bare gutter.
        SeriesType::Heatmap { .. } | SeriesType::Contour { .. } => StickyEdges::ALL_EDGES,
        // These reserve their own label ring inside `add_computed_series`;
        // padding them again would shrink the figure a second time.
        SeriesType::Pie { .. } | SeriesType::Radar { .. } | SeriesType::Polar { .. } => {
            StickyEdges::BY_CONSTRUCTION
        }
        // A computed series answers for itself — grouped and stacked bars are
        // bar-shaped and pin the baseline the same way `SeriesType::Bar` does.
        SeriesType::Computed { data } if data.pins_zero_baseline() => {
            match data.category_orientation() {
                Orientation::Vertical => StickyEdges::Y_ZERO_BASELINE,
                Orientation::Horizontal => StickyEdges::X_ZERO_BASELINE,
            }
        }
        _ => StickyEdges::NONE,
    }
}

/// Error bars attached to a series with `with_yerr` / `with_xerr`.
///
/// They live on `PlotSeries`, not on the series' data, so a bounds view that
/// only sees resolved values has to be handed them explicitly — otherwise the
/// whiskers fall outside the axis range and get clipped.
#[derive(Clone, Copy, Default)]
struct AttachedErrors<'a> {
    x: Option<&'a ErrorValues>,
    y: Option<&'a ErrorValues>,
}

impl<'a> AttachedErrors<'a> {
    fn of(series: &'a PlotSeries) -> Self {
        Self {
            x: series.x_errors.as_ref(),
            y: series.y_errors.as_ref(),
        }
    }
}

/// Running min/max over every series, annotation and error bar on one plot.
///
/// The accumulator carries the plot's [`AxisScale`]s and admits a coordinate
/// only when [`AxisScale::is_valid_value`] accepts it — the same predicate the
/// projection uses to decide whether a sample has a position at all. So the
/// axis range is exactly the range of the samples that will be drawn: a bar's
/// zero baseline or a non-positive data point does not drag a logarithmic axis
/// down to a value it cannot represent, which is what used to make
/// `.bar(..).yscale(Log)` fail range validation before it ever drew anything.
///
/// On a linear or symlog axis `is_valid_value` is exactly `is_finite`, so this
/// is bit-identical to the previous behaviour there.
#[derive(Debug, Clone, Copy)]
struct BoundsAccumulator {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: AxisScale,
    y_scale: AxisScale,
    /// A finite coordinate was offered that this axis' scale cannot represent
    /// — a zero or negative sample on a log axis. Used to tell "the plot has no
    /// data" apart from "the plot's data does not fit on the axis it was given",
    /// which deserve different answers.
    x_rejected_by_scale: bool,
    y_rejected_by_scale: bool,
}

impl BoundsAccumulator {
    fn new(x_scale: AxisScale, y_scale: AxisScale) -> Self {
        Self {
            x_min: f64::INFINITY,
            x_max: f64::NEG_INFINITY,
            y_min: f64::INFINITY,
            y_max: f64::NEG_INFINITY,
            x_scale,
            y_scale,
            x_rejected_by_scale: false,
            y_rejected_by_scale: false,
        }
    }

    fn from_bounds(bounds: (f64, f64, f64, f64), x_scale: AxisScale, y_scale: AxisScale) -> Self {
        Self {
            x_min: bounds.0,
            x_max: bounds.1,
            y_min: bounds.2,
            y_max: bounds.3,
            x_scale,
            y_scale,
            x_rejected_by_scale: false,
            y_rejected_by_scale: false,
        }
    }

    /// The axis, if any, that was offered data it cannot represent and kept
    /// none of it. Returns the axis name and the setter that produced it.
    fn axis_with_no_representable_data(&self) -> Option<(&'static str, &'static str)> {
        if self.x_rejected_by_scale && !(self.x_min.is_finite() && self.x_max.is_finite()) {
            return Some(("x", "xscale"));
        }
        if self.y_rejected_by_scale && !(self.y_min.is_finite() && self.y_max.is_finite()) {
            return Some(("y", "yscale"));
        }
        None
    }

    fn bounds(&self) -> (f64, f64, f64, f64) {
        (self.x_min, self.x_max, self.y_min, self.y_max)
    }

    /// The accumulated bounds, or `None` if no series contributed a finite one.
    fn finite_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        (self.x_min.is_finite()
            && self.x_max.is_finite()
            && self.y_min.is_finite()
            && self.y_max.is_finite())
        .then(|| self.bounds())
    }

    fn include_x(&mut self, x: f64) {
        if self.x_scale.is_valid_value(x) {
            self.x_min = self.x_min.min(x);
            self.x_max = self.x_max.max(x);
        } else if x.is_finite() {
            self.x_rejected_by_scale = true;
        }
    }

    fn include_y(&mut self, y: f64) {
        if self.y_scale.is_valid_value(y) {
            self.y_min = self.y_min.min(y);
            self.y_max = self.y_max.max(y);
        } else if y.is_finite() {
            self.y_rejected_by_scale = true;
        }
    }

    fn include_point(&mut self, x: f64, y: f64) {
        self.include_x(x);
        self.include_y(y);
    }

    /// Include both endpoints of a span; the order of the arguments is free.
    fn include_x_span(&mut self, a: f64, b: f64) {
        self.include_x(a);
        self.include_x(b);
    }

    fn include_y_span(&mut self, a: f64, b: f64) {
        self.include_y(a);
        self.include_y(b);
    }

    fn include_plot_data<T: crate::plots::traits::PlotData + ?Sized>(&mut self, data: &T) {
        let ((x_min, x_max), (y_min, y_max)) = crate::plots::traits::PlotData::data_bounds(data);
        self.include_x_span(x_min, x_max);
        self.include_y_span(y_min, y_max);
    }

    // -- per-plot-type extents -------------------------------------------

    /// Points, each optionally widened by its error bar.
    ///
    /// Line, scatter and both dedicated error-bar series share this routine: an
    /// error bar is just a point with an extent. Folding the extent in here is
    /// what keeps `with_yerr` whiskers inside the axis range instead of clipped
    /// against the spine.
    fn add_points_with_errors(
        &mut self,
        x: &[f64],
        y: &[f64],
        x_errors: Option<ErrorValuesRef<'_>>,
        y_errors: Option<ErrorValuesRef<'_>>,
    ) {
        for (index, (&x_value, &y_value)) in x.iter().zip(y.iter()).enumerate() {
            if x_value.is_finite() {
                match finite_error_at(x_errors, index) {
                    Some((lower, upper)) => self.include_x_span(x_value - lower, x_value + upper),
                    None => self.include_x(x_value),
                }
            }
            if y_value.is_finite() {
                match finite_error_at(y_errors, index) {
                    Some((lower, upper)) => self.include_y_span(y_value - lower, y_value + upper),
                    None => self.include_y(y_value),
                }
            }
        }
    }

    /// Categorical bars, with matplotlib's half-category padding on each side
    /// so the first and last bar are fully inside the axes.
    fn add_bars(
        &mut self,
        category_count: usize,
        values: &[f64],
        config: &crate::plots::basic::BarConfig,
    ) {
        let full_slot_span: (f64, f64) = (-0.5, category_count as f64 - 0.5);
        let actual_slot_span = if config.align_left {
            (
                0.0,
                category_count.saturating_sub(1) as f64 + f64::from(config.width),
            )
        } else {
            let half_width = f64::from(config.width) / 2.0;
            (
                -half_width,
                category_count.saturating_sub(1) as f64 + half_width,
            )
        };
        let category_span = (
            full_slot_span.0.min(actual_slot_span.0),
            full_slot_span.1.max(actual_slot_span.1),
        );

        match config.orientation {
            crate::plots::basic::BarOrientation::Vertical => {
                self.include_x_span(category_span.0, category_span.1);
                for &value in values {
                    if value.is_finite() {
                        self.include_y_span(value.min(config.bottom), value.max(config.bottom));
                    }
                }
            }
            crate::plots::basic::BarOrientation::Horizontal => {
                self.include_y_span(category_span.0, category_span.1);
                for &value in values {
                    if value.is_finite() {
                        self.include_x_span(value.min(config.bottom), value.max(config.bottom));
                    }
                }
            }
        }
    }

    fn add_histogram(&mut self, data: &crate::plots::histogram::HistogramData) {
        if let (Some(&first), Some(&last)) = (data.bin_edges.first(), data.bin_edges.last()) {
            self.include_x_span(first, last);
        }
        // Bars are drawn from the baseline, so the baseline is part of the data.
        self.include_y(0.0);
        for &count in &data.counts {
            if count > 0.0 {
                self.include_y(count);
            }
        }
    }

    fn add_box_plot(
        &mut self,
        data: &[f64],
        config: &crate::plots::boxplot::BoxPlotConfig,
    ) -> Result<()> {
        if data.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        // One box occupies the one-unit-wide category slot it was assigned, the
        // same slot geometry a bar chart uses.
        let (lo, hi) = crate::plots::boxplot::category_slot_span(config.x_center());
        self.include_x_span(lo, hi);
        for &value in data {
            self.include_y(value);
        }
        Ok(())
    }

    /// Series whose geometry lives in their own precomputed data type.
    ///
    /// These carry no reactive `PlotData`, so both the raw and the resolved
    /// views reach exactly this code.
    fn add_computed_series(&mut self, series_type: &SeriesType) {
        match series_type {
            SeriesType::Heatmap { data } => self.include_plot_data(data.as_ref()),
            SeriesType::Boxen { data } => self.include_plot_data(data.as_ref()),
            // Every `ComputedSeries` states its own extent through `PlotData`,
            // which is the same route heatmap and boxen take — so a plot type
            // wired through `SeriesType::Computed` needs no arm of its own here.
            SeriesType::Computed { data } => self.include_plot_data(data.as_ref()),
            SeriesType::Kde { data } => {
                self.add_points_with_errors(&data.x, &data.y, None, None);
                // Density curves are filled down to zero.
                self.include_y(0.0);
            }
            SeriesType::Ecdf { data } => {
                self.add_points_with_errors(&data.x, &data.y, None, None);
                // The step function starts at zero.
                self.include_y(0.0);
            }
            SeriesType::Violin { data } => {
                // The violin's value axis follows its KDE evaluation range,
                // which extends past the raw data by a few bandwidths.
                //
                // Every grid point is offered rather than just the two ends:
                // the grid is monotone, so on a linear axis this is exactly the
                // pair of endpoints, but on a log axis the low end can run past
                // the axis, and then the floor has to be the smallest grid point
                // the axis can actually show.
                let (lo, hi) = crate::plots::boxplot::category_slot_span(data.config.x_center());
                match data.config.orientation {
                    crate::plots::distribution::Orientation::Vertical => {
                        self.include_x_span(lo, hi);
                        if data.kde.x.is_empty() {
                            self.include_y_span(data.range.0, data.range.1);
                        } else {
                            for &value in &data.kde.x {
                                self.include_y(value);
                            }
                        }
                    }
                    crate::plots::distribution::Orientation::Horizontal => {
                        self.include_y_span(lo, hi);
                        if data.kde.x.is_empty() {
                            self.include_x_span(data.range.0, data.range.1);
                        } else {
                            for &value in &data.kde.x {
                                self.include_x(value);
                            }
                        }
                    }
                }
            }
            SeriesType::Quiver { data } => {
                for arrow in &data.arrows {
                    for (x, y) in [
                        arrow.start,
                        arrow.end,
                        arrow.head[0],
                        arrow.head[1],
                        arrow.head[2],
                    ] {
                        self.include_point(x, y);
                    }
                }
            }
            SeriesType::Contour { data } => {
                for &x in &data.x {
                    self.include_x(x);
                }
                for &y in &data.y {
                    self.include_y(y);
                }
            }
            SeriesType::Pie { .. } => {
                // Pie charts draw into a normalised unit square.
                self.include_x_span(0.0, 1.0);
                self.include_y_span(0.0, 1.0);
            }
            SeriesType::Radar { .. } => {
                // Radar polygons never leave the unit circle, but the axis
                // labels sit further out, so reserve the labelled square rather
                // than scanning polygon vertices. Both backends use the same
                // constant.
                let radius = crate::plots::polar::radar::RADAR_BOUNDS_RADIUS;
                self.include_x_span(-radius, radius);
                self.include_y_span(-radius, radius);
            }
            SeriesType::Polar { data } => {
                // Polar plots need a symmetric square centred on the origin, so
                // an asymmetric curve (a cardioid, say) still renders centred
                // with room for its labels. This *replaces* the range on
                // purpose: the sample points must not pull the centre off.
                let label_margin = data.bounds_radius();
                self.x_min = -label_margin;
                self.x_max = label_margin;
                self.y_min = -label_margin;
                self.y_max = label_margin;
            }
            SeriesType::Line { .. }
            | SeriesType::Scatter { .. }
            | SeriesType::Bar { .. }
            | SeriesType::ErrorBars { .. }
            | SeriesType::ErrorBarsXY { .. }
            | SeriesType::Histogram { .. }
            | SeriesType::BoxPlot { .. } => {
                unreachable!("data-carrying series are accumulated from their resolved values")
            }
        }
    }

    /// Fold in every annotation drawn in data coordinates.
    ///
    /// Called from exactly one place (`Plot::finish_bounds`) so no caller can
    /// forget it and silently clip an `HSpan` or a `FillBetween`.
    fn include_annotations(&mut self, annotations: &[Annotation]) {
        for annotation in annotations {
            match annotation {
                Annotation::Text { x, y, .. } => self.include_point(*x, *y),
                Annotation::Arrow { x1, y1, x2, y2, .. } => {
                    self.include_point(*x1, *y1);
                    self.include_point(*x2, *y2);
                }
                Annotation::HLine { y, .. } => self.include_y(*y),
                Annotation::VLine { x, .. } => self.include_x(*x),
                Annotation::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    self.include_point(*x, *y);
                    self.include_point(*x + *width, *y + *height);
                }
                Annotation::FillBetween { x, y1, y2, .. } => {
                    for ((&x_value, &y1_value), &y2_value) in x.iter().zip(y1).zip(y2) {
                        self.include_point(x_value, y1_value);
                        self.include_y(y2_value);
                    }
                }
                Annotation::HSpan { x_min, x_max, .. } => self.include_x_span(*x_min, *x_max),
                Annotation::VSpan { y_min, y_max, .. } => self.include_y_span(*y_min, *y_max),
            }
        }
    }
}

/// The `(lower, upper)` extent of an error bar, or `None` when it has no usable
/// value at `index` — a short/absent error array must leave the point itself in
/// the bounds rather than dropping it.
fn finite_error_at(errors: Option<ErrorValuesRef<'_>>, index: usize) -> Option<(f64, f64)> {
    errors
        .and_then(|errors| errors.bounds_at(index))
        .filter(|(lower, upper)| lower.is_finite() && upper.is_finite())
}

/// One series' contribution to a plot's data bounds.
///
/// Implemented for every shape a caller has on hand — a raw `PlotSeries`, a
/// resolved frame entry, or the two paired up — so all of them run the *same*
/// per-plot-type code in [`BoundsAccumulator`]. Prefer the paired form: it is
/// the only one that can see error bars attached with `with_yerr`/`with_xerr`.
trait SeriesBoundsSource {
    fn accumulate_bounds(&self, acc: &mut BoundsAccumulator) -> Result<()>;
}

impl<T: SeriesBoundsSource + ?Sized> SeriesBoundsSource for &T {
    fn accumulate_bounds(&self, acc: &mut BoundsAccumulator) -> Result<()> {
        (**self).accumulate_bounds(acc)
    }
}

impl SeriesBoundsSource for PlotSeries {
    fn accumulate_bounds(&self, acc: &mut BoundsAccumulator) -> Result<()> {
        let attached = AttachedErrors::of(self);
        match &self.series_type {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                acc.add_points_with_errors(
                    &x_data.resolve_cow(0.0),
                    &y_data.resolve_cow(0.0),
                    attached.x.map(ErrorValuesRef::from),
                    attached.y.map(ErrorValuesRef::from),
                );
            }
            SeriesType::Bar {
                categories,
                values,
                config,
            } => acc.add_bars(categories.len(), &values.resolve_cow(0.0), config),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                let y_errors = y_errors.resolve_cow(0.0);
                acc.add_points_with_errors(
                    &x_data.resolve_cow(0.0),
                    &y_data.resolve_cow(0.0),
                    attached.x.map(ErrorValuesRef::from),
                    Some(effective_error_values(attached.y, &y_errors)),
                );
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                let x_errors = x_errors.resolve_cow(0.0);
                let y_errors = y_errors.resolve_cow(0.0);
                acc.add_points_with_errors(
                    &x_data.resolve_cow(0.0),
                    &y_data.resolve_cow(0.0),
                    Some(effective_error_values(attached.x, &x_errors)),
                    Some(effective_error_values(attached.y, &y_errors)),
                );
            }
            SeriesType::Histogram { .. } => {
                // A histogram that cannot be binned contributes no extent; the
                // render path reports the error.
                if let Ok(data) = self.series_type.histogram_data_at(0.0) {
                    acc.add_histogram(&data);
                }
            }
            SeriesType::BoxPlot { data, config } => {
                acc.add_box_plot(&data.resolve_cow(0.0), config)?
            }
            series_type => acc.add_computed_series(series_type),
        }
        Ok(())
    }
}

impl ResolvedSeries<'_> {
    fn accumulate_bounds_with(
        &self,
        acc: &mut BoundsAccumulator,
        attached: AttachedErrors<'_>,
    ) -> Result<()> {
        match self {
            ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y } => acc
                .add_points_with_errors(
                    x,
                    y,
                    attached.x.map(ErrorValuesRef::from),
                    attached.y.map(ErrorValuesRef::from),
                ),
            ResolvedSeries::Bar {
                categories,
                values,
                config,
            } => acc.add_bars(categories.len(), values, config),
            ResolvedSeries::ErrorBars { x, y, y_errors } => acc.add_points_with_errors(
                x,
                y,
                attached.x.map(ErrorValuesRef::from),
                Some(effective_error_values(attached.y, y_errors)),
            ),
            ResolvedSeries::ErrorBarsXY {
                x,
                y,
                x_errors,
                y_errors,
            } => acc.add_points_with_errors(
                x,
                y,
                Some(effective_error_values(attached.x, x_errors)),
                Some(effective_error_values(attached.y, y_errors)),
            ),
            ResolvedSeries::Histogram { data } => acc.add_histogram(data),
            ResolvedSeries::BoxPlot { data, config } => acc.add_box_plot(data, config)?,
            ResolvedSeries::Other(series_type) => acc.add_computed_series(series_type),
        }
        Ok(())
    }
}

impl SeriesBoundsSource for ResolvedSeries<'_> {
    fn accumulate_bounds(&self, acc: &mut BoundsAccumulator) -> Result<()> {
        self.accumulate_bounds_with(acc, AttachedErrors::default())
    }
}

impl SeriesBoundsSource for (&PlotSeries, &ResolvedSeries<'_>) {
    fn accumulate_bounds(&self, acc: &mut BoundsAccumulator) -> Result<()> {
        let (series, resolved) = *self;
        resolved.accumulate_bounds_with(acc, AttachedErrors::of(series))
    }
}

impl Plot {
    // =======================================================================
    // Data bounds
    // =======================================================================

    /// The union of the sticky edges declared by `series_list`.
    ///
    /// See [`StickyEdges`]; the autoscale margin consults this instead of
    /// re-deriving "is it a bar? is it a heatmap?" for itself.
    /// An empty list yields [`StickyEdges::BY_CONSTRUCTION`], the identity of
    /// the `&&`-fold: with no series there is nothing to autoscale, so the
    /// default axes must stay exactly as constructed rather than growing a
    /// margin band around nothing.
    pub(super) fn sticky_edges_for_series(series_list: &[PlotSeries]) -> StickyEdges {
        series_list
            .iter()
            .map(|series| sticky_edges_of(&series.series_type))
            .reduce(StickyEdges::union)
            .unwrap_or(StickyEdges::BY_CONSTRUCTION)
    }

    /// The sticky edges of this plot's own series.
    pub(super) fn sticky_edges(&self) -> StickyEdges {
        Self::sticky_edges_for_series(&self.series_mgr.series)
    }

    /// Widen a degenerate (`min == max`) range so the axis has a span to scale.
    fn normalize_degenerate_bounds(&self, bounds: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        let (x_min, x_max) =
            crate::axes::expand_degenerate_range(bounds.0, bounds.1, &self.layout.x_scale);
        let (y_min, y_max) =
            crate::axes::expand_degenerate_range(bounds.2, bounds.3, &self.layout.y_scale);
        (x_min, x_max, y_min, y_max)
    }

    /// Scan a set of series into a [`BoundsAccumulator`].
    ///
    /// This is *the* bounds routine. Every caller differs only in the view it
    /// supplies — raw series, resolved frame entries, or the two paired — and
    /// each view runs the same per-plot-type code, so a plot type cannot end up
    /// with subtly different bounds on different paths.
    fn accumulate_series_bounds<S: SeriesBoundsSource>(
        &self,
        series: impl IntoIterator<Item = S>,
    ) -> Result<BoundsAccumulator> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        let mut acc = BoundsAccumulator::new(self.layout.x_scale, self.layout.y_scale);
        for source in series {
            source.accumulate_bounds(&mut acc)?;
        }
        Ok(acc)
    }

    /// Turn a scan into the plot's axis range.
    ///
    /// Annotations are folded in here and nowhere else, so no caller can drop
    /// annotation-driven axis expansion and clip an `HSpan` or a `FillBetween`.
    ///
    /// Fails when an axis was offered data it cannot represent and nothing
    /// survived — every sample on a log axis was zero or negative. Falling back
    /// to the empty-plot placeholder there would draw a blank figure and say
    /// nothing, which is the silent-loss failure this accumulator exists to
    /// avoid. A plot with no data at all still gets the placeholder.
    fn finish_bounds(&self, mut acc: BoundsAccumulator) -> Result<(f64, f64, f64, f64)> {
        acc.include_annotations(&self.annotations);
        if let Some(bounds) = acc.finite_bounds() {
            return Ok(self.normalize_degenerate_bounds(bounds));
        }
        if let Some((axis, setter)) = acc.axis_with_no_representable_data() {
            let shared = crate::axes::scale::LOG_SCALE_REQUIRES_POSITIVE;
            return Err(PlottingError::InvalidInput(format!(
                "Invalid {axis}-axis range: {shared} \
                 (no sample can be placed on the logarithmic {axis} axis because every \
                 {axis} value is zero or negative. Remove `.{setter}(AxisScale::Log)`, use \
                 `.{setter}(AxisScale::SymLog {{ linthresh }})`, or supply positive data.)"
            )));
        }
        Ok(self.empty_cartesian_bounds())
    }

    /// Fold this plot's annotations into an already-computed range.
    ///
    /// Kept for callers that hold bounds from elsewhere; it is idempotent
    /// against `finish_bounds`, which has already applied it.
    pub(super) fn expand_bounds_with_annotations(
        &self,
        bounds: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let mut acc =
            BoundsAccumulator::from_bounds(bounds, self.layout.x_scale, self.layout.y_scale);
        acc.include_annotations(&self.annotations);
        self.normalize_degenerate_bounds(acc.bounds())
    }

    /// Data bounds across every series on the plot.
    pub(super) fn calculate_data_bounds(&self) -> Result<(f64, f64, f64, f64)> {
        self.calculate_data_bounds_for_series(&self.series_mgr.series)
    }

    /// Data bounds across an explicit list of series.
    pub(super) fn calculate_data_bounds_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<(f64, f64, f64, f64)> {
        let acc = self.accumulate_series_bounds(series_list)?;
        self.finish_bounds(acc)
    }

    /// Data bounds across the entries of a resolved frame.
    ///
    /// Prefer [`Self::calculate_data_bounds_for_frame`] where the originating
    /// series are also on hand: a resolved entry alone cannot see the error
    /// bars attached with `with_yerr`/`with_xerr`, so their whiskers would fall
    /// outside the range this returns.
    pub(super) fn calculate_data_bounds_from_resolved<'frame, 'data>(
        &self,
        resolved_series: impl IntoIterator<Item = &'frame ResolvedSeries<'data>>,
    ) -> Result<(f64, f64, f64, f64)>
    where
        'data: 'frame,
    {
        let acc = self.accumulate_series_bounds(resolved_series)?;
        self.finish_bounds(acc)
    }

    /// Data bounds for a resolved frame, including attached error bars.
    ///
    /// `series_list` and `resolved_series` must be the parallel lists produced
    /// by `resolve_frame`; extra entries on either side are ignored.
    pub(super) fn calculate_data_bounds_for_frame(
        &self,
        series_list: &[PlotSeries],
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        self.calculate_data_bounds_for_pairs(series_list.iter().zip(resolved_series))
    }

    /// Data bounds over an arbitrary selection of (series, resolved) pairs.
    ///
    /// Same routine as [`Self::calculate_data_bounds_for_frame`]; it exists so
    /// callers that must *filter* the frame — the mixed-coordinate main panel,
    /// which drops the polar/pie entries — can still pair each resolved entry
    /// with its originating series, and therefore still see attached error
    /// bars, without cloning either list.
    pub(super) fn calculate_data_bounds_for_pairs<'frame, 'data>(
        &self,
        pairs: impl IntoIterator<Item = (&'frame PlotSeries, &'frame ResolvedSeries<'data>)>,
    ) -> Result<(f64, f64, f64, f64)>
    where
        'data: 'frame,
    {
        let acc = self.accumulate_series_bounds(pairs)?;
        self.finish_bounds(acc)
    }

    /// The axis range a resolved frame renders into.
    ///
    /// Drop-in replacement for `effective_data_bounds_from_resolved` that also
    /// sees the error bars attached to each series, so `with_yerr` whiskers are
    /// inside the axes instead of clipped against the spine.
    pub(super) fn effective_frame_bounds(
        &self,
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        if resolved_series.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        self.calculate_data_bounds_for_frame(&self.series_mgr.series, resolved_series)
            .map(|bounds| self.apply_manual_axis_limits(bounds))
    }

    /// The raw extent of a single resolved series, with no annotation pass.
    ///
    /// Insets carry their own coordinate space, so plot-level annotations must
    /// not stretch them. Every other caller wants the annotated plot bounds.
    pub(super) fn inset_bounds_from_resolved(
        &self,
        resolved: &ResolvedSeries<'_>,
    ) -> Result<(f64, f64, f64, f64)> {
        let acc = self.accumulate_series_bounds(std::iter::once(resolved))?;
        Ok(match acc.finite_bounds() {
            Some(bounds) => self.normalize_degenerate_bounds(bounds),
            None => self.empty_cartesian_bounds(),
        })
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;

    /// The bug this whole collapse exists to prevent: three bounds routines
    /// that agree today and drift tomorrow. Every plot type is scanned through
    /// all three views and the answers must match exactly.
    fn assert_all_views_agree(plot: &Plot, what: &str) -> (f64, f64, f64, f64) {
        let series = plot.snapshot_series(0.0);
        let frame = plot.resolve_frame(0.0).expect("frame should resolve");

        let all = plot
            .calculate_data_bounds()
            .unwrap_or_else(|e| panic!("{what}: whole-plot bounds failed: {e}"));
        let listed = plot
            .calculate_data_bounds_for_series(&series)
            .unwrap_or_else(|e| panic!("{what}: per-series bounds failed: {e}"));
        let resolved = plot
            .calculate_data_bounds_from_resolved(&frame.series)
            .unwrap_or_else(|e| panic!("{what}: resolved bounds failed: {e}"));
        let paired = plot
            .calculate_data_bounds_for_frame(&plot.series_mgr.series, &frame.series)
            .unwrap_or_else(|e| panic!("{what}: paired bounds failed: {e}"));

        assert_eq!(all, listed, "{what}: series-list view diverged");
        assert_eq!(all, paired, "{what}: paired frame view diverged");
        // The resolved-only view cannot see attached error bars; everything
        // else about it must be identical.
        if plot
            .series_mgr
            .series
            .iter()
            .all(|s| s.x_errors.is_none() && s.y_errors.is_none())
        {
            assert_eq!(all, resolved, "{what}: resolved view diverged");
        }
        all
    }

    fn grid() -> Vec<Vec<f64>> {
        vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]
    }

    #[test]
    fn every_plot_type_agrees_across_all_bounds_views() {
        let x = [1.0, 2.0, 3.0];
        let y = [10.0, 20.0, 30.0];

        assert_all_views_agree(&Plot::new().line(&x, &y).into_plot(), "line");
        assert_all_views_agree(&Plot::new().scatter(&x, &y).into_plot(), "scatter");
        assert_all_views_agree(
            &Plot::new().bar(&["a", "b"], &[1.0, 2.0]).into_plot(),
            "bar",
        );
        assert_all_views_agree(
            &Plot::new()
                .histogram(&[1.0, 2.0, 2.0, 3.0, 4.0])
                .into_plot(),
            "histogram",
        );
        assert_all_views_agree(
            &Plot::new().boxplot(&[1.0, 2.0, 3.0, 4.0]).into_plot(),
            "boxplot",
        );
        assert_all_views_agree(
            &Plot::new().error_bars(&x, &y, &[1.0, 1.0, 1.0]).into_plot(),
            "error_bars",
        );
        assert_all_views_agree(
            &Plot::new()
                .error_bars_xy(&x, &y, &[0.5, 0.5, 0.5], &[1.0, 1.0, 1.0])
                .into_plot(),
            "error_bars_xy",
        );
        assert_all_views_agree(&Plot::new().heatmap(&grid()).into_plot(), "heatmap");
    }

    #[test]
    fn annotations_reach_every_bounds_view() {
        let plot = Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .into_plot()
            .axvspan(-5.0, 7.0);

        let (x_min, x_max, ..) = assert_all_views_agree(&plot, "hspan-annotated line");
        assert!(x_min <= -5.0, "hspan lower edge clipped: {x_min}");
        assert!(x_max >= 7.0, "hspan upper edge clipped: {x_max}");
    }

    #[test]
    fn attached_y_error_whiskers_are_inside_the_bounds() {
        let plot = Plot::new()
            .line(&[0.0, 1.0, 2.0], &[10.0, 10.0, 10.0])
            .with_yerr(&[2.0, 2.0, 2.0])
            .into_plot();

        let (_, _, y_min, y_max) = plot
            .calculate_data_bounds()
            .expect("bounds should resolve for a line with attached y errors");

        assert!(y_min <= 8.0, "lower whisker clipped: y_min = {y_min}");
        assert!(y_max >= 12.0, "upper whisker clipped: y_max = {y_max}");
    }

    #[test]
    fn attached_x_error_whiskers_are_inside_the_bounds() {
        let plot = Plot::new()
            .scatter(&[5.0], &[0.0])
            .with_xerr(&[3.0])
            .into_plot();

        let (x_min, x_max, ..) = plot
            .calculate_data_bounds()
            .expect("bounds should resolve for a scatter with attached x errors");

        assert!(x_min <= 2.0, "lower whisker clipped: x_min = {x_min}");
        assert!(x_max >= 8.0, "upper whisker clipped: x_max = {x_max}");
    }

    #[test]
    fn attached_asymmetric_errors_override_the_dedicated_series_values() {
        // The renderer honours the attached override, so the bounds must too.
        let plot = Plot::new()
            .error_bars(&[0.0], &[0.0], &[0.25])
            .with_yerr_asymmetric(&[0.5], &[1.5])
            .into_plot();

        let (_, _, y_min, y_max) = plot
            .calculate_data_bounds()
            .expect("bounds should resolve for overridden error bars");

        assert!(y_min <= -0.5, "override lower ignored: y_min = {y_min}");
        assert!(y_max >= 1.5, "override upper ignored: y_max = {y_max}");
    }

    #[test]
    fn attached_errors_are_folded_in_on_the_resolved_frame_path() {
        let plot = Plot::new()
            .line(&[0.0, 1.0], &[10.0, 10.0])
            .with_yerr(&[4.0, 4.0])
            .into_plot();
        let frame = plot.resolve_frame(0.0).expect("frame should resolve");

        let paired = plot
            .calculate_data_bounds_for_frame(&plot.series_mgr.series, &frame.series)
            .expect("paired bounds should resolve");

        assert!(paired.2 <= 6.0, "lower whisker clipped: {}", paired.2);
        assert!(paired.3 >= 14.0, "upper whisker clipped: {}", paired.3);

        // ...and the range the frame actually renders into keeps them too.
        let effective = plot
            .effective_frame_bounds(&frame.series)
            .expect("frame bounds should resolve");
        assert!(effective.2 <= 6.0, "lower whisker clipped: {}", effective.2);
        assert!(
            effective.3 >= 14.0,
            "upper whisker clipped: {}",
            effective.3
        );
    }

    #[test]
    fn horizontal_bar_bounds_put_values_on_x_and_categories_on_y() {
        let plot = Plot::new()
            .bar(&["first", "second"], &[12.0, 8.0])
            .horizontal()
            .bottom(10.0)
            .into_plot();

        let bounds = plot
            .calculate_data_bounds()
            .expect("horizontal bars should produce data bounds");

        assert_eq!(bounds, (8.0, 12.0, -0.5, 1.5));
    }

    #[test]
    fn horizontal_violin_bounds_put_values_on_x_and_its_slot_on_y() {
        use crate::plots::traits::PlotData as _;

        let plot = Plot::new()
            .violin(&[1.0, 2.0, 2.5, 3.0, 4.0])
            .horizontal()
            .x_position(1.0)
            .into_plot();
        let expected = match &plot.series_mgr.series[0].series_type {
            SeriesType::Violin { data } => {
                let ((x_min, x_max), (y_min, y_max)) = data.data_bounds();
                (x_min, x_max, y_min, y_max)
            }
            other => panic!("expected a violin, got {other:?}"),
        };

        assert_eq!(
            plot.calculate_data_bounds()
                .expect("horizontal violin bounds"),
            expected
        );
        assert_eq!((expected.2, expected.3), (0.5, 1.5));
    }

    #[test]
    fn insets_do_not_inherit_plot_level_annotations() {
        let plot = Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .into_plot()
            .axvspan(-100.0, 100.0);
        let frame = plot.resolve_frame(0.0).expect("frame should resolve");

        let inset = plot
            .inset_bounds_from_resolved(&frame.series[0])
            .expect("inset bounds should resolve");

        assert_eq!(inset, (0.0, 1.0, 0.0, 1.0));
    }

    #[test]
    fn sticky_edges_are_declared_once_per_plot_type() {
        let bars = Plot::new().bar(&["a"], &[1.0]).into_plot().sticky_edges();
        assert!(!bars.x_zero_baseline);
        assert!(bars.y_zero_baseline);
        assert!(!bars.all_edges);

        let horizontal_bars = Plot::new()
            .bar(&["a"], &[1.0])
            .horizontal()
            .into_plot()
            .sticky_edges();
        assert!(horizontal_bars.x_zero_baseline);
        assert!(!horizontal_bars.y_zero_baseline);

        let heatmap = Plot::new().heatmap(&grid()).into_plot().sticky_edges();
        assert!(heatmap.all_edges);

        let line = Plot::new()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .into_plot()
            .sticky_edges();
        assert_eq!(line, StickyEdges::NONE);

        // "By construction" only survives when every series agrees.
        let mixed = Plot::new()
            .pie(&[1.0, 2.0])
            .into_plot()
            .line(&[0.0, 1.0], &[0.0, 1.0])
            .into_plot()
            .sticky_edges();
        assert!(!mixed.by_construction);
    }

    /// The autoscale margin used to skip a seriesless plot via
    /// `!has_cartesian_series(&[])`, which is vacuously true. Folding sticky
    /// edges must reproduce that: `by_construction` is an `&&`-fold, so its
    /// identity is `true`, not `false`. Getting this wrong padded the default
    /// empty axes from `0..1` to `-0.05..1.05`.
    #[test]
    fn sticky_edges_of_a_seriesless_plot_pin_the_default_axes() {
        let empty = Plot::new().sticky_edges();
        assert_eq!(empty, StickyEdges::BY_CONSTRUCTION);
        assert!(empty.by_construction);

        let plot = Plot::new();
        assert_eq!(
            plot.apply_autoscale_margins((0.0, 1.0, 0.0, 1.0)),
            (0.0, 1.0, 0.0, 1.0),
            "an empty plot must not grow a margin band around nothing"
        );
    }
}
