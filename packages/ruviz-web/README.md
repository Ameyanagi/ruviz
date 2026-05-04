# ruviz

`ruviz` is the published browser-first JS/TS SDK for the `ruviz` plotting
runtime.

It wraps the raw wasm-bindgen bridge in the Rust crate `ruviz-web` with a
higher-level API for static export, interactive canvas sessions, worker-backed
rendering, and reactive demos.

## Install

```sh
npm install ruviz
```

The package ships as ESM. Browser apps should use a bundler or runtime that can
load WebAssembly and module workers from package-relative `import.meta.url`
references.

The public package entrypoints are:

```ts,check
import { createPlot, createWorkerSession } from "ruviz";
import initRaw, { JsPlot } from "ruviz/raw";

await initRaw();
const rawPlot = new JsPlot();
```

## Quick Start

```ts,check
import { createPlot } from "ruviz";

const plot = createPlot()
  .line({ x: [0, 1, 2, 3], y: [0, 1, 4, 9] })
  .title("Quadratic")
  .setXLabel("x")
  .setYLabel("y = x^2");

await plot.save({ format: "png", fileName: "quadratic.png" });
```

## Interactive Canvas Sessions

Mount a plot into a normal HTML canvas:

```ts,check
import { createCanvasSession, createPlot } from "ruviz";

const canvas = document.querySelector("canvas")!;
const session = await createCanvasSession(canvas);
await session.setPlot(
  createPlot().line({ x: [0, 1, 2], y: [0, 1, 0] }).title("Interactive Plot"),
);
session.render();
```

Use `createWorkerSession(...)` when you want OffscreenCanvas worker rendering:

```ts,check
import { createPlot, createWorkerSession } from "ruviz";

const canvas = document.querySelector("canvas")!;
const session = await createWorkerSession(canvas, {
  backendPreference: "auto",
  fallbackToMainThread: true,
});
await session.setPlot(createPlot().scatter({ x: [0, 1, 2], y: [2, 1, 3] }));
session.render();
```

When `Worker`, `OffscreenCanvas`, or `transferControlToOffscreen()` is missing,
`createWorkerSession()` returns a `WorkerSession` in `mode === "main-thread"` by
default. Pass `{ fallbackToMainThread: false }` to throw instead.

Both `createCanvasSession()` and `createWorkerSession()` default to
`autoResize: true` and `bindInput: true`, so pointer, wheel, and resize events
are wired automatically unless you disable them.

## Plot Types

The fluent builder currently supports:

- `line({ x, y })` and `scatter({ x, y })`
- `bar({ categories, values })`
- `histogram(values)` and `boxplot(values)`
- `heatmap(rows)` where `rows` is a rectangular `number[][]`
- `errorBars({ x, y, yErrors })`
- `errorBarsXY({ x, y, xErrors, yErrors })`
- `kde(values)` and `ecdf(values)`
- `contour({ x, y, z })` where `z.length === x.length * y.length`
- `pie(values, labels?)`
- `radar({ labels, series })`
- `violin(values)`
- `polarLine({ r, theta })`

`number[]`, `Float64Array`, and other `ArrayLike<number>` values are accepted
for numeric inputs. `createObservable(...)` can drive line, scatter, bar,
histogram, boxplot, and error-bar inputs. `createSineSignal(...)` can be used as
the `y` input for line and scatter plots.

## Reactive Helpers

The package also exports:

- `createObservable(...)` for mutable numeric series
- `createSineSignal(...)` for time-varying demo inputs
- `createPlotFromSnapshot(...)` for rehydrating `plot.toSnapshot()` output
- `getRuntimeCapabilities()` for browser capability checks
- `registerFont(...)` for custom browser text faces

## Which Package Should You Use?

- Use npm `ruviz` for browser apps in JS/TS.
- Use the Rust crate `ruviz-web` if you need the raw wasm bridge.
- Use the root Rust crate `ruviz` for native Rust plotting.

## Docs and Examples

- VitePress docs source: `packages/ruviz-web/docs/`
- TypeScript examples: `packages/ruviz-web/examples/`
- Raw Rust bridge: <https://github.com/Ameyanagi/ruviz/tree/main/crates/ruviz-web>
- Root project README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>

## Local Development

From the repository root:

```sh
bun install
bun run --cwd packages/ruviz-web build:js
bun run --cwd packages/ruviz-web build
bun run --cwd packages/ruviz-web docs:dev
```

`build:js` runs the TypeScript build against existing generated wasm bindings.
`build` also rebuilds the wasm package before compiling TypeScript.
