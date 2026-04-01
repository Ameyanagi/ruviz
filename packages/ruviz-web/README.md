# ruviz

`ruviz` is the public browser-first JS/TS SDK for `ruviz`.

It wraps the low-level wasm bridge in [`crates/ruviz-web`](../../crates/ruviz-web) with:

- a fluent plot builder such as `createPlot().line(...).title(...).save()`
- backwards-compatible camelCase factories and methods
- auto-resize and pointer/wheel wiring for canvas sessions
- worker-session fallback to the main thread when `OffscreenCanvas` is unavailable
- direct PNG/SVG export from plot builders
- Observable and signal helpers for reactive demos
- snapshot rehydration with `createPlotFromSnapshot(...)`
- a raw escape hatch at `ruviz/raw`

## Install

This repo uses Bun as the JS package manager and task runner:

```sh
bun install
bun run build:web
```

## Examples and Docs

The canonical TS examples live in [`examples/`](examples) and feed the VitePress gallery.

Serve the package docs locally:

```sh
bun install
bun run --cwd packages/ruviz-web docs:dev
```

## Public API

```ts
import {
  createCanvasSession,
  createPlot,
  createPlotFromSnapshot,
  createWorkerSession,
  getRuntimeCapabilities,
} from "ruviz";
```

Main exports:

- `createPlot()`
- `createPlotFromSnapshot(snapshot)`
- `createCanvasSession(canvas, options?)`
- `createWorkerSession(canvas, options?)`
- `getRuntimeCapabilities()`
- `registerFont(bytes)`
- `createObservable(values)`
- `createSineSignal(options)`

Example:

```ts
const plot = createPlot()
  .line({ x: [0, 1, 2], y: [0, 1, 4] })
  .title("Quadratic")
  .xlabel("x")
  .ylabel("y");

await plot.save({ format: "png", fileName: "quadratic.png" });
```

## Raw Bindings

Advanced users can access the raw wasm-bindgen layer directly:

```ts
import initRaw, { JsPlot, WebCanvasSession } from "ruviz/raw";
```

That subpath is intentionally close to the Rust naming and runtime model. The package root is the
stable JS/TS-facing API.
