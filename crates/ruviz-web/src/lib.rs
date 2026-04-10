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
//! `packages/ruviz-web/README.md`, which wraps these bindings with a higher
//! level `createPlot()` builder and session APIs.
//!
//! If you do need the raw bridge, compile the crate for `wasm32-unknown-unknown`
//! and consume the generated bindings via wasm-bindgen or wasm-pack.
//!
//! # Documentation
//!
//! - Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
//! - npm package docs: <https://github.com/Ameyanagi/ruviz/tree/main/packages/ruviz-web>
//! - Release notes: <https://github.com/Ameyanagi/ruviz/tree/main/docs/releases>

#![allow(clippy::needless_pass_by_value)]

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{mem, sync::OnceLock};

    use js_sys::Reflect;
    use ruviz::{
        core::{
            Image, ImageTarget, InteractivePlotSession, IntoPlot, Plot, PlotInputEvent,
            SurfaceTarget, ViewportPoint, ViewportRect,
        },
        data::{Observable, Signal},
        render::register_font_bytes,
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

        pub fn clone_handle(&self) -> SignalVecF64 {
            Self {
                inner: self.inner.clone(),
                len: self.len,
            }
        }
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

    #[wasm_bindgen]
    impl JsPlot {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Self {
            Self { inner: Plot::new() }
        }

        pub fn line(&mut self, x: Vec<f64>, y: Vec<f64>) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str("x and y must have the same length"));
            }

            self.replace_with_series(|plot| plot.line(&x, &y).into_plot());
            Ok(())
        }

        pub fn scatter(&mut self, x: Vec<f64>, y: Vec<f64>) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str("x and y must have the same length"));
            }

            self.replace_with_series(|plot| plot.scatter(&x, &y).into_plot());
            Ok(())
        }

        pub fn bar(&mut self, categories: Vec<String>, values: Vec<f64>) -> Result<(), JsValue> {
            if categories.len() != values.len() {
                return Err(JsValue::from_str(
                    "bar categories and values must have the same length",
                ));
            }

            self.replace_with_series(|plot| plot.bar(&categories, &values).into_plot());
            Ok(())
        }

        pub fn bar_observable(
            &mut self,
            categories: Vec<String>,
            values: &ObservableVecF64,
        ) -> Result<(), JsValue> {
            if categories.len() != values.len() {
                return Err(JsValue::from_str(
                    "bar categories and observable values must have the same length",
                ));
            }

            let value_source = values.inner.clone();
            self.replace_with_series(|plot| plot.bar_source(&categories, value_source).into_plot());
            Ok(())
        }

        pub fn histogram(&mut self, data: Vec<f64>) {
            self.replace_with_series(|plot| plot.histogram(&data, None).into_plot());
        }

        pub fn histogram_observable(&mut self, data: &ObservableVecF64) {
            let data_source = data.inner.clone();
            self.replace_with_series(|plot| plot.histogram_source(data_source, None).into_plot());
        }

        pub fn boxplot(&mut self, data: Vec<f64>) {
            self.replace_with_series(|plot| plot.boxplot(&data, None).into_plot());
        }

        pub fn boxplot_observable(&mut self, data: &ObservableVecF64) {
            let data_source = data.inner.clone();
            self.replace_with_series(|plot| plot.boxplot_source(data_source, None).into_plot());
        }

        pub fn heatmap(
            &mut self,
            values: Vec<f64>,
            rows: usize,
            cols: usize,
        ) -> Result<(), JsValue> {
            let matrix = Self::flatten_grid(values, rows, cols)?;
            self.replace_with_series(|plot| plot.heatmap(&matrix, None).into_plot());
            Ok(())
        }

        pub fn error_bars(
            &mut self,
            x: Vec<f64>,
            y: Vec<f64>,
            y_errors: Vec<f64>,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), y_errors.len()],
                "x, y, and y_errors must have the same length",
            )?;

            self.replace_with_series(|plot| plot.error_bars(&x, &y, &y_errors).into_plot());
            Ok(())
        }

        pub fn error_bars_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            y_errors: &ObservableVecF64,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), y_errors.len()],
                "observable x, y, and y_errors must have the same length",
            )?;

            self.replace_with_series(|plot| {
                plot.error_bars_source(x.inner.clone(), y.inner.clone(), y_errors.inner.clone())
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
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "x, y, x_errors, and y_errors must have the same length",
            )?;

            self.replace_with_series(|plot| {
                plot.error_bars_xy(&x, &y, &x_errors, &y_errors).into_plot()
            });
            Ok(())
        }

        pub fn error_bars_xy_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
            x_errors: &ObservableVecF64,
            y_errors: &ObservableVecF64,
        ) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "observable x, y, x_errors, and y_errors must have the same length",
            )?;

            self.replace_with_series(|plot| {
                plot.error_bars_xy_source(
                    x.inner.clone(),
                    y.inner.clone(),
                    x_errors.inner.clone(),
                    y_errors.inner.clone(),
                )
                .into_plot()
            });
            Ok(())
        }

        pub fn kde(&mut self, data: Vec<f64>) {
            self.replace_with_series(|plot| plot.kde(&data).into_plot());
        }

        pub fn ecdf(&mut self, data: Vec<f64>) {
            self.replace_with_series(|plot| plot.ecdf(&data).into_plot());
        }

        pub fn contour(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> Result<(), JsValue> {
            if x.is_empty() || y.is_empty() {
                return Err(JsValue::from_str("contour x and y must not be empty"));
            }

            if z.len() != x.len().saturating_mul(y.len()) {
                return Err(JsValue::from_str(
                    "contour z length must equal x.len() * y.len()",
                ));
            }

            self.replace_with_series(|plot| plot.contour(&x, &y, &z).into_plot());
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
            if series_values.is_empty() || series_values.len() % points_per_series != 0 {
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

        pub fn violin(&mut self, data: Vec<f64>) {
            self.replace_with_series(|plot| plot.violin(&data).into_plot());
        }

        pub fn polar_line(&mut self, r: Vec<f64>, theta: Vec<f64>) -> Result<(), JsValue> {
            Self::validate_equal_lengths(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;

            self.replace_with_series(|plot| plot.polar_line(&r, &theta).into_plot());
            Ok(())
        }

        pub fn line_signal(&mut self, x: Vec<f64>, y: &SignalVecF64) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "signal y data and x must have the same length",
                ));
            }

            let y_signal = y.inner.clone();
            self.replace_with_series(|plot| plot.line_source(x, y_signal).into_plot());
            Ok(())
        }

        pub fn scatter_signal(&mut self, x: Vec<f64>, y: &SignalVecF64) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "signal y data and x must have the same length",
                ));
            }

            let y_signal = y.inner.clone();
            self.replace_with_series(|plot| plot.scatter_source(x, y_signal).into_plot());
            Ok(())
        }

        pub fn line_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "observable x and y must have the same length",
                ));
            }

            self.replace_with_series(|plot| {
                plot.line_source(x.inner.clone(), y.inner.clone())
                    .into_plot()
            });
            Ok(())
        }

        pub fn scatter_observable(
            &mut self,
            x: &ObservableVecF64,
            y: &ObservableVecF64,
        ) -> Result<(), JsValue> {
            if x.len() != y.len() {
                return Err(JsValue::from_str(
                    "observable x and y must have the same length",
                ));
            }

            self.replace_with_series(|plot| {
                plot.scatter_source(x.inner.clone(), y.inner.clone())
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

        pub fn ticks(&mut self, enabled: bool) {
            self.update_plot(|plot| plot.ticks(enabled));
        }

        pub fn theme_light(&mut self) {
            self.update_plot(|plot| plot.theme(ruviz::render::Theme::light()));
        }

        pub fn theme_dark(&mut self) {
            self.update_plot(|plot| plot.theme(ruviz::render::Theme::dark()));
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
