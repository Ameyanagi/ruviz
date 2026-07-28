use super::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorbarTicks {
    pub major_values: Vec<f64>,
    pub major_labels: Vec<String>,
    pub minor_values: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorbarLayoutMetrics {
    pub major_tick_width: f32,
    pub minor_tick_width: f32,
    pub tick_label_x_offset: f32,
    pub rotated_label_center_x_offset: Option<f32>,
    pub total_extent: f32,
}

/// Helper function to calculate plot area with margins
pub fn calculate_plot_area(canvas_width: u32, canvas_height: u32, margin_fraction: f32) -> Rect {
    let margin_x = (canvas_width as f32) * margin_fraction;
    let margin_y = (canvas_height as f32) * margin_fraction;

    Rect::from_xywh(
        margin_x,
        margin_y,
        (canvas_width as f32) - 2.0 * margin_x,
        (canvas_height as f32) - 2.0 * margin_y,
    )
    .unwrap_or_else(|| {
        Rect::from_xywh(
            10.0,
            10.0,
            (canvas_width as f32) - 20.0,
            (canvas_height as f32) - 20.0,
        )
        .unwrap()
    })
}

/// Calculate plot area with DPI-aware margins for text space
pub fn calculate_plot_area_dpi(canvas_width: u32, canvas_height: u32, dpi_scale: f32) -> Rect {
    let render_scale = RenderScale::from_reference_scale(dpi_scale);
    // Base margins in pixels (at 96 DPI) - asymmetric to account for labels
    let base_margin_left = 100.0; // Space for Y-axis label and tick labels (more space needed)
    let base_margin_right = 40.0; // Less space needed on right side
    let base_margin_top = 80.0; // Space for title (more space needed)
    let base_margin_bottom = 60.0; // Space for X-axis label

    // Scale margins with DPI
    let margin_left = render_scale.logical_pixels_to_pixels(base_margin_left);
    let margin_right = render_scale.logical_pixels_to_pixels(base_margin_right);
    let margin_top = render_scale.logical_pixels_to_pixels(base_margin_top);
    let margin_bottom = render_scale.logical_pixels_to_pixels(base_margin_bottom);

    let plot_width = (canvas_width as f32) - margin_left - margin_right;
    let plot_height = (canvas_height as f32) - margin_top - margin_bottom;

    // Ensure minimum plot area
    if plot_width > 100.0 && plot_height > 100.0 {
        // Center the plot area within the available space after accounting for labels
        let plot_x = margin_left;
        let plot_y = margin_top;

        Rect::from_xywh(plot_x, plot_y, plot_width, plot_height).unwrap_or_else(|| {
            Rect::from_xywh(
                40.0,
                40.0,
                (canvas_width as f32) - 80.0,
                (canvas_height as f32) - 80.0,
            )
            .unwrap()
        })
    } else {
        // Fallback for very small canvases
        let fallback_margin = (canvas_width.min(canvas_height) as f32) * 0.1;
        Rect::from_xywh(
            fallback_margin,
            fallback_margin,
            (canvas_width as f32) - 2.0 * fallback_margin,
            (canvas_height as f32) - 2.0 * fallback_margin,
        )
        .unwrap()
    }
}

/// Calculate plot area using config-based margins
///
/// This function uses pre-computed margins from `PlotConfig::compute_margins()`
/// which are already in inches and get converted to pixels using the provided DPI.
///
/// # Arguments
///
/// * `canvas_width` - Canvas width in pixels
/// * `canvas_height` - Canvas height in pixels
/// * `margins` - Computed margins from PlotConfig
/// * `dpi` - Output DPI for conversion
pub fn calculate_plot_area_config(
    canvas_width: u32,
    canvas_height: u32,
    margins: &ComputedMargins,
    dpi: f32,
) -> Rect {
    // Convert margins from inches to pixels
    let margin_left = margins.left_px(dpi);
    let margin_right = margins.right_px(dpi);
    let margin_top = margins.top_px(dpi);
    let margin_bottom = margins.bottom_px(dpi);

    let plot_width = (canvas_width as f32) - margin_left - margin_right;
    let plot_height = (canvas_height as f32) - margin_top - margin_bottom;

    // Ensure minimum plot area
    if plot_width > 50.0 && plot_height > 50.0 {
        let plot_x = margin_left;
        let plot_y = margin_top;

        Rect::from_xywh(plot_x, plot_y, plot_width, plot_height).unwrap_or_else(|| {
            // Fallback with minimal margins
            Rect::from_xywh(
                40.0,
                40.0,
                (canvas_width as f32) - 80.0,
                (canvas_height as f32) - 80.0,
            )
            .unwrap()
        })
    } else {
        // Fallback for very small canvases
        let fallback_margin = (canvas_width.min(canvas_height) as f32) * 0.1;
        Rect::from_xywh(
            fallback_margin,
            fallback_margin,
            (canvas_width as f32) - 2.0 * fallback_margin,
            (canvas_height as f32) - 2.0 * fallback_margin,
        )
        .unwrap()
    }
}

/// Helper function to map data coordinates to pixel coordinates
///
/// This function delegates to [`CoordinateTransform`] for the actual transformation,
/// providing a unified coordinate mapping implementation across the codebase.
pub fn map_data_to_pixels(
    data_x: f64,
    data_y: f64,
    data_x_min: f64,
    data_x_max: f64,
    data_y_min: f64,
    data_y_max: f64,
    plot_area: Rect,
) -> (f32, f32) {
    // Note: tiny_skia Rect uses top() for minimum y, bottom() for maximum y
    // CoordinateTransform expects screen_y as top..bottom (both increasing downward)
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        data_x_min,
        data_x_max,
        data_y_min,
        data_y_max,
    );
    transform.data_to_screen(data_x, data_y)
}

