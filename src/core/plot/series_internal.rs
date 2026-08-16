use super::*;
use crate::core::Point2f;
use crate::core::plot::raster_batches::{
    DensityBatch, RectGridBatch, SeriesRasterPlan, clip_rect_from_plot_area, plot_area_from_rect,
    project_xy_points, project_xy_subpaths,
};
use crate::core::plot::raster_fast_path::{
    canonicalize_line_points_exact, reduce_line_points_for_raster, should_reduce_line_series,
};
use crate::core::plot::series_api::series_from_style;
use crate::core::plot::types::MarkerEdge;
use crate::plots::boxplot::{CATEGORY_SLOT_HALF_WIDTH, category_slot_span};
use crate::plots::traits::{AxisScaleSupport, ComputedSeries};

impl Plot {
    /// Add a new line to existing plot (for incremental updates)
    pub fn add_line<X, Y>(&mut self, x_data: &X, y_data: &Y) -> Result<()>
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let x_vec = collect_numeric_data_1d(x_data, self.null_policy)?;
        let y_vec = collect_numeric_data_1d(y_data, self.null_policy)?;

        if x_vec.len() != y_vec.len() {
            return Err(PlottingError::DataLengthMismatch {
                x_len: x_vec.len(),
                y_len: y_vec.len(),
                series_index: None,
            });
        }

