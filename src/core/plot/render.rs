use super::*;
use crate::render::skia::{XTickLabelPlan, XTickRowBounds, XTickRowMetrics, draw_x_tick_label_row};

/// Where the data actually is, for [`LegendPosition::Best`].
///
/// One function, called by every backend that draws a legend, so raster and SVG
/// cannot answer `Best` with different corners. Samples are projected through
/// the same mapper the drawing code uses and a sample the axis scales cannot
/// place is dropped, exactly as the drawing code drops it — a log axis must not
/// push a legend away from a point it never drew.
///
/// Only the series that carry explicit `(x, y)` samples are binned. Bars,
/// histograms and box plots contribute nothing yet, which leaves the grid empty
/// for a bar-only figure and makes `Best` degrade to `UpperRight` — the
/// behaviour those plots already had.
fn legend_occupancy(
    series: &[ResolvedSeries<'_>],
    plot_area: tiny_skia::Rect,
    (x_min, x_max, y_min, y_max): (f64, f64, f64, f64),
    x_scale: &AxisScale,
    y_scale: &AxisScale,
) -> LegendOccupancy {
    let points = series
        .iter()
        .filter_map(|series| match series {
            ResolvedSeries::Line { x, y }
            | ResolvedSeries::Scatter { x, y }
            | ResolvedSeries::ErrorBars { x, y, .. }
            | ResolvedSeries::ErrorBarsXY { x, y, .. } => Some((x, y)),
            _ => None,
        })
        .flat_map(|(x, y)| {
            x.iter().zip(y.iter()).filter_map(|(&x, &y)| {
                try_map_data_to_pixels_scaled(
                    x, y, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
                )
            })
        });

    LegendOccupancy::from_screen_points(
        (
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
        ),
        points,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationRenderLayer {
    Underlay,
    Overlay,
}

/// One grid drawing pass produced by [`Plot::grid_layers`].
///
/// Carries the tick pixel positions to stroke plus the stroke appearance for
/// that pass, so major and minor grids keep their distinct weight and opacity.
#[derive(Debug, Clone)]
pub(crate) struct GridLayer {
    pub x_pixels: Vec<f32>,
    pub y_pixels: Vec<f32>,
    pub color: Color,
    pub width_px: f32,
}

impl Plot {
    pub(crate) fn axis_tick_metrics_px(&self) -> (f32, f32, f32, f32, f32) {
        let lines = &self.display.config.lines;
        let axis_width = self.line_width_px(lines.axis_width);
        let major_tick_size = self.line_width_px(lines.tick_length);
        let minor_tick_size = self.line_width_px((lines.tick_length * 0.6).max(0.1));
        let major_tick_width = self.line_width_px(lines.tick_width);
        let minor_tick_width = self.line_width_px((lines.tick_width * 0.75).max(0.1));
        (
            axis_width,
            major_tick_size,
            minor_tick_size,
            major_tick_width,
            minor_tick_width,
        )
    }

    /// The spines the theme allows.
    ///
    /// A theme with `frame == false` (seaborn) removes the box outright; every
    /// other theme leaves the plot's own [`SpineConfig`] untouched, which is
    /// what keeps existing output byte-identical.
    ///
    /// [`SpineConfig`]: crate::core::config::SpineConfig
    pub(crate) fn themed_spines(&self) -> crate::core::config::SpineConfig {
        if self.display.theme.frame {
            self.display.config.spines
        } else {
            crate::core::config::SpineConfig::none()
        }
    }

    /// The tick sides the theme allows.
    ///
    /// `Theme::tick_marks == false` drops the marks only. Tick *labels* are
    /// drawn by a separate pass keyed on `TickConfig::enabled`, so they stay.
    pub(crate) fn themed_tick_sides(&self, sides: TickSides) -> TickSides {
        if self.display.theme.tick_marks {
            sides
        } else {
            TickSides::none()
        }
    }

    /// The panel fill the theme asks for, if any.
    ///
    /// Painted inside the axes rect before the grid, so the grid reads as drawn
    /// *on* the panel.
    pub(crate) fn themed_panel_background(&self) -> Option<Color> {
        self.display.theme.panel_background
    }

    fn annotation_render_layer(annotation: &Annotation) -> AnnotationRenderLayer {
        match annotation {
            Annotation::FillBetween { .. }
            | Annotation::HSpan { .. }
            | Annotation::VSpan { .. }
            | Annotation::Rectangle { .. } => AnnotationRenderLayer::Underlay,
            // Arrows the library emits as series structure — `stem()` pushes its
            // stems this way — paint below the series so they do not cover their
            // own markers. Provenance is carried on the style, never inferred
            // from the head style: a caller-built headless arrow is a normal
            // pointer annotation and belongs in the overlay.
            Annotation::Arrow { style, .. } if style.is_series_structure() => {
                AnnotationRenderLayer::Underlay
            }
            Annotation::Text { .. }
            | Annotation::Arrow { .. }
            | Annotation::HLine { .. }
            | Annotation::VLine { .. } => AnnotationRenderLayer::Overlay,
        }
    }

    pub(super) fn is_underlay_annotation(annotation: &Annotation) -> bool {
        Self::annotation_render_layer(annotation) == AnnotationRenderLayer::Underlay
    }

    pub(super) fn is_overlay_annotation(annotation: &Annotation) -> bool {
        Self::annotation_render_layer(annotation) == AnnotationRenderLayer::Overlay
    }

    /// The one gate every output path passes through before a frame is
    /// resolved.
    ///
    /// `save`, `render`/`render_at`, `render_to_svg`/`export_svg`,
    /// `save_pdf_with_size` and `render_to_renderer` all call this first, so it
    /// sits *above* the raster/vector split. Validation that must hold for the
    /// figure regardless of how it is rasterised belongs here and not inside a
    /// renderer: a check duplicated per backend is a check the backends will
    /// eventually disagree about, which is exactly how `.export_svg()` came to
    /// accept a figure `.save()` refused.
    pub(super) fn validate_before_frame_resolution(&self) -> Result<()> {
        self.validate_runtime_environment()?;
        if let Some(error) = self.pending_ingestion_error() {
            return Err(error);
        }
        self.validate_aggregate_geometry_against_axis_scales(&self.series_mgr.series)?;
        self.validate_annotation_shapes_against_axis_scales()?;
        Ok(())
    }

    /// Refuse annotation *shapes* whose defining coordinates an axis cannot place.
    ///
    /// The same split as [`Self::validate_aggregate_geometry_against_axis_scales`],
    /// applied to annotations rather than to series.
    ///
    /// * A text label, an arrow or a reference line is a **mark at a
    ///   position**. One the axis cannot place is skipped — by both backends,
    ///   identically — exactly as an unplaceable scatter point is. Not checked.
    /// * A rectangle, a span or a filled region is a **shape**, and the
    ///   coordinates *are* what define it. A corner the axis cannot place does
    ///   not make the shape smaller, it makes it undrawable.
    ///
    /// Left to the backends, the second case is where they part company: the
    /// SVG renderer refuses the element and latches the fault, while tiny-skia
    /// answers the same input with `Rect::from_xywh`/`PathBuilder::finish`
    /// returning `None` and simply draws nothing. One would report and one
    /// would go quiet — the divergence this tranche exists to remove. Refusing
    /// here, above the split, means neither backend ever sees the `NaN` and the
    /// user gets the message that names the axis and the fix.
    fn validate_annotation_shapes_against_axis_scales(&self) -> Result<()> {
        for annotation in &self.annotations {
            match annotation {
                Annotation::Rectangle {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    Self::reject_unplaceable_values(
                        [*x, *x + *width],
                        &self.layout.x_scale,
                        "x",
                        "rectangle annotation",
                    )?;
                    Self::reject_unplaceable_values(
                        [*y, *y + *height],
                        &self.layout.y_scale,
                        "y",
                        "rectangle annotation",
                    )?;
                }
                Annotation::HSpan { x_min, x_max, .. } => {
                    Self::reject_unplaceable_values(
                        [*x_min, *x_max],
                        &self.layout.x_scale,
                        "x",
                        "horizontal span annotation",
                    )?;
                }
                Annotation::VSpan { y_min, y_max, .. } => {
                    Self::reject_unplaceable_values(
                        [*y_min, *y_max],
                        &self.layout.y_scale,
                        "y",
                        "vertical span annotation",
                    )?;
                }
                Annotation::FillBetween { x, y1, y2, .. } => {
                    // Both curves and the shared abscissa are outline vertices
                    // of one closed polygon, so any of them can break it.
                    Self::reject_unplaceable_values(
                        x.iter().copied(),
                        &self.layout.x_scale,
                        "x",
                        "filled region annotation",
                    )?;
                    Self::reject_unplaceable_values(
                        y1.iter().chain(y2.iter()).copied(),
                        &self.layout.y_scale,
                        "y",
                        "filled region annotation",
                    )?;
                }
                Annotation::Text { .. }
                | Annotation::Arrow { .. }
                | Annotation::HLine { .. }
                | Annotation::VLine { .. } => {}
            }
        }
        Ok(())
    }

    /// Refuse aggregate geometry whose defining values an axis cannot place.
    ///
    /// Two kinds of series meet an unrepresentable sample very differently:
    ///
    /// * A **point or line** series is a set of independent samples. One that a
    ///   log axis cannot place is simply not drawn and the polyline breaks at
    ///   the gap, so the user still gets the rest of their data. Those series
    ///   are deliberately *not* checked here.
    /// * **Aggregate geometry** — a bar, a histogram's bins, a box plot's
    ///   quartiles, a violin's density, a boxen's letter values — is *computed
    ///   from* its values. A single non-positive value does not make the shape
    ///   shorter, it makes it undefined: the box plot that motivated this
    ///   rendered two orphan strokes and a flier, with no box at all, and
    ///   reported success. Those are refused here.
    ///
    /// The scan runs on the **raw data** against the plot's scales, not on the
    /// computed axis bounds, and that is the whole point. The bounds
    /// accumulator admits only coordinates the scale can represent, so a range
    /// derived from it can never look invalid — which is why
    /// [`Self::validate_axis_scale_ranges_for_render`] stopped catching this
    /// case, the raster backend fell over later on a `NaN` rectangle, and the
    /// SVG backend happily wrote `height="NaN"`.
    pub(crate) fn validate_aggregate_geometry_against_axis_scales(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<()> {
        for series in series_list {
            match &series.series_type {
                SeriesType::Bar { values, config, .. } => {
                    // The bar's baseline (`config.bottom`, zero by default) is
                    // not data and is deliberately not checked: a log value
                    // axis draws bars down to the axis floor, and refusing that
                    // would make every log bar chart impossible.
                    let (axis, scale) = match config.orientation {
                        crate::plots::basic::BarOrientation::Vertical => {
                            ("y", &self.layout.y_scale)
                        }
                        crate::plots::basic::BarOrientation::Horizontal => {
                            ("x", &self.layout.x_scale)
                        }
                    };
                    Self::reject_unplaceable_values(
                        values.resolve_cow(0.0).iter().copied(),
                        scale,
                        axis,
                        "bar",
                    )?;
                }
                SeriesType::Histogram { data, config, .. } => {
                    // Bins run along x. The counts are a derived height rising
                    // from the baseline, exactly like a bar's, so the value
                    // axis is not checked.
                    let scale = &self.layout.x_scale;
                    match config.range {
                        // An explicit range *is* the outer pair of bin edges;
                        // samples outside it are never binned, so only the
                        // range itself has to be placeable.
                        Some((low, high)) => Self::reject_unplaceable_values(
                            [low, high].into_iter(),
                            scale,
                            "x",
                            "histogram",
                        )?,
                        None => Self::reject_unplaceable_values(
                            data.resolve_cow(0.0).iter().copied(),
                            scale,
                            "x",
                            "histogram",
                        )?,
                    }
                }
                SeriesType::BoxPlot { data, config } => {
                    let (axis, scale) = match config.orientation {
                        crate::plots::boxplot::BoxOrientation::Vertical => {
                            ("y", &self.layout.y_scale)
                        }
                        crate::plots::boxplot::BoxOrientation::Horizontal => {
                            ("x", &self.layout.x_scale)
                        }
                    };
                    Self::reject_unplaceable_values(
                        data.resolve_cow(0.0).iter().copied(),
                        scale,
                        axis,
                        "box plot",
                    )?;
                }
                SeriesType::Violin { data } => {
                    // The violin body is a kernel density estimate over the
                    // whole sample, so every value shapes the outline; one the
                    // axis cannot place does not shorten the body, it punches a
                    // hole in it.
                    let (axis, scale) = match data.config.orientation {
                        crate::plots::distribution::Orientation::Vertical => {
                            ("y", &self.layout.y_scale)
                        }
                        crate::plots::distribution::Orientation::Horizontal => {
                            ("x", &self.layout.x_scale)
                        }
                    };
                    Self::reject_unplaceable_values(
                        data.data.iter().copied(),
                        scale,
                        axis,
                        "violin",
                    )?;
                }
                SeriesType::Boxen { data } => {
                    // Only the letter-value bands and the median are checked.
                    // Outliers are drawn one marker at a time, exactly like a
                    // scatter point, so an unplaceable one is skippable and must
                    // not cost the user the whole figure.
                    let (axis, scale) = match data.config.orient {
                        crate::plots::distribution::BoxenOrientation::Vertical => {
                            ("y", &self.layout.y_scale)
                        }
                        crate::plots::distribution::BoxenOrientation::Horizontal => {
                            ("x", &self.layout.x_scale)
                        }
                    };
                    Self::reject_unplaceable_values(
                        data.boxes
                            .iter()
                            .flat_map(|band| [band.lower, band.upper])
                            .chain(std::iter::once(data.median)),
                        scale,
                        axis,
                        "boxen",
                    )?;
                }
                // Everything else is drawn sample by sample — a line, a scatter,
                // an error bar, a heatmap cell, a contour vertex. An
                // unrepresentable sample there is a gap in the drawing, not an
                // undefined shape, so it is dropped by the projection and the
                // geometry breaks at the hole instead of being refused.
                _ => {}
            }
        }
        Ok(())
    }

    /// Build the refusal for one unplaceable aggregate value.
    ///
    /// The message opens with the shared `LOG_SCALE_REQUIRES_POSITIVE` wording
    /// so it names the axis and points at `SymLog`, then says which value and
    /// which plot type provoked it — the two things the previous "Invalid
    /// rectangle dimensions" left the user to guess.
    fn reject_unplaceable_values(
        values: impl IntoIterator<Item = f64>,
        scale: &AxisScale,
        axis: &'static str,
        plot_kind: &'static str,
    ) -> Result<()> {
        let Some(offender) = scale.first_unplaceable(values) else {
            return Ok(());
        };
        let shared = crate::axes::scale::LOG_SCALE_REQUIRES_POSITIVE;
        Err(PlottingError::InvalidInput(format!(
            "Invalid {axis}-axis range: {shared} \
             (the {plot_kind} value {offender} has no position on the logarithmic {axis} axis, \
             and the shape is defined by it rather than drawn point by point, \
             so it cannot simply be skipped. Use `.{axis}scale(AxisScale::SymLog {{ linthresh }})` \
             or remove the non-positive values.)"
        )))
    }

    fn render_image_with_mode(&self, mode: RenderExecutionMode) -> Result<Image> {
        self.render_image_with_mode_at(mode, 0.0)
    }

    fn render_image_with_mode_at(&self, mode: RenderExecutionMode, time: f64) -> Result<Image> {
        self.render_image_with_mode_and_diagnostics_at(mode, time)
            .map(|(image, _)| image)
    }

    fn render_dynamic_style_frame(
        &self,
        mode: RenderExecutionMode,
        time: f64,
    ) -> Result<(Image, RenderDiagnostics)> {
        let frame = self.resolve_frame(time)?;
        let style_shell = self.resolved_style_shell(&frame.style);
        let result = style_shell.render_image_with_resolved_frame(mode, &frame);
        if result.is_ok() {
            frame.acknowledge_rendered(self);
        }
        result
    }

    pub(crate) fn validate_axis_scale_ranges_for_render(
        &self,
        series_list: &[PlotSeries],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        if !Self::needs_cartesian_axes_for_series(series_list) {
            return Ok(());
        }

        // Belt and braces. `validate_before_frame_resolution` already ran this
        // for every public entry point, and it is idempotent; repeating it in
        // the one function both backends call means a future path that reaches
        // a renderer without passing the entry gate still gets an error rather
        // than a `NaN` rectangle.
        self.validate_aggregate_geometry_against_axis_scales(series_list)?;
        self.validate_annotation_shapes_against_axis_scales()?;

        self.layout
            .x_scale
            .validate_range(x_min, x_max)
            .map_err(|message| {
                PlottingError::InvalidInput(format!("Invalid x-axis range: {message}"))
            })?;
        self.layout
            .y_scale
            .validate_range(y_min, y_max)
            .map_err(|message| {
                PlottingError::InvalidInput(format!("Invalid y-axis range: {message}"))
            })?;
        Ok(())
    }

    pub(crate) fn scaled_x_pixel(
        value: f64,
        min: f64,
        max: f64,
        plot_area: tiny_skia::Rect,
        scale: &AxisScale,
    ) -> f32 {
        if min == max || (!matches!(scale, AxisScale::Log) && (max - min).abs() < f64::EPSILON) {
            plot_area.left() + plot_area.width() * 0.5
        } else {
            let normalized = scale.normalized_position(value, min, max);
            plot_area.left() + normalized as f32 * plot_area.width()
        }
    }

    pub(crate) fn scaled_y_pixel(
        value: f64,
        min: f64,
        max: f64,
        plot_area: tiny_skia::Rect,
        scale: &AxisScale,
    ) -> f32 {
        if min == max || (!matches!(scale, AxisScale::Log) && (max - min).abs() < f64::EPSILON) {
            plot_area.top() + plot_area.height() * 0.5
        } else {
            let normalized = scale.normalized_position(value, min, max);
            plot_area.bottom() - normalized as f32 * plot_area.height()
        }
    }

    pub(crate) fn minor_tick_values_for_scale(
        major_ticks: &[f64],
        min: f64,
        max: f64,
        scale: &AxisScale,
        requested_count: usize,
    ) -> Vec<f64> {
        let (range_min, range_max) = if min <= max { (min, max) } else { (max, min) };
        let mut ticks = match scale {
            AxisScale::Log => Self::log_minor_tick_values_for_range(range_min, range_max),
            AxisScale::Linear | AxisScale::SymLog { .. } => {
                crate::axes::generate_minor_ticks(major_ticks, requested_count)
            }
        };

        ticks.retain(|tick| {
            tick.is_finite()
                && *tick >= range_min
                && *tick <= range_max
                && !major_ticks
                    .iter()
                    .any(|major| Self::tick_values_overlap(*major, *tick, scale))
        });
        ticks.sort_by(f64::total_cmp);
        ticks.dedup_by(|left, right| Self::tick_values_overlap(*left, *right, scale));
        ticks
    }

    fn log_minor_tick_values_for_range(min: f64, max: f64) -> Vec<f64> {
        if min <= 0.0 || max <= 0.0 || min >= max {
            return Vec::new();
        }

        let min_exp = min.log10().floor() as i32;
        let max_exp = max.log10().ceil() as i32;
        let mut ticks = Vec::new();

        for exp in min_exp..=max_exp {
            let decade = 10.0_f64.powi(exp);
            for multiplier in 2..=9 {
                let tick = decade * multiplier as f64;
                if tick >= min && tick <= max {
                    ticks.push(tick);
                }
            }
        }

        ticks
    }

    fn tick_values_overlap(left: f64, right: f64, scale: &AxisScale) -> bool {
        match scale {
            AxisScale::Log => left == right,
            AxisScale::Linear | AxisScale::SymLog { .. } => {
                (left - right).abs() <= left.abs().max(right.abs()).max(1.0) * 1e-10
            }
        }
    }

    /// Convert a grid stroke width from points to device pixels.
    ///
    /// Floored at one device pixel: a sub-pixel grid stroke is antialiased into
    /// a washed-out band and effectively disappears.
    pub(crate) fn grid_stroke_px(points: f32, points_to_px: &impl Fn(f32) -> f32) -> f32 {
        points_to_px(points).max(crate::core::style_utils::defaults::MIN_GRID_LINE_WIDTH_PX)
    }

    /// Split the grid into the passes the renderer has to draw.
    ///
    /// Major and minor grid lines are *not* interchangeable: [`GridStyle`]
    /// carries a separate `minor_line_width` and `minor_alpha` so minor lines
    /// read as subordinate. Concatenating both tick sets into a single draw call
    /// throws that away and paints minor lines at full major weight, so each
    /// pass is emitted separately with its own colour and stroke width.
    ///
    /// For [`GridMode::Both`] the minor pass comes first so that major lines
    /// overdraw any coincident minor line.
    pub(crate) fn grid_layers(
        style: &GridStyle,
        mode: &GridMode,
        x_major: &[f32],
        y_major: &[f32],
        x_minor: &[f32],
        y_minor: &[f32],
        points_to_px: impl Fn(f32) -> f32,
    ) -> Vec<GridLayer> {
        let major = || GridLayer {
            x_pixels: x_major.to_vec(),
            y_pixels: y_major.to_vec(),
            color: style.effective_color(),
            width_px: Self::grid_stroke_px(style.line_width, &points_to_px),
        };
        let minor = || GridLayer {
            x_pixels: x_minor.to_vec(),
            y_pixels: y_minor.to_vec(),
            color: style.effective_minor_color(),
            width_px: Self::grid_stroke_px(style.minor_line_width, &points_to_px),
        };

        match mode {
            GridMode::MajorOnly => vec![major()],
            GridMode::MinorOnly => vec![minor()],
            GridMode::Both => vec![minor(), major()],
        }
    }

    pub(super) fn render_renderer_with_resolved_frame<F>(
        &self,
        mode: RenderExecutionMode,
        frame: &ResolvedFrame<'_>,
        draw_series: F,
    ) -> Result<(SkiaRenderer, RenderDiagnostics)>
    where
        F: FnOnce(
            &Plot,
            &[PlotSeries],
            &[ResolvedSeries<'_>],
            &mut SkiaRenderer,
            tiny_skia::Rect,
            f64,
            f64,
            f64,
            f64,
            RenderScale,
            RenderExecutionMode,
        ) -> Result<()>,
    {
        self.validate_runtime_environment()?;
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        if !frame.series.is_empty() {
            self.validate_resolved_series(&frame.series)?;
        }

        let total_points = Self::calculate_total_points_from_resolved(&frame.series);
        const LARGE_DATASET_THRESHOLD: usize = 1_000_000;
        if total_points > LARGE_DATASET_THRESHOLD {
            log::warn!(
                "Rendering {} points (>1M); consider explicit data reduction or a supported backend for this output path.",
                total_points
            );
        }

        let (scaled_width, scaled_height) = self.config_canvas_size();
        let mut renderer = SkiaRenderer::with_font_family(
            scaled_width,
            scaled_height,
            self.display.theme.clone(),
            self.display.config.typography.family.clone(),
        )?;
        renderer.set_text_engine_mode(self.display.text_engine);
        renderer.set_render_mode_diagnostics(match mode {
            RenderExecutionMode::Reference => "reference",
            RenderExecutionMode::Optimized => "optimized",
        });
        let render_scale = self.render_scale();
        let dpi = render_scale.dpi();
        renderer.set_render_scale(render_scale);

        let (x_min, x_max, y_min, y_max) =
            self.effective_main_panel_bounds_from_resolved(&self.series_mgr.series, &frame.series)?;
        self.validate_axis_scale_ranges_for_render(
            &self.series_mgr.series,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;

        // One harvest for every categorical plot type: bars, box plots,
        // violins and boxen plots all sit in the same unit-wide slots.
        let category_axis = super::series_internal::CategoryAxis::harvest(&self.series_mgr.series);
        let (category_labels, category_positions): (&[String], &[f64]) = match &category_axis {
            Some(axis) => (&axis.labels, &axis.positions),
            None => (&[], &[]),
        };
        let is_categorical = category_axis.is_some();

        let content = self.create_plot_content_from_resolved_text(y_min, y_max, frame);
        let (layout, x_ticks, y_ticks, x_tick_label_plan) = self
            .compute_layout_with_categorical_ticks(
                &renderer,
                (scaled_width, scaled_height),
                &content,
                dpi,
                x_min,
                x_max,
                y_min,
                y_max,
                category_labels,
                category_positions,
            )?;
        // The row that was measured is the row that is drawn.
        renderer.set_x_tick_label_plan(x_tick_label_plan);
        let plot_area = Self::plot_area_from_layout(&layout)?;

        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| Self::scaled_x_pixel(tick, x_min, x_max, plot_area, &self.layout.x_scale))
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| Self::scaled_y_pixel(tick, y_min, y_max, plot_area, &self.layout.y_scale))
            .collect();
        let x_minor_ticks = Self::minor_tick_values_for_scale(
            &x_ticks,
            x_min,
            x_max,
            &self.layout.x_scale,
            self.layout.tick_config.minor_ticks_x,
        );
        let y_minor_ticks = Self::minor_tick_values_for_scale(
            &y_ticks,
            y_min,
            y_max,
            &self.layout.y_scale,
            self.layout.tick_config.minor_ticks_y,
        );
        let x_minor_tick_pixels: Vec<f32> = x_minor_ticks
            .iter()
            .map(|&tick| Self::scaled_x_pixel(tick, x_min, x_max, plot_area, &self.layout.x_scale))
            .collect();
        let y_minor_tick_pixels: Vec<f32> = y_minor_ticks
            .iter()
            .map(|&tick| Self::scaled_y_pixel(tick, y_min, y_max, plot_area, &self.layout.y_scale))
            .collect();

        let draw_axes = Self::needs_cartesian_axes_for_series(&self.series_mgr.series);
        if let Some(panel) = self.themed_panel_background()
            && draw_axes
        {
            renderer.draw_rectangle(
                plot_area.left(),
                plot_area.top(),
                plot_area.width(),
                plot_area.height(),
                panel,
                true,
            )?;
        }
        if self.layout.grid_style.visible && draw_axes {
            let layers = Self::grid_layers(
                &self.layout.grid_style,
                &self.layout.tick_config.grid_mode,
                &x_tick_pixels,
                &y_tick_pixels,
                &x_minor_tick_pixels,
                &y_minor_tick_pixels,
                |points| self.line_width_px(points),
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

        let categorical_x_tick_pixels =
            Self::categorical_x_tick_pixels(plot_area, x_min, x_max, category_positions);

        let draw_ticks = draw_axes && self.layout.tick_config.enabled;

        let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);

        if draw_axes && is_categorical {
            renderer.draw_axis_labels_at_categorical(
                &layout.plot_area,
                category_labels,
                category_positions,
                x_min,
                x_max,
                y_min,
                y_max,
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
        } else if draw_axes {
            renderer.draw_axis_labels_at_scaled(
                &layout.plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
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

        if let Some(ref pos) = layout.xlabel_pos
            && let Some(xlabel) = frame.xlabel.as_deref()
        {
            renderer.draw_xlabel_at(pos, xlabel, self.display.theme.foreground)?;
        }

        if let Some(ref pos) = layout.ylabel_pos
            && let Some(ylabel) = frame.ylabel.as_deref()
        {
            renderer.draw_ylabel_at(pos, ylabel, self.display.theme.foreground)?;
        }

        renderer.draw_annotations_where_scaled(
            &self.annotations,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            dpi,
            &self.layout.x_scale,
            &self.layout.y_scale,
            Self::is_underlay_annotation,
        )?;

        draw_series(
            self,
            &self.series_mgr.series,
            &frame.series,
            &mut renderer,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            render_scale,
            mode,
        )?;

        renderer.draw_annotations_where_scaled(
            &self.annotations,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            dpi,
            &self.layout.x_scale,
            &self.layout.y_scale,
            Self::is_overlay_annotation,
        )?;

        // Frame and ticks last, so data ink can never eat the border it is
        // measured against: a bar sitting on zero would otherwise paint over the
        // bottom spine. The grid, drawn before the series, stays underneath.
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
                &self.themed_tick_sides(self.layout.tick_config.sides),
                &self.themed_spines(),
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
                &self.themed_spines(),
                self.display.theme.foreground,
                axis_width,
                major_tick_size,
                minor_tick_size,
                major_tick_width,
                minor_tick_width,
            )?;
        }

        let legend_items = self.collect_legend_items();
        if !legend_items.is_empty() && frame.style.legend.enabled {
            let occupancy = legend_occupancy(
                &frame.series,
                plot_area,
                (x_min, x_max, y_min, y_max),
                &self.layout.x_scale,
                &self.layout.y_scale,
            );
            renderer.draw_legend_full_resolved(
                &legend_items,
                &frame.style.legend,
                plot_area,
                Some(&occupancy),
                layout.legend_rect.as_ref().map(|rect| rect.bounds()),
            )?;
        }

        let diagnostics = renderer.render_diagnostics().clone();
        Ok((renderer, diagnostics))
    }

    pub(super) fn render_image_with_mode_and_series_renderer<F>(
        &self,
        mode: RenderExecutionMode,
        time: f64,
        draw_series: F,
    ) -> Result<(Image, RenderDiagnostics)>
    where
        F: FnOnce(
            &Plot,
            &[PlotSeries],
            &[ResolvedSeries<'_>],
            &mut SkiaRenderer,
            tiny_skia::Rect,
            f64,
            f64,
            f64,
            f64,
            RenderScale,
            RenderExecutionMode,
        ) -> Result<()>,
    {
        self.validate_before_frame_resolution()?;
        let frame = self.resolve_frame(time)?;
        let style_shell = self.resolved_style_shell(&frame.style);
        let result = style_shell
            .render_renderer_with_resolved_frame(mode, &frame, draw_series)
            .map(|(renderer, diagnostics)| (renderer.into_image(), diagnostics));
        if result.is_ok() {
            frame.acknowledge_rendered(self);
        }
        result
    }

    pub(super) fn render_image_with_resolved_frame(
        &self,
        mode: RenderExecutionMode,
        frame: &ResolvedFrame<'_>,
    ) -> Result<(Image, RenderDiagnostics)> {
        self.render_renderer_with_frame_and_diagnostics(mode, frame)
            .map(|(renderer, diagnostics)| (renderer.into_image(), diagnostics))
    }

    pub(super) fn render_renderer_with_frame_and_diagnostics(
        &self,
        mode: RenderExecutionMode,
        frame: &ResolvedFrame<'_>,
    ) -> Result<(SkiaRenderer, RenderDiagnostics)> {
        self.render_renderer_with_resolved_frame(
            mode,
            frame,
            |plot,
             snapshot_series,
             resolved_series,
             renderer,
             plot_area,
             x_min,
             x_max,
             y_min,
             y_max,
             render_scale,
             mode| {
                if !plot.render_series_collection_auto_datashader(
                    snapshot_series,
                    resolved_series,
                    renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    render_scale,
                    mode,
                )? {
                    plot.render_series_collection_normal(
                        snapshot_series,
                        resolved_series,
                        renderer,
                        plot_area,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        render_scale,
                        mode,
                    )?;
                }
                Ok(())
            },
        )
    }

    fn render_image_with_mode_and_diagnostics(
        &self,
        mode: RenderExecutionMode,
    ) -> Result<(Image, RenderDiagnostics)> {
        self.render_image_with_mode_and_diagnostics_at(mode, 0.0)
    }

    fn render_image_with_mode_and_diagnostics_at(
        &self,
        mode: RenderExecutionMode,
        time: f64,
    ) -> Result<(Image, RenderDiagnostics)> {
        self.render_image_with_mode_and_series_renderer(
            mode,
            time,
            |plot,
             snapshot_series,
             resolved_series,
             renderer,
             plot_area,
             x_min,
             x_max,
             y_min,
             y_max,
             render_scale,
             mode| {
                if !plot.render_series_collection_auto_datashader(
                    snapshot_series,
                    resolved_series,
                    renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    render_scale,
                    mode,
                )? {
                    plot.render_series_collection_normal(
                        snapshot_series,
                        resolved_series,
                        renderer,
                        plot_area,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        render_scale,
                        mode,
                    )?;
                }
                Ok(())
            },
        )
    }

    fn backend_fallback(&self, reason: BackendFallbackReason) -> BackendResolution {
        BackendResolution::new(self.render.backend, BackendType::Skia, Some(reason))
    }

    fn backend_resolution_for_series(
        &self,
        operation: BackendOperation,
        series_list: &[PlotSeries],
    ) -> BackendResolution {
        let Some(requested_backend) = self.render.backend else {
            return BackendResolution::new(None, BackendType::Skia, None);
        };

        if requested_backend == BackendType::Skia {
            return BackendResolution::new(Some(BackendType::Skia), BackendType::Skia, None);
        }

        match requested_backend {
            BackendType::Skia => unreachable!("Skia resolution returned above"),
            // There is no 2D series-parallel raster backend in any build
            // configuration; the `parallel` cargo feature only parallelizes the
            // software 3D tile rasterizer. So this is always a fallback, and the
            // reason never depends on the feature flag.
            BackendType::Parallel => {
                self.backend_fallback(BackendFallbackReason::UnsupportedOperation)
            }
            BackendType::GPU => {
                #[cfg(not(feature = "gpu"))]
                {
                    self.backend_fallback(BackendFallbackReason::FeatureDisabled)
                }
                #[cfg(feature = "gpu")]
                {
                    self.backend_fallback(BackendFallbackReason::UnsupportedOperation)
                }
            }
            BackendType::DataShader => {
                if operation != BackendOperation::Png {
                    return self.backend_fallback(BackendFallbackReason::UnsupportedOperation);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.backend_fallback(BackendFallbackReason::UnsupportedTarget)
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if series_list.is_empty() {
                        return self.backend_fallback(BackendFallbackReason::EmptyPlot);
                    }
                    if Self::has_mixed_coordinate_series(series_list) {
                        return self
                            .backend_fallback(BackendFallbackReason::MixedCoordinateSystems);
                    }
                    if !series_list
                        .iter()
                        .all(Self::series_supports_auto_datashader)
                    {
                        return self.backend_fallback(BackendFallbackReason::UnsupportedSeries);
                    }
                    if !self.datashader_supports_axis_scales() {
                        return self.backend_fallback(BackendFallbackReason::UnsupportedAxisScale);
                    }
                    if !self.datashader_supports_axis_directions() {
                        return self.backend_fallback(BackendFallbackReason::ReversedAxisLimits);
                    }
                    BackendResolution::new(
                        Some(BackendType::DataShader),
                        BackendType::DataShader,
                        None,
                    )
                }
            }
        }
    }

    /// Resolve the configured backend for a specific public raster operation.
    ///
    /// The result separates the stored preference from the backend that can
    /// actually execute and provides a deterministic Skia fallback reason.
    pub fn backend_resolution(&self, operation: BackendOperation) -> BackendResolution {
        self.backend_resolution_for_series(operation, &self.series_mgr.series)
    }

    pub(super) fn render_execution_mode_for_series(
        &self,
        operation: BackendOperation,
        series_list: &[PlotSeries],
    ) -> RenderExecutionMode {
        match self
            .backend_resolution_for_series(operation, series_list)
            .actual_backend()
        {
            BackendType::DataShader => RenderExecutionMode::Optimized,
            BackendType::Skia => RenderExecutionMode::Reference,
            BackendType::Parallel | BackendType::GPU => {
                unreachable!("backend resolution only selects executable paths")
            }
        }
    }

    pub(super) fn render_execution_mode(&self, operation: BackendOperation) -> RenderExecutionMode {
        self.render_execution_mode_for_series(operation, &self.series_mgr.series)
    }

    fn public_png_render_mode_from_resolved(
        &self,
        _resolved_series: &[ResolvedSeries<'_>],
    ) -> RenderExecutionMode {
        self.render_execution_mode(BackendOperation::Png)
    }

    pub(crate) fn should_use_datashader_for_render(
        &self,
        series_list: &[PlotSeries],
        total_points: usize,
    ) -> bool {
        if !self.datashader_supports_axis_scales() || !self.datashader_supports_axis_directions() {
            return false;
        }

        if Self::has_mixed_coordinate_series(series_list)
            || !series_list
                .iter()
                .all(Self::series_supports_auto_datashader)
        {
            return false;
        }

        match self.render.backend {
            Some(BackendType::DataShader) if self.render.auto_optimized => {
                Self::should_auto_use_datashader(series_list, total_points)
            }
            Some(BackendType::DataShader) => !series_list.is_empty(),
            _ => Self::should_auto_use_datashader(series_list, total_points),
        }
    }

    fn datashader_supports_axis_scales(&self) -> bool {
        matches!(self.layout.x_scale, AxisScale::Linear)
            && matches!(self.layout.y_scale, AxisScale::Linear)
    }

    fn datashader_supports_axis_directions(&self) -> bool {
        self.layout
            .x_limits
            .is_none_or(|(x_min, x_max)| x_min < x_max)
            && self
                .layout
                .y_limits
                .is_none_or(|(y_min, y_max)| y_min < y_max)
    }

    /// Render the plot to an in-memory image.
    ///
    /// Reactive plots are first resolved into a static snapshot. Temporal
    /// `Signal` sources are sampled at `0.0`; push-based observables and
    /// streaming sources use their latest values. Use `render_at()` to sample
    /// temporal sources at a different time.
    pub fn render(&self) -> Result<Image> {
        self.render_at(0.0)
    }

    #[cfg(test)]
    pub(super) fn render_optimized_for_test(&self) -> Result<Image> {
        self.validate_before_frame_resolution()?;
        if self.has_dynamic_style_sources() {
            return self
                .render_dynamic_style_frame(RenderExecutionMode::Optimized, 0.0)
                .map(|(image, _)| image);
        }
        self.render_image_with_mode(RenderExecutionMode::Optimized)
    }

    #[cfg(test)]
    pub(super) fn render_optimized_for_test_with_diagnostics(
        &self,
    ) -> Result<(Image, RenderDiagnostics)> {
        self.validate_before_frame_resolution()?;
        if self.has_dynamic_style_sources() {
            return self.render_dynamic_style_frame(RenderExecutionMode::Optimized, 0.0);
        }
        self.render_image_with_mode_and_diagnostics(RenderExecutionMode::Optimized)
    }

    /// Render the plot using a caller-provided temporal sample time.
    ///
    /// Static plots delegate to [`render`](Self::render). Reactive plots first
    /// resolve a static snapshot, sampling temporal `Signal` sources at `time`
    /// and reading push-based observables and streaming sources at their latest
    /// values, then run through the usual backend-selection path.
    ///
    /// # Arguments
    ///
    /// * `time` - Time used to sample temporal `Signal` sources before rendering
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::data::Signal;
    /// use ruviz::prelude::*;
    ///
    /// let title = Signal::new(|t| format!("t = {:.1}s", t));
    /// let plot = Plot::new()
    ///     .title_signal(title)
    ///     .line(&[0.0, 1.0], &[0.0, 1.0])
    ///     .end_series();
    ///
    /// // Samples the signal-backed title at t = 1.5 before rendering.
    /// let image = plot.render_at(1.5)?;
    /// # let _ = image;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn render_at(&self, time: f64) -> Result<Image> {
        self.validate_before_frame_resolution()?;
        let mode = self.render_execution_mode(BackendOperation::RasterImage);
        if self.has_dynamic_style_sources() {
            return self
                .render_dynamic_style_frame(mode, time)
                .map(|(image, _)| image);
        }
        self.render_image_with_mode_at(mode, time)
    }

    /// Render the plot and encode it as PNG bytes.
    pub fn render_png_bytes(&self) -> Result<Vec<u8>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.save_png_bytes_with_backend()
                .map(|(png_bytes, _, _)| png_bytes)
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.render()?.encode_png()
        }
    }

    /// Render the plot to PNG bytes while reporting internal raster diagnostics.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(hidden)]
    pub fn benchmark_render_png_bytes_with_diagnostics(
        &self,
    ) -> Result<(Vec<u8>, RenderDiagnostics)> {
        self.save_png_bytes_with_backend()
            .map(|(png_bytes, _, diagnostics)| (png_bytes, diagnostics))
    }

    /// Render PNG bytes and include the deterministic backend decision.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(hidden)]
    pub fn benchmark_render_png_bytes_with_backend_resolution(
        &self,
    ) -> Result<(Vec<u8>, RenderDiagnostics, BackendResolution)> {
        let resolution = self.backend_resolution(BackendOperation::Png);
        self.save_png_bytes_with_backend()
            .map(|(png_bytes, _, diagnostics)| (png_bytes, diagnostics, resolution))
    }

    /// Check if this plot contains any reactive data (Signal or Observable).
    ///
    /// Returns `true` if any series data or text attributes are reactive,
    /// `false` if all data is static.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::prelude::*;
    /// use ruviz::animation::signal;
    ///
    /// let x = vec![0.0, 1.0];
    /// let y = vec![0.0, 1.0];
    /// let static_plot = Plot::new().line(&x, &y);
    /// assert!(!static_plot.is_reactive());
    ///
    /// let title = signal::of(|t| format!("t = {:.1}", t));
    /// let reactive_plot = Plot::new().title_signal(title).line(&x, &y);
    /// assert!(reactive_plot.is_reactive());
    /// ```
    pub fn is_reactive(&self) -> bool {
        self.display
            .title
            .as_ref()
            .is_some_and(|title| title.is_reactive())
            || self
                .display
                .xlabel
                .as_ref()
                .is_some_and(|label| label.is_reactive())
            || self
                .display
                .ylabel
                .as_ref()
                .is_some_and(|label| label.is_reactive())
            || self.series_mgr.series.iter().any(PlotSeries::is_reactive)
    }

    /// Render the plot to an external renderer (used for subplots)
    pub fn render_to_renderer(&self, renderer: &mut SkiaRenderer, dpi: f32) -> Result<()> {
        self.validate_before_frame_resolution()?;
        let frame = self.resolve_frame(0.0)?;
        let mut plot = self.resolved_style_shell(&frame.style);
        plot.display.config.figure.dpi = dpi;
        let plot = plot.set_subplot_output_pixels(renderer.width(), renderer.height());
        let mode = plot.render_execution_mode(BackendOperation::RasterImage);
        let (subplot_renderer, _) =
            plot.render_renderer_with_frame_and_diagnostics(mode, &frame)?;
        renderer.draw_image_layer(&subplot_renderer.into_image_demultiplied(), 0, 0)?;
        frame.acknowledge_rendered(self);
        Ok(())
    }

    /// Calculate total number of data points across all series
    pub(super) fn create_plot_content_at_time(
        &self,
        y_min: f64,
        y_max: f64,
        time: f64,
    ) -> PlotContent {
        self.create_plot_content_with_text(
            y_min,
            y_max,
            self.display.title.as_ref().map(|title| title.resolve(time)),
            self.display
                .xlabel
                .as_ref()
                .map(|label| label.resolve(time)),
            self.display
                .ylabel
                .as_ref()
                .map(|label| label.resolve(time)),
        )
    }

    pub(super) fn create_plot_content_from_resolved_text(
        &self,
        y_min: f64,
        y_max: f64,
        frame: &ResolvedFrame<'_>,
    ) -> PlotContent {
        self.create_plot_content_with_text(
            y_min,
            y_max,
            frame.title.clone(),
            frame.xlabel.clone(),
            frame.ylabel.clone(),
        )
    }

    fn create_plot_content_with_text(
        &self,
        y_min: f64,
        y_max: f64,
        title: Option<String>,
        xlabel: Option<String>,
        ylabel: Option<String>,
    ) -> PlotContent {
        // Estimate max characters in y-tick labels
        let y_ticks = generate_ticks(y_min, y_max, 6);
        let max_ytick_chars = y_ticks
            .iter()
            .map(|&v| {
                if v.abs() < 0.001 {
                    1 // "0"
                } else if v.abs() > 1000.0 {
                    format!("{:.0e}", v).len()
                } else {
                    format!("{:.1}", v).len()
                }
            })
            .max()
            .unwrap_or(5);

        PlotContent {
            title,
            xlabel,
            ylabel,
            show_tick_labels: self.layout.tick_config.enabled && self.needs_cartesian_axes(),
            max_ytick_chars,
            max_xtick_chars: 0, // Compatibility-only field; current layout ignores it.
        }
    }

    /// Create PlotContent for layout calculation.
    pub(super) fn create_plot_content(&self, y_min: f64, y_max: f64) -> PlotContent {
        self.create_plot_content_at_time(y_min, y_max, 0.0)
    }

    /// The colorbar whose width the layout has to reserve room for.
    ///
    /// One colorbar is drawn (they all share `colorbar_rect`), so the first
    /// series that asks for one is the one measured. It is the *same*
    /// [`ColorbarRequest`] the drawing code will use, which is what stops the
    /// margin being measured from one range and the strip drawn from another.
    fn colorbar_measurement_spec(&self) -> Option<crate::render::colorbar::ColorbarRequest> {
        self.series_mgr
            .series
            .iter()
            .find_map(|series| self.series_colorbar_request(&series.series_type))
    }

    fn measure_colorbar_right_margin(
        &self,
        renderer: &SkiaRenderer,
        spec: &crate::render::colorbar::ColorbarRequest,
    ) -> Result<f32> {
        let render_scale = renderer.render_scale();
        let colorbar_width = render_scale.logical_pixels_to_pixels(COLORBAR_WIDTH_PX);
        let colorbar_margin = render_scale.logical_pixels_to_pixels(COLORBAR_MARGIN_PX);
        let tick_font_size = render_scale.points_to_pixels(spec.tick_font_size);
        let label_font_size = render_scale.points_to_pixels(spec.label_font_size);
        let ticks = crate::render::skia::compute_colorbar_ticks(
            spec.vmin,
            spec.vmax,
            &spec.value_scale,
            spec.show_log_subticks,
        );
        let max_label_width =
            Self::measure_tick_label_extent(renderer, &ticks.major_labels, tick_font_size)?
                .map(|(width, _)| width)
                .unwrap_or(0.0);
        let rotated_label_width = if let Some(label) = spec.label.as_deref() {
            renderer.measure_text(label, label_font_size)?.1
        } else {
            0.0
        };
        let layout = crate::render::skia::compute_colorbar_layout_metrics(
            colorbar_width,
            tick_font_size,
            max_label_width,
            spec.label.as_ref().map(|_| rotated_label_width),
        );
        let outer_padding = tick_font_size.max(4.0) * 0.5;

        Ok(colorbar_margin + layout.total_extent + outer_padding)
    }

    /// Pre-measure title/xlabel/ylabel for Typst layout parity.
    pub(super) fn measure_layout_text(
        &self,
        renderer: &SkiaRenderer,
        content: &PlotContent,
        dpi: f32,
    ) -> Result<Option<LayoutMeasurements>> {
        let render_scale = RenderScale::new(dpi);
        let title_size_px =
            render_scale.points_to_pixels(self.display.config.typography.title_size());
        let label_size_px =
            render_scale.points_to_pixels(self.display.config.typography.label_size());

        let mut measurements = LayoutMeasurements::default();

        if let Some(title) = content.title.as_deref() {
            measurements.title = Some(renderer.measure_text_with_weight(
                title,
                title_size_px,
                self.display.config.typography.title_weight,
            )?);
        }
        if let Some(xlabel) = content.xlabel.as_deref() {
            measurements.xlabel = Some(renderer.measure_text(xlabel, label_size_px)?);
        }
        if let Some(ylabel) = content.ylabel.as_deref() {
            measurements.ylabel = Some(renderer.measure_text(ylabel, label_size_px)?);
        }

        Ok(Some(measurements))
    }

    pub(super) fn measure_layout_text_with_ticks(
        &self,
        renderer: &SkiaRenderer,
        content: &PlotContent,
        dpi: f32,
        x_tick_labels: &[String],
        y_tick_labels: &[String],
    ) -> Result<Option<LayoutMeasurements>> {
        let tick_size_px =
            RenderScale::new(dpi).points_to_pixels(self.display.config.typography.tick_size());
        let mut measurements = self
            .measure_layout_text(renderer, content, dpi)?
            .unwrap_or_default();

        if content.show_tick_labels {
            measurements.xtick =
                Self::measure_tick_label_extent(renderer, x_tick_labels, tick_size_px)?;
            measurements.ytick =
                Self::measure_tick_label_extent(renderer, y_tick_labels, tick_size_px)?;
        }
        if let Some(spec) = self.colorbar_measurement_spec() {
            measurements.right_margin = Some(self.measure_colorbar_right_margin(renderer, &spec)?);
        }
        let legend = self
            .layout
            .legend
            .to_legend(self.display.config.typography.legend_size());
        let legend_items = self.collect_legend_items();
        if legend.enabled && legend.position.is_outside() && !legend_items.is_empty() {
            measurements.legend = Some(renderer.measure_legend(&legend_items, &legend)?);
        }

        Ok(Some(measurements))
    }

    pub(super) fn compute_layout_from_measurements(
        &self,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        measurements: Option<&LayoutMeasurements>,
    ) -> ResolvedLayout {
        let measured_dimensions = measurements.map(|m| &m.dimensions);
        let layout = match &self.display.config.margins {
            MarginConfig::ContentDriven {
                edge_buffer,
                center_plot,
            } => LayoutCalculator::new(LayoutConfig {
                edge_buffer_pt: *edge_buffer,
                center_plot: *center_plot,
                ..Default::default()
            })
            .compute(
                canvas_size,
                content,
                &self.display.config.typography,
                &self.display.config.spacing,
                dpi,
                measured_dimensions,
            ),
            MarginConfig::Fixed { .. }
            | MarginConfig::Auto { .. }
            | MarginConfig::Proportional { .. } => self.compute_layout_with_explicit_margins(
                canvas_size,
                content,
                dpi,
                measured_dimensions,
            ),
        };
        self.reserve_outside_legend(
            ResolvedLayout {
                layout,
                legend_rect: None,
            },
            canvas_size,
            dpi,
            measurements,
        )
    }

    fn reserve_outside_legend(
        &self,
        mut layout: ResolvedLayout,
        canvas_size: (u32, u32),
        dpi: f32,
        measurements: Option<&LayoutMeasurements>,
    ) -> ResolvedLayout {
        let Some((legend_width, legend_height)) = measurements.and_then(|m| m.legend) else {
            return layout;
        };
        let position = self.layout.legend.position;
        if !position.is_outside() {
            return layout;
        }

        let legend = self
            .layout
            .legend
            .to_legend(self.display.config.typography.legend_size())
            .scaled_for_render(RenderScale::from_canvas_size(
                canvas_size.0,
                canvas_size.1,
                dpi,
            ));
        let pad = legend.spacing.to_pixels(legend.font_size).border_axes_pad;
        let canvas_width = canvas_size.0 as f32;
        let canvas_height = canvas_size.1 as f32;

        // Cap the reserved band so the data rectangle keeps a usable minimum
        // extent even for very wide labels or many legend rows. When capped,
        // the legend frame shrinks and its content may extend past the canvas
        // edge, but layout stays valid instead of failing to render.
        const MIN_PLOT_EXTENT_PX: f32 = 40.0;
        let max_horizontal_band =
            (layout.plot_area.right - layout.plot_area.left - MIN_PLOT_EXTENT_PX).max(0.0);
        let max_vertical_band =
            (layout.plot_area.bottom - layout.plot_area.top - MIN_PLOT_EXTENT_PX).max(0.0);
        let horizontal_band = (legend_width + pad * 2.0).min(max_horizontal_band);
        let vertical_band = (legend_height + pad * 2.0).min(max_vertical_band);
        let legend_width = match position {
            LegendPosition::OutsideRight | LegendPosition::OutsideLeft => {
                (horizontal_band - pad * 2.0).max(0.0)
            }
            _ => legend_width.min((canvas_width - pad * 2.0).max(0.0)),
        };
        let legend_height = match position {
            LegendPosition::OutsideUpper | LegendPosition::OutsideLower => {
                (vertical_band - pad * 2.0).max(0.0)
            }
            _ => legend_height.min((canvas_height - pad * 2.0).max(0.0)),
        };

        match position {
            LegendPosition::OutsideRight => {
                layout.plot_area.right -= horizontal_band;
                layout.margins.right += horizontal_band;
                let top = layout
                    .plot_area
                    .top
                    .clamp(pad, (canvas_height - legend_height - pad).max(pad));
                layout.legend_rect = Some(crate::core::layout::LayoutRect {
                    left: canvas_width - legend_width - pad,
                    top,
                    right: canvas_width - pad,
                    bottom: top + legend_height,
                });
            }
            LegendPosition::OutsideLeft => {
                layout.plot_area.left += horizontal_band;
                layout.margins.left += horizontal_band;
                layout.ytick_right_x += horizontal_band;
                if let Some(pos) = layout.ylabel_pos.as_mut() {
                    pos.x += horizontal_band;
                }
                let top = layout
                    .plot_area
                    .top
                    .clamp(pad, (canvas_height - legend_height - pad).max(pad));
                layout.legend_rect = Some(crate::core::layout::LayoutRect {
                    left: pad,
                    top,
                    right: pad + legend_width,
                    bottom: top + legend_height,
                });
            }
            LegendPosition::OutsideUpper => {
                layout.plot_area.top += vertical_band;
                layout.margins.top += vertical_band;
                if let Some(pos) = layout.title_pos.as_mut() {
                    pos.y += vertical_band;
                }
                if let Some(pos) = layout.ylabel_pos.as_mut() {
                    pos.y += vertical_band * 0.5;
                }
                let left = (layout.plot_area.right - legend_width)
                    .clamp(pad, (canvas_width - legend_width - pad).max(pad));
                layout.legend_rect = Some(crate::core::layout::LayoutRect {
                    left,
                    top: pad,
                    right: left + legend_width,
                    bottom: pad + legend_height,
                });
            }
            LegendPosition::OutsideLower => {
                layout.plot_area.bottom -= vertical_band;
                layout.margins.bottom += vertical_band;
                layout.xtick_baseline_y -= vertical_band;
                if let Some(pos) = layout.xlabel_pos.as_mut() {
                    pos.y -= vertical_band;
                }
                if let Some(pos) = layout.ylabel_pos.as_mut() {
                    pos.y -= vertical_band * 0.5;
                }
                let left = (layout.plot_area.right - legend_width)
                    .clamp(pad, (canvas_width - legend_width - pad).max(pad));
                layout.legend_rect = Some(crate::core::layout::LayoutRect {
                    left,
                    top: canvas_height - legend_height - pad,
                    right: left + legend_width,
                    bottom: canvas_height - pad,
                });
            }
            _ => {}
        }

        if position.is_outside() {
            let center_x = layout.plot_area.center_x();
            if let Some(pos) = layout.title_pos.as_mut() {
                pos.x = center_x;
            }
            if let Some(pos) = layout.xlabel_pos.as_mut() {
                pos.x = center_x;
            }
        }
        layout
    }

    fn compute_layout_with_explicit_margins(
        &self,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        measurements: Option<&MeasuredDimensions>,
    ) -> PlotLayout {
        let render_scale = RenderScale::from_canvas_size(canvas_size.0, canvas_size.1, dpi);
        let typography = &self.display.config.typography;
        let spacing = &self.display.config.spacing;
        let title_pad = render_scale.points_to_pixels(spacing.title_pad);
        let label_pad = render_scale.points_to_pixels(spacing.label_pad);
        let tick_pad_px = render_scale.points_to_pixels(spacing.tick_pad);
        let title_size_px = render_scale.points_to_pixels(typography.title_size());
        let label_size_px = render_scale.points_to_pixels(typography.label_size());
        let tick_size_px = render_scale.points_to_pixels(typography.tick_size());
        let measured_title = measurements.and_then(|m| m.title);
        let measured_xlabel = measurements.and_then(|m| m.xlabel);
        let measured_ylabel = measurements.and_then(|m| m.ylabel);
        let measured_xtick = measurements.and_then(|m| m.xtick);
        let measured_ytick = measurements.and_then(|m| m.ytick);
        let measured_right_margin = measurements.and_then(|m| m.right_margin);

        let title_height = if content.title.is_some() {
            measured_title
                .map(|(_, height)| height)
                .unwrap_or_else(|| crate::core::layout::estimate_text_height(title_size_px))
        } else {
            0.0
        };
        let xlabel_height = if content.xlabel.is_some() {
            measured_xlabel
                .map(|(_, height)| height)
                .unwrap_or_else(|| crate::core::layout::estimate_text_height(label_size_px))
        } else {
            0.0
        };
        let ylabel_width = if content.ylabel.is_some() {
            measured_ylabel
                .map(|(_, height)| height)
                .unwrap_or_else(|| crate::core::layout::estimate_text_height(label_size_px))
        } else {
            0.0
        };
        let (xtick_height, ytick_width, tick_pad) = if content.show_tick_labels {
            (
                measured_xtick
                    .map(|(_, height)| height)
                    .unwrap_or_else(|| crate::core::layout::estimate_text_height(tick_size_px)),
                measured_ytick.map(|(width, _)| width).unwrap_or_else(|| {
                    crate::core::layout::estimate_tick_label_width(
                        content.max_ytick_chars.max(5),
                        tick_size_px,
                    )
                }),
                tick_pad_px,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let computed_margins = self.display.config.compute_margins(
            content.title.is_some(),
            content.xlabel.is_some(),
            content.ylabel.is_some(),
        );
        let plot_area_rect =
            calculate_plot_area_config(canvas_size.0, canvas_size.1, &computed_margins, dpi);
        let canvas_width = canvas_size.0 as f32;
        let canvas_height = canvas_size.1 as f32;
        let configured_right_margin = canvas_width - plot_area_rect.right();
        let effective_right_margin =
            configured_right_margin.max(measured_right_margin.unwrap_or(0.0));
        let margins = crate::core::layout::ComputedMarginsPixels {
            left: plot_area_rect.left(),
            right: effective_right_margin,
            top: plot_area_rect.top(),
            bottom: canvas_height - plot_area_rect.bottom(),
        };
        let plot_area = crate::core::layout::LayoutRect {
            left: plot_area_rect.left(),
            top: plot_area_rect.top(),
            right: canvas_width - effective_right_margin,
            bottom: plot_area_rect.bottom(),
        };

        let top_outer_gap = if content.title.is_some() {
            (margins.top - title_height - title_pad).max(0.0)
        } else {
            0.0
        };
        let bottom_content_height = tick_pad
            + xtick_height
            + if content.xlabel.is_some() {
                label_pad + xlabel_height
            } else {
                0.0
            };
        let bottom_outer_gap = (margins.bottom - bottom_content_height).max(0.0);
        let left_content_width = ytick_width
            + tick_pad
            + if content.ylabel.is_some() {
                label_pad + ylabel_width
            } else {
                0.0
            };
        let left_outer_gap = (margins.left - left_content_width).max(0.0);

        PlotLayout {
            plot_area,
            title_pos: content
                .title
                .as_ref()
                .map(|_| crate::core::layout::TextPosition {
                    x: plot_area.center_x(),
                    y: top_outer_gap,
                    size: title_size_px,
                }),
            xlabel_pos: content
                .xlabel
                .as_ref()
                .map(|_| crate::core::layout::TextPosition {
                    x: plot_area.center_x(),
                    y: canvas_height - bottom_outer_gap - xlabel_height,
                    size: label_size_px,
                }),
            ylabel_pos: content
                .ylabel
                .as_ref()
                .map(|_| crate::core::layout::TextPosition {
                    x: left_outer_gap + ylabel_width / 2.0,
                    y: plot_area.center_y(),
                    size: label_size_px,
                }),
            xtick_baseline_y: plot_area.bottom + tick_pad,
            ytick_right_x: plot_area.left - tick_pad,
            margins,
        }
    }

    pub(super) fn plot_area_from_layout(layout: &PlotLayout) -> Result<tiny_skia::Rect> {
        tiny_skia::Rect::from_ltrb(
            layout.plot_area.left,
            layout.plot_area.top,
            layout.plot_area.right,
            layout.plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area from layout".to_string(),
            position: None,
        })
    }

    pub(super) fn configured_major_ticks(
        &self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        (
            crate::axes::generate_ticks_for_scale(
                x_min,
                x_max,
                self.layout.tick_config.major_ticks_x,
                &self.layout.x_scale,
            ),
            crate::axes::generate_ticks_for_scale(
                y_min,
                y_max,
                self.layout.tick_config.major_ticks_y,
                &self.layout.y_scale,
            ),
        )
    }

    pub(super) fn compute_layout_with_configured_ticks(
        &self,
        renderer: &SkiaRenderer,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<(ResolvedLayout, Vec<f64>, Vec<f64>)> {
        let (measurements, x_ticks, y_ticks) =
            self.measure_configured_ticks(renderer, content, dpi, x_min, x_max, y_min, y_max)?;
        let layout =
            self.compute_layout_from_measurements(canvas_size, content, dpi, measurements.as_ref());

        Ok((layout, x_ticks, y_ticks))
    }

    /// The same layout, plus the plan its categorical x tick labels are drawn
    /// with.
    ///
    /// Categorical labels are user strings: ten region names collide into one
    /// illegible run that no tick-count budget can prevent. This is where that
    /// row is measured and where the bottom margin is reserved from the answer —
    /// before the plot area is computed, so the labels are given room rather
    /// than clipped. `category_labels` empty (a numeric axis) leaves both the
    /// layout and the plan exactly as they were.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compute_layout_with_categorical_ticks(
        &self,
        renderer: &SkiaRenderer,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        category_labels: &[String],
        category_positions: &[f64],
    ) -> Result<(ResolvedLayout, Vec<f64>, Vec<f64>, XTickLabelPlan)> {
        let (measurements, x_ticks, y_ticks) =
            self.measure_configured_ticks(renderer, content, dpi, x_min, x_max, y_min, y_max)?;
        let layout =
            self.compute_layout_from_measurements(canvas_size, content, dpi, measurements.as_ref());
        let (layout, plan) = self.resolve_x_tick_label_row(
            renderer,
            canvas_size,
            content,
            dpi,
            measurements.as_ref(),
            layout,
            category_labels,
            category_positions,
            x_min,
            x_max,
        )?;

        Ok((layout, x_ticks, y_ticks, plan))
    }

    #[allow(clippy::too_many_arguments)]
    fn measure_configured_ticks(
        &self,
        renderer: &SkiaRenderer,
        content: &PlotContent,
        dpi: f32,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<(Option<LayoutMeasurements>, Vec<f64>, Vec<f64>)> {
        if !content.show_tick_labels {
            let measurements =
                self.measure_layout_text_with_ticks(renderer, content, dpi, &[], &[])?;
            return Ok((measurements, Vec::new(), Vec::new()));
        }

        let (x_ticks, y_ticks) = self.configured_major_ticks(x_min, x_max, y_min, y_max);
        // Must be the scale-aware formatter: the renderer draws a log axis as
        // "10³", so measuring "1000" here would size the margins for a string
        // that is never drawn.
        let x_labels = crate::axes::format_tick_labels_for_scale(&x_ticks, &self.layout.x_scale);
        let y_labels = crate::axes::format_tick_labels_for_scale(&y_ticks, &self.layout.y_scale);
        let measurements =
            self.measure_layout_text_with_ticks(renderer, content, dpi, &x_labels, &y_labels)?;

        Ok((measurements, x_ticks, y_ticks))
    }

    /// Decide how the categorical x tick label row is drawn, and re-reserve the
    /// bottom margin from that decision.
    ///
    /// The row is measured against the plot area the horizontal pass produced.
    /// That is sound because the only margin this can change is the bottom one,
    /// and the bottom margin does not move the plot area's left or right edge —
    /// so a label measured here is drawn at the same x.
    ///
    /// Whether a rotated row *fits* is a question only the layout can answer:
    /// a content-driven layout caps its margins by canvas fraction and a fixed
    /// one does not grow at all. So the trial layout below is computed and then
    /// asked, rather than the margin arithmetic being restated here.
    #[allow(clippy::too_many_arguments)]
    fn resolve_x_tick_label_row(
        &self,
        renderer: &SkiaRenderer,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        measurements: Option<&LayoutMeasurements>,
        layout: ResolvedLayout,
        category_labels: &[String],
        category_positions: &[f64],
        x_min: f64,
        x_max: f64,
    ) -> Result<(ResolvedLayout, XTickLabelPlan)> {
        if category_labels.is_empty() || !content.show_tick_labels {
            return Ok((layout, XTickLabelPlan::default()));
        }

        let tick_size_px =
            RenderScale::new(dpi).points_to_pixels(self.display.config.typography.tick_size());
        let centers = SkiaRenderer::categorical_label_centers(
            &layout.plot_area,
            category_positions,
            x_min,
            x_max,
        );
        // The row is kept inside the canvas, not inside the plot area: an end
        // label is allowed to overhang its own slot into the outer margin (that
        // is where a first or last category name lives), it is only stopped
        // from running off the figure.
        let metrics = renderer.measure_x_tick_row(
            category_labels,
            &centers,
            tick_size_px,
            XTickRowBounds::canvas(canvas_size.0 as f32),
        )?;
        if metrics.horizontal_extent <= 0.0 {
            // Every slot is unnamed: nothing is drawn, so nothing is reserved.
            return Ok((layout, XTickLabelPlan::default()));
        }

        let rotation = self.layout.tick_config.xtick_rotation;
        let rotated_fits = metrics.wants_rotation(rotation) && {
            let trial = self.layout_with_x_tick_extent(
                canvas_size,
                content,
                dpi,
                measurements,
                &metrics,
                metrics.max_label_width,
            );
            Self::x_tick_row_fits(&trial, canvas_size.1 as f32, metrics.max_label_width)
        };

        let plan = metrics.plan(rotation, rotated_fits);
        let layout = self.layout_with_x_tick_extent(
            canvas_size,
            content,
            dpi,
            measurements,
            &metrics,
            plan.extent,
        );

        Ok((layout, plan))
    }

    /// The layout that reserves `extent` vertical pixels for the x tick labels.
    fn layout_with_x_tick_extent(
        &self,
        canvas_size: (u32, u32),
        content: &PlotContent,
        dpi: f32,
        measurements: Option<&LayoutMeasurements>,
        metrics: &XTickRowMetrics,
        extent: f32,
    ) -> ResolvedLayout {
        let mut measurements = measurements.cloned().unwrap_or_default();
        measurements.xtick = Some((metrics.max_label_width, extent));
        self.compute_layout_from_measurements(canvas_size, content, dpi, Some(&measurements))
    }

    /// Whether a label row `extent` pixels tall clears the x label and the
    /// bottom of the canvas.
    fn x_tick_row_fits(layout: &ResolvedLayout, canvas_height: f32, extent: f32) -> bool {
        let limit = layout
            .xlabel_pos
            .as_ref()
            .map_or(canvas_height, |position| position.y);
        layout.xtick_baseline_y + extent <= limit
    }

    fn measure_tick_label_extent(
        renderer: &SkiaRenderer,
        labels: &[String],
        tick_size_px: f32,
    ) -> Result<Option<(f32, f32)>> {
        let mut max_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;

        for label in labels {
            let (width, height) = renderer.measure_label_text(label, tick_size_px)?;
            max_width = max_width.max(width);
            max_height = max_height.max(height);
        }

        if labels.is_empty() {
            Ok(None)
        } else {
            Ok(Some((max_width, max_height)))
        }
    }

    /// Render plot using DataShader optimization for large datasets
    pub(super) fn render_with_datashader(&self, series_list: &[PlotSeries]) -> Result<Image> {
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();

        // Collect all points from all series
        for series in series_list {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                        if x.is_finite() && y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::ErrorBars { x_data, y_data, .. }
                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                        if x.is_finite() && y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::Bar { values, .. } => {
                    let values = values.resolve_cow(0.0);
                    // For bar charts, convert category indices to points
                    for (i, &value) in values.iter().enumerate() {
                        if value.is_finite() {
                            x_values.push(i as f64);
                            y_values.push(value);
                        }
                    }
                }
                SeriesType::Heatmap { data } => {
                    // Heatmap has its own grid, convert to points
                    for (row, row_values) in data.values.iter().enumerate() {
                        for (col, &value) in row_values.iter().enumerate() {
                            if value.is_finite() {
                                x_values.push(col as f64);
                                y_values.push(row as f64);
                            }
                        }
                    }
                }
                SeriesType::Histogram { .. } => {
                    if let Ok(hist_data) = series.series_type.histogram_data_at(0.0) {
                        for (i, &count) in hist_data.counts.iter().enumerate() {
                            if count > 0.0 {
                                let x_center =
                                    (hist_data.bin_edges[i] + hist_data.bin_edges[i + 1]) / 2.0;
                                x_values.push(x_center);
                                y_values.push(count);
                            }
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Kde { data } => {
                    // Add KDE points
                    for (&x, &y) in data.x.iter().zip(data.y.iter()) {
                        if x.is_finite() && y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::Ecdf { data } => {
                    // Add ECDF points
                    for (&x, &y) in data.x.iter().zip(data.y.iter()) {
                        if x.is_finite() && y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::Violin { data } => {
                    // Add violin KDE points, at the centre of the category slot
                    // this violin was assigned.
                    for &y in &data.kde.x {
                        let x = data.config.x_center();
                        if y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::Boxen { data } => {
                    // Add boxen box corner points
                    for boxen_box in &data.boxes {
                        let rect = crate::plots::distribution::boxen_rect(
                            boxen_box,
                            data.config.x_center(),
                            data.config.orient,
                        );
                        for (x, y) in rect {
                            if x.is_finite() && y.is_finite() {
                                x_values.push(x);
                                y_values.push(y);
                            }
                        }
                    }
                }
                SeriesType::Quiver { data } => {
                    for arrow in &data.arrows {
                        for (x, y) in [arrow.start, arrow.end] {
                            if x.is_finite() && y.is_finite() {
                                x_values.push(x);
                                y_values.push(y);
                            }
                        }
                    }
                }
                SeriesType::Contour { data } => {
                    // Add contour line segment endpoints
                    for level in &data.lines {
                        for &(x1, y1, x2, y2) in &level.segments {
                            if x1.is_finite() && y1.is_finite() {
                                x_values.push(x1);
                                y_values.push(y1);
                            }
                            if x2.is_finite() && y2.is_finite() {
                                x_values.push(x2);
                                y_values.push(y2);
                            }
                        }
                    }
                }
                SeriesType::Pie { .. } => {
                    // Pie charts don't use point-based datashader, use normalized coords
                    x_values.push(0.5);
                    y_values.push(0.5);
                }
                SeriesType::Radar { data } => {
                    // Add radar series points (already in cartesian coordinates from polygon)
                    for series_data in &data.series {
                        for &(x, y) in &series_data.polygon {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
                SeriesType::Polar { data } => {
                    // Add polar plot points (already in cartesian coordinates)
                    for point in &data.points {
                        x_values.push(point.x);
                        y_values.push(point.y);
                    }
                }
                SeriesType::Computed { data } => {
                    // A compute-only plot type states its own extent; the
                    // corners of it are what the datashader grid needs.
                    let ((x0, x1), (y0, y1)) =
                        crate::plots::traits::PlotData::data_bounds(data.as_ref());
                    for (x, y) in [(x0, y0), (x1, y1)] {
                        if x.is_finite() && y.is_finite() {
                            x_values.push(x);
                            y_values.push(y);
                        }
                    }
                }
            }
        }

        if x_values.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }

        // Simple DataShader implementation - create basic aggregated image
        let (canvas_width, canvas_height) = self.config_canvas_size();
        let mut datashader =
            DataShader::with_canvas_size(canvas_width as usize, canvas_height as usize);
        let (x_min, x_max, y_min, y_max) = self.effective_data_bounds_for_series(series_list)?;

        datashader.aggregate_with_bounds(&x_values, &y_values, x_min, x_max, y_min, y_max)?;
        let ds_image = datashader.render();

        // Convert to Image format
        let image = Image::from_straight_rgba(
            ds_image.width as u32,
            ds_image.height as u32,
            ds_image.pixels,
        );

        Ok(image)
    }
    /// Select a backend that is safe for every public raster output path.
    ///
    /// Automatic routing remains on the reference Skia backend until another
    /// backend has a parity-approved path for the requested operation. Explicit
    /// compatible DataShader PNG output remains available through
    /// [`backend`](Self::backend).
    ///
    /// If a backend was explicitly set with `.backend()`, that choice is respected.
    pub fn auto_optimize(self) -> Self {
        self.auto_optimize_with_extra_points(0)
    }

    /// Apply conservative auto-selection while a builder still owns a pending series.
    pub(crate) fn auto_optimize_with_extra_points(mut self, _extra_points: usize) -> Self {
        if self.render.backend.is_some() {
            return self;
        }

        self.render.backend = Some(BackendType::Skia);
        self.render.auto_optimized = true;
        self
    }

    /// Set backend explicitly (overrides auto-optimization)
    pub fn backend(mut self, backend: BackendType) -> Self {
        self.render.backend = Some(backend);
        self.render.auto_optimized = false;
        self
    }

    /// Store a GPU backend preference for APIs that inspect plot configuration.
    ///
    /// Public raster operations currently resolve this preference to Skia and
    /// report the fallback through [`backend_resolution`](Self::backend_resolution).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let large_x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    /// let large_y: Vec<f64> = large_x.iter().map(|x| x * x).collect();
    ///
    /// Plot::new()
    ///     .gpu(true)
    ///     .line(&large_x, &large_y)
    ///     .save("gpu_plot.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Requirements
    ///
    /// Requires the `gpu` feature to be enabled.
    #[cfg(feature = "gpu")]
    pub fn gpu(mut self, enabled: bool) -> Self {
        self.render.enable_gpu = enabled;
        if enabled {
            self.render.backend = Some(BackendType::GPU);
            self.render.auto_optimized = false;
        }
        self
    }

    /// Get the current backend name (for testing)
    pub fn get_backend_name(&self) -> &'static str {
        self.render.backend.map_or("auto", BackendType::as_str)
    }

    /// Return the backend that the public PNG render/save path will use today.
    ///
    /// This differs from [`get_backend_name`](Self::get_backend_name), which
    /// reports the configured backend preference. Unsupported optimized backend
    /// preferences fall back to the reference Skia raster path.
    pub fn resolved_backend_name(&self) -> &'static str {
        self.backend_resolution(BackendOperation::Png)
            .actual_backend()
            .as_str()
    }

    /// Save the plot to a PNG file.
    ///
    /// Renders the plot and saves it to the specified path. Reactive plots are
    /// first resolved into a static snapshot: temporal `Signal` sources are
    /// sampled at `0.0`, while push-based observables and streaming sources use
    /// their latest values.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .title("Saved Plot")
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .end_series()
    ///     .save("output.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.validate_before_frame_resolution()?;
        let (png_bytes, _, _, frame) = self.save_png_bytes_with_backend_unacknowledged()?;
        crate::export::write_bytes_atomic(path, &png_bytes)?;
        frame.acknowledge_rendered(&self);
        Ok(())
    }

    /// Render PNG bytes through the same backend-selection path used by `save()`.
    ///
    /// This exists for benchmark tooling so we can measure the `save()` backend
    /// without file-system I/O.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(hidden)]
    pub fn benchmark_save_png_bytes(&self) -> Result<(Vec<u8>, &'static str)> {
        self.benchmark_save_png_bytes_with_diagnostics()
            .map(|(png, backend, _)| (png, backend))
    }

    /// Render PNG bytes through the same backend-selection path used by `save()`
    /// and report internal diagnostics about which exact raster fast paths ran.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(hidden)]
    pub fn benchmark_save_png_bytes_with_diagnostics(
        &self,
    ) -> Result<(Vec<u8>, &'static str, RenderDiagnostics)> {
        self.save_png_bytes_with_backend()
    }

    /// Render through the save path and include its backend decision.
    #[cfg(not(target_arch = "wasm32"))]
    #[doc(hidden)]
    pub fn benchmark_save_png_bytes_with_backend_resolution(
        &self,
    ) -> Result<(Vec<u8>, &'static str, RenderDiagnostics, BackendResolution)> {
        let resolution = self.backend_resolution(BackendOperation::Png);
        self.save_png_bytes_with_backend()
            .map(|(png_bytes, backend, diagnostics)| (png_bytes, backend, diagnostics, resolution))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_png_bytes_with_backend(&self) -> Result<(Vec<u8>, &'static str, RenderDiagnostics)> {
        self.validate_before_frame_resolution()?;
        let (png_bytes, backend, diagnostics, frame) =
            self.save_png_bytes_with_backend_unacknowledged()?;
        frame.acknowledge_rendered(self);
        Ok((png_bytes, backend, diagnostics))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_png_bytes_with_backend_unacknowledged(
        &self,
    ) -> Result<(Vec<u8>, &'static str, RenderDiagnostics, ResolvedFrame<'_>)> {
        let frame = self.resolve_frame(0.0)?;
        let mode = self.public_png_render_mode_from_resolved(&frame.series);
        let render_plot = self.resolved_style_shell(&frame.style);
        let (renderer, diagnostics) =
            render_plot.render_renderer_with_frame_and_diagnostics(mode, &frame)?;
        let png_bytes = renderer.encode_png_bytes()?;
        let backend = diagnostics.actual_backend_name();
        debug_assert_eq!(
            backend,
            self.backend_resolution_for_series(BackendOperation::Png, &self.series_mgr.series)
                .actual_backend()
                .as_str(),
            "backend resolution must match the renderer that executed"
        );
        Ok((png_bytes, backend, diagnostics, frame))
    }

    /// Save the plot to a PNG file with custom dimensions
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_with_size<P: AsRef<Path>>(
        mut self,
        path: P,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self = self.set_output_pixels(width, height);
        self.save(path)
    }

    /// Export to SVG format
    ///
    /// Renders the plot to a vector SVG file with full visual fidelity.
    /// Includes axes, grid, tick marks, labels, legend, and all data series.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.validate_before_frame_resolution()?;
        let frame = self.resolve_frame(0.0)?;
        let render_plot = self.resolved_style_shell(&frame.style);
        let svg_content = render_plot.render_to_svg_with_frame(&frame)?;
        crate::export::write_bytes_atomic(path, svg_content.as_bytes())?;
        frame.acknowledge_rendered(&self);
        Ok(())
    }

    fn render_svg_annotations(
        &self,
        svg: &mut crate::export::SvgRenderer,
        layer: AnnotationRenderLayer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        self.annotations
            .iter()
            .filter(|annotation| Self::annotation_render_layer(annotation) == layer)
            .try_for_each(|annotation| {
                self.render_svg_annotation(svg, annotation, plot_area, x_min, x_max, y_min, y_max)
            })
    }

    fn render_svg_annotation(
        &self,
        svg: &mut crate::export::SvgRenderer,
        annotation: &Annotation,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        match annotation {
            Annotation::Text { x, y, text, style } => {
                let (px, py) =
                    self.svg_annotation_point(*x, *y, plot_area, x_min, x_max, y_min, y_max);
                svg.draw_styled_text(text, px, py, &self.display.config.typography.family, style)?;
            }
            Annotation::Arrow {
                x1,
                y1,
                x2,
                y2,
                style,
            } => {
                let (px1, py1) =
                    self.svg_annotation_point(*x1, *y1, plot_area, x_min, x_max, y_min, y_max);
                let (px2, py2) =
                    self.svg_annotation_point(*x2, *y2, plot_area, x_min, x_max, y_min, y_max);
                let width = self.render_scale().points_to_pixels(style.line_width);
                svg.draw_line(
                    px1,
                    py1,
                    px2,
                    py2,
                    style.color,
                    width,
                    style.line_style.clone(),
                );

                if !matches!(style.head_style, crate::core::ArrowHead::None) {
                    self.draw_svg_arrow_head(svg, (px2, py2), (px1, py1), style);
                }
                if !matches!(style.tail_style, crate::core::ArrowHead::None) {
                    self.draw_svg_arrow_head(svg, (px1, py1), (px2, py2), style);
                }
            }
            Annotation::HLine {
                y,
                style,
                color,
                width,
            } => {
                let py = Self::scaled_y_pixel(*y, y_min, y_max, plot_area, &self.layout.y_scale);
                let width = self.render_scale().points_to_pixels(*width);
                svg.draw_line(
                    plot_area.left(),
                    py,
                    plot_area.right(),
                    py,
                    *color,
                    width,
                    style.clone(),
                );
            }
            Annotation::VLine {
                x,
                style,
                color,
                width,
            } => {
                let px = Self::scaled_x_pixel(*x, x_min, x_max, plot_area, &self.layout.x_scale);
                let width = self.render_scale().points_to_pixels(*width);
                svg.draw_line(
                    px,
                    plot_area.top(),
                    px,
                    plot_area.bottom(),
                    *color,
                    width,
                    style.clone(),
                );
            }
            Annotation::Rectangle {
                x,
                y,
                width,
                height,
                style,
            } => {
                let (px1, py1) = self.svg_annotation_point(
                    *x,
                    *y + *height,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                let (px2, py2) = self.svg_annotation_point(
                    *x + *width,
                    *y,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                self.draw_svg_styled_rect(
                    svg,
                    px1.min(px2),
                    py1.min(py2),
                    (px2 - px1).abs(),
                    (py2 - py1).abs(),
                    style,
                );
            }
            Annotation::FillBetween {
                x,
                y1,
                y2,
                style,
                where_positive,
            } => {
                let len = x.len().min(y1.len()).min(y2.len());
                if len >= 2 && x.len() == y1.len() && x.len() == y2.len() {
                    let mut points: Vec<(f32, f32)> = (0..len)
                        .map(|index| {
                            let y = if index > 0 && *where_positive && y1[index] < y2[index] {
                                y2[index]
                            } else {
                                y1[index]
                            };
                            self.svg_annotation_point(
                                x[index], y, plot_area, x_min, x_max, y_min, y_max,
                            )
                        })
                        .collect();

                    points.extend(
                        (0..len)
                            .rev()
                            .filter(|&index| !*where_positive || y1[index] >= y2[index])
                            .map(|index| {
                                self.svg_annotation_point(
                                    x[index], y2[index], plot_area, x_min, x_max, y_min, y_max,
                                )
                            }),
                    );

                    if points.len() >= 3 {
                        svg.draw_filled_polygon(&points, style.color.with_alpha(style.alpha));
                        if let Some(edge_color) = style.edge_color {
                            let width = self.render_scale().points_to_pixels(style.edge_width);
                            svg.draw_polygon_outline(&points, edge_color, width);
                        }
                    }
                }
            }
            Annotation::HSpan {
                x_min: span_min,
                x_max: span_max,
                style,
            } => {
                let px1 =
                    Self::scaled_x_pixel(*span_min, x_min, x_max, plot_area, &self.layout.x_scale);
                let px2 =
                    Self::scaled_x_pixel(*span_max, x_min, x_max, plot_area, &self.layout.x_scale);
                self.draw_svg_styled_rect(
                    svg,
                    px1.min(px2),
                    plot_area.top(),
                    (px2 - px1).abs(),
                    plot_area.height(),
                    style,
                );
            }
            Annotation::VSpan {
                y_min: span_min,
                y_max: span_max,
                style,
            } => {
                let py1 =
                    Self::scaled_y_pixel(*span_min, y_min, y_max, plot_area, &self.layout.y_scale);
                let py2 =
                    Self::scaled_y_pixel(*span_max, y_min, y_max, plot_area, &self.layout.y_scale);
                self.draw_svg_styled_rect(
                    svg,
                    plot_area.left(),
                    py1.min(py2),
                    plot_area.width(),
                    (py2 - py1).abs(),
                    style,
                );
            }
        }

        Ok(())
    }

    fn svg_annotation_point(
        &self,
        x: f64,
        y: f64,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> (f32, f32) {
        crate::render::skia::map_data_to_pixels_scaled(
            x,
            y,
            x_min,
            x_max,
            y_min,
            y_max,
            plot_area,
            &self.layout.x_scale,
            &self.layout.y_scale,
        )
    }

    fn draw_svg_styled_rect(
        &self,
        svg: &mut crate::export::SvgRenderer,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &ShapeStyle,
    ) {
        if let Some(fill_color) = style.fill_color {
            svg.draw_rectangle(
                x,
                y,
                width,
                height,
                fill_color.with_alpha(style.fill_alpha),
                true,
            );
        }

        if let Some(edge_color) = style.edge_color {
            let edge_width = self.render_scale().points_to_pixels(style.edge_width);
            svg.draw_line(
                x,
                y,
                x + width,
                y,
                edge_color,
                edge_width,
                style.edge_style.clone(),
            );
            svg.draw_line(
                x + width,
                y,
                x + width,
                y + height,
                edge_color,
                edge_width,
                style.edge_style.clone(),
            );
            svg.draw_line(
                x + width,
                y + height,
                x,
                y + height,
                edge_color,
                edge_width,
                style.edge_style.clone(),
            );
            svg.draw_line(
                x,
                y + height,
                x,
                y,
                edge_color,
                edge_width,
                style.edge_style.clone(),
            );
        }
    }

    fn draw_svg_arrow_head(
        &self,
        svg: &mut crate::export::SvgRenderer,
        tip: (f32, f32),
        from: (f32, f32),
        style: &ArrowStyle,
    ) {
        let dx = tip.0 - from.0;
        let dy = tip.1 - from.1;
        let len = (dx * dx + dy * dy).sqrt();
        // `NaN < 0.001` is false, so an unplaceable endpoint used to fall
        // straight through this guard and build a triangle out of `NaN`
        // vertices. An arrow is a mark, not aggregate geometry: both backends
        // skip it, rather than one refusing the document and the other quietly
        // dropping the head.
        if !len.is_finite() || len < 0.001 {
            return;
        }

        let ux = dx / len;
        let uy = dy / len;
        let perpendicular = (-uy, ux);
        let head_length = self.render_scale().points_to_pixels(style.head_length);
        let head_width = self.render_scale().points_to_pixels(style.head_width);
        let base = (tip.0 - ux * head_length, tip.1 - uy * head_length);
        let points = [
            tip,
            (
                base.0 + perpendicular.0 * head_width / 2.0,
                base.1 + perpendicular.1 * head_width / 2.0,
            ),
            (
                base.0 - perpendicular.0 * head_width / 2.0,
                base.1 - perpendicular.1 * head_width / 2.0,
            ),
        ];

        svg.draw_filled_polygon(&points, style.color);
    }

    /// Render the plot to an SVG string
    ///
    /// Returns the complete SVG content as a string. This can be saved to a file
    /// or converted to other formats like PDF.
    pub fn render_to_svg(&self) -> Result<String> {
        self.validate_before_frame_resolution()?;
        let frame = self.resolve_frame(0.0)?;
        let render_plot = self.resolved_style_shell(&frame.style);
        let result = render_plot.render_to_svg_with_frame(&frame);
        if result.is_ok() {
            frame.acknowledge_rendered(self);
        }
        result
    }

    fn render_to_svg_with_frame(&self, frame: &ResolvedFrame<'_>) -> Result<String> {
        use crate::axes::TickLayout;
        use crate::export::SvgRenderer;

        self.validate_runtime_environment()?;
        if !frame.series.is_empty() {
            self.validate_resolved_series(&frame.series)?;
        }

        let (width_px, height_px) = self.config_canvas_size();
        let width = width_px as f32;
        let height = height_px as f32;

        let mut svg = SvgRenderer::with_font_family(
            width,
            height,
            self.display.config.typography.family.clone(),
        );
        let render_scale = self.render_scale();
        svg.set_render_scale(render_scale);
        svg.set_text_engine_mode(self.display.text_engine);

        let (x_min, x_max, y_min, y_max) =
            self.effective_main_panel_bounds_from_resolved(&self.series_mgr.series, &frame.series)?;
        self.validate_axis_scale_ranges_for_render(
            &self.series_mgr.series,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;

        // Use the same content-driven layout path as PNG rendering.
        let content = self.create_plot_content_from_resolved_text(y_min, y_max, frame);
        let mut measurement_renderer = SkiaRenderer::with_font_family(
            width_px,
            height_px,
            self.display.theme.clone(),
            self.display.config.typography.family.clone(),
        )?;
        measurement_renderer.set_text_engine_mode(self.display.text_engine);
        measurement_renderer.set_render_scale(render_scale);
        let x_major_measurement_layout = TickLayout::compute(
            x_min,
            x_max,
            0.0,
            1.0,
            &self.layout.x_scale,
            self.layout.tick_config.major_ticks_x,
        );
        let y_major_measurement_layout = TickLayout::compute_y_axis(
            y_min,
            y_max,
            0.0,
            1.0,
            &self.layout.y_scale,
            self.layout.tick_config.major_ticks_y,
        );
        let measured_dimensions = self.measure_layout_text_with_ticks(
            &measurement_renderer,
            &content,
            self.display.config.figure.dpi,
            &x_major_measurement_layout.labels,
            &y_major_measurement_layout.labels,
        )?;
        let layout = self.compute_layout_from_measurements(
            (width_px, height_px),
            &content,
            self.display.config.figure.dpi,
            measured_dimensions.as_ref(),
        );

        // The same harvest the raster backend runs, so PNG and SVG cannot label
        // the same figure differently: bars, box plots, violins and boxen plots
        // all report the unit-wide slots they occupy.
        let category_axis = super::series_internal::CategoryAxis::harvest(&self.series_mgr.series);
        let (category_labels, category_positions): (&[String], &[f64]) = match &category_axis {
            Some(axis) => (&axis.labels, &axis.positions),
            None => (&[], &[]),
        };
        let is_categorical = category_axis.is_some();

        // ...and the same label row, measured with the same renderer, so the
        // bottom margin is reserved from the row both backends will draw.
        let (layout, x_tick_label_plan) = self.resolve_x_tick_label_row(
            &measurement_renderer,
            (width_px, height_px),
            &content,
            self.display.config.figure.dpi,
            measured_dimensions.as_ref(),
            layout,
            category_labels,
            category_positions,
            x_min,
            x_max,
        )?;

        let plot_left = layout.plot_area.left;
        let plot_right = layout.plot_area.right;
        let plot_top = layout.plot_area.top;
        let plot_bottom = layout.plot_area.bottom;
        let plot_width = layout.plot_area.width();
        let plot_height = layout.plot_area.height();

        // Create plot area rectangle for coordinate mapping
        let plot_area = tiny_skia::Rect::from_ltrb(plot_left, plot_top, plot_right, plot_bottom)
            .ok_or(PlottingError::InvalidData {
                message: "Invalid plot area from layout".to_string(),
                position: None,
            })?;

        // Draw background
        svg.draw_rectangle(0.0, 0.0, width, height, self.display.theme.background, true);

        // Compute Y-axis tick layout (fix parameter order: pixel_top then pixel_bottom)
        let y_tick_layout = TickLayout::compute_y_axis(
            y_min,
            y_max,
            plot_top,
            plot_bottom,
            &self.layout.y_scale,
            self.layout.tick_config.major_ticks_y,
        );
        let x_tick_layout = if !is_categorical {
            Some(TickLayout::compute(
                x_min,
                x_max,
                plot_left,
                plot_right,
                &self.layout.x_scale,
                self.layout.tick_config.major_ticks_x,
            ))
        } else {
            None
        };
        let y_minor_ticks = Self::minor_tick_values_for_scale(
            &y_tick_layout.data_positions,
            y_min,
            y_max,
            &self.layout.y_scale,
            self.layout.tick_config.minor_ticks_y,
        );
        let y_minor_tick_pixels: Vec<f32> = y_minor_ticks
            .iter()
            .map(|&tick| Self::scaled_y_pixel(tick, y_min, y_max, plot_area, &self.layout.y_scale))
            .collect();
        let x_minor_tick_pixels: Vec<f32> = x_tick_layout
            .as_ref()
            .map(|layout| {
                Self::minor_tick_values_for_scale(
                    &layout.data_positions,
                    x_min,
                    x_max,
                    &self.layout.x_scale,
                    self.layout.tick_config.minor_ticks_x,
                )
                .iter()
                .map(|&tick| {
                    Self::scaled_x_pixel(tick, x_min, x_max, plot_area, &self.layout.x_scale)
                })
                .collect()
            })
            .unwrap_or_default();

        // Draw grid lines (only horizontal for bar charts) - using unified GridStyle
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        let draw_axes = Self::needs_cartesian_axes_for_series(&self.series_mgr.series);
        if let Some(panel) = self.themed_panel_background()
            && draw_axes
        {
            svg.draw_rectangle(plot_left, plot_top, plot_width, plot_height, panel, true);
        }
        if self.layout.grid_style.visible && draw_axes {
            // Bar charts only get horizontal grid lines.
            let (x_major_pixels, x_minor_pixels): (&[f32], &[f32]) = if is_categorical {
                (&[], &[])
            } else {
                let x_tick_layout = x_tick_layout.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "missing x tick layout for non-categorical SVG grid".to_string(),
                    )
                })?;
                (&x_tick_layout.pixel_positions, &x_minor_tick_pixels)
            };
            let layers = Self::grid_layers(
                &self.layout.grid_style,
                &self.layout.tick_config.grid_mode,
                x_major_pixels,
                &y_tick_layout.pixel_positions,
                x_minor_pixels,
                &y_minor_tick_pixels,
                |points| self.line_width_px(points),
            );
            for layer in &layers {
                svg.draw_grid(
                    &layer.x_pixels,
                    &layer.y_pixels,
                    plot_left,
                    plot_right,
                    plot_top,
                    plot_bottom,
                    layer.color,
                    self.layout.grid_style.line_style.clone(),
                    layer.width_px,
                );
            }
        }

        let category_x_tick_positions = SkiaRenderer::categorical_label_centers(
            &layout.plot_area,
            category_positions,
            x_min,
            x_max,
        );

        // The frame and its ticks are emitted after the series, so data ink can
        // never eat the border it is measured against. The grid, emitted before
        // the series, stays underneath. Declared here, next to the grid, so the
        // two halves of the z-order read together.
        let themed_sides = self.themed_tick_sides(self.layout.tick_config.sides);
        let themed_spines = self.themed_spines();
        let draw_frame = |svg: &mut crate::export::SvgRenderer| -> Result<()> {
            if !draw_axes {
                return Ok(());
            }
            let (axis_width, major_tick_size, minor_tick_size, major_tick_width, minor_tick_width) =
                self.axis_tick_metrics_px();
            let (x_major, y_major, x_minor, y_minor, sides): (
                &[f32],
                &[f32],
                &[f32],
                &[f32],
                &TickSides,
            ) = if !self.layout.tick_config.enabled {
                (&[], &[], &[], &[], &TickSides::none())
            } else if is_categorical {
                (
                    &category_x_tick_positions,
                    &y_tick_layout.pixel_positions,
                    &[],
                    &y_minor_tick_pixels,
                    &themed_sides,
                )
            } else {
                let x_tick_layout = x_tick_layout.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "missing x tick layout for non-categorical SVG axes".to_string(),
                    )
                })?;
                (
                    &x_tick_layout.pixel_positions,
                    &y_tick_layout.pixel_positions,
                    &x_minor_tick_pixels,
                    &y_minor_tick_pixels,
                    &themed_sides,
                )
            };
            svg.draw_axes_with_minor_ticks_styled(
                plot_left,
                plot_right,
                plot_top,
                plot_bottom,
                x_major,
                y_major,
                x_minor,
                y_minor,
                &self.layout.tick_config.direction,
                sides,
                &themed_spines,
                self.display.theme.foreground,
                axis_width,
                major_tick_size,
                minor_tick_size,
                major_tick_width,
                minor_tick_width,
            );
            Ok(())
        };

        let tick_size_px = pt_to_px(
            self.display.config.typography.tick_size(),
            self.display.config.figure.dpi,
        );

        // Draw tick labels. The frame and its ticks follow the series; see
        // `draw_frame`.
        if draw_axes {
            if is_categorical {
                // Categorical axis: ticks at the slot centres, labels under them.
                if self.layout.tick_config.enabled {
                    // Draw Y-axis tick labels
                    svg.draw_tick_labels(
                        &[],
                        &[],
                        &y_tick_layout.pixel_positions,
                        &y_tick_layout.labels,
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        layout.xtick_baseline_y,
                        layout.ytick_right_x,
                        self.display.theme.foreground,
                        tick_size_px,
                    )?;

                    // Draw the category labels on the x axis through the one row
                    // drawer both backends use, following the one plan both
                    // backends resolved.
                    draw_x_tick_label_row(
                        &mut svg,
                        category_labels,
                        &category_x_tick_positions,
                        layout.xtick_baseline_y,
                        tick_size_px,
                        self.display.theme.foreground,
                        x_tick_label_plan,
                    )?;
                }
            } else {
                // Normal chart: draw axes with numeric labels
                let x_tick_layout = x_tick_layout.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "missing x tick layout for non-categorical SVG axes".to_string(),
                    )
                })?;
                if self.layout.tick_config.enabled {
                    svg.draw_tick_labels(
                        &x_tick_layout.pixel_positions,
                        &x_tick_layout.labels,
                        &y_tick_layout.pixel_positions,
                        &y_tick_layout.labels,
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        layout.xtick_baseline_y,
                        layout.ytick_right_x,
                        self.display.theme.foreground,
                        tick_size_px,
                    )?;
                }
            }
        }

        // Create clip path for data
        let clip_id = svg.add_clip_rect(plot_left, plot_top, plot_width, plot_height);
        svg.start_clip_group(&clip_id);
        self.render_svg_annotations(
            &mut svg,
            AnnotationRenderLayer::Underlay,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;

        // Collect legend items, including grouped-series collapse behavior.
        let legend_items = self.collect_legend_items();
        let render_scale = self.render_scale();

        let inset_rects =
            self.inset_rects_for_series(&self.series_mgr.series, plot_area, render_scale)?;

        // Render each series
        for (idx, (series, resolved)) in
            self.series_mgr.series.iter().zip(&frame.series).enumerate()
        {
            let default_color = series
                .props
                .color
                .value_or(self.display.theme.get_color(idx));
            let inset_rect = inset_rects[idx];
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rect {
                (inset_rect, self.inset_bounds_from_resolved(resolved)?)
            } else {
                (plot_area, (x_min, x_max, y_min, y_max))
            };

            if let Some(inset_rect) = inset_rect {
                let inset_clip_id = svg.add_clip_rect(
                    inset_rect.x(),
                    inset_rect.y(),
                    inset_rect.width(),
                    inset_rect.height(),
                );
                svg.start_clip_group(&inset_clip_id);
                self.render_series_svg(
                    &mut svg,
                    series,
                    resolved,
                    default_color,
                    series_area,
                    series_bounds.0,
                    series_bounds.1,
                    series_bounds.2,
                    series_bounds.3,
                )?;
                svg.end_group();
            } else {
                self.render_series_svg(
                    &mut svg,
                    series,
                    resolved,
                    default_color,
                    series_area,
                    series_bounds.0,
                    series_bounds.1,
                    series_bounds.2,
                    series_bounds.3,
                )?;
            }
        }

        self.render_svg_annotations(
            &mut svg,
            AnnotationRenderLayer::Overlay,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;
        svg.end_group(); // End clip group

        draw_frame(&mut svg)?;

        // Colorbars sit beside the plot area, so they belong outside its clip.
        self.render_svg_colorbars(&mut svg, plot_area)?;

        // Draw title/xlabel/ylabel using layout-computed positions.
        if let Some(ref pos) = layout.title_pos
            && let Some(title) = frame.title.as_deref()
        {
            svg.draw_text_centered_with_weight(
                title,
                pos.x,
                pos.y,
                pos.size,
                self.display.theme.foreground,
                self.display.config.typography.title_weight,
            )?;
        }
        if let Some(ref pos) = layout.xlabel_pos
            && let Some(xlabel) = frame.xlabel.as_deref()
        {
            svg.draw_text_centered(
                xlabel,
                pos.x,
                pos.y,
                pos.size,
                self.display.theme.foreground,
            )?;
        }
        if let Some(ref pos) = layout.ylabel_pos
            && let Some(ylabel) = frame.ylabel.as_deref()
        {
            svg.draw_text_rotated(
                ylabel,
                pos.x,
                pos.y,
                pos.size,
                self.display.theme.foreground,
                -90.0,
            )?;
        }

        // Draw legend if we have labeled series and legend is enabled
        if !legend_items.is_empty() && frame.style.legend.enabled {
            let plot_bounds = (plot_left, plot_top, plot_right, plot_bottom);
            let occupancy = legend_occupancy(
                &frame.series,
                plot_area,
                (x_min, x_max, y_min, y_max),
                &self.layout.x_scale,
                &self.layout.y_scale,
            );
            svg.draw_legend_full_resolved(
                &legend_items,
                &frame.style.legend,
                plot_bounds,
                Some(&occupancy),
                layout.legend_rect.as_ref().map(|rect| rect.bounds()),
            )?;
        }

        // The renderer drops a shape whose dimensions are non-finite rather
        // than printing `width="NaN"`, and latches why. `SvgRenderer::save`
        // checks that latch, but this path hands the string back to the caller
        // (and to `export_svg`'s own atomic write), so it has to check too —
        // otherwise a refused element is silently missing from the document.
        svg.check_geometry()?;
        Ok(svg.to_svg_string())
    }

    /// Export to PDF format (requires `pdf` feature)
    ///
    /// Creates a vector-based PDF file with the plot. PDF export produces
    /// publication-quality output with text rendered as vectors.
    ///
    /// # Example
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
    ///     .title("My Plot")
    ///     .save_pdf("plot.pdf")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    pub fn save_pdf<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.save_pdf_with_size(path, None)
    }

    /// Export to PDF format with custom page size in millimeters
    ///
    /// Uses SVG → PDF pipeline for high-quality vector output with full visual fidelity.
    /// This includes grid lines, tick marks, rotated labels, and legends.
    ///
    /// # Arguments
    /// * `path` - Output file path
    /// * `size` - Optional (width_mm, height_mm). If None, uses 160x120mm.
    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    pub fn save_pdf_with_size<P: AsRef<Path>>(
        mut self,
        path: P,
        size: Option<(f64, f64)>,
    ) -> Result<()> {
        use crate::export::svg_to_pdf::page_sizes;

        self.validate_before_frame_resolution()?;

        // Calculate pixel dimensions from mm (at 96 DPI)
        let (width_mm, height_mm) = size.unwrap_or(page_sizes::PLOT_DEFAULT);
        let width_px = page_sizes::mm_to_px(width_mm) as u32;
        let height_px = page_sizes::mm_to_px(height_mm) as u32;

        self = self.set_output_pixels(width_px, height_px);

        let frame = self.resolve_frame(0.0)?;
        let render_plot = self.resolved_style_shell(&frame.style);
        let svg_content = render_plot.render_to_svg_with_frame(&frame)?;
        let pdf_data = crate::export::svg_to_pdf(&svg_content)?;
        crate::export::write_bytes_atomic(path, &pdf_data)?;
        frame.acknowledge_rendered(&self);
        Ok(())
    }

    // ==========================================================================
    // Animation Methods (feature-gated)
    // ==========================================================================

    /// Render a single frame for animation capture
    ///
    /// Returns the raw RGB pixel data suitable for encoding.
    /// This is used internally by the animation recording system.
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    ///
    /// # Returns
    ///
    /// RGB pixel data as a `Vec<u8>` (width * height * 3 bytes)
    #[cfg(all(feature = "animation", not(target_arch = "wasm32")))]
    pub fn render_frame(&self, width: u32, height: u32) -> Result<Vec<u8>> {
        // Create a sized version of the plot
        let sized_plot = self.clone().set_output_pixels(width, height);

        // Render to RGBA image
        let image = sized_plot.render()?;
        let rgba_data = &image.pixels;

        // Convert RGBA to RGB
        let pixels = (width * height) as usize;
        let mut rgb_data = vec![0u8; pixels * 3];

        for i in 0..pixels {
            rgb_data[i * 3] = rgba_data[i * 4]; // R
            rgb_data[i * 3 + 1] = rgba_data[i * 4 + 1]; // G
            rgb_data[i * 3 + 2] = rgba_data[i * 4 + 2]; // B
        }

        Ok(rgb_data)
    }
}

/// One validity verdict for every backend.
///
/// These cover three regressions that arrived together, all from the same
/// cause: a sample a log axis cannot place started projecting to `NaN` instead
/// of being clamped, and the pre-render range check — which reads *projected*
/// bounds — stopped seeing anything wrong, because a bounds accumulator that
/// skips unrepresentable samples can never produce an invalid range.
///
/// 1. The backends disagreed. `.xscale(Log).histogram(&[0.0, ..])` was `Err`
///    from `save()` and `Ok` from `export_svg()`.
/// 2. The message regressed from one that named the axis and the fix to
///    "Rendering error: Invalid rectangle dimensions", which names neither.
/// 3. A log-y box plot over data containing a zero returned `Ok` and drew a
///    figure with no box in it — two orphan strokes and a flier.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod log_axis_validity_tests {
    use super::*;
    use crate::axes::scale::LOG_SCALE_REQUIRES_POSITIVE;
    use tempfile::tempdir;

    /// Push one figure through every public output path and require the same
    /// refusal from all of them.
    ///
    /// `$build` is re-evaluated per backend because each terminal method
    /// consumes the builder — which is also what makes this a genuine parity
    /// test rather than four looks at one cached result.
    macro_rules! assert_every_backend_refuses {
        ($axis:literal, $build:expr) => {{
            let dir = tempdir().expect("a temporary directory");

            let raster = $build
                .save(dir.path().join("figure.png"))
                .expect_err("save() must refuse this figure");
            let vector = $build
                .export_svg(dir.path().join("figure.svg"))
                .expect_err("export_svg() must refuse this figure");
            let svg_string = match $build.render_to_svg() {
                Ok(_) => panic!("render_to_svg() must refuse this figure"),
                Err(error) => error,
            };
            let image = match $build.render() {
                Ok(_) => panic!("render() must refuse this figure"),
                Err(error) => error,
            };
            let png_bytes = match $build.render_png_bytes() {
                Ok(_) => panic!("render_png_bytes() must refuse this figure"),
                Err(error) => error,
            };
            // The prepared runtime is a second front door to the raster
            // backend: it resolves its own frame and caches it, so it has to
            // pass the same gate rather than inherit it.
            let prepared = match $build.into_plot().prepare().render_png_bytes() {
                Ok(_) => panic!("PreparedPlot::render_png_bytes() must refuse this figure"),
                Err(error) => error,
            };

            let message = raster.to_string();
            assert_eq!(
                message,
                vector.to_string(),
                "save() and export_svg() must fail identically; a check that lives inside \
                 one backend is a check the backends will disagree about"
            );
            assert_eq!(message, svg_string.to_string(), "render_to_svg() disagreed");
            assert_eq!(message, image.to_string(), "render() disagreed");
            assert_eq!(
                message,
                png_bytes.to_string(),
                "render_png_bytes() disagreed"
            );
            assert_eq!(
                message,
                prepared.to_string(),
                "PreparedPlot::render_png_bytes() disagreed"
            );

            // The PDF pipeline renders through the SVG backend but writes the
            // file itself, so it is its own entry point and needs its own gate.
            #[cfg(feature = "pdf")]
            {
                let pdf = $build
                    .save_pdf(dir.path().join("figure.pdf"))
                    .expect_err("save_pdf() must refuse this figure");
                assert_eq!(message, pdf.to_string(), "save_pdf() disagreed");
            }

            assert!(
                matches!(raster, PlottingError::InvalidInput(_)),
                "an unplottable figure is bad input, not a rendering failure: {raster:?}"
            );
            assert!(
                message.contains(LOG_SCALE_REQUIRES_POSITIVE),
                "the refusal must keep the wording that names the fix: {message}"
            );
            assert!(
                message.contains(concat!("Invalid ", $axis, "-axis range")),
                concat!("the refusal must name the ", $axis, " axis: {}"),
                message
            );
            assert!(
                message.contains("SymLog"),
                "the refusal must point at the scale that can show these values: {message}"
            );
            assert!(
                !message.contains("Invalid rectangle dimensions"),
                "geometry-level fallout must never be what the user is shown: {message}"
            );

            message
        }};
    }

    /// Regression: `.save()` returned `Err` and `.export_svg()` returned `Ok`
    /// on the same figure.
    #[test]
    fn test_every_backend_refuses_a_histogram_a_log_x_axis_cannot_bin() {
        let data = vec![0.0, 1.0, 10.0, 100.0];
        let message = assert_every_backend_refuses!(
            "x",
            Plot::new()
                .size_px(240, 180)
                .histogram(&data)
                .xscale(AxisScale::Log)
        );
        assert!(
            message.contains("histogram"),
            "the refusal must say which series provoked it: {message}"
        );
    }

    #[test]
    fn test_every_backend_refuses_a_bar_a_log_value_axis_cannot_size() {
        let values = vec![1.0, 0.0, 100.0];
        let message = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .bar(&["a", "b", "c"], &values)
                .yscale(AxisScale::Log)
        );
        assert!(message.contains("bar"), "{message}");
    }

    /// Regression: this returned `Ok` and rendered a figure with no box —
    /// silently wrong output, the worst of the failure modes.
    #[test]
    fn test_every_backend_refuses_a_box_plot_a_log_value_axis_cannot_place() {
        let data = vec![-1.0, 0.0, 1.0, 10.0, 100.0];
        let message = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .boxplot(&data)
                .yscale(AxisScale::Log)
        );
        assert!(message.contains("box plot"), "{message}");
    }

    /// A violin's outline is a density estimate over every sample, and a
    /// boxen's bands are letter values: same class of geometry, same refusal.
    #[test]
    fn test_every_backend_refuses_distribution_bodies_a_log_axis_cannot_place() {
        // Symmetric about zero, so the sample set carries negatives (the
        // violin's density is fitted to them) and the median is exactly zero
        // (a letter-value band edge the boxen has to draw).
        let data: Vec<f64> = (0..=60).map(|index| index as f64 - 30.0).collect();

        let violin = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .violin(&data)
                .yscale(AxisScale::Log)
        );
        assert!(violin.contains("violin"), "{violin}");

        let boxen = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .boxen(&data)
                .yscale(AxisScale::Log)
        );
        assert!(boxen.contains("boxen"), "{boxen}");
    }

    /// The counterpart requirement: a point series is a set of independent
    /// samples, so one the axis cannot place is dropped and the line breaks at
    /// the gap. Refusing here would cost the user the other 99% of their data.
    #[test]
    fn test_a_line_series_keeps_rendering_around_a_sample_the_log_axis_drops() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 0.0, 40.0, 50.0];
        let dir = tempdir().expect("a temporary directory");

        Plot::new()
            .size_px(240, 180)
            .line(&x, &y)
            .yscale(AxisScale::Log)
            .save(dir.path().join("line.png"))
            .expect("a line series must still render around an unplaceable sample");

        Plot::new()
            .size_px(240, 180)
            .line(&x, &y)
            .yscale(AxisScale::Log)
            .export_svg(dir.path().join("line.svg"))
            .expect("both backends must agree that this figure is fine");

        let svg = Plot::new()
            .size_px(240, 180)
            .line(&x, &y)
            .yscale(AxisScale::Log)
            .render_to_svg()
            .expect("a line series must still render around an unplaceable sample");

        // "Dropped" means dropped: the sample must not reach an output
        // primitive, as `height="NaN"` and `y1="NaN"` are not valid SVG.
        assert!(
            !svg.contains("NaN"),
            "no NaN may reach the SVG output for a skipped sample"
        );
        assert!(
            svg.contains("<polyline") || svg.contains("<path"),
            "the rest of the series must still be drawn"
        );
    }

    /// The refusal is about the *log* axis, not about negative data. A scale
    /// that can place every finite number must never lose a figure to it.
    #[test]
    fn test_aggregate_geometry_is_never_refused_on_a_scale_that_can_place_it() {
        let values = vec![-5.0, 0.0, 5.0];
        for scale in [AxisScale::Linear, AxisScale::symlog(1.0)] {
            Plot::new()
                .size_px(240, 180)
                .bar(&["a", "b", "c"], &values)
                .yscale(scale)
                .render()
                .unwrap_or_else(|error| panic!("{scale:?} refused a figure it can draw: {error}"));
        }

        let data = vec![-1.0, 0.0, 1.0, 10.0, 100.0];
        Plot::new()
            .size_px(240, 180)
            .boxplot(&data)
            .render()
            .expect("the default linear axes must draw this box plot");
    }

    /// An explicit histogram range *is* the outer pair of bin edges. Samples
    /// outside it are never binned, so they cannot make a bar unplaceable and
    /// must not cost the user the figure.
    #[test]
    fn test_an_explicit_positive_histogram_range_survives_out_of_range_samples() {
        let data = vec![-50.0, 0.0, 1.0, 10.0, 100.0];
        let config = crate::plots::histogram::HistogramConfig::new()
            .range(1.0, 100.0)
            .bins(4);

        Plot::new()
            .size_px(240, 180)
            .histogram_with(&data, config)
            .xscale(AxisScale::Log)
            .render()
            .expect("only the bin edges have to be placeable on the axis");
    }

    /// The scan reads raw data against the configured scale, not the computed
    /// bounds — which is exactly why it catches what the range check cannot.
    ///
    /// Pinning the axis to an explicitly valid positive range satisfies
    /// `validate_axis_scale_ranges_for_render` outright, and the bounds
    /// accumulator would have satisfied it anyway by skipping the samples it
    /// cannot represent. The figure must still be refused.
    #[test]
    fn test_the_refusal_survives_an_explicitly_valid_axis_range() {
        let data = vec![-1.0, 0.0, 1.0, 10.0, 100.0];
        let message = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .boxplot(&data)
                .ylim(1.0, 100.0)
                .yscale(AxisScale::Log)
        );
        assert!(message.contains("box plot"), "{message}");
    }

    /// Annotation *shapes* are the same class of geometry as a bar, and they
    /// used to split the backends worse than a bar did.
    ///
    /// A rectangle, a span or a filled region whose corner a log axis cannot
    /// place projects to `NaN`. Left to the renderers, the SVG side refuses the
    /// element and latches the fault while tiny-skia answers the same input
    /// with `Rect::from_xywh`/`PathBuilder::finish` returning `None` and simply
    /// drawing nothing — one reports, one goes quiet. Refusing above the split
    /// is what makes them agree.
    #[test]
    fn test_every_backend_refuses_annotation_shapes_a_log_axis_cannot_place() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 30.0];

        let rectangle = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .line(&x, &y)
                .yscale(AxisScale::Log)
                // Bottom edge sits on zero, which a log y axis cannot place.
                .annotate(Annotation::rectangle(1.0, 0.0, 1.0, 5.0))
        );
        assert!(rectangle.contains("rectangle annotation"), "{rectangle}");

        let span = assert_every_backend_refuses!(
            "x",
            Plot::new()
                .size_px(240, 180)
                .line(&x, &y)
                .xscale(AxisScale::Log)
                .annotate(Annotation::hspan(0.0, 2.0))
        );
        assert!(span.contains("horizontal span annotation"), "{span}");

        let fill = assert_every_backend_refuses!(
            "y",
            Plot::new()
                .size_px(240, 180)
                .line(&x, &y)
                .yscale(AxisScale::Log)
                // The classic "fill down to zero" idiom, which a log axis has
                // no floor for: both backends drew nothing at all before.
                .fill_between(&x, &y, &[0.0, 0.0, 0.0])
        );
        assert!(fill.contains("filled region annotation"), "{fill}");
    }

    /// The counterpart: a mark is not a shape.
    ///
    /// Text, arrows and reference lines are drawn at a position rather than
    /// built out of one, so both backends skip an unplaceable one — the same
    /// answer they give an unplaceable scatter point. Refusing them would cost
    /// the user the whole figure over a label.
    #[test]
    fn test_annotation_marks_are_skipped_rather_than_refused_on_a_log_axis() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 30.0];
        let build = || {
            Plot::new()
                .size_px(240, 180)
                .line(&x, &y)
                .yscale(AxisScale::Log)
                .annotate(Annotation::text(2.0, 0.0, "unplaceable"))
                .annotate(Annotation::hline(0.0))
                .annotate(Annotation::vline(2.0))
                // The head is a triangle built from the endpoints, and `NaN`
                // slips through a bare `len < 0.001` test — so this covers the
                // arrow head as well as the shaft.
                .annotate(Annotation::arrow(1.0, 10.0, 3.0, 0.0))
        };
        let dir = tempdir().expect("a temporary directory");

        build()
            .save(dir.path().join("marks.png"))
            .expect("an unplaceable mark must not cost the figure");
        let svg = build()
            .render_to_svg()
            .expect("both backends must agree that this figure is fine");
        assert!(
            !svg.contains("NaN"),
            "a skipped mark must not reach the SVG output: {svg}"
        );
    }
}

