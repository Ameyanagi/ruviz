# Getting Started

## Static Export

The simplest path is to build a plot and save it directly:

```ts
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

- `await plot.renderPng()` to get PNG bytes
- `await plot.renderSvg()` to get SVG markup

## First Interactive Session

Mount the same plot into a canvas:

```ts
const canvas = document.querySelector("canvas")!;
const session = await plot.mount(canvas);
```

That sets the plot, sizes the session, and renders immediately.

## Explicit Session Constructors

Use the direct constructors when you want tighter lifecycle control:

```ts
import { createCanvasSession, createPlot } from "ruviz";

const session = await createCanvasSession(canvas, {
  autoResize: true,
  bindInput: true,
});

await session.setPlot(createPlot().line({ x: [0, 1], y: [0, 1] }));
session.render();
```

## Runtime Notes

- The package is ESM-first.
- WebAssembly and worker assets are resolved through module URLs.
- Use `createWorkerSession(...)` when you want OffscreenCanvas worker rendering.
