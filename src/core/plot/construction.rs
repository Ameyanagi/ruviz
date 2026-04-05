use super::*;

impl Plot {
    /// Create a new Plot with default settings
    ///
    /// Uses matplotlib-compatible defaults:
    /// - Figure size: 6.4 × 4.8 inches
    /// - DPI: 100 (640 × 480 pixels)
    /// - Font size: 10pt base
    /// - Line width: 1.5pt
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    /// let y = vec![1.0, 4.0, 9.0, 16.0, 25.0];
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .end_series()
    ///     .save("plot.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new() -> Self {
        let config = PlotConfig::default();
        let (width, height) = config.canvas_size();
        let display = PlotConfiguration {
            title: None,
            xlabel: None,
            ylabel: None,
            dimensions: (width, height),
            dpi: config.figure.dpi as u32,
            theme: Theme::default(),
            text_engine: TextEngineMode::Plain,
            config,
        };
        Self {
            display,
            series_mgr: SeriesManager::new(),
            layout: LayoutManager::new(),
            render: RenderPipeline::new(),
            annotations: Vec::new(),
            null_policy: NullPolicy::Error,
            pending_ingestion_error: None,
            series_groups: Vec::new(),
            next_group_id: 0,
        }
    }

    /// Create a new Plot with a specific configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = PlotConfig::builder()
    ///     .figure(8.0, 6.0)
    ///     .dpi(300.0)
    ///     .build();
    /// Plot::with_config(config).line(&x, &y).save("plot.png")?;
    /// ```
    pub fn with_config(config: PlotConfig) -> Self {
        let (width, height) = config.canvas_size();
        let mut plot = Self::new();
        plot.display.dimensions = (width, height);
        plot.display.dpi = config.figure.dpi as u32;
        plot.display.config = config;
        plot
    }

    /// Create a reusable prepared runtime for repeated frame rendering.
    pub fn prepare(&self) -> PreparedPlot {
        PreparedPlot::new(self.clone())
    }

    /// Prepare a shared interactive rendering session for repeated viewport-aware frames.
    pub fn prepare_interactive(&self) -> InteractivePlotSession {
        InteractivePlotSession::new(self.prepare())
    }

    /// Create a new Plot with a preset style
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::with_style(PlotStyle::Publication)
    ///     .line(&x, &y)
    ///     .save("paper_figure.png")?;
    /// ```
    pub fn with_style(style: PlotStyle) -> Self {
        Self::with_config(style.config())
    }

    /// Create a new Plot with a specific theme
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0, 4.0];
    /// let y = vec![1.0, 4.0, 2.0, 3.0];
    ///
    /// Plot::with_theme(Theme::dark())
    ///     .line(&x, &y)
    ///     .end_series()
    ///     .save("dark_plot.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_theme(theme: Theme) -> Self {
        let mut plot = Self::new();
        plot.display.theme = theme;
        plot
    }

    /// Set null handling policy for dataframe-backed numeric ingestion.
    ///
    /// Default is [`NullPolicy::Error`], which fails on null values with
    /// an explicit plotting error.
    pub fn null_policy(mut self, policy: NullPolicy) -> Self {
        self.null_policy = policy;
        self
    }

    pub(super) fn set_pending_ingestion_error(&mut self, err: PlottingError) {
        if let Some(existing) = &mut self.pending_ingestion_error {
            existing.record_additional_error();
        } else {
            self.pending_ingestion_error = Some(PendingIngestionError::from_plotting_error(err));
        }
    }

    pub(super) fn pending_ingestion_error(&self) -> Option<PlottingError> {
        self.pending_ingestion_error
            .as_ref()
            .map(PendingIngestionError::to_plotting_error)
    }

    pub(super) fn register_series_group(&mut self) -> usize {
        let group_id = self.next_group_id;
        self.next_group_id = self
            .next_group_id
            .checked_add(1)
            .expect("series group id overflow");
        self.series_groups.push(SeriesGroupMeta {
            id: group_id,
            label: None,
        });
        group_id
    }

    pub(super) fn set_series_group_label(&mut self, group_id: usize, label: String) {
        let mut found = false;
        if let Some(group) = self
            .series_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.label = Some(label);
            found = true;
        }
        debug_assert!(
            found,
            "set_series_group_label called with unknown group_id: {}",
            group_id
        );
    }

    pub(super) fn collect_legend_items(&self) -> Vec<LegendItem> {
        let mut legend_items = Vec::new();
        let mut seen_group_ids = HashSet::new();
        let group_labels: HashMap<usize, &str> = self
            .series_groups
            .iter()
            .filter_map(|group| group.label.as_deref().map(|label| (group.id, label)))
            .collect();

        for (idx, series) in self.series_mgr.series.iter().enumerate() {
            if let Some(group_id) = series.group_id {
                if !seen_group_ids.insert(group_id) {
                    continue;
                }

                let Some(label) = group_labels.get(&group_id).copied() else {
                    continue;
                };

                let default_color = self.display.theme.get_color(idx);
                if let Some(item) = series.to_legend_item_with_label(
                    label.to_string(),
                    default_color,
                    &self.display.theme,
                ) {
                    legend_items.push(item);
                }
                continue;
            }

            legend_items.extend(series.to_legend_items(idx, &self.display.theme));
        }

        legend_items
    }

    /// Set the theme for the plot (fluent API)
    ///
    /// Available themes include:
    /// - `Theme::light()` - default light theme
    /// - `Theme::dark()` - dark mode theme
    /// - `Theme::seaborn()` - seaborn-style theme
    /// - `Theme::publication()` - publication-ready theme
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    /// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    ///
    /// Plot::new()
    ///     .theme(Theme::dark())
    ///     .line(&x, &y)
    ///     .end_series()
    ///     .save("dark_theme.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// | Default | Dark | Seaborn | Publication |
    /// |---------|------|---------|-------------|
    /// | ![Default](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_default.png) | ![Dark](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_dark.png) | ![Seaborn](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_seaborn.png) | ![Publication](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/theme_publication.png) |
    pub fn theme(mut self, theme: Theme) -> Self {
        self.display.theme = theme;
        self
    }

    /// Get the current theme
    pub fn get_theme(&self) -> Theme {
        self.display.theme.clone()
    }

    /// Scale typography by a factor
    ///
    /// This is useful for rendering plots to smaller canvases (like subplots)
    /// where the default font sizes would be too large.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ruviz::prelude::*;
    ///
    /// let plot = Plot::new()
    ///     .line(&[1.0, 2.0], &[1.0, 4.0])
    ///     .end_series()
    ///     .title("Scaled Plot")
    ///     .scale_typography(0.7); // 70% of normal font sizes
    /// ```
    pub fn scale_typography(mut self, factor: f32) -> Self {
        self.display.config.typography = self.display.config.typography.scale(factor);
        self
    }

    /// Configure parallel rendering settings
    #[cfg(feature = "parallel")]
    pub fn with_parallel(mut self, threads: Option<usize>) -> Self {
        if let Some(thread_count) = threads {
            self.render.parallel_renderer = ParallelRenderer::with_threads(thread_count);
        }
        self
    }

    /// Set parallel processing threshold
    #[cfg(feature = "parallel")]
    pub fn parallel_threshold(mut self, threshold: usize) -> Self {
        self.render.parallel_renderer = self.render.parallel_renderer.with_threshold(threshold);
        self
    }

    /// Enable memory pooled rendering for allocation optimization
    ///
    /// This reduces allocation overhead by 30-50% for large datasets by reusing
    /// memory buffers for coordinate transformations and rendering operations.
    pub fn with_memory_pooling(mut self, enable: bool) -> Self {
        self.render.enable_pooled_rendering = enable;
        if enable && self.render.pooled_renderer.is_none() {
            self.render.pooled_renderer = Some(crate::render::PooledRenderer::new());
        }
        self
    }

    /// Configure memory pool sizes for specific workloads
    ///
    /// # Arguments
    /// * `f32_pool_size` - Initial capacity for coordinate transformation pools
    /// * `position_pool_size` - Initial capacity for position/point pools  
    /// * `segment_pool_size` - Initial capacity for line segment pools
    pub fn with_pool_sizes(
        mut self,
        f32_pool_size: usize,
        position_pool_size: usize,
        segment_pool_size: usize,
    ) -> Self {
        self.render.pooled_renderer = Some(crate::render::PooledRenderer::with_pool_sizes(
            f32_pool_size,
            position_pool_size,
            segment_pool_size,
        ));
        self.render.enable_pooled_rendering = true;
        self
    }

    /// Get memory pool statistics for monitoring and optimization
    pub fn pool_stats(&self) -> Option<crate::render::PooledRendererStats> {
        self.render
            .pooled_renderer
            .as_ref()
            .map(|renderer| renderer.get_pool_stats())
    }

    /// Set the plot title
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .title("My Plot Title")
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .end_series()
    ///     .save("titled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn title(mut self, title: impl Into<data::PlotText>) -> Self {
        self.display.title = Some(title.into());
        self
    }

    /// Set the plot title as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::prelude::*;
    /// use ruviz::animation::signal;
    ///
    /// let title_signal = signal::of(|t| format!("Time: {:.2}s", t));
    /// let x = vec![0.0, 1.0, 2.0];
    /// let y = vec![0.0, 0.5, 1.0];
    /// let plot = Plot::new()
    ///     .title_signal(title_signal)
    ///     .line(&x, &y);
    /// ```
    pub fn title_signal(mut self, title: impl Into<data::PlotText>) -> Self {
        self.display.title = Some(title.into());
        self
    }

    /// Set the X-axis label
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .xlabel("Time (s)")
    ///     .ylabel("Amplitude")
    ///     .line(&[0.0, 1.0, 2.0], &[0.0, 0.5, 1.0])
    ///     .end_series()
    ///     .save("labeled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn xlabel(mut self, label: impl Into<data::PlotText>) -> Self {
        self.display.xlabel = Some(label.into());
        self
    }

    /// Set the X-axis label as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    pub fn xlabel_signal(mut self, label: impl Into<data::PlotText>) -> Self {
        self.display.xlabel = Some(label.into());
        self
    }

    /// Set the Y-axis label
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .ylabel("Temperature (°C)")
    ///     .line(&[1.0, 2.0, 3.0], &[20.0, 22.0, 21.0])
    ///     .end_series()
    ///     .save("ylabel.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ylabel(mut self, label: impl Into<data::PlotText>) -> Self {
        self.display.ylabel = Some(label.into());
        self
    }

    /// Set the Y-axis label as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    pub fn ylabel_signal(mut self, label: impl Into<data::PlotText>) -> Self {
        self.display.ylabel = Some(label.into());
        self
    }

    /// Set X-axis limits (min, max)
    ///
    /// Passing descending bounds, such as `xlim(10.0, 0.0)`, preserves a
    /// reversed X axis.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .xlim(0.0, 10.0)
    ///     .ylim(-1.0, 1.0)
    ///     .line(&[0.0, 5.0, 10.0], &[0.0, 1.0, 0.0])
    ///     .end_series()
    ///     .save("limits.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        if min != max && min.is_finite() && max.is_finite() {
            self.layout.x_limits = Some((min, max));
        }
        self
    }

    /// Set Y-axis limits (min, max)
    ///
    /// Passing descending bounds, such as `ylim(10.0, 0.0)`, preserves a
    /// reversed Y axis.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .ylim(0.0, 100.0)
    ///     .line(&[1.0, 2.0, 3.0], &[25.0, 50.0, 75.0])
    ///     .end_series()
    ///     .save("ylim.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        if min != max && min.is_finite() && max.is_finite() {
            self.layout.y_limits = Some((min, max));
        }
        self
    }

    /// Set X-axis scale type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::prelude::*;
    ///
    /// // Logarithmic X axis
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .xscale(AxisScale::Log)
    ///     .save("log_x.png")?;
    ///
    /// // Symmetric log for data with zeros or negatives
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .xscale(AxisScale::symlog(1.0))
    ///     .save("symlog_x.png")?;
    /// ```
    pub fn xscale(mut self, scale: AxisScale) -> Self {
        self.layout.x_scale = scale;
        self
    }

    /// Set Y-axis scale type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::prelude::*;
    ///
    /// // Logarithmic Y axis (common for exponential data)
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .yscale(AxisScale::Log)
    ///     .save("log_y.png")?;
    ///
    /// // Log-log plot
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .xscale(AxisScale::Log)
    ///     .yscale(AxisScale::Log)
    ///     .save("loglog.png")?;
    /// ```
    pub fn yscale(mut self, scale: AxisScale) -> Self {
        self.layout.y_scale = scale;
        self
    }

    /// Set canvas dimensions in pixels
    ///
    /// This method automatically scales DPI based on canvas size to maintain
    /// proportional text and element sizes on larger canvases.
    ///
    /// # Deprecation
    ///
    /// Prefer `size(width, height)` which takes dimensions in inches for
    /// DPI-independent sizing, or `size_pixels(width, height)` for pixel-based sizing.
    #[deprecated(since = "0.2.0", note = "Use size() or size_pixels() instead")]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.display.dimensions = (width.max(100), height.max(100));

        // Auto-scale DPI based on canvas size relative to reference (640x480)
        // This ensures text maintains proportional size on larger canvases
        let reference_diagonal = ((640.0_f32).powi(2) + (480.0_f32).powi(2)).sqrt();
        let canvas_diagonal = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let scale_factor = (canvas_diagonal / reference_diagonal).max(1.0);
        let auto_dpi = (REFERENCE_DPI * scale_factor).round().max(100.0);

        self.display.dpi = auto_dpi as u32;
        self.display.config.figure.dpi = auto_dpi;
        self.display.config.figure.width = width as f32 / auto_dpi;
        self.display.config.figure.height = height as f32 / auto_dpi;
        self
    }

    /// Set figure size in inches
    ///
    /// This is the recommended way to set figure size for DPI-independent plots.
    /// The final pixel dimensions are calculated as `width × dpi` by `height × dpi`.
    ///
    /// # Size and DPI Relationship
    ///
    /// | Size (inches) | DPI | Pixel Dimensions |
    /// |---------------|-----|------------------|
    /// | 8×6           | 100 | 800×600          |
    /// | 8×6           | 150 | 1200×900         |
    /// | 8×6           | 300 | 2400×1800        |
    ///
    /// # Arguments
    ///
    /// * `width` - Figure width in inches (minimum 1.0)
    /// * `height` - Figure height in inches (minimum 1.0)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Screen display (100 DPI default)
    /// Plot::new()
    ///     .size(8.0, 6.0)  // 800×600 pixels
    ///     .save("screen.png")?;
    ///
    /// // Print quality (300 DPI)
    /// Plot::new()
    ///     .size(8.0, 6.0)  // Same size in inches
    ///     .dpi(300)        // 2400×1800 pixels
    ///     .save("print.png")?;
    /// ```
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.display.config.figure.width = width.max(1.0);
        self.display.config.figure.height = height.max(1.0);
        // Update legacy fields for backward compatibility
        let (w, h) = self.display.config.canvas_size();
        self.display.dimensions = (w, h);
        self
    }

    /// Set figure size in pixels
    ///
    /// Convenience method for users who prefer to think in pixels.
    /// Internally converts to inches using reference DPI (100).
    ///
    /// # Arguments
    ///
    /// * `width` - Figure width in pixels
    /// * `height` - Figure height in pixels
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .size_px(800, 600)  // 800×600 pixels at 100 DPI
    ///     .save("figure.png")?;
    /// ```
    pub fn size_px(mut self, width: u32, height: u32) -> Self {
        use crate::core::units::REFERENCE_DPI;
        self.display.config.figure.width = width as f32 / REFERENCE_DPI;
        self.display.config.figure.height = height as f32 / REFERENCE_DPI;
        // Update legacy fields
        let (w, h) = self.display.config.canvas_size();
        self.display.dimensions = (w, h);
        self
    }

    pub(crate) fn set_output_pixels(mut self, width: u32, height: u32) -> Self {
        let dpi = self.display.config.figure.dpi;
        if dpi.is_finite() && dpi != 0.0 {
            self.display.config.figure.width = width as f32 / dpi;
            self.display.config.figure.height = height as f32 / dpi;
        }
        self.display.dimensions = (width, height);
        self
    }

    /// Set DPI for export quality
    ///
    /// DPI only affects output resolution, not layout proportions.
    /// Higher DPI produces larger files with more detail.
    ///
    /// # Common values
    ///
    /// * 72-100: Screen/web display
    /// * 150: Good quality print
    /// * 300: Publication quality
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .size(6.4, 4.8)  // Same size in inches
    ///     .dpi(300)        // High resolution: 1920×1440 pixels
    ///     .save("print.png")?;
    /// ```
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.display.config.figure.dpi = dpi.max(72) as f32;
        self.display.dpi = dpi.max(72);
        // Update dimensions to reflect new DPI
        let (w, h) = self.display.config.canvas_size();
        self.display.dimensions = (w, h);
        self
    }

    /// Set maximum output resolution while preserving figure aspect ratio
    ///
    /// This method provides intuitive pixel-based sizing for screen outputs
    /// (PNG, GIF, video) while maintaining consistent visual styling with
    /// matplotlib/seaborn conventions.
    ///
    /// # How it works
    ///
    /// The `max_width` and `max_height` define a bounding box. The output
    /// dimensions are calculated to fit within this box while preserving
    /// the figure's aspect ratio. The internal DPI is automatically set
    /// to achieve the target pixel size.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Fit within 1920×1080 (e.g., for HD video)
    /// // Default 4:3 figure → outputs 1440×1080 (height-constrained)
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .max_resolution(1920, 1080)
    ///     .save("plot.png")?;
    ///
    /// // Fit within 1920×1440 (exact fit for 4:3)
    /// // → outputs 1920×1440 at 300 DPI
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .max_resolution(1920, 1440)
    ///     .save("hires.png")?;
    /// ```
    ///
    /// # Comparison with `dpi()`
    ///
    /// - Use `max_resolution()` for screen outputs where you care about pixel dimensions
    /// - Use `dpi()` for print outputs where you care about physical size and resolution
    ///
    /// Both produce identical results when properly configured:
    /// - `.max_resolution(1920, 1440)` ≈ `.dpi(300)` for default 6.4×4.8 figure
    pub fn max_resolution(mut self, max_width: u32, max_height: u32) -> Self {
        let fig_width = self.display.config.figure.width;
        let fig_height = self.display.config.figure.height;
        let aspect = fig_width / fig_height;

        // Try fitting to max_width
        let by_width = (max_width, (max_width as f32 / aspect).round() as u32);

        // Try fitting to max_height
        let by_height = ((max_height as f32 * aspect).round() as u32, max_height);

        // Choose the one that fits within both constraints
        let (width, height) = if by_width.1 <= max_height {
            by_width // Width-constrained
        } else {
            by_height // Height-constrained
        };

        // Calculate DPI to achieve these dimensions with current figure size
        let dpi = width as f32 / fig_width;

        self.display.config.figure.dpi = dpi;
        self.display.dpi = dpi as u32;
        self.display.dimensions = (width, height);
        self
    }

    /// Apply a style preset
    ///
    /// Style presets configure typography, line widths, and spacing
    /// for specific use cases.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .plot_style(PlotStyle::Publication)
    ///     .line(&x, &y)
    ///     .save("paper.png")?;
    /// ```
    pub fn plot_style(mut self, style: PlotStyle) -> Self {
        self.display.config = style.config();
        let (w, h) = self.display.config.canvas_size();
        self.display.dimensions = (w, h);
        self.display.dpi = self.display.config.figure.dpi as u32;
        self
    }

    /// Set the full plot configuration
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = PlotConfig::builder()
    ///     .figure(10.0, 7.5)
    ///     .font_size(14.0)
    ///     .build();
    /// Plot::new().plot_config(config).line(&x, &y).save("plot.png")?;
    /// ```
    pub fn plot_config(mut self, config: PlotConfig) -> Self {
        let (w, h) = config.canvas_size();
        self.display.dimensions = (w, h);
        self.display.dpi = config.figure.dpi as u32;
        self.display.config = config;
        self
    }

    /// Set the base font size in points
    ///
    /// All other font sizes (title, labels, ticks) scale relative to this.
    pub fn font_size(mut self, size: f32) -> Self {
        self.display.config.typography.base_size = size.max(4.0);
        self
    }

    /// Set the title font size in points (absolute)
    pub fn title_size(mut self, size: f32) -> Self {
        // Convert to scale factor
        self.display.config.typography.title_scale =
            size / self.display.config.typography.base_size;
        self
    }

    /// Set the data line width in points
    pub fn line_width_pt(mut self, width: f32) -> Self {
        self.display.config.lines.data_width = width.max(0.1);
        self
    }

    /// Get the current PlotConfig
    pub fn get_config(&self) -> &PlotConfig {
        &self.display.config
    }

    /// Adjust margins to tightly fit text with custom padding
    ///
    /// # Arguments
    ///
    /// * `pad` - Extra padding in points around text elements
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .tight_layout_pad(4.0)  // 4pt extra padding
    ///     .save("plot.png")?;
    /// ```
    pub fn tight_layout_pad(mut self, pad: f32) -> Self {
        let width = self.display.config.figure.width;
        let height = self.display.config.figure.height;

        // Estimate text sizes in inches
        let pad_in = crate::core::pt_to_in(pad);

        // Calculate required top margin (title)
        let top_margin = if self.display.title.is_some() {
            let title_size = self.display.config.typography.title_size();
            let title_pad = self.display.config.spacing.title_pad;
            crate::core::pt_to_in(title_size) + crate::core::pt_to_in(title_pad) + pad_in
        } else {
            pad_in.max(0.1) // Minimal margin
        };

        // Calculate required bottom margin (xlabel + tick labels)
        let tick_size = self.display.config.typography.tick_size();
        let label_size = self.display.config.typography.label_size();
        let tick_pad = self.display.config.spacing.tick_pad;
        let label_pad = self.display.config.spacing.label_pad;

        let bottom_margin = if self.display.xlabel.is_some() {
            crate::core::pt_to_in(tick_size)
                + crate::core::pt_to_in(tick_pad)
                + crate::core::pt_to_in(label_size)
                + crate::core::pt_to_in(label_pad)
                + pad_in
        } else {
            crate::core::pt_to_in(tick_size) + crate::core::pt_to_in(tick_pad) + pad_in
        };

        // Calculate required left margin (ylabel + tick labels)
        // Y-axis tick labels are typically 4-5 characters wide
        let estimated_tick_width = crate::core::pt_to_in(tick_size) * 4.0;

        let left_margin = if self.display.ylabel.is_some() {
            estimated_tick_width
                + crate::core::pt_to_in(tick_pad)
                + crate::core::pt_to_in(label_size)
                + crate::core::pt_to_in(label_pad)
                + pad_in
        } else {
            estimated_tick_width + crate::core::pt_to_in(tick_pad) + pad_in
        };

        // Right margin is minimal (just padding)
        let right_margin = pad_in.max(0.1);

        // Ensure margins don't exceed half the figure size
        let max_horizontal = width * 0.4;
        let max_vertical = height * 0.4;

        self.display.config.margins = MarginConfig::Fixed {
            left: left_margin.min(max_horizontal),
            right: right_margin.min(max_horizontal),
            top: top_margin.min(max_vertical),
            bottom: bottom_margin.min(max_vertical),
        };

        self
    }

    /// Calculate canvas dimensions from config
    pub(super) fn config_canvas_size(&self) -> (u32, u32) {
        self.display.config.figure.canvas_size()
    }

    pub(super) fn render_scale(&self) -> RenderScale {
        self.display.config.figure.render_scale()
    }

    pub(super) fn render_scale_with_device(&self, device_scale: f32) -> RenderScale {
        self.render_scale().with_host_scale(device_scale)
    }

    pub(super) fn x_data_to_pixel(
        plot_area: tiny_skia::Rect,
        x_data: f64,
        x_min: f64,
        x_max: f64,
    ) -> f32 {
        let x_range = x_max - x_min;
        if x_range.abs() < f64::EPSILON {
            plot_area.left() + plot_area.width() * 0.5
        } else {
            plot_area.left() + ((x_data - x_min) / x_range) as f32 * plot_area.width()
        }
    }

    pub(super) fn categorical_x_tick_pixels(
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        category_count: Option<usize>,
        violin_positions: &[f64],
    ) -> Option<Vec<f32>> {
        if !violin_positions.is_empty() {
            Some(
                violin_positions
                    .iter()
                    .map(|&x_pos| Self::x_data_to_pixel(plot_area, x_pos, x_min, x_max))
                    .collect(),
            )
        } else {
            category_count.map(|count| {
                (0..count)
                    .map(|index| Self::x_data_to_pixel(plot_area, index as f64, x_min, x_max))
                    .collect()
            })
        }
    }

    pub(super) fn resolved_series(&self, time: f64) -> Result<Vec<ResolvedSeries<'_>>> {
        self.series_mgr
            .series
            .iter()
            .map(|series| series.series_type.resolve_for_render(time))
            .collect()
    }

    pub(super) fn snapshot_series(&self, time: f64) -> Vec<PlotSeries> {
        self.series_mgr
            .series
            .iter()
            .cloned()
            .map(|mut series| {
                series.series_type = series.series_type.resolve(time);
                series
            })
            .collect()
    }

    pub(super) fn resolved_plot(&self, time: f64) -> Plot {
        let mut resolved = self.clone();
        resolved.series_mgr.series = self.snapshot_series(time);
        for series in &mut resolved.series_mgr.series {
            series.resolve_style_sources(time);
        }
        resolved.display.title = self
            .display
            .title
            .as_ref()
            .map(|title| data::PlotText::Static(title.resolve(time)));
        resolved.display.xlabel = self
            .display
            .xlabel
            .as_ref()
            .map(|label| data::PlotText::Static(label.resolve(time)));
        resolved.display.ylabel = self
            .display
            .ylabel
            .as_ref()
            .map(|label| data::PlotText::Static(label.resolve(time)));
        resolved
    }

    pub(crate) fn prepared_frame_plot(
        &self,
        size_px: (u32, u32),
        scale_factor: f32,
        time: f64,
    ) -> Plot {
        let device_scale = Self::sanitize_prepared_scale_factor(scale_factor);
        let dpi = (crate::core::REFERENCE_DPI * device_scale).round() as u32;

        self.resolved_plot(time)
            .dpi(dpi)
            .set_output_pixels(size_px.0, size_px.1)
    }

    pub(crate) fn sanitize_prepared_scale_factor(scale_factor: f32) -> f32 {
        let min_device_scale = crate::core::constants::dpi::MIN as f32 / crate::core::REFERENCE_DPI;
        if !scale_factor.is_finite() || scale_factor <= 0.0 {
            1.0
        } else {
            scale_factor.max(min_device_scale)
        }
    }

    pub(super) fn has_temporal_sources(&self) -> bool {
        self.display
            .title
            .as_ref()
            .is_some_and(|title| title.is_temporal())
            || self
                .display
                .xlabel
                .as_ref()
                .is_some_and(|label| label.is_temporal())
            || self
                .display
                .ylabel
                .as_ref()
                .is_some_and(|label| label.is_temporal())
            || self
                .series_mgr
                .series
                .iter()
                .any(PlotSeries::has_temporal_sources)
    }

    pub(crate) fn collect_reactive_versions(&self) -> Vec<u64> {
        let mut versions = Vec::new();
        for text in [
            self.display.title.as_ref(),
            self.display.xlabel.as_ref(),
            self.display.ylabel.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(version) = text.current_version() {
                versions.push(version);
            }
        }

        for series in &self.series_mgr.series {
            series.collect_source_versions(&mut versions);
        }

        versions
    }

    pub(crate) fn subscribe_push_updates(
        &self,
        callback: SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        for text in [
            self.display.title.as_ref(),
            self.display.xlabel.as_ref(),
            self.display.ylabel.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            text.subscribe_push_updates(Arc::clone(&callback), teardowns);
        }

        for series in &self.series_mgr.series {
            series.subscribe_push_updates(&callback, teardowns);
        }
    }

    pub(super) fn mark_reactive_sources_rendered(&self) {
        for series in &self.series_mgr.series {
            series.mark_rendered_sources();
        }
    }

    /// Get font size in pixels for rendering
    pub(super) fn font_size_px(&self, points: f32) -> f32 {
        self.render_scale().points_to_pixels(points)
    }

    /// Get line width in pixels for rendering
    pub(super) fn line_width_px(&self, points: f32) -> f32 {
        self.render_scale().points_to_pixels(points)
    }

    /// Convert legacy logical pixels (authored at REFERENCE_DPI) to output pixels.
    pub(super) fn logical_px(&self, logical_pixels: f32) -> f32 {
        self.render_scale().logical_pixels_to_pixels(logical_pixels)
    }

    /// Calculate DPI-scaled canvas dimensions
    /// **Deprecated**: Use config_canvas_size() instead
    pub(super) fn dpi_scaled_dimensions(&self) -> (u32, u32) {
        self.config_canvas_size()
    }

    /// Calculate DPI scaling factor
    /// **Deprecated**: Use config.figure.dpi with pt_to_px/in_to_px instead
    pub(super) fn dpi_scale(&self) -> f32 {
        self.render_scale().reference_scale()
    }

    /// Calculate DPI-scaled font size
    /// **Deprecated**: Use font_size_px() with config.typography instead
    pub fn dpi_scaled_font_size(&self, base_size: f32) -> f32 {
        pt_to_px(base_size, self.display.config.figure.dpi)
    }

    /// Calculate DPI-scaled line width
    /// **Deprecated**: Use line_width_px() with config.lines instead
    pub fn dpi_scaled_line_width(&self, base_width: f32) -> f32 {
        pt_to_px(base_width, self.display.config.figure.dpi)
    }

    /// Set margin around plot area
    ///
    /// The margin is specified as a fraction of the canvas size (0.0 to 0.5).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .margin(0.15)  // 15% margin on all sides
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .end_series()
    ///     .save("margin.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn margin(mut self, margin: f32) -> Self {
        self.layout.margin = Some(margin.clamp(0.0, 0.5));
        self
    }

    /// Enable/disable scientific notation on axes
    pub fn scientific_notation(mut self, enabled: bool) -> Self {
        self.layout.scientific_notation = enabled;
        self
    }
}
