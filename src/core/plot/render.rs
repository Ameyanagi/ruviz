use super::*;

#[derive(Debug, Clone)]
struct ColorbarMeasurementSpec {
    vmin: f64,
    vmax: f64,
    value_scale: AxisScale,
    label: Option<String>,
    tick_font_size: f32,
    label_font_size: f32,
    show_log_subticks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationRenderLayer {
    Underlay,
    Overlay,
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

    fn annotation_render_layer(annotation: &Annotation) -> AnnotationRenderLayer {
        match annotation {
            Annotation::FillBetween { .. }
            | Annotation::HSpan { .. }
            | Annotation::VSpan { .. }
            | Annotation::Rectangle { .. } => AnnotationRenderLayer::Underlay,
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

    pub(super) fn validate_before_frame_resolution(&self) -> Result<()> {
        self.validate_runtime_environment()?;
        if let Some(error) = self.pending_ingestion_error() {
            return Err(error);
        }
        Ok(())
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

    pub(crate) fn grid_tick_pixels(
        major_pixels: &[f32],
        minor_pixels: &[f32],
        mode: &GridMode,
    ) -> Vec<f32> {
        match mode {
            GridMode::MajorOnly => major_pixels.to_vec(),
            GridMode::MinorOnly => minor_pixels.to_vec(),
            GridMode::Both => major_pixels
                .iter()
                .chain(minor_pixels.iter())
                .copied()
                .collect(),
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

        let bar_categories: Option<Cow<'_, [String]>> =
            self.series_mgr.series.iter().find_map(|s| {
                if let SeriesType::Bar { categories, .. } = &s.series_type {
                    Some(Cow::Borrowed(categories.as_slice()))
                } else {
                    None
                }
            });

        let violin_data: Vec<(String, f64)> = self
            .series_mgr
            .series
            .iter()
            .filter_map(|s| {
                if let SeriesType::Violin { data } = &s.series_type {
                    data.config
                        .category
                        .clone()
                        .map(|cat| (cat, data.config.x_position))
                } else {
                    None
                }
            })
            .collect();

        let (violin_categories, violin_positions): (Vec<String>, Vec<f64>) =
            violin_data.into_iter().unzip();

        let is_violin_categorical = !violin_categories.is_empty();

        let bar_categories = bar_categories.or(if is_violin_categorical {
            Some(Cow::Borrowed(violin_categories.as_slice()))
        } else {
            None
        });
        let content = self.create_plot_content_from_resolved_text(y_min, y_max, frame);
        let (layout, x_ticks, y_ticks) = self.compute_layout_with_configured_ticks(
            &renderer,
            (scaled_width, scaled_height),
            &content,
            dpi,
            x_min,
            x_max,
            y_min,
            y_max,
        )?;
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
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_width_px = self.line_width_px(self.layout.grid_style.line_width);
            let grid_x_pixels = Self::grid_tick_pixels(
                &x_tick_pixels,
                &x_minor_tick_pixels,
                &self.layout.tick_config.grid_mode,
            );
            let grid_y_pixels = Self::grid_tick_pixels(
                &y_tick_pixels,
                &y_minor_tick_pixels,
                &self.layout.tick_config.grid_mode,
            );
            renderer.draw_grid(
                &grid_x_pixels,
                &grid_y_pixels,
                plot_area,
                grid_color,
                self.layout.grid_style.line_style.clone(),
                grid_width_px,
            )?;
        }

        let categorical_x_tick_pixels = Self::categorical_x_tick_pixels(
            plot_area,
            x_min,
            x_max,
            bar_categories.as_ref().map(|categories| categories.len()),
            &violin_positions,
        );

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

        let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);

        if draw_axes && is_violin_categorical {
            renderer.draw_axis_labels_at_categorical_violin(
                &layout.plot_area,
                &violin_categories,
                &violin_positions,
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
            )?;
        } else if draw_axes {
            if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
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
                )?;
            } else {
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
        }

        if let Some(ref pos) = layout.title_pos {
            if let Some(title) = frame.title.as_deref() {
                renderer.draw_title_at_with_weight(
                    pos,
                    title,
                    self.display.theme.foreground,
                    self.display.config.typography.title_weight,
                )?;
            }
        }

        if let Some(ref pos) = layout.xlabel_pos {
            if let Some(xlabel) = frame.xlabel.as_deref() {
                renderer.draw_xlabel_at(pos, xlabel, self.display.theme.foreground)?;
            }
        }

        if let Some(ref pos) = layout.ylabel_pos {
            if let Some(ylabel) = frame.ylabel.as_deref() {
                renderer.draw_ylabel_at(pos, ylabel, self.display.theme.foreground)?;
            }
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

        let diagnostics = renderer.render_diagnostics().clone();
        Ok((renderer, diagnostics))
    }

    #[cfg(feature = "parallel")]
    fn try_render_parallel_image_with_diagnostics(
        &self,
        mode: RenderExecutionMode,
        frame: &ResolvedFrame<'_>,
    ) -> Result<Option<(Image, RenderDiagnostics)>> {
        if !mode.allows_parallel() {
            return Ok(None);
        }

        self.validate_runtime_environment()?;
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        if !frame.series.is_empty() {
            self.validate_resolved_series(&frame.series)?;
        }

        let total_points = Self::calculate_total_points_from_resolved(&frame.series);
        let series_count = frame.series.len();
        let has_mixed_coordinates = Self::has_mixed_coordinate_series(&self.series_mgr.series);
        let parallel_safe_for_markers =
            self.series_mgr
                .series
                .iter()
                .all(|series| match &series.series_type {
                    SeriesType::Line { .. } => {
                        series.marker_style.is_none()
                            && series.x_errors.is_none()
                            && series.y_errors.is_none()
                    }
                    SeriesType::Scatter { .. }
                    | SeriesType::Bar { .. }
                    | SeriesType::ErrorBars { .. }
                    | SeriesType::ErrorBarsXY { .. }
                    | SeriesType::Histogram { .. }
                    | SeriesType::BoxPlot { .. } => true,
                    SeriesType::Heatmap { .. }
                    | SeriesType::Kde { .. }
                    | SeriesType::Ecdf { .. }
                    | SeriesType::Violin { .. }
                    | SeriesType::Boxen { .. }
                    | SeriesType::Contour { .. }
                    | SeriesType::Pie { .. }
                    | SeriesType::Radar { .. }
                    | SeriesType::Polar { .. }
                    | SeriesType::Quiver { .. } => false,
                });

        if has_mixed_coordinates
            || !parallel_safe_for_markers
            || !self
                .render
                .parallel_renderer
                .should_use_parallel(series_count, total_points)
        {
            return Ok(None);
        }

        let image = self.render_with_parallel_resolved(frame)?;
        let diagnostics = RenderDiagnostics {
            render_mode: match mode {
                RenderExecutionMode::Reference => "reference",
                RenderExecutionMode::Optimized => "optimized",
            },
            used_parallel: true,
            ..RenderDiagnostics::default()
        };
        Ok(Some((image, diagnostics)))
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
        #[cfg(feature = "parallel")]
        if let Some(parallel_render) =
            style_shell.try_render_parallel_image_with_diagnostics(mode, &frame)?
        {
            frame.acknowledge_rendered(self);
            return Ok(parallel_render);
        }

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
            BackendType::Parallel => {
                #[cfg(not(feature = "parallel"))]
                {
                    self.backend_fallback(BackendFallbackReason::FeatureDisabled)
                }
                #[cfg(feature = "parallel")]
                {
                    self.backend_fallback(BackendFallbackReason::UnsupportedOperation)
                }
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
        renderer.draw_subplot(subplot_renderer.into_image(), 0, 0)?;
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

    fn colorbar_measurement_spec(&self) -> Option<ColorbarMeasurementSpec> {
        self.series_mgr
            .series
            .iter()
            .find_map(|series| match &series.series_type {
                SeriesType::Heatmap { data } if data.config.colorbar => {
                    Some(ColorbarMeasurementSpec {
                        vmin: data.vmin,
                        vmax: data.vmax,
                        value_scale: data.config.value_scale.clone(),
                        label: data.config.colorbar_label.clone(),
                        tick_font_size: data.config.colorbar_tick_font_size,
                        label_font_size: data.config.colorbar_label_font_size,
                        show_log_subticks: data.config.colorbar_log_subticks,
                    })
                }
                SeriesType::Contour { data } if data.config.colorbar => {
                    let (vmin, vmax) = if data.levels.is_empty() {
                        (0.0, 1.0)
                    } else {
                        (
                            data.levels.first().copied().unwrap_or(0.0),
                            data.levels.last().copied().unwrap_or(1.0),
                        )
                    };

                    Some(ColorbarMeasurementSpec {
                        vmin,
                        vmax,
                        value_scale: AxisScale::Linear,
                        label: data.config.colorbar_label.clone(),
                        tick_font_size: data.config.colorbar_tick_font_size,
                        label_font_size: data.config.colorbar_label_font_size,
                        show_log_subticks: false,
                    })
                }
                _ => None,
            })
    }

    fn measure_colorbar_right_margin(
        &self,
        renderer: &SkiaRenderer,
        spec: &ColorbarMeasurementSpec,
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
            measurements.legend = Some(Self::measure_legend(renderer, &legend, &legend_items)?);
        }

        Ok(Some(measurements))
    }

    fn measure_legend(
        renderer: &SkiaRenderer,
        legend: &Legend,
        items: &[LegendItem],
    ) -> Result<(f32, f32)> {
        let legend = legend.scaled_for_render(renderer.render_scale());
        let spacing = legend.spacing.to_pixels(legend.font_size);
        let mut max_label_width = 0.0_f32;
        for item in items {
            max_label_width = max_label_width.max(
                renderer
                    .measure_label_text(&item.label, legend.font_size)?
                    .0,
            );
        }
        let columns = legend.columns.max(1);
        let rows = items.len().div_ceil(columns);
        let item_width = spacing.handle_length + spacing.handle_text_pad + max_label_width;
        let content_width =
            item_width * columns as f32 + columns.saturating_sub(1) as f32 * spacing.column_spacing;
        let content_height =
            rows as f32 * legend.font_size + rows.saturating_sub(1) as f32 * spacing.label_spacing;
        let title_size = if let Some(title) = legend.title.as_deref() {
            let title_width = renderer.measure_label_text(title, legend.font_size)?.0;
            (title_width, legend.font_size + spacing.label_spacing)
        } else {
            (0.0, 0.0)
        };

        Ok((
            content_width.max(title_size.0) + spacing.border_pad * 2.0,
            content_height + title_size.1 + spacing.border_pad * 2.0,
        ))
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
        if !content.show_tick_labels {
            let measurements =
                self.measure_layout_text_with_ticks(renderer, content, dpi, &[], &[])?;
            let layout = self.compute_layout_from_measurements(
                canvas_size,
                content,
                dpi,
                measurements.as_ref(),
            );
            return Ok((layout, Vec::new(), Vec::new()));
        }

        let (x_ticks, y_ticks) = self.configured_major_ticks(x_min, x_max, y_min, y_max);
        let x_labels = crate::render::skia::format_tick_labels(&x_ticks);
        let y_labels = crate::render::skia::format_tick_labels(&y_ticks);
        let measurements =
            self.measure_layout_text_with_ticks(renderer, content, dpi, &x_labels, &y_labels)?;
        let layout =
            self.compute_layout_from_measurements(canvas_size, content, dpi, measurements.as_ref());

        Ok((layout, x_ticks, y_ticks))
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
                    // Add violin KDE points
                    for &y in &data.kde.x {
                        let x = 0.5; // Centered position
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
                            0.5,
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
        let image = Image {
            width: ds_image.width as u32,
            height: ds_image.height as u32,
            pixels: ds_image.pixels,
        };

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
        if len < 0.001 {
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

        // Check if we have a bar chart (need special X-axis handling)
        let bar_categories: Option<&Vec<String>> = self.series_mgr.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories)
            } else {
                None
            }
        });

        // Compute Y-axis tick layout (fix parameter order: pixel_top then pixel_bottom)
        let y_tick_layout = TickLayout::compute_y_axis(
            y_min,
            y_max,
            plot_top,
            plot_bottom,
            &self.layout.y_scale,
            self.layout.tick_config.major_ticks_y,
        );
        let x_tick_layout = if bar_categories.is_none() {
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
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_width_px = self.line_width_px(self.layout.grid_style.line_width);
            let grid_y_pixels = Self::grid_tick_pixels(
                &y_tick_layout.pixel_positions,
                &y_minor_tick_pixels,
                &self.layout.tick_config.grid_mode,
            );
            if bar_categories.is_some() {
                // For bar charts, only draw horizontal grid lines
                svg.draw_grid(
                    &[], // no vertical grid lines for bar charts
                    &grid_y_pixels,
                    plot_left,
                    plot_right,
                    plot_top,
                    plot_bottom,
                    grid_color,
                    self.layout.grid_style.line_style.clone(),
                    grid_width_px,
                );
            } else {
                // For other charts, compute X-axis ticks and draw full grid
                let x_tick_layout = x_tick_layout.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "missing x tick layout for non-categorical SVG grid".to_string(),
                    )
                })?;
                let grid_x_pixels = Self::grid_tick_pixels(
                    &x_tick_layout.pixel_positions,
                    &x_minor_tick_pixels,
                    &self.layout.tick_config.grid_mode,
                );
                svg.draw_grid(
                    &grid_x_pixels,
                    &grid_y_pixels,
                    plot_left,
                    plot_right,
                    plot_top,
                    plot_bottom,
                    grid_color,
                    self.layout.grid_style.line_style.clone(),
                    grid_width_px,
                );
            }
        }

