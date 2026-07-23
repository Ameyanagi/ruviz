# ruviz 3D implementation plan

Status: living implementation plan  
Branch: `feat/3d-implementation`  
Base: `42533c6` (`v0.5.0`, current `main` when this branch was created)  
Started: 2026-07-23

This document is the source of truth for the first ruviz 3D implementation. It
records decisions as they are made so that discoveries do not remain only in
chat, issue comments, or unmerged branches.

## Goal

Add a 3D plotting path that is:

- simple enough for a first-time Rust user and a small code-generation model;
- performant for large interactive scatter plots and surfaces;
- deterministic and correct on the CPU;
- truly GPU-rendered when diagnostics say that the GPU executed;
- familiar to Matplotlib and Makie users without inheriting Matplotlib's
  painter-order and fixed-downsampling limitations;
- isolated from the mature 2D pipeline so existing 2D output remains unchanged.

The first stable scope is opaque `scatter3d`, `line3d`, `surface`, and
`wireframe`, automatic 3D axes, orthographic and perspective cameras, static
PNG/SVG/PDF output, and retained GPU interaction.

Volume rendering is not part of this project. It requires a separate
ray-marching design after the mesh/scatter pipeline is stable.

## Decisions

### 1. One canonical public API

The documented path uses four top-level functions:

```rust
scatter3d(&x, &y, &z).save("scatter.png")?;
line3d(&x, &y, &z).save("line.png")?;
surface(&x, &y, &z_grid).save("surface.png")?;
wireframe(&x, &y, &z_grid).save("wireframe.png")?;
```

Required data is positional and always ordered `x, y, z`. Styling and camera
configuration are fluent. No axis, camera, backend, config object, macro,
tuple-packed coordinate type, turbofish, `.end_series()`, or async runtime is
required for the common path.

Canonical signatures:

```rust
pub fn scatter3d<X, Y, Z>(
    x: &X,
    y: &Y,
    z: &Z,
) -> Scatter3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData1D + ?Sized;

pub fn line3d<X, Y, Z>(
    x: &X,
    y: &Y,
    z: &Z,
) -> Line3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData1D + ?Sized;

pub fn surface<X, Y, Z>(
    x: &X,
    y: &Y,
    z: &Z,
) -> Surface3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData2D + ?Sized;

pub fn wireframe<X, Y, Z>(
    x: &X,
    y: &Y,
    z: &Z,
) -> Wireframe3DBuilder
where
    X: NumericData1D + ?Sized,
    Y: NumericData1D + ?Sized,
    Z: NumericData2D + ?Sized;
```

The builders are concrete public types. Generics stop at ingestion so compiler
errors and rustdoc remain readable. Each builder exposes only valid operations,
terminal methods, and the other 3D series continuation methods.

The MVP will not add competing aliases such as `scatter_3d`, `plot3d`,
`plot_surface`, or `surface3d`. A lower-level `Plot3D` orchestration type may
exist internally, but it will not create a second documented construction path.

### 2. Surface data has one unambiguous shape

The canonical regular-grid form is:

```text
z.shape() == (y.len(), x.len())
z[y_index][x_index] is the height at (x[x_index], y[y_index])
```

The MVP does not accept flattened z data, full X/Y meshgrid matrices, implicit
index grids, or automatic transposition. This avoids allocations and prevents
small models from choosing among several equivalent input forms.

Required diagnostics include:

```text
scatter3d: x, y, and z must have the same length (x=3, y=2, z=3)
surface: z shape must be (y.len(), x.len()) = (80, 100), got (100, 80)
surface: row 3 has 99 values, expected 100
```

Add structured errors rather than encoding these cases in a generic string:

```rust
DataLengthMismatch3D { x_len, y_len, z_len, series_index }
GridShapeMismatch {
    expected_rows,
    expected_columns,
    actual_rows,
    actual_columns,
}
InvalidCamera3D { field, value, reason }
InvalidTopology3D { reason }
```

NaN values split lines and create holes in surfaces. Infinity is rejected with
the coordinate name and flattened index. Empty data, ragged grids, checked-size
overflow, and invalid indices are errors; data is never silently truncated,
zero-filled, or transposed.

### 3. Defaults favor scientific interpretation

| Setting | Default |
| --- | --- |
| Projection | Orthographic |
| Azimuth | -60 degrees |
| Elevation | 30 degrees |
| Roll | 0 degrees |
| View fit | Stable; no apparent-size pumping while orbiting |
| Box aspect | Automatic 4:4:3-style scientific box |
| Scatter marker | Opaque circle, 6 pt |
| Line | Opaque solid theme line, no markers |
| Surface | Opaque, smooth normals, viridis from z |
| Surface colorbar | Off unless requested |
| Backface culling | Off; surfaces are two-sided by default |
| Axes | Linear, auto limits, ticks/grid visible |
| Transparency | Unsupported in the MVP |