        if x_vec.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }

        let series = series_from_style(
            SeriesType::Line {
                x_data: PlotData::Static(x_vec),
                y_data: PlotData::Static(y_vec),
            },
            SeriesStyle::default(),
        );

        let auto_color_slot =
            (!series.props.color.is_set()).then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
        self.series_mgr.auto_color_index += 1;

        Ok(())
    }

    /// Internal method to add a KDE series (used by PlotBuilder)
    ///
    /// This method is called by the PlotBuilder when finalizing a KDE series.
    pub(crate) fn add_kde_series(
        self,
        kde_data: crate::plots::KdeData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Kde {
                data: Arc::new(kde_data),
            },
            style,
        ))
    }

    /// Internal method to add an ECDF series
    pub(crate) fn add_ecdf_series(
        self,
        ecdf_data: crate::plots::EcdfData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Ecdf {
                data: Arc::new(ecdf_data),
            },
            style,
        ))
    }

    /// Internal method to add a Contour series
    pub(crate) fn add_contour_series(
        self,
        contour_data: crate::plots::continuous::contour::ContourPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Contour {
                data: Arc::new(contour_data),
            },
            style,
        ))
    }

    /// Internal method to add a Pie series
    pub(crate) fn add_pie_series(
        self,
        pie_data: crate::plots::composition::pie::PieData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Pie {
                data: Arc::new(pie_data),
            },
            style,
        ))
    }

    /// Internal method to add a Radar series
    pub(crate) fn add_radar_series(
        self,
        radar_data: crate::plots::polar::radar::RadarPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Radar {
                data: Arc::new(radar_data),
            },
            style,
        ))
    }

    /// Internal method to add a Box Plot series
    ///
    /// Box plots, violins and boxen plots are all added through one of these
    /// three functions, and each one claims the series' category slot the same
    /// way before pushing it — so "which slot am I in" is answered once, at
    /// add time, and every backend afterwards just reads `x_position`.
    pub(crate) fn add_box_plot_series(
        self,
        data: PlotData,
        mut config: crate::plots::boxplot::BoxPlotConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let slot = config
            .x_position
            .unwrap_or_else(|| self.next_category_slot(config.category.as_deref()));
        config.x_position = Some(slot);

        self.push_builder_series(series_from_style(
            SeriesType::BoxPlot { data, config },
            style,
        ))
    }

    /// Internal method to add a Violin series
    pub(crate) fn add_violin_series(
        self,
        mut violin_data: crate::plots::ViolinData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let slot = violin_data
            .config
            .x_position
            .unwrap_or_else(|| self.next_category_slot(violin_data.config.category.as_deref()));
        violin_data.config.x_position = Some(slot);

        self.push_builder_series(series_from_style(
            SeriesType::Violin {
                data: Arc::new(violin_data),
            },
            style,
        ))
    }

    /// Internal method to add a Boxen series
    pub(crate) fn add_boxen_series(
        self,
        mut boxen_data: crate::plots::BoxenData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        if let Some(line_width) = style.props.line_width.cloned() {
            boxen_data.config.line_width = line_width.max(0.0);
        }
        if let Some(marker_size) = style.props.marker_size.cloned() {
            boxen_data.config.outlier_size = marker_size.max(0.0);
        }
        let slot = boxen_data
            .config
            .x_position
            .unwrap_or_else(|| self.next_category_slot(boxen_data.config.category.as_deref()));
        boxen_data.config.x_position = Some(slot);

        self.push_builder_series(series_from_style(
            SeriesType::Boxen {
                data: Arc::new(boxen_data),
            },
            style,
        ))
    }

    /// Internal method to add a Polar series
    pub(crate) fn add_polar_series(
        self,
        polar_data: crate::plots::polar::polar_plot::PolarPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.push_builder_series(series_from_style(
            SeriesType::Polar {
                data: Arc::new(polar_data),
            },
            style,
        ))
    }

    /// Internal method to add a Quiver series
    pub(crate) fn add_quiver_series(
        self,
        mut quiver_data: crate::plots::QuiverPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        if let Some(color) = style.props.color.cloned() {
            quiver_data.config.color = Some(color);
        }
        if let Some(line_width) = style.props.line_width.cloned() {
            quiver_data.config.width = line_width.max(0.1);
        }

        self.push_builder_series(series_from_style(
            SeriesType::Quiver {
                data: Arc::new(quiver_data),
            },
            style,
        ))
    }

    /// Internal method to add a Line series (used by PlotBuilder<LineConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a line series.
    pub(crate) fn add_line_series(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::LineConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_line_series_grouped(x_data, y_data, config, style, None, true)
    }

    /// Internal method to add a Line series with optional grouped-series metadata.
    pub(crate) fn add_line_series_grouped(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::LineConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let mut series = series_from_style(SeriesType::Line { x_data, y_data }, style);
        series
            .props
            .line_style
            .or_value(Some(config.line_style.clone()));
        series.props.marker_style.or_value(config.marker);
        series.props.alpha.or_value(Some(config.alpha));
        series.marker_edge = config
            .resolved_marker_edge_spec()
            .map(|(color, width)| MarkerEdge { color, width });
        series.group_id = group_id;

        self.push_grouped_series(series, consume_palette_index)
    }

    /// Internal method to add a Scatter series (used by PlotBuilder<ScatterConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a scatter series.
    pub(crate) fn add_scatter_series(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::ScatterConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_scatter_series_grouped(x_data, y_data, config, style, None, true)
    }

    /// Internal method to add a Scatter series with optional grouped-series metadata.
    pub(crate) fn add_scatter_series_grouped(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::ScatterConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let mut series = series_from_style(SeriesType::Scatter { x_data, y_data }, style);
        series.props.marker_style.or_value(Some(config.marker));
        series.props.alpha.or_value(Some(config.alpha));
        series.marker_edge = config
            .resolved_edge_spec()
            .map(|(color, width)| MarkerEdge { color, width });
        series.density = config.density;
        series.group_id = group_id;

        self.push_grouped_series(series, consume_palette_index)
    }

    /// Internal method to add a Bar series (used by PlotBuilder<BarConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a bar series.
    pub(crate) fn add_bar_series(
        self,
        categories: Vec<String>,
        values: PlotData,
        config: &crate::plots::basic::BarConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_bar_series_grouped(categories, values, config, style, None, true)
    }

    /// Internal method to add a Bar series with optional grouped-series metadata.
    pub(crate) fn add_bar_series_grouped(
        self,
        categories: Vec<String>,
        values: PlotData,
        config: &crate::plots::basic::BarConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let mut series = series_from_style(
            SeriesType::Bar {
                categories,
                values,
                config: config.clone(),
            },
            style,
        );
        series.props.color.or_value(config.color);
        series.props.line_width.or_value(Some(config.edge_width));
        series.props.alpha.or_value(Some(config.alpha));
        series.group_id = group_id;

        self.push_grouped_series(series, consume_palette_index)
    }

    /// Commit a series built by one of the grouped constructors above.
    ///
    /// The palette rule is written once here rather than per plot type: a series
    /// that chose no colour takes an auto-colour slot, and `consume_palette_index`
    /// says whether this series advances the palette (a standalone series does; a
    /// member of a group that shares one slot does not, and points back at the
    /// slot the group already took).
    fn push_grouped_series(mut self, series: PlotSeries, consume_palette_index: bool) -> Self {
        let auto_color_slot = (!series.props.color.is_set()).then(|| {
            if consume_palette_index {
                self.series_mgr.auto_color_index
            } else {
                self.series_mgr.auto_color_index.saturating_sub(1)
            }
        });
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
        if consume_palette_index {
            self.series_mgr.auto_color_index += 1;
        }
        self
    }

    /// Resolve a series' marker edge against the theme and its resolved fill.
    ///
    /// Returns `(colour, width_in_points)`; the renderer scales the width to
    /// device pixels, so a marker rim is DPI-invariant. `None` means the series
    /// asked for bare markers (`ScatterConfig::show_edge(false)` or a zero edge
    /// width), or carries no marker edge configuration at all.
    pub(super) fn resolved_marker_edge(
        &self,
        series: &PlotSeries,
        fill: Color,
    ) -> Option<(Color, f32)> {
        series
            .marker_edge
            .and_then(|edge| edge.resolve(&self.display.theme, fill))
    }

    pub(super) fn build_prepared_series_raster_plan(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<Option<SeriesRasterPlan>> {
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0));
        let line_width = self.dpi_scaled_line_width(
            series
                .props
                .line_width
                .value_or(self.display.config.lines.data_width),
        );
        let line_style = series.props.line_style.value_or(LineStyle::Solid);
        let clip_rect = clip_rect_from_plot_area(plot_area);

        let plan = match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                // One sub-path per contiguous run of representable samples: a
                // sample the axis cannot place (non-finite anywhere, or
                // non-positive on a log axis) breaks the line instead of being
                // joined across, which would draw a segment the user never
                // supplied.
                let subpaths = project_xy_subpaths(
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
                let mut raster_plan = SeriesRasterPlan::default();
                let mut marker_points: Vec<Point2f> = Vec::new();

                for mut points in subpaths {
                    let has_markers = series.props.marker_style.value().is_some();
                    if has_markers {
                        // Captured before any decimation: reducing the
                        // polyline must never move or drop a marker.
                        marker_points.extend_from_slice(points.as_ref());
                    }

                    if !has_markers
                        && series.x_errors.is_none()
                        && series.y_errors.is_none()
                        && let Some(canonicalized) = canonicalize_line_points_exact(points.as_ref())
                    {
                        raster_plan.note_exact_line_canonicalization();
                        points = canonicalized.into();
                    }

                    // A marked line keeps its full polyline by default —
                    // decimating the stroke changes its bytes — but under
                    // fast mode the stroke takes the same min/max reduction
                    // a bare solid line always gets, while the markers above
                    // stay complete.
                    if (!has_markers || self.render.fast)
                        && mode.allows_raster_line_reduction()
                        && should_reduce_line_series(series, points.len(), plot_area.width())
                        && let Some(reduced) = reduce_line_points_for_raster(
                            points.as_ref(),
                            plot_area.left(),
                            plot_area.width(),
                        )
                    {
                        raster_plan.note_raster_line_reduction();
                        points = reduced.into();
                    }

                    raster_plan.push_polyline(
                        points,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    );
                }

                if let Some(marker_style) = series.props.marker_style.cloned() {
                    let marker_size =
                        self.dpi_scaled_line_width(series.props.marker_size.value_or(8.0));
                    let marker_edge = self.resolved_marker_edge(series, color);
                    raster_plan.push_markers(
                        marker_points.into(),
                        marker_size,
                        marker_style,
                        color,
                        marker_edge,
                        clip_rect,
                    );
                }
                Some(raster_plan)
            }
            (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                let mut raster_plan = SeriesRasterPlan::default();
                // Fast mode upgrades a heavily overdrawn scatter to density
                // aggregation: past one point per plot pixel, exact markers
                // mostly repaint pixels that are already covered. Under
                // manual axis limits only the points inside the window can
                // overdraw it, so a zoomed view with a large but mostly
                // clipped dataset stays exact.
                let auto_density = self.render.fast && {
                    let plot_pixels = f64::from(plot_area.width()) * f64::from(plot_area.height());
                    if (x.len() as f64) <= plot_pixels {
                        false
                    } else if self.layout.x_limits.is_some() || self.layout.y_limits.is_some() {
                        let visible = x
                            .iter()
                            .zip(y.iter())
                            .filter(|&(&px, &py)| {
                                px.is_finite()
                                    && py.is_finite()
                                    && px >= x_min
                                    && px <= x_max
                                    && py >= y_min
                                    && py <= y_max
                            })
                            .count();
                        visible as f64 > plot_pixels
                    } else {
                        true
                    }
                };
                let marker_size =
                    self.dpi_scaled_line_width(series.props.marker_size.value_or(10.0));
                let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
                // An explicit per-series choice wins in both directions;
                // only an unset series takes fast mode's automatic upgrade.
                if series.density.unwrap_or(auto_density) {
                    raster_plan.push_density(DensityBatch::from_xy(
                        x,
                        y,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                        color,
                        // The same footprint the exact markers would paint, so
                        // the density silhouette matches the marker render.
                        marker_size,
                        marker_style,
                    ));
                    return Ok(Some(raster_plan));
                }

                let points = project_xy_points(
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
                let marker_edge = self.resolved_marker_edge(series, color);
                raster_plan.push_markers(
                    points,
                    marker_size,
                    marker_style,
                    color,
                    marker_edge,
                    clip_rect,
                );
                Some(raster_plan)
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                RectGridBatch::from_heatmap_data(
                    data,
                    heatmap_plot_area,
                    data.config.alpha * series.props.alpha.value_or(1.0),
                )
                .map(|rect_grid| {
                    let mut raster_plan = SeriesRasterPlan::default();
                    raster_plan.push_rect_grid(rect_grid);
                    raster_plan
                })
            }
            _ => None,
        };

        Ok(plan)
    }

    /// Draw the colour key this series asks for, if it asks for one.
    ///
    /// The request comes from [`Plot::series_colorbar_request`] — the same
    /// dispatcher the right-margin reservation and the SVG export read. Routing
    /// the raster draw through it too is what stops a plot type from reserving
    /// room for a key that is never drawn, which is exactly how quiver shipped a
    /// `colorbar` setting that did nothing.
    fn draw_series_colorbar(
        &self,
        renderer: &mut SkiaRenderer,
        series_type: &SeriesType,
        plot_area: tiny_skia::Rect,
    ) -> Result<()> {
        let Some(request) = self.series_colorbar_request(series_type) else {
            return Ok(());
        };
        let (x, y, width, height) = self.colorbar_rect(plot_area);
        crate::render::colorbar::draw_colorbar(
            renderer,
            &request.spec_at(x, y, width, height, self.display.theme.foreground),
        )
    }

    pub(super) fn render_series_overlays_after_raster(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        color: Color,
        line_width: f32,
        _line_style: &LineStyle,
    ) -> Result<()> {
        match (&series.series_type, resolved) {
            (
                SeriesType::Line { .. } | SeriesType::Scatter { .. },
                ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y },
            ) => {
                if series.y_errors.is_some() || series.x_errors.is_some() {
                    Self::render_attached_error_bars(
                        renderer,
                        x,
                        y,
                        series.y_errors.as_ref().map(ErrorValuesRef::from),
                        series.x_errors.as_ref().map(ErrorValuesRef::from),
                        series.error_config.as_ref(),
                        color,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        line_width,
                        self.render_scale(),
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    )?;
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let render_scale = self.render_scale();
                let min_annotation_font_px = render_scale.points_to_pixels(8.0);
                let max_annotation_font_px = render_scale.points_to_pixels(20.0);
                for (row_idx, row) in data.values.iter().enumerate() {
                    for (col_idx, &value) in row.iter().enumerate() {
                        if !data.config.annotate || data.should_mask_value(value) {
                            continue;
                        }

                        let alpha = data.config.alpha * series.props.alpha.value_or(1.0);
                        let cell_color = if alpha < 1.0 {
                            data.get_color(value).with_alpha(alpha)
                        } else {
                            data.get_color(value)
                        };

                        let (cell_x, cell_y, cell_width, cell_height) =
                            data.cell_screen_rect(&heatmap_plot_area, row_idx, col_idx);
                        let text = data.format_annotation(value);
                        let text_color = data.get_text_color(cell_color);
                        let text_x = cell_x + cell_width / 2.0;
                        let font_size = (cell_height * 0.3)
                            .clamp(min_annotation_font_px, max_annotation_font_px);
                        let text_y = cell_y + cell_height / 2.0 + font_size / 3.0;
                        renderer
                            .draw_text_centered(&text, text_x, text_y, font_size, text_color)?;
                    }
                }

                self.draw_series_colorbar(renderer, &series.series_type, plot_area)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn render_quiver_series_scaled(
        &self,
        renderer: &mut SkiaRenderer,
        data: &crate::plots::QuiverPlotData,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        default_color: Color,
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        if data.arrows.is_empty() {
            return Ok(());
        }

        let base_color = data.config.color.map_or(default_color, |color| {
            color.with_alpha((f32::from(color.a) / 255.0) * alpha)
        });
        let cmap = data.config.color_by_magnitude.then(|| {
            crate::render::ColorMap::by_name(&data.config.cmap)
                .unwrap_or_else(crate::render::ColorMap::viridis)
        });
        let (min_mag, max_mag) = data.magnitude_range;
        let mag_range = if (max_mag - min_mag).abs() < 1e-10 {
            1.0
        } else {
            max_mag - min_mag
        };
        let arrow_width = self
            .render_scale()
            .points_to_pixels(line_width.unwrap_or(data.config.width));

        for arrow in &data.arrows {
            let arrow_color = cmap
                .as_ref()
                .map(|colormap| {
                    colormap
                        .sample((arrow.magnitude - min_mag) / mag_range)
                        .with_alpha(alpha)
                })
                .unwrap_or(base_color);
            // An arrow is one shape built from several samples: if any vertex
            // has no position on these axes, the whole arrow is dropped rather
            // than drawn with a garbage vertex.
            let Some((sx1, sy1)) = crate::render::skia::try_map_data_to_pixels_scaled(
                arrow.start.0,
                arrow.start.1,
                x_min,
                x_max,
                y_min,
                y_max,
                plot_area,
                &self.layout.x_scale,
                &self.layout.y_scale,
            ) else {
                continue;
            };
            let Some((sx2, sy2)) = crate::render::skia::try_map_data_to_pixels_scaled(
                arrow.end.0,
                arrow.end.1,
                x_min,
                x_max,
                y_min,
                y_max,
                plot_area,
                &self.layout.x_scale,
                &self.layout.y_scale,
            ) else {
                continue;
            };
            let head: Option<Vec<(f32, f32)>> = arrow
                .head
                .iter()
                .map(|&(x, y)| {
                    crate::render::skia::try_map_data_to_pixels_scaled(
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
                })
                .collect();
            let Some(head) = head else {
                continue;
            };
            renderer.draw_line(
                sx1,
                sy1,
                sx2,
                sy2,
                arrow_color,
                arrow_width,
                LineStyle::Solid,
            )?;

            renderer.draw_filled_polygon(&head, arrow_color)?;
        }

        Ok(())
    }

    /// Helper method to render a single series using normal (non-DataShader) rendering
    pub(super) fn render_series_normal(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<()> {
        let base_color = series.props.color.value_or(Color::from_rgb(0, 0, 0));
        let alpha = series.props.alpha.value_or(1.0);
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0)); // Default black
        let line_width = self.dpi_scaled_line_width(
            series
                .props
                .line_width
                .value_or(self.display.config.lines.data_width),
        );
        let line_style = series.props.line_style.value_or(LineStyle::Solid);
        let clip_rect = clip_rect_from_plot_area(plot_area);

        if let Some(raster_plan) = self.build_prepared_series_raster_plan(
            series, resolved, plot_area, x_min, x_max, y_min, y_max, mode,
        )? {
            raster_plan.execute(renderer)?;
            self.render_series_overlays_after_raster(
                series,
                resolved,
                renderer,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
                color,
                line_width,
                &line_style,
            )?;
            return Ok(());
        }

        match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { .. })
            | (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { .. }) => unreachable!(
                "cacheable line/scatter series should return before fallback rendering"
            ),
            (SeriesType::Bar { config, .. }, ResolvedSeries::Bar { values, .. }) => {
                // The edge is configured in points, so it survives a DPI change.
                // `edge_color: None` derives it by darkening the fill.
                let edge = config.resolved_edge(&self.display.theme, color);

                for (i, &value) in values.iter().enumerate() {
                    let (bx, by, bw, bh) = bar_pixel_rect(
                        i,
                        value,
                        config.width,
                        plot_area,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        &self.layout.y_scale,
                    );
                    renderer.draw_rectangle_styled_clipped(
                        bx,
                        by,
                        bw,
                        bh,
                        Some(color),
                        edge,
                        clip_rect,
                    )?;
                }
            }
            (SeriesType::Histogram { .. }, ResolvedSeries::Histogram { data: hist_data }) => {
                // Histogram bars are adjacent, so the bin boundaries only read as
                // boundaries if the bars carry an edge. Resolve it from the series
                // data (the same source the standalone `HistogramData` renderer
                // uses) instead of relying on a primitive-level implicit border.
                let edge = hist_data.resolved_edge(&self.display.theme, color);

                // Render histogram bars
                for (i, &count) in hist_data.counts.iter().enumerate() {
                    if count > 0.0 {
                        let (bx, by, bw, bh) = histogram_bar_pixel_rect(
                            hist_data.bin_edges[i],
                            hist_data.bin_edges[i + 1],
                            count,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );

                        renderer.draw_rectangle_styled_clipped(
                            bx,
                            by,
                            bw,
                            bh,
                            Some(color),
                            edge,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::BoxPlot { .. }, ResolvedSeries::BoxPlot { data, config }) => {
                // Calculate box plot statistics
                let box_data = crate::plots::boxplot::calculate_box_plot(&data.as_ref(), config)
                    .map_err(|e| {
                        PlottingError::RenderError(format!("Box plot calculation failed: {}", e))
                    })?;

                // Every pixel below comes from the shared projection, so the
                // raster, SVG and parallel backends cannot place the same box
                // differently — and the value axis follows `yscale`.
                let BoxPlotPixels {
                    x_center: x_center_px,
                    box_left,
                    box_right,
                    cap_half_width,
                    q1_y,
                    median_y,
                    q3_y,
                    lower_whisker_y,
                    upper_whisker_y,
                } = BoxPlotPixels::new(
                    &box_data,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.y_scale,
                );

                // Draw the box (IQR) - ensure positive dimensions
                let box_width = box_right - box_left;
                let box_height = (q1_y - q3_y).abs(); // Ensure positive height
                let box_top = q3_y.min(q1_y); // Use the smaller y value as top

                // Validate dimensions before drawing
                let edge_color = box_data.edge_color.unwrap_or(color);
                let whisker_width_px = box_data
                    .whisker_width
                    .map(|w| self.render_scale().points_to_pixels(w))
                    .unwrap_or(line_width);
                let median_width_px = box_data
                    .median_width
                    .map(|w| self.render_scale().points_to_pixels(w))
                    .unwrap_or(line_width * 1.5);

                if box_width > 0.0
                    && box_height > 0.0
                    && box_width.is_finite()
                    && box_height.is_finite()
                {
                    renderer.draw_rectangle_styled_clipped(
                        box_left,
                        box_top,
                        box_width,
                        box_height,
                        Some(color.with_alpha(box_data.fill_alpha * alpha)),
                        Some((edge_color, box_data.edge_width)),
                        clip_rect,
                    )?;
                }

                // Draw median line - validate coordinates
                if box_left.is_finite() && median_y.is_finite() && box_right.is_finite() {
                    renderer.draw_line_clipped(
                        box_left,
                        median_y,
                        box_right,
                        median_y,
                        edge_color,
                        median_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw lower whisker - validate coordinates
                if x_center_px.is_finite() && q1_y.is_finite() && lower_whisker_y.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px,
                        q1_y,
                        x_center_px,
                        lower_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw upper whisker - validate coordinates
                if x_center_px.is_finite() && q3_y.is_finite() && upper_whisker_y.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px,
                        q3_y,
                        x_center_px,
                        upper_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw whisker caps - validate coordinates
                if x_center_px.is_finite()
                    && lower_whisker_y.is_finite()
                    && cap_half_width.is_finite()
                {
                    renderer.draw_line_clipped(
                        x_center_px - cap_half_width,
                        lower_whisker_y,
                        x_center_px + cap_half_width,
                        lower_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                if x_center_px.is_finite()
                    && upper_whisker_y.is_finite()
                    && cap_half_width.is_finite()
                {
                    renderer.draw_line_clipped(
                        x_center_px - cap_half_width,
                        upper_whisker_y,
                        x_center_px + cap_half_width,
                        upper_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw outliers - validate coordinates
                let outlier_marker_size = self.render_scale().points_to_pixels(box_data.flier_size);
                let outliers: &[f64] = if box_data.show_outliers {
                    &box_data.outliers
                } else {
                    &[]
                };
                for &outlier in outliers {
                    let outlier_y =
                        box_plot_value_y(outlier, plot_area, y_min, y_max, &self.layout.y_scale);
                    if x_center_px.is_finite() && outlier_y.is_finite() {
                        renderer.draw_marker_clipped(
                            x_center_px,
                            outlier_y,
                            outlier_marker_size,
                            MarkerStyle::Circle,
                            color,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.draw_cells_batch(renderer, &heatmap_plot_area, data.config.alpha * alpha)?;
                self.render_series_overlays_after_raster(
                    series,
                    resolved,
                    renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    color,
                    line_width,
                    &line_style,
                )?;
            }
            (SeriesType::ErrorBars { .. }, ResolvedSeries::ErrorBars { x, y, y_errors }) => {
                // Draw markers at data points
                let marker_size =
                    self.dpi_scaled_line_width(series.props.marker_size.value_or(8.0));
                let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
                let marker_edge = self.resolved_marker_edge(series, color);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    // `is_finite` was only half the rule: a zero or negative
                    // sample is finite but has no position on a log axis, and
                    // would have been drawn on the spine.
                    if let Some((px, py)) = crate::render::skia::try_map_data_to_pixels_scaled(
                        x_value,
                        y_value,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    ) {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
                            clip_rect,
                        )?;
                    }
                }

                // Draw Y error bars
                Self::render_attached_error_bars(
                    renderer,
                    x,
                    y,
                    Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                    series.x_errors.as_ref().map(ErrorValuesRef::from),
                    series.error_config.as_ref(),
                    color,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    line_width,
                    self.render_scale(),
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                )?;
            }
            (
                SeriesType::ErrorBarsXY { .. },
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                },
            ) => {
                // Draw markers at data points
                let marker_size =
                    self.dpi_scaled_line_width(series.props.marker_size.value_or(8.0));
                let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
                let marker_edge = self.resolved_marker_edge(series, color);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    // `is_finite` was only half the rule: a zero or negative
                    // sample is finite but has no position on a log axis, and
                    // would have been drawn on the spine.
                    if let Some((px, py)) = crate::render::skia::try_map_data_to_pixels_scaled(
                        x_value,
                        y_value,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    ) {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
                            clip_rect,
                        )?;
                    }
                }

                // Draw X and Y error bars
                Self::render_attached_error_bars(
                    renderer,
                    x,
                    y,
                    Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                    Some(effective_error_values(series.x_errors.as_ref(), x_errors)),
                    series.error_config.as_ref(),
                    color,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    line_width,
                    self.render_scale(),
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                )?;
            }
            (SeriesType::Computed { data }, ResolvedSeries::Other(_)) => {
                // Every compute-only plot type draws through this one arm, the
                // same way KDE does below. Adding a plot type therefore cannot
                // add a raster path without an SVG path — there is nothing
                // per-type to add.
                let plot_area_rect = plot_area;
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
                if let Some(request) = data.colorbar(&self.display.theme) {
                    let (x, y, width, height) = self.colorbar_rect(plot_area_rect);
                    crate::render::colorbar::draw_colorbar(
                        renderer,
                        &request.spec_at(x, y, width, height, self.display.theme.foreground),
                    )?;
                }
            }
            (SeriesType::Kde { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render KDE
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
            }
            (SeriesType::Ecdf { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render ECDF
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
            }
            (SeriesType::Violin { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Violin
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
            }
            (SeriesType::Boxen { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Boxen
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
            }
            (SeriesType::Quiver { data }, ResolvedSeries::Other(_)) => {
                self.render_quiver_series_scaled(
                    renderer,
                    data,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;

                self.draw_series_colorbar(renderer, &series.series_type, plot_area)?;
            }
            (SeriesType::Contour { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Contour
                let contour_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &contour_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;

                self.draw_series_colorbar(renderer, &series.series_type, plot_area)?;
            }
            (SeriesType::Pie { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Pie with 1:1 aspect ratio
                // (uses normalized 0-1 coordinates)
                let (pie_x, pie_y, pie_size) = {
                    let size = plot_area.width().min(plot_area.height());
                    let x_offset = (plot_area.width() - size) / 2.0;
                    let y_offset = (plot_area.height() - size) / 2.0;
                    (plot_area.x() + x_offset, plot_area.y() + y_offset, size)
                };
                let pie_plot_area = crate::plots::PlotArea::new(
                    pie_x, pie_y, pie_size, pie_size, 0.0, 1.0, 0.0, 1.0,
                );
                data.render_styled(
                    renderer,
                    &pie_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                )?;
            }
            (SeriesType::Radar { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Radar with 1:1 aspect ratio
                // and extra top padding for title clearance
                let radar_plot_area = Self::radar_plot_area(plot_area, x_min, x_max, y_min, y_max);
                let mut radar_theme = self.display.theme.clone();
                if let Some(colors) = &series.resolved_radar_colors {
                    radar_theme.color_palette = colors.to_vec();
                }
                data.render_styled_with_grid(
                    renderer,
                    &radar_plot_area,
                    &radar_theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                    Some(&self.layout.grid_style),
                )?;
            }
            (SeriesType::Polar { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Polar with 1:1 aspect ratio
                // Center the square plot area within available space
                let (polar_x, polar_y, polar_size) = {
                    let size = plot_area.width().min(plot_area.height());
                    let x_offset = (plot_area.width() - size) / 2.0;
                    let y_offset = (plot_area.height() - size) / 2.0;
                    (plot_area.x() + x_offset, plot_area.y() + y_offset, size)
                };
                let polar_plot_area = crate::plots::PlotArea::new(
                    polar_x, polar_y, polar_size, polar_size, x_min, x_max, y_min, y_max,
                );
                // Same call shape the radar arm above uses, so `.grid(false)`
                // and custom grid styling reach both radial plot types.
                data.render_styled_with_grid(
                    renderer,
                    &polar_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.props.line_width.cloned(),
                    Some(&self.layout.grid_style),
                )?;
            }
            _ => unreachable!("resolved series variant must match its declarative series"),
        }

        Ok(())
    }

    /// Render a series using GPU-accelerated coordinate transformation
    ///
    /// Uses GPU compute shaders for coordinate transformation when available,
    /// falling back to CPU for the actual drawing operations.
    #[cfg(feature = "gpu")]
    pub(super) fn render_series_gpu(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        gpu_renderer: &mut GpuRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<()> {
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0));
        let line_width = self.dpi_scaled_line_width(
            series
                .props
                .line_width
                .value_or(self.display.config.lines.data_width),
        );
        let line_style = series.props.line_style.value_or(LineStyle::Solid);
        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

        match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x.as_ref(),
                        &y.as_ref(),
                        (x_min, x_max),
                        (y_min, y_max),
                        viewport,
                    )
                    .map_err(|e| {
                        PlottingError::RenderError(format!("GPU transform failed: {}", e))
                    })?;

                // Convert to points for drawing
                let points: Vec<(f32, f32)> = x_transformed
                    .iter()
                    .zip(y_transformed.iter())
                    .map(|(&x, &y)| (x, y))
                    .collect();

                renderer
                    .draw_polyline_clipped(&points, color, line_width, line_style, clip_rect)?;
                if let Some(marker_style) = series.props.marker_style.cloned() {
                    let marker_size =
                        self.dpi_scaled_line_width(series.props.marker_size.value_or(8.0));
                    let marker_edge = self.resolved_marker_edge(series, color);
                    for &(px, py) in &points {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                // Density aggregation has no GPU implementation; erroring
                // matches SVG export rather than silently rendering the
                // per-marker output the series opted out of.
                if series.density == Some(true) {
                    return Err(PlottingError::RenderError(
                        "the GPU backend does not support density scatter series; \
                         render through the default backend or disable density mode"
                            .to_string(),
                    ));
                }
                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x.as_ref(),
                        &y.as_ref(),
                        (x_min, x_max),
                        (y_min, y_max),
                        viewport,
                    )
                    .map_err(|e| {
                        PlottingError::RenderError(format!("GPU transform failed: {}", e))
                    })?;

                let marker_size =
                    self.dpi_scaled_line_width(series.props.marker_size.value_or(10.0));
                let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
                let marker_edge = self.resolved_marker_edge(series, color);

                // Draw markers at transformed coordinates
                for (&px, &py) in x_transformed.iter().zip(y_transformed.iter()) {
                    renderer.draw_marker_styled_clipped(
                        px,
                        py,
                        marker_size,
                        marker_style,
                        color,
                        marker_edge,
                        clip_rect,
                    )?;
                }
            }
            // For other series types, fall back to normal rendering
            _ => {
                self.render_series_normal(
                    series, resolved, renderer, plot_area, x_min, x_max, y_min, y_max, mode,
                )?;
            }
        }

        Ok(())
    }

    pub(super) fn validate_series_list(series_list: &[PlotSeries]) -> Result<()> {
        if series_list.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        for (idx, series) in series_list.iter().enumerate() {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    if x_data.len() != y_data.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    if x_data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                }
                SeriesType::Bar {
                    categories, values, ..
                } => {
                    let values = values.resolve_cow(0.0);
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                            series_index: Some(idx),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&values)?;
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    if x_data.len() != y_data.len() || y_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                    PlottingError::validate_data(&y_errors)?;
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
                    if x_data.len() != y_data.len()
                        || x_data.len() != x_errors.len()
                        || x_data.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                    PlottingError::validate_data(&x_errors)?;
                    PlottingError::validate_data(&y_errors)?;
                }
                SeriesType::Histogram { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&data)?;
                }
                SeriesType::BoxPlot { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&data)?;
                }
                SeriesType::Heatmap { data } => {
                    if data.values.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Kde { data } => {
                    if data.x.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Ecdf { data } => {
                    if data.x.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Violin { data } => {
                    if data.data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Boxen { data } => {
                    if data.boxes.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Quiver { data } => {
                    if data.arrows.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    for (index, arrow) in data.arrows.iter().enumerate() {
                        let all_values = [
                            arrow.start.0,
                            arrow.start.1,
                            arrow.end.0,
                            arrow.end.1,
                            arrow.magnitude,
                            arrow.angle,
                            arrow.head[0].0,
                            arrow.head[0].1,
                            arrow.head[1].0,
                            arrow.head[1].1,
                            arrow.head[2].0,
                            arrow.head[2].1,
                        ];
                        if let Some(value) = all_values.iter().find(|value| !value.is_finite()) {
                            return Err(PlottingError::InvalidData {
                                message: format!("Non-finite quiver arrow value ({value}) found"),
                                position: Some(index),
                            });
                        }
                    }
                }
                SeriesType::Contour { data } => {
                    if data.levels.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Pie { data } => {
                    if data.values.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Radar { data } => {
                    if data.series.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Polar { data } => {
                    if data.points.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Computed { data } => {
                    if crate::plots::traits::PlotData::is_empty(data.as_ref()) {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) fn validate_resolved_series(
        &self,
        series_list: &[ResolvedSeries<'_>],
    ) -> Result<()> {
        if series_list.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        for (idx, series) in series_list.iter().enumerate() {
            match series {
                ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y } => {
                    if x.len() != y.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    let is_streaming = self.series_mgr.series.get(idx).is_some_and(|series| {
                        matches!(
                            &series.series_type,
                            SeriesType::Line { x_data, y_data }
                                | SeriesType::Scatter { x_data, y_data }
                                if matches!(x_data, PlotData::Streaming(_))
                                    && matches!(y_data, PlotData::Streaming(_))
                        )
                    });
                    if x.is_empty() && !is_streaming {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                }
                ResolvedSeries::Bar { categories, values } => {
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                            series_index: Some(idx),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(values)?;
                }
                ResolvedSeries::ErrorBars { x, y, y_errors } => {
                    if x.len() != y.len() || y.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                    PlottingError::validate_data(y_errors)?;
                }
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                } => {
                    if x.len() != y.len() || x.len() != x_errors.len() || x.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                    PlottingError::validate_data(x_errors)?;
                    PlottingError::validate_data(y_errors)?;
                }
                ResolvedSeries::Histogram { data } => {
                    if data.counts.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                ResolvedSeries::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(data)?;
                }
                ResolvedSeries::Other(series) => match series {
                    SeriesType::Heatmap { data } if data.values.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Kde { data } if data.x.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Ecdf { data } if data.x.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Violin { data } if data.data.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Boxen { data } if data.boxes.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Quiver { data } if data.arrows.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Contour { data } if data.levels.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Pie { data } if data.values.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Radar { data } if data.series.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Polar { data } if data.points.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Computed { data }
                        if crate::plots::traits::PlotData::is_empty(data.as_ref()) =>
                    {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Quiver { data } => {
                        for (position, arrow) in data.arrows.iter().enumerate() {
                            let all_values = [
                                arrow.start.0,
                                arrow.start.1,
                                arrow.end.0,
                                arrow.end.1,
                                arrow.magnitude,
                                arrow.angle,
                                arrow.head[0].0,
                                arrow.head[0].1,
                                arrow.head[1].0,
                                arrow.head[1].1,
                                arrow.head[2].0,
                                arrow.head[2].1,
                            ];
                            if let Some(value) = all_values.iter().find(|value| !value.is_finite())
                            {
                                return Err(PlottingError::InvalidData {
                                    message: format!(
                                        "Non-finite quiver arrow value ({value}) found"
                                    ),
                                    position: Some(position),
                                });
                            }
                        }
                    }
                    _ => {}
                },
            }
        }

        Ok(())
    }

    /// The axis scales a series' geometry can honour, plus the plot-type name
    /// to quote when refusing one.
    ///
    /// Returns `(plot type, x support, y support)`.
    ///
    /// The match is deliberately exhaustive with no wildcard arm: a new
    /// `SeriesType` variant does not compile until it declares its support
    /// here, so a new plot type cannot silently inherit "draw linear geometry
    /// under whatever axis the user configured". The rule is the one documented
    /// on [`AxisScaleSupport`]: `Scaled` exactly when the geometry is projected
    /// through the axis scale (`map_data_to_pixels_scaled`, or `PlotArea`,
    /// which performs the same transform), `Unsupported` for geometry placed at
    /// an ordinal slot or in a synthetic cell rather than at a data value, and
    /// `Independent` for plot types that draw in their own coordinate system.
    pub(super) fn series_axis_scale_support(
        series: &SeriesType,
    ) -> (&'static str, AxisScaleSupport, AxisScaleSupport) {
        // Category axis: positions are slots, so there is no quantity to take
        // a logarithm of. The wording lives on `AxisScaleSupport` so that every
        // categorical plot type — including the ones reached through
        // `SeriesType::Computed` — refuses in the same sentence.
        const ORDINAL: AxisScaleSupport = AxisScaleSupport::ORDINAL;
        const SCALED: AxisScaleSupport = AxisScaleSupport::Scaled;
        const OWN_COORDS: AxisScaleSupport = AxisScaleSupport::Independent;

        // A distribution plot puts its category on one axis and its values on
        // the other; which is which follows the series' own orientation.
        let across_and_along = |vertical: bool| {
            if vertical {
                (ORDINAL, SCALED)
            } else {
                (SCALED, ORDINAL)
            }
        };

        match series {
            SeriesType::Line { .. } => ("line", SCALED, SCALED),
            SeriesType::Scatter { .. } => ("scatter", SCALED, SCALED),
            SeriesType::Bar { .. } => ("bar", ORDINAL, SCALED),
            SeriesType::ErrorBars { .. } => ("errorbar", SCALED, SCALED),
            SeriesType::ErrorBarsXY { .. } => ("errorbar", SCALED, SCALED),
            SeriesType::Histogram { .. } => ("histogram", SCALED, SCALED),
            // Always vertical: `BoxPlotPixels` draws the box across the x axis
            // whatever `orientation` says, so declaring x as `Scaled` for a
            // horizontal box plot would promise a projection nothing performs.
            SeriesType::BoxPlot { .. } => ("boxplot", ORDINAL, SCALED),
            SeriesType::Quiver { .. } => ("quiver", SCALED, SCALED),
            SeriesType::Heatmap { .. } => ("heatmap", SCALED, SCALED),
            SeriesType::Kde { .. } => ("kde", SCALED, SCALED),
            SeriesType::Ecdf { .. } => ("ecdf", SCALED, SCALED),
            SeriesType::Contour { .. } => ("contour", SCALED, SCALED),
            SeriesType::Violin { data } => {
                let (x, y) = across_and_along(matches!(
                    data.config.orientation,
                    crate::plots::distribution::Orientation::Vertical
                ));
                ("violin", x, y)
            }
            SeriesType::Boxen { data } => {
                let (x, y) = across_and_along(matches!(
                    data.config.orient,
                    crate::plots::distribution::BoxenOrientation::Vertical
                ));
                ("boxen", x, y)
            }
            SeriesType::Pie { .. } => ("pie", OWN_COORDS, OWN_COORDS),
            SeriesType::Radar { .. } => ("radar", OWN_COORDS, OWN_COORDS),
            SeriesType::Polar { .. } => ("polar", OWN_COORDS, OWN_COORDS),
            // A `ComputedSeries` answers for itself, in the same vocabulary:
            // `AxisScaleSupport::ORDINAL` is the constant this arm's `ORDINAL`
            // aliases, so a compute-only categorical plot type refuses a log
            // axis in exactly the words a bar chart uses.
            SeriesType::Computed { data } => {
                let (x, y) = data.axis_scale_support();
                (data.kind(), x, y)
            }
        }
    }

    /// Refuse a figure whose own geometry cannot honour the axis scale it was
    /// given.
    ///
    /// The axis line, its ticks and its tick labels are always drawn
    /// scale-aware. A series that lays its geometry out linearly underneath a
    /// log-labelled axis therefore produces a quantitatively wrong figure and
    /// says nothing about it — so it is refused here instead of drawn.
    ///
    /// The raster, parallel and SVG paths all reach this through
    /// `validate_runtime_environment`, so no backend can accept a
    /// combination another one rejects.
    pub(super) fn validate_axis_scales(&self) -> Result<()> {
        let x_scale = &self.layout.x_scale;
        let y_scale = &self.layout.y_scale;
        if matches!(x_scale, crate::axes::AxisScale::Linear)
            && matches!(y_scale, crate::axes::AxisScale::Linear)
        {
            return Ok(());
        }

        for series in &self.series_mgr.series {
            let (plot_type, x_support, y_support) =
                Self::series_axis_scale_support(&series.series_type);

            for (axis, setter, support, scale) in [
                ("x", "xscale", x_support, x_scale),
                ("y", "yscale", y_support, y_scale),
            ] {
                if support.accepts(scale) {
                    continue;
                }

                let reason = support
                    .rejection_reason()
                    .unwrap_or("this plot type cannot honour a non-linear scale");
                return Err(PlottingError::InvalidInput(format!(
                    "`{plot_type}` cannot be drawn on a non-linear {axis} axis: {reason}. \
                     Remove `.{setter}(..)`, or plot this data with a series type whose \
                     {axis} geometry follows the axis scale (line, scatter, histogram, errorbar)."
                )));
            }
        }

        Ok(())
    }

    /// Internal validation logic for series data
    pub(super) fn validate_series(&self) -> Result<()> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        Self::validate_series_list(&self.series_mgr.series)
    }

    pub(super) fn validate_runtime_environment(&self) -> Result<()> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        self.validate_output_config()?;
        self.validate_annotations()?;
        self.validate_axis_scales()?;
        Ok(())
    }

    pub(super) fn validate_runtime_inputs_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<()> {
        self.validate_runtime_environment()?;
        Self::validate_series_list(series_list)
    }

    pub(super) fn validate_annotations(&self) -> Result<()> {
        for annotation in &self.annotations {
            if let Annotation::FillBetween { x, y1, y2, .. } = annotation {
                if x.len() != y1.len() || x.len() != y2.len() {
                    return Err(PlottingError::DataLengthMismatch {
                        x_len: x.len(),
                        y_len: y1.len().max(y2.len()),
                        series_index: None,
                    });
                }
                PlottingError::validate_data(x)?;
                PlottingError::validate_data(y1)?;
                PlottingError::validate_data(y2)?;
            }
        }

        Ok(())
    }

    pub(super) fn validate_output_config(&self) -> Result<()> {
        let figure = &self.display.config.figure;
        if !figure.dpi.is_finite() {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be a finite value (dpi={})",
                figure.dpi
            )));
        }
        if figure.dpi <= 0.0 {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be positive (dpi={})",
                figure.dpi
            )));
        }
        if figure.dpi < crate::core::constants::dpi::MIN as f32 && !self.render.allow_subminimum_dpi
        {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be at least {} (dpi={})",
                crate::core::constants::dpi::MIN,
                figure.dpi
            )));
        }
        if figure.dpi > crate::core::constants::dpi::MAX as f32 {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "DPI".to_string(),
                actual: figure.dpi.ceil() as usize,
                maximum: crate::core::constants::dpi::MAX as usize,
            });
        }
        if !figure.width.is_finite() || !figure.height.is_finite() {
            return Err(PlottingError::InvalidInput(format!(
                "Figure width/height must be finite values (width={}, height={})",
                figure.width, figure.height
            )));
        }
        let (width, height) = self.config_canvas_size();
        if self.render.allow_subplot_dimensions {
            PlottingError::validate_subplot_dimensions(width, height)?;
        } else {
            PlottingError::validate_dimensions(width, height)?;
        }
        self.display.config.margins.validate_for_figure(figure)?;
        Ok(())
    }

    /// Where a colorbar sits relative to the plot area: `(x, y, width, height)`.
    ///
    /// The layout pass reserves this strip; stating it in one place is what
    /// keeps the raster and SVG backends drawing it in the same rectangle.
    pub(super) fn colorbar_rect(&self, plot_area: tiny_skia::Rect) -> (f32, f32, f32, f32) {
        let render_scale = self.render_scale();
        let margin = render_scale.logical_pixels_to_pixels(COLORBAR_MARGIN_PX);
        let width = render_scale.logical_pixels_to_pixels(COLORBAR_WIDTH_PX);
        (
            plot_area.right() + margin,
            plot_area.y(),
            width,
            plot_area.height(),
        )
    }

    /// The colorbar any one series asks for, or `None` if it has no colour scale.
    ///
    /// The single answer to "what does this series' colorbar say?". Margin
    /// reservation ([`Plot::colorbar_measurement_spec`]), the raster draw and the
    /// SVG draw all call it, so a colorbar cannot be measured with one range and
    /// drawn with another, and a new plot type with a colour scale becomes
    /// readable in both backends at once.
    pub(super) fn series_colorbar_request(
        &self,
        series_type: &SeriesType,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        let theme = &self.display.theme;
        match series_type {
            SeriesType::Heatmap { data } => Self::heatmap_colorbar_request(data, theme),
            SeriesType::Contour { data } => Self::contour_colorbar_request(data, theme),
            SeriesType::Quiver { data } => data.colorbar(theme),
            SeriesType::Computed { data } => data.colorbar(theme),
            _ => None,
        }
    }

    /// The colorbar a heatmap asks for, or `None` when it has one turned off.
    ///
    /// The font sizes come from the one resolver every colorbar uses, so an
    /// unconfigured colorbar tracks the figure's theme instead of the 12/14 pt
    /// literals this used to carry.
    pub(super) fn heatmap_colorbar_request(
        data: &crate::plots::HeatmapData,
        theme: &crate::render::Theme,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        let fonts = data.config.colorbar_font_sizes(theme);
        data.config
            .colorbar
            .then(|| crate::render::colorbar::ColorbarRequest {
                colormap: data.config.colormap.clone(),
                vmin: data.vmin,
                vmax: data.vmax,
                value_scale: data.config.value_scale,
                label: data.config.colorbar_label.clone(),
                tick_font_size: fonts.tick,
                label_font_size: fonts.label,
                show_log_subticks: data.config.colorbar_log_subticks,
            })
    }

    /// The colorbar a contour asks for, or `None` when it has one turned off.
    ///
    /// A contour's range is the span of its levels, and its ramp comes from the
    /// `cmap` name; both are resolved here so the two backends cannot label the
    /// same figure differently.
    pub(super) fn contour_colorbar_request(
        data: &crate::plots::ContourPlotData,
        theme: &crate::render::Theme,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        if !data.config.colorbar {
            return None;
        }
        let fonts = data.config.colorbar_font_sizes(theme);
        let (vmin, vmax) = match (data.levels.first(), data.levels.last()) {
            (Some(&first), Some(&last)) => (first, last),
            _ => (0.0, 1.0),
        };
        Some(crate::render::colorbar::ColorbarRequest {
            colormap: crate::render::ColorMap::by_name(&data.config.cmap)
                .unwrap_or_else(crate::render::ColorMap::viridis),
            vmin,
            vmax,
            value_scale: crate::axes::AxisScale::Linear,
            label: data.config.colorbar_label.clone(),
            tick_font_size: fonts.tick,
            label_font_size: fonts.label,
            show_log_subticks: false,
        })
    }

    pub(super) fn validate_runtime_inputs(&self) -> Result<()> {
        self.validate_runtime_inputs_for_series(&self.series_mgr.series)
    }
}

/// Pixel y of the value-axis baseline a bar or histogram bin is drawn from.
///
/// Normally this is `0.0` projected through `y_scale`. A log axis has no
/// position for zero, and since a previous batch made that projection return
/// `NaN` rather than silently pretending zero sits on the spine, projecting it
/// blindly would make every bar's rectangle `NaN`. The bar instead bottoms out
/// on the axis floor, which is what the axis actually shows and what matplotlib
/// draws.
///
/// One helper so that the bar, histogram and parallel paths cannot each pick a
/// different answer for "where is zero on a log axis?". It is the rect-shaped
/// spelling of [`crate::plots::PlotArea::fill_baseline_y`], which the KDE and
/// area fills use, so bars and fills bottom out in the same place.
pub(super) fn value_axis_baseline_y(
    plot_area: tiny_skia::Rect,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> f32 {
    plot_area_from_rect(
        plot_area,
        0.0,
        1.0,
        y_min,
        y_max,
        &crate::axes::AxisScale::Linear,
        y_scale,
    )
    .fill_baseline_y()
}

/// Pixel rectangle `(x, y, width, height)` for one bar of a categorical series.
///
/// Categories sit one data unit apart, so a bar's width is `width_fraction` of a
/// unit measured through the same x mapping that places the bar centres, and its
/// body spans from the value to the zero baseline.
///
/// The value axis is projected through `y_scale`, so a bar chart on a log y axis
/// is drawn where its log-labelled ticks say it is. The category axis is always
/// linear: category positions are ordinals, and
/// `Plot::series_axis_scale_support` refuses a non-linear scale on it rather
/// than inventing a spacing for it.
///
/// The baseline is `0.0` mapped through the same projection, so on a log axis —
/// where zero has no position — the bar bottoms out at the axis floor, matching
/// the parallel backend and matplotlib.
///
/// Both the raster backend and the SVG backend call this, so the same bar chart
/// cannot land in a different place depending on the output format.
pub(super) fn bar_pixel_rect(
    index: usize,
    value: f64,
    width_fraction: f32,
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32, f32, f32) {
    let data_range = (x_max - x_min) as f32;
    let bar_width = width_fraction * (plot_area.width() / data_range);
    let x = index as f64;
    let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
        x,
        value,
        x_min,
        x_max,
        y_min,
        y_max,
        plot_area,
        &crate::axes::AxisScale::Linear,
        y_scale,
    );
    let py_zero = value_axis_baseline_y(plot_area, y_min, y_max, y_scale);
    (
        px - bar_width / 2.0,
        py.min(py_zero),
        bar_width,
        (py - py_zero).abs(),
    )
}

/// Pixel rectangle `(x, y, width, height)` for one histogram bin.
///
/// The bin is placed from its two mapped edges rather than from a mapped centre
/// plus half a width: on a non-linear x axis a bin is not symmetric about its
/// centre, and centre-based placement would leave gaps and overlaps between
/// adjacent bins.
///
/// Both axes are projected through their configured [`crate::axes::AxisScale`],
/// so the bars agree with the ticks drawn beside them.
///
/// The raster and SVG backends both call this, so a histogram cannot land in a
/// different place depending on the output format.
pub(super) fn histogram_bar_pixel_rect(
    bin_left: f64,
    bin_right: f64,
    count: f64,
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32, f32, f32) {
    let (px_left, py) = crate::render::skia::map_data_to_pixels_scaled(
        bin_left, count, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );
    let (px_right, _) = crate::render::skia::map_data_to_pixels_scaled(
        bin_right, count, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );
    let py_zero = value_axis_baseline_y(plot_area, y_min, y_max, y_scale);
    (
        px_left.min(px_right),
        py.min(py_zero),
        (px_right - px_left).abs(),
        (py - py_zero).abs(),
    )
}

// ===========================================================================
// The categorical x axis
// ===========================================================================

/// Give a plot type a place on the categorical x axis.
///
/// Emits, for one config type, the *only* two knobs a categorical series has —
/// `.category(..)` and `.x_position(..)` — on both the config itself and on
/// `PlotBuilder`, plus the `x_center()` accessor every render path reads.
///
/// Writing them once is the point. Before this, `.violin(..)` had a
/// `.category(..)` that only reached the tick labels, box plots had no category
/// at all and drew against a meaningless 0..1 numeric axis, and each renderer
/// picked its own "centre of the plot". A new categorical plot type now joins
/// the axis by adding one line to the invocation below, and it cannot join it
/// halfway.
macro_rules! impl_category_axis {
    ($($config:path),+ $(,)?) => {$(
        impl $config {
            /// Label this series with a category, shown under it on the x axis.
            ///
            /// Repeating a category name places this series in the slot that
            /// name already has; a new name claims the next slot to the right,
            /// exactly as an extra bar would.
            pub fn category<S: Into<String>>(mut self, name: S) -> Self {
                self.category = Some(name.into());
                self
            }

            /// Pin this series to an explicit centre on the category axis.
            ///
            /// The escape hatch for layouts [`category`](Self::category()) cannot
            /// express, such as two half-violins sharing one slot. Positions are
            /// in slot units: slot *i* is centred on `i` and one unit wide.
            pub fn x_position(mut self, position: f64) -> Self {
                self.x_position = Some(position);
                self
            }

            /// Centre of this series' category slot, in data units.
            ///
            /// `None` reads as slot 0 so that a series inspected before it was
            /// added to a plot still has a well-defined position.
            pub(crate) fn x_center(&self) -> f64 {
                self.x_position.unwrap_or(0.0)
            }
        }

        impl PlotBuilder<$config> {
            /// Label this series with a category, shown under it on the x axis.
            ///
            /// Chain it like any other series setter — adding a second series
            /// with a different category lays the two out side by side, the
            /// way a grouped bar chart does.
            pub fn category<S: Into<String>>(mut self, name: S) -> Self {
                self.config = std::mem::take(&mut self.config).category(name);
                self
            }

            /// Pin this series to an explicit centre on the category axis.
            pub fn x_position(mut self, position: f64) -> Self {
                self.config = std::mem::take(&mut self.config).x_position(position);
                self
            }
        }
    )+};
}

impl_category_axis!(
    crate::plots::boxplot::BoxPlotConfig,
    crate::plots::distribution::ViolinConfig,
    crate::plots::distribution::BoxenConfig,
);

/// The category slots one series occupies, left to right.
///
/// A bar series occupies one slot per bar, always slots `0..n`, because a bar's
/// position *is* its index; a distribution series occupies exactly one slot,
/// claimed when it was added. An empty label means "this series holds a slot
/// but has nothing to write under it", which is what stops a lone
/// `.boxplot(&d)` from falling back to a numeric 0..1 axis.
///
/// Both the slot *assignment* ([`Plot::next_category_slot`]) and the slot
/// *readback* ([`CategoryAxis::harvest`]) go through here, so a series cannot
/// be positioned by one rule and labelled by another.
pub(crate) fn series_category_slots(series: &SeriesType) -> Vec<(String, f64)> {
    match series {
        SeriesType::Bar { categories, .. } => categories
            .iter()
            .enumerate()
            .map(|(index, category)| (category.clone(), index as f64))
            .collect(),
        SeriesType::BoxPlot { config, .. } => vec![(
            config.category.clone().unwrap_or_default(),
            config.x_center(),
        )],
        SeriesType::Violin { data } => vec![(
            data.config.category.clone().unwrap_or_default(),
            data.config.x_center(),
        )],
        SeriesType::Boxen { data } => vec![(
            data.config.category.clone().unwrap_or_default(),
            data.config.x_center(),
        )],
        // Compute-only types answer for themselves: strip and swarm hold one
        // slot per category name, everything else holds none.
        SeriesType::Computed { data } => data.category_slots(),
        _ => Vec::new(),
    }
}

/// How close two slot centres have to be to count as the same slot.
///
/// Slots are whole numbers unless a caller pinned one with `x_position`, so
/// this only has to absorb float round-trips, not real spacing.
const SLOT_TOLERANCE: f64 = 1e-9;

/// The categorical x axis a figure has, if any series asked for one.
///
/// One harvest for every categorical plot type. The renderer used to run two:
/// a bar-only one that assumed ordinal positions and had no way to express a
/// gap, and a violin-only one that carried positions but which no other plot
/// type could reach. Box plots reached neither, which is why a single box drew
/// against a bare 0..1 numeric axis.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CategoryAxis {
    /// Tick label for each slot, in axis order. May be empty for a slot whose
    /// series carries no category name.
    pub(crate) labels: Vec<String>,
    /// Centre of each slot, in data units, in axis order.
    pub(crate) positions: Vec<f64>,
}

impl CategoryAxis {
    /// Collect every category slot in the figure, or `None` if it has none.
    ///
    /// Slots are deduplicated by position, so two series sharing a slot (a
    /// split violin pair, say) label it once, and the first non-empty name
    /// wins.
    pub(crate) fn harvest(series: &[PlotSeries]) -> Option<Self> {
        let mut slots: Vec<(String, f64)> = Vec::new();
        for entry in series {
            for (label, position) in series_category_slots(&entry.series_type) {
                let existing = slots
                    .iter()
                    .position(|(_, taken)| (taken - position).abs() < SLOT_TOLERANCE);
                match existing {
                    Some(index) => {
                        if slots[index].0.is_empty() {
                            slots[index].0 = label;
                        }
                    }
                    None => slots.push((label, position)),
                }
            }
        }

        if slots.is_empty() {
            return None;
        }

        slots.sort_by(|(_, a), (_, b)| a.total_cmp(b));
        let (labels, positions): (Vec<String>, Vec<f64>) = slots.into_iter().unzip();
        Some(Self { labels, positions })
    }

    /// The data-space span the axis needs to show every slot in full.
    pub(crate) fn x_span(&self) -> (f64, f64) {
        let first = self.positions.first().copied().unwrap_or(0.0);
        let last = self.positions.last().copied().unwrap_or(0.0);
        (category_slot_span(first).0, category_slot_span(last).1)
    }
}

impl Plot {
    /// The category slot a series added now should occupy.
    ///
    /// Reusing a category name reuses its slot — that is what makes two series
    /// "the same category" — and anything else claims the next slot to the
    /// right of everything already placed, so several box plots, violins or
    /// boxen plots lay out side by side without the caller counting anything.
    pub(crate) fn next_category_slot(&self, category: Option<&str>) -> f64 {
        let Some(axis) = CategoryAxis::harvest(&self.series_mgr.series) else {
            return 0.0;
        };

        if let Some(name) = category.filter(|name| !name.is_empty())
            && let Some(index) = axis.labels.iter().position(|label| label.as_str() == name)
        {
            return axis.positions[index];
        }

        // The next slot's centre is half a slot past where the axis ends today.
        axis.x_span().1 + CATEGORY_SLOT_HALF_WIDTH
    }
}

/// Every pixel coordinate one box plot is drawn from.
///
/// The raster, SVG and parallel backends each used to project these five
/// quantiles themselves, and the raster copy projected them linearly while the
/// figure's ticks were drawn scale-aware — so `.boxplot(&d).yscale(Log)` put the
/// box in the wrong place. Deriving them once here is what keeps the three
/// backends from drifting apart again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BoxPlotPixels {
    /// Horizontal centre of the box, in pixels.
    pub x_center: f32,
    /// Left edge of the box body, in pixels.
    pub box_left: f32,
    /// Right edge of the box body, in pixels.
    pub box_right: f32,
    /// Half-width of a whisker cap, in pixels.
    pub cap_half_width: f32,
    /// Q1 (bottom of the box) in pixels.
    pub q1_y: f32,
    /// Median line in pixels.
    pub median_y: f32,
    /// Q3 (top of the box) in pixels.
    pub q3_y: f32,
    /// Lower whisker end in pixels.
    pub lower_whisker_y: f32,
    /// Upper whisker end in pixels.
    pub upper_whisker_y: f32,
}

impl BoxPlotPixels {
    /// Project a computed box plot into pixels.
    ///
    /// Every geometry constant comes from `box_data`, which `calculate_box_plot`
    /// resolved from the user's `BoxPlotConfig` — see the contract on
    /// `BoxPlotData`. The value axis is projected through `y_scale`; the
    /// category axis is always linear, because
    /// `Plot::series_axis_scale_support` refuses a non-linear scale on it.
    pub(super) fn new(
        box_data: &crate::plots::boxplot::BoxPlotData,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        y_scale: &crate::axes::AxisScale,
    ) -> Self {
        // The box is one `width_ratio` of its category slot across, measured in
        // data units and then projected — so several boxes on one axis keep
        // their spacing instead of each claiming a fraction of the whole panel.
        let half_width = f64::from(box_data.width_ratio) * CATEGORY_SLOT_HALF_WIDTH;
        let slot_x = |x: f64| {
            crate::render::skia::map_data_to_pixels_scaled(
                x,
                0.0,
                x_min,
                x_max,
                y_min,
                y_max,
                plot_area,
                &crate::axes::AxisScale::Linear,
                y_scale,
            )
            .0
        };
        let x_center = slot_x(box_data.x_center);
        let box_left = slot_x(box_data.x_center - half_width);
        let box_right = slot_x(box_data.x_center + half_width);

        Self {
            x_center,
            box_left,
            box_right,
            cap_half_width: (box_right - box_left) * 0.5 * box_data.cap_width,
            q1_y: box_plot_value_y(box_data.q1, plot_area, y_min, y_max, y_scale),
            median_y: box_plot_value_y(box_data.median, plot_area, y_min, y_max, y_scale),
            q3_y: box_plot_value_y(box_data.q3, plot_area, y_min, y_max, y_scale),
            lower_whisker_y: box_plot_value_y(box_data.min, plot_area, y_min, y_max, y_scale),
            upper_whisker_y: box_plot_value_y(box_data.max, plot_area, y_min, y_max, y_scale),
        }
    }
}

/// Project one box plot value onto the (scale-aware) value axis.
///
/// Outliers are projected with this too, so a flier cannot end up on a
/// different mapping from the whisker it sits beyond.
pub(super) fn box_plot_value_y(
    value: f64,
    plot_area: tiny_skia::Rect,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> f32 {
    crate::render::skia::map_data_to_pixels_scaled(
        0.0,
        value,
        0.0,
        1.0,
        y_min,
        y_max,
        plot_area,
        &crate::axes::AxisScale::Linear,
        y_scale,
    )
    .1
}

#[cfg(test)]
mod marker_edge_series_tests {
    use super::*;
    use crate::render::MarkerStyle;

    const FILL: Color = Color {
        r: 31,
        g: 119,
        b: 180,
        a: 255,
    };
    const RIM: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    fn rim_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] > 200 && px[1] < 120 && px[2] < 120)
            .count()
    }

    fn xy() -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..8).map(|i| (i % 3) as f64).collect();
        (x, y)
    }

    fn exact_fill_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] == FILL.r && px[1] == FILL.g && px[2] == FILL.b)
            .count()
    }

    fn square_scatter_plot(
        size: f32,
    ) -> impl Fn() -> crate::core::plot::PlotBuilder<crate::plots::basic::ScatterConfig> {
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..8).map(|i| (i % 3) as f64).collect();
        move || {
            Plot::new()
                .size_px(240, 180)
                .ticks(false)
                .grid(false)
                .scatter(&x, &y)
                .marker(MarkerStyle::Square)
                .marker_size(size)
                .color(FILL)
        }
    }

    #[test]
    fn test_default_scatter_marker_is_exactly_its_series_colour() {
        // A contrasting rim cannot be the default: it is stroked over the
        // marker's own boundary, so it darkens every neighbour it overlaps and
        // swallows markers of a few points whole. Default = fill and nothing
        // else, matching matplotlib's `edgecolors='face'`.
        let plot = square_scatter_plot(12.0);

        let default = plot().render().expect("default scatter should render");
        let bare = plot()
            .show_edge(false)
            .render()
            .expect("bare scatter should render");

        assert_eq!(
            default.pixels, bare.pixels,
            "a default scatter marker must render exactly as an edgeless one"
        );
        assert!(
            exact_fill_pixels(&default) > 0,
            "the marker must stay the exact requested colour, not a tint"
        );
    }

    #[test]
    fn test_opt_in_scatter_rim_replaces_boundary_fill_without_tinting_the_interior() {
        // Asking for a rim must stroke a real boundary and leave the interior
        // at the exact requested colour — never tint the whole marker.
        let plot = square_scatter_plot(12.0);

        let edged = plot()
            .show_edge(true)
            .render()
            .expect("edged scatter should render");
        let bare = plot().render().expect("default scatter should render");

        assert_ne!(
            edged.pixels, bare.pixels,
            "show_edge(true) must reach the canvas"
        );
        assert!(
            exact_fill_pixels(&edged) > 0,
            "the marker interior must stay the exact requested colour, not a tint"
        );
        assert!(
            exact_fill_pixels(&edged) < exact_fill_pixels(&bare),
            "the rim must replace fill pixels at the marker boundary"
        );
    }

    #[test]
    fn test_small_default_markers_read_as_their_series_colour() {
        // A 0.8pt rim on a 2pt marker is most of the marker, so a defaulted-on
        // rim made small scatter render visibly darker than its own palette
        // colour and than its own legend key.
        for size in [2.0_f32, 3.0, 4.0] {
            let image = square_scatter_plot(size)()
                .render()
                .unwrap_or_else(|_| panic!("scatter at marker_size {size} should render"));

            let darker_than_fill = image
                .pixels
                .chunks_exact(4)
                .filter(|px| px[2] < FILL.b && px[2] > 0 && px[1] < FILL.g)
                .count();

            assert!(
                exact_fill_pixels(&image) > darker_than_fill,
                "a {size}pt marker must read as its series colour, \
                 got {} exact-fill vs {darker_than_fill} darker pixels",
                exact_fill_pixels(&image)
            );
        }
    }

    #[test]
    fn test_line_markers_carry_the_requested_rim() {
        // `.line().marker(..)` had no way to ask for a marker edge at all, so
        // the renderer's marker-edge hop was unreachable from the public API.
        let (x, y) = xy();
        let plot = || {
            Plot::new()
                .size_px(240, 180)
                .ticks(false)
                .grid(false)
                .line(&x, &y)
                .marker(MarkerStyle::Circle)
                .marker_size(12.0)
                .color(FILL)
        };

        let rimmed = plot()
            .marker_edge_color(RIM)
            .marker_edge_width(2.0)
            .render()
            .expect("rimmed line markers should render");
        let bare = plot()
            .show_marker_edge(false)
            .render()
            .expect("bare line markers should render");

        assert!(
            rim_pixels(&rimmed) > 0,
            "an explicit line-marker edge colour must reach the canvas"
        );
        assert_eq!(
            rim_pixels(&bare),
            0,
            "show_marker_edge(false) must reach the renderer as 'no edge'"
        );
    }

    #[test]
    fn test_scatter_legend_key_carries_the_same_rim_as_its_markers() {
        // A legend key that does not carry the rim its markers carry is a key
        // for a different series.
        use crate::core::legend::LegendItemType;

        let (x, y) = xy();
        let plot = Plot::new()
            .scatter(&x, &y)
            .label("points")
            .marker(MarkerStyle::Square)
            .color(FILL)
            .edge_color(RIM)
            .edge_width(2.0)
            .into_plot();

        let series = plot.series_mgr.series().first().expect("one series");
        let item = series
            .to_legend_item(FILL, &Theme::default())
            .expect("a labelled series produces a legend item");

        match item.item_type {
            LegendItemType::Scatter { edge, .. } => assert_eq!(
                edge,
                Some((RIM, 2.0)),
                "the key must carry the rim the markers are stroked with"
            ),
            other => panic!("expected a scatter key, got {other:?}"),
        }
    }

    #[test]
    fn test_line_and_scatter_markers_resolve_the_same_default_edge() {
        assert_eq!(
            crate::plots::basic::LineConfig::default().resolved_marker_edge_spec(),
            crate::plots::basic::ScatterConfig::default().resolved_edge_spec(),
            "the same marker must not look different for coming from .line() vs .scatter()"
        );
    }
}

