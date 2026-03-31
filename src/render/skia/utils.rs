use super::*;

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
/// This version applies logarithmic or symlog transformations to the data
/// before mapping to pixel coordinates. The base coordinate transformation
/// is delegated to [`CoordinateTransform`].
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
    use crate::axes::Scale;

    // Create scale objects for the data ranges
    let x_scale_obj = x_scale.create_scale(data_x_min, data_x_max);
    let y_scale_obj = y_scale.create_scale(data_y_min, data_y_max);

    // Transform data values to normalized [0, 1] space using the scales
    let normalized_x = x_scale_obj.transform(data_x);
    let normalized_y = y_scale_obj.transform(data_y);

    // Use CoordinateTransform with normalized [0, 1] data bounds
    // since scaling has already been applied
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        0.0, // normalized min
        1.0, // normalized max
        0.0, // normalized min
        1.0, // normalized max
    );
    transform.data_to_screen(normalized_x, normalized_y)
}

/// Generate intelligent ticks using matplotlib's MaxNLocator algorithm
/// Produces 5-7 major ticks with "nice" numbers for scientific plotting
pub fn generate_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![min, max];
    }

    // Clamp target_count to reasonable scientific range (5-7 ticks optimal)
    let max_ticks = target_count.clamp(3, 10);

    generate_scientific_ticks(min, max, max_ticks)
}

/// MaxNLocator algorithm implementation for scientific plotting
/// Based on matplotlib's tick generation with nice number selection
fn generate_scientific_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 {
        return vec![min];
    }

    // Calculate rough step size
    let rough_step = range / (max_ticks - 1) as f64;

    // Handle very small ranges
    if rough_step <= f64::EPSILON {
        return vec![min, max];
    }

    // Round to "nice" numbers using powers of 10
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let normalized_step = rough_step / magnitude;

    // Select nice step sizes: prefer 1, 2, 5, 10 sequence
    let nice_step = if normalized_step <= 1.0 {
        1.0
    } else if normalized_step <= 2.0 {
        2.0
    } else if normalized_step <= 5.0 {
        5.0
    } else {
        10.0
    };

    let step = nice_step * magnitude;

    // Find optimal start point that includes the data range
    let start = (min / step).floor() * step;
    let end = (max / step).ceil() * step;

    // Generate ticks with epsilon for floating point stability
    let mut ticks = Vec::new();
    let mut tick = start;
    let epsilon = step * 1e-10; // Very small epsilon for float comparison

    while tick <= end + epsilon {
        // Only include ticks within the actual data range
        if tick >= min - epsilon && tick <= max + epsilon {
            // Clean up floating point errors by rounding to appropriate precision
            let clean_tick = clean_tick_value(tick, step);
            ticks.push(clean_tick);
        }
        tick += step;

        // Safety check to prevent infinite loops
        if ticks.len() > max_ticks * 2 {
            break;
        }
    }

    // Ensure we have reasonable number of ticks (3-10)
    if ticks.len() < 3 {
        // Fall back to simple min/max/middle approach with cleaned values
        let range = max - min;
        let fallback_step = range / 2.0;
        let clean_min = clean_tick_value(min, fallback_step);
        let clean_max = clean_tick_value(max, fallback_step);
        let clean_middle = clean_tick_value((min + max) / 2.0, fallback_step);
        return vec![clean_min, clean_middle, clean_max];
    }

    // Limit to max_ticks to prevent overcrowding
    if ticks.len() > max_ticks {
        ticks.truncate(max_ticks);
    }

    ticks
}

/// Clean up floating point errors in tick values by rounding to appropriate precision
fn clean_tick_value(value: f64, step: f64) -> f64 {
    // Determine number of decimal places based on step size
    let decimals = if step >= 1.0 {
        0
    } else {
        (-step.log10().floor()) as i32 + 1
    };
    let mult = 10.0_f64.powi(decimals);
    (value * mult).round() / mult
}

/// Generate minor tick values between major ticks
pub fn generate_minor_ticks(major_ticks: &[f64], minor_count: usize) -> Vec<f64> {
    if major_ticks.len() < 2 || minor_count == 0 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();

    for i in 0..major_ticks.len() - 1 {
        let start = major_ticks[i];
        let end = major_ticks[i + 1];
        let step = (end - start) / (minor_count + 1) as f64;

        for j in 1..=minor_count {
            let minor_tick = start + step * j as f64;
            minor_ticks.push(minor_tick);
        }
    }

    minor_ticks
}

/// Format a tick value using the unified TickFormatter
///
/// This provides matplotlib-compatible tick label formatting:
/// - Integers display without decimals: "5" not "5.0"
/// - Minimal decimal precision: "3.14" not "3.140000"
/// - Scientific notation for very large/small values (|v| >= 10^4 or |v| <= 10^-4)
///
/// # Arguments
///
/// * `value` - The tick value to format
///
/// # Returns
///
/// A clean string representation of the tick value
pub fn format_tick_label(value: f64) -> String {
    // Use static formatter instance for consistency
    static FORMATTER: std::sync::LazyLock<TickFormatter> =
        std::sync::LazyLock::new(TickFormatter::default);
    FORMATTER.format_tick(value)
}

/// Format multiple tick values with consistent precision
///
/// All ticks will use the same number of decimal places,
/// determined by the tick that needs the most precision.
/// This ensures visual alignment of tick labels.
///
/// # Arguments
///
/// * `values` - The tick values to format
///
/// # Returns
///
/// Vector of formatted tick labels with consistent precision
pub fn format_tick_labels(values: &[f64]) -> Vec<String> {
    static FORMATTER: std::sync::LazyLock<TickFormatter> =
        std::sync::LazyLock::new(TickFormatter::default);
    FORMATTER.format_ticks(values)
}