All public angle methods include the unit:

```rust
let camera = Camera3D::default()
    .azimuth_deg(45.0)
    .elevation_deg(25.0)
    .perspective_deg(45.0);

surface(&x, &y, &z)
    .camera(camera)
    .save("surface.png")?;
```

The canonical example does not configure a camera. Named degree setters are
preferred over a two-number `view(...)` shortcut because they eliminate both
argument-order and degree/radian ambiguity.

### 4. Keep the 3D internals out of the 2D match graph

Do not add 3D plot variants to the existing `PlotInput`, `SeriesType`,
`ResolvedSeries`, `CoordinateTransform`, `LayoutManager`, or 2D interaction
session. Those abstractions have hundreds of references and encode 2D
assumptions about bounds, axes, clipping, SVG, and input mapping.

Create a parallel internal pipeline:

```text
3D builder
  -> coherent ResolvedFrame3D
  -> cached, camera-independent Scene3D
  -> RenderFrame3D with camera and Axis3 layout
  -> software depth renderer or direct wgpu renderer
  -> 2D text/legend/colorbar overlay
  -> Image/PNG or interactive surface
```

High-level series lower to a small backend-neutral primitive vocabulary:

```rust
struct Scene3D {
    meshes: Vec<MeshBatch3D>,
    lines: Vec<LineBatch3D>,
    points: Vec<PointBatch3D>,
    bounds: Bounds3D,
}
```

Backends consume mesh, line, and point batches. They never match on
`Surface`, `Scatter3D`, or later high-level plot kinds. Dynamic dispatch, if
used, happens once per high-level series rather than per vertex.

### 5. Shared math, separate renderers

Both CPU and GPU use the same:

- f64 axis scaling and bounds calculation;
- scene-relative origin/range normalization;
- right-handed model/view/projection contract;
- wgpu-compatible clip depth range of `0..1`;
- homogeneous six-plane clipping semantics;
- row-major surface triangulation and winding;
- normal and colormap inputs;
- Axis3 camera and layout snapshot.

Large scientific offsets are normalized in f64 before conversion to f32. A
regression scene around `1e12` with small deltas is mandatory.

`glam` is already an unconditional dependency and supplies the matrices,
vectors, and quaternions. The existing 2D `CoordinateTransform` stays unchanged.

### 6. The CPU renderer is a real reference renderer

Do not reproduce Matplotlib's average-z painter ordering for opaque PNG output.
Implement a deterministic software depth renderer with:

- 32x32 pixel tiles;
- tile-local color, 24-bit quantized depth, and primitive-owner samples;
- lock-free Rayon parallelism across disjoint tiles;
- stable primitive order and primitive-ID tie breaking;
- top-left triangle fill rules;
- homogeneous triangle clipping;
- perspective-correct interpolation;
- screen-space thick lines;
- depth-tested billboard markers;
- one sample per pixel for interactive CPU fallback;
- 4x MSAA for static/export quality.

Prepared geometry contains triangulation, indices, normals, scalar ranges, and
styles. It is not rebuilt when only the camera moves.

Regular-grid surface cells use a consistent two-triangle split:

```text
(r,c) -> (r,c+1) -> (r+1,c+1)
(r,c) -> (r+1,c+1) -> (r+1,c)
```

The initial lighting model is deterministic ambient plus Lambertian directional
light. `Unlit`, `Flat`, and `Smooth` shading are typed choices. Specular,
shadows, SSAO, PBR, and textures are out of scope.

### 7. SVG and PDF use correct hybrid export

For the MVP, the complete depth-tested 3D plot-area layer is rasterized and
embedded in SVG/PDF. Titles, tick labels, x/y/z labels, legends, and colorbars
remain vector elements.

This policy avoids incorrect triangle ordering and unbounded SVG sizes while
keeping publication text sharp. Exact vector surface export is not promised.
An approximate vector mode, if ever added, must be explicit and documented.

The export resolution is separate from SVG layout units. PDF defaults to a
300-DPI embedded 3D layer, subject to checked allocation limits.

### 8. GPU means direct GPU drawing

The current GPU path transforms 2D coordinates and reads them back for CPU
drawing. It is not the 3D renderer.

The 3D GPU path must:

