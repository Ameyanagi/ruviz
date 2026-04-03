use super::*;

impl Plot {
    /// Render the plot to an in-memory image.
    ///
    /// Reactive plots are first resolved into a static snapshot. Temporal
    /// `Signal` sources are sampled at `0.0`; push-based observables and
    /// streaming sources use their latest values. Use `render_at()` to sample
    /// temporal sources at a different time.
    pub fn render(&self) -> Result<Image> {
        if self.is_reactive() {
            let resolved = self.resolved_plot(0.0);
            let image = resolved.render()?;
            self.mark_reactive_sources_rendered();
            return Ok(image);
        }

        self.validate_runtime_environment()?;
        let snapshot_series = self.snapshot_series(0.0);
        Self::validate_series_list(&snapshot_series)?;

        // Check if DataShader optimization should be used
        let total_points = Self::calculate_total_points_for_series(&snapshot_series);
        let has_mixed_coordinates = Self::has_mixed_coordinate_series(&snapshot_series);

        // Warn for very large datasets
        const LARGE_DATASET_THRESHOLD: usize = 1_000_000;
        if total_points > LARGE_DATASET_THRESHOLD {
            log::warn!(
                "Rendering {} points (>1M). DataShader optimization is available for large datasets.",
                total_points
            );
        }

        let use_datashader = !has_mixed_coordinates
            && Self::should_auto_use_datashader(&snapshot_series, total_points);

        if use_datashader {
            // Use DataShader for large datasets
            return self.render_with_datashader(&snapshot_series);
        }

        // Check if parallel processing should be used.
        // Reactive plots are resolved into a static snapshot above, so they can
        // reuse the normal backend-selection path here.
        #[cfg(feature = "parallel")]
        {
            let series_count = snapshot_series.len();
            let parallel_safe_for_markers =
                snapshot_series
                    .iter()
                    .all(|series| match &series.series_type {
                        SeriesType::Line { .. } => {
                            series.marker_style.is_none()
                                && series.x_errors.is_none()
                                && series.y_errors.is_none()
                        }
                        SeriesType::Heatmap { .. } => false,
                        _ => true,
                    });
            if !has_mixed_coordinates
                && parallel_safe_for_markers
                && self
                    .render
                    .parallel_renderer
                    .should_use_parallel(series_count, total_points)
            {
                return self.render_with_parallel();
            }
        }

        // Create renderer for standard rendering with DPI scaling
        let (scaled_width, scaled_height) = self.config_canvas_size();
        let mut renderer =
            SkiaRenderer::new(scaled_width, scaled_height, self.display.theme.clone())?;
        renderer.set_text_engine_mode(self.display.text_engine);
        let render_scale = self.render_scale();
        let dpi = render_scale.dpi();
        renderer.set_render_scale(render_scale);

        let (x_min, x_max, y_min, y_max) =
            self.effective_main_panel_bounds_for_series(&snapshot_series)?;

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series_mgr.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Extract violin categories and positions if present (for categorical x-axis labels)
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

        // Check if this is a violin categorical plot (to use special rendering)
        let is_violin_categorical = !violin_categories.is_empty();

        // Use violin categories if bar_categories is not present and violin categories exist
        let bar_categories = bar_categories.or_else(|| {
            if is_violin_categorical {
                Some(violin_categories.clone())
            } else {
                None
            }
        });
        let content = self.create_plot_content(y_min, y_max);
        let measured_dimensions = self.measure_layout_text(&renderer, &content, dpi)?;
        let measurements = measured_dimensions.as_ref();

        // Choose layout method based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) =
            match &self.display.config.margins {
                MarginConfig::ContentDriven {
                    edge_buffer,
                    center_plot,
                } => {
                    // Use content-driven layout calculator
                    let layout_config = LayoutConfig {
                        edge_buffer_pt: *edge_buffer,
                        center_plot: *center_plot,
                        ..Default::default()
                    };
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
                _ => {
                    // Always use LayoutCalculator for consistent plot area and title/label positioning
                    let layout_config = LayoutConfig::default();
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
            };
        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

        // Generate nice tick values - adapt count to available space
        // Use ~100px per x-tick and ~60px per y-tick as minimum spacing for readability
        let x_tick_count = ((plot_area.width() / 100.0) as usize).clamp(2, 10);
        let y_tick_count = ((plot_area.height() / 60.0) as usize).clamp(2, 8);
        let x_ticks = generate_ticks(x_min, x_max, x_tick_count);
        let y_ticks = generate_ticks(y_min, y_max, y_tick_count);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, x_min, x_max, y_min, y_max, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, x_min, x_max, y_min, y_max, plot_area).1)
            .collect();

        // Draw grid if enabled - using unified GridStyle
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        let draw_axes = Self::needs_cartesian_axes_for_series(&snapshot_series);
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_width_px = self.line_width_px(self.layout.grid_style.line_width);
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
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
            bar_categories.as_ref().map(Vec::len),
            &violin_positions,
        );

        let draw_ticks = draw_axes && self.layout.tick_config.enabled;
        if draw_ticks {
            let x_axis_ticks = categorical_x_tick_pixels
                .as_deref()
                .unwrap_or(x_tick_pixels.as_slice());
            renderer.draw_axes(
                plot_area,
                x_axis_ticks,
                &y_tick_pixels,
                &self.layout.tick_config.direction,
                &self.layout.tick_config.sides,
                self.display.theme.foreground,
            )?;
        }