#[cfg(test)]
mod bar_edge_tests {
    use super::*;

    const FILL: Color = Color {
        r: 31,
        g: 119,
        b: 180,
        a: 255,
    };
    const EDGE: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Pixels that are more red than blue: the fill, the background and the
    /// spines all fail this, so it counts only the explicit red bar edge.
    fn edge_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] > 200 && px[1] < 120 && px[2] < 120)
            .count()
    }

    fn bar_plot(dpi: u32) -> PlotBuilder<crate::plots::basic::BarConfig> {
        let values = vec![1.0, 2.0];
        Plot::new()
            .size_px(240, 180)
            .dpi(dpi)
            .ticks(false)
            .grid(false)
            .bar(&["a", "b"], &values)
            .color(FILL)
    }

    #[test]
    fn test_bar_renders_an_explicit_edge_colour() {
        let image = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(3.0)
            .render()
            .expect("bar render should succeed");

        assert!(
            edge_pixels(&image) > 0,
            "an explicitly configured bar edge colour must reach the canvas"
        );
    }

    #[test]
    fn test_bar_edge_width_zero_draws_no_edge() {
        let image = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(0.0)
            .render()
            .expect("bar render should succeed");

        assert_eq!(
            edge_pixels(&image),
            0,
            "edge_width(0.0) must suppress the edge even when a colour is set"
        );
    }

    #[test]
    fn test_default_bar_renders_a_derived_edge() {
        let with_edge = bar_plot(100)
            .render()
            .expect("default bar render should succeed");
        let without_edge = bar_plot(100)
            .edge_width(0.0)
            .render()
            .expect("edgeless bar render should succeed");

        assert_ne!(
            with_edge.pixels, without_edge.pixels,
            "a default bar must draw the derived edge BarConfig::default() configures"
        );
    }

    #[test]
    fn test_zero_valued_bar_leaves_no_mark() {
        // A zero-height bar has no area, so its edge must not survive as a
        // hairline along the baseline: that would mark a datum that is not
        // there, and it would differ from the "no bar at all" case.
        let drawn = Plot::new()
            .size_px(240, 180)
            .ticks(false)
            .grid(false)
            .bar(&["zero", "one"], &[0.0f64, 1.0])
            .color(FILL)
            .edge_color(EDGE)
            .edge_width(3.0)
            .render()
            .expect("zero-valued bar render should succeed");

        // Only the second bar has a body, so every red pixel must sit in the
        // right-hand half of the plot.
        let width = drawn.width as usize;
        let left_half_edge = drawn
            .pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, px)| {
                index % width < width / 2 && px[0] > 200 && px[1] < 120 && px[2] < 120
            })
            .count();

        assert_eq!(
            left_half_edge, 0,
            "a zero-valued bar must not paint its edge along the baseline"
        );
        assert!(
            edge_pixels(&drawn) > 0,
            "the non-zero bar must still carry its edge"
        );
    }

    #[test]
    fn test_bar_width_fraction_reaches_the_geometry() {
        // `.bar_width(..)` used to be inert: both render paths hardcoded 0.8.
        let narrow = bar_pixel_rect(
            0,
            1.0,
            0.4,
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area"),
            -0.5,
            1.5,
            0.0,
            1.0,
            &crate::axes::AxisScale::Linear,
        );
        let wide = bar_pixel_rect(
            0,
            1.0,
            0.8,
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area"),
            -0.5,
            1.5,
            0.0,
            1.0,
            &crate::axes::AxisScale::Linear,
        );

        assert!(
            (wide.2 - narrow.2 * 2.0).abs() < 1e-3,
            "doubling the width fraction must double the bar: {} vs {}",
            narrow.2,
            wide.2
        );
        assert!(
            (narrow.0 + narrow.2 / 2.0 - (wide.0 + wide.2 / 2.0)).abs() < 1e-3,
            "the bar centre must not move when only the width changes"
        );
    }

    #[test]
    fn test_bar_edge_width_scales_with_dpi() {
        let low = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(2.0)
            .render()
            .expect("bar render should succeed");
        let high = bar_plot(200)
            .edge_color(EDGE)
            .edge_width(2.0)
            .render()
            .expect("bar render should succeed");

        let low_count = edge_pixels(&low);
        let high_count = edge_pixels(&high);

        assert!(low_count > 0, "the low-DPI edge must be drawn at all");
        // Doubling the DPI doubles the bar outline's length; if the stroke width
        // were left in raw pixels the count would merely double with it.
        assert!(
            high_count > low_count * 3,
            "edge width must scale with DPI: {low_count}px at 100 dpi vs {high_count}px at 200 dpi"
        );
    }
}

