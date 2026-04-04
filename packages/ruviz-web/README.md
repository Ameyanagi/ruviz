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

The package ships as ESM and expects a bundler or runtime that can load Web
Workers and WebAssembly from standard module URLs.

## Quick Start

```ts
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

```ts
import { createCanvasSession, createPlot } from "ruviz";

const canvas = document.querySelector("canvas")!;
const session = await createCanvasSession(canvas);
await session.setPlot(
  createPlot().line({ x: [0, 1, 2], y: [0, 1, 0] }).title("Interactive Plot"),
);
session.render();
```

Use `createWorkerSession(...)` when you want OffscreenCanvas worker rendering
with main-thread fallback support.

## Reactive Helpers

The package also exports:

- `createObservable(...)` for mutable numeric series
- `createSineSignal(...)` for time-varying demo inputs
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

```sh
bun install
bun run --cwd packages/ruviz-web build
bun run --cwd packages/ruviz-web docs:dev
```