- render directly with wgpu render passes;
- retain vertex/index/instance buffers by scene revision;
- use indexed surface meshes;
- use instanced billboard scatter markers;
- use instanced screen-space line segment quads;
- use a real depth attachment;
- update one camera uniform for camera-only motion;
- avoid vertex/index upload and CPU readback during interactive frames;
- read back once only for static `Image`/PNG output;
- expose actual backend, fallback reason, cache, upload, and draw diagnostics.

Reuse wgpu device/queue/capability initialization where sound. Do not build on
the current compute-and-readback `GpuRenderer` or the placeholder
`render::backend::Renderer` trait.

Explicit GPU requests fail clearly when unavailable. `Auto` may fall back to the
CPU renderer, but diagnostics must report the actual renderer and reason.

#### GPU audit findings that must be corrected

The existing GPU code is useful only as a source of device initialization ideas.
The 3D work must not inherit these current limitations:

- `GpuVertex` contains a 2D screen position.
- `GpuRenderer` dispatches a coordinate transform, waits, maps the result back,
  splits it into CPU arrays, and then relies on CPU drawing.
- the current renderer trait implementation is effectively a no-op;
- the current pipeline cache key omits vertex layout and blend details;
- its vertex stride is inferred from attribute sizes rather than using the
  declared buffer layout;
- its bind-group layout is fixed to one uniform;
- culling is disabled;
- the offscreen target lacks `COPY_SRC`;
- the depth target test cannot succeed with the currently reported format list;
- several capabilities are guessed rather than queried from format features,
  device limits, and surface capabilities.

Create a corrected GPU foundation:

```text
GpuContext
  Arc<Instance/Adapter/Device/Queue>
  exact capabilities
  device-loss state

Wgpu3DRenderer
  PipelineLibrary
  ResourceCache<(SceneObjectId, Revision)>
  FrameRing (2-3 frames)
  AttachmentCache
  optional timestamp profiler

RenderTarget3D
  SurfaceTarget       direct acquire/render/present; never read back
  OffscreenTarget     persistent sRGB texture; copy only for explicit export
```

The pipeline key includes shader/primitive variant, color/depth formats, sample
count, topology, front face, cull mode, depth state, blend state, vertex layout,
and material/text variant.

GPU primitive implementation:

- scatter: one unit quad plus one <=32-byte instance per point; vertex shader
  billboarding and fragment SDF markers;
- line: instanced endpoint pairs expanded to constant-pixel-width quads in the
  vertex shader;
- regular surface: retained height/scalar data and cached index LODs;
- wireframe: retained unique-edge segments, not polygon line mode;
- axes/text: retained glyph atlas and small projected instance buffers;
- colormaps: retained 1D textures;
- lighting: ambient plus directional, not PBR.

Depth is mandatory. Prefer `Depth32Float` and fall back to `Depth24Plus` only
after querying format support. Use 4x MSAA for static/balanced quality when
supported and 1x for dense interactive modes.

Offscreen export performs one padded texture-to-buffer copy and one asynchronous
map after submission. Interactive targets contain no map, blocking poll,
readback, or CPU image upload.

#### GPU LOD policy

LOD improves interaction but cannot silently change publication output.

- build regular-surface stride levels such as 1/2/4/8 once and select them from
  projected cell size with hysteresis;
- split very large surfaces into fixed patches for frustum culling;
- retain full scatter instances and, above the point budget, use a deterministic
  GPU filter/compaction path;
- report input, visible, culled, and drawn counts plus the chosen LOD;
- default static/publication output to full source geometry unless the user
  explicitly selects sampling.

The 10M-scatter interactive target assumes bounded, diagnosed drawing. It does
not claim that 10M fully overdrawn markers fit in a 16.7 ms frame.

### 9. Interaction owns one authoritative camera

Camera state lives in the core 3D session, never independently in winit, GPUI,
or web adapters.

MVP controls:

| Input | Action |
| --- | --- |
| Left drag | Orbit |
| Right or middle drag | Pan |
| Wheel | Zoom |
| Click below drag threshold | Pick/select |
| Double-left or Escape | Reset view |

Camera changes invalidate matrices, Axis3 decorations, and presentation only.
They must not resolve data, retriangulate, rebuild BVHs, or upload geometry.

The 2D `screen_to_data()` contract is not reused in 3D because a screen point
maps to a ray. Add explicit APIs:

```rust
screen_ray(position) -> ScreenRay3D
project3d(point) -> ProjectedPoint3D
unproject_at_depth(position, depth) -> Point3D
pick(position) -> HitResult3D
```

The initial cross-backend picker uses retained world-space BVHs and ray tests.
GPU ID/depth picking is a later optimization. Stale results carry scene and
camera generations and are discarded.

Axis labels are 2D overlays. Panes, cube edges, and grid lines participate in
depth. Outer Axis3 protrusions remain stable while orbiting to prevent layout
feedback and label jitter.

