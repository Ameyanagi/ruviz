use super::*;
use crate::core::Point2f;
use crate::core::plot::raster_fast_path::{
    canonicalize_line_points_exact, reduce_line_points_for_raster, should_reduce_line_series,
};

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
        y_zero_baseline: false,
        all_edges: false,
        by_construction: false,
    };

    /// Bars and histograms: the zero baseline they are drawn from is sticky.
    const ZERO_BASELINE: Self = Self {
        y_zero_baseline: true,
        all_edges: false,
        by_construction: false,
    };

    /// Grid-sampled fields: `imshow`/`ContourSet` reach the spines exactly.
    const ALL_EDGES: Self = Self {
        y_zero_baseline: false,
        all_edges: true,
        by_construction: false,
    };

    /// Pie/radar/polar: the bounds already include their own label ring.
    ///
    /// Also the identity of [`Self::union`]'s `by_construction` fold, so a plot
    /// with no series at all keeps its default axes untouched.
    pub(super) const BY_CONSTRUCTION: Self = Self {
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
        SeriesType::Bar { .. } | SeriesType::Histogram { .. } => StickyEdges::ZERO_BASELINE,
        // Grid-sampled fields fill the axes by construction: `imshow` marks all
        // four edges sticky and `ContourSet` calls `autoscale_view(tight=True)`.
        // Without this a filled contour floats inside a bare gutter.
        SeriesType::Heatmap { .. } | SeriesType::Contour { .. } => StickyEdges::ALL_EDGES,
        // These reserve their own label ring inside `add_computed_series`;
        // padding them again would shrink the figure a second time.
        SeriesType::Pie { .. } | SeriesType::Radar { .. } | SeriesType::Polar { .. } => {
            StickyEdges::BY_CONSTRUCTION
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
    fn add_bars(&mut self, category_count: usize, values: &[f64]) {
        self.include_x_span(-0.5, category_count as f64 - 0.5);
        for &value in values {
            if value.is_finite() {
                // Bars run from the zero baseline to the value, so both ends count.
                self.include_y_span(value.min(0.0), value.max(0.0));
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

    fn add_box_plot(&mut self, data: &[f64]) -> Result<()> {
        if data.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }
        // One box occupies the unit cell centred on 0.5.
        self.include_x_span(0.0, 1.0);
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
                // The violin is as tall as its KDE evaluation range, which
                // extends past the raw data by a few bandwidths.
                //
                // Every grid point is offered rather than just the two ends:
                // the grid is monotone, so on a linear axis this is exactly the
                // pair of endpoints, but on a log axis the low end can run past
                // the axis, and then the floor has to be the smallest grid point
                // the axis can actually show.
                if data.kde.x.is_empty() {
                    self.include_y_span(data.range.0, data.range.1);
                } else {
                    for &value in &data.kde.x {
                        self.include_y(value);
                    }
                }
                self.include_x_span(0.0, 1.0);
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
                let label_margin = data.r_max * 1.5;
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
                categories, values, ..
            } => acc.add_bars(categories.len(), &values.resolve_cow(0.0)),
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
            SeriesType::BoxPlot { data, .. } => acc.add_box_plot(&data.resolve_cow(0.0))?,
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
            ResolvedSeries::Bar { categories, values } => acc.add_bars(categories.len(), values),
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
            ResolvedSeries::BoxPlot { data, .. } => acc.add_box_plot(data)?,
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
    #[cfg(feature = "parallel")]
    pub(super) fn parallel_marker_size_px(&self, series: &PlotSeries, fallback_points: f32) -> f32 {
        self.render_scale()
            .points_to_pixels(series.marker_size.unwrap_or(fallback_points))
    }

    /// Render plot using parallel processing for multiple series
    #[cfg(feature = "parallel")]
    pub(super) fn render_with_parallel(&self) -> Result<Image> {
        let frame = self.resolve_frame(0.0)?;
        let style_shell = self.resolved_style_shell(&frame.style);
        let result = style_shell.render_with_parallel_resolved(&frame);
        if result.is_ok() {
            frame.acknowledge_rendered(self);
        }
        result
    }

    #[cfg(feature = "parallel")]
    pub(super) fn render_with_parallel_resolved(&self, frame: &ResolvedFrame<'_>) -> Result<Image> {
        use crate::render::parallel::{DataBounds, PlotArea, RenderSeriesType};

        let resolved_series = &frame.series;

        // Start timing for performance measurement
        let start_time = std::time::Instant::now();

        // Create renderer with DPI scaling
        let (scaled_width, scaled_height) = self.dpi_scaled_dimensions();
        let mut renderer = SkiaRenderer::with_font_family(
            scaled_width,
            scaled_height,
            self.display.theme.clone(),
            self.display.config.typography.family.clone(),
        )?;
        renderer.set_text_engine_mode(self.display.text_engine);
        renderer.note_parallel_render();
        let render_scale = self.render_scale();
        let dpi = render_scale.dpi();
        renderer.set_render_scale(render_scale);

        let bounds = self.effective_frame_bounds(resolved_series)?;
        self.validate_axis_scale_ranges_for_render(
            &self.series_mgr.series,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
        )?;

        let bar_categories: Option<Vec<String>> = self.series_mgr.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Compute content-driven layout FIRST for consistent positioning
        let content = self.create_plot_content_from_resolved_text(bounds.2, bounds.3, frame);
        let (layout, x_ticks, y_ticks) = self.compute_layout_with_configured_ticks(
            &renderer,
            (scaled_width, scaled_height),
            &content,
            dpi,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
        )?;

        // Convert layout plot_area to tiny_skia::Rect for series rendering
        let plot_area = Self::plot_area_from_layout(&layout).unwrap_or_else(|_| {
            // Fallback to simple calculation if layout rect is invalid
            calculate_plot_area_dpi(
                scaled_width,
                scaled_height,
                self.render_scale().reference_scale(),
            )
        });

        // Convert to parallel renderer format
        let parallel_plot_area = PlotArea {
            left: plot_area.left(),
            right: plot_area.right(),
            top: plot_area.top(),
            bottom: plot_area.bottom(),
        };
        let data_bounds = DataBounds {
            x_min: bounds.0,
            x_max: bounds.1,
            y_min: bounds.2,
            y_max: bounds.3,
        };

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| {
                Self::scaled_x_pixel(tick, bounds.0, bounds.1, plot_area, &self.layout.x_scale)
            })
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| {
                Self::scaled_y_pixel(tick, bounds.2, bounds.3, plot_area, &self.layout.y_scale)
            })
            .collect();
        let x_minor_ticks = Self::minor_tick_values_for_scale(
            &x_ticks,
            bounds.0,
            bounds.1,
            &self.layout.x_scale,
            self.layout.tick_config.minor_ticks_x,
        );
        let y_minor_ticks = Self::minor_tick_values_for_scale(
            &y_ticks,
            bounds.2,
            bounds.3,
            &self.layout.y_scale,
            self.layout.tick_config.minor_ticks_y,
        );
        let x_minor_tick_pixels: Vec<f32> = x_minor_ticks
            .iter()
            .map(|&tick| {
                Self::scaled_x_pixel(tick, bounds.0, bounds.1, plot_area, &self.layout.x_scale)
            })
            .collect();
        let y_minor_tick_pixels: Vec<f32> = y_minor_ticks
            .iter()
            .map(|&tick| {
                Self::scaled_y_pixel(tick, bounds.2, bounds.3, plot_area, &self.layout.y_scale)
            })
            .collect();

        // Draw grid if enabled - using unified GridStyle (sequential - UI elements)
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        if self.layout.grid_style.visible && self.needs_cartesian_axes() {
            let layers = Self::grid_layers(
                &self.layout.grid_style,
                &self.layout.tick_config.grid_mode,
                &x_tick_pixels,
                &y_tick_pixels,
                &x_minor_tick_pixels,
                &y_minor_tick_pixels,
                |points| self.dpi_scaled_line_width(points),
            );
            for layer in &layers {
                renderer.draw_grid(
                    &layer.x_pixels,
                    &layer.y_pixels,
                    plot_area,
                    layer.color,
                    self.layout.grid_style.line_style.clone(),
                    layer.width_px,
                )?;
            }
        }

        let categorical_x_tick_pixels = Self::categorical_x_tick_pixels(
            plot_area,
            bounds.0,
            bounds.1,
            bar_categories.as_ref().map(Vec::len),
            &[],
        );

        // Draw axes (sequential - UI elements) - only for Cartesian plots
        let draw_axes = self.needs_cartesian_axes();
        let draw_ticks = draw_axes && self.layout.tick_config.enabled;
        if draw_ticks {
            let x_axis_ticks = categorical_x_tick_pixels
                .as_deref()
                .unwrap_or(x_tick_pixels.as_slice());
            let x_axis_minor_ticks = if categorical_x_tick_pixels.is_some() {
                &[][..]
            } else {
                x_minor_tick_pixels.as_slice()
            };
            let (axis_width, major_tick_size, minor_tick_size, major_tick_width, minor_tick_width) =
                self.axis_tick_metrics_px();
            renderer.draw_axes_with_minor_ticks_styled(
                plot_area,
                x_axis_ticks,
                &y_tick_pixels,
                x_axis_minor_ticks,
                &y_minor_tick_pixels,
                &self.layout.tick_config.direction,
                &self.layout.tick_config.sides,
                &self.display.config.spines,
                self.display.theme.foreground,
                axis_width,
                major_tick_size,
                minor_tick_size,
                major_tick_width,
                minor_tick_width,
            )?;
        } else if draw_axes {
            let (axis_width, major_tick_size, minor_tick_size, major_tick_width, minor_tick_width) =
                self.axis_tick_metrics_px();
            renderer.draw_axes_with_minor_ticks_styled(
                plot_area,
                &[],
                &[],
                &[],
                &[],
                &self.layout.tick_config.direction,
                &TickSides::none(),
                &self.display.config.spines,
                self.display.theme.foreground,
                axis_width,
                major_tick_size,
                minor_tick_size,
                major_tick_width,
                minor_tick_width,
            )?;
        }

        // Process all series in parallel
        let render_scale = self.render_scale();
        let processed_series = self.render.parallel_renderer.process_series_parallel(
            &self.series_mgr.series,
            |series, index| -> Result<SeriesRenderData> {
                // Get series styling with defaults
                let color = series.color_with_alpha(self.display.theme.get_color(index));
                let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
                let alpha = series.alpha.unwrap_or(1.0);
                let resolved = &resolved_series[index];

                // Process each series type
                let render_series_type = match &series.series_type {
                    SeriesType::Line { .. } => {
                        let ResolvedSeries::Line { x, y } = resolved else {
                            unreachable!("resolved line must match its declarative series");
                        };
                        let (x_data, y_data) = (x.as_ref(), y.as_ref());
                        // Transform coordinates in parallel
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        // Break the line at every sample the axes cannot
                        // represent, using the same run splitter as the raster
                        // backend so the two cannot disagree about where a line
                        // stops and restarts.
                        let subpaths =
                            crate::core::plot::raster_batches::representable_sample_runs(
                                x_data,
                                y_data,
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )
                            .into_iter()
                            .map(|run| {
                                let mut points = points[run].to_vec();

                                if series.marker_style.is_none()
                                    && series.x_errors.is_none()
                                    && series.y_errors.is_none()
                                    && let Some(canonicalized) =
                                        canonicalize_line_points_exact(&points)
                                {
                                    points = canonicalized;
                                }

                                if should_reduce_line_series(
                                    series,
                                    points.len(),
                                    parallel_plot_area.width(),
                                ) && let Some(reduced) = reduce_line_points_for_raster(
                                    &points,
                                    parallel_plot_area.left,
                                    parallel_plot_area.width(),
                                ) {
                                    points = reduced;
                                }

                                points
                            })
                            .collect();

                        RenderSeriesType::Polyline {
                            subpaths,
                            style: series.line_style.clone().unwrap_or(LineStyle::Solid),
                            color,
                            width: line_width,
                        }
                    }
                    SeriesType::Scatter { .. } => {
                        let ResolvedSeries::Scatter { x, y } = resolved else {
                            unreachable!("resolved scatter must match its declarative series");
                        };
                        let (x_data, y_data) = (x.as_ref(), y.as_ref());
                        // Transform coordinates in parallel
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        // Process markers in parallel
                        let markers = self.render.parallel_renderer.process_markers_parallel(
                            &points,
                            series.marker_style.unwrap_or(MarkerStyle::Circle),
                            color,
                            self.parallel_marker_size_px(series, 10.0),
                            series
                                .marker_edge
                                .and_then(|edge| edge.resolve(&self.display.theme, color)),
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Bar {
                        categories, config, ..
                    } => {
                        let ResolvedSeries::Bar { values, .. } = resolved else {
                            unreachable!("resolved bars must match their declarative series");
                        };
                        let values = values.as_ref();

                        // Same resolution as the sequential path, so the two
                        // backends cannot drift apart on bar edges.
                        let edge = config.resolved_edge(&self.display.theme, color);

                        // Geometry comes from the one shared helper the raster
                        // and SVG backends use, so a bar chart cannot land in a
                        // different place depending on which backend drew it.
                        let bars = values
                            .iter()
                            .take(categories.len())
                            .enumerate()
                            .map(|(i, &value)| {
                                let (x, y, width, height) = super::series_internal::bar_pixel_rect(
                                    i,
                                    value,
                                    config.width,
                                    plot_area,
                                    bounds.0,
                                    bounds.1,
                                    bounds.2,
                                    bounds.3,
                                    &self.layout.y_scale,
                                );
                                crate::render::parallel::BarInstance {
                                    x,
                                    y,
                                    width,
                                    height,
                                    color,
                                    edge,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::ErrorBars { .. } | SeriesType::ErrorBarsXY { .. } => {
                        let (x_data, y_data) = match resolved {
                            ResolvedSeries::ErrorBars { x, y, .. }
                            | ResolvedSeries::ErrorBarsXY { x, y, .. } => (x.as_ref(), y.as_ref()),
                            _ => unreachable!(
                                "resolved error bars must match their declarative series"
                            ),
                        };
                        // For now, render error bars as scatter points
                        // Full error bar implementation would be added here
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        let markers = self.render.parallel_renderer.process_markers_parallel(
                            &points,
                            series.marker_style.unwrap_or(MarkerStyle::Circle),
                            color,
                            self.parallel_marker_size_px(series, 8.0),
                            series
                                .marker_edge
                                .and_then(|edge| edge.resolve(&self.display.theme, color)),
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Histogram { .. } => {
                        let hist_data = match resolved {
                            ResolvedSeries::Histogram { data } => data.clone(),
                            _ => {
                                unreachable!("resolved histogram must match its declarative series")
                            }
                        };

                        // Bins are adjacent, so they need an explicit edge to stay
                        // readable as separate bins (see `HistogramData::resolved_edge`).
                        let edge = hist_data.resolved_edge(&self.display.theme, color);

                        // Geometry comes from the one shared helper. This arm
                        // used to place bins centre-based from a *data*-space
                        // width used as a pixel width, so every bin was the
                        // wrong size and in the wrong place.
                        let bars = hist_data
                            .counts
                            .iter()
                            .enumerate()
                            .map(|(i, &count)| {
                                let (x, y, width, height) =
                                    super::series_internal::histogram_bar_pixel_rect(
                                        hist_data.bin_edges[i],
                                        hist_data.bin_edges[i + 1],
                                        count,
                                        plot_area,
                                        bounds.0,
                                        bounds.1,
                                        bounds.2,
                                        bounds.3,
                                        &self.layout.x_scale,
                                        &self.layout.y_scale,
                                    );
                                crate::render::parallel::BarInstance {
                                    x,
                                    y,
                                    width,
                                    height,
                                    color,
                                    edge,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::BoxPlot { config, .. } => {
                        let ResolvedSeries::BoxPlot { data, .. } = resolved else {
                            unreachable!("resolved box plot must match its declarative series");
                        };
                        let data = data.as_ref();
                        // Calculate box plot statistics
                        let box_data = crate::plots::boxplot::calculate_box_plot(&data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Box plot calculation failed: {}",
                                    e
                                ))
                            })?;

                        // One shared projection with the raster and SVG
                        // backends. This arm used to re-derive all five
                        // quantiles itself; deriving them once is what keeps
                        // the three backends from drifting apart again.
                        let px = super::series_internal::BoxPlotPixels::new(
                            &box_data,
                            plot_area,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            &self.layout.y_scale,
                        );

                        // Transform outliers
                        let visible_outliers: &[f64] = if box_data.show_outliers {
                            &box_data.outliers
                        } else {
                            &[]
                        };
                        let outliers = visible_outliers
                            .iter()
                            .map(|&outlier| crate::core::types::Point2f {
                                x: px.x_center,
                                y: super::series_internal::box_plot_value_y(
                                    outlier,
                                    plot_area,
                                    bounds.2,
                                    bounds.3,
                                    &self.layout.y_scale,
                                ),
                            })
                            .collect();

                        let edge_color = box_data.edge_color.unwrap_or(color);
                        let box_render_data = crate::render::parallel::BoxPlotRenderData {
                            x_center: px.x_center,
                            box_left: px.box_left,
                            box_right: px.box_right,
                            q1_y: px.q1_y,
                            median_y: px.median_y,
                            q3_y: px.q3_y,
                            lower_whisker_y: px.lower_whisker_y,
                            upper_whisker_y: px.upper_whisker_y,
                            outliers,
                            box_color: color.with_alpha(box_data.fill_alpha),
                            line_color: edge_color,
                            outlier_color: color,
                            cap_width: box_data.cap_width,
                            edge_width: box_data.edge_width,
                            whisker_width: box_data
                                .whisker_width
                                .map(|w| self.render_scale().points_to_pixels(w))
                                .unwrap_or(line_width),
                            median_width: box_data
                                .median_width
                                .map(|w| self.render_scale().points_to_pixels(w))
                                .unwrap_or(line_width * 1.5),
                            flier_size: self.render_scale().points_to_pixels(box_data.flier_size),
                        };

                        RenderSeriesType::BoxPlot {
                            box_data: box_render_data,
                        }
                    }
                    SeriesType::Heatmap { data } => {
                        let heatmap_plot_area = crate::plots::traits::PlotArea::new(
                            parallel_plot_area.left,
                            parallel_plot_area.top,
                            parallel_plot_area.width(),
                            parallel_plot_area.height(),
                            data_bounds.x_min,
                            data_bounds.x_max,
                            data_bounds.y_min,
                            data_bounds.y_max,
                        )
                        .with_scales(self.layout.x_scale, self.layout.y_scale);
                        let plot_left = parallel_plot_area.left;
                        let plot_top = parallel_plot_area.top;
                        let plot_right = parallel_plot_area.left + parallel_plot_area.width();
                        let plot_bottom = parallel_plot_area.top + parallel_plot_area.height();

                        // Create heatmap cells with colors
                        let cells: Vec<crate::render::parallel::HeatmapCell> = data
                            .values
                            .iter()
                            .enumerate()
                            .flat_map(|(row_idx, row)| {
                                row.iter().enumerate().filter_map(move |(col_idx, &value)| {
                                    if data.should_mask_value(value) {
                                        return None;
                                    }

                                    let (x, y, width, height) =
                                        data.cell_screen_rect(&heatmap_plot_area, row_idx, col_idx);
                                    let left = x.max(plot_left);
                                    let top = y.max(plot_top);
                                    let right = (x + width).min(plot_right);
                                    let bottom = (y + height).min(plot_bottom);
                                    if right <= left || bottom <= top {
                                        return None;
                                    }

                                    let cell_color =
                                        data.get_color(value).with_alpha(data.config.alpha * alpha);
                                    Some(crate::render::parallel::HeatmapCell {
                                        x: left,
                                        y: top,
                                        width: right - left,
                                        height: bottom - top,
                                        color: cell_color,
                                        border_color: data
                                            .config
                                            .cell_borders
                                            .then_some(cell_color.darken(0.2)),
                                    })
                                })
                            })
                            .collect();

                        RenderSeriesType::Heatmap {
                            cells,
                            n_rows: data.n_rows,
                            n_cols: data.n_cols,
                        }
                    }
                    SeriesType::Kde { data: kde_data } => {
                        // Transform KDE coordinates in parallel
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &kde_data.x,
                                &kde_data.y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        // Process line segments in parallel
                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            series.line_style.clone().unwrap_or(LineStyle::Solid),
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Ecdf { data: ecdf_data } => {
                        // Transform ECDF step vertices in parallel
                        let step_x: Vec<f64> =
                            ecdf_data.step_vertices.iter().map(|(x, _)| *x).collect();
                        let step_y: Vec<f64> =
                            ecdf_data.step_vertices.iter().map(|(_, y)| *y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &step_x,
                                &step_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        // Process line segments in parallel
                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            series.line_style.clone().unwrap_or(LineStyle::Solid),
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Violin { data: violin_data } => {
                        // Violin plots use polygon rendering, not supported in parallel mode
                        // Fall back to simple representation using KDE outline
                        let half_width = violin_data.config.width / 2.0;
                        let (left, right) = crate::plots::distribution::violin_polygon(
                            violin_data,
                            0.5,
                            half_width,
                            &violin_data.config,
                        );
                        let polygon =
                            crate::plots::distribution::close_violin_polygon(&left, &right);

                        let poly_x: Vec<f64> = polygon.iter().map(|(x, _)| *x).collect();
                        let poly_y: Vec<f64> = polygon.iter().map(|(_, y)| *y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Boxen { data: boxen_data } => {
                        // Boxen plots use polygon rendering, not supported in parallel mode
                        // Fall back to simple representation using box outlines
                        let mut all_points = Vec::new();
                        for boxen_box in &boxen_data.boxes {
                            let rect = crate::plots::distribution::boxen_rect(
                                boxen_box,
                                0.5,
                                boxen_data.config.orient,
                            );
                            for (x, y) in &rect {
                                all_points.push((*x, *y));
                            }
                        }

                        let poly_x: Vec<f64> = all_points.iter().map(|(x, _)| *x).collect();
                        let poly_y: Vec<f64> = all_points.iter().map(|(_, y)| *y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Quiver { .. } => RenderSeriesType::Line { segments: vec![] },
                    SeriesType::Contour { data: contour_data } => {
                        // Contour levels are disjoint segment soups, not one polyline:
                        // flatten the endpoints for the parallel transform, then pair
                        // them back up so no connector is drawn between segments.
                        let mut poly_x = Vec::new();
                        let mut poly_y = Vec::new();
                        // Level index for each emitted segment, so each keeps its own colour.
                        let mut segment_levels = Vec::new();
                        for (level_index, level) in contour_data.lines.iter().enumerate() {
                            for &(x1, y1, x2, y2) in &level.segments {
                                poly_x.push(x1);
                                poly_y.push(y1);
                                poly_x.push(x2);
                                poly_y.push(y2);
                                segment_levels.push(level_index);
                            }
                        }

                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        // Same colour decision as the raster and SVG backends.
                        let cmap = crate::render::ColorMap::by_name(&contour_data.config.cmap)
                            .unwrap_or_else(crate::render::ColorMap::viridis);
                        let n_levels = contour_data.levels.len();
                        let effective_alpha =
                            contour_data.config.alpha * series.alpha.unwrap_or(1.0).clamp(0.0, 1.0);
                        let contour_width = self.dpi_scaled_line_width(
                            series.line_width.unwrap_or(contour_data.config.line_width),
                        );

                        let segments = points
                            .chunks_exact(2)
                            .zip(segment_levels)
                            .map(|(pair, level_index)| {
                                let line_color =
                                    crate::plots::continuous::contour::contour_line_color(
                                        &contour_data.config,
                                        &self.display.theme,
                                        &cmap,
                                        color,
                                        level_index,
                                        n_levels,
                                    );
                                crate::data::elements::LineSegment {
                                    start: pair[0],
                                    end: pair[1],
                                    style: LineStyle::Solid,
                                    color: line_color.with_alpha(
                                        (f32::from(line_color.a) / 255.0) * effective_alpha,
                                    ),
                                    width: contour_width,
                                }
                            })
                            .collect();

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Pie { .. } => {
                        // Pie charts use polygon rendering, not supported in parallel mode
                        // Return empty segments (will be rendered using normal path)
                        RenderSeriesType::Line { segments: vec![] }
                    }
                    SeriesType::Radar { data: radar_data } => {
                        // Preserve each internal polygon's frame-resolved color instead of
                        // joining all radar payloads into one top-level palette color.
                        let mut segments = Vec::new();
                        for (internal_index, series_data) in radar_data.series.iter().enumerate() {
                            let mut polygon = series_data.polygon.clone();
                            if let Some(&first) = polygon.first() {
                                polygon.push(first);
                            }
                            let poly_x: Vec<f64> = polygon.iter().map(|(x, _)| *x).collect();
                            let poly_y: Vec<f64> = polygon.iter().map(|(_, y)| *y).collect();
                            let points = self
                                .render
                                .parallel_renderer
                                .transform_coordinates_parallel_scaled(
                                    &poly_x,
                                    &poly_y,
                                    data_bounds.clone(),
                                    parallel_plot_area.clone(),
                                    &self.layout.x_scale,
                                    &self.layout.y_scale,
                                )?;
                            let radar_color = series
                                .resolved_radar_colors
                                .as_ref()
                                .and_then(|colors| colors.get(internal_index).copied())
                                .unwrap_or(series.color.unwrap_or(color));
                            let radar_color =
                                radar_color.with_alpha((f32::from(radar_color.a) / 255.0) * alpha);
                            segments.extend(
                                self.render.parallel_renderer.process_polyline_parallel(
                                    &points,
                                    LineStyle::Solid,
                                    radar_color,
                                    line_width,
                                )?,
                            );
                        }

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Polar { data: polar_data } => {
                        // Polar plots use polygon rendering, not supported in parallel mode
                        // Fall back to line rendering of polar points
                        let poly_x: Vec<f64> = polar_data.points.iter().map(|p| p.x).collect();
                        let poly_y: Vec<f64> = polar_data.points.iter().map(|p| p.y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel_scaled(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                };

                Ok(SeriesRenderData {
                    series_type: render_series_type,
                    color,
                    line_width,
                    alpha,
                    label: series.label.clone(),
                })
            },
        )?;

        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

        renderer.draw_annotations_where_scaled(
            &self.annotations,
            plot_area,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            dpi,
            &self.layout.x_scale,
            &self.layout.y_scale,
            Self::is_underlay_annotation,
        )?;

        // Render processed series (sequential - final drawing)
        for processed in processed_series {
            match processed.series_type {
                RenderSeriesType::Polyline {
                    subpaths,
                    style,
                    color,
                    width,
                } => {
                    // One draw call per sub-path: the gaps between them are
                    // samples the axes cannot place, and must stay gaps.
                    for subpath in subpaths {
                        let points: Vec<(f32, f32)> = subpath
                            .into_iter()
                            .map(|point| (point.x, point.y))
                            .collect();
                        renderer.draw_polyline_clipped(
                            &points,
                            color,
                            width,
                            style.clone(),
                            clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::Line { segments } => {
                    // Draw all line segments
                    for segment in segments {
                        renderer.draw_polyline_clipped(
                            &[
                                (segment.start.x, segment.start.y),
                                (segment.end.x, segment.end.y),
                            ],
                            segment.color,
                            segment.width,
                            segment.style,
                            clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::Scatter { markers } => {
                    if let Some(first) = markers.first() {
                        if markers.iter().all(|marker| {
                            marker.style == first.style
                                && marker.color == first.color
                                && marker.size.to_bits() == first.size.to_bits()
                                && marker.edge == first.edge
                        }) {
                            let points: Vec<Point2f> =
                                markers.iter().map(|marker| marker.position).collect();
                            renderer.draw_markers_styled_clipped(
                                &points,
                                first.size,
                                first.style,
                                first.color,
                                first.edge,
                                clip_rect,
                            )?;
                        } else {
                            for marker in markers {
                                renderer.draw_marker_styled_clipped(
                                    marker.position.x,
                                    marker.position.y,
                                    marker.size,
                                    marker.style,
                                    marker.color,
                                    marker.edge,
                                    clip_rect,
                                )?;
                            }
                        }
                    }
                }
                RenderSeriesType::Bar { bars } => {
                    // Draw all bars
                    for bar in bars {
                        renderer.draw_rectangle_styled_clipped(
                            bar.x,
                            bar.y,
                            bar.width,
                            bar.height,
                            Some(bar.color),
                            bar.edge,
                            clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::BoxPlot { box_data } => {
                    // Draw box plot components. Widths and sizes come from the
                    // resolved `BoxPlotData`, so the parallel backend matches
                    // the raster and SVG ones.

                    // Draw the box (IQR)
                    renderer.draw_rectangle_styled_clipped(
                        box_data.box_left,
                        box_data.q3_y,
                        box_data.box_right - box_data.box_left,
                        box_data.q1_y - box_data.q3_y,
                        Some(box_data.box_color),
                        Some((box_data.line_color, box_data.edge_width)),
                        clip_rect,
                    )?;

                    // Draw median line
                    renderer.draw_line_clipped(
                        box_data.box_left,
                        box_data.median_y,
                        box_data.box_right,
                        box_data.median_y,
                        box_data.line_color,
                        box_data.median_width,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw lower whisker
                    renderer.draw_line_clipped(
                        box_data.x_center,
                        box_data.q1_y,
                        box_data.x_center,
                        box_data.lower_whisker_y,
                        box_data.line_color,
                        box_data.whisker_width,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw upper whisker
                    renderer.draw_line_clipped(
                        box_data.x_center,
                        box_data.q3_y,
                        box_data.x_center,
                        box_data.upper_whisker_y,
                        box_data.line_color,
                        box_data.whisker_width,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw whisker caps
                    let cap_width =
                        (box_data.box_right - box_data.box_left) * 0.5 * box_data.cap_width;
                    renderer.draw_line_clipped(
                        box_data.x_center - cap_width,
                        box_data.lower_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.lower_whisker_y,
                        box_data.line_color,
                        box_data.whisker_width,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    renderer.draw_line_clipped(
                        box_data.x_center - cap_width,
                        box_data.upper_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.upper_whisker_y,
                        box_data.line_color,
                        box_data.whisker_width,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw outliers
                    for outlier in &box_data.outliers {
                        renderer.draw_marker_clipped(
                            outlier.x,
                            outlier.y,
                            box_data.flier_size,
                            MarkerStyle::Circle,
                            box_data.outlier_color,
                            clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::ErrorBars { .. } => {
                    // Error bars implementation would go here
                }
                RenderSeriesType::Heatmap { cells, .. } => {
                    // Draw all heatmap cells as pixel-aligned rectangles so custom
                    // extents and clipped views stay seam-free.
                    for cell in cells {
                        renderer.draw_pixel_aligned_solid_rectangle(
                            cell.x,
                            cell.y,
                            cell.width,
                            cell.height,
                            cell.color,
                        )?;
                        if let Some(border_color) = cell.border_color {
                            renderer.draw_pixel_aligned_rectangle_outline(
                                cell.x,
                                cell.y,
                                cell.width,
                                cell.height,
                                border_color,
                            )?;
                        }
                    }
                }
            }
        }

        renderer.draw_annotations_where_scaled(
            &self.annotations,
            plot_area,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            dpi,
            &self.layout.x_scale,
            &self.layout.y_scale,
            Self::is_overlay_annotation,
        )?;

        // Draw tick labels (only for Cartesian plots)
        if draw_axes {
            let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
                    bounds.0,
                    bounds.1,
                    bounds.2,
                    bounds.3,
                    &y_ticks,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.display.theme.foreground,
                    dpi,
                    self.layout.tick_config.enabled,
                    false,
                    &self.layout.y_scale,
                )?;
            } else {
                renderer.draw_axis_labels_at_scaled(
                    &layout.plot_area,
                    bounds.0,
                    bounds.1,
                    bounds.2,
                    bounds.3,
                    &x_ticks,
                    &y_ticks,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.display.theme.foreground,
                    dpi,
                    self.layout.tick_config.enabled,
                    false,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                )?;
            }
        }

        // Draw title if present
        if let Some(ref pos) = layout.title_pos
            && let Some(title) = frame.title.as_deref()
        {
            renderer.draw_title_at_with_weight(
                pos,
                title,
                self.display.theme.foreground,
                self.display.config.typography.title_weight,
            )?;
        }

        // Draw xlabel if present (only for Cartesian plots)
        if self.needs_cartesian_axes() {
            if let Some(ref pos) = layout.xlabel_pos
                && let Some(xlabel) = frame.xlabel.as_deref()
            {
                renderer.draw_xlabel_at(pos, xlabel, self.display.theme.foreground)?;
            }

            // Draw ylabel if present
            if let Some(ref pos) = layout.ylabel_pos
                && let Some(ylabel) = frame.ylabel.as_deref()
            {
                renderer.draw_ylabel_at(pos, ylabel, self.display.theme.foreground)?;
            }
        }

        let legend_items = self.collect_legend_items();
        if !legend_items.is_empty() && frame.style.legend.enabled {
            renderer.draw_legend_full_resolved(
                &legend_items,
                &frame.style.legend,
                plot_area,
                None,
                layout.legend_rect.as_ref().map(|rect| rect.bounds()),
            )?;
        }

        // Record performance statistics
        let duration = start_time.elapsed();
        let total_points = self.calculate_total_points();

        // Performance stats available via self.render.parallel_renderer.performance_stats()
        // Uncomment for debugging:
        // let stats = self.render.parallel_renderer.performance_stats();
        // println!("⚡ Parallel: {} series, {} points in {:.1}ms ({:.1}x speedup, {} threads)",
        //     self.series_mgr.series.len(), total_points, duration.as_millis(),
        //     stats.estimated_speedup, stats.configured_threads);
        let _ = (start_time, total_points); // suppress unused warnings

        // Convert renderer output to Image
        Ok(renderer.into_image())
    }

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
        assert!(bars.y_zero_baseline);
        assert!(!bars.all_edges);

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