/// The categorical x tick label row: measured, then given room, then drawn.
#[cfg(test)]
mod categorical_tick_label_tests {
    use super::*;
    use crate::core::plot::builder::IntoPlot;

    const REGIONS: [&str; 10] = [
        "North America",
        "South America",
        "Western Europe",
        "Eastern Europe",
        "Middle East",
        "North Africa",
        "Sub-Saharan Africa",
        "Central Asia",
        "South East Asia",
        "Australasia",
    ];

    const INITIALS: [&str; 10] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

    fn bar_plot(categories: &[&str]) -> Plot {
        let values: Vec<f64> = (0..categories.len())
            .map(|index| index as f64 + 1.0)
            .collect();
        Plot::new()
            .size_px(800, 600)
            .bar(categories, &values)
            .into_plot()
    }

    /// Resolve the layout exactly as `render()` does.
    fn resolved(plot: &Plot) -> (ResolvedLayout, XTickLabelPlan) {
        let (x_min, x_max, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("data bounds should be available");
        let content = plot.create_plot_content(y_min, y_max);
        let mut renderer = SkiaRenderer::new(
            plot.display.dimensions.0,
            plot.display.dimensions.1,
            plot.display.theme.clone(),
        )
        .expect("measurement renderer");
        renderer.set_render_scale(plot.render_scale());
        renderer.set_text_engine_mode(plot.display.text_engine);
        let axis = super::super::series_internal::CategoryAxis::harvest(&plot.series_mgr.series)
            .expect("a bar chart has a category axis");
        let (layout, _, _, plan) = plot
            .compute_layout_with_categorical_ticks(
                &renderer,
                plot.display.dimensions,
                &content,
                plot.display.config.figure.dpi,
                x_min,
                x_max,
                y_min,
                y_max,
                &axis.labels,
                &axis.positions,
            )
            .expect("categorical layout");
        (layout, plan)
    }

    /// Ten region names in one 800 px figure are rotated rather than drawn on
    /// top of each other, and every one of them is still drawn.
    #[test]
    fn ten_region_names_rotate_instead_of_colliding() {
        let (_, plan) = resolved(&bar_plot(&REGIONS));

        assert!(plan.rotated, "ten region names must not stay horizontal");
        assert_eq!(plan.stride, 1, "rotated names all fit, so none is dropped");
    }

    /// The room the rotated row needs is reserved *before* the plot area is
    /// computed — reserve it afterwards and the labels are clipped instead of
    /// overlapping, which is not an improvement.
    #[test]
    fn the_rotated_row_is_given_its_room_before_the_plot_area() {
        let (long, plan) = resolved(&bar_plot(&REGIONS));
        let (short, _) = resolved(&bar_plot(&INITIALS));

        assert!(
            plan.extent <= long.margins.bottom,
            "the reserved bottom margin has to hold the whole row: {} into {}",
            plan.extent,
            long.margins.bottom
        );
        assert!(
            long.margins.bottom > short.margins.bottom + 20.0,
            "the rotated row must widen the bottom margin: {} vs {}",
            long.margins.bottom,
            short.margins.bottom
        );
        assert!(
            long.plot_area.bottom < short.plot_area.bottom,
            "and that room has to come out of the plot area, not off the canvas"
        );
    }

    /// Short names are left alone: no rotation, no thinning, and the same
    /// margins the figure had before any of this existed.
    #[test]
    fn short_names_are_left_exactly_as_they_were() {
        let (layout, plan) = resolved(&bar_plot(&INITIALS));

        assert!(!plan.rotated);
        assert_eq!(plan.stride, 1);
        assert!(
            layout.margins.bottom < 90.0,
            "single letters should not inflate the bottom margin: {}",
            layout.margins.bottom
        );
    }

    /// A figure whose bottom margin is fixed cannot grow, so the row is thinned
    /// instead of rotated into a margin that was never going to appear. This is
    /// why "does it fit?" is a question for the layout and not for arithmetic
    /// restated next to the labels.
    #[test]
    fn a_fixed_margin_thins_the_row_instead_of_clipping_it() {
        let values: Vec<f64> = (0..REGIONS.len()).map(|index| index as f64 + 1.0).collect();
        let plot = Plot::new()
            .size_px(800, 600)
            .tight_layout_pad(4.0)
            .bar(&REGIONS, &values)
            .into_plot();

        let (_, plan) = resolved(&plot);

        assert!(!plan.rotated, "a 30 px margin cannot hold a rotated row");
        assert!(
            plan.stride > 1,
            "so the row has to be thinned to stay legible"
        );
    }

    /// Both backends draw the row the same way, because both take it from the
    /// same plan: the SVG twin used not to measure its category labels at all.
    #[test]
    fn the_svg_twin_rotates_the_same_row() {
        let svg = bar_plot(&REGIONS)
            .render_to_svg()
            .expect("a bar chart renders to SVG");

        assert!(
            svg.contains("rotate(-90.0)"),
            "the SVG row must be rotated too"
        );
        for region in REGIONS {
            assert!(
                svg.contains(region),
                "{region} must survive into the SVG output"
            );
        }
    }
}