2D and 3D series cannot share one axes in the MVP. Separate static subplot cells
may contain 2D and 3D plots because each child owns its camera and depth buffer.

### 10. Experimental feature and release policy

Development and the first alpha use:

```toml
ruviz = { version = "...", features = ["three-d"] }
```

Feature composition:

| Features | Behavior |
| --- | --- |
| `three-d` | CPU/static 3D |
| `three-d,gpu` | Direct offscreen wgpu 3D |
| `three-d,interactive-gpu` | Native interactive 3D |
| `full` | Includes `three-d` |

The stable usability target is for the canonical code examples to work with the
normal/default ruviz dependency. After an alpha/RC feedback cycle, add
`three-d` to the default feature set if compile size, platform support, and API
stability gates pass. Until then, missing-feature errors and docs must show the
exact Cargo feature declaration.

## AI-callability acceptance gate

Every canonical workflow is scored 0-2. A release requires at least 18/20 and no
zero.

| Dimension | Two-point criterion |
| --- | --- |
| Name discovery | The requested chart maps to one public function |
| Argument order | Every geometry function uses `x, y, z` |
| Canonicality | Docs present exactly one preferred construction path |
| Type inference | Examples need no explicit generic arguments |
| Boilerplate | One import and one plotting expression |
| Defaults | Required data alone produces a useful plot |
| Styling | Common styles are plain fluent methods |
| Terminal action | `save(path)` and synchronous `show()` are direct |
| Diagnostics | Errors name the function, argument, expected, and actual value |
| Feature recovery | Missing optional support prints the exact Cargo feature |

Hard rules:

- No canonical example uses a config object, macro, tuple input, async runtime,
  backend choice, or `.end_series()`.
- One concept has one spelling.
- x/y/z angles and units are never implicit.
- Rustdoc and example code are compile-tested.
- A 12-prompt small-model corpus must achieve at least 90% first-attempt
  compilation at temperature zero before the stable release.

Representative corpus prompt:

```text
Plot x, y, and z as a 3D scatter plot and save it to plot.png.
```

Required canonical answer:

```rust
scatter3d(&x, &y, &z).save("plot.png")?;
```

## Module plan

New core modules:

```text
src/core/plot3d/
  mod.rs          Plot3D orchestration and public terminal behavior
  builder.rs      concrete line/scatter/surface/wireframe builders
  types.rs        camera, projection, axes, bounds, diagnostics
  resolve.rs      coherent owned frame snapshots
  layout.rs       Axis3 viewport, panes, ticks, and overlay anchors
  prepared.rs     granular retained caches
  backend.rs      truthful CPU/GPU capability resolution
  interaction.rs  camera events and session state
  picking.rs      rays, BVHs, and hit results
```

High-level plot lowering:

```text
src/plots/three_d/
  mod.rs
  data.rs
  scatter.rs
  line.rs
  surface.rs
  wireframe.rs
```

Backend-neutral scene and CPU renderer:

```text
src/render/three_d/
  mod.rs
  scene.rs
  math.rs
  overlay.rs
  software/
    mod.rs
    clip.rs
    raster.rs
    shading.rs
```

GPU renderer, behind `gpu`:

```text
src/render/three_d/gpu/
  mod.rs
  renderer.rs
  resources.rs
  pipelines.rs
  shaders/
    mesh.wgsl
    line.wgsl
    point.wgsl
```

Initial existing-file changes are limited to registration and shared leaf
utilities:

- `Cargo.toml`
- `src/lib.rs`
- `src/core/mod.rs`
- `src/core/error.rs`
- `src/plots/mod.rs`
- `src/render/mod.rs`
- `src/export/svg.rs`
- `src/export/svg_to_pdf.rs`
- shared tick formatting and text measurement utilities

Do not route 3D through `src/core/plot/parallel_render.rs`,
`src/core/plot/mixed_render.rs`, or the existing 2D SVG series matches.

## Cache and diagnostics contract

Use separate keys so dirty domains are observable:

| Key | Includes | Rebuild |
| --- | --- | --- |
| Geometry | source identity/version, shape, axis scale/limits, sampling | normalized vertices, indices, normals, BVH |
| Appearance | material, colors, colormap/range | uniforms or color texture |
| Layout | canvas, DPI, text, legend/colorbar | viewport and render targets |
| View | camera and viewport | camera uniform and Axis3 projected decorations |
| Image | all output-affecting keys | final static image |

Required counters:

```text
scene_compiles
triangulations
normal_recomputations
bvh_rebuilds
vertex_upload_bytes
index_upload_bytes
buffer_creations
camera_uniform_writes
draw_calls
points_submitted
triangles_submitted
primitives_culled
readback_bytes
actual_backend
fallback_reason
sampling_mode
```