/// Map data coordinates to pixel coordinates with axis scale transformations
///
/// The scale-aware coordinate transformation is delegated to
/// [`CoordinateTransform`].
pub fn map_data_to_pixels_scaled(
    data_x: f64,
    data_y: f64,
    data_x_min: f64,
    data_x_max: f64,
    data_y_min: f64,
    data_y_max: f64,
    plot_area: Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32) {
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        data_x_min,
        data_x_max,
        data_y_min,
        data_y_max,
    );
    transform.data_to_screen_scaled(data_x, data_y, x_scale, y_scale)
}

/// Map data coordinates to pixel coordinates, rejecting samples the axis scales
/// cannot represent.
///
/// Returns `None` for a non-finite sample on any scale, or a zero/negative
/// sample on a log scale — see
/// [`CoordinateTransform::try_data_to_screen_scaled`]. Callers that draw a
/// *shape* out of several samples (a bar, a whisker, an arrow) must drop the
/// whole shape when any vertex is rejected, and callers drawing a polyline must
/// break the line rather than joining across the gap.
#[allow(clippy::too_many_arguments)]
pub fn try_map_data_to_pixels_scaled(
    data_x: f64,
    data_y: f64,
    data_x_min: f64,
    data_x_max: f64,
    data_y_min: f64,
    data_y_max: f64,
    plot_area: Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> Option<(f32, f32)> {
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        data_x_min,
        data_x_max,
        data_y_min,
        data_y_max,
    );
    transform.try_data_to_screen_scaled(data_x, data_y, x_scale, y_scale)
}

// Tick generation lives in `crate::axes::ticks`. These re-exports keep the
// `render::skia` paths working while guaranteeing the raster backend cannot
// generate a different tick set from the layout/SVG backends.
pub use crate::axes::{generate_minor_ticks, generate_ticks};

fn generate_log_colorbar_major_ticks(min: f64, max: f64) -> Vec<f64> {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    if min <= 0.0 || max <= 0.0 {
        return vec![min.max(f64::EPSILON), max.max(f64::EPSILON)];
    }

    let start_exp = min.log10().ceil() as i32;
    let end_exp = max.log10().floor() as i32;
    let mut ticks = Vec::new();

    for exp in start_exp..=end_exp {
        let tick = 10.0_f64.powi(exp);
        if tick >= min && tick <= max {
            ticks.push(tick);
        }
    }

    if ticks.is_empty() {
        crate::axes::generate_log_ticks(min, max, 6)
    } else {
        ticks
    }
}

