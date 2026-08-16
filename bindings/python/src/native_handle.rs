use std::path::Path;
use std::sync::Arc;

use numpy::{PyReadonlyArray1, PyReadonlyArray2};
use pyo3::{
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyAny, PyBytes, PyDict},
};
use ruviz::{
    axes::AxisScale,
    core::{
        Annotation, IntoPlot, LegendPosition, Plot, PlotBuilder, PreparedPlot, TextStyle,
        plot::{IntoPlotData, PlotData},
    },
    data::Observable,
    plots::PlotConfig,
    render::{Color, FontFamily, LineStyle, MarkerStyle, Theme},
};

#[cfg(feature = "native-interactive")]
use ruviz::interactive::show_interactive;

#[cfg(not(feature = "native-interactive"))]
use crate::NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE;

/// One named theme: the name Python passes and the constructor it selects.
type NamedTheme = (&'static str, fn() -> Theme);

/// The core themes the 2D binding exposes by name.
///
/// One table so the setter's validation and the builder's lookup can never
/// drift; `Plot3D` deliberately keeps only `light`/`dark`.
const THEMES: &[NamedTheme] = &[
    ("light", Theme::light),
    ("dark", Theme::dark),
    ("seaborn", Theme::seaborn),
    ("publication", Theme::publication),
    ("minimal", Theme::minimal),
    ("presentation", Theme::presentation),
];

fn lookup_theme(name: &str) -> Option<Theme> {
    THEMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, build)| build())
}

#[derive(Clone)]
enum NumericSourceState {
    Static(Arc<Vec<f64>>),
    Observable(Observable<Vec<f64>>),
}

impl NumericSourceState {
    fn len(&self) -> usize {
        match self {
            Self::Static(values) => values.len(),
            Self::Observable(values) => values.get().len(),
        }
    }

    fn to_plot_data(&self) -> PlotData {
        match self {
            Self::Static(values) => Arc::clone(values).into_plot_data(),
            Self::Observable(values) => values.clone().into_plot_data(),
        }
    }
}

/// Optional per-series styling forwarded from the Python `style` mapping.
///
/// Only the options the Python layer declares per plot kind ever reach here;
/// each field maps to one core `PlotBuilder` setter.
#[derive(Clone, Default)]
struct SeriesStyle {
    label: Option<String>,
    color: Option<Color>,
    alpha: Option<f32>,
    width: Option<f32>,
    line_style: Option<LineStyle>,
    marker: Option<MarkerStyle>,
    marker_size: Option<f32>,
    bins: Option<usize>,
    density: Option<bool>,
    bandwidth: Option<f64>,
    levels: Option<usize>,
}

const MARKER_STYLES: [(&str, MarkerStyle); 12] = [
    ("circle", MarkerStyle::Circle),
    ("square", MarkerStyle::Square),
    ("triangle", MarkerStyle::Triangle),
    ("triangle-down", MarkerStyle::TriangleDown),
    ("diamond", MarkerStyle::Diamond),
    ("plus", MarkerStyle::Plus),
    ("cross", MarkerStyle::Cross),
    ("star", MarkerStyle::Star),
    ("circle-open", MarkerStyle::CircleOpen),
    ("square-open", MarkerStyle::SquareOpen),
    ("triangle-open", MarkerStyle::TriangleOpen),
    ("diamond-open", MarkerStyle::DiamondOpen),
];

const LINE_STYLES: [(&str, LineStyle); 5] = [
    ("solid", LineStyle::Solid),
    ("dashed", LineStyle::Dashed),
    ("dotted", LineStyle::Dotted),
    ("dash-dot", LineStyle::DashDot),
    ("dash-dot-dot", LineStyle::DashDotDot),
];

const LEGEND_POSITIONS: [(&str, LegendPosition); 15] = [
    ("best", LegendPosition::Best),
    ("upper_right", LegendPosition::UpperRight),
    ("upper_left", LegendPosition::UpperLeft),
    ("lower_left", LegendPosition::LowerLeft),
    ("lower_right", LegendPosition::LowerRight),
    ("right", LegendPosition::Right),
    ("center_left", LegendPosition::CenterLeft),
    ("center_right", LegendPosition::CenterRight),
    ("lower_center", LegendPosition::LowerCenter),
    ("upper_center", LegendPosition::UpperCenter),
    ("center", LegendPosition::Center),
    ("outside_right", LegendPosition::OutsideRight),
    ("outside_left", LegendPosition::OutsideLeft),
    ("outside_upper", LegendPosition::OutsideUpper),
    ("outside_lower", LegendPosition::OutsideLower),
];

/// Look a name up in a `(name, value)` table, or report the accepted names.
fn lookup<T: Clone>(table: &[(&str, T)], kind: &str, name: &str) -> PyResult<T> {
    table
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| {
            let accepted: Vec<&str> = table.iter().map(|(candidate, _)| *candidate).collect();
            PyValueError::new_err(format!(
                "unsupported {kind} '{name}'; expected one of: {}",
                accepted.join(", ")
            ))
        })
}

fn parse_color(value: &str) -> PyResult<Color> {
    Color::named(value)
        .or_else(|| Color::hex(value))
        .ok_or_else(|| {
            let hint = Color::suggest_named(value)
                .map(|name| format!(" (did you mean '{name}'?)"))
                .unwrap_or_default();
            PyValueError::new_err(format!(
                "unsupported color '{value}'; expected a hex string like '#2563eb' \
                 or a named color such as red, green, blue, orange, purple, black, white, gray{hint}"
            ))
        })
}

/// Validate 2D axis limits. Inverted bounds are passed through: the core keeps
/// them and renders a descending axis, matching matplotlib and the web surface.
fn distinct_limits(axis: &str, min: f64, max: f64) -> PyResult<(f64, f64)> {
    if !min.is_finite() || !max.is_finite() || min == max {
        return Err(PyValueError::new_err(format!(
            "{axis} limits must be finite and different"
        )));
    }
    Ok((min, max))
}

/// Validate 3D axis limits, which the 3D camera requires to be ascending.
pub(crate) fn ascending_limits(axis: &str, min: f64, max: f64) -> PyResult<(f64, f64)> {
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(PyValueError::new_err(format!(
            "{axis} limits must be finite and strictly ascending"
        )));
    }
    Ok((min, max))
}

fn parse_axis_scale(scale: &str, linthresh: Option<f64>) -> PyResult<AxisScale> {
    match scale {
        "linear" => Ok(AxisScale::Linear),
        "log" => Ok(AxisScale::Log),
        "symlog" => {
            let linthresh = linthresh.unwrap_or(1.0);
            if !linthresh.is_finite() || linthresh <= 0.0 {
                return Err(PyValueError::new_err(
                    "symlog linthresh must be a finite positive number",
                ));
            }
            Ok(AxisScale::SymLog { linthresh })
        }
        other => Err(PyValueError::new_err(format!(
            "unsupported axis scale '{other}'; expected one of: linear, log, symlog"
        ))),
    }
}

/// Validate a strictly positive finite style number, matching the Python-side
/// validator messages so `_native` callers see identical errors.
fn finite_positive_f64(value: &Bound<'_, PyAny>, name: &str) -> PyResult<f64> {
    let number: f64 = value.extract()?;
    if !number.is_finite() || number <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "{name} must be a finite positive number"
        )));
    }
    Ok(number)
}

