# ruviz-web

`ruviz-web` is the low-level Rust WebAssembly bridge for `ruviz`.

Use this crate when you need the raw wasm-bindgen layer from Rust. If you are
building a browser app in JavaScript or TypeScript, use the npm package
`ruviz` instead. If you are building a native Rust application, use the root
`ruviz` crate.

## What This Crate Provides

- `JsPlot` for static PNG and SVG export from WebAssembly
- `WebCanvasSession` for main-thread interactive canvas rendering
- browser capability probing through `web_runtime_capabilities()`
- numeric observable and sine-signal wrappers used by higher-level web runtimes
- browser font registration helpers

## Quick Start

Add the bridge crate to your Cargo manifest:

```toml
[dependencies]
ruviz-web = "0.4.0"
```

Build for `wasm32-unknown-unknown` and generate bindings with your preferred
wasm-bindgen or wasm-pack workflow.

## When To Use This Crate

Use `ruviz-web` if you are:

- embedding the wasm bridge directly from Rust
- working on the raw browser adapter
- debugging behavior below the npm SDK layer

Use the npm package `ruviz` if you want:

- `createPlot()` and session builders
- worker-session fallback logic
- browser-friendly ESM packaging
- published npm-facing docs and examples

## Browser Text

The crate ships with a bundled Noto Sans fallback for browser-hosted text and
registers it automatically for the browser runtime. You can still register your
own font bytes before rendering when you need a different face.

## Related Docs

- Root crate docs: <https://docs.rs/ruviz>
- Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
- npm package docs: <https://github.com/Ameyanagi/ruviz/tree/main/packages/ruviz-web>
- Release notes: <https://github.com/Ameyanagi/ruviz/tree/main/docs/releases>
