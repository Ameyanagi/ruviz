# Choose the API for your application

Start with a plot builder, then either export it or mount an interactive session.
Use the frontend's public package; renderer internals and memory utilities are
advanced integration points.

| Task | Rust | Python | Browser JS/TS | Native GUI |
| --- | --- | --- | --- | --- |
| Create a 2D plot | `Plot::new().line(&x, &y)` | `ruviz.plot().line(x, y)` | `createPlot().line({ x, y })` | Pass a Rust builder to the adapter |
| Create a 3D plot | `surface(...)`, `line3d(...)`, `scatter3d(...)` | `ruviz.surface(...)`, `ruviz.line3d(...)`, `ruviz.scatter3d(...)` | `surface(...)`, `line3d(...)`, `scatter3d(...)` from `ruviz/3d` | Pass a Rust 3D builder to the adapter |
| Combine 3D series | Chain series methods | Chain series methods | Chain series methods; `clearSeries()` explicitly replaces | Uses the core builder |
| Molecular spheres | `spheres3d(&atoms)` with `3d` | No dedicated binding yet | No dedicated binding yet | Available through the Rust builder; GPUI has a shading toggle |
| Stable 3D rotation | `.axis_aspect(...).stable_scale(true)` | No dedicated binding yet | `.axisAspect(x, y, z).stableScale()` or `.equalScale().stableScale()` | Uses the core builder |
| Hide 3D axes | `.axes(false)` retains an orientation cue | No dedicated binding yet | No dedicated binding yet | Uses the core builder |
| Export | `.save(path)` or `.render()` | `.save(path)` | `.save(...)` for 2D; `session.exportPng()` for mounted 3D | Uses the core export paths |
| Update a mounted view | Retained interactive session | Widget/session APIs | `session.setPlot(...)` for 2D; dispose and mount for new 3D geometry | Adapter methods retain core interaction state |

For ordinary Rust use, import `ruviz::prelude::*`. For the web, import from
`ruviz` or `ruviz/3d`; use `ruviz/raw` only when implementing an adapter or a
rendering benchmark. See the [molecular guide](13_molecular_views.md),
[Python guide](../../bindings/python/docs/index.md), and
[web SDK guide](../../packages/ruviz/README.md) for the supported options.

In browser code, use `detach()` to clear a 2D plot while keeping listeners for a
later `setPlot()`. Use `dispose()` when a session leaves your application.
`destroy()` keeps its historical meaning for compatibility and is not needed in
new code. For 3D render failures, inspect `session.error`, retry with `render()`,
and remount only when `needsRecreate()` reports a lost surface or device.
