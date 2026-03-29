# ruviz-web

`ruviz-web` is the low-level Rust wasm bridge for `ruviz`.

If you are building a browser app in JavaScript or TypeScript, use the npm package in
[`packages/ruviz-web`](../../packages/ruviz-web) instead of importing these raw wasm-bindgen
bindings directly.

It exposes:

- `WebCanvasSession` for main-thread HTML canvas rendering
- `OffscreenCanvasSession` for worker-driven `OffscreenCanvas` rendering
- `JsPlot::render_png_bytes()` and `JsPlot::render_svg()` for direct wasm export without a canvas session
- `SignalVecF64` for time-varying `Signal<Vec<f64>>` demo and playback use cases
- `ObservableVecF64` for narrow JS-side reactive numeric demos
- `web_runtime_capabilities()` for browser feature diagnostics
- `register_font_bytes_js()` and `register_default_browser_fonts_js()` for text setup

## Scope

This crate focuses on browser canvas interactivity over the CPU image path and stays close to
the Rust-side browser/runtime model. `WebBackendPreference::Gpu` is accepted as a preference
flag, but there is not yet a dedicated WebGPU canvas fast path.

## Browser Text

The crate ships a built-in Noto Sans fallback for browser text rendering and registers it
automatically when a canvas session is created. You can still register your own font bytes
before rendering if you need a different face.

`web_runtime_capabilities()` also reports touch input, worker support, OffscreenCanvas support,
and whether a dedicated GPU canvas path is currently available.

The web demo covers direct export, main-thread sessions, worker sessions, temporal signal
playback, and observable updates.

Bundled font asset:

- `assets/NotoSans-Regular.ttf`
- License: `assets/NotoSans-LICENSE.txt`

## Raw JS Example

```js
import init, { JsPlot, WebCanvasSession } from "./pkg/ruviz_web.js";

await init();

const plot = new JsPlot();
plot.size_px(640, 360);
plot.xlabel("x");
plot.ylabel("y");
plot.line([0, 1, 2], [0, 1, 0]);

const canvas = document.querySelector("canvas");
const session = new WebCanvasSession(canvas);
session.set_plot(plot);
```

For the higher-level JS/TS API with camelCase methods, worker fallback, auto-resize, and Bun
workspace packaging, see [`packages/ruviz-web`](../../packages/ruviz-web).