        // Draw axes and labels using computed layout positions
        // Note: layout_opt is always Some since all render paths now compute layout
        let layout = layout_opt.expect("layout should always be computed");
        let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);

        // Draw tick labels using layout positions
        // Use categorical labels for bar/violin charts, numeric for others
        if draw_axes && is_violin_categorical {
            // Violin plots use their own x-positions for category labels
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
                !draw_ticks,
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
                    !draw_ticks,
                )?;
            } else {
                renderer.draw_axis_labels_at(
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
                    !draw_ticks,
                )?;
            }
        }

        // Draw title if present (resolve at t=0 for static render)
        if let Some(ref pos) = layout.title_pos {
            if let Some(ref title) = self.display.title {
                let title_str = title.resolve(0.0);
                renderer.draw_title_at(pos, &title_str, self.display.theme.foreground)?;
            }
        }

        // Draw xlabel if present
        if let Some(ref pos) = layout.xlabel_pos {
            if let Some(ref xlabel) = self.display.xlabel {
                let xlabel_str = xlabel.resolve(0.0);
                renderer.draw_xlabel_at(pos, &xlabel_str, self.display.theme.foreground)?;
            }
        }

        // Draw ylabel if present
        if let Some(ref pos) = layout.ylabel_pos {
            if let Some(ref ylabel) = self.display.ylabel {
                let ylabel_str = ylabel.resolve(0.0);
                renderer.draw_ylabel_at(pos, &ylabel_str, self.display.theme.foreground)?;
            }
        }

        self.render_series_collection_normal(
            &snapshot_series,
            &mut renderer,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            render_scale,
        )?;

        // Convert renderer output to Image
        Ok(renderer.into_image())
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
        if !self.is_reactive() {
            return self.render();
        }

        let resolved = self.resolved_plot(time);
        let image = resolved.render()?;
        self.mark_reactive_sources_rendered();
        Ok(image)
    }

    /// Render the plot and encode it as PNG bytes.
    pub fn render_png_bytes(&self) -> Result<Vec<u8>> {
        self.render()?.encode_png()
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
        if self.is_reactive() {
            return self.resolved_plot(0.0).render_to_renderer(renderer, dpi);
        }
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }
        renderer.set_text_engine_mode(self.display.text_engine);

        let snapshot_series = self.snapshot_series(0.0);

        // Validate we have at least one series
        if snapshot_series.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        Self::validate_series_list(&snapshot_series)?;

        let (x_min, x_max, y_min, y_max) =
            self.effective_main_panel_bounds_for_series(&snapshot_series)?;

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series_mgr.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Extract violin categories and positions if present (for categorical x-axis labels)
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

        // Use violin categories if bar_categories is not present and violin categories exist
        let is_violin_categorical = !violin_categories.is_empty();
        let bar_categories = bar_categories.or_else(|| {
            if is_violin_categorical {
                Some(violin_categories.clone())
            } else {
                None
            }
        });
        let content = self.create_plot_content(y_min, y_max);
        let measured_dimensions = self.measure_layout_text(renderer, &content, dpi)?;
        let measurements = measured_dimensions.as_ref();

        // Choose layout method based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) =
            match &self.display.config.margins {
                MarginConfig::ContentDriven {
                    edge_buffer,
                    center_plot,
                } => {
                    // Use content-driven layout calculator
                    let layout_config = LayoutConfig {
                        edge_buffer_pt: *edge_buffer,
                        center_plot: *center_plot,
                        ..Default::default()
                    };
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (renderer.width(), renderer.height()),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
                _ => {
                    // Always use LayoutCalculator for consistent plot area and title/label positioning
                    let layout_config = LayoutConfig::default();
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (renderer.width(), renderer.height()),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
            };
        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

        // Generate nice tick values - adapt count to available space
        // Use ~100px per x-tick and ~60px per y-tick as minimum spacing for readability
        let x_tick_count = ((plot_area.width() / 100.0) as usize).clamp(2, 10);
        let y_tick_count = ((plot_area.height() / 60.0) as usize).clamp(2, 8);
        let x_ticks = generate_ticks(x_min, x_max, x_tick_count);
        let y_ticks = generate_ticks(y_min, y_max, y_tick_count);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(tick, 0.0, x_min, x_max, y_min, y_max, plot_area).0)
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| map_data_to_pixels(0.0, tick, x_min, x_max, y_min, y_max, plot_area).1)
            .collect();

        // Draw grid if enabled - using unified GridStyle
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        let draw_axes = Self::needs_cartesian_axes_for_series(&snapshot_series);
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_width_px = pt_to_px(self.layout.grid_style.line_width, dpi);
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
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
            bar_categories.as_ref().map(Vec::len),
            &violin_positions,
        );

        let draw_ticks = draw_axes && self.layout.tick_config.enabled;
        if draw_ticks {
            let x_axis_ticks = categorical_x_tick_pixels
                .as_deref()
                .unwrap_or(x_tick_pixels.as_slice());
            renderer.draw_axes(
                plot_area,
                x_axis_ticks,
                &y_tick_pixels,
                &self.layout.tick_config.direction,
                &self.layout.tick_config.sides,
                self.display.theme.foreground,
            )?;
        }

        // Draw axes and labels using computed layout positions
        // Note: layout_opt is always Some since all render paths now compute layout
        let layout = layout_opt.expect("layout should always be computed");
        let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);

        // Draw tick labels using layout positions
        // Use categorical labels for bar/violin charts, numeric for others
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
                !draw_ticks,
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
                    !draw_ticks,
                )?;
            } else {
                renderer.draw_axis_labels_at(
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
                    !draw_ticks,
                )?;
            }
        }

        // Draw title if present (resolve at t=0 for static render)
        if let Some(ref pos) = layout.title_pos {
            if let Some(ref title) = self.display.title {
                let title_str = title.resolve(0.0);
                renderer.draw_title_at(pos, &title_str, self.display.theme.foreground)?;
            }
        }

        // Draw xlabel if present
        if let Some(ref pos) = layout.xlabel_pos {
            if let Some(ref xlabel) = self.display.xlabel {
                let xlabel_str = xlabel.resolve(0.0);
                renderer.draw_xlabel_at(pos, &xlabel_str, self.display.theme.foreground)?;
            }
        }

        // Draw ylabel if present
        if let Some(ref pos) = layout.ylabel_pos {
            if let Some(ref ylabel) = self.display.ylabel {
                let ylabel_str = ylabel.resolve(0.0);
                renderer.draw_ylabel_at(pos, &ylabel_str, self.display.theme.foreground)?;
            }
        }

        self.render_series_collection_normal(
            &snapshot_series,
            renderer,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            RenderScale::new(dpi),
        )?;

        // Draw annotations after data series but before legend
        if !self.annotations.is_empty() {
            renderer.draw_annotations(
                &self.annotations,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
                dpi,
            )?;
        }

        // Collect legend items, including grouped-series collapse behavior.
        let legend_items = self.collect_legend_items();

        // Draw legend if there are labeled series and legend is enabled
        if !legend_items.is_empty() && self.layout.legend.enabled {
            let mut legend = self.layout.legend.to_legend();
            // Scale legend font size from points to pixels for proper DPI handling
            legend.font_size = pt_to_px(legend.font_size, dpi);

            // Collect data bounding boxes for best position algorithm
            let data_bboxes: Vec<(f32, f32, f32, f32)> =
                if matches!(legend.position, LegendPosition::Best) {
                    let marker_radius = 4.0_f32;
                    self.series_mgr
                        .series
                        .iter()
                        .flat_map(|series| match &series.series_type {
                            SeriesType::Line { x_data, y_data }
                            | SeriesType::Scatter { x_data, y_data } => {
                                let x_data = x_data.resolve(0.0);
                                let y_data = y_data.resolve(0.0);
                                x_data
                                    .iter()
                                    .zip(y_data.iter())
                                    .filter_map(|(&x, &y)| {
                                        if x.is_finite() && y.is_finite() {
                                            let (px, py) = map_data_to_pixels(
                                                x, y, x_min, x_max, y_min, y_max, plot_area,
                                            );
                                            Some((
                                                px - marker_radius,
                                                py - marker_radius,
                                                px + marker_radius,
                                                py + marker_radius,
                                            ))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }
                            _ => vec![],
                        })
                        .collect()
                } else {
                    vec![]
                };

            let bbox_slice: Option<&[(f32, f32, f32, f32)]> = if data_bboxes.is_empty() {
                None
            } else {
                Some(&data_bboxes)
            };

            renderer.draw_legend_full(&legend_items, &legend, plot_area, bbox_slice)?;
        }

        Ok(())
    }

    /// Calculate total number of data points across all series
    pub(super) fn create_plot_content_at_time(
        &self,
        y_min: f64,
        y_max: f64,
        time: f64,
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
            title: self.display.title.as_ref().map(|t| t.resolve(time)),
            xlabel: self.display.xlabel.as_ref().map(|t| t.resolve(time)),
            ylabel: self.display.ylabel.as_ref().map(|t| t.resolve(time)),
            show_tick_labels: self.layout.tick_config.enabled && self.needs_cartesian_axes(),
            max_ytick_chars,
            max_xtick_chars: 5, // Reasonable default
        }
    }

    /// Create PlotContent for layout calculation.
    pub(super) fn create_plot_content(&self, y_min: f64, y_max: f64) -> PlotContent {
        self.create_plot_content_at_time(y_min, y_max, 0.0)
    }

    /// Pre-measure title/xlabel/ylabel for Typst layout parity.
    pub(super) fn measure_layout_text(
        &self,
        renderer: &SkiaRenderer,
        content: &PlotContent,
        dpi: f32,
    ) -> Result<Option<MeasuredDimensions>> {
        let render_scale = RenderScale::new(dpi);
        let title_size_px =
            render_scale.points_to_pixels(self.display.config.typography.title_size());
        let label_size_px =
            render_scale.points_to_pixels(self.display.config.typography.label_size());

        let mut measurements = MeasuredDimensions::default();

        if let Some(title) = content.title.as_deref() {
            measurements.title = Some(renderer.measure_text(title, title_size_px)?);
        }
        if let Some(xlabel) = content.xlabel.as_deref() {
            measurements.xlabel = Some(renderer.measure_text(xlabel, label_size_px)?);
        }
        if let Some(ylabel) = content.ylabel.as_deref() {
            measurements.ylabel = Some(renderer.measure_text(ylabel, label_size_px)?);
        }

        Ok(Some(measurements))
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
    /// Infer and store a backend label based on data size
    ///
    /// Updates [`get_backend_name`](Self::get_backend_name) metadata based on
    /// dataset characteristics:
    /// - < 1K points: Skia (simple, fast)
    /// - 1,000 to 99,999 points: Parallel when available, otherwise Skia
    /// - 100,000+ points: GPU when available, otherwise DataShader
    ///
    /// If a backend was explicitly set with `.backend()`, that choice is respected.
    /// The current [`render`](Self::render) and [`save`](Self::save)
    /// implementations still use their own internal heuristics; calling
    /// `.auto_optimize()` does not directly switch those execution paths today.
    pub fn auto_optimize(self) -> Self {
        self.auto_optimize_with_extra_points(0)
    }

    /// Auto-optimize with additional pending points from PlotBuilder
    ///
    /// This internal method is used by PlotBuilder to include points from
    /// the current series that hasn't been finalized yet.
    pub(crate) fn auto_optimize_with_extra_points(mut self, extra_points: usize) -> Self {
        // If backend already explicitly set, respect that choice
        if self.render.backend.is_some() {
            self.render.auto_optimized = true;
            return self;
        }

        // Count total data points across all series
        let series_points: usize = self
            .series_mgr
            .series
            .iter()
            .map(|s| match &s.series_type {
                SeriesType::Line { x_data, .. } => x_data.len(),
                SeriesType::Scatter { x_data, .. } => x_data.len(),
                SeriesType::Bar { values, .. } => values.len(),
                SeriesType::Histogram { data, .. } => data.len(),
                SeriesType::BoxPlot { data, .. } => data.len(),
                SeriesType::ErrorBars { x_data, .. } => x_data.len(),
                SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Heatmap { data } => data.n_rows * data.n_cols,
                SeriesType::Kde { data } => data.x.len(),
                SeriesType::Ecdf { data } => data.x.len(),
                SeriesType::Violin { data } => data.data.len(),
                SeriesType::Boxen { data } => data.boxes.len() * 4,
                SeriesType::Contour { data } => data.x.len() * data.y.len(),
                SeriesType::Pie { data } => data.values.len(),
                SeriesType::Radar { data } => data.series.iter().map(|s| s.values.len()).sum(),
                SeriesType::Polar { data } => data.points.len(),
            })
            .sum();

        let total_points = series_points + extra_points;

        // Select backend based on data size
        let selected_backend =
            if Self::has_mixed_coordinate_series(&self.series_mgr.series) || total_points < 1000 {
                BackendType::Skia
            } else if total_points < 100_000 {
                #[cfg(feature = "parallel")]
                {
                    BackendType::Parallel
                }
                #[cfg(not(feature = "parallel"))]
                {
                    BackendType::Skia
                }
            } else {
                // For very large datasets, prefer GPU if available, else DataShader
                #[cfg(feature = "gpu")]
                {
                    BackendType::GPU
                }
                #[cfg(not(feature = "gpu"))]
                {
                    BackendType::DataShader
                }
            };

        self.render.backend = Some(selected_backend);
        self.render.auto_optimized = true;
        self
    }

    /// Set backend explicitly (overrides auto-optimization)
    pub fn backend(mut self, backend: BackendType) -> Self {
        self.render.backend = Some(backend);
        self
    }

    /// Enable GPU acceleration for the PNG export path used by [`save`](Self::save).
    ///
    /// When enabled, [`save`](Self::save) may use GPU-assisted rendering for
    /// sufficiently large datasets. [`render`](Self::render) continues to use
    /// the in-memory CPU/DataShader/parallel paths today.
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
    /// - Requires the `gpu` feature to be enabled
    /// - Falls back to CPU if GPU is not available
    #[cfg(feature = "gpu")]
    pub fn gpu(mut self, enabled: bool) -> Self {
        self.render.enable_gpu = enabled;
        if enabled {
            self.render.backend = Some(BackendType::GPU);
        }
        self
    }

    /// Get the current backend name (for testing)
    pub fn get_backend_name(&self) -> &'static str {
        match self.render.backend {
            Some(BackendType::Skia) => "skia",
            Some(BackendType::Parallel) => "parallel",
            Some(BackendType::GPU) => "gpu",
            Some(BackendType::DataShader) => "datashader",
            None => "auto",
        }
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
        if self.is_reactive() {
            let result = self.resolved_plot(0.0).save(path);
            if result.is_ok() {
                self.mark_reactive_sources_rendered();
            }
            return result;
        }

        use crate::render::skia::SkiaRenderer;

        self.validate_runtime_environment()?;
        let snapshot_series = self.snapshot_series(0.0);

        Self::validate_series_list(&snapshot_series)?;

        // Create renderer and render the plot with DPI scaling
        let (scaled_width, scaled_height) = self.dpi_scaled_dimensions();
        let mut renderer =
            SkiaRenderer::new(scaled_width, scaled_height, self.display.theme.clone())?;
        renderer.set_text_engine_mode(self.display.text_engine);

        // Clear background
        renderer.clear();

        let (x_min, x_max, y_min, y_max) = self.apply_auto_padding_to_bounds(
            self.effective_main_panel_bounds_for_series(&snapshot_series)?,
            0.05,
        );

        // Extract bar chart categories if present (for categorical x-axis labels)
        let bar_categories: Option<Vec<String>> = self.series_mgr.series.iter().find_map(|s| {
            if let SeriesType::Bar { categories, .. } = &s.series_type {
                Some(categories.clone())
            } else {
                None
            }
        });

        // Extract violin categories and positions if present (for categorical x-axis labels)
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

        // Check if this is a violin categorical plot (to use special rendering)
        let is_violin_categorical = !violin_categories.is_empty();

        // Use violin categories if bar_categories is not present and violin categories exist
        let bar_categories = bar_categories.or_else(|| {
            if is_violin_categorical {
                Some(violin_categories.clone())
            } else {
                None
            }
        });

        let dpi = self.display.dpi as f32;
        let render_scale = self.render_scale();
        renderer.set_render_scale(render_scale);
        let content = self.create_plot_content(y_min, y_max);
        let measured_dimensions = self.measure_layout_text(&renderer, &content, dpi)?;
        let measurements = measured_dimensions.as_ref();

        // Calculate plot area based on MarginConfig
        let (plot_area, layout_opt): (tiny_skia::Rect, Option<PlotLayout>) =
            match &self.display.config.margins {
                MarginConfig::ContentDriven {
                    edge_buffer,
                    center_plot,
                } => {
                    // Use content-driven layout calculator
                    let layout_config = LayoutConfig {
                        edge_buffer_pt: *edge_buffer,
                        center_plot: *center_plot,
                        ..Default::default()
                    };
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
                _ => {
                    // Always use LayoutCalculator for consistent plot area and title/label positioning
                    let layout_config = LayoutConfig::default();
                    let calculator = LayoutCalculator::new(layout_config);
                    let layout = calculator.compute(
                        (scaled_width, scaled_height),
                        &content,
                        &self.display.config.typography,
                        &self.display.config.spacing,
                        dpi,
                        measurements,
                    );
                    let skia_rect = tiny_skia::Rect::from_ltrb(
                        layout.plot_area.left,
                        layout.plot_area.top,
                        layout.plot_area.right,
                        layout.plot_area.bottom,
                    )
                    .ok_or(PlottingError::InvalidData {
                        message: "Invalid plot area from layout".to_string(),
                        position: None,
                    })?;
                    (skia_rect, Some(layout))
                }
            };

        // Generate major and minor ticks for axes using scale-aware tick generation
        let x_major_ticks = crate::axes::generate_ticks_for_scale(
            x_min,
            x_max,
            self.layout.tick_config.major_ticks_x,
            &self.layout.x_scale,
        );
        let y_major_ticks = crate::axes::generate_ticks_for_scale(
            y_min,
            y_max,
            self.layout.tick_config.major_ticks_y,
            &self.layout.y_scale,
        );

        // Generate minor ticks if configured (using log-aware minor ticks for log scales)
        let x_minor_ticks = if self.layout.tick_config.minor_ticks_x > 0 {
            match &self.layout.x_scale {
                AxisScale::Log => crate::axes::generate_log_minor_ticks(&x_major_ticks),
                _ => crate::render::skia::generate_minor_ticks(
                    &x_major_ticks,
                    self.layout.tick_config.minor_ticks_x,
                ),
            }
        } else {
            Vec::new()
        };
        let y_minor_ticks = if self.layout.tick_config.minor_ticks_y > 0 {
            match &self.layout.y_scale {
                AxisScale::Log => crate::axes::generate_log_minor_ticks(&y_major_ticks),
                _ => crate::render::skia::generate_minor_ticks(
                    &y_major_ticks,
                    self.layout.tick_config.minor_ticks_y,
                ),
            }
        } else {
            Vec::new()
        };

        // Combine ticks for rendering based on grid mode
        let x_ticks = match self.layout.tick_config.grid_mode {
            GridMode::MajorOnly => x_major_ticks.clone(),
            GridMode::MinorOnly => x_minor_ticks.clone(),
            GridMode::Both => {
                let mut combined = x_major_ticks.clone();
                combined.extend(x_minor_ticks.iter());
                combined.sort_by(|a, b| a.partial_cmp(b).unwrap());
                combined
            }
        };
        let y_ticks = match self.layout.tick_config.grid_mode {
            GridMode::MajorOnly => y_major_ticks.clone(),
            GridMode::MinorOnly => y_minor_ticks.clone(),
            GridMode::Both => {
                let mut combined = y_major_ticks.clone();
                combined.extend(y_minor_ticks.iter());
                combined.sort_by(|a, b| a.partial_cmp(b).unwrap());
                combined
            }
        };

        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels_scaled(
                    x,
                    0.0,
                    x_min,
                    x_max,
                    0.0,
                    1.0,
                    plot_area,
                    &self.layout.x_scale,
                    &AxisScale::Linear,
                )
                .0
            })
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels_scaled(
                    0.0,
                    y,
                    0.0,
                    1.0,
                    y_min,
                    y_max,
                    plot_area,
                    &AxisScale::Linear,
                    &self.layout.y_scale,
                )
                .1
            })
            .collect();

        // Render grid if enabled - using unified GridStyle
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        let draw_axes = Self::needs_cartesian_axes_for_series(&snapshot_series);
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            renderer.draw_grid(
                &x_tick_pixels,
                &y_tick_pixels,
                plot_area,
                grid_color,
                self.layout.grid_style.line_style.clone(),
                self.dpi_scaled_line_width(self.layout.grid_style.line_width),
            )?;
        }

        // Convert tick values to pixel positions for major and minor ticks (scale-aware)
        let x_major_tick_pixels: Vec<f32> = x_major_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels_scaled(
                    x,
                    0.0,
                    x_min,
                    x_max,
                    0.0,
                    1.0,
                    plot_area,
                    &self.layout.x_scale,
                    &AxisScale::Linear,
                )
                .0
            })
            .collect();
        let y_major_tick_pixels: Vec<f32> = y_major_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels_scaled(
                    0.0,
                    y,
                    0.0,
                    1.0,
                    y_min,
                    y_max,
                    plot_area,
                    &AxisScale::Linear,
                    &self.layout.y_scale,
                )
                .1
            })
            .collect();

        let x_minor_tick_pixels: Vec<f32> = x_minor_ticks
            .iter()
            .map(|&x| {
                crate::render::skia::map_data_to_pixels_scaled(
                    x,
                    0.0,
                    x_min,
                    x_max,
                    0.0,
                    1.0,
                    plot_area,
                    &self.layout.x_scale,
                    &AxisScale::Linear,
                )
                .0
            })
            .collect();
        let y_minor_tick_pixels: Vec<f32> = y_minor_ticks
            .iter()
            .map(|&y| {
                crate::render::skia::map_data_to_pixels_scaled(
                    0.0,
                    y,
                    0.0,
                    1.0,
                    y_min,
                    y_max,
                    plot_area,
                    &AxisScale::Linear,
                    &self.layout.y_scale,
                )
                .1
            })
            .collect();

        let categorical_x_tick_pixels = Self::categorical_x_tick_pixels(
            plot_area,
            x_min,
            x_max,
            bar_categories.as_ref().map(Vec::len),
            &violin_positions,
        );
        let x_major_axis_ticks = categorical_x_tick_pixels
            .as_deref()
            .unwrap_or(x_major_tick_pixels.as_slice());
        let x_minor_axis_ticks = if categorical_x_tick_pixels.is_some() {
            &[][..]
        } else {
            x_minor_tick_pixels.as_slice()
        };

        // Only draw axes for Cartesian plots (skip for Pie, Radar, etc.)
        let draw_ticks = draw_axes && self.layout.tick_config.enabled;

        if draw_ticks {
            let render_scale = self.render_scale();
            // Always draw axes with enhanced tick system
            renderer.draw_axes_with_config(
                plot_area,
                x_major_axis_ticks,
                &y_major_tick_pixels,
                x_minor_axis_ticks,
                &y_minor_tick_pixels,
                &self.layout.tick_config.direction,
                &self.layout.tick_config.sides,
                self.display.theme.foreground,
                render_scale.reference_scale(),
            )?;
        }

        // Draw axis labels, tick values, and title using computed layout positions
        // Note: layout_opt is always Some since all render paths now compute layout
        let layout = layout_opt.expect("layout should always be computed");
        let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);

        // Draw tick labels using layout positions (only for Cartesian plots)
        if draw_axes {
            // Use categorical labels for bar/violin charts, numeric for others
            if is_violin_categorical {
                // Violin plots use their own x-positions for category labels
                renderer.draw_axis_labels_at_categorical_violin(
                    &layout.plot_area,
                    &violin_categories,
                    &violin_positions,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &y_major_ticks,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.display.theme.foreground,
                    dpi,
                    self.layout.tick_config.enabled,
                    !draw_ticks,
                )?;
            } else if let Some(ref categories) = bar_categories {
                renderer.draw_axis_labels_at_categorical(
                    &layout.plot_area,
                    categories,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &y_major_ticks,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.display.theme.foreground,
                    dpi,
                    self.layout.tick_config.enabled,
                    !draw_ticks,
                )?;
            } else {
                renderer.draw_axis_labels_at(
                    &layout.plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &x_major_ticks,
                    &y_major_ticks,
                    layout.xtick_baseline_y,
                    layout.ytick_right_x,
                    tick_size_px,
                    self.display.theme.foreground,
                    dpi,
                    self.layout.tick_config.enabled,
                    !draw_ticks,
                )?;
            }
        }

        // Draw title if present (resolve at t=0 for static render)
        if let Some(ref pos) = layout.title_pos {
            if let Some(ref title) = self.display.title {
                let title_str = title.resolve(0.0);
                renderer.draw_title_at(pos, &title_str, self.display.theme.foreground)?;
            }
        }

        // Draw x-axis label if present (only for Cartesian plots)
        if draw_axes {
            if let Some(ref pos) = layout.xlabel_pos {
                if let Some(ref xlabel) = self.display.xlabel {
                    let xlabel_str = xlabel.resolve(0.0);
                    renderer.draw_xlabel_at(pos, &xlabel_str, self.display.theme.foreground)?;
                }
            }

            // Draw y-axis label if present
            if let Some(ref pos) = layout.ylabel_pos {
                if let Some(ref ylabel) = self.display.ylabel {
                    let ylabel_str = ylabel.resolve(0.0);
                    renderer.draw_ylabel_at(pos, &ylabel_str, self.display.theme.foreground)?;
                }
            }
        }

        // Check if we should use DataShader for large datasets
        let total_points = Self::calculate_total_points_for_series(&snapshot_series);
        let has_mixed_coordinates = Self::has_mixed_coordinate_series(&snapshot_series);
        #[cfg(feature = "gpu")]
        const GPU_THRESHOLD: usize = 5_000; // Activate GPU for >5K points

        if !has_mixed_coordinates
            && Self::should_auto_use_datashader(&snapshot_series, total_points)
        {
            // Use DataShader for massive datasets - simplified version
            use crate::data::DataShader;

            for series in &snapshot_series {
                match &series.series_type {
                    SeriesType::Scatter { x_data, y_data } => {
                        let x_data = x_data.resolve(0.0);
                        let y_data = y_data.resolve(0.0);
                        let mut datashader = DataShader::with_canvas_size(
                            plot_area.width() as usize,
                            plot_area.height() as usize,
                        );

                        datashader
                            .aggregate_with_bounds(&x_data, &y_data, x_min, x_max, y_min, y_max)?;
                        let image = datashader.render();

                        // Draw the DataShader result
                        renderer.draw_datashader_image(&image, plot_area)?;
                    }
                    SeriesType::Histogram { .. } => {
                        let hist_data = series.series_type.histogram_data_at(0.0)?;

                        // Convert histogram to x,y data for DataShader
                        let x_data: Vec<f64> = hist_data
                            .bin_edges
                            .windows(2)
                            .map(|w| (w[0] + w[1]) / 2.0)
                            .collect();
                        let y_data: Vec<f64> = hist_data.counts;

                        let mut datashader = DataShader::with_canvas_size(
                            plot_area.width() as usize,
                            plot_area.height() as usize,
                        );

                        datashader
                            .aggregate_with_bounds(&x_data, &y_data, x_min, x_max, y_min, y_max)?;
                        let image = datashader.render();

                        // Draw the DataShader result
                        renderer.draw_datashader_image(&image, plot_area)?;
                    }
                    _ => {
                        // For other plot types, use normal rendering
                        self.render_series_normal(
                            series,
                            &mut renderer,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                        )?;
                    }
                }
            }
        } else {
            // Check if GPU rendering should be used for medium datasets
            #[cfg(feature = "gpu")]
            let use_gpu_rendering =
                self.render.enable_gpu && total_points >= GPU_THRESHOLD && !has_mixed_coordinates;

            #[cfg(feature = "gpu")]
            if use_gpu_rendering {
                // Initialize GPU renderer
                match pollster::block_on(GpuRenderer::new()) {
                    Ok(mut gpu_renderer) => {
                        log::info!(
                            "Using GPU rendering for {} points (threshold: {})",
                            total_points,
                            GPU_THRESHOLD
                        );
                        for series in &snapshot_series {
                            self.render_series_gpu(
                                series,
                                &mut renderer,
                                &mut gpu_renderer,
                                plot_area,
                                x_min,
                                x_max,
                                y_min,
                                y_max,
                            )?;
                        }
                    }
                    Err(e) => {
                        log::warn!("GPU initialization failed, falling back to CPU: {}", e);
                        // Fall back to normal rendering
                        self.render_series_collection_normal(
                            &snapshot_series,
                            &mut renderer,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            render_scale,
                        )?;
                    }
                }
            } else {
                // Use normal rendering for smaller datasets
                self.render_series_collection_normal(
                    &snapshot_series,
                    &mut renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    render_scale,
                )?;
            }

            #[cfg(not(feature = "gpu"))]
            {
                // Use normal rendering for smaller datasets (no GPU feature)
                self.render_series_collection_normal(
                    &snapshot_series,
                    &mut renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    render_scale,
                )?;
            }
        }

        // Draw annotations after data series but before legend
        if !self.annotations.is_empty() {
            renderer.draw_annotations(
                &self.annotations,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
                self.display.config.figure.dpi,
            )?;
        }

        // Collect legend items, including grouped-series collapse behavior.
        let legend_items = self.collect_legend_items();

        // Draw legend if there are labeled series and legend is enabled
        if !legend_items.is_empty() && self.layout.legend.enabled {
            // Convert old LegendConfig to new Legend type with DPI scaling
            let mut legend = self.layout.legend.to_legend();
            // Scale legend font size from points to pixels for proper DPI handling
            legend.font_size = pt_to_px(legend.font_size, self.display.config.figure.dpi);

            // Collect data bounding boxes for best position algorithm
            let data_bboxes: Vec<(f32, f32, f32, f32)> =
                if matches!(legend.position, LegendPosition::Best) {
                    let marker_radius = 4.0_f32; // Approximate marker radius in pixels
                    snapshot_series
                        .iter()
                        .flat_map(|series| {
                            match &series.series_type {
                                SeriesType::Line { x_data, y_data }
                                | SeriesType::Scatter { x_data, y_data }
                                | SeriesType::ErrorBars { x_data, y_data, .. }
                                | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                                    let x_data = x_data.resolve(0.0);
                                    let y_data = y_data.resolve(0.0);
                                    x_data
                                        .iter()
                                        .zip(y_data.iter())
                                        .map(|(&x, &y)| {
                                            let (px, py) =
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
                                                );
                                            // Create bounding box around each point
                                            (
                                                px - marker_radius,
                                                py - marker_radius,
                                                px + marker_radius,
                                                py + marker_radius,
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                }
                                _ => vec![], // Skip bar charts, histograms, etc. for now
                            }
                        })
                        .collect()
                } else {
                    vec![]
                };

            // Use new legend rendering with proper handles
            let bbox_slice = if data_bboxes.is_empty() {
                None
            } else {
                Some(data_bboxes.as_slice())
            };
            renderer.draw_legend_full(&legend_items, &legend, plot_area, bbox_slice)?;
        }

        renderer.save_png(path)
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
        let svg_content = self.render_to_svg()?;
        crate::export::write_bytes_atomic(path, svg_content.as_bytes())
    }

    /// Render the plot to an SVG string
    ///
    /// Returns the complete SVG content as a string. This can be saved to a file
    /// or converted to other formats like PDF.
    pub fn render_to_svg(&self) -> Result<String> {
        if self.is_reactive() {
            let svg = self.resolved_plot(0.0).render_to_svg()?;
            self.mark_reactive_sources_rendered();
            return Ok(svg);
        }

        use crate::axes::TickLayout;
        use crate::export::SvgRenderer;

        self.validate_runtime_environment()?;
        let snapshot_series = self.snapshot_series(0.0);
        Self::validate_series_list(&snapshot_series)?;

        let (width_px, height_px) = self.config_canvas_size();
        let width = width_px as f32;
        let height = height_px as f32;

        let mut svg = SvgRenderer::new(width, height);
        let render_scale = self.render_scale();
        svg.set_render_scale(render_scale);
        svg.set_text_engine_mode(self.display.text_engine);

        let (x_min, x_max, y_min, y_max) =
            self.effective_main_panel_bounds_for_series(&snapshot_series)?;

        // Use the same content-driven layout path as PNG rendering.
        let content = self.create_plot_content(y_min, y_max);
        let mut measurement_renderer =
            SkiaRenderer::new(width_px, height_px, self.display.theme.clone())?;
        measurement_renderer.set_text_engine_mode(self.display.text_engine);
        measurement_renderer.set_render_scale(render_scale);
        let measured_dimensions = self.measure_layout_text(
            &measurement_renderer,
            &content,
            self.display.config.figure.dpi,
        )?;
        let measurements = measured_dimensions.as_ref();

        let layout_config = match &self.display.config.margins {
            MarginConfig::ContentDriven {
                edge_buffer,
                center_plot,
            } => LayoutConfig {
                edge_buffer_pt: *edge_buffer,
                center_plot: *center_plot,
                ..Default::default()
            },
            _ => LayoutConfig::default(),
        };
        let calculator = LayoutCalculator::new(layout_config);
        let layout = calculator.compute(
            (width_px, height_px),
            &content,
            &self.display.config.typography,
            &self.display.config.spacing,
            self.display.config.figure.dpi,
            measurements,
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
            6,
        );

        // Draw grid lines (only horizontal for bar charts) - using unified GridStyle
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        let draw_axes = Self::needs_cartesian_axes_for_series(&snapshot_series);
        if self.layout.grid_style.visible && draw_axes {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_width_px = self.line_width_px(self.layout.grid_style.line_width);
            if bar_categories.is_some() {
                // For bar charts, only draw horizontal grid lines
                svg.draw_grid(
                    &[], // no vertical grid lines for bar charts
                    &y_tick_layout.pixel_positions,
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
                let x_tick_layout = TickLayout::compute(
                    x_min,
                    x_max,
                    plot_left,
                    plot_right,
                    &self.layout.x_scale,
                    7,
                );
                svg.draw_grid(
                    &x_tick_layout.pixel_positions,
                    &y_tick_layout.pixel_positions,
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
            // Keep frame stroke width consistent with the tick-enabled path.
            svg.draw_axes(
                plot_left,
                plot_right,
                plot_top,
                plot_bottom,
                &[],
                &[],
                &self.layout.tick_config.direction,
                &TickSides::none(),
                self.display.theme.foreground,
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
                    svg.draw_axes(
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        &category_x_tick_positions,
                        &y_tick_layout.pixel_positions,
                        &self.layout.tick_config.direction,
                        &self.layout.tick_config.sides,
                        self.display.theme.foreground,
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
                let x_tick_layout = TickLayout::compute(
                    x_min,
                    x_max,
                    plot_left,
                    plot_right,
                    &self.layout.x_scale,
                    7,
                );
                if self.layout.tick_config.enabled {
                    svg.draw_axes(
                        plot_left,
                        plot_right,
                        plot_top,
                        plot_bottom,
                        &x_tick_layout.pixel_positions,
                        &y_tick_layout.pixel_positions,
                        &self.layout.tick_config.direction,
                        &self.layout.tick_config.sides,
                        self.display.theme.foreground,
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

        // Collect legend items, including grouped-series collapse behavior.
        let legend_items = self.collect_legend_items();
        let render_scale = self.render_scale();

        let inset_rects = self.inset_rects_for_series(&snapshot_series, plot_area, render_scale)?;

        // Render each series
        for (idx, series) in snapshot_series.iter().enumerate() {
            let default_color = self.display.theme.get_color(idx);
            let inset_rect = inset_rects[idx];
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rect {
                (inset_rect, self.raw_bounds_for_single_series(series)?)
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
                    default_color,
                    series_area,
                    series_bounds.0,
                    series_bounds.1,
                    series_bounds.2,
                    series_bounds.3,
                )?;
            }
        }

        svg.end_group(); // End clip group

        // Draw title/xlabel/ylabel using layout-computed positions.
        if let Some(ref pos) = layout.title_pos {
            if let Some(ref title) = self.display.title {
                let title_str = title.resolve(0.0);
                svg.draw_text_centered(
                    &title_str,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                )?;
            }
        }
        if let Some(ref pos) = layout.xlabel_pos {
            if let Some(ref xlabel) = self.display.xlabel {
                let xlabel_str = xlabel.resolve(0.0);
                svg.draw_text_centered(
                    &xlabel_str,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                )?;
            }
        }
        if let Some(ref pos) = layout.ylabel_pos {
            if let Some(ref ylabel) = self.display.ylabel {
                let ylabel_str = ylabel.resolve(0.0);
                svg.draw_text_rotated(
                    &ylabel_str,
                    pos.x,
                    pos.y,
                    pos.size,
                    self.display.theme.foreground,
                    -90.0,
                )?;
            }
        }

        // Draw legend if we have labeled series and legend is enabled
        if !legend_items.is_empty() && self.layout.legend.enabled {
            // Convert old LegendConfig to new Legend type
            let legend = self.layout.legend.to_legend();
            // Use new legend rendering with proper handles
            let plot_bounds = (plot_left, plot_top, plot_right, plot_bottom);
            svg.draw_legend_full(&legend_items, &legend, plot_bounds, None)?;
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

        // Calculate pixel dimensions from mm (at 96 DPI)
        let (width_mm, height_mm) = size.unwrap_or(page_sizes::PLOT_DEFAULT);
        let width_px = page_sizes::mm_to_px(width_mm) as u32;
        let height_px = page_sizes::mm_to_px(height_mm) as u32;

        self = self.set_output_pixels(width_px, height_px);

        // Render to SVG
        let svg_content = self.render_to_svg()?;

        // Convert SVG to PDF
        let pdf_data = crate::export::svg_to_pdf(&svg_content)?;

        crate::export::write_bytes_atomic(path, &pdf_data)
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
