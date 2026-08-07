//! Browser runtime adapter for [`ruviz`](https://docs.rs/ruviz).
//!
//! `ruviz-web` is the Rust-side WebAssembly bridge that powers the browser and
//! notebook runtimes in this repository. It exposes wasm-bindgen types that can
//! render static plots, drive interactive canvas sessions, and surface browser
//! capability checks to JavaScript or TypeScript hosts.
//!
//! This crate is intentionally lower level than the published npm package. Use:
//!
//! - the root [`ruviz`](https://docs.rs/ruviz) crate for native Rust plotting
//! - the npm package `ruviz` for browser-first JS/TS usage
//! - `ruviz-web` when you are embedding the wasm bridge directly from Rust or
//!   working on the browser adapter itself
//!
//! # What This Crate Provides
//!
//! - `web_runtime_capabilities()` to probe worker, OffscreenCanvas, WebGPU, and
//!   font-registration availability
//! - `JsPlot` bindings for static PNG/SVG export
//! - `WebCanvasSession` bindings for main-thread interactive canvas rendering
//! - observable and signal wrappers used by the higher-level JS SDK
//! - font registration helpers for browser-hosted text rendering
//!
//! # Typical Usage
//!
//! Most users should not talk to these bindings directly. The recommended
//! browser-facing surface is the npm package documented in the repo at
//! `packages/ruviz/README.md`, which wraps these bindings with a higher
//! level `createPlot()` builder and session APIs.
//!
//! If you do need the raw bridge, compile the crate for `wasm32-unknown-unknown`
//! and consume the generated bindings via wasm-bindgen or wasm-pack.
//!
//! # Documentation
//!
//! - Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
//! - npm package docs: <https://github.com/Ameyanagi/ruviz/tree/main/packages/ruviz>
//! - Release notes: <https://github.com/Ameyanagi/ruviz/tree/main/docs/releases>