/// [`finite_positive_f64`] bounded to f32: for every value the callers cast
/// down, where anything above f32::MAX would saturate to +infinity.
fn finite_positive(value: &Bound<'_, PyAny>, name: &str) -> PyResult<f64> {
    let number = finite_positive_f64(value, name)?;
    if number > f64::from(f32::MAX) {
        return Err(PyValueError::new_err(format!(
            "{name} must be a finite positive number"
        )));
    }
    Ok(number)
}

fn count_at_least(value: &Bound<'_, PyAny>, name: &str, minimum: i64) -> PyResult<usize> {
    let count: i64 = value.extract()?;
    if count < minimum {
        return Err(PyValueError::new_err(format!(
            "{name} must be an integer >= {minimum}"
        )));
    }
    Ok(count as usize)
}

/// Validate a boolean style flag; `1` and `"yes"` are caller mistakes, not flags.
fn flag(value: &Bound<'_, PyAny>, name: &str) -> PyResult<bool> {
    value
        .extract::<bool>()
        .map_err(|_| PyTypeError::new_err(format!("{name} must be a bool")))
}

/// Every style key any plot kind understands, in snapshot spelling.
const STYLE_KEYS: [&str; 11] = [
    "label",
    "color",
    "alpha",
    "width",
    "linestyle",
    "marker",
    "markerSize",
    "bins",
    "density",
    "bandwidth",
    "levels",
];

/// The style keys each plot kind's core builder honors, mirroring the Python
/// `_SERIES_KINDS[kind].style` sets so both layers reject the same combinations.
mod style_keys {
    pub(super) const COMMON: &[&str] = &["label", "color", "alpha"];
    pub(super) const STROKED: &[&str] = &["label", "color", "alpha", "width"];
    pub(super) const LINE: &[&str] = &[
        "label",
        "color",
        "alpha",
        "width",
        "linestyle",
        "marker",
        "markerSize",
    ];
    pub(super) const SCATTER: &[&str] =
        &["label", "color", "alpha", "marker", "markerSize", "density"];
    pub(super) const HISTOGRAM: &[&str] = &["label", "color", "alpha", "bins", "density"];
    pub(super) const BOXPLOT: &[&str] = &["label", "color", "alpha", "width", "linestyle"];
    pub(super) const KDE: &[&str] = &["label", "color", "alpha", "width", "bandwidth"];
    pub(super) const CONTOUR: &[&str] = &["alpha", "width", "levels"];
    pub(super) const NONE: &[&str] = &[];
}

/// The Python keyword spelling of a snapshot style key, for error messages.
fn style_keyword(key: &str) -> &str {
    if key == "markerSize" {
        "marker_size"
    } else {
        key
    }
}

/// Report a style key a different plot kind supports, matching the Python message.
fn unsupported_for_kind(kind: &str, key: &str, allowed: &[&str]) -> PyErr {
    let mut accepted: Vec<&str> = allowed.iter().copied().map(style_keyword).collect();
    accepted.sort_unstable();
    let accepted = if accepted.is_empty() {
        "none".to_string()
    } else {
        accepted.join(", ")
    };
    PyValueError::new_err(format!(
        "{kind} does not support {}=; accepted: {accepted}",
        style_keyword(key)
    ))
}

fn extract_style(
    style: Option<&Bound<'_, PyDict>>,
    kind: &str,
    allowed: &[&str],
) -> PyResult<SeriesStyle> {
    let mut parsed = SeriesStyle::default();
    let Some(style) = style else {
        return Ok(parsed);
    };

    for (key, value) in style.iter() {
        let key: String = key.extract()?;
        if !allowed.contains(&key.as_str()) {
            // Producer-side validator: a key this kind ignores is a caller
            // mistake, whether another kind uses it or nothing does.
            return Err(if STYLE_KEYS.contains(&key.as_str()) {
                unsupported_for_kind(kind, &key, allowed)
            } else {
                PyValueError::new_err(format!("unsupported style option: {key}"))
            });
        }
        match key.as_str() {
            "label" => parsed.label = Some(value.extract()?),
            "color" => parsed.color = Some(parse_color(&value.extract::<String>()?)?),
            "alpha" => {
                let alpha: f64 = value.extract()?;
                if !(0.0..=1.0).contains(&alpha) {
                    return Err(PyValueError::new_err("alpha must be between 0.0 and 1.0"));
                }
                parsed.alpha = Some(alpha as f32);
            }
            "width" => parsed.width = Some(finite_positive(&value, "width")? as f32),
            "linestyle" => {
                parsed.line_style = Some(lookup(
                    &LINE_STYLES,
                    "linestyle",
                    &value.extract::<String>()?,
                )?)
            }
            "marker" => {
                parsed.marker = Some(lookup(
                    &MARKER_STYLES,
                    "marker",
                    &value.extract::<String>()?,
                )?)
            }
            "markerSize" => {
                parsed.marker_size = Some(finite_positive(&value, "marker_size")? as f32)
            }
            "bins" => parsed.bins = Some(count_at_least(&value, "bins", 1)?),
            "density" => parsed.density = Some(flag(&value, "density")?),
            "bandwidth" => {
                // Bandwidth stays f64 end to end, so it takes the unbounded
                // validator: the f32 cap guards only values cast down.
                parsed.bandwidth = Some(finite_positive_f64(&value, "bandwidth")?);
            }
            "levels" => parsed.levels = Some(count_at_least(&value, "levels", 2)?),
            // Unreachable: `allowed` is a subset of `STYLE_KEYS`, checked above.
            other => {
                return Err(PyValueError::new_err(format!(
                    "unsupported style option: {other}"
                )));
            }
        }
    }

    Ok(parsed)
}

/// Apply the styling every core `PlotBuilder<C>` shares.
fn styled<C: PlotConfig>(mut builder: PlotBuilder<C>, style: &SeriesStyle) -> PlotBuilder<C> {
    if let Some(label) = &style.label {
        builder = builder.label(label.clone());
    }
    if let Some(color) = style.color {
        builder = builder.color(color);
    }
    if let Some(alpha) = style.alpha {
        builder = builder.alpha(alpha);
    }
    if let Some(width) = style.width {
        builder = builder.line_width(width);
    }
    if let Some(line_style) = &style.line_style {
        builder = builder.line_style(line_style.clone());
    }
    builder
}

/// One stored series: its data plus the style the Python call attached to it.
#[derive(Clone)]
struct NativeSeries {
    data: NativeSeriesState,
    style: SeriesStyle,
}

