//! SVG export functionality
//!
//! Provides vector-based SVG export for plots with full visual fidelity.
//! This renderer is also used as the intermediate format for PDF export.

use crate::core::{
    Legend, LegendItem, LegendItemType, LegendPosition, LegendSpacingPixels, LegendStyle,
    PlottingError, RenderScale, Result, SpineConfig, TextAlign, TextStyle, find_best_position,
    plot::{TextEngineMode, TickDirection, TickSides},
};
use crate::render::{
    Color, FontConfig, FontFamily, FontWeight, LineStyle, MarkerStyle, TextRenderer,
    text_anchor::{
        TextPlacementMetrics, annotation_text_layout, center_anchor_to_baseline,
        top_anchor_to_baseline,
    },
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
    /// Shared render scale for unit conversion.
    render_scale: RenderScale,
    /// Active text rendering engine.
    text_engine_mode: TextEngineMode,
    /// Plain text metrics for anchor conversion.
    text_renderer: TextRenderer,
    /// Font family for plain SVG text and Typst-rendered SVG text.
    font_family: FontFamily,
}

impl SvgRenderer {
    /// Create a new SVG renderer with specified dimensions
    pub fn new(width: f32, height: f32) -> Self {
        Self::with_font_family(width, height, FontFamily::SansSerif)
    }

    /// Create a new SVG renderer with a specified text font family.
    pub fn with_font_family(width: f32, height: f32, font_family: FontFamily) -> Self {
        Self {
            width,
            height,
            content: String::new(),
            defs: String::new(),
            clip_id_counter: 0,
            render_scale: RenderScale::from_canvas_size(
                width.max(1.0).round() as u32,
                height.max(1.0).round() as u32,
                crate::core::REFERENCE_DPI,
            ),
            text_engine_mode: TextEngineMode::Plain,
            text_renderer: TextRenderer::new(),
            font_family,
        }
    }

    /// Set the render scale context used for unit conversion.
    pub fn set_render_scale(&mut self, render_scale: RenderScale) {
        self.render_scale = render_scale;
    }

    /// Get the render scale context used for unit conversion.
    pub fn render_scale(&self) -> RenderScale {
        self.render_scale
    }