#![allow(clippy::needless_pass_by_value)]

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{mem, sync::OnceLock};

    use js_sys::{Array, Object, Reflect};
    #[cfg(feature = "3d-gpu")]
    use ruviz::core::{GpuSurfacePresentStatus3D, GpuSurfaceSession3D, RenderDiagnostics3D};
    use ruviz::{
        axes::AxisScale,
        core::{
            Image, ImageTarget, InteractivePlotSession, IntoPlot, LegendPosition, Plot,
            PlotBuilder, PlotInputEvent, SurfaceTarget, ViewportPoint, ViewportRect,
        },
        data::{Observable, Signal},
        plots::{LineConfig, PlotConfig, ScatterConfig},
        render::{Color, LineStyle, MarkerStyle, register_font_bytes},
    };
    #[cfg(feature = "3d")]
    use ruviz::{
        core::{InputEvent3D, InteractivePlot3DSession, PickHit3D, PointerButton3D},
        line3d, scatter3d, surface, wireframe,
    };
    use wasm_bindgen::{Clamped, JsCast, JsValue, prelude::*};
    use web_sys::{
        CanvasRenderingContext2d, HtmlCanvasElement, ImageData, OffscreenCanvas,
        OffscreenCanvasRenderingContext2d,
    };

    const DEFAULT_BROWSER_FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSans-Regular.ttf");

    static DEFAULT_BROWSER_FONT_REGISTRATION: OnceLock<std::result::Result<(), String>> =
        OnceLock::new();

    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
    }

    fn js_err<E: std::fmt::Display>(err: E) -> JsValue {
        JsValue::from_str(&err.to_string())
    }

    fn browser_bool(global_or_object: &JsValue, property: &str) -> bool {
        Reflect::has(global_or_object, &JsValue::from_str(property)).unwrap_or(false)
    }

    fn browser_number(global_or_object: &JsValue, property: &str) -> Option<f64> {
        Reflect::get(global_or_object, &JsValue::from_str(property))
            .ok()
            .and_then(|value| value.as_f64())
    }

    fn ensure_default_browser_fonts() -> Result<(), JsValue> {
        DEFAULT_BROWSER_FONT_REGISTRATION
            .get_or_init(|| {
                register_font_bytes(DEFAULT_BROWSER_FONT_BYTES.to_vec())
                    .map_err(|err| err.to_string())
            })
            .clone()
            .map_err(|err| JsValue::from_str(&err))
    }

    #[wasm_bindgen]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum WebBackendPreference {
        Auto,
        Cpu,
        Svg,
        Gpu,
    }

    #[wasm_bindgen]
    pub fn register_font_bytes_js(bytes: Vec<u8>) -> Result<(), JsValue> {
        register_font_bytes(bytes).map_err(js_err)
    }

    #[wasm_bindgen]
    pub fn register_default_browser_fonts_js() -> Result<(), JsValue> {
        ensure_default_browser_fonts()
    }

    #[wasm_bindgen]
    pub struct WebRuntimeCapabilities {
        offscreen_canvas_supported: bool,
        worker_supported: bool,
        webgpu_supported: bool,
        touch_input_supported: bool,
        default_browser_font_registered: bool,
        gpu_canvas_fast_path_available: bool,
    }

    #[wasm_bindgen]
    impl WebRuntimeCapabilities {
        #[wasm_bindgen(getter)]
        pub fn offscreen_canvas_supported(&self) -> bool {
            self.offscreen_canvas_supported
        }

        #[wasm_bindgen(getter)]
        pub fn worker_supported(&self) -> bool {
            self.worker_supported
        }

        #[wasm_bindgen(getter)]
        pub fn webgpu_supported(&self) -> bool {
            self.webgpu_supported
        }

        #[wasm_bindgen(getter)]
        pub fn touch_input_supported(&self) -> bool {
            self.touch_input_supported
        }

        #[wasm_bindgen(getter)]
        pub fn default_browser_font_registered(&self) -> bool {
            self.default_browser_font_registered
        }

        #[wasm_bindgen(getter)]
        pub fn gpu_canvas_fast_path_available(&self) -> bool {
            self.gpu_canvas_fast_path_available
        }
    }

    #[wasm_bindgen]
    pub fn web_runtime_capabilities() -> WebRuntimeCapabilities {
        let global = js_sys::global();
        let navigator = Reflect::get(&global, &JsValue::from_str("navigator")).ok();
        let webgpu_supported = navigator
            .as_ref()
            .is_some_and(|navigator| browser_bool(navigator, "gpu"));
        let touch_input_supported = navigator.as_ref().is_some_and(|navigator| {
            browser_number(navigator, "maxTouchPoints").unwrap_or(0.0) > 0.0
        }) || browser_bool(&global, "TouchEvent");
        let default_browser_font_registered = ensure_default_browser_fonts().is_ok();

        WebRuntimeCapabilities {
            offscreen_canvas_supported: browser_bool(&global, "OffscreenCanvas"),
            worker_supported: browser_bool(&global, "Worker"),
            webgpu_supported,
            touch_input_supported,
            default_browser_font_registered,
            gpu_canvas_fast_path_available: false,
        }
    }

    #[wasm_bindgen]
    pub struct ObservableVecF64 {
        inner: Observable<Vec<f64>>,
    }

    #[wasm_bindgen]
    impl ObservableVecF64 {
        #[wasm_bindgen(constructor)]
        pub fn new(values: Vec<f64>) -> Self {
            Self {
                inner: Observable::new(values),
            }
        }

        pub fn replace(&self, values: Vec<f64>) {
            self.inner.set(values);
        }

        pub fn set_at(&self, index: usize, value: f64) -> Result<(), JsValue> {
            let len = self.inner.read().len();
            if index >= len {
                return Err(JsValue::from_str("observable index is out of bounds"));
            }

            self.inner.update(|values| values[index] = value);
            Ok(())
        }

        pub fn values(&self) -> Vec<f64> {
            self.inner.read().clone()
        }

        pub fn len(&self) -> usize {
            self.inner.read().len()
        }

        pub fn is_empty(&self) -> bool {
            self.inner.read().is_empty()
        }

        pub fn clone_handle(&self) -> ObservableVecF64 {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    #[wasm_bindgen]
    pub struct SignalVecF64 {
        inner: Signal<Vec<f64>>,
        len: usize,
    }

    #[wasm_bindgen]
    impl SignalVecF64 {
        #[wasm_bindgen(js_name = sineWave)]
        #[allow(clippy::too_many_arguments)]
        pub fn sine_wave(
            points: usize,
            domain_start: f64,
            domain_end: f64,
            amplitude: f64,
            cycles: f64,
            phase_velocity: f64,
            phase_offset: f64,
            vertical_offset: f64,
        ) -> Self {
            let len = points.max(2);
            let start = if domain_start.is_finite() {
                domain_start
            } else {
                0.0
            };
            let end = if domain_end.is_finite() && domain_end.to_bits() != start.to_bits() {
                domain_end
            } else {
                start + std::f64::consts::TAU
            };
            let amplitude = if amplitude.is_finite() {
                amplitude
            } else {
                1.0
            };
            let cycles = if cycles.is_finite() { cycles } else { 1.0 };
            let phase_velocity = if phase_velocity.is_finite() {
                phase_velocity
            } else {
                0.0
            };
            let phase_offset = if phase_offset.is_finite() {
                phase_offset
            } else {
                0.0
            };
            let vertical_offset = if vertical_offset.is_finite() {
                vertical_offset
            } else {
                0.0
            };
            let span = end - start;

            let inner = Signal::new(move |time_seconds| {
                let phase = phase_offset + phase_velocity * time_seconds;
                let denom = (len - 1) as f64;

                (0..len)
                    .map(|index| {
                        let progress = index as f64 / denom;
                        let x = start + span * progress;
                        vertical_offset + amplitude * (cycles * x + phase).sin()
                    })
                    .collect()
            });

            Self { inner, len }
        }

        pub fn values_at(&self, time_seconds: f64) -> Vec<f64> {
            self.inner.at(time_seconds)
        }

        pub fn len(&self) -> usize {
            self.len
        }

        pub fn is_empty(&self) -> bool {
            self.len == 0
        }

        pub fn clone_handle(&self) -> SignalVecF64 {
            Self {
                inner: self.inner.clone(),
                len: self.len,
            }
        }
    }

    /// Optional per-series styling forwarded from a snapshot `style` object.
    ///
    /// Mirrors the Python binding's `SeriesStyle`: each field maps to one core
    /// `PlotBuilder` setter, and the accepted names come from the same tables.
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

    /// One named theme: the name JavaScript passes and the constructor it selects.
    type NamedTheme = (&'static str, fn() -> ruviz::render::Theme);

    /// The core themes the web binding exposes by name, matching the Python
    /// binding's list. `Plot3D` deliberately keeps only `light`/`dark`.
    const THEMES: [NamedTheme; 6] = [
        ("light", ruviz::render::Theme::light),
        ("dark", ruviz::render::Theme::dark),
        ("seaborn", ruviz::render::Theme::seaborn),
        ("publication", ruviz::render::Theme::publication),
        ("minimal", ruviz::render::Theme::minimal),
        ("presentation", ruviz::render::Theme::presentation),
    ];

    /// Look a name up in a `(name, value)` table, or report the accepted names.
    fn lookup<T: Clone>(table: &[(&str, T)], kind: &str, name: &str) -> Result<T, JsValue> {
        table
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| {
                let accepted: Vec<&str> = table.iter().map(|(candidate, _)| *candidate).collect();
                JsValue::from_str(&format!(
                    "unsupported {kind} '{name}'; expected one of: {}",
                    accepted.join(", ")
                ))
            })
    }

    fn parse_color(value: &str) -> Result<Color, JsValue> {
        Color::named(value)
            .or_else(|| Color::hex(value))
            .ok_or_else(|| {
                let hint = Color::suggest_named(value)
                    .map(|name| format!(" (did you mean '{name}'?)"))
                    .unwrap_or_default();
                JsValue::from_str(&format!(
                    "unsupported color '{value}'; expected a hex string like '#2563eb' \
                     or a named color such as red, green, blue, orange, purple, black, white, gray{hint}"
                ))
            })
    }

    fn parse_axis_scale(scale: &str, linthresh: Option<f64>) -> Result<AxisScale, JsValue> {
        match scale {
            "linear" => Ok(AxisScale::Linear),
            "log" => Ok(AxisScale::Log),
            "symlog" => {
                let linthresh = linthresh.unwrap_or(1.0);
                if !linthresh.is_finite() || linthresh <= 0.0 {
                    return Err(JsValue::from_str(
                        "symlog linthresh must be a finite positive number",
                    ));
                }
                Ok(AxisScale::SymLog { linthresh })
            }
            other => Err(JsValue::from_str(&format!(
                "unsupported axis scale '{other}'; expected one of: linear, log, symlog"
            ))),
        }
    }

    fn style_string(value: &JsValue, name: &str) -> Result<String, JsValue> {
        value
            .as_string()
            .ok_or_else(|| JsValue::from_str(&format!("{name} must be a string")))
    }

    fn style_number(value: &JsValue, name: &str) -> Result<f64, JsValue> {
        value
            .as_f64()
            .ok_or_else(|| JsValue::from_str(&format!("{name} must be a number")))
    }

    fn finite_positive(value: &JsValue, name: &str) -> Result<f64, JsValue> {
        let number = style_number(value, name)?;
        if !number.is_finite() || number <= 0.0 {
            return Err(JsValue::from_str(&format!(
                "{name} must be a finite positive number"
            )));
        }
        Ok(number)
    }

    fn count_at_least(value: &JsValue, name: &str, minimum: usize) -> Result<usize, JsValue> {
        let count = style_number(value, name)?;
        if !count.is_finite() || count.fract() != 0.0 || count < minimum as f64 {
            return Err(JsValue::from_str(&format!(
                "{name} must be an integer >= {minimum}"
            )));
        }
        Ok(count as usize)
    }

    /// Validate a boolean style flag; `1` and `"yes"` are caller mistakes, not flags.
    fn style_flag(value: &JsValue, name: &str) -> Result<bool, JsValue> {
        value
            .as_bool()
            .ok_or_else(|| JsValue::from_str(&format!("{name} must be a bool")))
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
    /// binding's per-kind sets so both surfaces reject the same combinations.
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
        pub(super) const SCATTER: &[&str] = &["label", "color", "alpha", "marker", "markerSize"];
        pub(super) const HISTOGRAM: &[&str] = &["label", "color", "alpha", "bins", "density"];
        pub(super) const BOXPLOT: &[&str] = &["label", "color", "alpha", "width", "linestyle"];
        pub(super) const KDE: &[&str] = &["label", "color", "alpha", "width", "bandwidth"];
        pub(super) const CONTOUR: &[&str] = &["alpha", "width", "levels"];
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
    fn unsupported_for_kind(kind: &str, key: &str, allowed: &[&str]) -> JsValue {
        let mut accepted: Vec<&str> = allowed.iter().copied().map(style_keyword).collect();
        accepted.sort_unstable();
        let accepted = if accepted.is_empty() {
            "none".to_string()
        } else {
            accepted.join(", ")
        };
        JsValue::from_str(&format!(
            "{kind} does not support {}=; accepted: {accepted}",
            style_keyword(key)
        ))
    }

    impl SeriesStyle {
        /// Parse a JS `style` object from a snapshot.
        ///
        /// Keys this build has never heard of are ignored: a snapshot written by
        /// a newer ruviz must still render everything this build understands. A
        /// *known* key the plot kind does not honor is still an error, because
        /// silently dropping it would misrender a snapshot this build does
        /// understand.
        fn from_js(style: Option<Object>, kind: &str, allowed: &[&str]) -> Result<Self, JsValue> {
            let mut parsed = Self::default();
            let Some(style) = style else {
                return Ok(parsed);
            };

            for entry in Object::entries(&style).iter() {
                let entry = Array::from(&entry);
                let key = style_string(&entry.get(0), "style key")?;
                let value = entry.get(1);
                // Optional TypeScript fields serialize as `undefined`; treat them as unset.
                if value.is_undefined() || value.is_null() {
                    continue;
                }

                if !allowed.contains(&key.as_str()) {
                    if STYLE_KEYS.contains(&key.as_str()) {
                        return Err(unsupported_for_kind(kind, &key, allowed));
                    }
                    continue;
                }

                match key.as_str() {
                    "label" => parsed.label = Some(style_string(&value, "label")?),
                    "color" => parsed.color = Some(parse_color(&style_string(&value, "color")?)?),
                    "alpha" => {
                        let alpha = style_number(&value, "alpha")?;
                        if !(0.0..=1.0).contains(&alpha) {
                            return Err(JsValue::from_str("alpha must be between 0.0 and 1.0"));
                        }
                        parsed.alpha = Some(alpha as f32);
                    }
                    "width" => parsed.width = Some(finite_positive(&value, "width")? as f32),
                    "linestyle" => {
                        parsed.line_style = Some(lookup(
                            &LINE_STYLES,
                            "linestyle",
                            &style_string(&value, "linestyle")?,
                        )?)
                    }
                    "marker" => {
                        parsed.marker = Some(lookup(
                            &MARKER_STYLES,
                            "marker",
                            &style_string(&value, "marker")?,
                        )?)
                    }
                    "markerSize" => {
                        parsed.marker_size = Some(finite_positive(&value, "marker_size")? as f32)
                    }
                    "bins" => parsed.bins = Some(count_at_least(&value, "bins", 1)?),
                    "density" => parsed.density = Some(style_flag(&value, "density")?),
                    "bandwidth" => parsed.bandwidth = Some(finite_positive(&value, "bandwidth")?),
                    "levels" => parsed.levels = Some(count_at_least(&value, "levels", 2)?),
                    // Unreachable: `allowed` is a subset of `STYLE_KEYS`, checked above.
                    _ => {}
                }
            }

            Ok(parsed)
        }
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

    /// Apply the marker options to a line. Lines draw markers only when one is
    /// chosen, so a bare `markerSize` implies circles.
    fn line_markers(
        mut builder: PlotBuilder<LineConfig>,
        style: &SeriesStyle,
    ) -> PlotBuilder<LineConfig> {
        if let Some(marker) = style
            .marker
            .or_else(|| style.marker_size.map(|_| MarkerStyle::Circle))
        {
            builder = builder.marker(marker);
        }
        if let Some(size) = style.marker_size {
            builder = builder.marker_size(size);
        }
        builder
    }

    fn scatter_markers(
        mut builder: PlotBuilder<ScatterConfig>,
        style: &SeriesStyle,
    ) -> PlotBuilder<ScatterConfig> {
        if let Some(marker) = style.marker {
            builder = builder.marker(marker);
        }
        if let Some(size) = style.marker_size {
            builder = builder.marker_size(size);
        }
        builder
    }

    #[wasm_bindgen]
    pub struct JsPlot {
        inner: Plot,
    }

    impl JsPlot {
        fn update_plot<F>(&mut self, f: F)
        where
            F: FnOnce(Plot) -> Plot,
        {
            let plot = mem::replace(&mut self.inner, Plot::new());
            self.inner = f(plot);
        }

        fn replace_with_series<F>(&mut self, f: F)
        where
            F: FnOnce(Plot) -> Plot,
        {
            let plot = mem::replace(&mut self.inner, Plot::new());
            self.inner = f(plot);
        }

        fn validate_equal_lengths(
            lengths: &[usize],
            message: &'static str,
        ) -> std::result::Result<(), JsValue> {
            let Some((&first, rest)) = lengths.split_first() else {
                return Ok(());
            };

            if rest.iter().all(|len| *len == first) {
                Ok(())
            } else {
                Err(JsValue::from_str(message))
            }
        }

        fn flatten_grid(
            values: Vec<f64>,
            rows: usize,
            cols: usize,
        ) -> Result<Vec<Vec<f64>>, JsValue> {
            if rows == 0 || cols == 0 {
                return Err(JsValue::from_str(
                    "heatmap rows and cols must be greater than zero",
                ));
            }

            if values.len() != rows.saturating_mul(cols) {
                return Err(JsValue::from_str(
                    "heatmap values length must match rows * cols",
                ));
            }

            Ok(values
                .chunks(cols)
                .map(|chunk| chunk.to_vec())
                .collect::<Vec<_>>())
        }
    }

    impl Default for JsPlot {
        fn default() -> Self {
            Self::new()
        }
    }

    #[wasm_bindgen]
    impl JsPlot {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self { inner: Plot::new() }
        }

        pub fn line(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str("x and y must have the same length"));
            }

            let style = SeriesStyle::from_js(style, "line", style_keys::LINE)?;
            self.replace_with_series(|plot| {
                styled(line_markers(plot.line(&x, &y), &style), &style).into_plot()
            });
            Ok(())
        }

        pub fn scatter(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str("x and y must have the same length"));
            }

            let style = SeriesStyle::from_js(style, "scatter", style_keys::SCATTER)?;
            self.replace_with_series(|plot| {
                styled(scatter_markers(plot.scatter(&x, &y), &style), &style).into_plot()
            });
            Ok(())
        }

        pub fn bar(
            &mut self,
            categories: Vec<String>,
            values: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if categories.len() != values.len() {
                return Err(JsValue::from_str(
                    "bar categories and values must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "bar", style_keys::COMMON)?;
            self.replace_with_series(|plot| {
                styled(plot.bar(&categories, &values), &style).into_plot()
            });
            Ok(())
        }

        pub fn bar_observable(
            &mut self,
            categories: Vec<String>,
            values: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if categories.len() != values.len() {
                return Err(JsValue::from_str(
                    "bar categories and observable values must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "bar", style_keys::COMMON)?;
            let value_source = values.inner.clone();
            self.replace_with_series(|plot| {
                styled(plot.bar_source(&categories, value_source), &style).into_plot()
            });
            Ok(())
        }

        pub fn histogram(&mut self, data: Vec<f64>, style: Option<Object>) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "histogram", style_keys::HISTOGRAM)?;
            self.replace_with_series(|plot| {
                let mut builder = plot.histogram(&data);
                if let Some(bins) = style.bins {
                    builder = builder.bins(bins);
                }
                if let Some(density) = style.density {
                    builder = builder.density(density);
                }
                styled(builder, &style).into_plot()
            });
            Ok(())
        }

        pub fn histogram_observable(
            &mut self,
            data: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "histogram", style_keys::HISTOGRAM)?;
            let data_source = data.inner.clone();
            self.replace_with_series(|plot| {
                let mut builder = plot.histogram_source(data_source);
                if let Some(bins) = style.bins {
                    builder = builder.bins(bins);
                }
                if let Some(density) = style.density {
                    builder = builder.density(density);
                }
                styled(builder, &style).into_plot()
            });
            Ok(())
        }

        pub fn boxplot(&mut self, data: Vec<f64>, style: Option<Object>) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "boxplot", style_keys::BOXPLOT)?;
            self.replace_with_series(|plot| styled(plot.boxplot(&data), &style).into_plot());
            Ok(())
        }

        pub fn boxplot_observable(
            &mut self,
            data: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "boxplot", style_keys::BOXPLOT)?;
            let data_source = data.inner.clone();
            self.replace_with_series(|plot| {
                styled(plot.boxplot_source(data_source), &style).into_plot()
            });
            Ok(())
        }

        pub fn heatmap(
            &mut self,
            values: Vec<f64>,
            rows: usize,
            cols: usize,
        ) -> Result<(), JsValue> {
            let matrix = Self::flatten_grid(values, rows, cols)?;
            self.replace_with_series(|plot| plot.heatmap(&matrix).into_plot());
            Ok(())
        }

        pub fn error_bars(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            y_errors: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), y_errors.len()],
                "x, y, and y_errors must have the same length",
            )?;

            let style = SeriesStyle::from_js(style, "error-bars", style_keys::STROKED)?;
            self.replace_with_series(|plot| {
                styled(plot.error_bars(&x, &y, &y_errors), &style).into_plot()
            });
            Ok(())
        }

        pub fn error_bars_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            y_errors: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), y_errors.len()],
                "observable x, y, and y_errors must have the same length",
            )?;

            let style = SeriesStyle::from_js(style, "error-bars", style_keys::STROKED)?;
            self.replace_with_series(|plot| {
                styled(
                    plot.error_bars_source(
                        x.inner.clone(),
                        y.inner.clone(),
                        y_errors.inner.clone(),
                    ),
                    &style,
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn error_bars_xy(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            x_errors: Vec<f64>,
            y_errors: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "x, y, x_errors, and y_errors must have the same length",
            )?;

            let style = SeriesStyle::from_js(style, "error-bars-xy", style_keys::STROKED)?;
            self.replace_with_series(|plot| {
                styled(plot.error_bars_xy(&x, &y, &x_errors, &y_errors), &style).into_plot()
            });
            Ok(())
        }

        pub fn error_bars_xy_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            x_errors: &ObservableVecF64,
            y_errors: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "observable x, y, x_errors, and y_errors must have the same length",
            )?;

            let style = SeriesStyle::from_js(style, "error-bars-xy", style_keys::STROKED)?;
            self.replace_with_series(|plot| {
                styled(
                    plot.error_bars_xy_source(
                        x.inner.clone(),
                        y.inner.clone(),
                        x_errors.inner.clone(),
                        y_errors.inner.clone(),
                    ),
                    &style,
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn kde(&mut self, data: Vec<f64>, style: Option<Object>) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "kde", style_keys::KDE)?;
            self.replace_with_series(|plot| {
                let mut builder = plot.kde(&data);
                if let Some(bandwidth) = style.bandwidth {
                    builder = builder.bandwidth(bandwidth);
                }
                styled(builder, &style).into_plot()
            });
            Ok(())
        }

        pub fn ecdf(&mut self, data: Vec<f64>, style: Option<Object>) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "ecdf", style_keys::STROKED)?;
            self.replace_with_series(|plot| styled(plot.ecdf(&data), &style).into_plot());
            Ok(())
        }

        pub fn contour(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            z: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.is_empty() || y.is_empty() {
                return Err(JsValue::from_str("contour x and y must not be empty"));
            }

            if z.len() != x.len().saturating_mul(y.len()) {
                return Err(JsValue::from_str(
                    "contour z length must equal x.len() * y.len()",
                ));
            }

            let style = SeriesStyle::from_js(style, "contour", style_keys::CONTOUR)?;
            self.replace_with_series(|plot| {
                let mut builder = plot.contour(&x, &y, &z);
                if let Some(levels) = style.levels {
                    builder = builder.levels(levels);
                }
                styled(builder, &style).into_plot()
            });
            Ok(())
        }

        pub fn pie(&mut self, values: Vec<f64>) {
            self.replace_with_series(|plot| plot.pie(&values).into_plot());
        }

        pub fn pie_with_labels(
            &mut self,
            values: Vec<f64>,
            labels: Vec<String>,
        ) -> Result<(), JsValue> {
            if values.len() != labels.len() {
                return Err(JsValue::from_str(
                    "pie values and labels must have the same length",
                ));
            }

            self.replace_with_series(|plot| plot.pie(&values).labels(&labels).into_plot());
            Ok(())
        }

        pub fn radar(
            &mut self,
            labels: Vec<String>,
            series_names: Vec<String>,
            series_values: Vec<f64>,
        ) -> Result<(), JsValue> {
            if labels.is_empty() {
                return Err(JsValue::from_str("radar labels must not be empty"));
            }

            let points_per_series = labels.len();
            if series_values.is_empty() || !series_values.len().is_multiple_of(points_per_series) {
                return Err(JsValue::from_str(
                    "radar series values length must be a multiple of labels length",
                ));
            }

            let series_count = series_values.len() / points_per_series;
            if !series_names.is_empty() && series_names.len() != series_count {
                return Err(JsValue::from_str(
                    "radar series_names length must match the number of series",
                ));
            }

            self.replace_with_series(|plot| {
                let mut builder = plot.radar(&labels);
                for (index, chunk) in series_values.chunks(points_per_series).enumerate() {
                    if let Some(name) = series_names.get(index).filter(|name| !name.is_empty()) {
                        builder = builder.add_series(name.clone(), &chunk);
                    } else {
                        builder = builder.series(&chunk);
                    }
                }
                builder.into_plot()
            });
            Ok(())
        }

        pub fn violin(&mut self, data: Vec<f64>, style: Option<Object>) -> Result<(), JsValue> {
            let style = SeriesStyle::from_js(style, "violin", style_keys::STROKED)?;
            self.replace_with_series(|plot| styled(plot.violin(&data), &style).into_plot());
            Ok(())
        }

        pub fn polar_line(
            &mut self,
            r: Vec<f64>,
            theta: Vec<f64>,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;

            let style = SeriesStyle::from_js(style, "polar-line", style_keys::STROKED)?;
            self.replace_with_series(|plot| {
                styled(plot.polar_line(&r, &theta), &style).into_plot()
            });
            Ok(())
        }

        pub fn line_signal(
            &mut self,
            x: Vec<f64>,
            y: &SignalVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "signal y data and x must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "line", style_keys::LINE)?;
            let y_signal = y.inner.clone();
            self.replace_with_series(|plot| {
                styled(line_markers(plot.line_source(x, y_signal), &style), &style).into_plot()
            });
            Ok(())
        }

        pub fn scatter_signal(
            &mut self,
            x: Vec<f64>,
            y: &SignalVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "signal y data and x must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "scatter", style_keys::SCATTER)?;
            let y_signal = y.inner.clone();
            self.replace_with_series(|plot| {
                styled(
                    scatter_markers(plot.scatter_source(x, y_signal), &style),
                    &style,
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn line_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "observable x and y must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "line", style_keys::LINE)?;
            self.replace_with_series(|plot| {
                styled(
                    line_markers(plot.line_source(x.inner.clone(), y.inner.clone()), &style),
                    &style,
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn scatter_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            style: Option<Object>,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "observable x and y must have the same length",
                ));
            }

            let style = SeriesStyle::from_js(style, "scatter", style_keys::SCATTER)?;
            self.replace_with_series(|plot| {
                styled(
                    scatter_markers(
                        plot.scatter_source(x.inner.clone(), y.inner.clone()),
                        &style,
                    ),
                    &style,
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn title(&mut self, title: &str) {
            self.update_plot(|plot| plot.title(title));
        }

        pub fn xlabel(&mut self, label: &str) {
            self.update_plot(|plot| plot.xlabel(label));
        }

        pub fn ylabel(&mut self, label: &str) {
            self.update_plot(|plot| plot.ylabel(label));
        }

        pub fn size_px(&mut self, width: u32, height: u32) {
            self.update_plot(|plot| plot.size_px(width, height));
        }

        /// Sets the output DPI, which scales the exported pixels from `size_px`.
        /// Apply it after `size_px`, which fixes the figure size in inches.
        pub fn dpi(&mut self, dpi: u32) -> Result<(), JsValue> {
            if dpi == 0 {
                return Err(JsValue::from_str(
                    "plot dpi must be an integer greater than zero",
                ));
            }
            self.update_plot(|plot| plot.dpi(dpi));
            Ok(())
        }

        /// Sets the x-axis limits. Inverted bounds keep a descending axis.
        pub fn xlim(&mut self, min: f64, max: f64) {
            self.update_plot(|plot| plot.xlim(min, max));
        }

        /// Sets the y-axis limits. Inverted bounds keep a descending axis.
        pub fn ylim(&mut self, min: f64, max: f64) {
            self.update_plot(|plot| plot.ylim(min, max));
        }

        /// Sets the x-axis scale. `linthresh` applies to `symlog` only and
        /// defaults to `1.0`.
        pub fn xscale(&mut self, scale: &str, linthresh: Option<f64>) -> Result<(), JsValue> {
            let scale = parse_axis_scale(scale, linthresh)?;
            self.update_plot(|plot| plot.xscale(scale));
            Ok(())
        }

        /// Sets the y-axis scale. `linthresh` applies to `symlog` only and
        /// defaults to `1.0`.
        pub fn yscale(&mut self, scale: &str, linthresh: Option<f64>) -> Result<(), JsValue> {
            let scale = parse_axis_scale(scale, linthresh)?;
            self.update_plot(|plot| plot.yscale(scale));
            Ok(())
        }

        /// Shows the legend at a lowercase position name such as `upper_right`,
        /// or `best` to auto-place it.
        pub fn legend(&mut self, position: &str) -> Result<(), JsValue> {
            let position = lookup(&LEGEND_POSITIONS, "legend position", position)?;
            self.update_plot(|plot| plot.legend(position));
            Ok(())
        }

        pub fn grid(&mut self, enabled: bool) {
            self.update_plot(|plot| plot.grid(enabled));
        }

        pub fn ticks(&mut self, enabled: bool) {
            self.update_plot(|plot| plot.ticks(enabled));
        }

        /// Applies a built-in theme by name, such as `light` or `seaborn`.
        pub fn theme(&mut self, name: &str) -> Result<(), JsValue> {
            let build = lookup(&THEMES, "theme", name)?;
            self.update_plot(move |plot| plot.theme(build()));
            Ok(())
        }

        pub fn render_png_bytes(&self) -> Result<Vec<u8>, JsValue> {
            ensure_default_browser_fonts()?;
            self.inner.render_png_bytes().map_err(js_err)
        }

        pub fn render_svg(&self) -> Result<String, JsValue> {
            ensure_default_browser_fonts()?;
            self.inner.render_to_svg().map_err(js_err)
        }

        pub fn clone_plot(&self) -> JsPlot {
            JsPlot {
                inner: self.inner.clone(),
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum DragMode {
        Pan { last: ViewportPoint, moved: bool },
        ZoomRect { anchor: ViewportPoint, moved: bool },
    }

    struct BrowserSession {
        plot: Option<Plot>,
        session: Option<InteractivePlotSession>,
        size_px: (u32, u32),
        scale_factor: f32,
        time_seconds: f64,
        backend: WebBackendPreference,
        drag: Option<DragMode>,
        frame_version: u64,
        export_png_cache: Option<(u64, Vec<u8>)>,
        export_svg_cache: Option<(u64, String)>,
    }

    impl BrowserSession {
        fn new(size_px: (u32, u32)) -> Self {
            Self {
                plot: None,
                session: None,
                size_px,
                scale_factor: 1.0,
                time_seconds: 0.0,
                backend: WebBackendPreference::Auto,
                drag: None,
                frame_version: 0,
                export_png_cache: None,
                export_svg_cache: None,
            }
        }

        fn session(&self) -> Result<&InteractivePlotSession, JsValue> {
            self.session
                .as_ref()
                .ok_or_else(|| JsValue::from_str("no plot is attached to this session"))
        }

        fn has_plot(&self) -> bool {
            self.session.is_some()
        }

        fn destroy(&mut self) {
            self.plot = None;
            self.session = None;
            self.drag = None;
            self.mark_frame_dirty();
        }

        fn mark_frame_dirty(&mut self) {
            self.frame_version = self.frame_version.wrapping_add(1);
            self.export_png_cache = None;
            self.export_svg_cache = None;
        }

        fn configure_session(&self, session: &InteractivePlotSession) {
            session.resize(self.size_px, self.scale_factor);
            session.apply_input(PlotInputEvent::SetTime {
                time_seconds: self.time_seconds,
            });
            session.set_prefer_gpu(matches!(self.backend, WebBackendPreference::Gpu));
        }

        fn set_plot(&mut self, plot: Plot) {
            let session = plot.prepare_interactive();
            self.configure_session(&session);
            self.plot = Some(plot);
            self.session = Some(session);
            self.drag = None;
            self.mark_frame_dirty();
        }

        fn resize(&mut self, width: u32, height: u32, scale_factor: f32) {
            self.size_px = (width.max(1), height.max(1));
            self.scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
                scale_factor
            } else {
                1.0
            };
            if let Some(session) = &self.session {
                self.configure_session(session);
            }
            self.mark_frame_dirty();
        }

        fn set_backend_preference(&mut self, backend: WebBackendPreference) {
            self.backend = backend;
            if let Some(session) = &self.session {
                session.set_prefer_gpu(matches!(backend, WebBackendPreference::Gpu));
            }
            self.mark_frame_dirty();
        }

        fn set_time(&mut self, time_seconds: f64) {
            self.time_seconds = time_seconds;
            if let Some(session) = &self.session {
                session.apply_input(PlotInputEvent::SetTime { time_seconds });
            }
            self.mark_frame_dirty();
        }

        fn render_frame(&mut self) -> Result<Image, JsValue> {
            let frame = self
                .session()?
                .render_to_surface(SurfaceTarget {
                    size_px: self.size_px,
                    scale_factor: self.scale_factor,
                    time_seconds: self.time_seconds,
                })
                .map_err(js_err)?;
            if let Some(overlay) = frame.layers.overlay.as_ref() {
                Ok(compose_browser_layers(
                    frame.layers.base.as_ref(),
                    overlay.as_ref(),
                ))
            } else {
                Ok((*frame.layers.base).clone())
            }
        }

        fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            if let Some((version, cached)) = &self.export_png_cache {
                if *version == self.frame_version {
                    return Ok(cached.clone());
                }
            }

            let png = self
                .session()?
                .render_to_image(ImageTarget {
                    size_px: self.size_px,
                    scale_factor: self.scale_factor,
                    time_seconds: self.time_seconds,
                })
                .map_err(js_err)?
                .image
                .encode_png()
                .map_err(js_err)?;
            self.export_png_cache = Some((self.frame_version, png.clone()));
            Ok(png)
        }

        fn export_svg(&mut self) -> Result<String, JsValue> {
            if let Some((version, cached)) = &self.export_svg_cache {
                if *version == self.frame_version {
                    return Ok(cached.clone());
                }
            }

            let mut plot = self
                .plot
                .clone()
                .ok_or_else(|| JsValue::from_str("no plot is attached to this session"))?;

            if let Some(session) = &self.session {
                let snapshot = session.viewport_snapshot().map_err(js_err)?;
                plot = plot
                    .xlim(snapshot.visible_bounds.min.x, snapshot.visible_bounds.max.x)
                    .ylim(snapshot.visible_bounds.min.y, snapshot.visible_bounds.max.y);
            }

            let svg = plot.render_to_svg().map_err(js_err)?;
            self.export_svg_cache = Some((self.frame_version, svg.clone()));
            Ok(svg)
        }

        fn reset_view(&mut self) -> Result<(), JsValue> {
            self.session()?.apply_input(PlotInputEvent::ResetView);
            self.drag = None;
            self.mark_frame_dirty();
            Ok(())
        }

        /// Maps a backing-surface pixel to displayed data coordinates.
        fn data_at(&self, x: f64, y: f64) -> Result<(f64, f64), JsValue> {
            let point = self
                .session()?
                .screen_to_data_clamped(ViewportPoint::new(x, y))
                .map_err(js_err)?;
            Ok((point.x, point.y))
        }

        fn pointer_down(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            let point = ViewportPoint::new(x, y);
            match button {
                0 => {
                    self.drag = Some(DragMode::Pan {
                        last: point,
                        moved: false,
                    });
                }
                2 => {
                    self.session()?
                        .apply_input(PlotInputEvent::BrushStart { position_px: point });
                    self.drag = Some(DragMode::ZoomRect {
                        anchor: point,
                        moved: false,
                    });
                    self.mark_frame_dirty();
                }
                _ => {}
            }
            Ok(())
        }

        fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
            let point = ViewportPoint::new(x, y);
            let session = self.session()?;
            let mut frame_changed = false;

            match self.drag {
                Some(DragMode::Pan {
                    mut last,
                    moved: _moved,
                }) => {
                    let delta = ViewportPoint::new(point.x - last.x, point.y - last.y);
                    if delta.x != 0.0 || delta.y != 0.0 {
                        session.apply_input(PlotInputEvent::Pan { delta_px: delta });
                        frame_changed = true;
                    }
                    last = point;
                    self.drag = Some(DragMode::Pan { last, moved: true });
                }
                Some(DragMode::ZoomRect { anchor, .. }) => {
                    session.apply_input(PlotInputEvent::BrushMove { position_px: point });
                    self.drag = Some(DragMode::ZoomRect {
                        anchor,
                        moved: true,
                    });
                    frame_changed = true;
                }
                None => {
                    session.apply_input(PlotInputEvent::Hover { position_px: point });
                    frame_changed = true;
                }
            }

            if frame_changed {
                self.mark_frame_dirty();
            }

            Ok(())
        }

        fn pointer_up(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            let point = ViewportPoint::new(x, y);
            let drag = self.drag.take();
            let session = self.session()?;
            let mut frame_changed = false;

            match (drag, button) {
                (Some(DragMode::Pan { moved, .. }), 0) => {
                    if !moved {
                        session.apply_input(PlotInputEvent::SelectAt { position_px: point });
                        frame_changed = true;
                    }
                }
                (Some(DragMode::ZoomRect { anchor, moved }), 2) => {
                    session.apply_input(PlotInputEvent::BrushEnd { position_px: point });
                    if moved {
                        session.apply_input(PlotInputEvent::ZoomRect {
                            region_px: ViewportRect::from_points(anchor, point),
                        });
                    }
                    frame_changed = true;
                }
                _ => {}
            }

            if frame_changed {
                self.mark_frame_dirty();
            }

            Ok(())
        }

        fn clear_hover(&mut self) -> Result<(), JsValue> {
            self.session()?.apply_input(PlotInputEvent::ClearHover);
            self.mark_frame_dirty();
            Ok(())
        }

        fn wheel(&mut self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
            let steps = (delta_y / 120.0).clamp(-10.0, 10.0);
            let factor = 1.1_f64.powf(-steps);
            self.session()?.apply_input(PlotInputEvent::Zoom {
                factor,
                center_px: ViewportPoint::new(x, y),
            });
            self.mark_frame_dirty();
            Ok(())
        }
    }

    fn compose_browser_layers(base: &Image, overlay: &Image) -> Image {
        debug_assert_eq!(
            (base.width, base.height),
            (overlay.width, overlay.height),
            "compose_browser_layers: base and overlay must have the same dimensions"
        );
        let mut pixels = base.pixels.clone();
        for (dst, src) in pixels
            .chunks_exact_mut(4)
            .zip(overlay.pixels.chunks_exact(4))
        {
            let alpha = src[3] as f32 / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            dst[0] = blend_channel(dst[0], src[0], alpha);
            dst[1] = blend_channel(dst[1], src[1], alpha);
            dst[2] = blend_channel(dst[2], src[2], alpha);
            dst[3] = 255;
        }
        Image::new(base.width, base.height, pixels)
    }

    fn blend_channel(background: u8, foreground: u8, alpha: f32) -> u8 {
        let bg = background as f32 / 255.0;
        let fg = foreground as f32 / 255.0;
        ((bg * (1.0 - alpha) + fg * alpha) * 255.0) as u8
    }

    enum CanvasSurface {
        Html {
            canvas: HtmlCanvasElement,
            context: CanvasRenderingContext2d,
        },
        Offscreen {
            canvas: OffscreenCanvas,
            context: OffscreenCanvasRenderingContext2d,
        },
    }

    impl CanvasSurface {
        fn from_html(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
            let context = canvas
                .get_context("2d")?
                .ok_or_else(|| JsValue::from_str("2d canvas context is not available"))?
                .dyn_into::<CanvasRenderingContext2d>()?;
            Ok(Self::Html { canvas, context })
        }

        fn from_offscreen(canvas: OffscreenCanvas) -> Result<Self, JsValue> {
            let context = canvas
                .get_context("2d")?
                .ok_or_else(|| JsValue::from_str("2d offscreen canvas context is not available"))?
                .dyn_into::<OffscreenCanvasRenderingContext2d>()?;
            Ok(Self::Offscreen { canvas, context })
        }

        fn size_px(&self) -> (u32, u32) {
            match self {
                Self::Html { canvas, .. } => (canvas.width().max(1), canvas.height().max(1)),
                Self::Offscreen { canvas, .. } => (canvas.width().max(1), canvas.height().max(1)),
            }
        }

        fn set_size(&self, width: u32, height: u32) {
            match self {
                Self::Html { canvas, .. } => {
                    canvas.set_width(width.max(1));
                    canvas.set_height(height.max(1));
                }
                Self::Offscreen { canvas, .. } => {
                    canvas.set_width(width.max(1));
                    canvas.set_height(height.max(1));
                }
            }
        }

        fn draw_image(&self, image: &Image) -> Result<(), JsValue> {
            let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                Clamped(image.pixels.as_slice()),
                image.width,
                image.height,
            )?;

            match self {
                Self::Html { context, .. } => context.put_image_data(&image_data, 0.0, 0.0),
                Self::Offscreen { context, .. } => context.put_image_data(&image_data, 0.0, 0.0),
            }
        }

        fn clear(&self) {
            match self {
                Self::Html { canvas, context } => {
                    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
                }
                Self::Offscreen { canvas, context } => {
                    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
                }
            }
        }
    }

    #[cfg(feature = "3d")]
    enum WebPlot3DKind {
        Scatter {
            x: Vec<f64>,
            y: Vec<f64>,
            z: Vec<f64>,
        },
        Line {
            x: Vec<f64>,
            y: Vec<f64>,
            z: Vec<f64>,
        },
        Surface {
            x: Vec<f64>,
            y: Vec<f64>,
            z: Vec<Vec<f64>>,
        },
        Wireframe {
            x: Vec<f64>,
            y: Vec<f64>,
            z: Vec<Vec<f64>>,
        },
    }

    #[cfg(feature = "3d")]
    type WebGrid3D = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>);

    /// Small JavaScript-owned 3d plot description.
    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    pub struct JsPlot3D {
        kind: Option<WebPlot3DKind>,
        title: Option<String>,
    }

    #[cfg(feature = "3d")]
    impl JsPlot3D {
        fn validate_points(x: &[f64], y: &[f64], z: &[f64]) -> Result<(), JsValue> {
            if x.len() == y.len() && y.len() == z.len() {
                Ok(())
            } else {
                Err(JsValue::from_str(
                    "3d x, y, and z must have the same length",
                ))
            }
        }

        fn grid(x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<WebGrid3D, JsValue> {
            if x.is_empty() || y.is_empty() || z.len() != x.len().saturating_mul(y.len()) {
                return Err(JsValue::from_str(
                    "3d surface z length must equal x.length * y.length",
                ));
            }
            let rows = z.chunks(x.len()).map(<[f64]>::to_vec).collect();
            Ok((x, y, rows))
        }

        fn build_session(&self) -> Result<InteractivePlot3DSession, JsValue> {
            match self.kind.as_ref() {
                Some(WebPlot3DKind::Scatter { x, y, z }) => {
                    let builder = scatter3d(x, y, z);
                    match &self.title {
                        Some(title) => builder.title(title).interactive_session(),
                        None => builder.interactive_session(),
                    }
                }
                Some(WebPlot3DKind::Line { x, y, z }) => {
                    let builder = line3d(x, y, z);
                    match &self.title {
                        Some(title) => builder.title(title).interactive_session(),
                        None => builder.interactive_session(),
                    }
                }
                Some(WebPlot3DKind::Surface { x, y, z }) => {
                    let builder = surface(x, y, z);
                    match &self.title {
                        Some(title) => builder.title(title).interactive_session(),
                        None => builder.interactive_session(),
                    }
                }
                Some(WebPlot3DKind::Wireframe { x, y, z }) => {
                    let builder = wireframe(x, y, z);
                    match &self.title {
                        Some(title) => builder.title(title).interactive_session(),
                        None => builder.interactive_session(),
                    }
                }
                None => Err(ruviz::core::PlottingError::NoDataSeries),
            }
            .map_err(js_err)
        }
    }

    #[cfg(feature = "3d")]
    impl Default for JsPlot3D {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    impl JsPlot3D {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self {
                kind: None,
                title: None,
            }
        }

        pub fn scatter3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<(), JsValue> {
            Self::validate_points(&x, &y, &z)?;
            self.kind = Some(WebPlot3DKind::Scatter { x, y, z });
            Ok(())
        }

        pub fn line3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<(), JsValue> {
            Self::validate_points(&x, &y, &z)?;
            self.kind = Some(WebPlot3DKind::Line { x, y, z });
            Ok(())
        }

        pub fn surface(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<(), JsValue> {
            let (x, y, z) = Self::grid(x, y, z)?;
            self.kind = Some(WebPlot3DKind::Surface { x, y, z });
            Ok(())
        }

        pub fn wireframe(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<(), JsValue> {
            let (x, y, z) = Self::grid(x, y, z)?;
            self.kind = Some(WebPlot3DKind::Wireframe { x, y, z });
            Ok(())
        }

        pub fn title(&mut self, title: &str) {
            self.title = (!title.is_empty()).then(|| title.to_string());
        }
    }

    #[cfg(feature = "3d")]
    struct Browser3DSession {
        session: Option<InteractivePlot3DSession>,
        surface: CanvasSurface,
        selected: Option<PickHit3D>,
        scale_factor: f32,
    }

    #[cfg(feature = "3d")]
    impl Browser3DSession {
        fn new(surface: CanvasSurface) -> Self {
            Self {
                session: None,
                surface,
                selected: None,
                scale_factor: 1.0,
            }
        }

        fn session_mut(&mut self) -> Result<&mut InteractivePlot3DSession, JsValue> {
            self.session
                .as_mut()
                .ok_or_else(|| JsValue::from_str("no 3d plot is attached to this session"))
        }

        fn set_plot(&mut self, plot: &JsPlot3D) -> Result<(), JsValue> {
            let mut session = plot.build_session()?;
            let (width, height) = self.surface.size_px();
            session
                .resize(width.max(1), height.max(1), self.scale_factor)
                .map_err(js_err)?;
            self.session = Some(session);
            self.selected = None;
            self.render()
        }

        fn resize(&mut self, width: u32, height: u32, scale_factor: f32) -> Result<(), JsValue> {
            self.scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
                scale_factor
            } else {
                1.0
            };
            self.surface.set_size(width.max(1), height.max(1));
            if let Some(session) = &mut self.session {
                session
                    .resize(width.max(1), height.max(1), self.scale_factor)
                    .map_err(js_err)?;
                self.render()?;
            }
            Ok(())
        }

        fn apply(&mut self, event: InputEvent3D, render: bool) -> Result<(), JsValue> {
            let result = self.session_mut()?.handle_input(event).map_err(js_err)?;
            if let Some(hit) = result.picked {
                self.selected = Some(hit);
            }
            if render && result.request_redraw {
                self.render()?;
            }
            Ok(())
        }

        fn pointer_button(button: i16) -> Result<PointerButton3D, JsValue> {
            match button {
                0 => Ok(PointerButton3D::Left),
                1 => Ok(PointerButton3D::Middle),
                2 => Ok(PointerButton3D::Right),
                _ => Err(JsValue::from_str(
                    "3d pointer button must be 0 (left), 1 (middle), or 2 (right)",
                )),
            }
        }

        fn render(&mut self) -> Result<(), JsValue> {
            let image = self.session_mut()?.render().map_err(js_err)?;
            self.surface.draw_image(&image)
        }

        fn selected_series(&self) -> i32 {
            self.selected.map_or(-1, |hit| hit.series_index as i32)
        }

        fn selected_source(&self) -> i32 {
            self.selected
                .and_then(|hit| hit.sources().first().copied())
                .map_or(-1, |index| index as i32)
        }

        fn destroy(&mut self) {
            self.session = None;
            self.selected = None;
            self.surface.clear();
        }
    }

    /// Main-thread Canvas2D correctness adapter for retained 3d plots.
    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    pub struct Web3DCanvasSession {
        browser: Browser3DSession,
    }

    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    impl Web3DCanvasSession {
        #[wasm_bindgen(constructor)]
        pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
            ensure_default_browser_fonts()?;
            Ok(Self {
                browser: Browser3DSession::new(CanvasSurface::from_html(canvas)?),
            })
        }

        pub fn set_plot(&mut self, plot: &JsPlot3D) -> Result<(), JsValue> {
            self.browser.set_plot(plot)
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.browser.resize(width, height, scale_factor)
        }

        pub fn pointer_down(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::PointerDown {
                    x,
                    y,
                    button: Browser3DSession::pointer_button(button)?,
                },
                false,
            )
        }

        pub fn pointer_move(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerMove { x, y }, true)
        }

        pub fn pointer_up(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::PointerUp {
                    x,
                    y,
                    button: Browser3DSession::pointer_button(button)?,
                },
                true,
            )
        }

        pub fn double_click(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::DoubleClick {
                    x,
                    y,
                    button: PointerButton3D::Left,
                },
                true,
            )
        }

        pub fn wheel(&mut self, delta_y: f32) -> Result<(), JsValue> {
            self.browser
                .apply(InputEvent3D::Wheel { delta_y: -delta_y }, true)
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::Escape, true)
        }

        pub fn selected_series(&self) -> i32 {
            self.browser.selected_series()
        }

        pub fn selected_source(&self) -> i32 {
            self.browser.selected_source()
        }

        pub fn render(&mut self) -> Result<(), JsValue> {
            self.browser.render()
        }

        pub fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            self.browser
                .session_mut()?
                .render()
                .and_then(|image| image.encode_png())
                .map_err(js_err)
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
        }
    }

    /// Worker/OffscreenCanvas adapter with the same 3d event semantics.
    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    pub struct Offscreen3DCanvasSession {
        browser: Browser3DSession,
    }

    #[cfg(feature = "3d")]
    #[wasm_bindgen]
    impl Offscreen3DCanvasSession {
        #[wasm_bindgen(constructor)]
        pub fn new(canvas: OffscreenCanvas) -> Result<Self, JsValue> {
            ensure_default_browser_fonts()?;
            Ok(Self {
                browser: Browser3DSession::new(CanvasSurface::from_offscreen(canvas)?),
            })
        }

        pub fn set_plot(&mut self, plot: &JsPlot3D) -> Result<(), JsValue> {
            self.browser.set_plot(plot)
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.browser.resize(width, height, scale_factor)
        }

        pub fn pointer_down(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::PointerDown {
                    x,
                    y,
                    button: Browser3DSession::pointer_button(button)?,
                },
                false,
            )
        }

        pub fn pointer_move(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerMove { x, y }, true)
        }

        pub fn pointer_up(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::PointerUp {
                    x,
                    y,
                    button: Browser3DSession::pointer_button(button)?,
                },
                true,
            )
        }

        pub fn double_click(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(
                InputEvent3D::DoubleClick {
                    x,
                    y,
                    button: PointerButton3D::Left,
                },
                true,
            )
        }

        pub fn wheel(&mut self, delta_y: f32) -> Result<(), JsValue> {
            self.browser
                .apply(InputEvent3D::Wheel { delta_y: -delta_y }, true)
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::Escape, true)
        }

        pub fn selected_series(&self) -> i32 {
            self.browser.selected_series()
        }

        pub fn selected_source(&self) -> i32 {
            self.browser.selected_source()
        }

        pub fn render(&mut self) -> Result<(), JsValue> {
            self.browser.render()
        }

        pub fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            self.browser
                .session_mut()?
                .render()
                .and_then(|image| image.encode_png())
                .map_err(js_err)
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
        }
    }

    #[cfg(feature = "3d-gpu")]
    struct BrowserWebGpu3DSession {
        surface: Option<wgpu::Surface<'static>>,
        gpu: Option<GpuSurfaceSession3D>,
        selected: Option<PickHit3D>,
        last_diagnostics: Option<RenderDiagnostics3D>,
        totals: RenderDiagnostics3D,
        render_pending: bool,
        needs_recreate: bool,
        scale_factor: f32,
    }

    #[cfg(feature = "3d-gpu")]
    impl BrowserWebGpu3DSession {
        async fn new(
            instance: wgpu::Instance,
            surface: wgpu::Surface<'static>,
            session: InteractivePlot3DSession,
        ) -> Result<Self, JsValue> {
            let gpu = GpuSurfaceSession3D::new(session, instance, &surface)
                .await
                .map_err(js_err)?;
            Ok(Self {
                surface: Some(surface),
                gpu: Some(gpu),
                selected: None,
                last_diagnostics: None,
                totals: RenderDiagnostics3D::default(),
                render_pending: true,
                needs_recreate: false,
                scale_factor: 1.0,
            })
        }

        fn apply(&mut self, event: InputEvent3D) -> Result<(), JsValue> {
            let result = self
                .gpu
                .as_mut()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d session was destroyed"))?
                .handle_input(event)
                .map_err(js_err)?;
            if let Some(hit) = result.picked {
                self.selected = Some(hit);
            }
            self.render_pending |= result.request_redraw;
            Ok(())
        }

        fn resize(&mut self, width: u32, height: u32, scale_factor: f32) -> Result<(), JsValue> {
            self.scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
                scale_factor
            } else {
                1.0
            };
            let surface = self
                .surface
                .as_ref()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d surface was destroyed"))?;
            self.gpu
                .as_mut()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d session was destroyed"))?
                .resize(surface, width.max(1), height.max(1), self.scale_factor)
                .map_err(js_err)?;
            self.render_pending = true;
            Ok(())
        }

        fn render(&mut self) -> Result<bool, JsValue> {
            if !self.render_pending {
                return Ok(false);
            }
            let surface = self
                .surface
                .as_ref()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d surface was destroyed"))?;
            let status = self
                .gpu
                .as_mut()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d session was destroyed"))?
                .present(surface)
                .map_err(js_err)?;
            match status {
                GpuSurfacePresentStatus3D::Presented(diagnostics) => {
                    debug_assert_eq!(diagnostics.readback_bytes, 0);
                    self.totals.readback_bytes = self
                        .totals
                        .readback_bytes
                        .saturating_add(diagnostics.readback_bytes);
                    self.totals.vertex_upload_bytes = self
                        .totals
                        .vertex_upload_bytes
                        .saturating_add(diagnostics.vertex_upload_bytes);
                    self.totals.index_upload_bytes = self
                        .totals
                        .index_upload_bytes
                        .saturating_add(diagnostics.index_upload_bytes);
                    self.totals.texture_upload_bytes = self
                        .totals
                        .texture_upload_bytes
                        .saturating_add(diagnostics.texture_upload_bytes);
                    self.totals.presentation_vertex_upload_bytes = self
                        .totals
                        .presentation_vertex_upload_bytes
                        .saturating_add(diagnostics.presentation_vertex_upload_bytes);
                    self.totals.presentation_texture_upload_bytes = self
                        .totals
                        .presentation_texture_upload_bytes
                        .saturating_add(diagnostics.presentation_texture_upload_bytes);
                    self.totals.surface_presents = self
                        .totals
                        .surface_presents
                        .saturating_add(diagnostics.surface_presents);
                    self.last_diagnostics = Some(diagnostics);
                    self.render_pending = false;
                    Ok(true)
                }
                GpuSurfacePresentStatus3D::Skipped => Ok(false),
                GpuSurfacePresentStatus3D::RecreateSurface => {
                    self.needs_recreate = true;
                    Err(JsValue::from_str(
                        "the WebGPU 3d surface or device was lost; recreate the session",
                    ))
                }
            }
        }

        fn selected_series(&self) -> i32 {
            self.selected.map_or(-1, |hit| hit.series_index as i32)
        }

        fn selected_source(&self) -> i32 {
            self.selected
                .and_then(|hit| hit.sources().first().copied())
                .map_or(-1, |index| index as i32)
        }

        fn export_png(&self) -> Result<Vec<u8>, JsValue> {
            self.gpu
                .as_ref()
                .ok_or_else(|| JsValue::from_str("the WebGPU 3d session was destroyed"))?
                .render_png_bytes()
                .map_err(js_err)
        }

        fn destroy(&mut self) {
            self.gpu = None;
            self.surface = None;
            self.selected = None;
            self.last_diagnostics = None;
            self.totals = RenderDiagnostics3D::default();
            self.render_pending = false;
        }
    }

    #[cfg(feature = "3d-gpu")]
    fn webgpu_instance() -> wgpu::Instance {
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: if cfg!(debug_assertions) {
                wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION
            } else {
                wgpu::InstanceFlags::default()
            },
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        })
    }

    /// Main-thread direct WebGPU adapter for retained 3d plots.
    ///
    /// Input methods only mark the retained frame dirty. Hosts call `render`
    /// once per animation frame, so pointer bursts coalesce into one submit.
    #[cfg(feature = "3d-gpu")]
    #[wasm_bindgen(js_name = WebGPU3DCanvasSession)]
    pub struct WebGpu3DCanvasSession {
        canvas: HtmlCanvasElement,
        browser: BrowserWebGpu3DSession,
    }

    #[cfg(feature = "3d-gpu")]
    #[wasm_bindgen]
    impl WebGpu3DCanvasSession {
        pub async fn create(
            canvas: HtmlCanvasElement,
            plot: &JsPlot3D,
        ) -> Result<WebGpu3DCanvasSession, JsValue> {
            ensure_default_browser_fonts()?;
            let mut session = plot.build_session()?;
            session
                .resize(canvas.width().max(1), canvas.height().max(1), 1.0)
                .map_err(js_err)?;
            let instance = webgpu_instance();
            let surface: wgpu::Surface<'static> = instance
                .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
                .map_err(js_err)?;
            let browser = BrowserWebGpu3DSession::new(instance, surface, session).await?;
            let mut adapter = Self { canvas, browser };
            let _ = adapter.browser.render()?;
            Ok(adapter)
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.canvas.set_width(width.max(1));
            self.canvas.set_height(height.max(1));
            self.browser.resize(width, height, scale_factor)
        }

        pub fn pointer_down(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerDown {
                x,
                y,
                button: Browser3DSession::pointer_button(button)?,
            })
        }

        pub fn pointer_move(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerMove { x, y })
        }

        pub fn pointer_up(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerUp {
                x,
                y,
                button: Browser3DSession::pointer_button(button)?,
            })
        }

        pub fn double_click(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::DoubleClick {
                x,
                y,
                button: PointerButton3D::Left,
            })
        }

        pub fn wheel(&mut self, delta_y: f32) -> Result<(), JsValue> {
            self.browser
                .apply(InputEvent3D::Wheel { delta_y: -delta_y })
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::Escape)
        }

        /// Submit at most one dirty frame.
        pub fn render(&mut self) -> Result<bool, JsValue> {
            self.browser.render()
        }

        pub fn selected_series(&self) -> i32 {
            self.browser.selected_series()
        }

        pub fn selected_source(&self) -> i32 {
            self.browser.selected_source()
        }

        pub fn backend(&self) -> String {
            self.browser.last_diagnostics.as_ref().map_or_else(
                || "webgpu".to_string(),
                |value| value.actual_backend.clone(),
            )
        }

        pub fn readback_bytes(&self) -> u64 {
            self.browser.totals.readback_bytes
        }

        pub fn cpu_frame_upload_bytes(&self) -> u64 {
            0
        }

        pub fn texture_upload_bytes(&self) -> u64 {
            self.browser
                .totals
                .texture_upload_bytes
                .saturating_add(self.browser.totals.presentation_texture_upload_bytes)
        }

        pub fn vertex_upload_bytes(&self) -> u64 {
            self.browser.totals.vertex_upload_bytes
        }

        pub fn index_upload_bytes(&self) -> u64 {
            self.browser.totals.index_upload_bytes
        }

        pub fn surface_presents(&self) -> u64 {
            self.browser.totals.surface_presents
        }

        pub fn needs_recreate(&self) -> bool {
            self.browser.needs_recreate
        }

        pub fn export_png(&self) -> Result<Vec<u8>, JsValue> {
            self.browser.export_png()
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
        }
    }

    /// Worker-owned OffscreenCanvas direct WebGPU adapter.
    #[cfg(feature = "3d-gpu")]
    #[wasm_bindgen(js_name = OffscreenWebGPU3DCanvasSession)]
    pub struct OffscreenWebGpu3DCanvasSession {
        canvas: OffscreenCanvas,
        browser: BrowserWebGpu3DSession,
    }

    #[cfg(feature = "3d-gpu")]
    #[wasm_bindgen]
    impl OffscreenWebGpu3DCanvasSession {
        pub async fn create(
            canvas: OffscreenCanvas,
            plot: &JsPlot3D,
        ) -> Result<OffscreenWebGpu3DCanvasSession, JsValue> {
            ensure_default_browser_fonts()?;
            let mut session = plot.build_session()?;
            session
                .resize(canvas.width().max(1), canvas.height().max(1), 1.0)
                .map_err(js_err)?;
            let instance = webgpu_instance();
            let surface: wgpu::Surface<'static> = instance
                .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas.clone()))
                .map_err(js_err)?;
            let browser = BrowserWebGpu3DSession::new(instance, surface, session).await?;
            let mut adapter = Self { canvas, browser };
            let _ = adapter.browser.render()?;
            Ok(adapter)
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.canvas.set_width(width.max(1));
            self.canvas.set_height(height.max(1));
            self.browser.resize(width, height, scale_factor)
        }

        pub fn pointer_down(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerDown {
                x,
                y,
                button: Browser3DSession::pointer_button(button)?,
            })
        }

        pub fn pointer_move(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerMove { x, y })
        }

        pub fn pointer_up(&mut self, x: f32, y: f32, button: i16) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::PointerUp {
                x,
                y,
                button: Browser3DSession::pointer_button(button)?,
            })
        }

        pub fn double_click(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::DoubleClick {
                x,
                y,
                button: PointerButton3D::Left,
            })
        }

        pub fn wheel(&mut self, delta_y: f32) -> Result<(), JsValue> {
            self.browser
                .apply(InputEvent3D::Wheel { delta_y: -delta_y })
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.apply(InputEvent3D::Escape)
        }

        /// Submit at most one dirty frame.
        pub fn render(&mut self) -> Result<bool, JsValue> {
            self.browser.render()
        }

        pub fn selected_series(&self) -> i32 {
            self.browser.selected_series()
        }

        pub fn selected_source(&self) -> i32 {
            self.browser.selected_source()
        }

        pub fn backend(&self) -> String {
            self.browser.last_diagnostics.as_ref().map_or_else(
                || "webgpu".to_string(),
                |value| value.actual_backend.clone(),
            )
        }

        pub fn readback_bytes(&self) -> u64 {
            self.browser.totals.readback_bytes
        }

        pub fn cpu_frame_upload_bytes(&self) -> u64 {
            0
        }

        pub fn texture_upload_bytes(&self) -> u64 {
            self.browser
                .totals
                .texture_upload_bytes
                .saturating_add(self.browser.totals.presentation_texture_upload_bytes)
        }

        pub fn vertex_upload_bytes(&self) -> u64 {
            self.browser.totals.vertex_upload_bytes
        }

        pub fn index_upload_bytes(&self) -> u64 {
            self.browser.totals.index_upload_bytes
        }

        pub fn surface_presents(&self) -> u64 {
            self.browser.totals.surface_presents
        }

        pub fn needs_recreate(&self) -> bool {
            self.browser.needs_recreate
        }

        pub fn export_png(&self) -> Result<Vec<u8>, JsValue> {
            self.browser.export_png()
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
        }
    }

    #[wasm_bindgen]
    pub struct WebCanvasSession {
        browser: BrowserSession,
        surface: CanvasSurface,
    }

    #[wasm_bindgen]
    impl WebCanvasSession {
        #[wasm_bindgen(constructor)]
        pub fn new(canvas: HtmlCanvasElement) -> Result<WebCanvasSession, JsValue> {
            ensure_default_browser_fonts()?;
            let surface = CanvasSurface::from_html(canvas)?;
            let browser = BrowserSession::new(surface.size_px());
            Ok(Self { browser, surface })
        }

        pub fn has_plot(&self) -> bool {
            self.browser.has_plot()
        }

        pub fn set_plot(&mut self, plot: &JsPlot) -> Result<(), JsValue> {
            self.browser.set_plot(plot.inner.clone());
            self.render()
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.surface.set_size(width, height);
            self.browser.resize(width, height, scale_factor);
            if self.browser.has_plot() {
                self.render()
            } else {
                Ok(())
            }
        }

        pub fn set_backend_preference(&mut self, backend: WebBackendPreference) {
            self.browser.set_backend_preference(backend);
        }

        pub fn set_time(&mut self, time_seconds: f64) -> Result<(), JsValue> {
            self.browser.set_time(time_seconds);
            if self.browser.has_plot() {
                self.render()
            } else {
                Ok(())
            }
        }

        pub fn pointer_down(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            self.browser.pointer_down(x, y, button)
        }

        pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
            self.browser.pointer_move(x, y)?;
            self.render()
        }

        pub fn pointer_up(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            self.browser.pointer_up(x, y, button)?;
            self.render()
        }

        pub fn pointer_leave(&mut self) -> Result<(), JsValue> {
            self.browser.clear_hover()?;
            self.render()
        }

        pub fn wheel(&mut self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
            self.browser.wheel(delta_y, x, y)?;
            self.render()
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.reset_view()?;
            self.render()
        }

        /// Returns the data x-coordinate under a backing-surface pixel.
        pub fn data_x_at(&self, x: f64, y: f64) -> Result<f64, JsValue> {
            Ok(self.browser.data_at(x, y)?.0)
        }

        pub fn render(&mut self) -> Result<(), JsValue> {
            let image = self.browser.render_frame()?;
            self.surface.draw_image(&image)
        }

        pub fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            self.browser.export_png()
        }

        pub fn export_svg(&mut self) -> Result<String, JsValue> {
            self.browser.export_svg()
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
            self.surface.clear();
        }
    }

    #[wasm_bindgen]
    pub struct OffscreenCanvasSession {
        browser: BrowserSession,
        surface: CanvasSurface,
    }

    #[wasm_bindgen]
    impl OffscreenCanvasSession {
        #[wasm_bindgen(constructor)]
        pub fn new(canvas: OffscreenCanvas) -> Result<OffscreenCanvasSession, JsValue> {
            ensure_default_browser_fonts()?;
            let surface = CanvasSurface::from_offscreen(canvas)?;
            let browser = BrowserSession::new(surface.size_px());
            Ok(Self { browser, surface })
        }

        pub fn has_plot(&self) -> bool {
            self.browser.has_plot()
        }

        pub fn set_plot(&mut self, plot: &JsPlot) -> Result<(), JsValue> {
            self.browser.set_plot(plot.inner.clone());
            self.render()
        }

        pub fn resize(
            &mut self,
            width: u32,
            height: u32,
            scale_factor: f32,
        ) -> Result<(), JsValue> {
            self.surface.set_size(width, height);
            self.browser.resize(width, height, scale_factor);
            if self.browser.has_plot() {
                self.render()
            } else {
                Ok(())
            }
        }

        pub fn set_backend_preference(&mut self, backend: WebBackendPreference) {
            self.browser.set_backend_preference(backend);
        }

        pub fn set_time(&mut self, time_seconds: f64) -> Result<(), JsValue> {
            self.browser.set_time(time_seconds);
            if self.browser.has_plot() {
                self.render()
            } else {
                Ok(())
            }
        }

        pub fn pointer_down(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            self.browser.pointer_down(x, y, button)
        }

        pub fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
            self.browser.pointer_move(x, y)?;
            self.render()
        }

        pub fn pointer_up(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            self.browser.pointer_up(x, y, button)?;
            self.render()
        }

        pub fn pointer_leave(&mut self) -> Result<(), JsValue> {
            self.browser.clear_hover()?;
            self.render()
        }

        pub fn wheel(&mut self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
            self.browser.wheel(delta_y, x, y)?;
            self.render()
        }

        pub fn reset_view(&mut self) -> Result<(), JsValue> {
            self.browser.reset_view()?;
            self.render()
        }

        pub fn render(&mut self) -> Result<(), JsValue> {
            let image = self.browser.render_frame()?;
            self.surface.draw_image(&image)
        }

        pub fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            self.browser.export_png()
        }

        pub fn export_svg(&mut self) -> Result<String, JsValue> {
            self.browser.export_svg()
        }

        pub fn destroy(&mut self) {
            self.browser.destroy();
            self.surface.clear();
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub const RUVIZ_WEB_TARGET_NOTE: &str =
    "ruviz-web only provides browser bindings on wasm32-unknown-unknown targets";
