//! The one place an error bar becomes pixels, and the one place it is drawn.
//!
//! Error bars used to be projected and stroked separately by the raster
//! backend ([`Plot::render_attached_error_bars`]) and by the SVG backend
//! ([`Plot::render_error_bars_series_svg`]), and the SVG backend never drew the
//! bars attached with `with_yerr`/`with_xerr` at all — so the same figure
//! carried whiskers in PNG and none in SVG, while the axis bounds reserved room
//! for them in both. Both defects were possible only because the geometry and
//! the stroking were written twice.
//!
//! Here they are written once: [`ErrorBarPixels`] answers "where is this error
//! bar?" and [`draw_error_bars`] answers "how is it stroked?", over the
//! [`ErrorBarCanvas`] abstraction that both backends implement. A backend that
//! forgets to draw error bars now has to forget to call one function.

use crate::core::Result;
use crate::core::plot::ErrorValuesRef;
use crate::render::{Color, LineStyle};

/// One whisker's pixel extent along a single axis.
///
/// The two ends are named for the *data* side they came from, not the pixel
/// direction, because the y axis runs the other way in pixels and naming them
/// `top`/`bottom` is exactly how a sign error hides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Whisker {
    /// Pixel coordinate of the data-lower end, clamped into the plot area.
    pub lower: f32,
    /// Pixel coordinate of the data-upper end, clamped into the plot area.
    pub upper: f32,
    /// Whether the data-lower end carries a cap.
    ///
    /// False when that end is off the visible axis, or when the axis cannot
    /// represent it at all — a cap there would claim the whisker stops at the
    /// frame edge, which is a different (and wrong) reading.
    pub lower_cap: bool,
    /// Whether the data-upper end carries a cap. See [`Self::lower_cap`].
    pub upper_cap: bool,
}

impl Whisker {
    /// Resolve one whisker from its two projected ends.
    ///
    /// `raw_lower` / `raw_upper` are `None` when the axis cannot represent that
    /// end — a non-positive value on a log axis is the reachable case. Such an
    /// end lies beyond the axis *minimum* (log maps it to negative infinity),
    /// so the stem runs off the `px_at_data_min` edge and grows no cap there.
    ///
    /// Treating an unrepresentable end as `None` rather than letting a NaN
    /// pixel through is the whole point: `NaN.max(a).min(b)` silently yields
    /// `a`, which is how a whisker whose bottom fell off a log axis came out
    /// pinned to the *top* of the frame and drew a stem across the entire plot.
    fn resolve(
        raw_lower: Option<f32>,
        raw_upper: Option<f32>,
        px_at_data_min: f32,
        px_at_data_max: f32,
    ) -> Option<Self> {
        let (clip_lo, clip_hi) = if px_at_data_min <= px_at_data_max {
            (px_at_data_min, px_at_data_max)
        } else {
            (px_at_data_max, px_at_data_min)
        };
        if !clip_lo.is_finite() || !clip_hi.is_finite() {
            return None;
        }
        // An end the axis cannot represent falls off the data-minimum edge.
        let visible = |raw: Option<f32>| raw.is_some_and(|px| px >= clip_lo && px <= clip_hi);
        let place = |raw: Option<f32>| raw.unwrap_or(px_at_data_min).clamp(clip_lo, clip_hi);

        let lower = place(raw_lower);
        let upper = place(raw_upper);
        if (upper - lower).abs() <= 0.5 {
            // Nothing visible is left after clipping; drawing a sub-pixel stub
            // would only add a stray dot on the frame edge.
            return None;
        }
        Some(Self {
            lower,
            upper,
            lower_cap: visible(raw_lower),
            upper_cap: visible(raw_upper),
        })
    }
}

/// Every pixel one sample's error bars are drawn from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ErrorBarPixels {
    /// The sample itself, in pixels — where the marker goes.
    pub x: f32,
    /// See [`Self::x`].
    pub y: f32,
    /// The vertical whisker, in pixel-y, or `None` when there is nothing to draw.
    pub vertical: Option<Whisker>,
    /// The horizontal whisker, in pixel-x, or `None` when there is nothing to draw.
    pub horizontal: Option<Whisker>,
}

/// The axis window and plot rectangle an error bar is projected into.
///
/// Bundled because every projection here needs all seven values and threading
/// them individually is what pushed the old copies past the argument limit and
/// into positional mistakes.
#[derive(Clone, Copy)]
pub(super) struct ErrorBarFrame<'a> {
    pub plot_area: tiny_skia::Rect,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub x_scale: &'a crate::axes::AxisScale,
    pub y_scale: &'a crate::axes::AxisScale,
}

