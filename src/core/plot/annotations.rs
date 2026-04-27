use super::*;

impl Plot {
    /// Configure legend with position
    ///
    /// For matplotlib-compatible position codes, use `legend_position()` with `LegendPosition`.
    /// For automatic positioning (like matplotlib's `plt.legend()`), use `legend_best()`.
    pub fn legend(mut self, position: Position) -> Self {
        self.layout.legend.enabled = true;
        self.layout.legend.position = position;
        self
    }

    /// Configure legend with new position system
    ///
    /// Uses the matplotlib-compatible position codes including:
    /// - `LegendPosition::Best` - automatic position to minimize data overlap
    /// - `LegendPosition::UpperRight`, `UpperLeft`, etc. - standard positions
    /// - `LegendPosition::OutsideRight`, etc. - outside plot positions
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    /// let sin_y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    /// let cos_y: Vec<f64> = x.iter().map(|&v| v.cos()).collect();
    ///
    /// Plot::new()
    ///     .legend_position(LegendPosition::Best)
    ///     .line(&x, &sin_y).label("sin(x)")
    ///     .line(&x, &cos_y).label("cos(x)")
    ///     .end_series()
    ///     .save("legend.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Legend example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/legend.png)
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.layout.legend.enabled = true;
        // Convert LegendPosition to old Position for backward compatibility
        self.layout.legend.position = match position {
            LegendPosition::UpperRight | LegendPosition::Right => Position::TopRight,
            LegendPosition::UpperLeft => Position::TopLeft,
            LegendPosition::LowerLeft => Position::BottomLeft,
            LegendPosition::LowerRight => Position::BottomRight,
            LegendPosition::CenterLeft => Position::CenterLeft,
            LegendPosition::CenterRight => Position::CenterRight,
            LegendPosition::LowerCenter => Position::BottomCenter,
            LegendPosition::UpperCenter => Position::TopCenter,
            LegendPosition::Center => Position::Center,
            LegendPosition::Best => Position::Best,
            LegendPosition::OutsideRight
            | LegendPosition::OutsideLeft
            | LegendPosition::OutsideUpper
            | LegendPosition::OutsideLower => Position::TopRight,
            LegendPosition::Custom { x, y, .. } => Position::Custom { x, y },
        };
        self
    }

    /// Enable legend with "best" automatic positioning
    ///
    /// This is equivalent to matplotlib's `plt.legend()`. The legend will be placed
    /// in the position that minimizes overlap with data points.
    ///
    /// Note: Like matplotlib, legends are NOT shown unless you explicitly call
    /// `legend_best()` or `legend_position()`, even if series have labels.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    /// let sin_y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    /// let cos_y: Vec<f64> = x.iter().map(|&v| v.cos()).collect();
    ///
    /// // Labels define legend entries, legend_best() enables the legend
    /// Plot::new()
    ///     .line(&x, &sin_y).label("sin(x)")
    ///     .line(&x, &cos_y).label("cos(x)")
    ///     .end_series()       // Finish series chain
    ///     .legend_best()      // Like plt.legend() in matplotlib
    ///     .save("legend.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn legend_best(mut self) -> Self {
        self.layout.legend.enabled = true;
        self.layout.legend.position = Position::Best;
        self
    }

    /// Set legend font size
    pub fn legend_font_size(mut self, size: f32) -> Self {
        self.layout.legend.font_size = Some(size);
        self
    }

    /// Set legend corner radius for rounded corners
    ///
    /// A value of 0.0 gives sharp corners (default).
    /// Typical values are 3.0 to 8.0 for subtle rounded corners.
    pub fn legend_corner_radius(mut self, radius: f32) -> Self {
        self.layout.legend.corner_radius = Some(radius);
        self
    }

    /// Set number of legend columns
    ///
    /// - 1 column (default): vertical layout with all items stacked
    /// - 2+ columns: horizontal/multi-column layout
    ///
    /// For a single-row layout with N items, use `legend_columns(N)`.
    pub fn legend_columns(mut self, columns: usize) -> Self {
        self.layout.legend.columns = Some(columns.max(1));
        self
    }

    /// Enable/disable grid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// // Disable grid
    /// Plot::new()
    ///     .grid(false)
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .end_series()
    ///     .save("no_grid.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn grid(mut self, enabled: bool) -> Self {
        self.layout.grid_style.visible = enabled;
        self
    }

    /// Configure grid styling with unified GridStyle
    ///
    /// This is the preferred way to configure grid appearance, using
    /// the unified `GridStyle` configuration that ensures consistency
    /// across all plot types.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use ruviz::core::GridStyle;
    /// use ruviz::render::Color;
    ///
    /// Plot::new()
    ///     .with_grid_style(GridStyle::default().color(Color::BLUE).alpha(0.5))
    ///     .line(&x, &y)
    ///     .save("styled_grid.png")?;
    /// ```
    pub fn with_grid_style(mut self, style: GridStyle) -> Self {
        self.layout.grid_style = style;
        self
    }

    /// Set tick direction to inside (default)
    pub fn tick_direction_inside(mut self) -> Self {
        self.layout.tick_config.direction = TickDirection::Inside;
        self
    }

    /// Enable or disable tick marks and tick labels.
    ///
    /// Ticks are enabled by default and render on all four sides inward.
    pub fn ticks(mut self, enabled: bool) -> Self {
        self.layout.tick_config.enabled = enabled;
        self
    }

    /// Set tick direction to outside
    pub fn tick_direction_outside(mut self) -> Self {
        self.layout.tick_config.direction = TickDirection::Outside;
        self
    }

    /// Set tick direction to straddle the plot border.
    pub fn tick_direction_inout(mut self) -> Self {
        self.layout.tick_config.direction = TickDirection::InOut;
        self
    }

    /// Set which plot borders render tick marks.
    pub fn tick_sides(mut self, sides: TickSides) -> Self {
        self.layout.tick_config.sides = sides;
        self
    }

    /// Show ticks on all four sides of the plot area.
    pub fn ticks_all_sides(mut self) -> Self {
        self.layout.tick_config.sides = TickSides::all();
        self
    }

    /// Show ticks only on the bottom and left sides.
    pub fn ticks_bottom_left(mut self) -> Self {
        self.layout.tick_config.sides = TickSides::bottom_left();
        self
    }

    /// Enable or disable top ticks.
    pub fn show_top_ticks(mut self, enabled: bool) -> Self {
        self.layout.tick_config.sides.top = enabled;
        self
    }

    /// Enable or disable bottom ticks.
    pub fn show_bottom_ticks(mut self, enabled: bool) -> Self {
        self.layout.tick_config.sides.bottom = enabled;
        self
    }

    /// Enable or disable left ticks.
    pub fn show_left_ticks(mut self, enabled: bool) -> Self {
        self.layout.tick_config.sides.left = enabled;
        self
    }

    /// Enable or disable right ticks.
    pub fn show_right_ticks(mut self, enabled: bool) -> Self {
        self.layout.tick_config.sides.right = enabled;
        self
    }

    /// Set number of major ticks for both axes
    pub fn major_ticks(mut self, count: usize) -> Self {
        self.layout.tick_config.major_ticks_x = count;
        self.layout.tick_config.major_ticks_y = count;
        self
    }

    /// Set number of minor ticks between major ticks for both axes
    pub fn minor_ticks(mut self, count: usize) -> Self {
        self.layout.tick_config.minor_ticks_x = count;
        self.layout.tick_config.minor_ticks_y = count;
        self
    }

    /// Set number of major ticks for X axis
    pub fn major_ticks_x(mut self, count: usize) -> Self {
        self.layout.tick_config.major_ticks_x = count;
        self
    }

    /// Set number of minor ticks between major ticks for X axis
    pub fn minor_ticks_x(mut self, count: usize) -> Self {
        self.layout.tick_config.minor_ticks_x = count;
        self
    }

    /// Set number of major ticks for Y axis
    pub fn major_ticks_y(mut self, count: usize) -> Self {
        self.layout.tick_config.major_ticks_y = count;
        self
    }

    /// Set number of minor ticks between major ticks for Y axis
    pub fn minor_ticks_y(mut self, count: usize) -> Self {
        self.layout.tick_config.minor_ticks_y = count;
        self
    }

    /// Grid lines only at major ticks
    pub fn grid_major_only(mut self) -> Self {
        self.layout.tick_config.grid_mode = GridMode::MajorOnly;
        self
    }

    /// Grid lines only at minor ticks
    pub fn grid_minor_only(mut self) -> Self {
        self.layout.tick_config.grid_mode = GridMode::MinorOnly;
        self
    }

    /// Grid lines at both major and minor ticks
    pub fn grid_both(mut self) -> Self {
        self.layout.tick_config.grid_mode = GridMode::Both;
        self
    }

    /// Enable tight layout (automatic margin adjustment like matplotlib)
    ///
    /// When enabled, computes minimum required margins based on:
    /// - Title dimensions (if present)
    /// - X-axis label and tick label dimensions
    /// - Y-axis label and tick label dimensions
    ///
    /// The result is `Fixed` margins that eliminate dead space while
    /// ensuring no text is clipped.
    ///
    /// # Arguments
    ///
    /// * `enabled` - If true, compute tight margins; if false, use default proportional margins
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .title("My Plot")
    ///     .xlabel("X Values")
    ///     .ylabel("Y Values")
    ///     .line(&x, &y)
    ///     .tight_layout(true)  // Compute optimal margins
    ///     .save("tight.png")?;
    /// ```
    pub fn tight_layout(self, enabled: bool) -> Self {
        if enabled {
            self.tight_layout_pad(2.0) // Default 2pt padding
        } else {
            // Reset to default proportional margins
            let mut s = self;
            s.display.config.margins = MarginConfig::default();
            s
        }
    }

    /// Set grid color
    ///
    /// # Deprecated
    ///
    /// Use `with_grid_style(GridStyle::default().color(color))` instead for
    /// unified grid configuration.
    #[deprecated(since = "0.2.0", note = "Use with_grid_style() instead")]
    pub fn grid_color(mut self, color: Color) -> Self {
        self.layout.grid_style.color = color;
        self
    }

    /// Set grid line style
    ///
    /// # Deprecated
    ///
    /// Use `with_grid_style(GridStyle::default().line_style(style))` instead for
    /// unified grid configuration.
    #[deprecated(since = "0.2.0", note = "Use with_grid_style() instead")]
    pub fn grid_line_style(mut self, style: LineStyle) -> Self {
        self.layout.grid_style.line_style = style;
        self
    }

    /// Set grid line width
    pub fn grid_line_width(mut self, width: f32) -> Self {
        self.layout.grid_style.line_width = width;
        self
    }

    /// Set grid transparency
    pub fn grid_alpha(mut self, alpha: f32) -> Self {
        self.layout.grid_style.alpha = alpha;
        self
    }

    // ========== Annotation Methods ==========

    /// Add a text annotation at data coordinates
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in data space
    /// * `y` - Y coordinate in data space
    /// * `text` - Text content to display
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .text(2.5, 100.0, "Peak value")
    ///     .save("annotated.png")?;
    /// ```
    pub fn text<S: Into<String>>(mut self, x: f64, y: f64, text: S) -> Self {
        self.annotations.push(Annotation::text(x, y, text));
        self
    }

    /// Add a text annotation with custom styling
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let style = TextStyle::new()
    ///     .font_size(14.0)
    ///     .color(Color::RED)
    ///     .align(TextAlign::Left);
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .text_styled(2.5, 100.0, "Peak value", style)
    ///     .save("annotated.png")?;
    /// ```
    pub fn text_styled<S: Into<String>>(
        mut self,
        x: f64,
        y: f64,
        text: S,
        style: TextStyle,
    ) -> Self {
        self.annotations
            .push(Annotation::text_styled(x, y, text, style));
        self
    }

    /// Add an arrow annotation between two points
    ///
    /// The arrow points from (x1, y1) to (x2, y2).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .arrow(1.0, 50.0, 2.5, 100.0)  // Arrow pointing to peak
    ///     .save("annotated.png")?;
    /// ```
    pub fn arrow(mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        self.annotations.push(Annotation::arrow(x1, y1, x2, y2));
        self
    }

    /// Add an arrow annotation with custom styling
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let style = ArrowStyle::new()
    ///     .color(Color::RED)
    ///     .line_width(2.0)
    ///     .head_style(ArrowHead::Stealth);
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .arrow_styled(1.0, 50.0, 2.5, 100.0, style)
    ///     .save("annotated.png")?;
    /// ```
    pub fn arrow_styled(mut self, x1: f64, y1: f64, x2: f64, y2: f64, style: ArrowStyle) -> Self {
        self.annotations
            .push(Annotation::arrow_styled(x1, y1, x2, y2, style));
        self
    }

    /// Add a horizontal reference line spanning the plot width
    ///
    /// Uses dashed gray style by default.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .hline(50.0)  // Add reference line at y=50
    ///     .save("annotated.png")?;
    /// ```
    pub fn hline(mut self, y: f64) -> Self {
        self.annotations.push(Annotation::hline(y));
        self
    }

    /// Add a horizontal reference line with custom styling
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .hline_styled(50.0, Color::RED, 2.0, LineStyle::Solid)
    ///     .save("annotated.png")?;
    /// ```
    pub fn hline_styled(mut self, y: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.annotations
            .push(Annotation::hline_styled(y, color, width, style));
        self
    }

    /// Add a vertical reference line spanning the plot height
    ///
    /// Uses dashed gray style by default.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .vline(2.5)  // Add reference line at x=2.5
    ///     .save("annotated.png")?;
    /// ```
    pub fn vline(mut self, x: f64) -> Self {
        self.annotations.push(Annotation::vline(x));
        self
    }

    /// Add a vertical reference line with custom styling
    pub fn vline_styled(mut self, x: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.annotations
            .push(Annotation::vline_styled(x, color, width, style));
        self
    }

    /// Add a rectangle annotation in data coordinates
    ///
    /// # Arguments
    ///
    /// * `x` - Left X coordinate in data space
    /// * `y` - Bottom Y coordinate in data space
    /// * `width` - Width in data units
    /// * `height` - Height in data units
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .rect(1.0, 20.0, 2.0, 60.0)  // Highlight region
    ///     .save("annotated.png")?;
    /// ```
    pub fn rect(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.annotations
            .push(Annotation::rectangle(x, y, width, height));
        self
    }

    /// Add a rectangle annotation with custom styling
    pub fn rect_styled(
        mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: ShapeStyle,
    ) -> Self {
        self.annotations
            .push(Annotation::rectangle_styled(x, y, width, height, style));
        self
    }

    /// Add a fill between two curves
    ///
    /// Fills the region between y1 and y2 at each x position.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let x = vec![1.0, 2.0, 3.0, 4.0];
    /// let y_upper = vec![10.0, 15.0, 12.0, 18.0];
    /// let y_lower = vec![5.0, 8.0, 6.0, 9.0];
    ///
    /// Plot::new()
    ///     .line(&x, &y_upper)
    ///     .fill_between(&x, &y_lower, &y_upper)
    ///     .save("filled.png")?;
    /// ```
    pub fn fill_between(mut self, x: &[f64], y1: &[f64], y2: &[f64]) -> Self {
        self.annotations.push(Annotation::fill_between(
            x.to_vec(),
            y1.to_vec(),
            y2.to_vec(),
        ));
        self
    }

    /// Add a fill between a curve and a constant baseline
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .fill_to_baseline(&x, &y, 0.0)  // Fill to y=0
    ///     .save("filled.png")?;
    /// ```
    pub fn fill_to_baseline(mut self, x: &[f64], y: &[f64], baseline: f64) -> Self {
        self.annotations.push(Annotation::fill_to_baseline(
            x.to_vec(),
            y.to_vec(),
            baseline,
        ));
        self
    }

    /// Add a fill between with custom styling
    pub fn fill_between_styled(
        mut self,
        x: &[f64],
        y1: &[f64],
        y2: &[f64],
        style: FillStyle,
        where_positive: bool,
    ) -> Self {
        self.annotations.push(Annotation::fill_between_styled(
            x.to_vec(),
            y1.to_vec(),
            y2.to_vec(),
            style,
            where_positive,
        ));
        self
    }

    /// Add a horizontal span (shaded vertical region)
    ///
    /// Highlights a vertical region from x_min to x_max across the full plot height.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .axvspan(2.0, 3.0)  // Highlight region between x=2 and x=3
    ///     .save("annotated.png")?;
    /// ```
    pub fn axvspan(mut self, x_min: f64, x_max: f64) -> Self {
        self.annotations.push(Annotation::hspan(x_min, x_max));
        self
    }

    /// Add a vertical span (shaded horizontal region)
    ///
    /// Highlights a horizontal region from y_min to y_max across the full plot width.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .axhspan(20.0, 80.0)  // Highlight region between y=20 and y=80
    ///     .save("annotated.png")?;
    /// ```
    pub fn axhspan(mut self, y_min: f64, y_max: f64) -> Self {
        self.annotations.push(Annotation::vspan(y_min, y_max));
        self
    }

    /// Add a generic annotation
    ///
    /// Use this method to add pre-constructed annotations.
    pub fn annotate(mut self, annotation: Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Get annotations for iteration (used during rendering)
    pub fn get_annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    // ========== End Annotation Methods ==========

    /// Enable or disable Typst rendering for all plot text surfaces.
    ///
    /// When enabled, all static text surfaces (title, axis labels, tick labels,
    /// legend labels, category labels, and annotations) are rendered through Typst.
    ///
    /// Requires the `typst-math` feature.
    /// If your crate makes Typst optional, guard this call with
    /// `#[cfg(feature = "typst-math")]`.
    #[cfg(feature = "typst-math")]
    #[cfg_attr(docsrs, doc(cfg(feature = "typst-math")))]
    pub fn typst(mut self, enabled: bool) -> Self {
        self.display.text_engine = if enabled {
            TextEngineMode::Typst
        } else {
            TextEngineMode::Plain
        };
        self
    }

    /// Compatibility shim for older code paths.
    ///
    /// This method is currently a no-op on [`Plot`]. To set transparency on a
    /// series, call `.alpha(...)` on the returned series builder after adding a
    /// plot series.
    #[deprecated(
        since = "0.1.4",
        note = "Plot::alpha() is a no-op compatibility shim; call .alpha(...) on the returned series builder instead"
    )]
    pub fn alpha(self, _alpha: f32) -> Self {
        // This would be handled by the series builder
        // Keeping for API compatibility
        self
    }
}
