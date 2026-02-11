//! SVG export functionality
//!
//! Provides vector-based SVG export for plots with full visual fidelity.
//! This renderer is also used as the intermediate format for PDF export.

use crate::core::{
    Legend, LegendItem, LegendItemType, LegendPosition, LegendSpacingPixels, LegendStyle,
    PlottingError, Result, find_best_position, plot::TextEngineMode,
};
use crate::render::{
    Color, FontConfig, FontFamily, LineStyle, MarkerStyle, TextRenderer,
    text_anchor::{TextPlacementMetrics, center_anchor_to_baseline, top_anchor_to_baseline},
    typst_text::{self, TypstBackendKind, TypstTextAnchor},
};
use std::borrow::Cow;
use std::fmt::Write as FmtWrite;
use std::path::Path;

/// SVG renderer for vector-based plot export
pub struct SvgRenderer {
    width: f32,
    height: f32,
    content: String,
    defs: String,
    clip_id_counter: u32,
    /// DPI scale factor (1.0 = 100 DPI base)
    dpi_scale: f32,
    /// Active text rendering engine.
    text_engine_mode: TextEngineMode,
    /// Plain text metrics for anchor conversion.
    text_renderer: TextRenderer,
}

impl SvgRenderer {
    fn sanitize_dpi_scale(dpi_scale: f32) -> f32 {
        if dpi_scale.is_finite() && dpi_scale > 0.0 {
            dpi_scale
        } else {
            log::warn!(
                "Invalid dpi_scale ({:?}) for SvgRenderer; falling back to 1.0",
                dpi_scale
            );
            1.0
        }
    }