    /// Legacy compatibility shim for callers that still pass `dpi / 100.0`.
    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        self.set_render_scale(RenderScale::from_reference_scale(dpi_scale));
    }

    /// Legacy compatibility shim for callers that still expect `dpi / 100.0`.
    pub fn dpi_scale(&self) -> f32 {
        self.render_scale.reference_scale()
    }

    fn logical_pixels_to_pixels(&self, logical_pixels: f32) -> f32 {
        self.render_scale.logical_pixels_to_pixels(logical_pixels)
    }

    fn points_to_pixels(&self, points: f32) -> f32 {
        self.render_scale.points_to_pixels(points)
    }

    /// Set text rendering backend mode.
    pub fn set_text_engine_mode(&mut self, mode: TextEngineMode) {
        self.text_engine_mode = mode;
    }

    /// Get text rendering backend mode.
    pub fn text_engine_mode(&self) -> TextEngineMode {
        self.text_engine_mode
    }

    /// Set the font family used by plain and Typst text rendering.
    pub fn set_font_family<F>(&mut self, family: F)
    where
        F: Into<FontFamily>,
    {
        self.font_family = family.into();
    }

    /// Get the configured font family.
    pub fn font_family(&self) -> &FontFamily {
        &self.font_family
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

    /// Convert style to a scaled dash pattern using the shared render scale.
    fn scaled_dash_pattern(&self, style: &LineStyle) -> Option<Vec<f32>> {
        style.to_dash_array().map(|base| {
            base.into_iter()
                .map(|segment| self.logical_pixels_to_pixels(segment))
                .collect()
        })
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
        let trimmed = svg.trim_start();
        let without_decl = if trimmed.starts_with("<?xml") {
            if let Some(end) = trimmed.find("?>") {
                trimmed[end + 2..].trim_start()
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        if let Some(start) = without_decl.find("<svg") {
            &without_decl[start..]
        } else {
            without_decl
        }
    }

    fn embedded_typst_svg(&self, rendered: &typst_text::TypstSvgOutput) -> String {
        let mut svg = self.strip_xml_declaration(&rendered.svg).to_string();
        Self::set_root_svg_dimension(&mut svg, "width", rendered.width);
        Self::set_root_svg_dimension(&mut svg, "height", rendered.height);
        svg
    }

    fn set_root_svg_dimension(svg: &mut String, attribute: &str, value: f32) {
        let Some(tag_end) = svg.find('>') else {
            return;
        };
        let marker = format!(r#"{attribute}=""#);
        let Some(relative_start) = svg[..tag_end].find(&marker) else {
            return;
        };
        let value_start = relative_start + marker.len();
        let Some(relative_end) = svg[value_start..tag_end].find('"') else {
            return;
        };
        let value_end = value_start + relative_end;
        svg.replace_range(value_start..value_end, &format!("{value:.2}"));
    }

    fn generated_label<'a>(&self, text: &'a str) -> Cow<'a, str> {
        #[cfg(feature = "typst-math")]
        if self.text_engine_mode.uses_typst() {
            return Cow::Owned(typst_text::literal_text_snippet(text));
        }

        Cow::Borrowed(text)
    }

    fn plain_text_metrics(&self, text: &str, font_size: f32) -> Result<TextPlacementMetrics> {
        let config = FontConfig::new(self.font_family.clone(), font_size);
        self.plain_text_metrics_with_config(text, &config)
    }

    fn plain_text_metrics_with_config(
        &self,
        text: &str,
        config: &FontConfig,
    ) -> Result<TextPlacementMetrics> {
        self.text_renderer.measure_text_placement(text, config)
    }

    fn escape_css_string(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for character in value.chars() {
            match character {
                '\0' => escaped.push('\u{FFFD}'),
                '"' | '\\' => {
                    escaped.push('\\');
                    escaped.push(character);
                }
                '\u{0001}'..='\u{001F}' | '\u{007F}' | '\u{FFFE}' | '\u{FFFF}' => {
                    write!(escaped, "\\{:06X}", character as u32)
                        .expect("writing CSS escape to String cannot fail");
                }
                _ => escaped.push(character),
            }
        }
        escaped
    }

    fn escaped_font_family(&self) -> String {
        self.escaped_font_family_for(&self.font_family)
    }

    fn escaped_font_family_for(&self, family: &FontFamily) -> String {
        let css_value = match family {
            FontFamily::Serif
            | FontFamily::SansSerif
            | FontFamily::Monospace
            | FontFamily::Cursive
            | FontFamily::Fantasy => family.as_str().to_string(),
            FontFamily::Name(name) => format!("\"{}\"", Self::escape_css_string(name)),
        };
        self.escape_xml(&css_value)
    }

    fn svg_text_anchor(align: TextAlign) -> &'static str {
        match align {
            TextAlign::Left => "start",
            TextAlign::Center => "middle",
            TextAlign::Right => "end",
        }
    }

    fn measure_text_for_layout(&self, text: &str, font_size: f32) -> Result<(f32, f32)> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let metrics = self.plain_text_metrics(text, font_size)?;
                Ok((metrics.width, metrics.height))
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(font_size);
                typst_text::measure_text_with_font_family(
                    text,
                    size_pt,
                    Color::BLACK,
                    0.0,
                    TypstBackendKind::Svg,
                    &self.font_family,
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

    /// Draw a filled polygon.
    pub fn draw_filled_polygon(&mut self, points: &[(f32, f32)], color: Color) {
        if points.len() < 3 {
            return;
        }

        let color_str = self.color_to_svg(color);
        let points_str = points
            .iter()
            .map(|(x, y)| format!("{:.2},{:.2}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(
            self.content,
            r#"  <polygon points="{}" fill="{}" stroke="none"/>"#,
            points_str, color_str
        )
        .unwrap();
    }

    /// Draw a polygon outline.
    pub fn draw_polygon_outline(&mut self, points: &[(f32, f32)], color: Color, width: f32) {
        if points.len() < 3 {
            return;
        }

        let color_str = self.color_to_svg(color);
        let points_str = points
            .iter()
            .map(|(x, y)| format!("{:.2},{:.2}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(
            self.content,
            r#"  <polygon points="{}" fill="none" stroke="{}" stroke-width="{:.2}" stroke-linejoin="round"/>"#,
            points_str, color_str, width
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

    fn draw_polygon_marker(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        stroke_width: Option<f32>,
    ) {
        let color_str = self.color_to_svg(color);
        let points_str = points
            .iter()
            .map(|(x, y)| format!("{:.2},{:.2}", x, y))
            .collect::<Vec<_>>()
            .join(" ");

        if let Some(stroke_width) = stroke_width {
            writeln!(
                self.content,
                r#"  <polygon points="{}" fill="none" stroke="{}" stroke-width="{:.2}"/>"#,
                points_str, color_str, stroke_width
            )
            .unwrap();
        } else {
            writeln!(
                self.content,
                r#"  <polygon points="{}" fill="{}"/>"#,
                points_str, color_str
            )
            .unwrap();
        }
    }

    fn draw_marker_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        let color_str = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}" stroke-linecap="butt"/>"#,
            x1, y1, x2, y2, color_str, width
        )
        .unwrap();
    }

    /// Draw a marker at a point, matching the raster marker semantics.
    pub fn draw_marker(&mut self, x: f32, y: f32, size: f32, style: MarkerStyle, color: Color) {
        let radius = size / 2.0;

        match style {
            MarkerStyle::Circle => self.draw_circle(x, y, radius, color, true),
            MarkerStyle::CircleOpen => self.draw_circle(x, y, radius, color, false),
            MarkerStyle::Square => {
                self.draw_rectangle(x - radius, y - radius, size, size, color, true)
            }
            MarkerStyle::SquareOpen => {
                self.draw_rectangle(x - radius, y - radius, size, size, color, false)
            }
            MarkerStyle::Triangle => self.draw_polygon_marker(
                &[
                    (x, y - radius),
                    (x - radius * 0.866, y + radius * 0.5),
                    (x + radius * 0.866, y + radius * 0.5),
                ],
                color,
                None,
            ),
            MarkerStyle::TriangleOpen => self.draw_polygon_marker(
                &[
                    (x, y - radius),
                    (x - radius * 0.866, y + radius * 0.5),
                    (x + radius * 0.866, y + radius * 0.5),
                ],
                color,
                Some((size * 0.15).max(1.0)),
            ),
            MarkerStyle::TriangleDown => self.draw_polygon_marker(
                &[
                    (x, y + radius),
                    (x - radius * 0.866, y - radius * 0.5),
                    (x + radius * 0.866, y - radius * 0.5),
                ],
                color,
                None,
            ),
            MarkerStyle::Diamond => self.draw_polygon_marker(
                &[
                    (x, y - radius),
                    (x + radius, y),
                    (x, y + radius),
                    (x - radius, y),
                ],
                color,
                None,
            ),
            MarkerStyle::DiamondOpen => self.draw_polygon_marker(
                &[
                    (x, y - radius),
                    (x + radius, y),
                    (x, y + radius),
                    (x - radius, y),
                ],
                color,
                Some((size * 0.15).max(1.0)),
            ),
            MarkerStyle::Plus => {
                let line_width = (size * 0.25).max(1.0);
                self.draw_marker_line(x - radius, y, x + radius, y, color, line_width);
                self.draw_marker_line(x, y - radius, x, y + radius, color, line_width);
            }
            MarkerStyle::Cross => {
                let line_width = (size * 0.25).max(1.0);
                let offset = radius * 0.707;
                self.draw_marker_line(
                    x - offset,
                    y - offset,
                    x + offset,
                    y + offset,
                    color,
                    line_width,
                );
                self.draw_marker_line(
                    x - offset,
                    y + offset,
                    x + offset,
                    y - offset,
                    color,
                    line_width,
                );
            }
            MarkerStyle::Star => {
                let line_width = (size * 0.22).max(1.0);
                let offset = radius * 0.707;
                for (x1, y1, x2, y2) in [
                    (x - radius, y, x + radius, y),
                    (x, y - radius, x, y + radius),
                    (x - offset, y - offset, x + offset, y + offset),
                    (x - offset, y + offset, x + offset, y - offset),
                ] {
                    self.draw_marker_line(x1, y1, x2, y2, color, line_width);
                }
            }
        }
    }

    pub(crate) fn draw_styled_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        family: &FontFamily,
        style: &TextStyle,
    ) -> Result<()> {
        let font_size = self.points_to_pixels(style.font_size.max(0.1));
        let padding = self.points_to_pixels(style.padding.max(0.0));
        let border_width = self.points_to_pixels(style.border_width.max(0.0));
        let text_visible = style.color.a > 0 && !text.trim().is_empty();
        let background_visible = style.background.is_some_and(|color| color.a > 0);
        let border_visible =
            border_width > 0.0 && style.border_color.is_some_and(|color| color.a > 0);
        if !text_visible && !background_visible && !border_visible {
            return Ok(());
        }

        let weight = FontWeight::Normal;
        let config = FontConfig::new(family.clone(), font_size).weight(weight);
        #[cfg(feature = "typst-math")]
        let mut typst_rendered = None;
        let metrics = if text.trim().is_empty() {
            TextPlacementMetrics::new(0.0, font_size, font_size)
        } else {
            match self.text_engine_mode {
                TextEngineMode::Plain => self.plain_text_metrics_with_config(text, &config)?,
                #[cfg(feature = "typst-math")]
                TextEngineMode::Typst => {
                    let multiline_text = typst_text::with_explicit_line_breaks(text);
                    let weighted_text = typst_text::with_font_weight(&multiline_text, weight);
                    let aligned_text =
                        typst_text::with_horizontal_alignment(&weighted_text, style.align);
                    let rendered = typst_text::render_svg_with_font_family(
                        &aligned_text,
                        self.typst_size_pt(font_size),
                        style.color,
                        0.0,
                        family,
                        "SVG annotation text rendering",
                    )?;
                    let metrics =
                        TextPlacementMetrics::new(rendered.width, rendered.height, rendered.height);
                    typst_rendered = Some(rendered);
                    metrics
                }
            }
        };
        let layout =
            annotation_text_layout(metrics, style.align, style.valign, padding, style.rotation);

        writeln!(
            self.content,
            r#"  <g data-ruviz-text-style="annotation" transform="translate({:.2},{:.2}) rotate({:.2})">"#,
            x, y, layout.rotation
        )
        .unwrap();

        if background_visible || border_visible {
            let fill = style
                .background
                .filter(|_| background_visible)
                .map(|color| self.color_to_svg(color))
                .unwrap_or_else(|| "none".to_string());
            let stroke = style
                .border_color
                .filter(|_| border_visible)
                .map(|color| self.color_to_svg(color))
                .unwrap_or_else(|| "none".to_string());
            writeln!(
                self.content,
                r#"    <rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" stroke="{}" stroke-width="{:.2}"/>"#,
                layout.box_x,
                layout.box_y,
                layout.box_width,
                layout.box_height,
                fill,
                stroke,
                border_width
            )
            .unwrap();
        }

        if text_visible {
            match self.text_engine_mode {
                TextEngineMode::Plain => {
                    let font_family = self.escaped_font_family_for(family);
                    let color = self.color_to_svg(style.color);
                    let text_anchor = Self::svg_text_anchor(style.align);
                    let baseline_y = layout.text_y + metrics.baseline_from_top;
                    if text.contains('\n') {
                        write!(
                            self.content,
                            r#"    <text x="0" font-family="{}" font-size="{:.1}" font-weight="{}" fill="{}" text-anchor="{}" xml:space="preserve">"#,
                            font_family,
                            font_size,
                            weight.numeric(),
                            color,
                            text_anchor
                        )
                        .unwrap();
                        let line_height = font_size * 1.2;
                        for (line_index, line) in text.split('\n').enumerate() {
                            let line = line.strip_suffix('\r').unwrap_or(line);
                            let line_y = baseline_y + line_index as f32 * line_height;
                            write!(
                                self.content,
                                r#"<tspan x="0" y="{:.2}">{}</tspan>"#,
                                line_y,
                                self.escape_xml(line)
                            )
                            .unwrap();
                        }
                        writeln!(self.content, "</text>").unwrap();
                    } else {
                        writeln!(
                            self.content,
                            r#"    <text x="0" y="{:.2}" font-family="{}" font-size="{:.1}" font-weight="{}" fill="{}" text-anchor="{}" xml:space="preserve">{}</text>"#,
                            baseline_y,
                            font_family,
                            font_size,
                            weight.numeric(),
                            color,
                            text_anchor,
                            self.escape_xml(text)
                        )
                        .unwrap();
                    }
                }
                #[cfg(feature = "typst-math")]
                TextEngineMode::Typst => {
                    let rendered = typst_rendered
                        .take()
                        .expect("Typst annotation rendering must produce SVG output");
                    let embedded_svg = self.embedded_typst_svg(&rendered);
                    writeln!(
                        self.content,
                        r#"    <g data-ruviz-text-engine="typst" transform="translate({:.2},{:.2})">{}</g>"#,
                        layout.text_x, layout.text_y, embedded_svg
                    )
                    .unwrap();
                }
            }
        }

        writeln!(self.content, "  </g>").unwrap();
        Ok(())
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
                let font_family = self.escaped_font_family();
                writeln!(
                    self.content,
                    r#"  <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}">{}</text>"#,
                    x, baseline_y, font_family, size, color_str, escaped_text
                )
                .unwrap();
                Ok(())
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_svg_with_font_family(
                    text,
                    size_pt,
                    color,
                    0.0,
                    &self.font_family,
                    "SVG text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopLeft,
                );
                let embedded_svg = self.embedded_typst_svg(&rendered);
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
        self.draw_text_centered_impl(text, x, y, size, color, None)
    }

    pub(crate) fn draw_text_centered_with_weight(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: FontWeight,
    ) -> Result<()> {
        self.draw_text_centered_impl(text, x, y, size, color, Some(weight))
    }

    fn draw_text_centered_impl(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        weight: Option<FontWeight>,
    ) -> Result<()> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let color_str = self.color_to_svg(color);
                let resolved_weight = weight.unwrap_or(FontWeight::Normal);
                let config =
                    FontConfig::new(self.font_family.clone(), size).weight(resolved_weight);
                let metrics = self.plain_text_metrics_with_config(text, &config)?;
                let baseline_y = top_anchor_to_baseline(y, metrics);
                let font_family = self.escaped_font_family();
                let weight_attr = weight
                    .map(|weight| format!(r#" font-weight="{}""#, weight.numeric()))
                    .unwrap_or_default();
                if text.contains('\n') {
                    write!(
                        self.content,
                        r#"  <text x="{:.2}" font-family="{}" font-size="{:.1}"{} fill="{}" text-anchor="middle" xml:space="preserve">"#,
                        x, font_family, size, weight_attr, color_str
                    )
                    .unwrap();
                    let line_height = size * 1.2;
                    for (line_index, line) in text.split('\n').enumerate() {
                        let line = line.strip_suffix('\r').unwrap_or(line);
                        let line_y = baseline_y + line_index as f32 * line_height;
                        write!(
                            self.content,
                            r#"<tspan x="{:.2}" y="{:.2}">{}</tspan>"#,
                            x,
                            line_y,
                            self.escape_xml(line)
                        )
                        .unwrap();
                    }
                    writeln!(self.content, "</text>").unwrap();
                } else {
                    writeln!(
                        self.content,
                        r#"  <text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}"{} fill="{}" text-anchor="middle" xml:space="preserve">{}</text>"#,
                        x,
                        baseline_y,
                        font_family,
                        size,
                        weight_attr,
                        color_str,
                        self.escape_xml(text)
                    )
                    .unwrap();
                }
                Ok(())
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let multiline_text = typst_text::with_explicit_line_breaks(text);
                let weighted_text =
                    weight.map(|weight| typst_text::with_font_weight(&multiline_text, weight));
                let aligned_text = typst_text::with_horizontal_alignment(
                    weighted_text.as_deref().unwrap_or(&multiline_text),
                    TextAlign::Center,
                );
                let rendered = typst_text::render_svg_with_font_family(
                    &aligned_text,
                    size_pt,
                    color,
                    0.0,
                    &self.font_family,
                    "SVG centered text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::TopCenter,
                );
                let embedded_svg = self.embedded_typst_svg(&rendered);
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
                let font_family = self.escaped_font_family();
                writeln!(
                    self.content,
                    r#"  <g transform="translate({:.2},{:.2}) rotate({:.1})"><text x="0" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" text-anchor="middle">{}</text></g>"#,
                    x, y, angle, center_baseline_y, font_family, size, color_str, escaped_text
                )
                .unwrap();
                Ok(())
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => {
                let size_pt = self.typst_size_pt(size);
                let rendered = typst_text::render_svg_with_font_family(
                    text,
                    size_pt,
                    color,
                    angle,
                    &self.font_family,
                    "SVG rotated text rendering",
                )?;
                let (draw_x, draw_y) = typst_text::anchored_top_left(
                    x,
                    y,
                    rendered.width,
                    rendered.height,
                    TypstTextAnchor::Center,
                );
                let embedded_svg = self.embedded_typst_svg(&rendered);
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

    fn vertical_tick_span(
        spine_y: f32,
        tick_size: f32,
        tick_direction: &TickDirection,
        top: bool,
    ) -> (f32, f32) {
        match tick_direction {
            TickDirection::Inside => {
                if top {
                    (spine_y, spine_y + tick_size)
                } else {
                    (spine_y, spine_y - tick_size)
                }
            }
            TickDirection::Outside => {
                if top {
                    (spine_y, spine_y - tick_size)
                } else {
                    (spine_y, spine_y + tick_size)
                }
            }
            TickDirection::InOut => (spine_y - tick_size / 2.0, spine_y + tick_size / 2.0),
        }
    }

    fn horizontal_tick_span(
        spine_x: f32,
        tick_size: f32,
        tick_direction: &TickDirection,
        right: bool,
    ) -> (f32, f32) {
        match tick_direction {
            TickDirection::Inside => {
                if right {
                    (spine_x, spine_x - tick_size)
                } else {
                    (spine_x, spine_x + tick_size)
                }
            }
            TickDirection::Outside => {
                if right {
                    (spine_x, spine_x + tick_size)
                } else {
                    (spine_x, spine_x - tick_size)
                }
            }
            TickDirection::InOut => (spine_x - tick_size / 2.0, spine_x + tick_size / 2.0),
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
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        color: Color,
    ) {
        // Axis metrics are authored in logical pixels and resolved via RenderScale.
        let axis_width = self.logical_pixels_to_pixels(1.5);
        let major_tick_size = self.logical_pixels_to_pixels(6.0);
        let tick_width = self.logical_pixels_to_pixels(1.0);

        // Draw the full plot frame. Tick side selection only controls tick marks.
        self.draw_line(
            plot_left,
            plot_bottom,
            plot_right,
            plot_bottom,
            color,
            axis_width,
            LineStyle::Solid,
        );

        self.draw_line(
            plot_left,
            plot_top,
            plot_left,
            plot_bottom,
            color,
            axis_width,
            LineStyle::Solid,
        );

        self.draw_line(
            plot_left,
            plot_top,
            plot_right,
            plot_top,
            color,
            axis_width,
            LineStyle::Solid,
        );

        self.draw_line(
            plot_right,
            plot_top,
            plot_right,
            plot_bottom,
            color,
            axis_width,
            LineStyle::Solid,
        );

        for &x in x_ticks {
            if x >= plot_left && x <= plot_right {
                if tick_sides.bottom {
                    let (tick_start, tick_end) = Self::vertical_tick_span(
                        plot_bottom,
                        major_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    );
                }
                if tick_sides.top {
                    let (tick_start, tick_end) =
                        Self::vertical_tick_span(plot_top, major_tick_size, tick_direction, true);
                    self.draw_line(
                        x,
                        tick_start,
                        x,
                        tick_end,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    );
                }
            }
        }

        for &y in y_ticks {
            if y >= plot_top && y <= plot_bottom {
                if tick_sides.left {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_left,
                        major_tick_size,
                        tick_direction,
                        false,
                    );
                    self.draw_line(
                        tick_start,
                        y,
                        tick_end,
                        y,
                        color,
                        tick_width,
                        LineStyle::Solid,
                    );
                }
                if tick_sides.right {
                    let (tick_start, tick_end) = Self::horizontal_tick_span(
                        plot_right,
                        major_tick_size,
                        tick_direction,
                        true,
                    );
                    self.draw_line(
                        tick_start,
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
    }

    /// Draw axis lines with major and minor tick marks.
    pub fn draw_axes_with_minor_ticks(
        &mut self,
        plot_left: f32,
        plot_right: f32,
        plot_top: f32,
        plot_bottom: f32,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        color: Color,
    ) {
        let axis_width = self.logical_pixels_to_pixels(1.5);
        let major_tick_size = self.logical_pixels_to_pixels(6.0);
        let minor_tick_size = self.logical_pixels_to_pixels(3.5);
        let tick_width = self.logical_pixels_to_pixels(1.0);
        let minor_tick_width = self.logical_pixels_to_pixels(0.8);

        self.draw_axes_with_minor_ticks_styled(
            plot_left,
            plot_right,
            plot_top,
            plot_bottom,
            x_major_ticks,
            y_major_ticks,
            x_minor_ticks,
            y_minor_ticks,
            tick_direction,
            tick_sides,
            &SpineConfig::default(),
            color,
            axis_width,
            major_tick_size,
            minor_tick_size,
            tick_width,
            minor_tick_width,
        );
    }

    /// Draw axis lines with caller-supplied axis and tick metrics in pixels.
    pub fn draw_axes_with_minor_ticks_styled(
        &mut self,
        plot_left: f32,
        plot_right: f32,
        plot_top: f32,
        plot_bottom: f32,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &TickDirection,
        tick_sides: &TickSides,
        spines: &SpineConfig,
        color: Color,
        axis_width: f32,
        major_tick_size: f32,
        minor_tick_size: f32,
        tick_width: f32,
        minor_tick_width: f32,
    ) {
        let spine_offset = self.render_scale.points_to_pixels(spines.offset.max(0.0));
        let bottom_spine_y = plot_bottom + spine_offset;
        let top_spine_y = plot_top - spine_offset;
        let left_spine_x = plot_left - spine_offset;
        let right_spine_x = plot_right + spine_offset;

        if spines.bottom {
            self.draw_line(
                plot_left,
                bottom_spine_y,
                plot_right,
                bottom_spine_y,
                color,
                axis_width,
                LineStyle::Solid,
            );
        }

        if spines.left {
            self.draw_line(
                left_spine_x,
                plot_top,
                left_spine_x,
                plot_bottom,
                color,
                axis_width,
                LineStyle::Solid,
            );
        }

        if spines.top {
            self.draw_line(
                plot_left,
                top_spine_y,
                plot_right,
                top_spine_y,
                color,
                axis_width,
                LineStyle::Solid,
            );
        }

        if spines.right {
            self.draw_line(
                right_spine_x,
                plot_top,
                right_spine_x,
                plot_bottom,
                color,
                axis_width,
                LineStyle::Solid,
            );
        }

        for (tick_size, tick_width, ticks) in [
            (major_tick_size, tick_width, x_major_ticks),
            (minor_tick_size, minor_tick_width, x_minor_ticks),
        ] {
            for &x in ticks {
                if x >= plot_left && x <= plot_right {
                    if tick_sides.bottom && spines.bottom {
                        let (tick_start, tick_end) = Self::vertical_tick_span(
                            bottom_spine_y,
                            tick_size,
                            tick_direction,
                            false,
                        );
                        self.draw_line(
                            x,
                            tick_start,
                            x,
                            tick_end,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        );
                    }
                    if tick_sides.top && spines.top {
                        let (tick_start, tick_end) =
                            Self::vertical_tick_span(top_spine_y, tick_size, tick_direction, true);
                        self.draw_line(
                            x,
                            tick_start,
                            x,
                            tick_end,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        );
                    }
                }
            }
        }

        for (tick_size, tick_width, ticks) in [
            (major_tick_size, tick_width, y_major_ticks),
            (minor_tick_size, minor_tick_width, y_minor_ticks),
        ] {
            for &y in ticks {
                if y >= plot_top && y <= plot_bottom {
                    if tick_sides.left && spines.left {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            left_spine_x,
                            tick_size,
                            tick_direction,
                            false,
                        );
                        self.draw_line(
                            tick_start,
                            y,
                            tick_end,
                            y,
                            color,
                            tick_width,
                            LineStyle::Solid,
                        );
                    }
                    if tick_sides.right && spines.right {
                        let (tick_start, tick_end) = Self::horizontal_tick_span(
                            right_spine_x,
                            tick_size,
                            tick_direction,
                            true,
                        );
                        self.draw_line(
                            tick_start,
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
                    let label_x = (ytick_right_x - text_width).max(0.0);
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
        self.draw_marker(center_x, y, size, *marker, color);
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
                let scaled_width = self.points_to_pixels(*width);
                self.draw_legend_line_handle(x, y, handle_length, item.color, style, scaled_width);
            }
            LegendItemType::Scatter { marker, size } => {
                let scaled_size = self.points_to_pixels(*size);
                self.draw_legend_scatter_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    marker,
                    scaled_size,
                );
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
            } => {
                let scaled_line_width = self.points_to_pixels(*line_width);
                let scaled_marker_size = self.points_to_pixels(*marker_size);
                self.draw_legend_line_marker_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    line_style,
                    scaled_line_width,
                    marker,
                    scaled_marker_size,
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
                self.draw_marker(center_x, y, marker_size, MarkerStyle::Circle, item.color);
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

        let legend = legend.scaled_for_render(self.render_scale);
        let legend = &legend;
        let spacing = legend.spacing.to_pixels(legend.font_size);
        let (legend_width, legend_height, label_width) = match self.text_engine_mode {
            TextEngineMode::Plain => {
                let char_width = legend.font_size * 0.6;
                let (width, height) = legend.calculate_size(items, char_width);
                let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);
                (width, height, max_label_len as f32 * char_width)
            }
            #[cfg(feature = "typst-math")]
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
        crate::export::write_bytes_atomic(path, svg_string.as_bytes())
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
mod tests;