impl ErrorBarFrame<'_> {
    /// Project one data point, rejecting what the axis scales cannot represent.
    fn project(&self, x: f64, y: f64) -> Option<(f32, f32)> {
        crate::render::skia::try_map_data_to_pixels_scaled(
            x,
            y,
            self.x_min,
            self.x_max,
            self.y_min,
            self.y_max,
            self.plot_area,
            self.x_scale,
            self.y_scale,
        )
        .filter(|(px, py)| px.is_finite() && py.is_finite())
    }
}

impl ErrorBarPixels {
    /// Project one sample's error bars, or `None` if the sample itself is not
    /// on the axes.
    ///
    /// Rejecting the anchor is what stops an invalid sample — `y = 0` under
    /// `.yscale(Log)` — from contributing a marker and a pair of whiskers
    /// derived from a NaN pixel. The renderers already drop such a sample from
    /// the *line*; the error bars must agree, or the figure grows a whisker
    /// hanging off a point that is not drawn.
    pub(super) fn new(
        x_value: f64,
        y_value: f64,
        y_error: Option<(f64, f64)>,
        x_error: Option<(f64, f64)>,
        frame: ErrorBarFrame<'_>,
    ) -> Option<Self> {
        let (x, y) = frame.project(x_value, y_value)?;

        let plot_top = frame.plot_area.top();
        let plot_bottom = frame.plot_area.bottom();
        let plot_left = frame.plot_area.left();
        let plot_right = frame.plot_area.right();

        let vertical = y_error.and_then(|(lower, upper)| {
            Whisker::resolve(
                frame.project(x_value, y_value - lower).map(|(_, py)| py),
                frame.project(x_value, y_value + upper).map(|(_, py)| py),
                // Data-minimum is the *bottom* of the frame in pixel-y.
                plot_bottom,
                plot_top,
            )
        });
        let horizontal = x_error.and_then(|(lower, upper)| {
            Whisker::resolve(
                frame.project(x_value - lower, y_value).map(|(px, _)| px),
                frame.project(x_value + upper, y_value).map(|(px, _)| px),
                plot_left,
                plot_right,
            )
        });

        Some(Self {
            x,
            y,
            vertical,
            horizontal,
        })
    }
}

/// Normalise one sample's error entry into a non-negative, finite `(lower, upper)`
/// extent, or `None` when there is no bar to draw.
///
/// Both backends used to inline this `abs()`-and-finite dance twice each; a
/// missing, short, non-finite or all-zero entry means "no whisker here".
pub(super) fn error_extent_at(
    errors: Option<ErrorValuesRef<'_>>,
    index: usize,
) -> Option<(f64, f64)> {
    let (lower, upper) = errors?.bounds_at(index)?;
    let (lower, upper) = (lower.abs(), upper.abs());
    if !lower.is_finite() || !upper.is_finite() || (lower <= 0.0 && upper <= 0.0) {
        return None;
    }
    Some((lower, upper))
}

/// Project every sample of a series into error-bar pixels, dropping the ones
/// the axes cannot place.
///
/// A sample rejected here is rejected for its marker *and* its whiskers, which
/// is what keeps error bars consistent with the line, whose run splitter drops
/// the same samples.
pub(super) fn error_bar_pixels_for_series<'a>(
    x_data: &'a [f64],
    y_data: &'a [f64],
    y_errors: Option<ErrorValuesRef<'a>>,
    x_errors: Option<ErrorValuesRef<'a>>,
    frame: ErrorBarFrame<'a>,
) -> impl Iterator<Item = ErrorBarPixels> + 'a {
    x_data
        .iter()
        .zip(y_data)
        .enumerate()
        .filter_map(move |(index, (&x_value, &y_value))| {
            ErrorBarPixels::new(
                x_value,
                y_value,
                error_extent_at(y_errors, index),
                error_extent_at(x_errors, index),
                frame,
            )
        })
}

/// The stroke an error bar is drawn with, resolved from its config once.
///
/// Both backends resolved these four lines identically and independently; they
/// are here so a change to the cap size or the alpha compositing cannot land in
/// only one of them.
#[derive(Debug, Clone, Copy)]
pub(super) struct AttachedErrorBarStyle {
    pub color: Color,
    pub line_width: f32,
    pub half_cap: f32,
}

