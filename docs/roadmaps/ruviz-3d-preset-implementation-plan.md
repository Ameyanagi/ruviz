# ruviz 3d preset and professional-polish implementation plan

Status: proposed; ready for implementation  
Date: 2026-07-24  
Branch: `feat/3d-implementation`  
Feature name: `3d`  
Research input: [ruviz 3d preset research](ruviz-3d-preset-research.md)  
Primary behavioral reference: Makie `Axis3` and themes  
Primary API references: Plotly templates and PyVista task-oriented views

## Outcome

Make a professional 3d result available through one memorable call while
keeping camera and fit controls independent:

```rust,ignore,reason=proposed-api
use ruviz::{surface, Preset3D, View3D};

surface(&x, &y, &z)
    .preset(Preset3D::Publication)
    .view(View3D::Isometric)
    .title("Measured response")
    .xlabel("x")
    .ylabel("y")
    .zlabel("response")
    .save("response.png")?;
```

The work is not a renderer rewrite. It is a shared, resolved Axis3 styling and
layout layer that both CPU and GPU presentation consume. Makie is the primary
reference for the internal separation of theme, Axis3 style, fit, projection,
and decoration protrusions. The public API remains deliberately smaller than
Makie's so it is easy for people and small code-generation models to use.

## Decisions

These decisions are part of the plan and do not need to be reopened during
implementation unless a correctness or performance test disproves them.

1. Keep the feature and module name exactly `3d`.
2. Keep the existing generic `Theme`. Do not add 3d-only panes, camera, or
   lighting fields to it.
3. Add three public concepts only: `Preset3D`, `View3D`, and `Fit3D`.
4. Keep the detailed `Axis3Style` internal in the first release. A large public
   Makie-like attribute surface is explicitly deferred.
5. Use enums in Rust and closed lowercase string unions in Python and
   TypeScript.
6. Resolve all preset values before layout or rendering. Renderers receive
   concrete values and never branch on a preset name.
7. Make override precedence independent of method call order:

   ```text
   library defaults < preset < explicit theme/view/fit < individual setters
   ```

8. A named view controls orientation only. A preset may supply its default
   orientation, but an explicit view replaces it and does not silently change
   theme, aspect, fit, data limits, or sampling.
9. A fit controls projected box use only. It does not alter data coordinates
   or explicit axis aspect.
10. Presets do not choose a backend, output format, sampling density, LOD, or
    interaction behavior.
11. CPU, native GPU, browser WebGPU, PNG, and hybrid SVG/PDF consume the same
    resolved Axis3 layout and style.
12. Preset state is retained. Camera-only frames do not reconstruct preset
    objects, rebuild geometry, upload an unchanged text atlas, or recompile a
    shader.
13. Land presets without silently changing the no-preset image first. Change
    the default appearance only after the preset gallery passes visual review.
14. Do not add user-defined preset registration in this milestone. Users can
    combine an existing `Theme`, named view, fit, and existing individual
    camera/series setters.
15. Within one override tier and for the same property, preserve existing
    builder behavior: the last setter wins. Cross-tier precedence remains
    independent of call order. For example, `.camera(c).azimuth_deg(10.0)`
    uses `10°`, while `.azimuth_deg(10.0).camera(c)` uses `c`'s azimuth; an
    explicit theme still overrides a preset regardless of which is called
    first.

## Current baseline and findings

The 3d renderer already has the right major boundaries:

- `Plot3D` owns camera, figure, theme, labels, limits, and series.
- `ResolvedFrame3D` owns validated data bounds and retained cache keys.
- `Axis3Layout` projects panes, grids, the box, ticks, labels, title, legend,
  and colorbars once for the CPU and GPU presentation paths.
- CPU, native GPU, browser WebGPU, interaction, and picking already share the
  camera and projected Axis3 model.
- `Theme::publication()`, `Theme::dark()`, and `Theme::minimal()` already
  provide reusable 2d/generic visual values.

The professional-polish gaps are concrete:

- `axis_viewport` reserves fixed percentages rather than measured decoration
  bounds.
- Every box edge uses the same color and width, so rear structure competes
  with foreground axes.
- Pane, grid, tick, and box styles are derived from hard-coded alpha/width
  functions.
- Axis ticks are fixed at six and label offsets are fixed pixel multiples.
- Text is clamped after placement rather than being included in the layout
  solve.
- The title, axis labels, legends, and colorbars are not solved together as
  measured protrusions.
- Surface lighting values are duplicated as constants in CPU shading and
  WGSL, which prevents a resolved preset from controlling them.
- The current camera accepts elevations only through `±89.9°`. Exact
  `Top`/`Bottom` views need a pole-safe camera basis instead of pretending that
  `89.9°` is an orthogonal view.
- The current builder stores concrete default camera/theme values, so it
  cannot distinguish a library default from an explicit user setter. A small
  provenance/override layer is required to make precedence deterministic.
- `frame_keys` currently hashes every x/y/z value while resolving a plot.
  Rebuilding a large builder for a style-only change can therefore remain
  O(data size) even when the retained renderer correctly reuses geometry.
- `Axis3Layout::resolve` regenerates tick values, formatted strings,
  decoration vectors, and text records on camera frames. Presets would
  magnify that fixed cost unless static axis content is separated from
  projected camera state.
- The direct GPU presenter rebuilds presentation vectors and text-atlas keys
  during composition. Professional decorations must use retained capacities,
  precomputed text keys, and bounded batches.
- Non-uniform axis aspect is applied to positions, but surface normals are not
  currently transformed by the inverse aspect before lighting. That becomes
  visible when presets make aspect and lighting intentional.

These findings define the implementation order: provenance and resolved
style first, measured layout and fit second, renderer consumption third, then
public bindings and visual/performance gates.

## Scope

### In scope

- Six built-in presets: `Default`, `Publication`, `Presentation`, `Dark`,
  `Minimal`, and `Technical`.
