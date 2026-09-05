# UI, UX, API, and workflow review — 2026-09-05

Reviewed the repository's major subsystems: public Rust builders, 2D and 3D
rendering/layout, data and reactive updates, export/animation, Python and wasm
bindings, JS sessions, native adapter interfaces, examples, documentation, and
CI/release tooling. Also opened the existing built 2D and 3D web demos in Edge
and inspected the new sphere exports and native GPUI example. This is an architectural and workflow
review, not a claim that every branch of every source file was audited.

The strongest direction is to keep the builder → session → render/export flow,
reuse core interaction behavior in each frontend, and remove repeated policies
as affected code is changed. A wholesale rewrite would add migration work and
make rendering regressions harder to isolate.

## Implemented improvements

| Area | Original finding | Result |
| --- | --- | --- |
| Web 3D composition | A second series silently replaced the first. | The wasm bridge retains core builders, and the SDK appends series. `clearSeries()` makes replacement explicit. Browser tests verify that both series remain and that changing a builder does not change a mounted scene. |
| Scheduled rendering | Animation-frame failures escaped the calling code. | `onError` and `session.error` expose scheduled render/input failures. A successful retry clears the error; device loss remains an explicit dispose/remount operation. Failed mount setup releases resources. |
| Demo maintenance | The main demo duplicated raw input and frame scheduling. | Both 3D demo renderers use `ruviz/3d`; input coalescing lives in the SDK. The raw scheduler remains in a separately named benchmark fixture and retains its zero-readback regression test. |
| Accessible interaction | Canvases were unnamed and several actions required pointer gestures. | Named canvases and descriptions, status announcements, keyboard zoom/reset/rotation buttons, and native HTML series checkboxes. Detached 2D controls are disabled until reattachment. |
| Demo workflow | Implementation details dominated the introductory panels. | Task-focused sections for exploring data, playback, live updates, and export; diagnostics are disclosed separately. Small public-API examples accompany the plots. |
| Molecular presentation | Foreground axes crossed atoms and paths. | `.axes(false)` removes panes, grid, box, ticks, and axis labels. A compact orientation cue remains with title/legend support. Both molecular examples use it. |
| Orbit stability | The camera and label layout refitted after each rotation, making the scene change size. | `stable_scale(true)` fixes the projection and viewport through orbit. Spheres enable it by default. Fixed X:Y:Z ratios and equal data-unit scaling remain distinct, with matching web builder controls. |
| Bar preparation | Raster, SVG, and legend occupancy repeated geometry/style preparation. | A shared `BarBatch` prepares rectangles and edge style through one path. Existing prepared-plot caches retain bar batches. Broader data-bounds and picking policies remain in their existing owners. |
| Local validation | Root commands did not cover the adapter workspaces. | `CONTRIBUTING.md` explains the three workspaces and gives checks by changed area. `make fmt` covers all three, with explicit `clippy-gpui` and `clippy-gui` targets. SDK behavior tests run in `check:web`. |
| Session lifecycle | `destroy()` meant different things in 2D and 3D. | New examples use `detach()` to clear a 2D plot and `dispose()` to remove bindings or a worker. Historical `destroy()` behavior remains as a documented compatibility alias. |
| API onboarding | Newcomers could not easily compare frontend capabilities. | The [API map](guide/14_api_map.md) leads with builder → session/export and documents binding differences. Advanced APIs remain available; no broad public API removal was needed. |

Source entry points: [web SDK](../packages/ruviz/src/3d.ts),
[wasm bridge](../bindings/wasm/src/lib.rs),
[demo](../apps/web-demo/src/3d.js),
[bar batches](../src/core/plot/raster_batches.rs), and
[contribution workflow](../CONTRIBUTING.md).

## Preserve these strengths

- `TryIntoPlot3DSession` lets a new builder reach native adapters with little
  adapter-specific code. The sphere feature uses that existing boundary.
- Frame stamps and latest-request-wins rendering avoid presenting stale images
  after orbit, resize, or replacement. Appearance-only changes should preserve
  interaction state while superseding the old rendered frame.
- Measured text layout, shared legend layout, explicit coordinate units, and
  CPU/GPU parity tests are more valuable than extra styling aliases.
- Separate workspaces, locked dependencies, package verification, and the
  explicit CI test-coverage guard should remain. Simplify the entry commands
  rather than removing those checks.
- Keep full geometry as the default. Sampling or a faster drag mode must be an
  explicit user decision, especially for scientific cluster membership.

## Scope of this implementation

Issue [#182](https://github.com/Ameyanagi/ruviz/issues/182) adds one sphere data
type, one builder, data-unit axis aspect, and one shading mutation on the core
session and GPUI view. It reuses the existing camera, export, worker, cache, and
depth-buffer paths. The sphere renderer adds no dependencies. The ordinary scatter shader
is unchanged, and sphere GPU pipelines are created only for sphere scenes.

The feature includes physical radii, stable picking IDs, ambient/diffuse lighting
and optional gloss, faded spheres, matching software/export output, examples,
and a [release benchmark](benchmarks/spheres-2026-09-05.md). Transparent intersecting
surfaces/polyhedra, cylindrical bonds, order-independent transparency, and
Python/JS sphere bindings remain explicit limitations, described in the
[molecular guide](guide/13_molecular_views.md).

Native visual verification also uncovered and fixed a shared macOS example
configuration bug: `gpui_macos` lacked its `font-kit` feature, so it selected
`NoopTextSystem` and left all host labels invisible. The adapter's development
dependency now enables that existing font backend; the pinned version is unchanged.

The changes above are included in the v0.13.0 implementation. Further migrations
should use the same incremental approach: extend existing prepared data and
session boundaries when behavior changes, without replacing the rendering
architecture or generating new public API layers.