#[cfg(test)]
mod axis_scale_geometry_tests {
    use super::*;
    use crate::axes::AxisScale;
    use crate::render::MarkerStyle;

    const SERIES: Color = Color {
        r: 220,
        g: 20,
        b: 20,
        a: 255,
    };

    fn area() -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(20.0, 40.0, 200.0, 300.0).expect("valid plot area")
    }

    /// Screen y for `value` on a log axis spanning `y_min..y_max`, derived from
    /// the axis definition rather than from the mapping under test.
    fn log_screen_y(value: f64, y_min: f64, y_max: f64, plot_area: tiny_skia::Rect) -> f32 {
        let normalized = (value.log10() - y_min.log10()) / (y_max.log10() - y_min.log10());
        plot_area.top() + plot_area.height() * (1.0 - normalized as f32)
    }

    #[test]
    fn test_bar_body_follows_a_logarithmic_value_axis() {
        // The bug: bars were projected linearly while the ticks beside them
        // were projected in log space, so the bar top read off the wrong tick.
        let plot_area = area();
        let log = bar_pixel_rect(
            0,
            10.0,
            0.8,
            plot_area,
            -0.5,
            1.5,
            1.0,
            1000.0,
            &AxisScale::Log,
        );
        let linear = bar_pixel_rect(
            0,
            10.0,
            0.8,
            plot_area,
            -0.5,
            1.5,
            1.0,
            1000.0,
            &AxisScale::Linear,
        );

        // 10 is one decade into a three-decade axis, so the bar top sits a
        // third of the way up.
        let expected_top = log_screen_y(10.0, 1.0, 1000.0, plot_area);
        assert!(
            (log.1 - expected_top).abs() < 0.05,
            "bar top must sit at the log-mapped value: {} vs {expected_top}",
            log.1
        );
        // Zero has no position on a log axis, so the body bottoms out on the
        // axis floor — the same convention the parallel backend uses.
        assert!(
            (log.1 + log.3 - plot_area.bottom()).abs() < 0.05,
            "the bar must run down to the axis floor: {}",
            log.1 + log.3
        );
        assert!(
            (log.1 - linear.1).abs() > 50.0,
            "the log placement must actually differ from the linear one"
        );
        // Only the value axis is scaled; the categories keep their spacing.
        assert!((log.0 - linear.0).abs() < 1e-4);
        assert!((log.2 - linear.2).abs() < 1e-4);
    }

    #[test]
    fn test_histogram_bins_tile_without_gaps_on_a_logarithmic_x_axis() {
        // Centre-based placement (mapped centre ± half a width) only tiles on a
        // linear axis; on a log axis it leaves gaps and overlaps.
        let plot_area = area();
        let lower = histogram_bar_pixel_rect(
            1.0,
            10.0,
            5.0,
            plot_area,
            1.0,
            1000.0,
            0.0,
            10.0,
            &AxisScale::Log,
            &AxisScale::Linear,
        );
        let upper = histogram_bar_pixel_rect(
            10.0,
            100.0,
            3.0,
            plot_area,
            1.0,
            1000.0,
            0.0,
            10.0,
            &AxisScale::Log,
            &AxisScale::Linear,
        );

        assert!(
            (lower.0 + lower.2 - upper.0).abs() < 0.05,
            "adjacent bins must share an edge: {} vs {}",
            lower.0 + lower.2,
            upper.0
        );
        assert!(
            (lower.2 - upper.2).abs() < 0.05,
            "equal decades must occupy equal pixel widths on a log axis: {} vs {}",
            lower.2,
            upper.2
        );
        assert!(
            (lower.2 - plot_area.width() / 3.0).abs() < 0.05,
            "one decade of a three-decade axis must be a third of it: {}",
            lower.2
        );
    }

    #[test]
    fn test_histogram_bar_top_follows_a_logarithmic_count_axis() {
        let plot_area = area();
        let bar = histogram_bar_pixel_rect(
            1.0,
            2.0,
            10.0,
            plot_area,
            0.0,
            10.0,
            1.0,
            1000.0,
            &AxisScale::Linear,
            &AxisScale::Log,
        );

        let expected_top = log_screen_y(10.0, 1.0, 1000.0, plot_area);
        assert!(
            (bar.1 - expected_top).abs() < 0.05,
            "bar top must sit at the log-mapped count: {} vs {expected_top}",
            bar.1
        );
    }

    fn box_data() -> crate::plots::boxplot::BoxPlotData {
        let values: Vec<f64> = vec![1.0, 3.0, 10.0, 30.0, 100.0];
        crate::plots::boxplot::calculate_box_plot(
            &values,
            &crate::plots::boxplot::BoxPlotConfig::default(),
        )
        .expect("box plot statistics should compute")
    }

    #[test]
    fn test_boxplot_quantiles_sit_at_their_log_mapped_positions() {
        // `.boxplot(&d).yscale(Log)` used to draw a log-labelled axis with a
        // linearly-positioned box.
        let plot_area = area();
        let data = box_data();
        let log = BoxPlotPixels::new(&data, plot_area, -0.5, 0.5, 1.0, 1000.0, &AxisScale::Log);
        let linear =
            BoxPlotPixels::new(&data, plot_area, -0.5, 0.5, 1.0, 1000.0, &AxisScale::Linear);

        for (label, drawn, value) in [
            ("median", log.median_y, data.median),
            ("q1", log.q1_y, data.q1),
            ("q3", log.q3_y, data.q3),
            ("lower whisker", log.lower_whisker_y, data.min),
            ("upper whisker", log.upper_whisker_y, data.max),
        ] {
            let expected = log_screen_y(value, 1.0, 1000.0, plot_area);
            assert!(
                (drawn - expected).abs() < 0.05,
                "{label} must sit at its log-mapped position: {drawn} vs {expected}"
            );
        }

        assert!(
            (log.median_y - linear.median_y).abs() > 20.0,
            "the log placement must actually differ from the linear one"
        );
        // The category axis is untouched: only the value axis is scaled.
        assert!((log.x_center - linear.x_center).abs() < 1e-4);
        assert!((log.box_left - linear.box_left).abs() < 1e-4);
        assert!((log.box_right - linear.box_right).abs() < 1e-4);
    }

    #[test]
    fn test_boxplot_outliers_use_the_same_projection_as_its_whiskers() {
        let plot_area = area();
        let data = box_data();
        let pixels = BoxPlotPixels::new(&data, plot_area, -0.5, 0.5, 1.0, 1000.0, &AxisScale::Log);

        assert!(
            (box_plot_value_y(data.max, plot_area, 1.0, 1000.0, &AxisScale::Log)
                - pixels.upper_whisker_y)
                .abs()
                < 1e-4,
            "a flier must be projected exactly like the whisker it lies beyond"
        );
    }

    fn boxplot_image(scale: AxisScale) -> Image {
        let values = vec![1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 900.0];
        Plot::new()
            .size_px(240, 320)
            .ticks(false)
            .grid(false)
            .boxplot(&values)
            .color(SERIES)
            .ylim(1.0, 1000.0)
            .yscale(scale)
            .render()
            .expect("box plot should render")
    }

    #[test]
    fn test_rendered_boxplot_moves_when_the_value_axis_becomes_logarithmic() {
        // Ticks, grid and the y limits are all pinned, so the only thing that
        // can differ between these two figures is the box geometry itself.
        // Before the fix they were pixel-identical.
        let linear = boxplot_image(AxisScale::Linear);
        let log = boxplot_image(AxisScale::Log);

        assert_ne!(
            linear.pixels, log.pixels,
            "`yscale(Log)` must reach the box geometry, not only the axis labels"
        );
    }

    /// Rows in `column` carrying series-coloured ink.
    ///
    /// Measured as "redder than it is grey", so partially covered
    /// anti-aliased pixels count too; the white background and the neutral
    /// spines and axes never do.
    fn inked_rows(image: &Image, column: usize) -> Vec<usize> {
        let width = image.width as usize;
        (0..image.height as usize)
            .filter(|row| {
                let index = (row * width + column) * 4;
                let px = &image.pixels[index..index + 4];
                i16::from(px[0]) - i16::from(px[1].max(px[2])) > 30
            })
            .collect()
    }

    #[test]
    fn test_error_bars_stay_attached_to_their_markers_on_a_logarithmic_axis() {
        // The markers were projected scale-aware and the whiskers linearly, so
        // on a log axis the error bar detached from the point it belonged to
        // and slid down towards the axis floor.
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .line(&[1.0, 2.0, 3.0], &[10.0, 10.0, 10.0])
            .color(SERIES)
            .marker(MarkerStyle::Circle)
            .marker_size(6.0)
            .with_yerr(&[5.0, 5.0, 5.0])
            .ylim(1.0, 1000.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("line with y errors on a log axis should render");

        // An error-bar column carries far more series-coloured pixels than the
        // horizontal line does; the spines are not series-coloured.
        let (column, rows) = (0..image.width as usize)
            .map(|column| (column, inked_rows(&image, column)))
            .max_by_key(|(_, rows)| rows.len())
            .expect("the figure has at least one column");

        assert!(
            rows.len() > 10,
            "expected to find an error-bar column, found {} inked rows at column {column}",
            rows.len()
        );

        let largest_gap = rows
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .max()
            .unwrap_or(0);
        assert!(
            largest_gap <= 2,
            "the error bar must stay attached to its marker: {largest_gap}px gap in column {column}"
        );
    }

    #[test]
    fn test_a_sample_the_log_axis_cannot_place_draws_no_whisker_at_all() {
        // `y = 0` and `y = -5` have no position on a log axis. Their whisker
        // ends projected to NaN pixels, and `NaN.max(top).min(bottom)` yields
        // `top` — so the stem was pinned to the top of the frame and drawn all
        // the way down to the other end, a runaway line across 87% of the plot
        // height hanging off a point that was (correctly) not drawn.
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .line(
                &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
                &[10.0, 20.0, 0.0, 40.0, -5.0, 60.0],
            )
            .color(SERIES)
            .with_yerr(&[2.0; 6])
            .ylim(1.0, 200.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log plot with unrepresentable samples should still render");

        // Each valid sample's whisker spans +-2 around its value, which at this
        // scale is a few dozen pixels; nothing may span a large fraction of the
        // plot. The bound is generous so it fails only on a runaway.
        let tallest = (0..image.width as usize)
            .map(|column| {
                let rows = inked_rows(&image, column);
                match (rows.first(), rows.last()) {
                    (Some(&first), Some(&last)) => last - first + 1,
                    _ => 0,
                }
            })
            .max()
            .expect("the figure has at least one column");

        assert!(
            tallest < image.height as usize / 3,
            "no whisker may run the height of the plot: tallest column spans {tallest}px \
             of {}px",
            image.height
        );
    }

    #[test]
    fn test_bar_chart_refuses_a_non_linear_category_axis() {
        // Categories are ordinals: there is no quantity to take a log of, so
        // the only honest answers are "reject" and "draw something wrong".
        let error = Plot::new()
            .bar(&["a", "b"], &[1.0, 2.0])
            .xscale(AxisScale::Log)
            .render()
            .expect_err("a log category axis must be refused, not drawn linearly");

        assert!(
            matches!(error, PlottingError::InvalidInput(_)),
            "expected InvalidInput, got {error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("bar"), "{message}");
        assert!(message.contains("x axis"), "{message}");
        assert!(message.contains("xscale"), "{message}");
    }

    #[test]
    fn test_heatmap_cells_follow_a_logarithmic_axis() {
        // A heatmap draws through `PlotArea`, which projects through the
        // figure's axis scales. The boundary between two rows must therefore sit
        // where the log axis puts it, not where a linear one would.
        use crate::plots::heatmap::HeatmapConfig;

        let values = vec![vec![1.0, 1.0], vec![2.0, 2.0]];
        let config = HeatmapConfig::new()
            .extent(0.0, 1.0, 1.0, 100.0)
            .colorbar(false);

        // How many rows of the middle column the lower (bright) cell covers.
        let lower_cell_rows = |scale: AxisScale| {
            let image = Plot::new()
                .size_px(200, 300)
                .ticks(false)
                .grid(false)
                .heatmap_with(&values, config.clone())
                .yscale(scale)
                .render()
                .expect("a heatmap must render on any axis scale");
            let width = image.width as usize;
            let column = width / 2;
            // The two cells sit at the ends of viridis: the lower one is the
            // bright yellow end, which nothing else in the figure resembles.
            (0..image.height as usize)
                .filter(|row| {
                    let index = (row * width + column) * 4;
                    let px = &image.pixels[index..index + 4];
                    px[0] > 150 && px[1] > 150 && px[2] < 120
                })
                .count()
        };

        // The lower cell covers data y 1..50.5 of an extent of 1..100.
        // Linearly that is half the axis; logarithmically it is
        // log10(50.5 / 1) / log10(100 / 1) ≈ 85% of it.
        let linear = lower_cell_rows(AxisScale::Linear);
        let log = lower_cell_rows(AxisScale::Log);
        assert!(
            log > linear + 40,
            "a log y axis must stretch the lower heatmap cell: \
             linear covered {linear} rows, log covered {log}"
        );
    }

    #[test]
    fn test_scale_aware_series_still_accept_log_axes() {
        // The rejection must be narrow: everything that does project through
        // the axis scale keeps working.
        Plot::new()
            .size_px(240, 180)
            .line(&[1.0, 10.0, 100.0], &[1.0, 10.0, 100.0])
            .xscale(AxisScale::Log)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-log line plot must still render");

        Plot::new()
            .size_px(240, 180)
            .scatter(&[1.0, 10.0, 100.0], &[1.0, 10.0, 100.0])
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y scatter plot must still render");
    }

    #[test]
    fn test_plot_area_series_accept_log_axes() {
        // These all draw through `PlotArea`, which projects through the figure's
        // axis scales. They were refused outright while it mapped linearly; the
        // refusal has to be gone now that it does not.
        let samples: Vec<f64> = (0..120)
            .map(|index| 3.0 + 15.0 * ((index as f64 * 0.37).sin() * 0.5 + 0.5))
            .collect();

        Plot::new()
            .size_px(240, 180)
            .kde(&samples)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x KDE must render");

        Plot::new()
            .size_px(240, 180)
            .ecdf(&samples)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x ECDF must render");

        Plot::new()
            .size_px(240, 180)
            .violin(&samples)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y violin must render");

        Plot::new()
            .size_px(240, 180)
            .boxen(&samples)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y boxen must render");

        let x: Vec<f64> = (1..=12).map(f64::from).collect();
        let y: Vec<f64> = (1..=10).map(f64::from).collect();
        let z: Vec<f64> = y
            .iter()
            .flat_map(|&yy| x.iter().map(move |&xx| (xx * 0.2).sin() + (yy * 0.2).cos()))
            .collect();
        Plot::new()
            .size_px(240, 180)
            .contour(&x, &y, &z)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x contour must render");
    }

    /// A bar or histogram fills down to zero, and a log axis has no position for
    /// zero. Folding the baseline into the auto-range therefore made the range
    /// itself invalid, and the default path could not be exercised at all: it
    /// failed with "Logarithmic scale requires positive values" before drawing.
    #[test]
    fn test_bar_and_histogram_autoscale_on_a_log_value_axis() {
        Plot::new()
            .size_px(240, 180)
            .bar(&["a", "b", "c"], &[1.0, 10.0, 100.0])
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y bar chart must render without an explicit ylim");

        let samples: Vec<f64> = (0..400).map(|index| (index as f64 * 0.031).sin()).collect();
        Plot::new()
            .size_px(240, 180)
            .histogram(&samples)
            .bins(12)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y histogram must render without an explicit ylim");
    }

    /// Bars still have to bottom out on the axis floor and top out at their
    /// value: the decades between two bars must be the decades the axis shows.
    #[test]
    fn test_log_bar_tops_land_one_decade_apart() {
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .bar(&["a", "b", "c"], &[1.0, 10.0, 100.0])
            .color(SERIES)
            .ylim(0.5, 200.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y bar chart must render");

        let tops: Vec<usize> = [0usize, 1, 2]
            .iter()
            .map(|slot| {
                // Sample the middle of each of the three category slots.
                let column = (image.width as usize * (2 * slot + 1)) / 6;
                *inked_rows(&image, column)
                    .first()
                    .unwrap_or_else(|| panic!("bar {slot} must put ink in column {column}"))
            })
            .collect();

        let first_gap = tops[0] as f64 - tops[1] as f64;
        let second_gap = tops[1] as f64 - tops[2] as f64;
        assert!(
            (first_gap - second_gap).abs() < 2.0,
            "each decade must be the same height on a log axis, got {first_gap} then {second_gap}"
        );
    }

    #[test]
    fn test_linear_figures_are_never_refused() {
        // Every plot type must stay renderable on the default axes, including
        // the ones that declare a non-linear scale unsupported.
        let values = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        Plot::new()
            .size_px(240, 180)
            .heatmap(&values)
            .render()
            .expect("a heatmap on linear axes must render");

        Plot::new()
            .size_px(240, 180)
            .bar(&["a", "b"], &[1.0, 2.0])
            .render()
            .expect("a bar chart on linear axes must render");
    }
}

#[cfg(test)]
mod category_axis_tests {
    use super::*;
    use crate::plots::boxplot::{BoxPlotConfig, calculate_box_plot};

    fn samples() -> Vec<f64> {
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    }

    fn boxplot_series(plot: Plot, category: &str) -> Plot {
        plot.add_box_plot_series(
            PlotData::Static(samples()),
            BoxPlotConfig::new().category(category),
            crate::core::plot::builder::SeriesStyle::default(),
        )
    }

    #[test]
    fn distinct_categories_claim_consecutive_slots() {
        // The defect: several boxes could not be laid out side by side at all.
        let plot = boxplot_series(boxplot_series(Plot::new(), "control"), "treated");
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(
            axis.labels,
            vec!["control".to_string(), "treated".to_string()]
        );
        assert_eq!(axis.positions, vec![0.0, 1.0]);
        // Exactly the span a two-category bar chart asks for.
        assert_eq!(axis.x_span(), (-0.5, 1.5));
    }

    #[test]
    fn a_repeated_category_reuses_its_slot() {
        // Two series in one category is what "grouped" means; they must not
        // drift apart onto two slots with the same name.
        let plot = boxplot_series(boxplot_series(Plot::new(), "control"), "control");
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(axis.labels, vec!["control".to_string()]);
        assert_eq!(axis.positions, vec![0.0]);
    }

    #[test]
    fn distributions_continue_the_slots_a_bar_series_already_took() {
        // Bars and box plots share one axis, so a box added after a two-bar
        // series lands beside them rather than on top of the first bar.
        let bars = Plot::new().add_bar_series(
            vec!["a".to_string(), "b".to_string()],
            PlotData::Static(vec![1.0, 2.0]),
            &crate::plots::basic::BarConfig::default(),
            crate::core::plot::builder::SeriesStyle::default(),
        );
        let plot = boxplot_series(bars, "c");
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(
            axis.labels,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(axis.positions, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn an_uncategorised_box_still_holds_a_slot() {
        // `.boxplot(&d)` with no category used to fall through to a numeric
        // 0..1 axis reading "0, 0.2, ... 1.0" and meaning nothing. It now owns
        // slot 0 with an empty label, so the axis has nothing to write.
        let plot = Plot::new().add_box_plot_series(
            PlotData::Static(samples()),
            BoxPlotConfig::new(),
            crate::core::plot::builder::SeriesStyle::default(),
        );
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(axis.labels, vec![String::new()]);
        assert_eq!(axis.positions, vec![0.0]);
        assert_eq!(axis.x_span(), (-0.5, 0.5));
    }

    /// The multi-slot categorical types — the two whose *only* input is a list
    /// of category names — put the names the caller passed on the same axis.
    ///
    /// They used to reach no harvest at all, so `.strip(&["A", "B", "C"], ..)`
    /// printed `-0.5, 0, 0.5, 1 …` under three columns plainly labelled by the
    /// caller, which is the one thing a category axis exists to prevent.
    #[test]
    fn strip_and_swarm_put_their_category_names_on_the_axis() {
        let categories = ["A", "A", "B", "C"];
        let values = [1.0, 2.0, 3.0, 4.0];
        for plot in [
            Plot::new().strip(&categories, &values).finalize(),
            Plot::new().swarm(&categories, &values).finalize(),
        ] {
            let axis = CategoryAxis::harvest(&plot.series_mgr.series)
                .expect("a categorical plot type must produce a category axis");

            assert_eq!(
                axis.labels,
                vec!["A".to_string(), "B".to_string(), "C".to_string()]
            );
            assert_eq!(axis.positions, vec![0.0, 1.0, 2.0]);
            assert_eq!(axis.x_span(), (-0.5, 2.5));
        }
    }

    /// A dendrogram's leaf axis is a category axis too: the labels belong under
    /// the leaves, not a second set of numbers beside them.
    #[test]
    fn a_dendrogram_labels_its_leaves() {
        let distances = crate::stats::clustering::pdist_euclidean(&[
            vec![0.0, 0.0],
            vec![0.1, 0.0],
            vec![5.0, 0.0],
        ]);
        let tree = crate::stats::clustering::linkage(
            &distances,
            crate::stats::clustering::LinkageMethod::Average,
        );
        let plot = Plot::new().dendrogram(&tree).finalize();
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(axis.positions, vec![0.0, 1.0, 2.0]);
        assert_eq!(axis.labels.len(), 3);
        let mut sorted = axis.labels.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec!["0".to_string(), "1".to_string(), "2".to_string()],
            "every leaf must be named exactly once"
        );
    }

    #[test]
    fn a_figure_with_no_categorical_series_has_no_category_axis() {
        let xs = vec![0.0, 1.0];
        let mut plot = Plot::new();
        plot.add_line(&xs, &xs).expect("a line series");
        assert!(CategoryAxis::harvest(&plot.series_mgr.series).is_none());
    }

    #[test]
    fn an_explicit_x_position_overrides_the_automatic_slot() {
        let plot = Plot::new().add_box_plot_series(
            PlotData::Static(samples()),
            BoxPlotConfig::new().category("only").x_position(3.0),
            crate::core::plot::builder::SeriesStyle::default(),
        );
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(axis.positions, vec![3.0]);
        assert_eq!(axis.x_span(), (2.5, 3.5));
    }

    fn violin_series(plot: Plot, category: &str) -> Plot {
        let config = crate::plots::ViolinConfig::new().category(category);
        let data =
            crate::plots::ViolinData::from_values(&samples(), &config).expect("violin statistics");
        plot.add_violin_series(data, crate::core::plot::builder::SeriesStyle::default())
    }

    fn boxen_series(plot: Plot, category: &str) -> Plot {
        let config = crate::plots::BoxenConfig::new().category(category);
        let data = crate::plots::compute_boxen(&samples(), &config);
        plot.add_boxen_series(data, crate::core::plot::builder::SeriesStyle::default())
    }

    #[test]
    fn every_distribution_type_lands_on_the_same_axis() {
        // Box plot, violin and boxen all sit in unit slots on one axis; a
        // per-type convention here is exactly what let them drift before.
        let plot = boxen_series(
            violin_series(boxplot_series(Plot::new(), "box"), "violin"),
            "boxen",
        );
        let axis = CategoryAxis::harvest(&plot.series_mgr.series).expect("a category axis");

        assert_eq!(
            axis.labels,
            vec!["box".to_string(), "violin".to_string(), "boxen".to_string()]
        );
        assert_eq!(axis.positions, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn a_box_is_drawn_at_the_centre_of_its_own_slot() {
        // Two categories, so the axis runs -0.5..1.5 and each slot is half the
        // panel wide. The second box must sit in the right half.
        let plot_area =
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area");
        let first = calculate_box_plot(&samples(), &BoxPlotConfig::new().x_position(0.0))
            .expect("box statistics");
        let second = calculate_box_plot(&samples(), &BoxPlotConfig::new().x_position(1.0))
            .expect("box statistics");

        let left = BoxPlotPixels::new(
            &first,
            plot_area,
            -0.5,
            1.5,
            0.0,
            10.0,
            &crate::axes::AxisScale::Linear,
        );
        let right = BoxPlotPixels::new(
            &second,
            plot_area,
            -0.5,
            1.5,
            0.0,
            10.0,
            &crate::axes::AxisScale::Linear,
        );

        assert!((left.x_center - 50.0).abs() < 1e-3, "{}", left.x_center);
        assert!((right.x_center - 150.0).abs() < 1e-3, "{}", right.x_center);
        // The boxes must not overlap: each is `width_ratio` of its own slot.
        assert!(left.box_right < right.box_left);
    }

    #[test]
    fn box_width_is_a_fraction_of_one_slot_not_of_the_panel() {
        // `width_ratio` used to be applied to the whole panel width, so adding
        // a second category silently doubled every box's apparent width.
        let plot_area =
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area");
        let data = calculate_box_plot(&samples(), &BoxPlotConfig::new().width_ratio(0.5))
            .expect("box statistics");

        let alone = BoxPlotPixels::new(
            &data,
            plot_area,
            -0.5,
            0.5,
            0.0,
            10.0,
            &crate::axes::AxisScale::Linear,
        );
        let paired = BoxPlotPixels::new(
            &data,
            plot_area,
            -0.5,
            1.5,
            0.0,
            10.0,
            &crate::axes::AxisScale::Linear,
        );

        // One slot of 200 px vs one slot of 100 px, both half-filled.
        assert!((alone.box_right - alone.box_left - 100.0).abs() < 1e-3);
        assert!((paired.box_right - paired.box_left - 50.0).abs() < 1e-3);
    }

    #[test]
    fn grouped_boxen_widens_the_axis_and_separates_the_boxes() {
        // End-to-end: bounds, geometry and spacing for the one distribution
        // family whose bounds already flow through `PlotData::data_bounds`.
        use crate::plots::traits::PlotData as _;

        let plot = boxen_series(boxen_series(Plot::new(), "a"), "b");

        let bounds: Vec<(f64, f64)> = plot
            .series_mgr
            .series
            .iter()
            .map(|series| match &series.series_type {
                SeriesType::Boxen { data } => data.data_bounds().0,
                other => panic!("expected boxen series, got {other:?}"),
            })
            .collect();

        assert_eq!(bounds, vec![(-0.5, 0.5), (0.5, 1.5)]);
    }
}