impl AttachedErrorBarStyle {
    pub(super) fn resolve(
        error_config: Option<&crate::plots::error::errorbar::ErrorBarConfig>,
        series_color: Color,
        default_line_width: f32,
        render_scale: crate::core::units::RenderScale,
    ) -> Self {
        let config = error_config.cloned().unwrap_or_default();
        let color = config.color.unwrap_or(series_color);
        let color = color.with_alpha((f32::from(color.a) / 255.0) * config.alpha);
        Self {
            color,
            // Error-bar configuration is still authored in legacy logical pixels.
            // Keep the bar slightly thinner than the data line it hangs off.
            line_width: render_scale
                .logical_pixels_to_pixels(config.line_width)
                .max(default_line_width * 0.75),
            half_cap: render_scale.logical_pixels_to_pixels(config.cap_size) * 0.5,
        }
    }
}

/// Draw every error bar of a series onto any backend.
pub(super) fn stroke_error_bar_series<C: ErrorBarCanvas>(
    canvas: &mut C,
    x_data: &[f64],
    y_data: &[f64],
    y_errors: Option<ErrorValuesRef<'_>>,
    x_errors: Option<ErrorValuesRef<'_>>,
    frame: ErrorBarFrame<'_>,
    style: AttachedErrorBarStyle,
) -> Result<()> {
    for bars in error_bar_pixels_for_series(x_data, y_data, y_errors, x_errors, frame) {
        draw_error_bars(
            canvas,
            &bars,
            frame.plot_area,
            style.color,
            style.line_width,
            style.half_cap,
        )?;
    }
    Ok(())
}

/// A surface `draw_error_bars` can stroke onto.
///
/// The raster and SVG line primitives already take the same arguments and
/// differ only in whether they can fail; this is the seam that lets one
/// routine feed both.
pub(super) trait ErrorBarCanvas {
    fn stroke(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
    ) -> Result<()>;
}

impl ErrorBarCanvas for crate::render::skia::SkiaRenderer {
    fn stroke(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
    ) -> Result<()> {
        self.draw_line(x1, y1, x2, y2, color, width, LineStyle::Solid)
    }
}

impl ErrorBarCanvas for crate::export::SvgRenderer {
    fn stroke(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
    ) -> Result<()> {
        self.draw_line(x1, y1, x2, y2, color, width, LineStyle::Solid);
        Ok(())
    }
}