- Seven named views: `Isometric`, `Front`, `Back`, `Left`, `Right`, `Top`, and
  `Bottom`.
- Three fit policies: `Stable`, `Tight`, and `Stretch`.
- Measured title, tick, label, legend, and colorbar protrusions.
- A data-proportional `AxisAspect3D::Data` mode used by `Technical`.
- Front/rear edge and grid hierarchy.
- Pane visibility/color, tick density/offsets, label/title spacing, legend and
  colorbar placement, projection defaults, axis aspect defaults, and lighting.
- Rust API plus matching Python and TypeScript strings.
- CPU, native GPU, browser WebGPU, PNG, SVG, and PDF parity.
- Working examples, exact CPU goldens, GPU/browser comparison captures,
  documentation, and performance evidence.

### Out of scope

- Transparency sorting or order-independent transparency.
- Arbitrary mesh, volume, contour, or text-in-world primitives.
- A general scene graph or Makie-style observables system.
- A public field for every Axis3 visual property.
- Runtime registration or composition of named presets.
- Automatic data resampling, LOD changes, or backend selection by a preset.
- Mixed 2d/3d subplot layout.
- Replacing the current font or theme system.

## Public API contract

### Rust

Add these public enums under `core::plot3d` and re-export them from the crate
root beside the existing 3d types:

```rust,ignore,reason=proposed-api
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Preset3D {
    #[default]
    Default,
    Publication,
    Presentation,
    Dark,
    Minimal,
    Technical,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum View3D {
    #[default]
    Isometric,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Fit3D {
    #[default]
    Stable,
    Tight,
    Stretch,
}
```

Every 3d builder receives:

```rust,ignore,reason=proposed-api
pub fn preset(self, preset: Preset3D) -> Self;
pub fn view(self, view: View3D) -> Self;
pub fn fit(self, fit: Fit3D) -> Self;
```

The methods live in the existing common 3d builder macro, so scatter, line,
surface, wireframe, and mixed-series chains expose exactly the same API.

Adding `AxisAspect3D::Data` is part of the advanced existing camera vocabulary.
It resolves the physical box proportions from finite data extents and
normalizes by the largest extent. Degenerate extents use the existing
finite-span fallback. This mode changes only the plotted box proportions.

The advanced camera API remains valid:

```rust,ignore,reason=proposed-api
surface(&x, &y, &z)
    .preset(Preset3D::Publication)
    .view(View3D::Top)
    .axis_aspect(AxisAspect3D::Equal)
    .orthographic()
    .save("top.png")?;
```

The `axis_aspect` and `orthographic` calls win because they are individual
setters. Their result is identical whether they appear before or after
`.preset(...)` and `.view(...)`.

### Python

Expose closed `Literal` aliases in the stub and validate them at the binding
boundary:

```python
Preset3D = Literal[
    "default", "publication", "presentation", "dark", "minimal", "technical"
]
View3D = Literal[
    "isometric", "front", "back", "left", "right", "top", "bottom"
]
Fit3D = Literal["stable", "tight", "stretch"]

surface(x, y, z).preset("publication").view("isometric").save("surface.png")
```

`Plot3D.preset`, `Plot3D.view`, and `Plot3D.fit` return `self`, matching the
current fluent Python API. Invalid strings fail immediately with the complete
allowed-value list.

### TypeScript

Export closed string unions from `ruviz/3d`:

```ts
export type Preset3d =
  | "default"
  | "publication"
  | "presentation"
  | "dark"
  | "minimal"
  | "technical";
export type View3d =
  | "isometric"
  | "front"
  | "back"
  | "left"
  | "right"
  | "top"
  | "bottom";
export type Fit3d = "stable" | "tight" | "stretch";

await surface(x, y, z)
  .preset("publication")
  .view("isometric")
  .mount(canvas);
```

The TypeScript builder forwards the choices once while constructing the raw
Wasm plot. It does not resolve a second browser-only preset.

### Small-model usability rules

- One canonical method per concept: `preset`, `view`, and `fit`.
- One spelling across Python and TypeScript.
- No aliases such as `paper`, `pub`, `iso`, or `fitzoom`.
- Rust errors and dynamic-language errors list every valid value.
- Every example starts with the one-call preset form.
- Raw angles remain documented as advanced overrides, not the primary
  quick-start path.
- Rustdoc, Python docstrings, TypeScript JSDoc, and the user guide use the same
  one-sentence definition for every value.

## Exact semantics

### Named views

Named views resolve to an orientation tuple, not a complete camera:

| View | Eye direction | Initial orientation |
| --- | --- | --- |
| `Isometric` | balanced x/y/z | azimuth `-45°`, elevation `35.264°`, roll `0°` |
| `Front` | toward the x/z plane | azimuth `-90°`, elevation `0°`, roll `0°` |
| `Back` | opposite front | azimuth `90°`, elevation `0°`, roll `0°` |
| `Right` | toward the y/z plane | azimuth `0°`, elevation `0°`, roll `0°` |
| `Left` | opposite right | azimuth `180°`, elevation `0°`, roll `0°` |
| `Top` | down the z axis | elevation `90°` with a deterministic x-up basis |
| `Bottom` | up the z axis | elevation `-90°` with a deterministic x-up basis |

Before freezing these constants, a coordinate-orientation test must render
positive x/y/z labels in all seven views. If the current handedness makes a
name visually inverted, fix the table and test together; do not preserve a
misleading name.

`Top` and `Bottom` require `Camera3D::prepare` to select a non-collinear base
up vector near the poles before applying roll. Camera validation should then
accept the closed `[-90°, 90°]` interval. Projection, inverse projection,
picking, panning, and reset tests must cover both poles.

### Fit policies

| Fit | Contract | Interaction behavior |
| --- | --- | --- |
| `Stable` | Preserve aspect and use a conservative orientation-independent scale inside measured decoration margins | Apparent size remains stable while orbiting |
| `Tight` | Preserve aspect and fit the current projected eight-corner bounds plus padding | Uses more canvas; apparent size may change while orbiting |
| `Stretch` | Independently fill the available x/y viewport after decoration layout | Maximum fill; requested axis aspect is not visually preserved |