A camera-only frame must assert:

```text
scene_compiles          = 0
triangulations          = 0
normal_recomputations   = 0
bvh_rebuilds            = 0
vertex_upload_bytes     = 0
index_upload_bytes      = 0
buffer_creations        = 0
camera_uniform_writes   = 1
```

## Delivery plan

Each row is intended to be independently reviewable and mergeable.

| ID | Deliverable | Depends on | Primary acceptance gate |
| --- | --- | --- | --- |
| 3D-00 | ADR, API contract, feature flag, compile examples | none | Canonical examples and feature matrix compile |
| 3D-01 | Benchmark manifest and diagnostics schema | 3D-00 | Cold/warm/update boundaries emit comparable structured data |
| 3D-02 | Bounds, scales, camera, projection, clipping math | 3D-00 | Numeric/property tests including large offsets pass |
| 3D-03 | Data ingestion, typed errors, concrete builders | 3D-02 | AI-callability compile-pass/fail suite passes |
| 3D-04 | ResolvedFrame3D, primitive lowering, cache keys | 3D-03 | Camera/style/data dirty-domain tests pass |
| 3D-05 | Axis3 layout, ticks, labels, panes | 3D-02, 3D-04 | DPI/theme/view golden layout tests pass |
| 3D-06 | Software depth renderer for mesh/line/points | 3D-02, 3D-04 | Exact depth, fill-rule, determinism tests pass |
| 3D-07 | PNG/Image plus hybrid SVG/PDF export | 3D-05, 3D-06 | PNG/SVG/PDF semantic parity and goldens pass |
| 3D-08 | Direct retained wgpu offscreen renderer | 3D-01, 3D-04, 3D-06 | Required-adapter CPU/GPU differential tests pass |
| 3D-09 | Backend routing, retention, device loss, readback | 3D-08 | `actual_backend=gpu3d`; camera upload invariants pass |
| 3D-10 | Core camera interaction, snapshots, CPU picking | 3D-05, 3D-09 | Orbit/pan/zoom/reset/pick tests pass |
| 3D-11 | Native winit and GPUI adapters | 3D-10 | 2D controls unchanged; frame coalescing works |
| 3D-12 | Web input adapter and worker coalescing | 3D-10 | WASM compile and main/worker event parity pass |
| 3D-13 | Direct native/WebGPU presentation | 3D-09, 3D-11/12 | No per-frame GPU readback or CPU texture upload |
| 3D-14 | Docs, gallery, migration guide, alpha release | all MVP rows | Platform/package/docs/performance release gates pass |

### Milestone M0: contract and baseline

Includes 3D-00 and 3D-01.

Deliver:

- `three-d` feature contract;
- canonical API compile tests;
- deterministic dataset manifests and hashes;
- benchmark schema extended from the current plotting runners;
- baseline 2D measurements and fixed 3D performance runner definition;
- actual-backend and cache/upload diagnostics.

No public performance claim is allowed at M0.

### Milestone M1: math, API, and retained scene

Includes 3D-02 through 3D-04.

Deliver:

- orthographic and perspective camera math;
- f64 axis normalization and f32 local scene coordinates;
- fixed surface topology and normals;
- public builders and typed validation;
- backend-neutral mesh/line/point batches;
- granular prepared caches.

No raster output is required until the scene and invalidation contracts are
tested.

### Milestone M2: correct static MVP

Includes 3D-05 through 3D-07.

Deliver:

- automatic Axis3;
- opaque scatter, line, surface, and wireframe;
- deterministic CPU depth output;
- PNG/Image;
- hybrid SVG/PDF;
- orthographic and perspective views;
- rustdoc examples and exact CPU goldens.

All existing 2D golden images must remain unchanged.

### Milestone M3: retained GPU

Includes 3D-08 and 3D-09.

Deliver:

- indexed surface mesh pipeline;
- instanced points and thick line segments;
- depth and MSAA targets;
- retained resource cache;
- camera uniform updates;
- static readback;
- hard adapter/backend assertions;
- device loss and resize handling.

No result may claim GPU execution after falling back to CPU.

### Milestone M4: interaction

Includes 3D-10 through 3D-13.

Deliver:

- core orbit/pan/zoom/reset;
- camera snapshots and keep-view behavior;
- CPU ray/BVH picking;
- native and web event adapters;
- direct GPU presentation before advertising high-performance interaction.

An offscreen-GPU/readback/CPU-blit adapter may exist as a correctness fallback,
but it is not the performance endpoint.

