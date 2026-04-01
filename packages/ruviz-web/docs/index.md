# ruviz for Web

`ruviz` is the browser-first JS/TS SDK for the Rust `ruviz` rendering engine.

## Highlights

- Fluent plot builder with `createPlot().line(...).title(...)`
- Direct PNG and SVG export from the same builder used for interactive rendering
- Main-thread and worker-backed canvas sessions
- Observable and temporal signal helpers for reactive plots
- Low-level raw WASM escape hatch at `ruviz/raw`

## Install

```sh
bun install
bun run --cwd packages/ruviz-web build
```

Use the guide for API entrypoints, the gallery for live examples, and the API section for the
generated TypeDoc reference.
