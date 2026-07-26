//! One colorbar, drawn by every backend.
//!
//! A colorbar is a gradient strip, a border, a ladder of ticks, a column of tick
//! labels and an optional rotated caption. None of that is raster-specific — the
//! only backend-dependent parts are "put a rectangle here", "put a line here",
//! "put text here" and "how wide is this text?".
//!
//! [`ColorbarCanvas`] is exactly that set of operations, and [`draw_colorbar`]
//! is the geometry, written once against it. The raster backend used to own the
//! whole routine and the SVG backend simply had no colorbar at all: a heatmap
//! exported to SVG silently lost its value scale. Both now call the same body,
//! so neither can drift from the other or lose it again.

use std::borrow::Cow;

use crate::axes::AxisScale;
use crate::core::error::Result;
use crate::render::Color;
use crate::render::skia::{
    colorbar_major_label_anchor_center_from_top, colorbar_major_label_top,
    compute_colorbar_layout_metrics, compute_colorbar_ticks,
};

/// Everything a colorbar needs to know about itself, independent of backend.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColorbarSpec<'a> {
    /// Ramp the strip is sampled from.
    pub colormap: &'a crate::render::ColorMap,
    /// Value at the bottom of the strip.
    pub vmin: f64,
    /// Value at the top of the strip.
    pub vmax: f64,
    /// Left edge of the strip, in pixels.
    pub x: f32,
    /// Top edge of the strip, in pixels.
    pub y: f32,
    /// Strip width, in pixels.
    pub width: f32,
    /// Strip height, in pixels.
    pub height: f32,
    /// Scale the values are distributed along the strip with.
    pub value_scale: &'a AxisScale,
    /// Optional caption, drawn rotated beside the tick labels.
    pub label: Option<&'a str>,
    /// Colour of the border, ticks and text.
    pub foreground_color: Color,
    /// Tick label size, in points.
    pub tick_font_size: f32,
    /// Caption size in points; defaults to a little over the tick size.
    pub label_font_size: Option<f32>,
    /// Whether to draw the unlabelled decade subticks of a log scale.
    pub show_log_subticks: bool,
}

/// The drawing and measurement operations a colorbar needs from a backend.
///
/// Every method is a primitive both the raster and the SVG renderer already had;
/// this trait exists only so the colorbar's geometry can be stated once.
pub(crate) trait ColorbarCanvas {
    /// Convert a size in points to pixels at this canvas' scale.
    fn colorbar_points_to_pixels(&self, points: f32) -> f32;
    /// Convert a logical (96 DPI) pixel length to device pixels.
    fn colorbar_logical_pixels_to_pixels(&self, pixels: f32) -> f32;
    /// The text this canvas will actually lay out for `text`.
    ///
    /// The typst text engine wraps plain labels in a math snippet, and the
    /// measured string has to be the drawn string.
    fn colorbar_label_snippet<'a>(&self, text: &'a str) -> Cow<'a, str>;
    /// `(width, height)` of `text` at `size` pixels.
    fn colorbar_measure_text(&self, text: &str, size: f32) -> Result<(f32, f32)>;
    /// Distance from the top of `text`'s layout box to the centre of its ink.
    fn colorbar_measure_ink_center_from_top(&self, text: &str, size: f32) -> Result<f32>;
    /// Fill a rectangle with a flat colour.
    fn colorbar_fill_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()>;
    /// Stroke a rectangle outline.
    fn colorbar_stroke_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()>;
    /// Draw a straight line.
    fn colorbar_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()>;
    /// Draw text with `(x, y)` at the top-left of its layout box.
    fn colorbar_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()>;
    /// Draw text rotated a quarter turn, centred on `(x, y)`.
    fn colorbar_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()>;
}