Current frontend facts:

- the native/GPUI image path still consumes a CPU image;
- the macOS GPUI "surface fast path" converts CPU RGBA into YUV and is not a
  direct wgpu texture bridge;
- the web crate currently paints through Canvas2D/ImageData and does not create
  a WebGPU canvas surface.

Therefore:

- web gets an async WebGPU canvas session and retains the existing Canvas2D
  fallback;
- GPUI first gets a clearly diagnosed `GpuReadbackFallback`;
- platform-specific zero-copy GPUI interop is a later adapter milestone and
  isolated from the safe core renderer;
- no adapter is described as direct or fast unless frame diagnostics prove
  there was no readback/CPU upload.

### Milestone M5: alpha

Includes 3D-14.

Ship as an opt-in alpha or RC. Publish:

- supported-backend table;
- explicit MVP limitations;
- raw benchmark artifacts;
- Matplotlib migration examples;
- Makie comparison examples;
- deterministic gallery images;
- known platform gaps.

### Milestone M6: stable/default decision

After at least one alpha/RC feedback cycle:

- resolve all P0/P1 correctness and resource-lifetime defects;
- verify Metal, Vulkan, DX12, and claimed WebGPU paths;
- confirm API corpus results and error quality;
- confirm no unresolved depth/precision leaks;
- consider adding `three-d` to default features.

Transparency, volumes, arbitrary meshes, and advanced lighting remain separate
projects and do not block the opaque stable MVP.

## Verification plan

### Fast unit and property tests

- canonical orthographic/perspective projections;
- camera handedness and near/far mapping;
- project/unproject at explicit depth;
- orbit pole clamps and invalid camera values;
- log/reversed x/y/z scales once enabled;
- grid triangle count, indices, winding, normals, and wireframe deduplication;
- NaN holes/gaps and infinity rejection;
- checked u32 index and allocation boundaries;
- homogeneous clipping on all six planes;
- overlapping primitives and exact depth ties;
- two triangles forming a crack-free quad;
- CPU serial/parallel byte equality;
- arbitrary finite input never panics;
- camera change preserves geometry/BVH/upload identity;
- data change invalidates geometry;
- style change does not retriangulate;
- device loss drops retained handles.

### API tests

Compile-pass:

- prelude-only canonical functions;
- arrays, slices, `Vec<f32>`, `Vec<f64>`, and integer inputs;
- `Vec<Vec<f32/f64>>` and fixed arrays;
- ndarray/nalgebra behind their existing features;
- camera, labels, limits, save, render, and show;
- multiple 3D series in one plot.

Compile-fail:

```rust
scatter3d(&x, &y);                    // missing z
scatter3d(&x, &y, &matrix);           // z must be 1D
surface(&x, &y, &z_1d);               // z must be 2D
surface(&matrix);                      // no ambiguous implicit-grid overload
Camera3D::default().azimuth(45.0);     // unitless spelling is absent
```

### CPU goldens

- fixed-camera orthographic scatter and Axis3 cube;
- perspective helix;
- opaque saddle/peaks surface with colormap;
- wireframe;
- intersecting/overlapping depth scene;
- dark theme labels and ticks;
- large-offset coordinates;
- degenerate ranges;
- 1x and 2x DPI.

Exact hashes apply to isolated raster layers with text disabled. Pinned
Ubuntu/Rust/font jobs own whole-image goldens.

### GPU correctness

Use a required-adapter CI job. It fails rather than skips when the adapter,
pipeline, or actual GPU backend is unavailable.

Cross-vendor GPU images use semantic probes and tolerant comparisons:

- projected bounds;
- visible primitive counts;
- selected front fragment/depth;
- masked RMS and edge overlap excluding text/AA borders;
- CPU/GPU scene and camera manifests;
- output dimensions;
- `actual_backend == gpu3d`.

Do not use cross-vendor exact-pixel gating.

### Matplotlib and Makie comparison

Commit canonical dataset/camera manifests and hashes. Compare:

- ruviz CPU vs Matplotlib Agg and CairoMakie for static output;
- ruviz GPU vs GLMakie for warm camera interaction;
- semantic landmarks, bounds, occlusion, and matching geometry;
- cold scene and warm camera frames separately.

Matplotlib comparisons must explicitly set full `rcount`/`ccount` for
matched-geometry surfaces and separately report its default 50x50 behavior.
Exclude import/JIT/startup and data generation from warm rendering comparisons.

## Benchmark plan

Deterministic datasets:

| Kind | Sizes |
| --- | --- |
| Empty Axis3/camera | fixed overhead |
| Uniform/clustered scatter | 100K, 1M, 10M points |
| Helix/random-walk line | 100K, 1M points |
| Surface | 100x100, 512x512, 1024x1024 |
| Generated mesh stress | about 20K, 200K, 2M triangles |