The implementation uses a target rectangle produced by the decoration
solver:

- `Stable` derives projection scale from the aspect-adjusted box bounding
  sphere and the smaller target dimension.
- `Tight` projects the eight box corners in camera space, computes their
  two-dimensional bounds, and solves one uniform scale.
- `Stretch` solves independent x and y projection scales. Picking remains
  correct because it uses the same invertible view-projection matrix.

Fit changes projection only. It never rewrites bounds, source points, axis
aspect, or geometry buffers.

### Built-in presets

The initial constants below are starting contracts, not unexplained magic
numbers. Each value must be represented by a named constant and may be tuned
only with an updated gallery and benchmark result.

| Property | Default | Publication | Presentation |
| --- | --- | --- | --- |
| Generic theme | `Theme::light()` | `Theme::publication()` | `Theme::presentation()` |
| Projection | orthographic | orthographic | perspective, `30°` vertical FOV |
| View | isometric | isometric | isometric |
| Fit | stable | stable | tight |
| Axis aspect | auto | auto | auto |
| Target short-dimension use | `74%` | `72%` | `80%` |
| Major ticks per axis | 5 | 5 | 4 |
| Panes | 3, subtle | 3, print-safe | 2 rear panes, subtle |
| Rear spine strength | `35%` of foreground | `30%` | `24%` |
| Front spine strength | `90%` | `100%` | `90%` |
| Major grid strength | `26%` | `22%` | `18%` |
| Tick-label pad | `8 pt` | `9 pt` | `10 pt` |
| Axis-label pad | `26 pt` | `30 pt` | `32 pt` |
| Title pad | `12 pt` | `14 pt` | `16 pt` |
| Lighting ambient/diffuse | `0.38 / 0.62` | `0.46 / 0.54` | `0.34 / 0.66` |

| Property | Dark | Minimal | Technical |
| --- | --- | --- | --- |
| Generic theme | `Theme::dark()` | `Theme::minimal()` | `Theme::light()` |
| Projection | orthographic | orthographic | orthographic |
| View | isometric | isometric | isometric |
| Fit | stable | tight | stable |
| Axis aspect | auto | auto | data |
| Target short-dimension use | `74%` | `80%` | `70%` |
| Major ticks per axis | 5 | 4 | 7 |
| Panes | 3, low contrast | hidden | 3, visible |
| Rear spine strength | `40%` | hidden except structural minimum | `55%` |
| Front spine strength | `100%` | `85%` | `100%` |
| Major grid strength | `30%` | hidden | `42%` |
| Tick-label pad | `8 pt` | `8 pt` | `8 pt` |
| Axis-label pad | `26 pt` | `24 pt` | `28 pt` |
| Title pad | `12 pt` | `12 pt` | `12 pt` |
| Lighting ambient/diffuse | `0.42 / 0.58` | `0.44 / 0.56` | `0.36 / 0.64` |

All color strengths are resolved as accessible colors against the selected
background, not implemented by multiplying sRGB bytes blindly. The light
direction remains the current normalized `[0.35, -0.45, 0.82]` initially so
presets improve tone without changing the perceived direction of existing
surfaces.

`Minimal` still shows foreground axes and user-requested labels. It does not
turn the plot into an unlabeled decorative image. `Technical` uses
data-proportional box aspect; it does not change data values or limits.

The table describes the approved end state. During the default-preserving
rollout, `Preset3D::Default` and an omitted preset both map to the exact
current contract. The curated Default values are activated only by the
separate default-migration gate.

### Delivery slices

The first reviewable slice contains `Default`, `Publication`, `Presentation`,
and `Dark`, all seven views, and `Stable`/`Tight`. These are enough for the
common paper, slide, dark UI, and explicit-view use cases.

The completion slice adds `Minimal`, `Technical`, `AxisAspect3D::Data`, and
`Stretch` after their focused goldens prove the semantics. `Stretch` must be
documented as intentionally distorting screen-space aspect. Keeping the
completion slice separate lets the core professional path ship without
forcing a vague or unreviewed mode into the first API review.

## Internal design

### New types

Add a focused `src/core/plot3d/style.rs` module with:

```rust,ignore,reason=proposed-internal
pub enum Preset3D { /* public values above */ }
pub enum View3D { /* public values above */ }
pub enum Fit3D { /* public values above */ }

pub(crate) struct Axis3Style {
    pub panes: PaneStyle3D,
    pub front_spine: Stroke3D,
    pub rear_spine: Stroke3D,
    pub grid: GridStyle3D,
    pub ticks: TickStyle3D,
    pub labels: LabelStyle3D,
    pub decorations: DecorationStyle3D,
    pub lighting: Lighting3D,
    pub fit: Fit3D,
    pub target_fill: f32,
}

pub(crate) struct Plot3DOverrides {
    pub preset: Option<Preset3D>,
    pub theme: Option<Theme>,
    pub view: Option<View3D>,
    pub fit: Option<Fit3D>,
    pub camera: CameraOverrides3D,
    pub axis: Axis3Overrides,
}
```

`CameraOverrides3D` stores `Option` values for azimuth, elevation, roll,
projection, aspect, zoom, and target. Calling `.camera(camera)` populates
every camera option. Calling `.azimuth_deg(...)` populates only azimuth.
This preserves the documented precedence without depending on builder method
order across tiers. Repeated writes to the same camera field within the
individual-setter tier preserve current last-call-wins behavior.

`Axis3Overrides` is internal initially. It reserves the merge point for
future focused setters without making all Axis3 fields public now.

### Resolution pipeline

Use one deterministic resolution path:

```text
Plot3D + series
  -> validate raw inputs
  -> start with library Default3D contract
  -> merge selected Preset3D
  -> merge explicit Theme / View3D / Fit3D
  -> merge individual camera and Axis3 overrides
  -> ResolvedFrame3D { theme, camera, axis_style, ... }
  -> Axis3LayoutEngine + shared text metrics
  -> Axis3Layout { geometry, semantic edge roles, concrete draw styles }
  -> CPU / native GPU / browser WebGPU / SVG-PDF
```

The preset enum itself should not be hashed into retained cache keys.
Hash the resolved values so two configurations that resolve identically can
reuse the same cache state.

### Retained dirty domains and cache-key ownership

The existing broad keys are not sufficient for cheap preset updates. Split
retained state into explicit dirty domains:

| Key | New inputs |
| --- | --- |
| Geometry | data identity/version, topology, sampling, normal policy |
| Material | per-series colors, colormaps, marker/line material, resolved lighting |
| Axis static | tick strings, fonts, labels, legend/colorbar sources, visibility and spacing |
| Axis view | resolved camera, fit, projected anchors/lines, viewport dimensions |
| Presentation | background, pane/spine/grid/tick/text colors and widths |

A preset change may invalidate material, Axis static/view, and presentation.
It must not invalidate geometry unless an existing explicit series option
independently requires that. Theme-only changes must not clear scene geometry
or material buffers when only Axis3 presentation changed.

Add a retained data identity/generation so a style-only mutation does not
rehash every x/y/z value. Initial one-shot builder validation may remain
O(data size), but a retained preset/theme/view/fit update must be
O(series + text/palette), preserve the geometry/BVH identity, and avoid
retransferring browser arrays.

Hash resolved values, never raw preset names. A preset and a manual
configuration with identical resolved values must produce identical dirty
domains, retained keys, diagnostics, and output.

### Semantic Axis3 geometry

Replace undifferentiated line vectors with semantic draw items:

```rust,ignore,reason=proposed-internal
enum Axis3LineRole {
    FrontSpine,
    RearSpine,
    MajorGrid,
    Tick,
}

struct StyledLine3D {
    geometry: OverlayLine3D,
    role: Axis3LineRole,
}
```

Classify box edges using camera-space depth and their relationship to the
selected outer anchor. Ties use a stable axis/index ordering so fixed-camera
goldens remain deterministic. A camera orbit changes roles and screen-space
line positions, but not source geometry.

Renderers look up the concrete resolved style for the role. They do not
repeat classification logic.

### Measured decoration layout

Introduce an `Axis3LayoutEngine` owned by a retained session or a one-shot
render preparation. It contains a bounded text-measurement cache keyed by
font family, text, size, and DPI, plus reusable scratch vectors.

Split its products:

```rust,ignore,reason=proposed-internal
struct Axis3StaticLayout {
    ticks: [ResolvedTicks3D; 3],
    text_metrics: TextMetricsSet3D,
    protrusions: Protrusions3D,
    legend: Option<Legend3D>,
    colorbars: Vec<Colorbar3D>,
}

struct Axis3ProjectedView {
    viewport: Viewport3D,
    camera: PreparedCamera3D,
    panes: Vec<StyledPane3D>,
    lines: Vec<StyledLine3D>,
    text: Vec<StyledText3D>,
}
```

Camera frames reuse the static tick strings, measured text, legend/colorbar
sources, and a precomputed text-atlas key. They only update bounded projected
geometry in retained scratch storage.

The layout solve is limited to two deterministic passes:

1. Generate ticks and strings from data limits and the resolved tick count.
2. Measure title, tick labels, axis labels, legend labels, and colorbar tick
   labels with the same font metrics used for rendering.
3. Reserve outer bands for title and outside decorations.
4. Build an initial target rectangle from measured protrusions.
5. Resolve the camera projection and selected fit against that rectangle.
6. Project the box, choose visible axis anchors, and place ticks/labels.
7. Compute exact decoration bounding rectangles and collision pairs.
8. Expand only the affected protrusion and place decorations once more without
   feeding the adjusted result back into another projection-scale solve.
9. If a collision remains, apply deterministic priority rules instead of an
   unbounded iteration.

Collision priority is:

```text
data box > axis labels > tick labels > title > legend/colorbar
```

The meaning is spatial reservation, not z-order. Lower-priority outside
decorations move farther outward; requested content is not silently hidden.
At very small canvas sizes, return the best valid layout and a diagnostic
counter rather than panic or loop.

The normal path should no longer rely on final clamping to keep text on the
canvas. Clamping remains a last-resort safety check.

The direct GPU compositor retains capacities for underlay, scene, foreground,
and text batches. Camera frames update only camera-dependent ranges. Draw
calls are bounded by non-empty scene batches plus four presentation batches,
not by the number of individual ticks, grids, legend items, or colorbar
segments.

### Stable interaction layout

For `Stable`, reserve protrusions using conservative maxima from the current
font/tick set and retain them across orbit frames. This prevents the plot box
from jumping as labels change sides.

For `Tight`, the projected box scale may change while orbiting, but outer
title/legend/colorbar bands remain stable. Only the inner projected-box solve
changes.

Use a one-pixel quantization for resolved viewport edges. Do not add
time-dependent interpolation to static exports or tests.

### Lighting

Move the current hard-coded ambient/diffuse values and light direction into
the resolved `Lighting3D` value:

```rust,ignore,reason=proposed-internal
struct Lighting3D {
    direction: [f32; 3],
    ambient: f32,
    diffuse: f32,
}
```

CPU shading receives this value directly. GPU shading receives one small
uniform shared by mesh batches. Preset changes update that uniform; camera
changes do not. Do not generate shader variants or rebuild normals for a
lighting-only change.

Before applying lighting, transform surface normals by the inverse axis aspect
and normalize them. Perform lighting in linear RGB and convert back to sRGB.
The CPU and WGSL formulas remain mathematically equivalent within `1/255` per
channel and retain the existing two-sided behavior. `Unlit` stays byte-exact.

## Work breakdown

### P3D-00 — Freeze baselines and instrumentation

Dependencies: none

Work:

- Preserve the four working example outputs and the current eight 3d gallery
  images as before-state evidence.