    /// Create a new SVG renderer with specified dimensions
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            content: String::new(),
            defs: String::new(),
            clip_id_counter: 0,
            dpi_scale: 1.0, // Default to 100 DPI base
            text_engine_mode: TextEngineMode::Plain,
            text_renderer: TextRenderer::new(),
        }
    }

    /// Set the DPI scale factor (dpi / 100.0)
    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        self.dpi_scale = Self::sanitize_dpi_scale(dpi_scale);
    }

    /// Get the DPI scale factor
    pub fn dpi_scale(&self) -> f32 {
        self.dpi_scale
    }

    /// Set text rendering backend mode.
    pub fn set_text_engine_mode(&mut self, mode: TextEngineMode) {
        self.text_engine_mode = mode;
    }

    /// Get text rendering backend mode.
    pub fn text_engine_mode(&self) -> TextEngineMode {
        self.text_engine_mode
    }

    /// Map renderer font size to Typst size units.
    ///
    /// Typst SVG output aligns with existing plot sizing when using the
    /// same numeric size value.
    fn typst_size_pt(&self, size_px: f32) -> f32 {
        size_px.max(0.1)
    }

    /// Get a unique clip path ID
    fn next_clip_id(&mut self) -> String {
        self.clip_id_counter += 1;
        format!("clip{}", self.clip_id_counter)
    }

    /// Convert Color to SVG color string
    fn color_to_svg(&self, color: Color) -> String {
        if color.a == 255 {
            format!("rgb({},{},{})", color.r, color.g, color.b)
        } else {
            format!(
                "rgba({},{},{},{:.3})",
                color.r,
                color.g,
                color.b,
                color.a as f32 / 255.0
            )
        }
    }

    /// Convert LineStyle to SVG stroke-dasharray
    fn line_style_to_dasharray(&self, style: &LineStyle) -> Option<String> {
        self.scaled_dash_pattern(style).map(|pattern| {
            pattern
                .iter()
                .map(|v| self.format_dash_value(*v))
                .collect::<Vec<_>>()
                .join(",")
        })
    }

    /// Convert style to a DPI-scaled dash pattern.
    ///
    /// Dash values are sourced from `LineStyle::to_dash_array()` (100-DPI
    /// baseline), then scaled so SVG spacing matches raster output.
    fn scaled_dash_pattern(&self, style: &LineStyle) -> Option<Vec<f32>> {
        let scale = self.dpi_scale;
        style
            .to_dash_array()
            .map(|base| base.into_iter().map(|segment| segment * scale).collect())
    }

    fn format_dash_value(&self, value: f32) -> String {
        if (value - value.round()).abs() < 1e-6 {
            return (value.round() as i32).to_string();
        }

        let mut s = format!("{:.3}", value);
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    fn strip_xml_declaration<'a>(&self, svg: &'a str) -> &'a str {
        if let Some(start) = svg.find("<svg") {
            &svg[start..]
        } else {
            svg
        }
    }

    fn generated_label<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if matches!(self.text_engine_mode, TextEngineMode::Typst) {
            Cow::Owned(typst_text::literal_text_snippet(text))
        } else {
            Cow::Borrowed(text)
        }
    }

    fn plain_text_metrics(&self, text: &str, font_size: f32) -> Result<TextPlacementMetrics> {
        let config = FontConfig::new(FontFamily::SansSerif, font_size);
        self.text_renderer.measure_text_placement(text, &config)
    }

    fn measure_text_for_layout(&self, text: &str, font_size: f32) -> Result<(f32, f32)> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let metrics = self.plain_text_metrics(text, font_size)?;
                Ok((metrics.width, metrics.height))
            }
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(font_size);
                typst_text::measure_text(
                    text,
                    size_pt,
                    Color::BLACK,
                    0.0,
                    TypstBackendKind::Svg,
                    "SVG text measurement",
                )
            }
        }
    }

    /// Draw a filled or stroked rectangle
    pub fn draw_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
    ) {
        let color_str = self.color_to_svg(color);
        if filled {
            writeln!(
                self.content,
                r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                x, y, width, height, color_str
            )
            .unwrap();
        } else {
            writeln!(
                self.content,
                r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="{}" stroke-width="1"/>"#,
                x, y, width, height, color_str
            )
            .unwrap();
        }
    }

    /// Draw a filled or stroked rectangle with rounded corners
    pub fn draw_rounded_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        corner_radius: f32,
        color: Color,
        filled: bool,
    ) {
        let color_str = self.color_to_svg(color);
        // Clamp radius to half of the smallest dimension
        let max_radius = (width.min(height) / 2.0).max(0.0);
        let radius = corner_radius.min(max_radius);

        if filled {
            writeln!(
                self.content,
                r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" ry="{:.2}" fill="{}"/>"#,
                x, y, width, height, radius, radius, color_str
            )
            .unwrap();
        } else {
            writeln!(
                self.content,
                r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" ry="{:.2}" fill="none" stroke="{}" stroke-width="1"/>"#,
                x, y, width, height, radius, radius, color_str
            )
            .unwrap();
        }
    }

    /// Draw a line segment
    pub fn draw_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
    ) {
        let color_str = self.color_to_svg(color);
        let dasharray = self.line_style_to_dasharray(&style);

        let dash_attr = dasharray
            .map(|d| format!(r#" stroke-dasharray="{}""#, d))
            .unwrap_or_default();

        writeln!(
            self.content,
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}"{} stroke-linecap="round"/>"#,
            x1, y1, x2, y2, color_str, width, dash_attr
        )
        .unwrap();
    }

    /// Draw a polyline (connected line segments)
    pub fn draw_polyline(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: LineStyle,
    ) {
        if points.len() < 2 {
            return;
        }

        let color_str = self.color_to_svg(color);
        let dasharray = self.line_style_to_dasharray(&style);

        let dash_attr = dasharray
            .map(|d| format!(r#" stroke-dasharray="{}""#, d))
            .unwrap_or_default();

        let points_str: String = points
            .iter()
            .map(|(x, y)| format!("{:.2},{:.2}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(
            self.content,
            r#"  <polyline points="{}" fill="none" stroke="{}" stroke-width="{:.2}"{} stroke-linecap="round" stroke-linejoin="round"/>"#,
            points_str, color_str, width, dash_attr
        )
        .unwrap();
    }

    /// Draw a filled circle
    pub fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, color: Color, filled: bool) {
        let color_str = self.color_to_svg(color);
        if filled {
            writeln!(
                self.content,
                r#"  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}"/>"#,
                cx, cy, r, color_str
            )
            .unwrap();
        } else {
            writeln!(
                self.content,
                r#"  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="none" stroke="{}" stroke-width="1"/>"#,
                cx, cy, r, color_str
            )
            .unwrap();
        }
    }

    /// Draw a marker (circle) at a point
    pub fn draw_marker(&mut self, x: f32, y: f32, size: f32, color: Color) {
        self.draw_circle(x, y, size / 2.0, color, true);
    }

    /// Draw text at specified position.
    /// `y` is interpreted as the top of the text rendering area.
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let color_str = self.color_to_svg(color);
                let escaped_text = self.escape_xml(text);
                let metrics = self.plain_text_metrics(text, size)?;
                let baseline_y = top_anchor_to_baseline(y, metrics);
                writeln!(
                    self.content,
                    r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}">{}</text>"#,
                    x, baseline_y, size, color_str, escaped_text
                )
                .unwrap();
                Ok(())
            }
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered =
                    typst_text::render_svg(text, size_pt, color, 0.0, "SVG text rendering")?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopLeft,
                );
                let embedded_svg = self.strip_xml_declaration(&rendered.svg);
                writeln!(
                    self.content,
                    r#"  <g data-ruviz-text-engine="typst" transform="translate({:.2},{:.2})">{}</g>"#,
                    draw_x, draw_y, embedded_svg
                )
                .unwrap();
                Ok(())
            }
        }
    }

    /// Draw text centered at specified position.
    /// `y` is interpreted as the top of the text rendering area.
    pub fn draw_text_centered(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let color_str = self.color_to_svg(color);
                let escaped_text = self.escape_xml(text);
                let metrics = self.plain_text_metrics(text, size)?;
                let baseline_y = top_anchor_to_baseline(y, metrics);
                writeln!(
                    self.content,
                    r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text>"#,
                    x, baseline_y, size, color_str, escaped_text
                )
                .unwrap();
                Ok(())
            }
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_svg(
                    text,
                    size_pt,
                    color,
                    0.0,
                    "SVG centered text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopCenter,
                );
                let embedded_svg = self.strip_xml_declaration(&rendered.svg);
                writeln!(
                    self.content,
                    r#"  <g data-ruviz-text-engine="typst" transform="translate({:.2},{:.2})">{}</g>"#,
                    draw_x, draw_y, embedded_svg
                )
                .unwrap();
                Ok(())
            }
        }
    }

    /// Draw rotated text (typically for Y-axis labels)
    pub fn draw_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        angle: f32,
    ) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let color_str = self.color_to_svg(color);
                let escaped_text = self.escape_xml(text);
                let metrics = self.plain_text_metrics(text, size)?;
                let center_baseline_y = center_anchor_to_baseline(0.0, metrics);
                writeln!(
                    self.content,
                    r#"  <g transform="translate({:.2},{:.2}) rotate({:.1})"><text x="0" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text></g>"#,
                    x, y, angle, center_baseline_y, size, color_str, escaped_text
                )
                .unwrap();
                Ok(())
            }
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_svg(
                    text,
                    size_pt,
                    color,
                    angle,
                    "SVG rotated text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::Center,
                );
                let embedded_svg = self.strip_xml_declaration(&rendered.svg);
                writeln!(
                    self.content,
                    r#"  <g data-ruviz-text-engine="typst" transform="translate({:.2},{:.2})">{}</g>"#,
                    draw_x, draw_y, embedded_svg
                )
                .unwrap();
                Ok(())
            }
        }
    }

    /// Draw grid lines
    pub fn draw_grid(
        &mut self,
        x_ticks: &[f32],
        y_ticks: &[f32],
        plot_left: f32,
        plot_right: f32,
        plot_top: f32,
        plot_bottom: f32,
        color: Color,
        style: LineStyle,
        line_width: f32,
    ) {
        // Vertical grid lines
        for &x in x_ticks {
            if x >= plot_left && x <= plot_right {
                self.draw_line(
                    x,
                    plot_top,
                    x,
                    plot_bottom,
                    color,
                    line_width,
                    style.clone(),
                );
            }
        }

        // Horizontal grid lines
        for &y in y_ticks {
            if y >= plot_top && y <= plot_bottom {
                self.draw_line(
                    plot_left,
                    y,
                    plot_right,
                    y,
                    color,
                    line_width,
                    style.clone(),
                );
            }
        }
    }

    /// Draw axis lines and tick marks
    pub fn draw_axes(
        &mut self,
        plot_left: f32,
        plot_right: f32,
        plot_top: f32,
        plot_bottom: f32,
        x_ticks: &[f32],
        y_ticks: &[f32],
        color: Color,
        tick_outside: bool,
    ) {
        // Scale widths and sizes by DPI (base values at 100 DPI)
        let axis_width = 1.5 * self.dpi_scale;
        let major_tick_size = 6.0 * self.dpi_scale;
        let tick_width = 1.0 * self.dpi_scale;
        let tick_dir = if tick_outside { 1.0 } else { -1.0 };

        // Draw X-axis (bottom)
        self.draw_line(
            plot_left,
            plot_bottom,
            plot_right,
            plot_bottom,
            color,
            axis_width,
            LineStyle::Solid,
        );

        // Draw Y-axis (left)
        self.draw_line(
            plot_left,
            plot_top,
            plot_left,
            plot_bottom,
            color,
            axis_width,
            LineStyle::Solid,
        );

        // Draw X-axis tick marks
        for &x in x_ticks {
            if x >= plot_left && x <= plot_right {
                let tick_end = plot_bottom + major_tick_size * tick_dir;
                self.draw_line(
                    x,
                    plot_bottom,
                    x,
                    tick_end,
                    color,
                    tick_width,
                    LineStyle::Solid,
                );
            }
        }

        // Draw Y-axis tick marks
        for &y in y_ticks {
            if y >= plot_top && y <= plot_bottom {
                let tick_end = plot_left - major_tick_size * tick_dir;
                self.draw_line(
                    plot_left,
                    y,
                    tick_end,
                    y,
                    color,
                    tick_width,
                    LineStyle::Solid,
                );
            }
        }
    }

    /// Draw axis tick labels
    pub fn draw_tick_labels(
        &mut self,
        x_ticks: &[f32],
        x_labels: &[String],
        y_ticks: &[f32],
        y_labels: &[String],
        plot_left: f32,
        plot_right: f32,
        plot_top: f32,
        plot_bottom: f32,
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        color: Color,
        font_size: f32,
    ) -> Result<()> {
        // X-axis labels
        for (i, &x) in x_ticks.iter().enumerate() {
            if x >= plot_left && x <= plot_right {
                if let Some(label) = x_labels.get(i) {
                    let label_snippet = self.generated_label(label);
                    let (text_width, _) =
                        self.measure_text_for_layout(&label_snippet, font_size)?;
                    let label_x = (x - text_width / 2.0).max(0.0).min(self.width - text_width);
                    self.draw_text(&label_snippet, label_x, xtick_baseline_y, font_size, color)?;
                }
            }
        }

        // Y-axis labels
        for (i, &y) in y_ticks.iter().enumerate() {
            if y >= plot_top && y <= plot_bottom {
                if let Some(label) = y_labels.get(i) {
                    let label_snippet = self.generated_label(label);
                    let (text_width, text_height) =
                        self.measure_text_for_layout(&label_snippet, font_size)?;
                    let gap = font_size * 0.5;
                    let min_x = font_size * 0.5;
                    let label_x = (ytick_right_x - text_width - gap).max(min_x);
                    let centered_y = y - text_height / 2.0;
                    self.draw_text(&label_snippet, label_x, centered_y, font_size, color)?;
                }
            }
        }

        Ok(())
    }

    /// Draw legend
    pub fn draw_legend(
        &mut self,
        items: &[(String, Color)],
        x: f32,
        y: f32,
        font_size: f32,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let item_height = font_size + 6.0;
        let legend_width = 120.0;
        let legend_height = items.len() as f32 * item_height + 10.0;
        let swatch_size = 12.0;
        let swatch_gap = 8.0;

        // Draw legend background
        self.draw_rectangle(
            x,
            y,
            legend_width,
            legend_height,
            Color::new_rgba(255, 255, 255, 230),
            true,
        );
        self.draw_rectangle(
            x,
            y,
            legend_width,
            legend_height,
            Color::new_rgba(0, 0, 0, 100),
            false,
        );

        // Draw legend items
        for (i, (label, color)) in items.iter().enumerate() {
            let item_y = y + 8.0 + i as f32 * item_height;

            // Draw color swatch
            self.draw_rectangle(x + 8.0, item_y, swatch_size, swatch_size, *color, true);

            // Draw label
            self.draw_text(
                label,
                x + 8.0 + swatch_size + swatch_gap,
                item_y + swatch_size / 2.0 - font_size * 0.5,
                font_size,
                Color::BLACK,
            )?;
        }

        Ok(())
    }

    // =========================================================================
    // New Legend System with proper handle rendering
    // =========================================================================

    /// Draw a line handle in the legend (for line series)
    fn draw_legend_line_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        style: &LineStyle,
        width: f32,
    ) {
        let dash_attr = self
            .line_style_to_dasharray(style)
            .map(|pattern| format!(r#" stroke-dasharray="{}""#, pattern))
            .unwrap_or_default();

        let color_str = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}"{}/>"#,
            x, y, x + length, y, color_str, width, dash_attr
        )
        .unwrap();
    }

    /// Draw a scatter/marker handle in the legend
    fn draw_legend_scatter_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        marker: &MarkerStyle,
        size: f32,
    ) {
        let center_x = x + length / 2.0;
        let color_str = self.color_to_svg(color);
        let radius = size / 2.0;

        match marker {
            MarkerStyle::Circle => {
                writeln!(
                    self.content,
                    r#"  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}"/>"#,
                    center_x, y, radius, color_str
                )
                .unwrap();
            }
            MarkerStyle::Square => {
                writeln!(
                    self.content,
                    r#"  <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                    center_x - radius,
                    y - radius,
                    size,
                    size,
                    color_str
                )
                .unwrap();
            }
            MarkerStyle::Triangle => {
                let x1 = center_x;
                let y1 = y - radius;
                let x2 = center_x - radius;
                let y2 = y + radius;
                let x3 = center_x + radius;
                let y3 = y + radius;
                writeln!(
                    self.content,
                    r#"  <polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}"/>"#,
                    x1, y1, x2, y2, x3, y3, color_str
                )
                .unwrap();
            }
            MarkerStyle::Diamond => {
                let x1 = center_x;
                let y1 = y - radius;
                let x2 = center_x + radius;
                let y2 = y;
                let x3 = center_x;
                let y3 = y + radius;
                let x4 = center_x - radius;
                let y4 = y;
                writeln!(
                    self.content,
                    r#"  <polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}"/>"#,
                    x1, y1, x2, y2, x3, y3, x4, y4, color_str
                )
                .unwrap();
            }
            _ => {
                // Default to circle for other marker types
                writeln!(
                    self.content,
                    r#"  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}"/>"#,
                    center_x, y, radius, color_str
                )
                .unwrap();
            }
        }
    }

    /// Draw a bar handle in the legend
    fn draw_legend_bar_handle(&mut self, x: f32, y: f32, length: f32, height: f32, color: Color) {
        let rect_y = y - height / 2.0;
        self.draw_rectangle(x, rect_y, length, height, color, true);
    }

    /// Draw a line+marker handle in the legend
    fn draw_legend_line_marker_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        line_style: &LineStyle,
        line_width: f32,
        marker: &MarkerStyle,
        marker_size: f32,
    ) {
        self.draw_legend_line_handle(x, y, length, color, line_style, line_width);
        self.draw_legend_scatter_handle(x, y, length, color, marker, marker_size);
    }

    /// Draw a legend handle based on the item type
    fn draw_legend_handle(
        &mut self,
        item: &LegendItem,
        x: f32,
        y: f32,
        spacing: &LegendSpacingPixels,
    ) {
        let handle_length = spacing.handle_length;
        let handle_height = spacing.handle_height;

        match &item.item_type {
            LegendItemType::Line { style, width } => {
                self.draw_legend_line_handle(x, y, handle_length, item.color, style, *width);
            }
            LegendItemType::Scatter { marker, size } => {
                self.draw_legend_scatter_handle(x, y, handle_length, item.color, marker, *size);
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
            } => {
                self.draw_legend_line_marker_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    line_style,
                    *line_width,
                    marker,
                    *marker_size,
                );
            }
            LegendItemType::Bar | LegendItemType::Histogram => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color);
            }
            LegendItemType::Area { edge_color } => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color);
                if let Some(edge) = edge_color {
                    let rect_y = y - handle_height / 2.0;
                    self.draw_rectangle(x, rect_y, handle_length, handle_height, *edge, false);
                }
            }
            LegendItemType::ErrorBar => {
                // Draw vertical error bar with marker (matplotlib-style)
                let center_x = x + handle_length / 2.0;
                let error_height = handle_height * 0.8;
                let half_error = error_height / 2.0;
                let cap_width = handle_height * 0.5;
                let half_cap = cap_width / 2.0;
                let color_str = self.color_to_svg(item.color);

                // Vertical error bar line
                writeln!(
                    self.content,
                    r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5"/>"#,
                    center_x, y - half_error, center_x, y + half_error, color_str
                )
                .unwrap();
                // Top cap (horizontal)
                writeln!(
                    self.content,
                    r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5"/>"#,
                    center_x - half_cap, y - half_error, center_x + half_cap, y - half_error, color_str
                )
                .unwrap();
                // Bottom cap (horizontal)
                writeln!(
                    self.content,
                    r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5"/>"#,
                    center_x - half_cap, y + half_error, center_x + half_cap, y + half_error, color_str
                )
                .unwrap();
                // Draw marker in center
                let marker_size = handle_height * 0.4;
                self.draw_marker(center_x, y, marker_size, item.color);
            }
        }

        // If the series has attached error bars (not ErrorBar type), overlay error bar indicator
        if item.has_error_bars && !matches!(item.item_type, LegendItemType::ErrorBar) {
            let center_x = x + handle_length / 2.0;
            let error_height = handle_height * 0.7; // Slightly smaller for overlay
            let half_error = error_height / 2.0;
            let cap_width = handle_height * 0.4;
            let half_cap = cap_width / 2.0;
            let color_str = self.color_to_svg(item.color);

            // Vertical error bar line
            writeln!(
                self.content,
                r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.0"/>"#,
                center_x, y - half_error, center_x, y + half_error, color_str
            )
            .unwrap();
            // Top cap (horizontal)
            writeln!(
                self.content,
                r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.0"/>"#,
                center_x - half_cap, y - half_error, center_x + half_cap, y - half_error, color_str
            )
            .unwrap();
            // Bottom cap (horizontal)
            writeln!(
                self.content,
                r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.0"/>"#,
                center_x - half_cap, y + half_error, center_x + half_cap, y + half_error, color_str
            )
            .unwrap();
        }
    }

    /// Draw legend frame with background and optional border
    fn draw_legend_frame(&mut self, x: f32, y: f32, width: f32, height: f32, style: &LegendStyle) {
        if !style.visible {
            return;
        }

        let radius = style.effective_corner_radius();

        // Draw shadow if enabled
        if style.shadow {
            let (shadow_dx, shadow_dy) = style.shadow_offset;
            if radius > 0.0 {
                self.draw_rounded_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    radius,
                    style.shadow_color,
                    true,
                );
            } else {
                self.draw_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    style.shadow_color,
                    true,
                );
            }
        }

        // Draw background with alpha applied
        let face_color = style.effective_face_color();
        if radius > 0.0 {
            self.draw_rounded_rectangle(x, y, width, height, radius, face_color, true);
        } else {
            self.draw_rectangle(x, y, width, height, face_color, true);
        }

        // Draw border if specified
        if let Some(edge_color) = style.edge_color {
            if radius > 0.0 {
                self.draw_rounded_rectangle(x, y, width, height, radius, edge_color, false);
            } else {
                self.draw_rectangle(x, y, width, height, edge_color, false);
            }
        }
    }

    /// Draw legend with full LegendItem support
    ///
    /// This is the new legend drawing method that properly renders different
    /// series types with their correct visual handles.
    pub fn draw_legend_full(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: (f32, f32, f32, f32), // (left, top, right, bottom)
        data_bboxes: Option<&[(f32, f32, f32, f32)]>,
    ) -> Result<()> {
        if items.is_empty() || !legend.enabled {
            return Ok(());
        }

        let spacing = legend.spacing.to_pixels(legend.font_size);
        let (legend_width, legend_height, label_width) = match self.text_engine_mode {
            TextEngineMode::Plain => {
                let char_width = legend.font_size * 0.6;
                let (width, height) = legend.calculate_size(items, char_width);
                let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);
                (width, height, max_label_len as f32 * char_width)
            }
            TextEngineMode::Typst => {
                let mut max_label_width = 0.0_f32;
                for item in items {
                    let (w, _) = self.measure_text_for_layout(&item.label, legend.font_size)?;
                    max_label_width = max_label_width.max(w);
                }
                let item_width = spacing.handle_length + spacing.handle_text_pad + max_label_width;
                let items_per_col = items.len().div_ceil(legend.columns);
                let content_width = item_width * legend.columns as f32
                    + (legend.columns.saturating_sub(1)) as f32 * spacing.column_spacing;
                let content_height = items_per_col as f32 * legend.font_size
                    + (items_per_col.saturating_sub(1)) as f32 * spacing.label_spacing;
                let title_height = if legend.title.is_some() {
                    legend.font_size + spacing.label_spacing
                } else {
                    0.0
                };
                let width = content_width + spacing.border_pad * 2.0;
                let height = content_height + title_height + spacing.border_pad * 2.0;
                (width, height, max_label_width)
            }
        };

        // Determine position
        let position = if matches!(legend.position, LegendPosition::Best) {
            let bboxes = data_bboxes.unwrap_or(&[]);
            if bboxes.len() > 100000 {
                LegendPosition::UpperRight
            } else {
                find_best_position(
                    (legend_width, legend_height),
                    plot_area,
                    bboxes,
                    &legend.spacing,
                    legend.font_size,
                )
            }
        } else {
            legend.position
        };

        let resolved_legend = Legend {
            position,
            ..legend.clone()
        };

        let (legend_x, legend_y) =
            resolved_legend.calculate_position((legend_width, legend_height), plot_area);

        // Draw frame
        self.draw_legend_frame(
            legend_x,
            legend_y,
            legend_width,
            legend_height,
            &legend.style,
        );

        // Starting position for items
        let item_x = legend_x + spacing.border_pad;
        let mut item_y = legend_y + spacing.border_pad + legend.font_size / 2.0;

        // Draw title if present
        if let Some(ref title) = legend.title {
            let title_x = legend_x + legend_width / 2.0;
            self.draw_text_centered(title, title_x, item_y, legend.font_size, legend.text_color)?;
            item_y += legend.font_size + spacing.label_spacing;
        }

        // Calculate items per column
        let items_per_col = items.len().div_ceil(legend.columns);

        // Calculate column width
        let col_width = spacing.handle_length + spacing.handle_text_pad + label_width;

        // Draw items column by column
        for col in 0..legend.columns {
            let col_x = item_x + col as f32 * (col_width + spacing.column_spacing);
            let mut row_y = item_y;

            for row in 0..items_per_col {
                let idx = col * items_per_col + row;
                if idx >= items.len() {
                    break;
                }

                let item = &items[idx];

                // Draw handle
                self.draw_legend_handle(item, col_x, row_y, &spacing);

                // Draw label
                let text_x = col_x + spacing.handle_length + spacing.handle_text_pad;
                let centered_y = row_y - legend.font_size * 0.65;
                self.draw_text(
                    &item.label,
                    text_x,
                    centered_y,
                    legend.font_size,
                    legend.text_color,
                )?;

                row_y += legend.font_size + spacing.label_spacing;
            }
        }

        Ok(())
    }

    /// Add a clip path definition and return the ID
    pub fn add_clip_rect(&mut self, x: f32, y: f32, width: f32, height: f32) -> String {
        let clip_id = self.next_clip_id();
        writeln!(
            self.defs,
            r#"    <clipPath id="{}"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/></clipPath>"#,
            clip_id, x, y, width, height
        )
        .unwrap();
        clip_id
    }

    /// Start a clipped group
    pub fn start_clip_group(&mut self, clip_id: &str) {
        writeln!(self.content, r#"  <g clip-path="url(#{})">"#, clip_id).unwrap();
    }

    /// End a group
    pub fn end_group(&mut self) {
        writeln!(self.content, "  </g>").unwrap();
    }

    /// Render to SVG string
    pub fn to_svg_string(&self) -> String {
        let mut svg = String::new();
        writeln!(svg, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
        writeln!(
            svg,
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">"#,
            self.width as u32, self.height as u32
        )
        .unwrap();

        // Add defs section if we have any
        if !self.defs.is_empty() {
            writeln!(svg, "  <defs>").unwrap();
            svg.push_str(&self.defs);
            writeln!(svg, "  </defs>").unwrap();
        }

        // Add content
        svg.push_str(&self.content);

        writeln!(svg, "</svg>").unwrap();
        svg
    }

    /// Save to SVG file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let svg_string = self.to_svg_string();
        std::fs::write(path, svg_string).map_err(PlottingError::IoError)?;
        Ok(())
    }

    /// Get width
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get height
    pub fn height(&self) -> f32 {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_svg_attr(line: &str, attr: &str) -> f32 {
        let marker = format!(r#"{}=""#, attr);
        let start = line
            .find(&marker)
            .unwrap_or_else(|| panic!("missing {} in line: {}", attr, line))
            + marker.len();
        let end = line[start..]
            .find('"')
            .unwrap_or_else(|| panic!("unterminated {} in line: {}", attr, line))
            + start;
        line[start..end]
            .parse::<f32>()
            .unwrap_or_else(|_| panic!("invalid {} in line: {}", attr, line))
    }

    fn extract_svg_text_xy(svg: &str, text: &str) -> (f32, f32) {
        let marker = format!(">{}</text>", text);
        let line = svg
            .lines()
            .find(|line| line.contains(&marker))
            .unwrap_or_else(|| panic!("missing text node for {}", text));
        (parse_svg_attr(line, "x"), parse_svg_attr(line, "y"))
    }

    fn extract_typst_group_translates(svg: &str) -> Vec<(f32, f32)> {
        svg.lines()
            .filter(|line| line.contains(r#"data-ruviz-text-engine="typst""#))
            .map(|line| {
                let marker = r#"transform="translate("#;
                let start = line
                    .find(marker)
                    .unwrap_or_else(|| panic!("missing translate transform in line: {}", line))
                    + marker.len();
                let end = line[start..].find(')').unwrap_or_else(|| {
                    panic!("unterminated translate transform in line: {}", line)
                }) + start;
                let coords = &line[start..end];
                let mut parts = coords.split(',');
                let x = parts
                    .next()
                    .unwrap_or_else(|| panic!("missing translate x in line: {}", line))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("invalid translate x in line: {}", line));
                let y = parts
                    .next()
                    .unwrap_or_else(|| panic!("missing translate y in line: {}", line))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("invalid translate y in line: {}", line));
                (x, y)
            })
            .collect()
    }

    #[test]
    fn test_svg_renderer_creation() {
        let renderer = SvgRenderer::new(800.0, 600.0);
        assert_eq!(renderer.width(), 800.0);
        assert_eq!(renderer.height(), 600.0);
    }

    #[test]
    fn test_color_conversion() {
        let renderer = SvgRenderer::new(100.0, 100.0);
        assert_eq!(renderer.color_to_svg(Color::new(255, 0, 0)), "rgb(255,0,0)");
        assert_eq!(
            renderer.color_to_svg(Color::new_rgba(255, 0, 0, 128)),
            "rgba(255,0,0,0.502)"
        );
    }

    #[test]
    fn test_line_style_conversion() {
        let renderer = SvgRenderer::new(100.0, 100.0);
        assert_eq!(renderer.line_style_to_dasharray(&LineStyle::Solid), None);
        assert_eq!(
            renderer.line_style_to_dasharray(&LineStyle::Dashed),
            Some("5,5".to_string())
        );
    }

    #[test]
    fn test_line_style_conversion_scales_with_dpi() {
        let mut renderer = SvgRenderer::new(100.0, 100.0);
        renderer.set_dpi_scale(2.0);

        assert_eq!(
            renderer.line_style_to_dasharray(&LineStyle::Dashed),
            Some("10,10".to_string())
        );
        assert_eq!(
            renderer.line_style_to_dasharray(&LineStyle::Dotted),
            Some("2,4".to_string())
        );
        assert_eq!(
            renderer.line_style_to_dasharray(&LineStyle::Custom(vec![1.5, 2.0])),
            Some("3,4".to_string())
        );
    }

    #[test]
    fn test_set_dpi_scale_sanitizes_invalid_values() {
        let mut renderer = SvgRenderer::new(100.0, 100.0);

        renderer.set_dpi_scale(2.5);
        assert!((renderer.dpi_scale() - 2.5).abs() < f32::EPSILON);

        renderer.set_dpi_scale(0.0);
        assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);
        assert_eq!(
            renderer.line_style_to_dasharray(&LineStyle::Dashed),
            Some("5,5".to_string())
        );

        renderer.set_dpi_scale(-3.0);
        assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

        renderer.set_dpi_scale(f32::NAN);
        assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

        renderer.set_dpi_scale(f32::INFINITY);
        assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_xml_escaping() {
        let renderer = SvgRenderer::new(100.0, 100.0);
        assert_eq!(
            renderer.escape_xml("a < b & c > d"),
            "a &lt; b &amp; c &gt; d"
        );
    }

    #[test]
    fn test_svg_output() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);
        renderer.draw_rectangle(0.0, 0.0, 200.0, 150.0, Color::WHITE, true);
        renderer.draw_line(
            10.0,
            10.0,
            190.0,
            140.0,
            Color::BLACK,
            2.0,
            LineStyle::Solid,
        );

        let svg = renderer.to_svg_string();
        assert!(svg.contains("svg"));
        assert!(svg.contains("rect"));
        assert!(svg.contains("line"));
    }

    #[test]
    fn test_legend_line_handle_attribute_spacing() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);

        renderer.draw_legend_line_handle(10.0, 20.0, 30.0, Color::BLACK, &LineStyle::Solid, 1.5);
        renderer.draw_legend_line_handle(10.0, 30.0, 30.0, Color::BLACK, &LineStyle::Dashed, 1.5);

        let svg = renderer.to_svg_string();
        let line_nodes: Vec<&str> = svg.lines().filter(|line| line.contains("<line")).collect();

        assert!(
            line_nodes
                .iter()
                .any(|line| line.contains(r#"stroke-width="1.5"/>"#)),
            "solid legend handle should not include dangling whitespace before '/>'"
        );
        assert!(
            line_nodes
                .iter()
                .any(|line| { line.contains(r#"stroke-width="1.5" stroke-dasharray="5,5""#) }),
            "dashed legend handle should include dash attribute with one leading space"
        );
        assert!(
            !line_nodes
                .iter()
                .any(|line| line.contains(r#"stroke-width="1.5" />"#)),
            "legend handle should not emit dangling whitespace before '/>'"
        );
    }

    #[test]
    fn test_plain_text_uses_top_origin_baseline() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);
        renderer
            .draw_text("Top Origin", 10.0, 20.0, 12.0, Color::BLACK)
            .unwrap();
        renderer
            .draw_text_centered("Centered Top", 100.0, 25.0, 12.0, Color::BLACK)
            .unwrap();

        let svg = renderer.to_svg_string();
        assert!(!svg.contains("dominant-baseline=\"text-before-edge\""));

        let (x1, y1) = extract_svg_text_xy(&svg, "Top Origin");
        let (x2, y2) = extract_svg_text_xy(&svg, "Centered Top");
        let metrics1 = renderer.plain_text_metrics("Top Origin", 12.0).unwrap();
        let metrics2 = renderer.plain_text_metrics("Centered Top", 12.0).unwrap();

        assert!((x1 - 10.0).abs() <= 0.01);
        assert!((x2 - 100.0).abs() <= 0.01);
        assert!((y1 - top_anchor_to_baseline(20.0, metrics1)).abs() <= 0.6);
        assert!((y2 - top_anchor_to_baseline(25.0, metrics2)).abs() <= 0.6);
    }

    #[test]
    fn test_tick_labels_use_layout_positions() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);
        let x_ticks = vec![100.0];
        let x_labels = vec!["1.0".to_string()];
        let y_ticks = vec![75.0];
        let y_labels = vec!["2.0".to_string()];

        renderer
            .draw_tick_labels(
                &x_ticks,
                &x_labels,
                &y_ticks,
                &y_labels,
                40.0,
                160.0,
                20.0,
                120.0,
                120.0,
                35.0,
                Color::BLACK,
                10.0,
            )
            .unwrap();

        let svg = renderer.to_svg_string();
        let (x_tick_x, x_tick_y) = extract_svg_text_xy(&svg, "1.0");
        let (y_tick_x, y_tick_y) = extract_svg_text_xy(&svg, "2.0");
        let x_metrics = renderer.plain_text_metrics("1.0", 10.0).unwrap();
        let y_metrics = renderer.plain_text_metrics("2.0", 10.0).unwrap();

        let x_top = 120.0;
        let expected_x_tick_x = (100.0 - x_metrics.width / 2.0)
            .max(0.0)
            .min(renderer.width - x_metrics.width);
        let expected_x_tick_y = top_anchor_to_baseline(x_top, x_metrics);

        let y_top = 75.0 - y_metrics.height / 2.0;
        let expected_y_tick_x = (35.0 - y_metrics.width - 5.0).max(5.0);
        let expected_y_tick_y = top_anchor_to_baseline(y_top, y_metrics);

        assert!(
            (x_tick_x - expected_x_tick_x).abs() <= 0.6
                && (x_tick_y - expected_x_tick_y).abs() <= 0.6,
            "x-axis tick label should use layout xtick baseline"
        );
        assert!(
            (y_tick_x - expected_y_tick_x).abs() <= 0.6
                && (y_tick_y - expected_y_tick_y).abs() <= 0.6,
            "y-axis tick label should use layout ytick anchor and centered y"
        );
    }

    #[cfg(feature = "typst-math")]
    #[test]
    fn test_typst_tick_labels_follow_plain_anchor_math() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);
        renderer.set_text_engine_mode(TextEngineMode::Typst);
        let x_ticks = vec![100.0];
        let x_labels = vec!["1.0".to_string()];
        let y_ticks = vec![75.0];
        let y_labels = vec!["2.0".to_string()];

        renderer
            .draw_tick_labels(
                &x_ticks,
                &x_labels,
                &y_ticks,
                &y_labels,
                40.0,
                160.0,
                20.0,
                120.0,
                120.0,
                35.0,
                Color::BLACK,
                10.0,
            )
            .unwrap();

        let svg = renderer.to_svg_string();
        assert!(svg.contains("data-ruviz-text-engine=\"typst\""));

        let translates = extract_typst_group_translates(&svg);
        assert_eq!(translates.len(), 2, "expected two typst text groups");

        let x_snippet = typst_text::literal_text_snippet("1.0");
        let y_snippet = typst_text::literal_text_snippet("2.0");
        let (x_w, _x_h) = typst_text::measure_text(
            &x_snippet,
            10.0,
            Color::BLACK,
            0.0,
            TypstBackendKind::Svg,
            "typst tick test",
        )
        .unwrap();
        let (y_w, y_h) = typst_text::measure_text(
            &y_snippet,
            10.0,
            Color::BLACK,
            0.0,
            TypstBackendKind::Svg,
            "typst tick test",
        )
        .unwrap();

        let expected_x = (100.0 - x_w / 2.0).max(0.0).min(renderer.width - x_w);
        let expected_y = (35.0 - y_w - 5.0).max(5.0);
        let expected_y_top = 75.0 - y_h / 2.0;

        assert!(
            (translates[0].0 - expected_x).abs() <= 0.6 && (translates[0].1 - 120.0).abs() <= 0.6
        );
        assert!(
            (translates[1].0 - expected_y).abs() <= 0.6
                && (translates[1].1 - expected_y_top).abs() <= 0.6
        );
    }

    #[cfg(feature = "typst-math")]
    #[test]
    fn test_typst_rotated_text_uses_typst_rotation_path() {
        let mut renderer = SvgRenderer::new(200.0, 150.0);
        renderer.set_text_engine_mode(TextEngineMode::Typst);
        renderer
            .draw_text_rotated("Y Axis", 100.0, 75.0, 12.0, Color::BLACK, -90.0)
            .unwrap();

        let svg = renderer.to_svg_string();
        assert!(svg.contains("data-ruviz-text-engine=\"typst\""));
        assert!(!svg.contains("data-ruviz-text-engine=\"typst\" transform=\"rotate("));
    }
}
