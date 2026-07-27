//! SVG export functionality
//!
//! Provides vector-based SVG export for plots with full visual fidelity.
//! This renderer is also used as the intermediate format for PDF export.

use crate::core::{
    Legend, LegendItem, LegendItemType, LegendSpacingPixels, LegendStyle, PlottingError,
    RenderScale, Result, SpineConfig, TextAlign, TextStyle,
    legend::{
        LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT, LegendLayout, LegendOccupancy, LegendPlacement,
        layout_legend, legacy_legend_swatch_edge, measure_legend_size,
    },
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

#[cfg(feature = "3d")]
use base64::Engine as _;

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
    /// First shape that reached an emitter with a non-finite dimension.
    ///
    /// Latched rather than returned because most emitters are infallible by
    /// signature; [`SvgRenderer::check_geometry`] and [`SvgRenderer::save`]
    /// turn it into an `Err`. See the non-finite policy on
    /// [`SvgRenderer::reject_shape`].
    invalid_geometry: Option<String>,
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
            invalid_geometry: None,
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

    // ------------------------------------------------------------------
    // Non-finite geometry policy
    // ------------------------------------------------------------------
    //
    // `NaN` and `±inf` must never be formatted into an SVG attribute:
    // `height="NaN"` is not valid SVG, and viewers disagree about how much of
    // the document to discard when they meet one. Every emitter therefore
    // checks the numbers it is about to print, and there are exactly two
    // permitted responses. When you add an emitter, pick the bucket it belongs
    // to and use the matching helper — do not invent a third answer.
    //
    // 1. **Open stroked geometry** — polylines, line segments, marker strokes,
    //    text anchors. A non-finite coordinate is a *gap*: the sample has no
    //    position on these axes, so the stroke is split around it (or the
    //    single element is skipped) and everything else is still drawn. This is
    //    the intended log-axis behaviour, matching
    //    `raster_batches::representable_sample_runs`. Use [`Self::all_finite`]
    //    / [`Self::finite_point_runs`]. No error is raised: a gap is data the
    //    axes genuinely cannot show, not a bug.
    //
    // 2. **Shapes with defining dimensions** — rectangles, circles, images,
    //    clip rects, and closed polygons. There is no meaningful partial shape:
    //    a rect with a `NaN` height means unvalidated geometry got past the
    //    series-level checks, i.e. an internal invariant failed. The element is
    //    not emitted and the failure is latched by [`Self::reject_shape`], which
    //    [`Self::check_geometry`] and [`Self::save`] surface as an `Err`.

    /// Is every one of these numbers printable into an SVG attribute?
    fn all_finite(values: &[f32]) -> bool {
        values.iter().all(|value| value.is_finite())
    }

    /// Are all of these vertices printable into an SVG attribute?
    fn all_points_finite(points: &[(f32, f32)]) -> bool {
        points.iter().all(|&(x, y)| x.is_finite() && y.is_finite())
    }

    /// Split a vertex list at every non-finite point.
    ///
    /// Bucket 1 of the policy above: the surviving runs are stroked as separate
    /// elements, so the line breaks at the hole instead of being drawn straight
    /// through it. Runs shorter than two points are kept here and dropped by the
    /// emitter, which already refuses a one-point polyline.
    fn finite_point_runs(points: &[(f32, f32)]) -> Vec<&[(f32, f32)]> {
        let mut runs = Vec::new();
        let mut start = 0usize;
        for (index, &(x, y)) in points.iter().enumerate() {
            if x.is_finite() && y.is_finite() {
                continue;
            }
            if index > start {
                runs.push(&points[start..index]);
            }
            start = index + 1;
        }
        if points.len() > start {
            runs.push(&points[start..]);
        }
        runs
    }

    /// Bucket 2 of the policy above: refuse a shape whose defining dimensions
    /// are not all finite.
    ///
    /// Returns `false` when the caller must not emit the element. Only the first
    /// rejection is kept — it is the one closest to the cause.
    fn reject_shape(&mut self, element: &str, dims: &[(&str, f32)]) -> bool {
        let Some((name, value)) = dims.iter().copied().find(|&(_, value)| !value.is_finite())
        else {
            return false;
        };
        self.latch_invalid_geometry(element, &format!("non-finite {name} ({value})"));
        true
    }

    /// Bucket 2 for closed polygons, whose vertices *are* the defining shape.
    fn reject_polygon(&mut self, element: &str, points: &[(f32, f32)]) -> bool {
        let Some((index, &(x, y))) = points
            .iter()
            .enumerate()
            .find(|&(_, &(x, y))| !x.is_finite() || !y.is_finite())
        else {
            return false;
        };
        self.latch_invalid_geometry(element, &format!("non-finite vertex {index} ({x}, {y})"));
        true
    }

    fn latch_invalid_geometry(&mut self, element: &str, detail: &str) {
        if self.invalid_geometry.is_some() {
            return;
        }
        self.invalid_geometry = Some(format!(
            "SVG <{element}> was given a {detail}. Geometry reached the emitter without \
             being validated; this is an internal invariant failure in the renderer, not \
             a limit of the supplied data."
        ));
    }

    /// `Err` if any shape was refused for a non-finite dimension.
    ///
    /// The emitted SVG is always well formed — the offending element is simply
    /// missing — so this is the only signal that something was dropped. Callers
    /// that hand out an SVG string must check it; [`Self::save`] already does.
    pub fn check_geometry(&self) -> Result<()> {
        match &self.invalid_geometry {
            Some(detail) => Err(PlottingError::RenderError(detail.clone())),
            None => Ok(()),
        }
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
        self.scaled_dash_pattern(style)
            // A non-finite dash length would print as `stroke-dasharray="NaN"`.
            // Dropping the attribute leaves a solid stroke, which is a far
            // better failure than an unparseable one.
            .filter(|pattern| Self::all_finite(pattern))
            .map(|pattern| {
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
        if !value.is_finite() {
            // Leave the measured dimension the embedded document already has
            // rather than overwriting it with `NaN`.
            return;
        }
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
        if self.reject_shape(
            "rect",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        ) {
            return;
        }
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

    /// Draw a filled rectangle with no anti-aliased edges.
    ///
    /// Twin of the raster backend's pixel-aligned solid rectangle: for geometry
    /// that tiles — heatmap cells, filled contour bands — anti-aliased edges
    /// leave a pale seam wherever two shapes meet.
    pub fn draw_seamless_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) {
        if self.reject_shape(
            "rect",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        ) {
            return;
        }
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let fill = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{fill}" shape-rendering="crispEdges"/>"#
        )
        .unwrap();
    }

    /// Draw a rectangle with an explicit fill and/or an explicit edge.
    ///
    /// Twin of [`SkiaRenderer::draw_rectangle_styled`](crate::render::SkiaRenderer::draw_rectangle_styled):
    /// `edge` is `(colour, width_in_points)` and the width is scaled by this
    /// renderer's [`RenderScale`], so PNG and SVG agree at every DPI.
    pub fn draw_rectangle_styled(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        edge: Option<(Color, f32)>,
    ) {
        if fill.is_none() && edge.is_none() {
            return;
        }
        // A bar or histogram column whose height could not be projected is the
        // exact corruption this guard exists for: `<rect height="NaN">`.
        if self.reject_shape(
            "rect",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        ) {
            return;
        }
        // Twin of the raster guard: a rectangle with no area has no interior to
        // fill and no boundary to stroke, so a zero-value bar stays unmarked.
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let fill_attr = match fill {
            Some(color) => self.color_to_svg(color),
            None => "none".to_string(),
        };
        let stroke_attr = match edge.filter(|&(_, width_pt)| width_pt.is_finite()) {
            Some((color, width_pt)) => {
                let stroke_color = self.color_to_svg(color);
                let stroke_width = self.points_to_pixels(width_pt);
                format!(r#" stroke="{stroke_color}" stroke-width="{stroke_width:.2}""#)
            }
            None => String::new(),
        };
        writeln!(
            self.content,
            r#"  <rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="{fill_attr}"{stroke_attr}/>"#
        )
        .unwrap();
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
        if self.reject_shape(
            "rect",
            &[
                ("x", x),
                ("y", y),
                ("width", width),
                ("height", height),
                ("corner radius", corner_radius),
            ],
        ) {
            return;
        }
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
        // Open stroked geometry: a segment with an endpoint the axes cannot
        // place is a gap, so it is dropped rather than drawn or raised.
        if !Self::all_finite(&[x1, y1, x2, y2, width]) {
            return;
        }
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

    /// Draw a polyline (connected line segments).
    ///
    /// A vertex the axes cannot place (`NaN`/`±inf`) **breaks** the line: the
    /// run before the hole and the run after it are emitted as separate
    /// `<polyline>` elements, so the stroke shows the gap instead of inventing
    /// a segment across it.
    pub fn draw_polyline(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: LineStyle,
    ) {
        if !width.is_finite() {
            return;
        }
        if Self::all_points_finite(points) {
            self.emit_polyline(points, color, width, &style);
            return;
        }
        for run in Self::finite_point_runs(points) {
            self.emit_polyline(run, color, width, &style);
        }
    }

    /// Emit one `<polyline>`. Every vertex must already be finite.
    fn emit_polyline(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: &LineStyle,
    ) {
        if points.len() < 2 {
            return;
        }

        let color_str = self.color_to_svg(color);
        let dasharray = self.line_style_to_dasharray(style);

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
    ///
    /// Unlike a polyline, a closed filled shape has no sensible "gap": dropping
    /// a vertex silently redraws a different area, so a non-finite vertex is
    /// refused (see the non-finite geometry policy).
    pub fn draw_filled_polygon(&mut self, points: &[(f32, f32)], color: Color) {
        if points.len() < 3 {
            return;
        }
        if self.reject_polygon("polygon", points) {
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

    /// Embed a straight-alpha RGBA image as a PNG data URI.
    #[cfg(feature = "3d")]
    pub(crate) fn draw_embedded_png(
        &mut self,
        image: &crate::core::plot::Image,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<()> {
        if !Self::all_finite(&[x, y, width, height]) {
            return Err(PlottingError::RenderError(format!(
                "SVG <image> was given a non-finite placement ({x}, {y}, {width}x{height}). \
                 Geometry reached the emitter without being validated; this is an internal \
                 invariant failure in the renderer, not a limit of the supplied data."
            )));
        }
        let png = image.encode_png()?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(png);
        writeln!(
            self.content,
            r#"  <image x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" href="data:image/png;base64,{encoded}"/>"#
        )
        .map_err(|error| {
            PlottingError::RenderError(format!("failed to compose embedded SVG image: {error}"))
        })?;
        Ok(())
    }

    /// Draw a polygon outline.
    pub fn draw_polygon_outline(&mut self, points: &[(f32, f32)], color: Color, width: f32) {
        if points.len() < 3 {
            return;
        }
        if self.reject_shape("polygon", &[("stroke width", width)])
            || self.reject_polygon("polygon", points)
        {
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
        if self.reject_shape("circle", &[("cx", cx), ("cy", cy), ("r", r)]) {
            return;
        }
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
        if self.reject_polygon("polygon", points) {
            return;
        }
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
        // Open stroked geometry: skip, do not raise. `draw_marker` already
        // refuses an unplaceable marker, so this is pure defence in depth.
        if !Self::all_finite(&[x1, y1, x2, y2, width]) {
            return;
        }
        let color_str = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}" stroke-linecap="butt"/>"#,
            x1, y1, x2, y2, color_str, width
        )
        .unwrap();
    }

    /// Vertices of the polygonal marker styles, or `None` for the round and
    /// line-drawn ones.
    ///
    /// Shared by the fill pass and the edge pass so a rim can never sit on a
    /// different outline than the shape it is rimming.
    fn marker_polygon(x: f32, y: f32, radius: f32, style: MarkerStyle) -> Option<Vec<(f32, f32)>> {
        match style {
            MarkerStyle::Triangle | MarkerStyle::TriangleOpen => Some(vec![
                (x, y - radius),
                (x - radius * 0.866, y + radius * 0.5),
                (x + radius * 0.866, y + radius * 0.5),
            ]),
            MarkerStyle::TriangleDown => Some(vec![
                (x, y + radius),
                (x - radius * 0.866, y - radius * 0.5),
                (x + radius * 0.866, y - radius * 0.5),
            ]),
            MarkerStyle::Diamond | MarkerStyle::DiamondOpen => Some(vec![
                (x, y - radius),
                (x + radius, y),
                (x, y + radius),
                (x - radius, y),
            ]),
            _ => None,
        }
    }

    /// Draw a marker with an optional edge (rim), matching the raster semantics.
    ///
    /// Twin of [`SkiaRenderer::draw_markers_styled_clipped`](crate::render::SkiaRenderer::draw_markers_styled_clipped):
    /// `edge` is `(colour, width_in_points)` and the width is scaled by this
    /// renderer's [`RenderScale`], so a PNG and an SVG rim are the same physical
    /// thickness at every DPI. Only the closed filled styles take an edge — see
    /// [`MarkerStyle::takes_edge`].
    pub fn draw_marker_styled(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
    ) {
        // A marker is point geometry: a sample the axes cannot place is simply
        // not drawn (bucket 1). Caught here so the shape emitters below — which
        // *do* raise — never see the non-finite centre.
        if !Self::all_finite(&[x, y, size]) {
            return;
        }
        self.draw_marker(x, y, size, style, color);

        let Some((edge_color, width_pt)) = edge
            .filter(|&(_, width_pt)| width_pt.is_finite())
            .filter(|_| style.takes_edge())
        else {
            return;
        };
        let width_px = self.points_to_pixels(width_pt);
        if width_px <= 0.0 || edge_color.a == 0 {
            return;
        }

        let radius = size / 2.0;
        if let Some(points) = Self::marker_polygon(x, y, radius, style) {
            self.draw_polygon_marker(&points, edge_color, Some(width_px));
            return;
        }
        match style {
            MarkerStyle::Circle => {
                let color_str = self.color_to_svg(edge_color);
                writeln!(
                    self.content,
                    r#"  <circle cx="{x:.2}" cy="{y:.2}" r="{radius:.2}" fill="none" stroke="{color_str}" stroke-width="{width_px:.2}"/>"#
                )
                .unwrap();
            }
            MarkerStyle::Square => self.draw_rectangle_styled(
                x - radius,
                y - radius,
                size,
                size,
                None,
                Some((edge_color, width_pt)),
            ),
            // `takes_edge` already rejected every other style.
            _ => {}
        }
    }

    /// Draw a marker at a point, matching the raster marker semantics.
    ///
    /// A marker whose centre or size is not finite is a sample the axes cannot
    /// place, so it is skipped — the same "gap" rule polylines follow.
    pub fn draw_marker(&mut self, x: f32, y: f32, size: f32, style: MarkerStyle, color: Color) {
        if !Self::all_finite(&[x, y, size]) {
            return;
        }
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
            MarkerStyle::Triangle | MarkerStyle::TriangleDown | MarkerStyle::Diamond => {
                let points = Self::marker_polygon(x, y, radius, style)
                    .expect("filled polygonal marker always has vertices");
                self.draw_polygon_marker(&points, color, None)
            }
            MarkerStyle::TriangleOpen | MarkerStyle::DiamondOpen => {
                let points = Self::marker_polygon(x, y, radius, style)
                    .expect("open polygonal marker always has vertices");
                self.draw_polygon_marker(&points, color, Some((size * 0.15).max(1.0)))
            }
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
        // A label is anchored to geometry: if the anchor cannot be placed the
        // label is skipped, exactly as an unplaceable marker is. Text carries no
        // data of its own, so this is bucket 1, not a latched failure.
        if !Self::all_finite(&[x, y, style.rotation]) {
            return Ok(());
        }
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
        // See `draw_styled_text`: an unplaceable label is skipped, not raised.
        if !Self::all_finite(&[x, y, size]) {
            return Ok(());
        }
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
        // See `draw_styled_text`: an unplaceable label is skipped, not raised.
        if !Self::all_finite(&[x, y, size]) {
            return Ok(());
        }
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
        // See `draw_styled_text`: an unplaceable label is skipped, not raised.
        if !Self::all_finite(&[x, y, size, angle]) {
            return Ok(());
        }
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

    /// Draw a colorbar for heatmaps and filled contours.
    ///
    /// Twin of [`SkiaRenderer::draw_colorbar`](crate::render::SkiaRenderer::draw_colorbar):
    /// both hand the same internal `ColorbarSpec` to the same
    /// routine, so the SVG export carries the value scale the PNG shows instead
    /// of dropping it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_colorbar(
        &mut self,
        colormap: &crate::render::ColorMap,
        vmin: f64,
        vmax: f64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        value_scale: &crate::axes::AxisScale,
        label: Option<&str>,
        foreground_color: Color,
        tick_font_size: f32,
        label_font_size: Option<f32>,
        show_log_subticks: bool,
    ) -> Result<()> {
        crate::render::colorbar::draw_colorbar(
            self,
            &crate::render::colorbar::ColorbarSpec {
                colormap,
                vmin,
                vmax,
                x,
                y,
                width,
                height,
                value_scale,
                label,
                foreground_color,
                tick_font_size,
                label_font_size,
                show_log_subticks,
            },
        )
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
            if x >= plot_left
                && x <= plot_right
                && let Some(label) = x_labels.get(i)
            {
                let label_snippet = self.generated_label(label);
                let (text_width, _) = self.measure_text_for_layout(&label_snippet, font_size)?;
                let label_x = (x - text_width / 2.0).max(0.0).min(self.width - text_width);
                self.draw_text(&label_snippet, label_x, xtick_baseline_y, font_size, color)?;
            }
        }

        // Y-axis labels
        for (i, &y) in y_ticks.iter().enumerate() {
            if y >= plot_top
                && y <= plot_bottom
                && let Some(label) = y_labels.get(i)
            {
                let label_snippet = self.generated_label(label);
                let (text_width, text_height) =
                    self.measure_text_for_layout(&label_snippet, font_size)?;
                let label_x = (ytick_right_x - text_width).max(0.0);
                let centered_y = y - text_height / 2.0;
                self.draw_text(&label_snippet, label_x, centered_y, font_size, color)?;
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
            Color::from_rgba(255, 255, 255, 230),
            true,
        );
        self.draw_rectangle(
            x,
            y,
            legend_width,
            legend_height,
            Color::from_rgba(0, 0, 0, 100),
            false,
        );

        // Draw legend items
        for (i, (label, color)) in items.iter().enumerate() {
            let item_y = y + 8.0 + i as f32 * item_height;

            // Draw color swatch. The panel above is near-white, so a white or
            // near-white series colour needs a neutral contour of its own to
            // stay visible. The fill is left exactly as the series colour.
            self.draw_rectangle_styled(
                x + 8.0,
                item_y,
                swatch_size,
                swatch_size,
                Some(*color),
                Some((
                    legacy_legend_swatch_edge(*color),
                    LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT,
                )),
            );

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
        if !Self::all_finite(&[x, y, length, width]) {
            return;
        }
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
    /// Draw a marker handle in the legend
    ///
    /// `edge` is `(colour, width_in_points)` — the rim the plotted markers
    /// carry. `draw_marker_styled` scales the width through the same
    /// [`RenderScale`] as the raster twin, so the key matches at any DPI.
    fn draw_legend_scatter_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        marker: &MarkerStyle,
        size: f32,
        edge: Option<(Color, f32)>,
    ) {
        let center_x = x + length / 2.0;
        self.draw_marker_styled(center_x, y, size, *marker, color, edge);
    }

    /// Draw a bar handle in the legend
    ///
    /// `edge` is `(colour, width_in_points)` — the very edge the patch it
    /// stands for is stroked with. The width goes through the same
    /// [`RenderScale`] as the raster twin, so the key stays faithful at any DPI.
    fn draw_legend_bar_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        height: f32,
        color: Color,
        edge: Option<(Color, f32)>,
    ) {
        let rect_y = y - height / 2.0;
        self.draw_rectangle_styled(x, rect_y, length, height, Some(color), edge);
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
        marker_edge: Option<(Color, f32)>,
    ) {
        self.draw_legend_line_handle(x, y, length, color, line_style, line_width);
        self.draw_legend_scatter_handle(x, y, length, color, marker, marker_size, marker_edge);
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

        // Legend geometry comes from layout, never from data, so a non-finite
        // value here means the layout itself is broken. Skipping the handle
        // keeps the key readable; the individual emitters below still guard.
        if !Self::all_finite(&[x, y, handle_length, handle_height]) {
            return;
        }

        match &item.item_type {
            LegendItemType::Line { style, width } => {
                let scaled_width = self.points_to_pixels(*width);
                self.draw_legend_line_handle(x, y, handle_length, item.color, style, scaled_width);
            }
            LegendItemType::Scatter { marker, size, edge } => {
                let scaled_size = self.points_to_pixels(*size);
                self.draw_legend_scatter_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    marker,
                    scaled_size,
                    *edge,
                );
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
                marker_edge,
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
                    *marker_edge,
                );
            }
            LegendItemType::Bar { edge } | LegendItemType::Histogram { edge } => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color, *edge);
            }
            LegendItemType::Area { edge_color } => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color, None);
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
    /// Paint a legend frame: shadow, face and edge, from one [`LegendStyle`].
    ///
    /// `pub(crate)` so the 3D overlay emits its legend box from this exact
    /// code rather than a themed look-alike of it. `style` must already be in
    /// device pixels (see `Legend::scaled_for_render`).
    pub(crate) fn draw_legend_frame(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &LegendStyle,
    ) {
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

    /// Size and place the legend through the one shared layout.
    ///
    /// `legend` must already be scaled for this renderer. The measurement
    /// callback is this backend's own — including its Typst branch — which is
    /// how the two backends keep honest text metrics without either of them
    /// owning a second copy of the layout.
    fn legend_layout(
        &self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: (f32, f32, f32, f32),
        placement: LegendPlacement<'_>,
    ) -> Result<LegendLayout> {
        layout_legend(items, legend, plot_area, placement, |text| {
            Ok(self.measure_text_for_layout(text, legend.font_size)?.0)
        })
    }

    /// The room this legend needs, measured exactly as it will be drawn.
    ///
    /// Shares [`layout_legend`] with [`SvgRenderer::draw_legend_full`], so an
    /// SVG legend cannot be reserved at one width and drawn at another.
    /// `legend` is in points and is scaled for this renderer internally.
    pub(crate) fn measure_legend(
        &self,
        items: &[LegendItem],
        legend: &Legend,
    ) -> Result<(f32, f32)> {
        let legend = legend.scaled_for_render(self.render_scale);
        measure_legend_size(items, &legend, |text| {
            Ok(self.measure_text_for_layout(text, legend.font_size)?.0)
        })
    }

    /// Draw legend with full LegendItem support
    ///
    /// This is the new legend drawing method that properly renders different
    /// series types with their correct visual handles.
    ///
    /// `occupancy` is only consulted for
    /// [`LegendPosition::Best`](crate::core::LegendPosition::Best); `None`
    /// means "no idea where the data is", which degrades to `UpperRight`.
    pub fn draw_legend_full(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: (f32, f32, f32, f32), // (left, top, right, bottom)
        occupancy: Option<&LegendOccupancy>,
    ) -> Result<()> {
        self.draw_legend_full_resolved(items, legend, plot_area, occupancy, None)
    }

    pub(crate) fn draw_legend_full_resolved(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: (f32, f32, f32, f32),
        occupancy: Option<&LegendOccupancy>,
        resolved_rect: Option<(f32, f32, f32, f32)>,
    ) -> Result<()> {
        if items.is_empty() || !legend.enabled {
            return Ok(());
        }

        let legend = legend.scaled_for_render(self.render_scale);
        let placement = LegendPlacement {
            reserved: resolved_rect,
            occupancy,
        };
        let layout = self.legend_layout(items, &legend, plot_area, placement)?;

        self.draw_legend_frame(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            &legend.style,
        );

        if let (Some(title_layout), Some(title)) = (layout.title, legend.title.as_deref()) {
            self.draw_text_centered(
                title,
                title_layout.center_x,
                title_layout.top_y,
                layout.font_size,
                legend.text_color,
            )?;
        }

        for entry in &layout.entries {
            let item = &items[entry.item_index];
            self.draw_legend_handle(item, entry.handle_x, entry.handle_center_y, &layout.spacing);
            self.draw_text(
                &item.label,
                entry.label_x,
                entry.label_top_y,
                layout.font_size,
                legend.text_color,
            )?;
        }

        Ok(())
    }

    /// Add a clip path definition and return the ID
    pub fn add_clip_rect(&mut self, x: f32, y: f32, width: f32, height: f32) -> String {
        let clip_id = self.next_clip_id();
        // A clip rect is a shape with defining dimensions, so a non-finite one
        // is a latched failure. It still has to resolve to *something* — an
        // empty `<clipPath>` hides the whole group — so fall back to the full
        // canvas, which leaves the document readable while `check_geometry`
        // reports the fault.
        let (x, y, width, height) = if self.reject_shape(
            "clipPath",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        ) {
            (0.0, 0.0, self.width.max(0.0), self.height.max(0.0))
        } else {
            (x, y, width, height)
        };
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
        // Never write a file that is missing geometry we refused to draw.
        self.check_geometry()?;
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

/// The SVG backend's colorbar primitives.
///
/// The geometry lives in [`crate::render::colorbar::draw_colorbar`]; this only
/// says how each primitive becomes an SVG element.
impl crate::render::colorbar::ColorbarCanvas for SvgRenderer {
    fn colorbar_points_to_pixels(&self, points: f32) -> f32 {
        self.points_to_pixels(points)
    }

    fn colorbar_logical_pixels_to_pixels(&self, pixels: f32) -> f32 {
        self.logical_pixels_to_pixels(pixels)
    }

    fn colorbar_label_snippet<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.generated_label(text)
    }

    fn colorbar_measure_text(&self, text: &str, size: f32) -> Result<(f32, f32)> {
        self.measure_text_for_layout(text, size)
    }

    fn colorbar_measure_ink_center_from_top(&self, text: &str, size: f32) -> Result<f32> {
        match self.text_engine_mode {
            TextEngineMode::Plain => {
                let config = FontConfig::new(self.font_family.clone(), size);
                self.text_renderer
                    .measure_text_ink_center_from_top(text, &config)
            }
            #[cfg(feature = "typst-math")]
            TextEngineMode::Typst => Ok(self.measure_text_for_layout(text, size)?.1 / 2.0),
        }
    }

    fn colorbar_fill_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        self.draw_rectangle(x, y, width, height, color, true);
        Ok(())
    }

    fn colorbar_stroke_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()> {
        if self.reject_shape(
            "rect",
            &[
                ("x", x),
                ("y", y),
                ("width", width),
                ("height", height),
                ("stroke width", stroke_width),
            ],
        ) {
            return self.check_geometry();
        }
        let stroke = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" fill="none" stroke="{stroke}" stroke-width="{stroke_width:.2}"/>"#
        )
        .unwrap();
        Ok(())
    }

    fn colorbar_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()> {
        self.draw_line(x1, y1, x2, y2, color, stroke_width, LineStyle::Solid);
        Ok(())
    }

    fn colorbar_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        self.draw_text(text, x, y, size, color)
    }

    fn colorbar_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        self.draw_text_rotated(text, x, y, size, color, -90.0)
    }
}

#[cfg(test)]
mod tests;

/// Regressions for the non-finite geometry policy documented on
/// [`SvgRenderer::reject_shape`].
///
/// Kept in this file, next to the guards, so a change to the policy and the
/// tests that pin it stay in the same diff.
#[cfg(test)]
mod non_finite_geometry_tests {
    use super::*;
    use crate::core::plot::Plot;

    fn elements<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
        let prefix = format!("<{tag} ");
        svg.lines()
            .filter(|line| line.trim_start().starts_with(&prefix))
            .collect()
    }

    /// The corruption this policy exists to stop: `height="NaN"` and
    /// `y1="NaN"` are not valid SVG, and viewers disagree about how much of the
    /// document to throw away when they meet one.
    #[test]
    fn test_a_non_finite_dimension_never_reaches_an_svg_attribute() {
        let mut renderer = SvgRenderer::new(200.0, 200.0);
        renderer.draw_rectangle(10.0, 10.0, 50.0, f32::NAN, Color::from_rgb(0, 0, 0), true);
        renderer.draw_line(
            0.0,
            f32::NAN,
            100.0,
            f32::NAN,
            Color::from_rgb(0, 0, 0),
            1.0,
            LineStyle::Solid,
        );
        renderer.draw_circle(f32::INFINITY, 10.0, 4.0, Color::from_rgb(0, 0, 0), true);

        let svg = renderer.to_svg_string();
        assert!(
            !svg.contains("NaN") && !svg.contains("inf"),
            "no non-finite number may be printed into the document: {svg}"
        );
        // The rect and the circle are shapes, so the failure is reported.
        let error = renderer
            .check_geometry()
            .expect_err("a refused shape must be reported, not silently dropped");
        assert!(
            error.to_string().contains("internal invariant"),
            "the message must read as an unvalidated-input bug: {error}"
        );
    }

    /// Bucket 1. An unrepresentable sample inside a line is a *gap*: the runs
    /// either side of it are stroked separately, so the line shows the hole
    /// rather than a segment drawn straight across it.
    #[test]
    fn test_a_polyline_breaks_into_two_elements_at_an_interior_hole() {
        let mut renderer = SvgRenderer::new(200.0, 200.0);
        renderer.draw_polyline(
            &[
                (10.0, 10.0),
                (20.0, 20.0),
                (f32::NAN, f32::NAN),
                (40.0, 40.0),
                (50.0, 50.0),
            ],
            Color::from_rgb(0, 0, 0),
            1.0,
            LineStyle::Solid,
        );

        let svg = renderer.to_svg_string();
        assert_eq!(
            elements(&svg, "polyline").len(),
            2,
            "the hole must split the line into two elements: {svg}"
        );
        assert!(!svg.contains("NaN"), "no NaN may survive into the document");
        renderer
            .check_geometry()
            .expect("a gap in a line is data the axes cannot show, not a failure");
    }

    /// A gap at either end trims the line instead of splitting it, and a line
    /// with nothing left emits nothing at all.
    #[test]
    fn test_a_polyline_hole_at_the_edge_trims_rather_than_splits() {
        let mut renderer = SvgRenderer::new(200.0, 200.0);
        renderer.draw_polyline(
            &[(f32::NAN, 0.0), (10.0, 10.0), (20.0, 20.0)],
            Color::from_rgb(0, 0, 0),
            1.0,
            LineStyle::Solid,
        );
        renderer.draw_polyline(
            &[(f32::NAN, 0.0), (f32::NAN, 1.0)],
            Color::from_rgb(0, 0, 0),
            1.0,
            LineStyle::Solid,
        );

        let svg = renderer.to_svg_string();
        assert_eq!(
            elements(&svg, "polyline").len(),
            1,
            "a leading hole trims; a wholly unrepresentable line draws nothing: {svg}"
        );
    }

    /// End to end. A line series whose samples are mostly representable on a
    /// log axis must export the representable part — and the exported document
    /// must not contain the literal `NaN` that the unrepresentable sample used
    /// to project to.
    ///
    /// The limits are explicit so the test pins the emitter, not whatever
    /// autoscaling decides to do with a non-positive sample.
    #[test]
    fn test_exported_svg_of_a_log_axis_line_contains_no_literal_nan() {
        let x = vec![1.0_f64, 2.0, 3.0, 4.0];
        let y = vec![1.0_f64, 0.0, 100.0, 1000.0];

        let svg = Plot::new()
            .line(&x, &y)
            .yscale(crate::axes::AxisScale::Log)
            .ylim(1.0, 1000.0)
            .render_to_svg()
            .expect("a line keeps its representable samples on a log axis");

        assert!(
            !svg.contains("NaN"),
            "an unrepresentable sample must never be printed into the document"
        );
        // Checked on attribute values only: "inf" is too short to blanket-ban
        // across a document that also carries user text.
        assert!(
            !svg.contains(r#"="inf"#) && !svg.contains(r#"="-inf"#),
            "an unrepresentable sample must never be printed into the document"
        );
    }
}