- Record current Axis3 layout time separately from geometry preparation and
  raster/present time.
- Add diagnostics for layout solves, text metric cache hits/misses, collision
  fallbacks, preset resolutions, style uniform uploads, and text atlas uploads.
- Add counters for geometry-key scans, scene/BVH identity, presentation
  allocations, buffer creations, material writes, per-domain upload bytes,
  shader/pipeline creation, draw calls, and browser data transfers.
- Record before-change CPU and GPU benchmark JSON on the same machine used
  for after-change comparison.

Acceptance:

- No visual behavior changes.
- Existing tests remain green.
- A benchmark result can distinguish preset/layout overhead from geometry and
  presentation.

Likely touchpoints:

- `src/core/plot3d/diagnostics.rs`
- `src/core/plot3d/layout.rs`
- `src/core/plot3d/prepared.rs`
- `benches/`
- `docs/benchmarks/`

### P3D-01 — Add public vocabulary and override provenance

Dependencies: P3D-00

Work:

- Add the three enums and exact docs.
- Add `Plot3DOverrides` and `CameraOverrides3D`.
- Change current camera/theme builder methods to record explicit overrides.
- Implement a pure `resolve_3d_style` merge function.
- Add `.preset`, `.view`, and `.fit` to the common builder macro.
- Re-export public types.
- Introduce retained source identity/generations so a style-only update does
  not scan or hash all data arrays.
- Add resolved-style identity to diagnostics without placing preset branches
  in a renderer.

Acceptance:

- All four plot builders expose identical methods.
- Every pairwise ordering of preset/theme/view/fit/individual camera calls
  resolves identically.
- Existing no-preset calls resolve to their current visual contract.
- Invalid camera values still fail at the same terminal validation boundary.
- No renderer reads a raw preset enum.
- A retained preset/theme/view/fit update is O(series + text/palette), does not
  hash data elements, and preserves scene and BVH identity.
- A preset and its manually expanded equivalent produce identical keys and
  diagnostics.

Likely touchpoints:

- `src/core/plot3d/style.rs`
- `src/core/plot3d/types.rs`
- `src/core/plot3d/builder.rs`
- `src/core/plot3d/resolve.rs`
- `src/core/plot3d/mod.rs`
- `src/lib.rs`

### P3D-02 — Pole-safe named views

Dependencies: P3D-01

Work:

- Implement all named orientation mappings.
- Make camera basis construction deterministic at `±90°`.
- Apply roll around the resolved forward vector after selecting a safe base
  up direction.
- Cover projection, unprojection, picking, panning, reset, and snapshots at
  top/bottom views.

Acceptance:

- All seven views show the expected positive axis directions.
- Top/bottom matrices and inverses are finite.
- A project/unproject round trip remains within the existing numeric
  tolerance.
- Restoring a camera snapshot after a named view is exact.

Likely touchpoints:

- `src/core/plot3d/types.rs`
- `src/core/plot3d/interaction.rs`
- `src/core/plot3d/picking.rs`
- `tests/`

### P3D-03 — Resolved Axis3 style and semantic line roles

Dependencies: P3D-01

Work:

- Add internal pane, stroke, grid, tick, label, decoration, and lighting
  structs.
- Resolve concrete colors in linear color space with contrast checks.
- Classify front/rear spines deterministically.
- Make CPU overlay, SVG overlay, and GPU presenter consume semantic roles and
  concrete styles.
- Preserve current output for the no-preset compatibility path.
- Split broad appearance/layout hashes into geometry, material, Axis static,
  Axis view, and presentation dirty domains.

Acceptance:

- Publication foreground axes are visibly stronger than rear spines.
- CPU, SVG, and direct GPU use the same line count, role count, coordinates,
  colors, and widths before rasterization.
- A theme-only or preset-only change never rebuilds source geometry.
- Axis-only style changes create no scene vertex/index buffer and do not
  recreate attachments, pipelines, or bind groups once capacity is warm.

Likely touchpoints:

- `src/core/plot3d/style.rs`
- `src/core/plot3d/layout.rs`
- `src/render/three_d/overlay.rs`
- `src/render/three_d/gpu/presenter.rs`
- `src/core/plot3d/resolve.rs`

### P3D-04 — Measured protrusions and collision-safe layout

Dependencies: P3D-03

Work:

- Add a shared bounded text metrics cache.
- Split static ticks/text/decorations from camera-projected Axis3 state.
- Replace fixed viewport percentages with measured protrusions.
- Resolve title, axis labels, tick labels, legend, and colorbars together.
- Add the deterministic two-pass solve and collision diagnostics.
- Keep layout stable across orbit frames under `Stable`.
- Retain compositor vectors, presentation buffers, and the precomputed
  text-atlas key across camera frames.

Acceptance:

- No canonical title, tick label, axis label, legend, or colorbar overlaps
  another decoration or the projected data box.
- No canonical text is positioned by the last-resort clamp.
- Layout terminates after at most two passes.
- Repeated warm frames reuse text metrics and the GPU text atlas.
- Canvas-size and DPI changes invalidate only the required layout/text state.
- Camera frames do no tick formatting or text measurement.
- Presentation draw calls remain bounded by the scene batch count plus four
  overlay layers.

Likely touchpoints:

- `src/core/plot3d/layout.rs`
- `src/core/plot3d/prepared.rs`
- `src/render/skia/`
- `src/render/three_d/gpu/presenter.rs`
- `src/render/three_d/overlay.rs`

### P3D-05 — Fit policies

Dependencies: P3D-02, P3D-04

Work:

- Implement Stable, Tight, and Stretch projection solving.
- Use the measured target rectangle and preset target-fill value.
- Keep inverse matrices and picking consistent.
- Add wide, tall, square, small, high-DPI, orthographic, and perspective
  cases.

Acceptance:

- Stable changes apparent box size by no more than `1 px` during the canonical
  full-orbit sample.
- Tight reaches its target fill within `±2 percentage points` for canonical
  cameras.
