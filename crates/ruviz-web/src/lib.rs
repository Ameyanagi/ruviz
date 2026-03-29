#![allow(clippy::needless_pass_by_value)]

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{mem, sync::OnceLock};

    use js_sys::Reflect;
    use ruviz::{
        core::{
            Image, ImageTarget, InteractivePlotSession, IntoPlot, Plot, PlotInputEvent,
            ViewportPoint, ViewportRect,
        },
        data::Observable,
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
        }

        fn set_backend_preference(&mut self, backend: WebBackendPreference) {
            self.backend = backend;
            if let Some(session) = &self.session {
                session.set_prefer_gpu(matches!(backend, WebBackendPreference::Gpu));
            }
        }

        fn set_time(&mut self, time_seconds: f64) {
            self.time_seconds = time_seconds;
            if let Some(session) = &self.session {
                session.apply_input(PlotInputEvent::SetTime { time_seconds });
            }
        }

        fn render_frame(&mut self) -> Result<Image, JsValue> {
            let frame = self
                .session()?
                .render_to_image(ImageTarget {
                    size_px: self.size_px,
                    scale_factor: self.scale_factor,
                    time_seconds: self.time_seconds,
                })
                .map_err(js_err)?;
            Ok((*frame.image).clone())
        }

        fn export_png(&mut self) -> Result<Vec<u8>, JsValue> {
            self.render_frame()?.encode_png().map_err(js_err)
        }

        fn export_svg(&self) -> Result<String, JsValue> {
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

            plot.render_to_svg().map_err(js_err)
        }

        fn reset_view(&mut self) -> Result<(), JsValue> {
            self.session()?.apply_input(PlotInputEvent::ResetView);
            self.drag = None;
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
                }
                _ => {}
            }
            Ok(())
        }

        fn pointer_move(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
            let point = ViewportPoint::new(x, y);
            let session = self.session()?;

            match self.drag {
                Some(DragMode::Pan {
                    mut last,
                    moved: _moved,
                }) => {
                    let delta = ViewportPoint::new(point.x - last.x, point.y - last.y);
                    if delta.x != 0.0 || delta.y != 0.0 {
                        session.apply_input(PlotInputEvent::Pan { delta_px: delta });
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
                }
                None => {
                    session.apply_input(PlotInputEvent::Hover { position_px: point });
                }
            }

            Ok(())
        }

        fn pointer_up(&mut self, x: f64, y: f64, button: i16) -> Result<(), JsValue> {
            let point = ViewportPoint::new(x, y);
            let drag = self.drag.take();
            let session = self.session()?;

            match (drag, button) {
                (Some(DragMode::Pan { moved, .. }), 0) => {
                    if !moved {
                        session.apply_input(PlotInputEvent::SelectAt { position_px: point });
                    }
                }
                (Some(DragMode::ZoomRect { anchor, moved }), 2) => {
                    session.apply_input(PlotInputEvent::BrushEnd { position_px: point });
                    if moved {
                        session.apply_input(PlotInputEvent::ZoomRect {
                            region_px: ViewportRect::from_points(anchor, point),
                        });
                    }
                }
                _ => {}
            }

            Ok(())
        }

        fn clear_hover(&mut self) -> Result<(), JsValue> {
            self.session()?.apply_input(PlotInputEvent::ClearHover);
            Ok(())
        }

        fn wheel(&mut self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
            let steps = (delta_y / 120.0).clamp(-10.0, 10.0);
            let factor = 1.1_f64.powf(-steps);
            self.session()?.apply_input(PlotInputEvent::Zoom {
                factor,
                center_px: ViewportPoint::new(x, y),
            });
            Ok(())
        }
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

        pub fn export_svg(&self) -> Result<String, JsValue> {
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

        pub fn export_svg(&self) -> Result<String, JsValue> {
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
