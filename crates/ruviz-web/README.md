# ruviz-web

`ruviz-web` is the browser runtime adapter for `ruviz`.

It exposes:

- `WebCanvasSession` for main-thread HTML canvas rendering
- `OffscreenCanvasSession` for worker-driven `OffscreenCanvas` rendering
- `JsPlot::render_png_bytes()` and `JsPlot::render_svg()` for direct wasm export without a canvas session
- `ObservableVecF64` for narrow JS-side reactive numeric demos
- `web_runtime_capabilities()` for browser feature diagnostics
- `register_font_bytes_js()` and `register_default_browser_fonts_js()` for text setup

## Scope

This crate currently focuses on browser canvas interactivity over the CPU image path.
`WebBackendPreference::Gpu` is accepted as a preference flag, but there is not yet a
dedicated WebGPU canvas fast path.

## Browser Text

The crate ships a built-in Noto Sans fallback for browser text rendering and registers it
automatically when a canvas session is created. You can still register your own font bytes
before rendering if you need a different face.

`web_runtime_capabilities()` also reports touch input, worker support, OffscreenCanvas support,
and whether a dedicated GPU canvas path is currently available.

Bundled font asset:

- `assets/NotoSans-Regular.ttf`
- License: `assets/NotoSans-LICENSE.txt`

## Minimal JS Example

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