- Stretch fills both target dimensions within `±2 percentage points`.
- The projected Axis3 box uses `68–82%` of the short canvas dimension in
  standard preset examples.
- No projected corner escapes the target rectangle.

Likely touchpoints:

- `src/core/plot3d/layout.rs`
- `src/core/plot3d/types.rs`
- `src/core/plot3d/interaction.rs`
- `src/core/plot3d/picking.rs`

### P3D-06 — Shared preset lighting

Dependencies: P3D-03

Work:

- Move light direction and ambient/diffuse values out of CPU and WGSL
  constants.
- Feed resolved lighting to CPU shading and one GPU uniform.
- Hash lighting under material, not geometry or Axis view.
- Add CPU/GPU formula parity vectors.
- Correct normals for non-uniform axis aspect before lighting.
- Perform lit shading in linear RGB on both backends.

Acceptance:

- CPU and GPU produce matching expected intensities for a fixed normal set.
- Lighting changes do not recompute normals, meshes, or sampling.
- Camera-only frames do not upload the lighting uniform.
- Existing unlit surfaces remain unaffected.
- Non-uniform fixed/data aspect produces physically consistent light response.

Likely touchpoints:

- `src/render/three_d/software/shading.rs`
- `src/render/three_d/gpu/resources.rs`
- `src/render/three_d/gpu/shaders/mesh.wgsl`
- `src/core/plot3d/prepared.rs`

### P3D-07 — Implement and tune built-ins

Dependencies: P3D-03 through P3D-06

Work:

- Implement `Publication`, `Presentation`, and `Dark`.
- Render and review the canonical matrix.
- Tune named constants only against documented acceptance metrics.
- Add `Minimal` and `Technical` after the first three pass review.
- Freeze every preset as a resolved-value snapshot test.

Acceptance:

- Every table value in this document has a corresponding named constant and
  test.
- Text/background contrast is at least WCAG AA `4.5:1`.
- Rear structure remains subordinate in color and grayscale captures.
- Presets remain recognizable and useful on scatter, line, surface, and
  wireframe plots.

Likely touchpoints:

- `src/core/plot3d/style.rs`
- `src/render/theme.rs`
- `tests/`
- `examples/generate_3d_gallery.rs`

### P3D-08 — Python and browser API parity

Dependencies: P3D-01, stable preset constants from P3D-07

Work:

- Add Python fluent methods and `Literal` types.
- Add Wasm raw setters for preset/view/fit.
- Add TypeScript string unions and fluent methods.
- Forward values to Rust once; do not duplicate resolution tables.
- Update package exports, declarations, README examples, and package
  verification.
- Add a versioned contract fixture containing the canonical dynamic-language
  names and representative resolved-value snapshots.

Acceptance:

- The same conceptual example works in Rust, Python, and TypeScript.
- All valid strings reach the matching Rust enum.
- Every invalid string fails with the complete allowed list.
- Dynamic-language call order resolves identically to Rust.
- `ruviz/3d` package verification checks the new methods and declarations.
- Browser preset construction and retained updates never resend numeric data
  arrays or recreate the canvas/device/session.

Likely touchpoints:

- `python/src/lib.rs`
- `python/ruviz_py/ruviz/_api.py`
- `python/ruviz_py/ruviz/__init__.py`
- `python/tests/test_3d_api.py`
- `crates/ruviz-web/src/lib.rs`
- `packages/ruviz-web/src/3d.ts`
- `packages/ruviz-web/src/index.ts`
- `demo/web/tests/3d.spec.js`
- `packages/ruviz-web/scripts/verify-npm-package.mjs`
- `tests/fixtures/contracts/3d_presets.json`

### P3D-09 — Working examples and visual evidence

Dependencies: P3D-07, P3D-08

Work:

- Add one minimal publication example.
- Add one six-preset surface contact sheet/generator.
- Add one seven-view contact sheet/generator.
- Add a technical scatter/line example and a presentation surface example.
- Generate committed PNGs and the matching exact CPU golden fixtures.
- Capture native GPU and browser WebGPU comparison images.
- Document exact commands to run and open every example.

Canonical commands should remain simple:

```bash
cargo run --no-default-features --features 3d --example doc_surface3d_preset
cargo run --no-default-features --features 3d --example generate_3d_preset_gallery
```

Acceptance:

- Every documented command runs from a clean checkout.
- Gallery assets are reproducible and checked for freshness.
- CPU fixed-camera PNGs match exactly.
- GPU/browser captures pass the existing bounded visual-difference policy.
- Documentation shows actual output, not only API snippets.

Likely touchpoints:

- `examples/`
- `docs/assets/gallery/rust/3d/`
- `docs/gallery.md`
- `tests/golden/3d/`
- `tests/gallery_3d_golden_test.rs`
- `packages/ruviz-web/e2e/`

### P3D-10 — Performance, compatibility, and release gates

Dependencies: all previous tasks

Work:

- Run the full CPU and retained GPU benchmark matrix before and after.
- Run allocation and upload diagnostics during 256-frame orbit loops.
- Run a 10,000-frame alternating orbit/preset stress for bounded host/GPU
  resource growth.
- Verify CPU/GPU/browser parity, minimal feature builds, all features, Wasm,
  Python, npm packaging, Rust MSRV, strict Clippy, docs, and examples.
- Run `greptile review --agent` after the branch is fully verified.
- Address all correctness and performance findings, rerun focused and full
  gates, then request a follow-up Greptile review.

Acceptance:

- Every quality and performance budget below passes.
- Existing no-preset users keep current behavior until the explicit default
  migration gate.
- The working tree is clean and the branch contains the complete documented
  commit series.
- Final Greptile result has no unresolved P0/P1 findings.
- Matched before/after artifacts expose raw median/p95/p99, allocations,
  uploads, resource counts, and dirty domains rather than only aggregate FPS.

## Performance plan

### Current local reference medians

The most recent Apple Metal run recorded:

