use super::*;

impl Plot {
    /// Render plot using parallel processing for multiple series
    #[cfg(feature = "parallel")]
    pub(super) fn render_with_parallel(&self) -> Result<Image> {
        use crate::render::parallel::{DataBounds, PlotArea, RenderSeriesType};

        // Start timing for performance measurement
        let start_time = std::time::Instant::now();

        // Create renderer with DPI scaling
        let (scaled_width, scaled_height) = self.dpi_scaled_dimensions();
        let mut renderer =
            SkiaRenderer::new(scaled_width, scaled_height, self.display.theme.clone())?;
        renderer.set_text_engine_mode(self.display.text_engine);
        let dpi = self.display.dpi as f32;
        let render_scale = self.render_scale();
        renderer.set_render_scale(render_scale);

        let resolved_series = self.resolved_series(0.0);
        let bounds = if resolved_series
            .iter()
            .all(|series| !matches!(series, ResolvedSeries::Other(_)))
        {
            self.effective_data_bounds_from_resolved(&resolved_series)?
        } else {
            self.effective_data_bounds()?
        };

        // Compute content-driven layout FIRST for consistent positioning
        let content = self.create_plot_content(bounds.2, bounds.3);
        let layout_config = LayoutConfig {
            edge_buffer_pt: 8.0,
            center_plot: true,
            max_margin_fraction: 0.4,
        };
        let layout_calculator = LayoutCalculator::new(layout_config);
        let measured_dimensions = self.measure_layout_text(&renderer, &content, dpi)?;
        let layout = layout_calculator.compute(
            (scaled_width, scaled_height),
            &content,
            &self.display.config.typography,
            &self.display.config.spacing,
            dpi,
            measured_dimensions.as_ref(),
        );

        // Convert layout plot_area to tiny_skia::Rect for series rendering
        let plot_area = tiny_skia::Rect::from_ltrb(
            layout.plot_area.left,
            layout.plot_area.top,
            layout.plot_area.right,
            layout.plot_area.bottom,
        )
        .unwrap_or_else(|| {
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

        // Generate nice tick values - adapt count to available space
        // Use ~100px per x-tick and ~60px per y-tick as minimum spacing for readability
        let x_tick_count = ((plot_area.width() / 100.0) as usize).clamp(2, 10);
        let y_tick_count = ((plot_area.height() / 60.0) as usize).clamp(2, 8);
        let x_ticks = generate_ticks(bounds.0, bounds.1, x_tick_count);
        let y_ticks = generate_ticks(bounds.2, bounds.3, y_tick_count);

        // Convert ticks to pixel coordinates
        let x_tick_pixels: Vec<f32> = x_ticks
            .iter()
            .map(|&tick| {
                map_data_to_pixels(tick, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).0
            })
            .collect();
        let y_tick_pixels: Vec<f32> = y_ticks
            .iter()
            .map(|&tick| {
                map_data_to_pixels(0.0, tick, bounds.0, bounds.1, bounds.2, bounds.3, plot_area).1
            })
            .collect();

        // Draw grid if enabled - using unified GridStyle (sequential - UI elements)
        // Skip grid for non-Cartesian plots (Pie, Radar, Polar)
        if self.layout.grid_style.visible && self.needs_cartesian_axes() {
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

        // Draw axes (sequential - UI elements) - only for Cartesian plots
        if self.needs_cartesian_axes() && self.layout.tick_config.enabled {
            renderer.draw_axes(
                plot_area,
                &x_tick_pixels,
                &y_tick_pixels,
                &self.layout.tick_config.direction,
                &self.layout.tick_config.sides,
                self.display.theme.foreground,
            )?;
        }

        // Process all series in parallel
        let processed_series = self.render.parallel_renderer.process_series_parallel(
            &self.series_mgr.series,
            |series, index| -> Result<SeriesRenderData> {
                // Get series styling with defaults
                let color = series
                    .color
                    .unwrap_or_else(|| self.display.theme.get_color(index));
                let line_width = self.dpi_scaled_line_width(
                    series.line_width.unwrap_or(self.display.theme.line_width),
                );
                let alpha = series.alpha.unwrap_or(1.0);
                let resolved = &resolved_series[index];

                // Process each series type
                let render_series_type = match &series.series_type {
                    SeriesType::Line { x_data, y_data } => {
                        let x_storage;
                        let y_storage;
                        let (x_data, y_data) = match resolved {
                            ResolvedSeries::Line { x, y } => (x.as_ref(), y.as_ref()),
                            _ => {
                                x_storage = x_data.resolve(0.0);
                                y_storage = y_data.resolve(0.0);
                                (x_storage.as_slice(), y_storage.as_slice())
                            }
                        };
                        // Transform coordinates in parallel
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
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
                    SeriesType::Scatter { x_data, y_data } => {
                        let x_storage;
                        let y_storage;
                        let (x_data, y_data) = match resolved {
                            ResolvedSeries::Scatter { x, y } => (x.as_ref(), y.as_ref()),
                            _ => {
                                x_storage = x_data.resolve(0.0);
                                y_storage = y_data.resolve(0.0);
                                (x_storage.as_slice(), y_storage.as_slice())
                            }
                        };
                        // Transform coordinates in parallel
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        // Process markers in parallel
                        let markers = self.render.parallel_renderer.process_markers_parallel(
                            &points,
                            series.marker_style.unwrap_or(MarkerStyle::Circle),
                            color,
                            8.0, // Default marker size
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Bar { categories, values } => {
                        let values_storage;
                        let values = match resolved {
                            ResolvedSeries::Bar { values, .. } => values.as_ref(),
                            _ => {
                                values_storage = values.resolve(0.0);
                                values_storage.as_slice()
                            }
                        };
                        // Convert categories to x-coordinates
                        let x_data: Vec<f64> = (0..categories.len()).map(|i| i as f64).collect();

                        // Transform coordinates
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                &x_data,
                                values,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        // Bar width as fraction of category spacing (0.8 = 80%, matching matplotlib)
                        let bar_width_fraction = 0.8;
                        let data_range = (bounds.1 - bounds.0) as f32;
                        let pixels_per_unit = parallel_plot_area.width() / data_range;
                        let bar_width = bar_width_fraction * pixels_per_unit;

                        let baseline_y = map_data_to_pixels(
                            0.0, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .1;

                        let bars = points
                            .iter()
                            .enumerate()
                            .map(|(i, point)| {
                                let height = (baseline_y - point.y).abs();
                                crate::render::parallel::BarInstance {
                                    x: point.x - bar_width * 0.5,
                                    y: if values[i] >= 0.0 {
                                        point.y
                                    } else {
                                        baseline_y
                                    },
                                    width: bar_width,
                                    height,
                                    color,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::ErrorBars { x_data, y_data, .. }
                    | SeriesType::ErrorBarsXY { x_data, y_data, .. } => {
                        let x_storage;
                        let y_storage;
                        let (x_data, y_data) = match resolved {
                            ResolvedSeries::ErrorBars { x, y, .. }
                            | ResolvedSeries::ErrorBarsXY { x, y, .. } => (x.as_ref(), y.as_ref()),
                            _ => {
                                x_storage = x_data.resolve(0.0);
                                y_storage = y_data.resolve(0.0);
                                (x_storage.as_slice(), y_storage.as_slice())
                            }
                        };
                        // For now, render error bars as scatter points
                        // Full error bar implementation would be added here
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                x_data,
                                y_data,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        let markers = self.render.parallel_renderer.process_markers_parallel(
                            &points,
                            MarkerStyle::Circle,
                            color,
                            6.0,
                        )?;

                        RenderSeriesType::Scatter { markers }
                    }
                    SeriesType::Histogram { data, config } => {
                        let data_storage;
                        let data = match resolved {
                            ResolvedSeries::Histogram { data, .. } => data.as_ref(),
                            _ => {
                                data_storage = data.resolve(0.0);
                                data_storage.as_slice()
                            }
                        };
                        // Calculate histogram data
                        let hist_data = crate::plots::histogram::calculate_histogram(&data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Histogram calculation failed: {}",
                                    e
                                ))
                            })?;

                        // Convert histogram to bar format for parallel rendering
                        let x_data: Vec<f64> = hist_data
                            .bin_edges
                            .windows(2)
                            .map(|w| (w[0] + w[1]) / 2.0) // bin centers
                            .collect();

                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                &x_data,
                                &hist_data.counts,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        // Create bar instances for histogram
                        let baseline_y = map_data_to_pixels(
                            0.0, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .1;

                        let bars = points
                            .iter()
                            .enumerate()
                            .map(|(i, point)| {
                                let bar_width =
                                    (hist_data.bin_edges[i + 1] - hist_data.bin_edges[i]) as f32;
                                let height = (baseline_y - point.y).abs();
                                crate::render::parallel::BarInstance {
                                    x: point.x - bar_width * 0.5,
                                    y: point.y,
                                    width: bar_width,
                                    height,
                                    color,
                                }
                            })
                            .collect();

                        RenderSeriesType::Bar { bars }
                    }
                    SeriesType::BoxPlot { data, config } => {
                        let data_storage;
                        let data = match resolved {
                            ResolvedSeries::BoxPlot { data, .. } => data.as_ref(),
                            _ => {
                                data_storage = data.resolve(0.0);
                                data_storage.as_slice()
                            }
                        };
                        // Calculate box plot statistics
                        let box_data = crate::plots::boxplot::calculate_box_plot(&data, config)
                            .map_err(|e| {
                                PlottingError::RenderError(format!(
                                    "Box plot calculation failed: {}",
                                    e
                                ))
                            })?;

                        // Transform coordinates for box plot elements
                        let x_center = 0.5; // Center the box plot
                        let box_width = 0.3; // Box width

                        // Map Y coordinates to plot area
                        let q1_y = map_data_to_pixels(
                            box_data.q1,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let median_y = map_data_to_pixels(
                            box_data.median,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let q3_y = map_data_to_pixels(
                            box_data.q3,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let lower_whisker_y = map_data_to_pixels(
                            box_data.min,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;
                        let upper_whisker_y = map_data_to_pixels(
                            box_data.max,
                            0.0,
                            bounds.0,
                            bounds.1,
                            bounds.2,
                            bounds.3,
                            plot_area,
                        )
                        .1;

                        // Map X coordinate
                        let x_center_px = map_data_to_pixels(
                            x_center, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                        )
                        .0;
                        let box_left = x_center_px - box_width * plot_area.width() * 0.5;
                        let box_right = x_center_px + box_width * plot_area.width() * 0.5;

                        // Transform outliers
                        let mut outliers = Vec::new();
                        for &outlier in &box_data.outliers {
                            let outlier_y = map_data_to_pixels(
                                outlier, 0.0, bounds.0, bounds.1, bounds.2, bounds.3, plot_area,
                            )
                            .1;
                            outliers.push(crate::core::types::Point2f {
                                x: x_center_px,
                                y: outlier_y,
                            });
                        }

                        let box_render_data = crate::render::parallel::BoxPlotRenderData {
                            x_center: x_center_px,
                            box_left,
                            box_right,
                            q1_y,
                            median_y,
                            q3_y,
                            lower_whisker_y,
                            upper_whisker_y,
                            outliers,
                            box_color: color,
                            line_color: color,
                            outlier_color: color,
                        };

                        RenderSeriesType::BoxPlot {
                            box_data: box_render_data,
                        }
                    }
                    SeriesType::Heatmap { data } => {
                        // Calculate cell dimensions in pixel space
                        let cell_width = parallel_plot_area.width() / data.n_cols as f32;
                        let cell_height = parallel_plot_area.height() / data.n_rows as f32;

                        // Create heatmap cells with colors
                        let cells: Vec<crate::render::parallel::HeatmapCell> = data
                            .values
                            .iter()
                            .enumerate()
                            .flat_map(|(row_idx, row)| {
                                row.iter().enumerate().map(move |(col_idx, &value)| {
                                    let cell_color = data.get_color(value);
                                    crate::render::parallel::HeatmapCell {
                                        x: parallel_plot_area.left + col_idx as f32 * cell_width,
                                        // Flip Y axis (row 0 at top)
                                        y: parallel_plot_area.top
                                            + (data.n_rows - 1 - row_idx) as f32 * cell_height,
                                        width: cell_width,
                                        height: cell_height,
                                        color: cell_color,
                                    }
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
                            .transform_coordinates_parallel(
                                &kde_data.x,
                                &kde_data.y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
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
                            .transform_coordinates_parallel(
                                &step_x,
                                &step_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
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
                            .transform_coordinates_parallel(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
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
                            .transform_coordinates_parallel(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Contour { data: contour_data } => {
                        // Contour plots use line segment rendering
                        let mut all_points = Vec::new();
                        for level in &contour_data.lines {
                            for &(x1, y1, x2, y2) in &level.segments {
                                all_points.push((x1, y1));
                                all_points.push((x2, y2));
                            }
                        }

                        let poly_x: Vec<f64> = all_points.iter().map(|(x, _)| *x).collect();
                        let poly_y: Vec<f64> = all_points.iter().map(|(_, y)| *y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

                        RenderSeriesType::Line { segments }
                    }
                    SeriesType::Pie { .. } => {
                        // Pie charts use polygon rendering, not supported in parallel mode
                        // Return empty segments (will be rendered using normal path)
                        RenderSeriesType::Line { segments: vec![] }
                    }
                    SeriesType::Radar { data: radar_data } => {
                        // Radar plots use polygon rendering, not supported in parallel mode
                        // Fall back to line rendering of radar polygon outlines
                        let mut all_points = Vec::new();
                        for series_data in &radar_data.series {
                            for &(x, y) in &series_data.polygon {
                                all_points.push((x, y));
                            }
                        }

                        let poly_x: Vec<f64> = all_points.iter().map(|(x, _)| *x).collect();
                        let poly_y: Vec<f64> = all_points.iter().map(|(_, y)| *y).collect();
                        let points = self
                            .render
                            .parallel_renderer
                            .transform_coordinates_parallel(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
                            )?;

                        let segments = self.render.parallel_renderer.process_polyline_parallel(
                            &points,
                            LineStyle::Solid,
                            color,
                            line_width,
                        )?;

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
                            .transform_coordinates_parallel(
                                &poly_x,
                                &poly_y,
                                data_bounds.clone(),
                                parallel_plot_area.clone(),
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

        // Render processed series (sequential - final drawing)
        for processed in processed_series {
            match processed.series_type {
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
                    // Draw all markers
                    for marker in markers {
                        renderer.draw_marker_clipped(
                            marker.position.x,
                            marker.position.y,
                            marker.size,
                            marker.style,
                            marker.color,
                            clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::Bar { bars } => {
                    // Draw all bars
                    for bar in bars {
                        renderer.draw_rectangle_clipped(
                            bar.x, bar.y, bar.width, bar.height, bar.color, true, clip_rect,
                        )?;
                    }
                }
                RenderSeriesType::BoxPlot { box_data } => {
                    // Draw box plot components

                    // Draw the box (IQR)
                    renderer.draw_rectangle_clipped(
                        box_data.box_left,
                        box_data.q3_y,
                        box_data.box_right - box_data.box_left,
                        box_data.q1_y - box_data.q3_y,
                        box_data.box_color,
                        false, // outline only
                        clip_rect,
                    )?;

                    // Draw median line
                    renderer.draw_line_clipped(
                        box_data.box_left,
                        box_data.median_y,
                        box_data.box_right,
                        box_data.median_y,
                        box_data.line_color,
                        2.0, // median line width
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
                        1.0,
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
                        1.0,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw whisker caps
                    let cap_width = (box_data.box_right - box_data.box_left) * 0.3;
                    renderer.draw_line_clipped(
                        box_data.x_center - cap_width,
                        box_data.lower_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.lower_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    renderer.draw_line_clipped(
                        box_data.x_center - cap_width,
                        box_data.upper_whisker_y,
                        box_data.x_center + cap_width,
                        box_data.upper_whisker_y,
                        box_data.line_color,
                        1.0,
                        LineStyle::Solid,
                        clip_rect,
                    )?;

                    // Draw outliers
                    for outlier in &box_data.outliers {
                        renderer.draw_marker_clipped(
                            outlier.x,
                            outlier.y,
                            4.0, // outlier marker size
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
                    // Draw all heatmap cells as filled rectangles
                    for cell in cells {
                        renderer.draw_rectangle_clipped(
                            cell.x,
                            cell.y,
                            cell.width,
                            cell.height,
                            cell.color,
                            true, // filled
                            clip_rect,
                        )?;
                    }
                }
            }
        }

        // Draw tick labels (only for Cartesian plots)
        if self.needs_cartesian_axes() {
            let tick_size_px = pt_to_px(self.display.config.typography.tick_size(), dpi);
            renderer.draw_axis_labels_at(
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
                !self.layout.tick_config.enabled,
            )?;
        }

        // Draw title if present
        if let Some(ref pos) = layout.title_pos {
            if let Some(ref title) = self.display.title {
                let title_str = title.resolve(0.0);
                renderer.draw_title_at(pos, &title_str, self.display.theme.foreground)?;
            }
        }

        // Draw xlabel if present (only for Cartesian plots)
        if self.needs_cartesian_axes() {
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

    /// Calculate data bounds across all series
    pub(super) fn calculate_data_bounds(&self) -> Result<(f64, f64, f64, f64)> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series_mgr.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    for (&x_val, &y_val) in x_data.iter().zip(y_data.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                SeriesType::Bar { categories, values } => {
                    let values = values.resolve_cow(0.0);
                    // Add 0.5-unit padding on each side for bar charts (matplotlib-compatible)
                    // This ensures bars at positions 0 and n-1 are fully visible
                    x_min = x_min.min(-0.5);
                    x_max = x_max.max(categories.len() as f64 - 0.5);

                    for &val in values.iter() {
                        if val.is_finite() {
                            y_min = y_min.min(val.min(0.0));
                            y_max = y_max.max(val.max(0.0));
                        }
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    for ((&x_val, &y_val), &y_err) in
                        x_data.iter().zip(y_data.iter()).zip(y_errors.iter())
                    {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let x_errors = x_errors.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    for (((&x_val, &y_val), &x_err), &y_err) in x_data
                        .iter()
                        .zip(y_data.iter())
                        .zip(x_errors.iter())
                        .zip(y_errors.iter())
                    {
                        if x_val.is_finite() && x_err.is_finite() {
                            x_min = x_min.min(x_val - x_err);
                            x_max = x_max.max(x_val + x_err);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::Histogram { data, config } => {
                    let data = data.resolve(0.0);
                    // Calculate histogram to get data bounds
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(&data, config)
                    {
                        // X bounds from bin edges
                        if !hist_data.bin_edges.is_empty() {
                            x_min = x_min.min(*hist_data.bin_edges.first().unwrap());
                            x_max = x_max.max(*hist_data.bin_edges.last().unwrap());
                        }

                        // Y bounds from counts (include zero baseline)
                        y_min = y_min.min(0.0);
                        for &count in &hist_data.counts {
                            if count.is_finite() && count > 0.0 {
                                y_max = y_max.max(count);
                            }
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }

                    // Set x bounds for box plot (centered at 0.5)
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);

                    // Y bounds include all data values
                    for &value in data.iter() {
                        if value.is_finite() {
                            y_min = y_min.min(value);
                            y_max = y_max.max(value);
                        }
                    }
                }
                SeriesType::Heatmap { data } => {
                    // Heatmap bounds: x from 0 to n_cols, y from 0 to n_rows
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(data.n_cols as f64);
                    y_min = y_min.min(0.0);
                    y_max = y_max.max(data.n_rows as f64);
                }
                SeriesType::Kde { data } => {
                    // KDE bounds from x/y data
                    for i in 0..data.x.len() {
                        let x_val = data.x[i];
                        let y_val = data.y[i];

                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                    // Include zero baseline for density plots
                    y_min = y_min.min(0.0);
                }
                SeriesType::Ecdf { data } => {
                    // ECDF bounds from x/y data
                    for i in 0..data.x.len() {
                        let x_val = data.x[i];
                        let y_val = data.y[i];

                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                    // Include zero baseline for ECDF
                    y_min = y_min.min(0.0);
                }
                SeriesType::Violin { data } => {
                    // Violin bounds from KDE range (extends beyond data range by 3 bandwidths)
                    // Use KDE's x values which represent the evaluation range
                    let (kde_min, kde_max) = if !data.kde.x.is_empty() {
                        (
                            data.kde.x.first().copied().unwrap_or(data.range.0),
                            data.kde.x.last().copied().unwrap_or(data.range.1),
                        )
                    } else {
                        data.range
                    };
                    if kde_min.is_finite() {
                        y_min = y_min.min(kde_min);
                    }
                    if kde_max.is_finite() {
                        y_max = y_max.max(kde_max);
                    }
                    // X bounds for centered violin
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                }
                SeriesType::Boxen { data } => {
                    // Boxen bounds from data range
                    let (data_min, data_max) = data.data_range;
                    if data_min.is_finite() {
                        y_min = y_min.min(data_min);
                    }
                    if data_max.is_finite() {
                        y_max = y_max.max(data_max);
                    }
                    // X bounds for centered boxen
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                }
                SeriesType::Contour { data } => {
                    // Contour bounds from grid coordinates
                    for &x_val in &data.x {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                    }
                    for &y_val in &data.y {
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                SeriesType::Pie { .. } => {
                    // Pie charts use normalized 0-1 coordinate space
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                    y_min = y_min.min(0.0);
                    y_max = y_max.max(1.0);
                }
                SeriesType::Radar { data } => {
                    // Radar charts use normalized coordinate space
                    // Bounds are determined by the polygon coordinates
                    for series_data in &data.series {
                        for &(x_val, y_val) in &series_data.polygon {
                            if x_val.is_finite() {
                                x_min = x_min.min(x_val);
                                x_max = x_max.max(x_val);
                            }
                            if y_val.is_finite() {
                                y_min = y_min.min(y_val);
                                y_max = y_max.max(y_val);
                            }
                        }
                    }
                }
                SeriesType::Polar { data } => {
                    // Polar plots need symmetric space centered at origin
                    // Use ONLY the label margin bounds, ignoring actual data points
                    // This ensures asymmetric curves (like cardioid) still have centered labels
                    let label_margin = data.r_max * 1.5;
                    x_min = -label_margin;
                    x_max = label_margin;
                    y_min = -label_margin;
                    y_max = label_margin;
                }
            }
        }

        // Handle edge cases
        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        Ok((x_min, x_max, y_min, y_max))
    }

    pub(super) fn calculate_data_bounds_from_resolved(
        &self,
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for resolved in resolved_series {
            match resolved {
                ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y } => {
                    for (&x_val, &y_val) in x.iter().zip(y.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                ResolvedSeries::Bar { categories, values } => {
                    x_min = x_min.min(-0.5);
                    x_max = x_max.max(categories.len() as f64 - 0.5);
                    for &value in values.iter() {
                        if value.is_finite() {
                            y_min = y_min.min(value.min(0.0));
                            y_max = y_max.max(value.max(0.0));
                        }
                    }
                }
                ResolvedSeries::ErrorBars { x, y, y_errors } => {
                    for ((&x_val, &y_val), &y_err) in x.iter().zip(y.iter()).zip(y_errors.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                } => {
                    for (((&x_val, &y_val), &x_err), &y_err) in x
                        .iter()
                        .zip(y.iter())
                        .zip(x_errors.iter())
                        .zip(y_errors.iter())
                    {
                        if x_val.is_finite() && x_err.is_finite() {
                            x_min = x_min.min(x_val - x_err);
                            x_max = x_max.max(x_val + x_err);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                ResolvedSeries::Histogram { data, config } => {
                    let data: &[f64] = data.as_ref();
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(&data, config)
                    {
                        if !hist_data.bin_edges.is_empty() {
                            x_min = x_min.min(*hist_data.bin_edges.first().unwrap());
                            x_max = x_max.max(*hist_data.bin_edges.last().unwrap());
                        }
                        y_min = y_min.min(0.0);
                        for &count in &hist_data.counts {
                            if count.is_finite() && count > 0.0 {
                                y_max = y_max.max(count);
                            }
                        }
                    }
                }
                ResolvedSeries::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                    for &value in data.iter() {
                        if value.is_finite() {
                            y_min = y_min.min(value);
                            y_max = y_max.max(value);
                        }
                    }
                }
                ResolvedSeries::Other(_) => {}
            }
        }

        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        Ok((x_min, x_max, y_min, y_max))
    }

    pub(super) fn calculate_data_bounds_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<(f64, f64, f64, f64)> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in series_list {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    for (&x_val, &y_val) in x_data.iter().zip(y_data.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                SeriesType::Bar { categories, values } => {
                    let values = values.resolve_cow(0.0);
                    x_min = x_min.min(-0.5);
                    x_max = x_max.max(categories.len() as f64 - 0.5);
                    for &value in values.iter() {
                        if value.is_finite() {
                            y_min = y_min.min(value.min(0.0));
                            y_max = y_max.max(value.max(0.0));
                        }
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    for ((&x_val, &y_val), &y_err) in
                        x_data.iter().zip(y_data.iter()).zip(y_errors.iter())
                    {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let x_errors = x_errors.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    for (((&x_val, &y_val), &x_err), &y_err) in x_data
                        .iter()
                        .zip(y_data.iter())
                        .zip(x_errors.iter())
                        .zip(y_errors.iter())
                    {
                        if x_val.is_finite() && x_err.is_finite() {
                            x_min = x_min.min(x_val - x_err);
                            x_max = x_max.max(x_val + x_err);
                        }
                        if y_val.is_finite() && y_err.is_finite() {
                            y_min = y_min.min(y_val - y_err);
                            y_max = y_max.max(y_val + y_err);
                        }
                    }
                }
                SeriesType::Histogram { data, config } => {
                    let data = data.resolve(0.0);
                    if let Ok(hist_data) =
                        crate::plots::histogram::calculate_histogram(&data, config)
                    {
                        if !hist_data.bin_edges.is_empty() {
                            x_min = x_min.min(*hist_data.bin_edges.first().unwrap());
                            x_max = x_max.max(*hist_data.bin_edges.last().unwrap());
                        }
                        y_min = y_min.min(0.0);
                        for &count in &hist_data.counts {
                            if count.is_finite() && count > 0.0 {
                                y_max = y_max.max(count);
                            }
                        }
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                    for &value in data.iter() {
                        if value.is_finite() {
                            y_min = y_min.min(value);
                            y_max = y_max.max(value);
                        }
                    }
                }
                SeriesType::Heatmap { data } => {
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(data.n_cols as f64);
                    y_min = y_min.min(0.0);
                    y_max = y_max.max(data.n_rows as f64);
                }
                SeriesType::Kde { data } => {
                    for (&x_val, &y_val) in data.x.iter().zip(data.y.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                    y_min = y_min.min(0.0);
                }
                SeriesType::Ecdf { data } => {
                    for (&x_val, &y_val) in data.x.iter().zip(data.y.iter()) {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                    y_min = y_min.min(0.0);
                }
                SeriesType::Violin { data } => {
                    let (kde_min, kde_max) = if !data.kde.x.is_empty() {
                        (
                            data.kde.x.first().copied().unwrap_or(data.range.0),
                            data.kde.x.last().copied().unwrap_or(data.range.1),
                        )
                    } else {
                        data.range
                    };
                    if kde_min.is_finite() {
                        y_min = y_min.min(kde_min);
                    }
                    if kde_max.is_finite() {
                        y_max = y_max.max(kde_max);
                    }
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                }
                SeriesType::Boxen { data } => {
                    let (data_min, data_max) = data.data_range;
                    if data_min.is_finite() {
                        y_min = y_min.min(data_min);
                    }
                    if data_max.is_finite() {
                        y_max = y_max.max(data_max);
                    }
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                }
                SeriesType::Contour { data } => {
                    for &x_val in &data.x {
                        if x_val.is_finite() {
                            x_min = x_min.min(x_val);
                            x_max = x_max.max(x_val);
                        }
                    }
                    for &y_val in &data.y {
                        if y_val.is_finite() {
                            y_min = y_min.min(y_val);
                            y_max = y_max.max(y_val);
                        }
                    }
                }
                SeriesType::Pie { .. } => {
                    x_min = x_min.min(0.0);
                    x_max = x_max.max(1.0);
                    y_min = y_min.min(0.0);
                    y_max = y_max.max(1.0);
                }
                SeriesType::Radar { data } => {
                    for series_data in &data.series {
                        for &(x_val, y_val) in &series_data.polygon {
                            if x_val.is_finite() {
                                x_min = x_min.min(x_val);
                                x_max = x_max.max(x_val);
                            }
                            if y_val.is_finite() {
                                y_min = y_min.min(y_val);
                                y_max = y_max.max(y_val);
                            }
                        }
                    }
                }
                SeriesType::Polar { data } => {
                    let label_margin = data.r_max * 1.5;
                    x_min = -label_margin;
                    x_max = label_margin;
                    y_min = -label_margin;
                    y_max = label_margin;
                }
            }
        }

        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        Ok((x_min, x_max, y_min, y_max))
    }
}
