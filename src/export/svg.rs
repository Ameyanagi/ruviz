//! SVG export functionality
//!
//! Provides vector-based SVG export for plots with full visual fidelity.
//! This renderer is also used as the intermediate format for PDF export.

use crate::core::{
    Legend, LegendFrame, LegendItem, LegendItemType, LegendPosition, LegendSpacingPixels,
    PlottingError, Result, find_best_position,
};
use crate::render::{Color, LineStyle, MarkerStyle};
use std::fmt::Write as FmtWrite;
use std::path::Path;

/// SVG renderer for vector-based plot export
pub struct SvgRenderer {
    width: f32,
    height: f32,
    content: String,
    defs: String,
    clip_id_counter: u32,
}

impl SvgRenderer {
    /// Create a new SVG renderer with specified dimensions
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            content: String::new(),
            defs: String::new(),
            clip_id_counter: 0,
        }
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
        match style {
            LineStyle::Solid => None,
            LineStyle::Dashed => Some("6,3".to_string()),
            LineStyle::Dotted => Some("2,2".to_string()),
            LineStyle::DashDot => Some("6,2,2,2".to_string()),
            LineStyle::DashDotDot => Some("6,2,2,2,2,2".to_string()),
            LineStyle::Custom(pattern) => Some(
                pattern
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        }
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
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
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.2}"{}  stroke-linecap="round"/>"#,
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

    /// Draw text at specified position
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let color_str = self.color_to_svg(color);
        let escaped_text = self.escape_xml(text);
        writeln!(
            self.content,
            r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}">{}</text>"#,
            x, y, size, color_str, escaped_text
        )
        .unwrap();
    }

    /// Draw text centered at specified position
    pub fn draw_text_centered(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let color_str = self.color_to_svg(color);
        let escaped_text = self.escape_xml(text);
        writeln!(
            self.content,
            r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" text-anchor="middle" dominant-baseline="central">{}</text>"#,
            x, y, size, color_str, escaped_text
        )
        .unwrap();
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
    ) {
        let color_str = self.color_to_svg(color);
        let escaped_text = self.escape_xml(text);
        writeln!(
            self.content,
            r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" text-anchor="middle" dominant-baseline="central" transform="rotate({:.1},{:.2},{:.2})">{}</text>"#,
            x, y, size, color_str, angle, x, y, escaped_text
        )
        .unwrap();
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
        let axis_width = 1.5;
        let major_tick_size = 6.0;
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
                self.draw_line(x, plot_bottom, x, tick_end, color, 1.0, LineStyle::Solid);
            }
        }

        // Draw Y-axis tick marks
        for &y in y_ticks {
            if y >= plot_top && y <= plot_bottom {
                let tick_end = plot_left - major_tick_size * tick_dir;
                self.draw_line(plot_left, y, tick_end, y, color, 1.0, LineStyle::Solid);
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
        color: Color,
        font_size: f32,
    ) {
        let tick_label_offset = 15.0;

        // X-axis labels
        for (i, &x) in x_ticks.iter().enumerate() {
            if x >= plot_left && x <= plot_right {
                if let Some(label) = x_labels.get(i) {
                    self.draw_text_centered(
                        label,
                        x,
                        plot_bottom + tick_label_offset,
                        font_size,
                        color,
                    );
                }
            }
        }

        // Y-axis labels (right-aligned)
        for (i, &y) in y_ticks.iter().enumerate() {
            if y >= plot_top && y <= plot_bottom {
                if let Some(label) = y_labels.get(i) {
                    let color_str = self.color_to_svg(color);
                    let escaped_label = self.escape_xml(label);
                    writeln!(
                        self.content,
                        r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" text-anchor="end" dominant-baseline="central">{}</text>"#,
                        plot_left - 8.0, y, font_size, color_str, escaped_label
                    )
                    .unwrap();
                }
            }
        }
    }

    /// Draw legend
    pub fn draw_legend(&mut self, items: &[(String, Color)], x: f32, y: f32, font_size: f32) {
        if items.is_empty() {
            return;
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
            let color_str = self.color_to_svg(Color::BLACK);
            let escaped_label = self.escape_xml(label);
            writeln!(
                self.content,
                r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="central">{}</text>"#,
                x + 8.0 + swatch_size + swatch_gap, item_y + swatch_size / 2.0, font_size, color_str, escaped_label
            )
            .unwrap();
        }
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
        let dash_array = match style {
            LineStyle::Solid => String::new(),
            LineStyle::Dashed => "stroke-dasharray=\"6,4\"".to_string(),
            LineStyle::Dotted => "stroke-dasharray=\"2,2\"".to_string(),
            LineStyle::DashDot => "stroke-dasharray=\"6,2,2,2\"".to_string(),
            LineStyle::DashDotDot => "stroke-dasharray=\"6,2,2,2,2,2\"".to_string(),
            LineStyle::Custom(pattern) => {
                let pattern_str = pattern
                    .iter()
                    .map(|v| format!("{:.1}", v))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("stroke-dasharray=\"{}\"", pattern_str)
            }
        };

        let color_str = self.color_to_svg(color);
        writeln!(
            self.content,
            r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="{:.1}" {}/>"#,
            x, y, x + length, y, color_str, width, dash_array
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
                let cap_size = handle_height * 0.4;
                self.draw_legend_line_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    &LineStyle::Solid,
                    1.5,
                );
                // Left cap
                let color_str = self.color_to_svg(item.color);
                writeln!(
                    self.content,
                    r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5"/>"#,
                    x, y - cap_size, x, y + cap_size, color_str
                )
                .unwrap();
                // Right cap
                writeln!(
                    self.content,
                    r#"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5"/>"#,
                    x + handle_length, y - cap_size, x + handle_length, y + cap_size, color_str
                )
                .unwrap();
            }
        }
    }

    /// Draw legend frame with background and optional border
    fn draw_legend_frame(&mut self, x: f32, y: f32, width: f32, height: f32, frame: &LegendFrame) {
        if !frame.visible {
            return;
        }

        let radius = frame.corner_radius;

        // Draw shadow if enabled
        if frame.shadow {
            let (shadow_dx, shadow_dy) = frame.shadow_offset;
            if radius > 0.0 {
                self.draw_rounded_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    radius,
                    frame.shadow_color,
                    true,
                );
            } else {
                self.draw_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    frame.shadow_color,
                    true,
                );
            }
        }

        // Draw background
        if radius > 0.0 {
            self.draw_rounded_rectangle(x, y, width, height, radius, frame.background, true);
        } else {
            self.draw_rectangle(x, y, width, height, frame.background, true);
        }

        // Draw border if specified
        if let Some(border_color) = frame.border_color {
            if radius > 0.0 {
                self.draw_rounded_rectangle(x, y, width, height, radius, border_color, false);
            } else {
                self.draw_rectangle(x, y, width, height, border_color, false);
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
    ) {
        if items.is_empty() || !legend.enabled {
            return;
        }

        let spacing = legend.spacing.to_pixels(legend.font_size);
        let char_width = legend.font_size * 0.6;

        // Calculate legend size
        let (legend_width, legend_height) = legend.calculate_size(items, char_width);

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
            &legend.frame,
        );

        // Starting position for items
        let item_x = legend_x + spacing.border_pad;
        let mut item_y = legend_y + spacing.border_pad + legend.font_size / 2.0;

        // Draw title if present
        if let Some(ref title) = legend.title {
            let title_x = legend_x + legend_width / 2.0;
            self.draw_text_centered(title, title_x, item_y, legend.font_size, legend.text_color);
            item_y += legend.font_size + spacing.label_spacing;
        }

        // Calculate items per column
        let items_per_col = (items.len() + legend.columns - 1) / legend.columns;

        // Calculate column width
        let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);
        let label_width = max_label_len as f32 * char_width;
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
                let color_str = self.color_to_svg(legend.text_color);
                let escaped_label = self.escape_xml(&item.label);
                writeln!(
                    self.content,
                    r#"  <text x="{:.2}" y="{:.2}" font-family="sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="central">{}</text>"#,
                    text_x, row_y, legend.font_size, color_str, escaped_label
                )
                .unwrap();

                row_y += legend.font_size + spacing.label_spacing;
            }
        }
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
            Some("6,3".to_string())
        );
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
}
