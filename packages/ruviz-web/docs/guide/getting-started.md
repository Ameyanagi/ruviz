# Getting Started

## Static Export

The simplest path is to build a plot and save it directly:

```ts,check
import { createPlot } from "ruviz";

const plot = createPlot()
  .sizePx(800, 420)
  .title("Quadratic")
  .setXLabel("x")
  .setYLabel("y = x^2")
  .line({ x: [0, 1, 2, 3], y: [0, 1, 4, 9] });

await plot.save({ format: "png", fileName: "quadratic.png" });
```

You can also call:

- `await plot.renderPng()` to get PNG bytes as a `Uint8Array`
- `await plot.renderSvg()` to get SVG markup

`save()` triggers a browser download. Use `renderPng()` or `renderSvg()` when
you want bytes or markup for your own storage flow.

## First Interactive Session

Mount the same plot into a canvas:

```ts,check
import { createPlot } from "ruviz";

const canvas = document.querySelector<HTMLCanvasElement>("canvas");
if (!canvas) {
  throw new Error("Expected a <canvas> element");
}

const plot = createPlot()
  .sizePx(800, 420)
  .title("Interactive quadratic")
  .line({ x: [0, 1, 2, 3], y: [0, 1, 4, 9] });
const session = await plot.mount(canvas);
```

That sets the plot, sizes the session, and renders immediately.

## Explicit Session Constructors

Use the direct constructors when you want tighter lifecycle control:

```ts,check
import { createCanvasSession, createPlot } from "ruviz";

const canvas = document.querySelector<HTMLCanvasElement>("canvas");
if (!canvas) {
  throw new Error("Expected a <canvas> element");
}

const session = await createCanvasSession(canvas, {
  autoResize: true,
  bindInput: true,
});

await session.setPlot(createPlot().line({ x: [0, 1], y: [0, 1] }));
session.render();
```

The constructor options are:

- `backendPreference: "auto" | "cpu" | "svg" | "gpu"`
- `autoResize`, default `true`
- `bindInput`, default `true`
- `initialTime`, for signal-backed plots

## Runtime Notes

- The package is ESM-first.
- WebAssembly and worker assets are resolved through package-relative module URLs.
- Use `createWorkerSession(...)` when you want OffscreenCanvas worker rendering.
- Use `import ... from "ruviz/raw"` only when you need the low-level wasm-bindgen API.