#[derive(Clone)]
enum NativeSeriesState {
    Line {
        x: NumericSourceState,
        y: NumericSourceState,
    },
    Scatter {
        x: NumericSourceState,
        y: NumericSourceState,
    },
    Bar {
        categories: Vec<String>,
        values: NumericSourceState,
    },
    Histogram {
        data: NumericSourceState,
    },
    Boxplot {
        data: NumericSourceState,
    },
    Heatmap {
        values: Vec<f64>,
        rows: usize,
        cols: usize,
    },
    ErrorBars {
        x: NumericSourceState,
        y: NumericSourceState,
        y_errors: NumericSourceState,
    },
    ErrorBarsXy {
        x: NumericSourceState,
        y: NumericSourceState,
        x_errors: NumericSourceState,
        y_errors: NumericSourceState,
    },
    Kde {
        data: Vec<f64>,
    },
    Ecdf {
        data: Vec<f64>,
    },
    Contour {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Pie {
        values: Vec<f64>,
        labels: Option<Vec<String>>,
    },
    Radar {
        labels: Vec<String>,
        series: Vec<(Option<String>, Vec<f64>)>,
    },
    Violin {
        data: Vec<f64>,
    },
    PolarLine {
        r: Vec<f64>,
        theta: Vec<f64>,
    },
}

#[derive(Clone, Default)]
struct NativePlotState {
    size_px: Option<(u32, u32)>,
    size_in: Option<(f32, f32)>,
    dpi: Option<u32>,
    max_resolution: Option<(u32, u32)>,
    font_size: Option<f32>,
    title_size: Option<f32>,
    font_family: Option<String>,
    scale_typography: Option<f32>,
    line_width_pt: Option<f32>,
    margin: Option<f32>,
    tight_layout_pad: Option<f32>,
    scientific_notation: Option<bool>,
    fast: bool,
    theme: Option<String>,
    ticks: Option<bool>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    legend: Option<LegendPosition>,
    grid: Option<bool>,
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
    x_scale: Option<AxisScale>,
    y_scale: Option<AxisScale>,
    annotations: Vec<NativeAnnotation>,
    series: Vec<NativeSeries>,
}

/// One plot-level annotation, stored in call order.
///
/// `style: None` on a line means "use the core's un-styled constructor" — the
/// 1pt dashed gray default — so an unstyled call and a styled one replay
/// identically to how they were made.
#[derive(Clone)]
enum NativeAnnotation {
    VLine {
        x: f64,
        style: Option<(Color, f32, LineStyle)>,
    },
    HLine {
        y: f64,
        style: Option<(Color, f32, LineStyle)>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        color: Option<Color>,
        font_size: Option<f32>,
    },
}

impl NativePlotState {
    fn build_plot(&self) -> Result<Plot, String> {
        let mut plot = Plot::new();

        // Figure geometry first: DPI, max-resolution and tight layout are all
        // measured against it. `size_in` wins over `size_px` — a caller who
        // asked for inches is targeting a physical output.
        if let Some((width, height)) = self.size_px {
            plot = plot.size_px(width, height);
        }

        if let Some((width_in, height_in)) = self.size_in {
            plot = plot.size(width_in, height_in);
        }

        // After the figure size: raising the DPI then scales the exported
        // pixels instead of reshaping the figure.
        if let Some(dpi) = self.dpi {
            plot = plot.dpi(dpi);
        }

        if let Some((max_width, max_height)) = self.max_resolution {
            plot = plot.max_resolution(max_width, max_height);
        }

        if let Some(enabled) = self.scientific_notation {
            plot = plot.scientific_notation(enabled);
        }

        if self.fast {
            plot = plot.fast(true);
        }

        if let Some(margin) = self.margin {
            plot = plot.margin(margin);
        }

        if let Some(theme) = &self.theme {
            let resolved =
                lookup_theme(theme).ok_or_else(|| format!("unsupported theme: {theme}"))?;
            plot = plot.theme(resolved);
        }

        // Typography and line width come *after* the theme: `apply_theme`
        // assigns `config.typography` and `config.lines` wholesale, so setting
        // these first would let the theme silently discard an explicit request.
        if let Some(family) = &self.font_family {
            plot = plot.font_family(font_family_from_str(family));
        }

        if let Some(size) = self.font_size {
            plot = plot.font_size(size);
        }

        // After `font_size`: the title size is stored as a ratio of it.
        if let Some(size) = self.title_size {
            plot = plot.title_size(size);
        }

        if let Some(factor) = self.scale_typography {
            plot = plot.scale_typography(factor);
        }

        if let Some(width) = self.line_width_pt {
            plot = plot.line_width_pt(width);
        }

        if let Some(ticks) = self.ticks {
            plot = plot.ticks(ticks);
        }

        if let Some(title) = &self.title {
            plot = plot.title(title.clone());
        }

        if let Some(x_label) = &self.x_label {
            plot = plot.xlabel(x_label.clone());
        }

        if let Some(y_label) = &self.y_label {
            plot = plot.ylabel(y_label.clone());
        }

        if let Some(legend) = self.legend {
            plot = plot.legend(legend);
        }

        if let Some(grid) = self.grid {
            plot = plot.grid(grid);
        }

        if let Some((min, max)) = self.x_limits {
            plot = plot.xlim(min, max);
        }

        if let Some((min, max)) = self.y_limits {
            plot = plot.ylim(min, max);
        }

        if let Some(scale) = &self.x_scale {
            plot = plot.xscale(*scale);
        }

        if let Some(scale) = &self.y_scale {
            plot = plot.yscale(*scale);
        }

        // Last of the plot-level settings: tight layout measures the title and
        // axis labels it packs margins around, so they have to be set first.
        if let Some(pad) = self.tight_layout_pad {
            plot = plot.tight_layout_pad(pad);
        }

        // Annotations render on their own layer above the data, so their order
        // here only fixes their order relative to each other.
        for annotation in &self.annotations {
            plot = match annotation {
                NativeAnnotation::VLine { x, style } => match style {
                    Some((color, width, line_style)) => {
                        plot.vline_styled(*x, *color, *width, line_style.clone())
                    }
                    None => plot.vline(*x),
                },
                NativeAnnotation::HLine { y, style } => match style {
                    Some((color, width, line_style)) => {
                        plot.hline_styled(*y, *color, *width, line_style.clone())
                    }
                    None => plot.hline(*y),
                },
                NativeAnnotation::Text {
                    x,
                    y,
                    text,
                    color,
                    font_size,
                } => {
                    let mut style = TextStyle::default();
                    if let Some(color) = color {
                        style.color = *color;
                    }
                    if let Some(size) = font_size {
                        style.font_size = *size;
                    }
                    plot.annotate(Annotation::text_styled(*x, *y, text.clone(), style))
                }
            };
        }

        for series in &self.series {
            plot = apply_series(plot, &series.data, &series.style)?;
        }

        Ok(plot)
    }
}

/// Validate an annotation's data coordinate: NaN or infinity would place the
/// line or label nowhere while reporting success.
fn annotation_coordinate(value: f64, name: &str) -> PyResult<f64> {
    if !value.is_finite() {
        return Err(PyValueError::new_err(format!(
            "{name} must be a finite number"
        )));
    }
    Ok(value)
}

/// Parse a reference-line style dict into `(color, width, line_style)`.
///
/// `None` keeps the core's un-styled default (1pt dashed gray); a present dict
/// fills unset fields with those same defaults so a partial style like
/// `{"color": "red"}` keeps the default dash and width. Unknown keys are
/// skipped so a snapshot written by a newer build still replays.
fn extract_reference_line_style(
    style: Option<&Bound<'_, PyDict>>,
    kind: &str,
) -> PyResult<Option<(Color, f32, LineStyle)>> {
    let Some(style) = style else {
        return Ok(None);
    };

    let (mut color, mut width, mut line_style) = Annotation::reference_line_defaults();

    for (key, value) in style.iter() {
        let key: String = key.extract()?;
        if value.is_none() {
            continue;
        }
        match key.as_str() {
            "color" => color = parse_color(&value.extract::<String>()?)?,
            "width" => width = finite_positive(&value, &format!("{kind} width"))? as f32,
            "linestyle" => {
                line_style = lookup(&LINE_STYLES, "linestyle", &value.extract::<String>()?)?;
            }
            _ => {}
        }
    }

    Ok(Some((color, width, line_style)))
}

/// Parse a text-annotation style dict accepting `color` and `fontSize`.
fn extract_text_annotation_style(
    style: Option<&Bound<'_, PyDict>>,
) -> PyResult<(Option<Color>, Option<f32>)> {
    let Some(style) = style else {
        return Ok((None, None));
    };

    let mut color = None;
    let mut font_size = None;
    for (key, value) in style.iter() {
        let key: String = key.extract()?;
        if value.is_none() {
            continue;
        }
        match key.as_str() {
            "color" => color = Some(parse_color(&value.extract::<String>()?)?),
            "fontSize" => {
                font_size = Some(finite_positive(&value, "annotation fontSize")? as f32);
            }
            _ => {}
        }
    }

    Ok((color, font_size))
}

/// Reject values the core builders would silently clamp. Clamping suits Rust
/// call sites; from Python it would turn a typo into a subtly wrong figure.
fn finite_positive_f32(value: f32, field: &str) -> PyResult<f32> {
    if !value.is_finite() || value <= 0.0 {
        return Err(PyValueError::new_err(format!(
            "{field} must be a finite positive number"
        )));
    }
    Ok(value)
}

/// Map a CSS-style generic family name onto `FontFamily`, falling back to
/// treating the string as a specific registered family.
fn font_family_from_str(family: &str) -> FontFamily {
    // Trim the fallback too: `"Arial "` must match the registered family
    // `"Arial"`, not silently fall back to the default face.
    let family = family.trim();
    match family.to_ascii_lowercase().as_str() {
        "serif" => FontFamily::Serif,
        "sans-serif" | "sans_serif" | "sansserif" => FontFamily::SansSerif,
        "monospace" | "mono" => FontFamily::Monospace,
        "cursive" => FontFamily::Cursive,
        "fantasy" => FontFamily::Fantasy,
        _ => FontFamily::Name(family.to_string()),
    }
}

fn ensure_same_len(lengths: &[usize], message: &str) -> Result<(), String> {
    let Some((&first, rest)) = lengths.split_first() else {
        return Ok(());
    };

    if rest.iter().all(|len| *len == first) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn apply_series(
    plot: Plot,
    series: &NativeSeriesState,
    style: &SeriesStyle,
) -> Result<Plot, String> {
    match series {
        NativeSeriesState::Line { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "line x and y must have the same length",
            )?;
            let mut builder = plot.line_source(x.to_plot_data(), y.to_plot_data());
            // Lines draw markers only when one is chosen, so a bare size implies circles.
            if let Some(marker) = style
                .marker
                .or_else(|| style.marker_size.map(|_| MarkerStyle::Circle))
            {
                builder = builder.marker(marker);
            }
            if let Some(size) = style.marker_size {
                builder = builder.marker_size(size);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Scatter { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "scatter x and y must have the same length",
            )?;
            let mut builder = plot.scatter_source(x.to_plot_data(), y.to_plot_data());
            if let Some(marker) = style.marker {
                builder = builder.marker(marker);
            }
            if let Some(size) = style.marker_size {
                builder = builder.marker_size(size);
            }
            if let Some(density) = style.density {
                builder = builder.density(density);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Bar { categories, values } => {
            ensure_same_len(
                &[categories.len(), values.len()],
                "bar categories and values must have the same length",
            )?;
            let builder = plot.bar_source(categories, values.to_plot_data());
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Histogram { data } => {
            let mut builder = match data {
                NumericSourceState::Static(values) => plot.histogram(values.as_ref()),
                NumericSourceState::Observable(values) => plot.histogram_source(values.clone()),
            };
            if let Some(bins) = style.bins {
                builder = builder.bins(bins);
            }
            if let Some(density) = style.density {
                builder = builder.density(density);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Boxplot { data } => {
            let builder = plot.boxplot_source(data.to_plot_data());
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Heatmap { values, rows, cols } => {
            if *rows == 0 || *cols == 0 || values.len() != rows.saturating_mul(*cols) {
                return Err("heatmap values length must match rows * cols".to_string());
            }
            let matrix: Vec<Vec<f64>> = values.chunks(*cols).map(|chunk| chunk.to_vec()).collect();
            Ok(plot.heatmap(&matrix).into_plot())
        }
        NativeSeriesState::ErrorBars { x, y, y_errors } => {
            ensure_same_len(
                &[x.len(), y.len(), y_errors.len()],
                "error bar x, y, and y_errors must have the same length",
            )?;
            let builder =
                plot.error_bars_source(x.to_plot_data(), y.to_plot_data(), y_errors.to_plot_data());
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::ErrorBarsXy {
            x,
            y,
            x_errors,
            y_errors,
        } => {
            ensure_same_len(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "error bar x, y, x_errors, and y_errors must have the same length",
            )?;
            let builder = plot.error_bars_xy_source(
                x.to_plot_data(),
                y.to_plot_data(),
                x_errors.to_plot_data(),
                y_errors.to_plot_data(),
            );
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Kde { data } => {
            let mut builder = plot.kde(&data);
            if let Some(bandwidth) = style.bandwidth {
                builder = builder.bandwidth(bandwidth);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Ecdf { data } => Ok(styled(plot.ecdf(&data), style).into_plot()),
        NativeSeriesState::Contour { x, y, z } => {
            if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
                return Err("contour z must contain x.len() * y.len() values".to_string());
            }
            let mut builder = plot.contour(&x, &y, &z);
            if let Some(levels) = style.levels {
                builder = builder.levels(levels);
            }
            Ok(styled(builder, style).into_plot())
        }
        // Heatmap, pie, and radar declare no style options on the Python side.
        NativeSeriesState::Pie { values, labels } => {
            let builder = plot.pie(&values);
            if let Some(labels) = labels {
                ensure_same_len(
                    &[values.len(), labels.len()],
                    "pie values and labels must have the same length",
                )?;
                Ok(builder.labels(labels).into_plot())
            } else {
                Ok(builder.into_plot())
            }
        }
        NativeSeriesState::Radar { labels, series } => {
            if labels.is_empty() {
                return Err("radar labels must not be empty".to_string());
            }

            let mut builder = plot.radar(labels);
            for (name, values) in series {
                ensure_same_len(
                    &[labels.len(), values.len()],
                    "radar series values must match labels length",
                )?;
                builder = if let Some(name) = name {
                    builder.add_series(name, &values)
                } else {
                    builder.series(&values)
                };
            }
            Ok(builder.into_plot())
        }
        NativeSeriesState::Violin { data } => Ok(styled(plot.violin(&data), style).into_plot()),
        NativeSeriesState::PolarLine { r, theta } => {
            ensure_same_len(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;
            Ok(styled(plot.polar_line(&r, &theta), style).into_plot())
        }
    }
}

const SUPPORTED_SAVE_EXTENSIONS: &str = ".png, .svg, or .pdf";

/// Validate an export path extension against the supported writer set.
pub(crate) fn save_extension(path: &str) -> PyResult<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension {
        Some(extension) if matches!(extension.as_str(), "png" | "svg" | "pdf") => Ok(extension),
        Some(extension) => Err(PyValueError::new_err(format!(
            "unsupported save extension '.{extension}'; expected {SUPPORTED_SAVE_EXTENSIONS}"
        ))),
        None => Err(PyValueError::new_err(format!(
            "save path '{path}' has no extension; expected {SUPPORTED_SAVE_EXTENSIONS}"
        ))),
    }
}

/// Copy a numeric vector, taking a single `memcpy` from a float64 NumPy array
/// and falling back to element-wise sequence extraction for anything else.
pub(crate) fn extract_f64_vec(values: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
    if let Ok(array) = values.extract::<PyReadonlyArray1<'_, f64>>() {
        return Ok(match array.as_slice() {
            Ok(slice) => slice.to_vec(),
            Err(_) => array.as_array().iter().copied().collect(),
        });
    }

    values.extract()
}

/// Copy a numeric matrix row by row, fast-pathing a 2D float64 NumPy array.
pub(crate) fn extract_f64_rows(values: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
    if let Ok(array) = values.extract::<PyReadonlyArray2<'_, f64>>() {
        return Ok(array
            .as_array()
            .rows()
            .into_iter()
            .map(|row| row.to_vec())
            .collect());
    }

    values.extract()
}

fn extract_numeric_source(source: &Bound<'_, PyAny>) -> PyResult<NumericSourceState> {
    if let Ok(observable) = source.extract::<PyRef<'_, NativeObservable1D>>() {
        return Ok(NumericSourceState::Observable(observable.inner.clone()));
    }

    extract_f64_vec(source)
        .map(Arc::new)
        .map(NumericSourceState::Static)
        .map_err(|_| PyTypeError::new_err("expected a numeric list or NativeObservable1D source"))
}

#[pyclass(module = "ruviz._native")]
pub struct NativeObservable1D {
    inner: Observable<Vec<f64>>,
}

#[pymethods]
impl NativeObservable1D {
    #[new]
    fn new(values: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: Observable::new(extract_f64_vec(values)?),
        })
    }

    fn replace(&self, values: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.set(extract_f64_vec(values)?);
        Ok(())
    }

    fn set_at(&self, index: usize, value: f64) -> PyResult<()> {
        self.inner.update_with(|values| {
            if index >= values.len() {
                return Err(PyValueError::new_err("observable index is out of bounds"));
            }
            values[index] = value;
            Ok(())
        })
    }
}

#[pyclass(module = "ruviz._native")]
pub struct NativePlotHandle {
    state: NativePlotState,
    plot: Plot,
    prepared: PreparedPlot,
    dirty: bool,
}

impl NativePlotHandle {
    /// Rebuild the core plot when a mutator ran since the last render.
    ///
    /// `Plot` and `PreparedPlot` are `Send + Sync`, so the copy-heavy rebuild
    /// *and* the layout pass in `prepare` both run with the GIL released.
    fn ensure_built(&mut self, py: Python<'_>) -> PyResult<()> {
        if !self.dirty {
            return Ok(());
        }

        let state = &self.state;
        let (plot, prepared) = py
            .allow_threads(|| {
                let plot = state.build_plot()?;
                let prepared = plot.prepare();
                Ok::<_, String>((plot, prepared))
            })
            .map_err(PyValueError::new_err)?;
        self.prepared = prepared;
        self.plot = plot;
        self.dirty = false;
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Store one series with its parsed style, rejecting bad style values and
    /// keys this kind does not honor here, so they surface at the add-series
    /// call rather than being silently ignored at render time.
    fn push_series(
        &mut self,
        kind: &str,
        allowed: &[&str],
        data: NativeSeriesState,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let style = extract_style(style, kind, allowed)?;
        self.state.series.push(NativeSeries { data, style });
        self.mark_dirty();
        Ok(())
    }
}

#[pymethods]
impl NativePlotHandle {
    #[new]
    fn new() -> PyResult<Self> {
        let state = NativePlotState::default();
        let plot = state.build_plot().map_err(PyValueError::new_err)?;
        let prepared = plot.prepare();
        Ok(Self {
            state,
            plot,
            prepared,
            dirty: false,
        })
    }

    fn size_px(&mut self, width: u32, height: u32) -> PyResult<()> {
        if width == 0 || height == 0 {
            return Err(PyValueError::new_err(
                "plot dimensions must be integers greater than zero",
            ));
        }
        self.state.size_px = Some((width, height));
        self.mark_dirty();
        Ok(())
    }

    fn size(&mut self, width_in: f32, height_in: f32) -> PyResult<()> {
        let width_in = finite_positive_f32(width_in, "plot width")?;
        let height_in = finite_positive_f32(height_in, "plot height")?;
        self.state.size_in = Some((width_in, height_in));
        self.mark_dirty();
        Ok(())
    }

    fn dpi(&mut self, dpi: u32) -> PyResult<()> {
        if dpi == 0 {
            return Err(PyValueError::new_err(
                "plot dpi must be an integer greater than zero",
            ));
        }
        self.state.dpi = Some(dpi);
        self.mark_dirty();
        Ok(())
    }

    fn max_resolution(&mut self, max_width: u32, max_height: u32) -> PyResult<()> {
        if max_width == 0 || max_height == 0 {
            return Err(PyValueError::new_err(
                "plot max resolution bounds must be integers greater than zero",
            ));
        }
        self.state.max_resolution = Some((max_width, max_height));
        self.mark_dirty();
        Ok(())
    }

    fn font_size(&mut self, points: f32) -> PyResult<()> {
        self.state.font_size = Some(finite_positive_f32(points, "plot font size")?);
        self.mark_dirty();
        Ok(())
    }

    fn title_size(&mut self, points: f32) -> PyResult<()> {
        self.state.title_size = Some(finite_positive_f32(points, "plot title size")?);
        self.mark_dirty();
        Ok(())
    }

    fn font_family(&mut self, family: &str) -> PyResult<()> {
        if family.trim().is_empty() {
            return Err(PyValueError::new_err(
                "plot font family must be a non-empty string",
            ));
        }
        self.state.font_family = Some(family.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn scale_typography(&mut self, factor: f32) -> PyResult<()> {
        self.state.scale_typography = Some(finite_positive_f32(factor, "plot typography scale")?);
        self.mark_dirty();
        Ok(())
    }

    fn line_width_pt(&mut self, points: f32) -> PyResult<()> {
        self.state.line_width_pt = Some(finite_positive_f32(points, "plot line width")?);
        self.mark_dirty();
        Ok(())
    }

    fn margin(&mut self, fraction: f32) -> PyResult<()> {
        // The core builder clamps to 0.0..=0.5; reject out-of-range here
        // instead so 0.9 does not silently render as 0.5.
        if !fraction.is_finite() || !(0.0..=0.5).contains(&fraction) {
            return Err(PyValueError::new_err(
                "plot margin must be a fraction between 0.0 and 0.5",
            ));
        }
        self.state.margin = Some(fraction);
        self.mark_dirty();
        Ok(())
    }

    fn tight_layout_pad(&mut self, points: f32) -> PyResult<()> {
        if !points.is_finite() || points < 0.0 {
            return Err(PyValueError::new_err(
                "plot tight layout padding must be a finite, non-negative number of points",
            ));
        }
        self.state.tight_layout_pad = Some(points);
        self.mark_dirty();
        Ok(())
    }

    fn scientific_notation(&mut self, enabled: bool) -> PyResult<()> {
        self.state.scientific_notation = Some(enabled);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (x, style=None))]
    fn vline(&mut self, x: f64, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let x = annotation_coordinate(x, "vline x")?;
        let style = extract_reference_line_style(style, "vline")?;
        self.state
            .annotations
            .push(NativeAnnotation::VLine { x, style });
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (y, style=None))]
    fn hline(&mut self, y: f64, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let y = annotation_coordinate(y, "hline y")?;
        let style = extract_reference_line_style(style, "hline")?;
        self.state
            .annotations
            .push(NativeAnnotation::HLine { y, style });
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (x, y, text, style=None))]
    fn annotate_text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = annotation_coordinate(x, "annotation x")?;
        let y = annotation_coordinate(y, "annotation y")?;
        if text.is_empty() {
            return Err(PyValueError::new_err("annotation text must not be empty"));
        }
        let (color, font_size) = extract_text_annotation_style(style)?;
        self.state.annotations.push(NativeAnnotation::Text {
            x,
            y,
            text: text.to_string(),
            color,
            font_size,
        });
        self.mark_dirty();
        Ok(())
    }

    fn fast(&mut self, enabled: bool) -> PyResult<()> {
        self.state.fast = enabled;
        self.mark_dirty();
        Ok(())
    }

    fn theme(&mut self, theme: &str) -> PyResult<()> {
        if lookup_theme(theme).is_none() {
            return Err(PyValueError::new_err(format!("unsupported theme: {theme}")));
        }
        self.state.theme = Some(theme.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn ticks(&mut self, enabled: bool) -> PyResult<()> {
        self.state.ticks = Some(enabled);
        self.mark_dirty();
        Ok(())
    }

    fn title(&mut self, title: &str) -> PyResult<()> {
        self.state.title = Some(title.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn xlabel(&mut self, label: &str) -> PyResult<()> {
        self.state.x_label = Some(label.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn ylabel(&mut self, label: &str) -> PyResult<()> {
        self.state.y_label = Some(label.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn legend(&mut self, position: &str) -> PyResult<()> {
        self.state.legend = Some(lookup(&LEGEND_POSITIONS, "legend position", position)?);
        self.mark_dirty();
        Ok(())
    }

    fn grid(&mut self, enabled: bool) -> PyResult<()> {
        self.state.grid = Some(enabled);
        self.mark_dirty();
        Ok(())
    }

    fn xlim(&mut self, min: f64, max: f64) -> PyResult<()> {
        self.state.x_limits = Some(distinct_limits("x", min, max)?);
        self.mark_dirty();
        Ok(())
    }

    fn ylim(&mut self, min: f64, max: f64) -> PyResult<()> {
        self.state.y_limits = Some(distinct_limits("y", min, max)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (scale, linthresh=None))]
    fn xscale(&mut self, scale: &str, linthresh: Option<f64>) -> PyResult<()> {
        self.state.x_scale = Some(parse_axis_scale(scale, linthresh)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (scale, linthresh=None))]
    fn yscale(&mut self, scale: &str, linthresh: Option<f64>) -> PyResult<()> {
        self.state.y_scale = Some(parse_axis_scale(scale, linthresh)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (x, y, style=None))]
    fn line(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "line x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "line",
            style_keys::LINE,
            NativeSeriesState::Line { x, y },
            style,
        )
    }

    #[pyo3(signature = (x, y, style=None))]
    fn scatter(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "scatter x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "scatter",
            style_keys::SCATTER,
            NativeSeriesState::Scatter { x, y },
            style,
        )
    }

    #[pyo3(signature = (categories, values, style=None))]
    fn bar(
        &mut self,
        categories: Vec<String>,
        values: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let values = extract_numeric_source(values)?;
        ensure_same_len(
            &[categories.len(), values.len()],
            "bar categories and values must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "bar",
            style_keys::COMMON,
            NativeSeriesState::Bar { categories, values },
            style,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn histogram(
        &mut self,
        data: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.push_series(
            "histogram",
            style_keys::HISTOGRAM,
            NativeSeriesState::Histogram { data },
            style,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn boxplot(
        &mut self,
        data: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.push_series(
            "boxplot",
            style_keys::BOXPLOT,
            NativeSeriesState::Boxplot { data },
            style,
        )
    }

    fn heatmap(&mut self, values: &Bound<'_, PyAny>, rows: usize, cols: usize) -> PyResult<()> {
        let values = extract_f64_vec(values)?;
        if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
            return Err(PyValueError::new_err(
                "heatmap values length must match rows * cols",
            ));
        }
        self.push_series(
            "heatmap",
            style_keys::NONE,
            NativeSeriesState::Heatmap { values, rows, cols },
            None,
        )
    }

    #[pyo3(signature = (x, y, y_errors, style=None))]
    fn error_bars(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        let y_errors = extract_numeric_source(y_errors)?;
        ensure_same_len(
            &[x.len(), y.len(), y_errors.len()],
            "error bar x, y, and y_errors must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "error-bars",
            style_keys::STROKED,
            NativeSeriesState::ErrorBars { x, y, y_errors },
            style,
        )
    }

    #[pyo3(signature = (x, y, x_errors, y_errors, style=None))]
    fn error_bars_xy(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        x_errors: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        let x_errors = extract_numeric_source(x_errors)?;
        let y_errors = extract_numeric_source(y_errors)?;
        ensure_same_len(
            &[x.len(), y.len(), x_errors.len(), y_errors.len()],
            "error bar x, y, x_errors, and y_errors must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "error-bars-xy",
            style_keys::STROKED,
            NativeSeriesState::ErrorBarsXy {
                x,
                y,
                x_errors,
                y_errors,
            },
            style,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn kde(&mut self, data: &Bound<'_, PyAny>, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let data = extract_f64_vec(data)?;
        self.push_series(
            "kde",
            style_keys::KDE,
            NativeSeriesState::Kde { data },
            style,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn ecdf(&mut self, data: &Bound<'_, PyAny>, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let data = extract_f64_vec(data)?;
        self.push_series(
            "ecdf",
            style_keys::STROKED,
            NativeSeriesState::Ecdf { data },
            style,
        )
    }

    #[pyo3(signature = (x, y, z, style=None))]
    fn contour(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        z: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let (x, y, z) = (
            extract_f64_vec(x)?,
            extract_f64_vec(y)?,
            extract_f64_vec(z)?,
        );
        if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
            return Err(PyValueError::new_err(
                "contour z must contain x.length * y.length values",
            ));
        }
        self.push_series(
            "contour",
            style_keys::CONTOUR,
            NativeSeriesState::Contour { x, y, z },
            style,
        )
    }

    fn pie(&mut self, values: &Bound<'_, PyAny>, labels: Option<Vec<String>>) -> PyResult<()> {
        let values = extract_f64_vec(values)?;
        if let Some(labels) = &labels {
            ensure_same_len(
                &[values.len(), labels.len()],
                "pie values and labels must have the same length",
            )
            .map_err(PyValueError::new_err)?;
        }
        self.push_series(
            "pie",
            style_keys::NONE,
            NativeSeriesState::Pie { values, labels },
            None,
        )
    }

    fn radar(
        &mut self,
        labels: Vec<String>,
        series: Vec<(Option<String>, Vec<f64>)>,
    ) -> PyResult<()> {
        if labels.is_empty() {
            return Err(PyValueError::new_err("radar labels must not be empty"));
        }
        for (_, values) in &series {
            ensure_same_len(
                &[labels.len(), values.len()],
                "each radar series must match the labels length",
            )
            .map_err(PyValueError::new_err)?;
        }
        self.push_series(
            "radar",
            style_keys::NONE,
            NativeSeriesState::Radar { labels, series },
            None,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn violin(
        &mut self,
        data: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let data = extract_f64_vec(data)?;
        self.push_series(
            "violin",
            style_keys::STROKED,
            NativeSeriesState::Violin { data },
            style,
        )
    }

    #[pyo3(signature = (r, theta, style=None))]
    fn polar_line(
        &mut self,
        r: &Bound<'_, PyAny>,
        theta: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let (r, theta) = (extract_f64_vec(r)?, extract_f64_vec(theta)?);
        ensure_same_len(
            &[r.len(), theta.len()],
            "polar r and theta must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            "polar-line",
            style_keys::STROKED,
            NativeSeriesState::PolarLine { r, theta },
            style,
        )
    }

    fn render_png_bytes<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.ensure_built(py)?;
        let prepared = &self.prepared;
        let bytes = py
            .allow_threads(|| prepared.render_png_bytes())
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_png_bytes_uncached<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.ensure_built(py)?;
        let prepared = &self.prepared;
        let bytes = py
            .allow_threads(|| prepared.render_png_bytes_uncached())
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_svg(&mut self, py: Python<'_>) -> PyResult<String> {
        self.ensure_built(py)?;
        let plot = &self.plot;
        py.allow_threads(|| plot.render_to_svg())
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }

    fn save(&mut self, py: Python<'_>, path: &str) -> PyResult<()> {
        let extension = save_extension(path)?;
        self.ensure_built(py)?;
        let plot = &self.plot;
        let output = Path::new(path);

        py.allow_threads(|| match extension.as_str() {
            "svg" => plot.clone().export_svg(output),
            "pdf" => plot.clone().save_pdf(output),
            _ => plot.clone().save(output),
        })
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }

    fn show_native(&mut self, py: Python<'_>) -> PyResult<()> {
        #[cfg(not(feature = "native-interactive"))]
        {
            let _ = py;
            return Err(PyRuntimeError::new_err(
                NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE,
            ));
        }

        #[cfg(feature = "native-interactive")]
        self.ensure_built(py)?;
        #[cfg(feature = "native-interactive")]
        {
            let plot = &self.plot;
            py.allow_threads(|| pollster::block_on(show_interactive(plot.clone())))
                .map_err(|err| PyRuntimeError::new_err(err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use image::RgbaImage;

    fn decode_png(bytes: &[u8]) -> RgbaImage {
        image::load_from_memory(bytes)
            .expect("expected valid PNG bytes")
            .to_rgba8()
    }

    fn assert_pngs_equal(name: &str, actual: &[u8], expected: &[u8]) {
        let actual = decode_png(actual);
        let expected = decode_png(expected);

        assert_eq!(
            actual.dimensions(),
            expected.dimensions(),
            "{name}: PNG dimensions differ"
        );

        let diff_pixels = actual
            .pixels()
            .zip(expected.pixels())
            .filter(|(left, right)| left != right)
            .count();

        assert_eq!(
            diff_pixels, 0,
            "{name}: rendered PNG differs at {diff_pixels} pixel(s)"
        );
    }

    fn render_state_png(state: NativePlotState) -> Vec<u8> {
        state
            .build_plot()
            .expect("state should build")
            .render_png_bytes()
            .expect("state render should succeed")
    }

    fn render_direct_png(plot: Plot) -> Vec<u8> {
        plot.render_png_bytes()
            .expect("direct render should succeed")
    }

    fn plain(data: NativeSeriesState) -> NativeSeries {
        NativeSeries {
            data,
            style: SeriesStyle::default(),
        }
    }

    fn base_state(title: &str, series: Vec<NativeSeriesState>) -> NativePlotState {
        NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("light".to_string()),
            ticks: Some(true),
            title: Some(title.to_string()),
            x_label: Some("x".to_string()),
            y_label: Some("y".to_string()),
            series: series.into_iter().map(plain).collect(),
            ..NativePlotState::default()
        }
    }

    fn base_plot(title: &str) -> Plot {
        Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title(title)
            .xlabel("x")
            .ylabel("y")
    }

    fn static_source(values: &[f64]) -> NumericSourceState {
        NumericSourceState::Static(Arc::new(values.to_vec()))
    }

    #[test]
    fn state_render_matches_direct_plot_for_xy_series() {
        let state = base_state(
            "XY parity",
            vec![
                NativeSeriesState::Line {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.2, 1.1, 0.7, 1.8]),
                },
                NativeSeriesState::Scatter {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.3, 1.0, 0.9, 1.6]),
                },
            ],
        );

        let expected = base_plot("XY parity")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .scatter(&[0.0, 1.0, 2.0, 3.0], &[0.3, 1.0, 0.9, 1.6])
            .into_plot();

        assert_pngs_equal(
            "xy_series",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_statistical_series() {
        let samples = [
            -2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9,
        ];

        let mut state = base_state(
            "Histogram parity",
            vec![NativeSeriesState::Histogram {
                data: static_source(&samples),
            }],
        );
        state.x_label = Some("value".to_string());
        state.y_label = Some("count".to_string());

        let expected = base_plot("Histogram parity")
            .xlabel("value")
            .ylabel("count")
            .histogram(&samples)
            .into_plot();

        assert_pngs_equal(
            "histogram",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_matrix_and_errorbar_series() {
        let heatmap_matrix = vec![
            vec![0.1, 0.4, 0.8],
            vec![0.3, 0.5, 0.7],
            vec![0.2, 0.6, 0.9],
        ];

        let heatmap_state = NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("dark".to_string()),
            ticks: Some(false),
            title: Some("Heatmap parity".to_string()),
            series: vec![plain(NativeSeriesState::Heatmap {
                values: vec![0.1, 0.4, 0.8, 0.3, 0.5, 0.7, 0.2, 0.6, 0.9],
                rows: 3,
                cols: 3,
            })],
            ..NativePlotState::default()
        };

        let heatmap_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::dark())
            .ticks(false)
            .title("Heatmap parity")
            .heatmap(&heatmap_matrix)
            .into_plot();

        assert_pngs_equal(
            "heatmap",
            &render_state_png(heatmap_state),
            &render_direct_png(heatmap_expected),
        );

        let error_state = base_state(
            "Error parity",
            vec![NativeSeriesState::ErrorBarsXy {
                x: static_source(&[1.0, 2.0, 3.0]),
                y: static_source(&[1.2, 1.8, 1.4]),
                x_errors: static_source(&[0.1, 0.15, 0.12]),
                y_errors: static_source(&[0.2, 0.18, 0.16]),
            }],
        );

        let error_expected = base_plot("Error parity")
            .error_bars_xy(
                &[1.0, 2.0, 3.0],
                &[1.2, 1.8, 1.4],
                &[0.1, 0.15, 0.12],
                &[0.2, 0.18, 0.16],
            )
            .into_plot();

        assert_pngs_equal(
            "error_bars_xy",
            &render_state_png(error_state),
            &render_direct_png(error_expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_specialized_series() {
        let titled_state = |title: &str, series| NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("light".to_string()),
            ticks: Some(true),
            title: Some(title.to_string()),
            series: vec![plain(series)],
            ..NativePlotState::default()
        };
        let titled_plot = |title: &str| {
            Plot::new()
                .size_px(320, 200)
                .theme(Theme::light())
                .ticks(true)
                .title(title)
        };

        let contour_state = titled_state(
            "Contour parity",
            NativeSeriesState::Contour {
                x: vec![-1.0, 0.0, 1.0],
                y: vec![-1.0, 0.0, 1.0],
                z: vec![0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            },
        );
        let contour_expected = titled_plot("Contour parity")
            .contour(
                &[-1.0, 0.0, 1.0],
                &[-1.0, 0.0, 1.0],
                &[0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            )
            .into_plot();

        assert_pngs_equal(
            "contour",
            &render_state_png(contour_state),
            &render_direct_png(contour_expected),
        );

        let pie_state = titled_state(
            "Pie parity",
            NativeSeriesState::Pie {
                values: vec![30.0, 25.0, 20.0, 25.0],
                labels: Some(vec![
                    "A".to_string(),
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                ]),
            },
        );
        let pie_expected = titled_plot("Pie parity")
            .pie(&[30.0, 25.0, 20.0, 25.0])
            .labels(&["A", "B", "C", "D"])
            .into_plot();

        assert_pngs_equal(
            "pie",
            &render_state_png(pie_state),
            &render_direct_png(pie_expected),
        );

        let radar_state = titled_state(
            "Radar parity",
            NativeSeriesState::Radar {
                labels: ["API", "Docs", "Export", "Interactive", "Scale"]
                    .map(str::to_string)
                    .to_vec(),
                series: vec![
                    (Some("Python".to_string()), vec![4.5, 4.7, 4.8, 4.3, 4.0]),
                    (Some("Web".to_string()), vec![4.2, 4.1, 4.0, 4.8, 4.6]),
                ],
            },
        );
        let radar_expected = titled_plot("Radar parity")
            .radar(&["API", "Docs", "Export", "Interactive", "Scale"])
            .add_series("Python", &[4.5, 4.7, 4.8, 4.3, 4.0])
            .add_series("Web", &[4.2, 4.1, 4.0, 4.8, 4.6])
            .into_plot();

        assert_pngs_equal(
            "radar",
            &render_state_png(radar_state),
            &render_direct_png(radar_expected),
        );
    }

    fn styled_line_state(style: SeriesStyle) -> NativePlotState {
        NativePlotState {
            series: vec![NativeSeries {
                data: NativeSeriesState::Line {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.2, 1.1, 0.7, 1.8]),
                },
                style,
            }],
            ..base_state("Styled", vec![])
        }
    }

    #[test]
    fn styled_series_render_matches_direct_plot_and_differs_from_default() {
        let style = SeriesStyle {
            label: Some("Revenue".to_string()),
            color: Some(Color::hex("#2563eb").expect("hex color")),
            alpha: Some(0.6),
            width: Some(3.5),
            line_style: Some(LineStyle::Dashed),
            marker: Some(MarkerStyle::Square),
            marker_size: Some(9.0),
            ..SeriesStyle::default()
        };

        let expected = base_plot("Styled")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .marker(MarkerStyle::Square)
            .marker_size(9.0)
            .label("Revenue")
            .color(Color::hex("#2563eb").expect("hex color"))
            .alpha(0.6)
            .line_width(3.5)
            .line_style(LineStyle::Dashed)
            .into_plot();

        let styled_png = render_state_png(styled_line_state(style));
        assert_pngs_equal("styled_line", &styled_png, &render_direct_png(expected));
        assert_ne!(
            styled_png,
            render_state_png(styled_line_state(SeriesStyle::default())),
            "styled render should differ from the unstyled default"
        );
    }

    #[test]
    fn plot_level_settings_match_direct_plot() {
        let state = NativePlotState {
            legend: Some(LegendPosition::UpperLeft),
            grid: Some(false),
            x_limits: Some((-1.0, 6.0)),
            y_limits: Some((0.0, 4.0)),
            x_scale: Some(AxisScale::SymLog { linthresh: 1.0 }),
            y_scale: Some(AxisScale::Linear),
            ..styled_line_state(SeriesStyle {
                label: Some("Revenue".to_string()),
                ..SeriesStyle::default()
            })
        };

        let expected = base_plot("Styled")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .label("Revenue")
            .into_plot()
            .legend(LegendPosition::UpperLeft)
            .grid(false)
            .xlim(-1.0, 6.0)
            .ylim(0.0, 4.0)
            .xscale(AxisScale::SymLog { linthresh: 1.0 })
            .yscale(AxisScale::Linear);

        assert_pngs_equal(
            "plot_level_settings",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    /// The 2D API forwards inverted bounds instead of rejecting them, so this
    /// pins the core behavior the relaxed validation depends on.
    #[test]
    fn inverted_axis_limits_render_differently_from_ascending_ones() {
        let render = |limits: (f64, f64)| {
            render_state_png(NativePlotState {
                x_limits: Some(limits),
                ..base_state(
                    "Descending",
                    vec![NativeSeriesState::Line {
                        x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                        y: static_source(&[0.2, 1.1, 0.7, 1.8]),
                    }],
                )
            })
        };

        assert_ne!(
            render((0.0, 3.0)),
            render((3.0, 0.0)),
            "the core should honor inverted x limits as a descending axis"
        );
    }

    #[test]
    fn style_lookups_reject_unknown_names() {
        assert!(parse_color("not-a-color").is_err());
        assert!(lookup(&MARKER_STYLES, "marker", "blob").is_err());
        assert!(lookup(&LINE_STYLES, "linestyle", "wavy").is_err());
        assert!(lookup(&LEGEND_POSITIONS, "legend position", "nowhere").is_err());
        assert!(parse_axis_scale("logarithmic", None).is_err());
        assert!(parse_axis_scale("symlog", Some(0.0)).is_err());
        assert!(distinct_limits("x", 1.0, 1.0).is_err());
        assert!(distinct_limits("x", f64::NAN, 1.0).is_err());
        assert!(distinct_limits("x", 10.0, 0.0).is_ok());
        assert!(ascending_limits("z", 10.0, 0.0).is_err());
    }
}