Measure these boundaries independently:

- validation and f64 bounds;
- triangulation and normals;
- cold adapter and pipeline;
- cold geometry upload and first frame;
- warm unchanged frame;
- camera-only frame;
- style-only update;
- data-update frame;
- GPU readback;
- PNG encoding and file save.

Record median/p95/p99, CPU submit time, GPU timestamps when available, FPS,
elements/triangles per second, host allocations, GPU bytes, bytes uploaded per
frame, resource counts, draw calls, culled primitives, cache hits, backend,
fallback reason, platform/driver, viewport/DPI, feature set, and dataset hash.

Initial performance targets, ratified or adjusted after M0 on fixed hardware:

| Target | Gate |
| --- | --- |
| 2D performance | no regression above 5% median or 10% p95 |
| GPU 1M scatter orbit, 800x600 | p95 <= 16.7 ms |
| GPU 512x512 surface orbit, 800x600 | p95 <= 16.7 ms |
| GPU 10M scatter orbit | p95 <= 33.3 ms |
| GPU 1024x1024 surface orbit | p95 <= 33.3 ms |
| CPU 100K scatter warm frame | <= 33 ms |
| CPU 1M scatter warm frame | <= 250 ms |
| CPU 512x512 surface warm frame | <= 300 ms |
| CPU 1024x1024 static surface | <= 1.2 s |
| Long orbit | <= 1 MiB host growth across 10K frames |
| Warm frame stability | p99 <= 2x median |
| Competitive sanity | <= 1.5x GLMakie warm camera time on matched hardware |

Wall-clock targets run on fixed benchmark hardware rather than noisy hosted CI.
Criterion and structured results enforce regressions; changes over 10-15%
require investigation and recorded justification.

## CI and packaging

Required feature rows:

```text
--no-default-features
--no-default-features --features three-d
--no-default-features --features three-d,gpu
--no-default-features --features three-d,interactive-gpu
--all-features
```

Required checks:

- formatting, clippy, rustdoc warnings, MSRV 1.92;
- align `clippy.toml`'s declared MSRV with `Cargo.toml` before using clippy
  results as an MSRV gate;
- current 2D tests and exact goldens;
- 3D math/API/validation/cache tests;
- `scripts/check_no_new_production_unwraps.py` wired into CI;
- CPU 3D exact goldens;
- required-adapter llvmpipe/Vulkan correctness;
- Linux/macOS/Windows compile matrix;
- FreeBSD CPU check;
- WASM CPU and WebGPU compile checks;
- packaged external consumer with `features = ["three-d"]`;
- registered examples and deterministic gallery freshness.

Scheduled or dedicated hardware checks:

- property tests and retained regression seeds;
- memory/resource lifetime stress;
- Matplotlib/Makie differential runs;
- full performance matrix on Vulkan, Metal, and DX12;
- Chromium WebGPU before making a browser performance claim.

## Documentation and examples

Add:

```text
examples/doc_scatter3d.rs
examples/doc_line3d.rs
examples/doc_surface3d.rs
examples/doc_wireframe3d.rs
examples/interactive_orbit3d.rs
gallery/three_d/
docs/guide/12_three_d.md
docs/migration/matplotlib-3d.md
```

Every example is registered and compile-tested. Fixed-camera examples feed the
existing rustdoc/gallery/golden asset pipeline.

Documentation must state:

- exact surface row/column orientation;
- orthographic default and explicit perspective;
- opaque-only MVP;
- hybrid SVG/PDF behavior;
- actual backend and fallback diagnostics;
- CPU vs GPU capability table;
- performance-sensitive sampling behavior;
- unsupported mixed 2D/3D axes;
- volume and transparency roadmap status.

## Risk register

| Risk | Mitigation |
| --- | --- |
| Existing 2D match graph becomes harder to maintain | Separate Plot3D pipeline and primitive scene |
| GPU label hides CPU fallback | Mandatory actual-backend diagnostics and hard assertions |
| Camera motion retriangulates or uploads | Dirty-domain cache counters and benchmark assertions |
| Large f64 offsets jitter after f32 upload | Scene-relative normalization and 1e12 regression |
| CPU/GPU depth conventions diverge | Shared contract and canonical depth probes |
| Surface grids overflow u32 or memory | Checked arithmetic, explicit limits, chunking later |
| Transparency renders incorrectly | Opaque-only MVP; reject unsupported alpha |
| SVG surfaces are huge or misordered | Rasterized 3D layer with vector text |
| Axis labels jump during orbit | Stable protrusions, deterministic edge choice, hysteresis |
| GPUI/web still read back every frame | Do not claim high performance until direct presentation |
| GPU tests silently skip | Required adapter job that fails on absence |
| Cross-vendor anti-aliasing differs | Exact CPU tests; semantic/tolerant GPU tests |
| Small models choose the wrong API | One canonical spelling, compile corpus, explicit diagnostics |
| Stale backlog branch regresses main | Do not merge `integration/all-backlog` |

