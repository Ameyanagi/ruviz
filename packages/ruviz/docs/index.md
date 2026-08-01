# ruviz for Web

`ruviz` is the browser-first JS/TS SDK for the Rust plotting runtime.

It combines:

- a fluent `createPlot()` builder
- direct PNG and SVG export
- interactive main-thread canvas sessions
- worker-backed canvas sessions with fallback
- reactive helpers for observable and time-based demo data
- serializable plot snapshots for worker transfer and rehydration

## Install

```sh
npm install ruviz
```

The package exports the high-level SDK from `ruviz` and the raw wasm-bindgen
bridge from `ruviz/raw`.

## Start Here

- **Getting Started** covers static export and the first canvas session.
- **Interactivity** explains session modes, resize/input wiring, and workers.
- **Gallery** points at the example-backed docs pages.
- **API** contains the generated TypeDoc reference for the published package.
