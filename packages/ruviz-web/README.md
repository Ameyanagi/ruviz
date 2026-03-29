# ruviz-web

`ruviz-web` is the public browser-first JS/TS SDK for `ruviz`.

It wraps the low-level wasm bridge in [`crates/ruviz-web`](../../crates/ruviz-web) with:

- camelCase factories and methods
- auto-resize and pointer/wheel wiring for canvas sessions
- worker-session fallback to the main thread when `OffscreenCanvas` is unavailable
- direct PNG/SVG export from plot builders
- Observable and signal helpers for reactive demos
- a raw escape hatch at `ruviz-web/raw`

## Install

This repo uses Bun as the JS package manager and task runner:

```sh
bun install
bun run build:web
```

## Public API

```ts
import {
  createCanvasSession,
  createPlot,
  createWorkerSession,
  getRuntimeCapabilities,
} from "ruviz-web";
```

Main exports:

- `createPlot()`
- `createCanvasSession(canvas, options?)`
- `createWorkerSession(canvas, options?)`
- `getRuntimeCapabilities()`
- `registerFont(bytes)`
- `createObservable(values)`
- `createSineSignal(options)`

## Raw Bindings

Advanced users can access the raw wasm-bindgen layer directly:

```ts
import initRaw, { JsPlot, WebCanvasSession } from "ruviz-web/raw";
```

That subpath is intentionally close to the Rust naming and runtime model. The package root is the
stable JS/TS-facing API.