/// Draw a colorbar onto any [`ColorbarCanvas`].
///
/// The gradient runs `vmax` at the top to `vmin` at the bottom, ticks and labels
/// sit to its right, and the caption sits beyond them.
pub(crate) fn draw_colorbar<C: ColorbarCanvas + ?Sized>(
    canvas: &mut C,
    spec: &ColorbarSpec<'_>,
) -> Result<()> {
    let ColorbarSpec {
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
    } = *spec;

    let tick_font_size_px = canvas.colorbar_points_to_pixels(tick_font_size);
    let label_font_size_px = label_font_size
        .map(|size| canvas.colorbar_points_to_pixels(size))
        .unwrap_or(tick_font_size_px * 1.1);

    // One segment per pixel row, so the gradient has no anti-aliasing seams.
    let num_segments = (height as usize).max(50);
    let segment_height = height / num_segments as f32;
    for index in 0..num_segments {
        let normalized = 1.0 - (index as f64 / (num_segments - 1).max(1) as f64);
        let color = colormap.sample(normalized);
        let segment_y = y + index as f32 * segment_height;
        // Overlap each segment slightly so no background shows between them.
        canvas.colorbar_fill_rect(x, segment_y, width, segment_height + 0.5, color)?;
    }

    let stroke_width = canvas.colorbar_logical_pixels_to_pixels(1.0);
    canvas.colorbar_stroke_rect(x, y, width, height, foreground_color, stroke_width)?;

    let ticks = compute_colorbar_ticks(vmin, vmax, value_scale, show_log_subticks);
    let mut measured_major_labels = Vec::with_capacity(ticks.major_labels.len());
    let mut max_label_width: f32 = 0.0;
    for label_text in &ticks.major_labels {
        let label_snippet = canvas.colorbar_label_snippet(label_text).into_owned();
        let (text_width, _) = canvas.colorbar_measure_text(&label_snippet, tick_font_size_px)?;
        let ink_center_from_top =
            canvas.colorbar_measure_ink_center_from_top(&label_snippet, tick_font_size_px)?;
        max_label_width = max_label_width.max(text_width);
        measured_major_labels.push((label_snippet, ink_center_from_top));
    }

    let rotated_label_width = match label {
        Some(text) => Some(canvas.colorbar_measure_text(text, label_font_size_px)?.1),
        None => None,
    };
    let log_decade_base_center = match value_scale {
        AxisScale::Log => {
            Some(canvas.colorbar_measure_ink_center_from_top("10", tick_font_size_px)?)
        }
        _ => None,
    };
    let layout = compute_colorbar_layout_metrics(
        width,
        tick_font_size_px,
        max_label_width,
        rotated_label_width,
    );

    for minor_value in &ticks.minor_values {
        let t = value_scale
            .normalized_position(*minor_value, vmin, vmax)
            .clamp(0.0, 1.0);
        let tick_y = y + height * (1.0 - t as f32);
        canvas.colorbar_line(
            x + width,
            tick_y,
            x + width + layout.minor_tick_width,
            tick_y,
            foreground_color,
            stroke_width * 0.8,
        )?;
    }

    for ((value, _), (label_text, ink_center_from_top)) in ticks
        .major_values
        .iter()
        .zip(ticks.major_labels.iter())
        .zip(measured_major_labels.iter())
    {
        let t = value_scale
            .normalized_position(*value, vmin, vmax)
            .clamp(0.0, 1.0);
        let tick_y = y + height * (1.0 - t as f32);

        canvas.colorbar_line(
            x + width,
            tick_y,
            x + width + layout.major_tick_width,
            tick_y,
            foreground_color,
            stroke_width,
        )?;

        let anchor_center = colorbar_major_label_anchor_center_from_top(
            value_scale,
            label_text,
            *ink_center_from_top,
            log_decade_base_center,
        );
        canvas.colorbar_text(
            label_text,
            x + layout.tick_label_x_offset,
            colorbar_major_label_top(tick_y, anchor_center),
            tick_font_size_px,
            foreground_color,
        )?;
    }

    if let Some((text, label_center_x_offset)) = label.zip(layout.rotated_label_center_x_offset) {
        canvas.colorbar_text_rotated(
            text,
            x + label_center_x_offset,
            y + height / 2.0,
            label_font_size_px,
            foreground_color,
        )?;
    }

    Ok(())
}

/// A colorbar a plot type has asked for, before it is placed on a canvas.
///
/// Deriving this is the part that differs per plot type (a heatmap knows its
/// own value scale, a contour reads its range off its levels); placing and
/// drawing it is the same everywhere. Splitting the two is what lets the raster
/// and SVG heatmap paths — and the raster and SVG contour paths — share one
/// answer for "what does this colorbar say?".
#[derive(Debug, Clone)]
pub(crate) struct ColorbarRequest {
    /// Ramp the strip is sampled from.
    pub colormap: crate::render::ColorMap,
    /// Value at the bottom of the strip.
    pub vmin: f64,
    /// Value at the top of the strip.
    pub vmax: f64,
    /// Scale the values are distributed along the strip with.
    pub value_scale: AxisScale,
    /// Optional caption.
    pub label: Option<String>,
    /// Tick label size, in points.
    pub tick_font_size: f32,
    /// Caption size, in points.
    pub label_font_size: f32,
    /// Whether to draw the unlabelled decade subticks of a log scale.
    pub show_log_subticks: bool,
}

impl ColorbarRequest {
    /// Place this colorbar in a pixel rectangle.
    pub(crate) fn spec_at(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        foreground_color: Color,
    ) -> ColorbarSpec<'_> {
        ColorbarSpec {
            colormap: &self.colormap,
            vmin: self.vmin,
            vmax: self.vmax,
            x,
            y,
            width,
            height,
            value_scale: &self.value_scale,
            label: self.label.as_deref(),
            foreground_color,
            tick_font_size: self.tick_font_size,
            label_font_size: Some(self.label_font_size),
            show_log_subticks: self.show_log_subticks,
        }
    }
}