| Workload | Median |
| --- | ---: |
| CPU camera, 100K scatter | `42.7816 ms` |
| CPU camera, 100×100 surface | `33.3598 ms` |
| CPU warm unchanged, 100K scatter | `49.7231 ms` |
| CPU warm unchanged, 100×100 surface | `27.4099 ms` |
| GPU camera no-readback, 100K scatter | `6.4817 ms` |
| GPU camera no-readback, 100×100 surface | `2.0519 ms` |
| GPU warm no-readback, 100K scatter | `8.3379 ms` |
| GPU warm no-readback, 100×100 surface | `2.1504 ms` |

These numbers are reference evidence, not portable absolute promises. CI gates
use same-machine before/after ratios; fixed-hardware reports also retain raw
times.

### Required benchmark matrix

Measure:

- empty Axis3;
- 100, 10K, and 100K scatter points;
- 1K and 100K line vertices;
- 25×25, 100×100, and 250×250 surfaces;
- warm unchanged frame;
- camera-only orbit frame;
- preset expansion alone and the identical manually expanded configuration;
- preset change;
- view change;
- fit change;
- theme change;
- canvas/DPI resize;
- legend/colorbar present and absent;
- CPU export, retained native GPU no-readback, and Chromium WebGPU.

Run Default, Publication, and Dark in every backend. Run all six presets on
the empty Axis3 and 100×100 surface cases.

### Hard budgets

- Preset resolution itself: less than `1%` of retained camera-frame time and
  no heap allocation on a warm camera-only frame.
- Total retained camera-frame regression: less than `3%` CPU and less than
  `2%` GPU/WebGPU in same-machine comparisons.
- Geometry rebuilds during preset/view/fit changes: zero unless an independent
  geometry option changed.
- Normal recomputations for preset lighting changes: zero.
- Text atlas uploads during camera-only frames with unchanged strings/fonts:
  zero.
- Style uniform uploads during camera-only frames: zero.
- Shader or pipeline compilations after session warm-up: zero.
- Layout solve: at most two passes, with a text-metric cache hit rate above
  `99%` after warm-up.
- Memory retained by the text metric cache: bounded and reported; initial cap
  `4,096` entries per session with deterministic least-recently-used eviction.
- Browser frame scheduling remains coalesced to at most one submission per
  animation frame.
- One-time preset expansion is p99 `≤10 µs` after startup/font discovery and
  performs no dataset-sized work.
- A preset and its manually expanded equivalent have byte-identical CPU
  output, semantically identical GPU output, identical retained keys, and
  `≤1%` timing difference.
- Camera projection/layout after warming performs no tick formatting or text
  measurement and has p95 `≤1 ms` at `800×600`.
- A 10,000-frame orbit/preset-toggle stress grows retained host memory by no
  more than `1 MiB` and leaves GPU resource counts stable after warm-up.
- Visual preset changes perform zero triangulations, normal recomputations,
  BVH rebuilds, vertex/index uploads, attachment recreations, and pipeline
  compilations.
- Same-shape presentation updates create no new buffers once retained
  capacity is established.
- The WebAssembly/package addition for preset vocabulary and resolution should
  remain `≤10 KiB` gzip and add no shader variants.

If a budget fails, optimize or narrow the preset behavior. Do not hide the
regression by changing benchmark data, sampling, or backend routing.

Hosted CI enforces semantic invariants and broad smoke bounds rather than
unstable wall-clock thresholds. Fixed-hardware performance runs warm at least
30 frames, record at least 600 camera frames, retain raw samples, and report
median/p95/p99. A separate 10,000-frame run checks memory and resource
stability.

For external context, run matched warm-camera comparisons against GLMakie and
matched static-output comparisons against Matplotlib/CairoMakie on the same
machine. Match geometry, viewport, labels, projection, and sampling and
separate startup/JIT time. These comparisons are evidence, not CI gates and
not a promise of pixel compatibility.

## Verification matrix

### Unit and property tests

- Enum defaults, debug names, and all dynamic-language parsers.
- Full precedence table, including permutations of method call order.
- Preset-to-resolved-value snapshots.
- Linear color mixing and WCAG contrast.
- Front/rear edge classification for canonical and tie cameras.
- Stable/Tight/Stretch solve math.
- Pole-safe camera basis and finite inverse matrices.
- Project/unproject and pick round trips for all views and fits.
- `AxisAspect3D::Data` finite/degenerate/extreme-range behavior.
- Inverse-aspect normal transformation and linear CPU/GPU lighting parity.
- Text metrics cache keying and bounded eviction.
- Two-pass layout termination.
- Collision rectangles at multiple label lengths and DPIs.
- Cache-key invalidation ownership.
- Preset/manual-equivalence keys, output, and diagnostics.
- Retained style update with no data hash scan, geometry/BVH rebuild, or
  browser array transfer.

### Layout cases

At minimum:

- canvases `320×240`, `640×480`, `800×800`, `1200×400`, and `400×1200`;
- DPI `72`, `96`, `144`, and `300`;
- no title/labels;
- long title and three long axis labels;
- negative, scientific-notation, and degenerate-range ticks;
- legend only, colorbar only, and both;
- two stacked colorbars;
- light, dark, publication, and minimal themes;
- orthographic and perspective;
- every named view;
- empty labels rejected by existing validation;
- non-Latin labels when the registered test font supports them.

### Golden strategy

Avoid an unmaintainable Cartesian product:

- one canonical surface under all six presets;
- one canonical Axis3 under all seven views;
- scatter, line, surface, and wireframe under `Publication`;
- surface under `Presentation`, `Dark`, `Minimal`, and `Technical`;
- long-decoration stress plot;
- top and bottom orthographic plots;
- one perspective tight-fit plot.

CPU fixed-camera images are exact goldens. Native GPU and browser WebGPU use
bounded perceptual comparisons and shared layout-geometry assertions because
GPU rasterization may vary across adapters.

### Full gates