/// Stroke resolved error-bar pixels onto any backend.
///
/// Caps are clamped to the plot rectangle so a whisker at the very edge of the
/// frame cannot paint over the spine, and a cap is only stroked where
/// [`Whisker`] says the end is real.
pub(super) fn draw_error_bars<C: ErrorBarCanvas>(
    canvas: &mut C,
    bars: &ErrorBarPixels,
    plot_area: tiny_skia::Rect,
    color: Color,
    line_width: f32,
    half_cap: f32,
) -> Result<()> {
    if let Some(whisker) = bars.vertical {
        canvas.stroke(
            bars.x,
            whisker.lower,
            bars.x,
            whisker.upper,
            color,
            line_width,
        )?;
        let left = (bars.x - half_cap).max(plot_area.left());
        let right = (bars.x + half_cap).min(plot_area.right());
        for (py, draw) in [
            (whisker.lower, whisker.lower_cap),
            (whisker.upper, whisker.upper_cap),
        ] {
            if draw {
                canvas.stroke(left, py, right, py, color, line_width)?;
            }
        }
    }

    if let Some(whisker) = bars.horizontal {
        canvas.stroke(
            whisker.lower,
            bars.y,
            whisker.upper,
            bars.y,
            color,
            line_width,
        )?;
        let top = (bars.y - half_cap).max(plot_area.top());
        let bottom = (bars.y + half_cap).min(plot_area.bottom());
        for (px, draw) in [
            (whisker.lower, whisker.lower_cap),
            (whisker.upper, whisker.upper_cap),
        ] {
            if draw {
                canvas.stroke(px, top, px, bottom, color, line_width)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisScale;

    fn rect() -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(100.0, 50.0, 400.0, 300.0).unwrap()
    }

    fn linear_frame() -> ErrorBarFrame<'static> {
        ErrorBarFrame {
            plot_area: rect(),
            x_min: 0.0,
            x_max: 10.0,
            y_min: 0.0,
            y_max: 100.0,
            x_scale: &AxisScale::Linear,
            y_scale: &AxisScale::Linear,
        }
    }

    fn log_y_frame() -> ErrorBarFrame<'static> {
        ErrorBarFrame {
            plot_area: rect(),
            x_min: 0.0,
            x_max: 10.0,
            y_min: 1.0,
            y_max: 200.0,
            x_scale: &AxisScale::Linear,
            y_scale: &AxisScale::Log,
        }
    }

    #[test]
    fn linear_whisker_spans_its_error_extent() {
        let bars = ErrorBarPixels::new(5.0, 50.0, Some((10.0, 10.0)), None, linear_frame())
            .expect("sample is on the axes");
        let whisker = bars.vertical.expect("y error present");
        // 300 px over 100 data units: +-10 units is +-30 px around the middle.
        assert!((bars.y - 200.0).abs() < 0.01, "anchor y {}", bars.y);
        assert!(
            (whisker.lower - 230.0).abs() < 0.01,
            "lower {}",
            whisker.lower
        );
        assert!(
            (whisker.upper - 170.0).abs() < 0.01,
            "upper {}",
            whisker.upper
        );
        assert!(whisker.lower_cap && whisker.upper_cap);
    }

    #[test]
    fn sample_the_log_axis_cannot_represent_is_dropped_whole() {
        // The regression this module exists to kill: y = 0 under a log axis
        // used to project to NaN, and NaN-poisoned clamping pinned the stem to
        // the top of the frame, drawing a whisker across the whole plot.
        assert!(
            ErrorBarPixels::new(2.0, 0.0, Some((2.0, 2.0)), None, log_y_frame()).is_none(),
            "a non-positive sample has no place on a log axis"
        );
        assert!(ErrorBarPixels::new(4.0, -5.0, Some((2.0, 2.0)), None, log_y_frame()).is_none());
    }

    #[test]
    fn whisker_running_off_a_log_axis_falls_to_the_floor_without_a_cap() {
        // y = 10 with a -20 error: the lower end is at -10, which a log axis
        // cannot place. It must go to the BOTTOM of the frame (the data
        // minimum), never the top, and it must not grow a cap there.
        let bars = ErrorBarPixels::new(5.0, 10.0, Some((20.0, 5.0)), None, log_y_frame())
            .expect("the sample itself is positive");
        let whisker = bars.vertical.expect("y error present");
        let area = rect();
        assert!(
            (whisker.lower - area.bottom()).abs() < 0.01,
            "unrepresentable lower end must clamp to the frame bottom, got {}",
            whisker.lower
        );
        assert!(
            !whisker.lower_cap,
            "no cap on an end that is not really there"
        );
        assert!(
            whisker.upper < whisker.lower,
            "upper end is higher on screen"
        );
        assert!(
            whisker.upper_cap,
            "the upper end is representable and visible"
        );
    }

    #[test]
    fn cap_is_suppressed_when_an_end_leaves_the_visible_frame() {
        // Anchor inside, upper end far above y_max: the stem clips to the top
        // spine but must not claim a cap sitting on it.
        let bars = ErrorBarPixels::new(5.0, 90.0, Some((5.0, 500.0)), None, linear_frame())
            .expect("sample is on the axes");
        let whisker = bars.vertical.expect("y error present");
        let area = rect();
        assert!((whisker.upper - area.top()).abs() < 0.01);
        assert!(!whisker.upper_cap);
        assert!(whisker.lower_cap);
    }

    #[test]
    fn horizontal_whisker_uses_the_x_axis_orientation() {
        let bars = ErrorBarPixels::new(5.0, 50.0, None, Some((1.0, 1.0)), linear_frame())
            .expect("sample is on the axes");
        let whisker = bars.horizontal.expect("x error present");
        // 400 px over 10 data units: +-1 unit is +-40 px around x = 300.
        assert!(
            (whisker.lower - 260.0).abs() < 0.01,
            "lower {}",
            whisker.lower
        );
        assert!(
            (whisker.upper - 340.0).abs() < 0.01,
            "upper {}",
            whisker.upper
        );
        assert!(
            whisker.lower < whisker.upper,
            "data-lower is LEFT in pixel-x, unlike y"
        );
    }

    #[test]
    fn degenerate_error_extents_draw_nothing() {
        assert_eq!(error_extent_at(None, 0), None);
        assert_eq!(
            error_extent_at(Some(ErrorValuesRef::Symmetric(&[0.0])), 0),
            None
        );
        assert_eq!(
            error_extent_at(Some(ErrorValuesRef::Symmetric(&[f64::NAN])), 0),
            None
        );
        assert_eq!(
            error_extent_at(Some(ErrorValuesRef::Symmetric(&[1.0])), 5),
            None
        );
        // Negative magnitudes are read as extents, matching matplotlib.
        assert_eq!(
            error_extent_at(Some(ErrorValuesRef::Symmetric(&[-3.0])), 0),
            Some((3.0, 3.0))
        );
    }

    #[test]
    fn sub_pixel_whiskers_are_dropped_rather_than_stubbed() {
        // 0.001 data units is 0.003 px here: below the half-pixel floor.
        let bars = ErrorBarPixels::new(5.0, 50.0, Some((0.001, 0.001)), None, linear_frame())
            .expect("sample is on the axes");
        assert!(bars.vertical.is_none());
    }
}