fn generate_log_colorbar_minor_ticks(min: f64, max: f64) -> Vec<f64> {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    if min <= 0.0 || max <= 0.0 {
        return Vec::new();
    }

    let start_exp = min.log10().floor() as i32;
    let end_exp = max.log10().ceil() as i32;
    let mut ticks = Vec::new();

    for exp in start_exp..=end_exp {
        let base = 10.0_f64.powi(exp);
        for multiplier in 2..=9 {
            let tick = base * multiplier as f64;
            if tick > min && tick < max {
                ticks.push(tick);
            }
        }
    }

    ticks.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ticks
}

pub(crate) fn compute_colorbar_layout_metrics(
    colorbar_width: f32,
    tick_font_size: f32,
    max_tick_label_width: f32,
    rotated_label_width: Option<f32>,
) -> ColorbarLayoutMetrics {
    let major_tick_width = colorbar_width * 0.3;
    let minor_tick_width = colorbar_width * 0.18;
    let tick_label_x_offset = colorbar_width + tick_font_size * 0.5;
    let label_gap = tick_font_size.max(4.0) * 0.75;
    let tick_label_extent = tick_label_x_offset + max_tick_label_width;
    let rotated_label_center_x_offset = rotated_label_width
        .map(|width| tick_label_x_offset + max_tick_label_width + label_gap + width / 2.0);
    let rotated_label_extent = rotated_label_width
        .map(|width| tick_label_x_offset + max_tick_label_width + label_gap + width)
        .unwrap_or(0.0);
    let total_extent =
        (colorbar_width + major_tick_width).max(tick_label_extent.max(rotated_label_extent));

    ColorbarLayoutMetrics {
        major_tick_width,
        minor_tick_width,
        tick_label_x_offset,
        rotated_label_center_x_offset,
        total_extent,
    }
}

pub(crate) fn colorbar_major_label_top(tick_center_y: f32, label_center_from_top: f32) -> f32 {
    tick_center_y - label_center_from_top
}

fn is_superscript_digit(ch: char) -> bool {
    matches!(
        ch,
        '⁰' | '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '⁻'
    )
}

pub(crate) fn colorbar_major_label_anchor_center_from_top(
    scale: &crate::axes::AxisScale,
    label: &str,
    rendered_center_from_top: f32,
    log_decade_base_center_from_top: Option<f32>,
) -> f32 {
    match scale {
        crate::axes::AxisScale::Log
            if label.starts_with("10")
                && label.chars().skip(2).all(is_superscript_digit)
                && label.chars().count() > 2 =>
        {
            log_decade_base_center_from_top.unwrap_or(rendered_center_from_top)
        }
        _ => rendered_center_from_top,
    }
}

// Tick label formatting lives in `crate::axes::ticks`. Re-exported so the
// raster backend, the SVG backend and the layout measurement pass cannot
// render the same tick value three different ways.
pub use crate::axes::{
    format_log_tick_label, format_tick_label, format_tick_labels, format_tick_labels_for_scale,
};

pub fn compute_colorbar_ticks(
    vmin: f64,
    vmax: f64,
    scale: &crate::axes::AxisScale,
    show_log_subticks: bool,
) -> ColorbarTicks {
    match scale {
        crate::axes::AxisScale::Log => {
            let major_values = generate_log_colorbar_major_ticks(vmin, vmax);
            let major_labels = format_tick_labels_for_scale(&major_values, scale);
            let minor_values = if show_log_subticks {
                generate_log_colorbar_minor_ticks(vmin, vmax)
            } else {
                Vec::new()
            };

            ColorbarTicks {
                major_values,
                major_labels,
                minor_values,
            }
        }
        _ => {
            let major_values = crate::axes::generate_ticks_for_scale(vmin, vmax, 6, scale);
            let major_labels = format_tick_labels_for_scale(&major_values, scale);

            ColorbarTicks {
                major_values,
                major_labels,
                minor_values: Vec::new(),
            }
        }
    }
}