- `cargo test --all-features --lib`
- public 3d API integration tests
- correctness/property tests
- required-adapter GPU tests
- exact gallery freshness/goldens
- Rust MSRV build
- strict Clippy on all targets/features
- `wasm32` root and `ruviz-web` builds
- Chromium main-thread and worker WebGPU tests
- GPUI and pixel-buffer ownership regressions
- documentation checker and code-fence compilation
- Python tests, maturin build, and Ruff
- npm tarball/import/declaration verification for `ruviz/3d`
- packaged-crate consumer against registry dependencies
- `greptile review --agent`

## Professional visual acceptance

A preset is not considered complete merely because it renders.

- No canonical decoration intersects another decoration or the data box.
- No canonical text is clipped or positioned by the safety clamp.
- Standard figures use `68–82%` of the short canvas dimension for the
  projected Axis3 box.
- Foreground axes are clearly stronger than rear spines.
- Rear grids never dominate surface shading or scatter markers.
- Text/background contrast is at least `4.5:1`.
- Dark preset colors remain distinct under common color-vision simulations.
- Publication remains legible in grayscale and at final-column-size preview.
- Presentation remains legible from a `1280×720` capture.
- Minimal retains enough structure to identify x/y/z directions.
- Technical preserves equal visual axis lengths and readable dense ticks.
- CPU, native GPU, and browser use the same viewport, text anchors, line
  roles, and style values.
- CPU, SVG, native GPU, and browser can serialize an identical resolved Axis3
  layout snapshot before backend rasterization.
- Rear-spine width is at most `75%` of front-spine width and effective grid
  opacity is at most `75%` of rear-spine opacity in professional presets.
- Lit CPU/GPU canonical-normal channel error is at most `1/255`; unlit output
  remains byte-exact.
- Presets add no GPU draw call beyond the bounded shared presentation layers.
- All examples are visually reviewed at `100%` and `200%` scale before goldens
  are accepted.

## Compatibility and default migration

The first preset commit keeps existing no-preset output unchanged. That gives
users an explicit opt-in and prevents a large golden rewrite from hiding
implementation mistakes.

After all six presets pass visual, parity, and performance gates:

1. Render the existing default and `Preset3D::Default` galleries side by side.
2. Obtain explicit visual approval for the new default.
3. Change no-preset resolution to the curated `Default` contract in a
   separately named commit.
4. Update release notes and all intentionally changed goldens in that commit.
5. Keep `.preset(Preset3D::Default)` equivalent to omitting `.preset(...)`.

Until step 3, an internal compatibility style may preserve the old output.
It is not exposed as a public `Legacy` preset.

## Commit plan

Keep changes reviewable in this order:

1. `test(3d): record preset polish baselines`
2. `feat(3d): add preset view fit resolution`
3. `fix(3d): support exact orthogonal camera views`
4. `feat(3d): add resolved axis style roles`
5. `feat(3d): measure axis protrusions and fit`
6. `feat(3d): share preset lighting across backends`
7. `feat(3d): add professional preset catalog`
8. `feat(3d): expose presets in python and web`
9. `docs(3d): add preset examples and visual gallery`
10. `perf(3d): verify retained preset budgets`
11. Optional after approval:
    `style(3d): adopt professional default preset`
12. Greptile fixes, if any, in focused commits that name the corrected issue.

Do not mix generated gallery updates into core algorithm commits. Do not
squash performance evidence into an unrelated documentation change.

## Risks and mitigations

| Risk | Effect | Mitigation |
| --- | --- | --- |
| Measured layout adds warm-frame work | GPU surface frame regression | Retained bounded text metrics, stable outer bands, two-pass maximum, dedicated diagnostics |
| Top/bottom camera basis is singular | NaN matrices or broken picking | Pole-safe base up vector and round-trip tests |
| Preset precedence follows call order accidentally | Unpredictable API and generated-code failures | Explicit `Option` provenance and permutation tests |
| CPU/GPU styles drift | Different exported and interactive plots | Resolve once, semantic roles, shared layout assertions |
| Lighting preset rebuilds meshes | Large surface regressions | Appearance uniform only; normals remain geometry state |
| Tight fit pumps during orbit | Distracting interaction | Documented contract; Stable remains default |
| Long labels still collide | Unprofessional outputs | Measured two-pass solve, explicit priority, stress goldens |
| Too many goldens become brittle | Slow reviews and noisy changes | Targeted canonical matrix plus layout-property tests |
| A preset silently changes data detail | Misleading scientific output | Preset contract excludes sampling, LOD, limits, and backend |
| Dynamic bindings duplicate constants | Rust/Python/TS drift | Parse strings into Rust enums and resolve only in Rust |
| Default visual change hides regressions | Difficult review | Separate opt-in implementation from explicit default-migration commit |

## Definition of done

This plan is complete only when:

- `Preset3D`, `View3D`, and `Fit3D` are public and documented.
- The one-call publication example works in Rust, Python, and TypeScript.
- All six presets and seven views have actual committed visual examples.
- Axis3 layout uses measured protrusions and passes collision tests.
- Front/rear hierarchy and shared lighting work in CPU and GPU paths.
- CPU, native GPU, browser WebGPU, PNG, SVG, and PDF agree on resolved layout.
- Performance budgets pass with committed before/after evidence.
- Existing no-preset behavior is either deliberately preserved or changed in
  the separately approved migration commit.
- The full verification matrix passes.
- Final Greptile review has no unresolved P0/P1 findings.
- The roadmap and release notes identify any remaining hardware-only evidence
  honestly.

## Implementation start point

Begin with P3D-00 and P3D-01. Do not start by tuning colors in
`overlay.rs`: without override provenance and a resolved Axis3 style, that
would create backend duplication and call-order bugs. The critical path is:

```text
baseline
  -> deterministic resolution
  -> pole-safe views
  -> semantic Axis3 style
  -> measured protrusions
  -> fit policies
  -> shared lighting
  -> preset tuning
  -> bindings/examples/performance/review
```