## Explicit non-goals for the opaque MVP

- volume/voxel ray marching;
- transparent surfaces or order-independent transparency;
- shadows, SSAO, PBR, or arbitrary user lighting;
- arbitrary indexed mesh import;
- irregular X/Y surface matrices;
- texture mapping;
- mixed 2D and 3D series in one axes;
- fully vector depth-correct surfaces;
- interactive multi-subplot session;
- zero-copy GPUI/WebGPU presentation before dedicated adapter work;
- GPU picking in the first interaction milestone.

## Findings log

### 2026-07-23

- Created `feat/3d-implementation` from clean `main` at `42533c6`.
- Confirmed `src/plots/three_d/mod.rs` is only a placeholder.
- Confirmed the current roadmap already names 3D surface/volume as a deliberate
  next target.
- Audited backlog patch IDs. Scale-aware transforms, resolved frames, resolved
  styles, interaction scaling/reentrancy, hit-test indexing, DPI/layout, backend
  truth, and GPUI coordinate events are already squash-merged into `main`.
- Decided not to merge `integration/all-backlog`; it is stale and would regress
  current fixes.
- Counted extensive 2D `SeriesType`/`ResolvedSeries` coupling and chose a
  separate internal Plot3D pipeline.
- Confirmed `glam` is already available for shared camera math.
- Confirmed the current public GPU preference resolves to Skia and the existing
  GPU renderer performs coordinate compute followed by CPU readback. A direct
  retained wgpu renderer is required.
- Audited the current GPU pipeline, buffer, target, and capability code. Its
  cache key/layout, depth discovery, offscreen usage, and capability guesses are
  not sufficient for 3D and will be replaced in a separate 3D GPU foundation.
- Confirmed the current GPUI and web presentation paths are CPU-image paths, so
  retained GPU rendering alone is not enough to claim fast interaction.
- Defined GPU primitive paths: indexed surfaces, instanced billboard points,
  instanced thick segments, unique-edge wireframes, retained colormaps, depth,
  and optional MSAA.
- Defined diagnosed, deterministic interaction LOD while keeping publication
  output full-resolution by default.
- Ran the repository pre-commit checks for the plan commit. Rustfmt, clippy, and
  documentation-example validation passed. The hook reported that
  `clippy.toml` still declares Rust 1.87 while `Cargo.toml` declares MSRV 1.92;
  align them in 3D-00 or a preceding maintenance change.
- Chose Matplotlib for familiar x/y/z names and camera angles, but not its
  2D-projection renderer, average-z ordering, or default 50x50 surface
  downsampling.
- Chose Makie's scene/camera/primitive/backend separation, orthographic default,
  and vector-x/vector-y/matrix-z surface semantics.
- Chose a deterministic software z-buffer as the CPU reference renderer.
- Chose hybrid SVG/PDF export with a rasterized depth-tested 3D layer and vector
  text.
- Made small-model API reliability a release gate with a compile-tested prompt
  corpus.

## Primary references

- [Matplotlib mplot3d](https://matplotlib.org/stable/api/toolkits/mplot3d.html)
- [Matplotlib Axes3D](https://matplotlib.org/stable/api/toolkits/mplot3d/axes3d.html)
- [Matplotlib plot_surface](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.plot_surface.html)
- [Makie architecture](https://docs.makie.org/stable/explanations/architecture)
- [Makie Axis3](https://docs.makie.org/stable/reference/blocks/axis3.html)
- [Makie scatter](https://docs.makie.org/stable/reference/plots/scatter.html)
- [Makie surface](https://docs.makie.org/stable/reference/plots/surface.html)
- [Makie backends](https://docs.makie.org/stable/explanations/backends/backends.html)
- [wgpu SurfaceCapabilities](https://docs.rs/wgpu/29.0.1/wgpu/struct.SurfaceCapabilities.html)
- [wgpu SurfaceTarget](https://docs.rs/wgpu/29.0.1/wgpu/enum.SurfaceTarget.html)
- [wgpu StagingBelt](https://docs.rs/wgpu/29.0.1/wgpu/util/struct.StagingBelt.html)
- [WebGPU specification](https://www.w3.org/TR/webgpu/)