        if draw_axes && !self.layout.tick_config.enabled {
            let (axis_width, major_tick_size, minor_tick_size, major_tick_width, minor_tick_width) =
                self.axis_tick_metrics_px();
            svg.draw_axes_with_minor_ticks_styled(
                plot_left,
                plot_right,
                plot_top,
                plot_bottom,
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
            );
        }

        let tick_size_px = pt_to_px(
            self.display.config.typography.tick_size(),
            self.display.config.figure.dpi,
        );

        // Draw axes and tick labels
        if draw_axes {
            if let Some(categories) = bar_categories {
                let x_range = x_max - x_min;
                let category_x_tick_positions: Vec<f32> = (0..categories.len())
                    .map(|index| {
                        if x_range.abs() < f64::EPSILON {
                            plot_left + plot_width * 0.5
                        } else {
                            plot_left + (((index as f64) - x_min) / x_range) as f32 * plot_width
                        }
                    })
                    .collect();

                // Bar chart: draw axes with category labels
                if self.layout.tick_config.enabled {
                    let (
                        axis_width,
                        major_tick_size,
                        minor_tick_size,
                        major_tick_width,
                        minor_tick_width,
                    ) = self.axis_tick_metrics_px();
                    svg.draw_axes_with_minor_ticks_styled(
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        &category_x_tick_positions,
                        &y_tick_layout.pixel_positions,
                        &[],
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
                    );

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

                    // Draw category labels on X-axis
                    for (category, &x) in categories.iter().zip(category_x_tick_positions.iter()) {
                        svg.draw_text_centered(
                            category,
                            x,
                            layout.xtick_baseline_y,
                            tick_size_px,
                            self.display.theme.foreground,
                        )?;
                    }
                }
            } else {
                // Normal chart: draw axes with numeric labels
                let x_tick_layout = x_tick_layout.as_ref().ok_or_else(|| {
                    PlottingError::RenderError(
                        "missing x tick layout for non-categorical SVG axes".to_string(),
                    )
                })?;
                if self.layout.tick_config.enabled {
                    let (
                        axis_width,
                        major_tick_size,
                        minor_tick_size,
                        major_tick_width,
                        minor_tick_width,
                    ) = self.axis_tick_metrics_px();
                    svg.draw_axes_with_minor_ticks_styled(
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        &x_tick_layout.pixel_positions,
                        &y_tick_layout.pixel_positions,
                        &x_minor_tick_pixels,
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
                    );
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
                .color
                .unwrap_or_else(|| self.display.theme.get_color(idx));
            let inset_rect = inset_rects[idx];
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rect {
                (
                    inset_rect,
                    self.calculate_data_bounds_from_resolved(std::slice::from_ref(resolved))?,
                )
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

        // Draw title/xlabel/ylabel using layout-computed positions.
        if let Some(ref pos) = layout.title_pos {
            if let Some(title) = frame.title.as_deref() {
                svg.draw_text_centered_with_weight(
                    title,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                    self.display.config.typography.title_weight,
                )?;
            }
        }
        if let Some(ref pos) = layout.xlabel_pos {
            if let Some(xlabel) = frame.xlabel.as_deref() {
                svg.draw_text_centered(
                    xlabel,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                )?;
            }
        }
        if let Some(ref pos) = layout.ylabel_pos {
            if let Some(ylabel) = frame.ylabel.as_deref() {
                svg.draw_text_rotated(
                    ylabel,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                    -90.0,
                )?;
            }
        }

        // Draw legend if we have labeled series and legend is enabled
        if !legend_items.is_empty() && frame.style.legend.enabled {
            let plot_bounds = (plot_left, plot_top, plot_right, plot_bottom);
            svg.draw_legend_full_resolved(
                &legend_items,
                &frame.style.legend,
                plot_bounds,
                None,
                layout.legend_rect.as_ref().map(|rect| rect.bounds()),
            )?;
        }

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
