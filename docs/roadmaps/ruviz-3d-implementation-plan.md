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

## Current implementation status

| Work item | Status on `feat/3d-implementation` |
| --- | --- |
| 3D-00 | Complete: exact `3d` feature, exports, canonical compile contract, concrete builders, a dedicated Criterion target, and a deterministic hashed dataset manifest exist |
| 3D-01 | In progress: serializable diagnostics, retained-scene counters, committed dataset hashes, and cold compile/render benchmarks exist; warm-update runner integration remains |
| 3D-02 | Complete for the linear MVP: f64 bounds/normalization, typed camera validation, orthographic/perspective projection, unprojection, screen rays, and homogeneous six-plane clipping are tested |
| 3D-03 | Complete for the linear MVP: common 1D/2D ingestion, structured diagnostics, multi-series builders, explicit limits, retained lowering, compile-pass/fail contracts, and ndarray/nalgebra input probes exist |
| 3D-04 | Complete for the linear MVP: coherent owned frames, stable dirty-domain keys, normalized primitive lowering, geometry/appearance retention, and lazy deterministic surface BVH preparation exist |
| 3D-05 | In progress: automatic camera-projected Axis3 viewport, panes, box, grid, ticks, labels, title, DPI scaling, and collision nudging exist; theme/view goldens remain |
| 3D-06 | In progress: deterministic tiled depth rendering for mesh/line/points, 24-bit depth ties, top-left fill, perspective-correct attributes, shading, 1x/4x sampling, and a committed exact isolated-layer hash exist; the full exact-golden/property corpus remains |
| 3D-07 | In progress: Image/PNG and hybrid SVG/PDF output are live and tested, with large-offset orthographic/perspective semantic probes; whole-image goldens and broader semantic parity remain |
| 3D-08 | In progress: independent direct-wgpu offscreen mesh, instanced line, and instanced point pipelines render through queried RGBA/depth/MSAA attachments and pass the local required-Metal-adapter coverage probe; cross-vendor required-adapter jobs and the full differential corpus remain |
| 3D-09 | In progress: explicit `render_gpu()`, truthful `gpu3d` diagnostics, bounded geometry/appearance caches, persistent device/pipelines/attachments, one-write camera frames, static readback, resize recreation, next-frame device-loss recreation, and native direct presentation exist; Auto routing and stress tests remain |
| 3D-10 | Complete: one authoritative retained session provides orbit/pan/zoom/reset, portable camera snapshots, replacement-data keep-view, process-unique generation-safe point/line/surface picking, interactive CPU frames, and diagnosed GPU-readback frames |
| 3D-11 | Complete: native winit now presents retained geometry and Axis3 directly through wgpu, while GPUI shares the retained core controls and truthfully identifies its image-backed GPU-readback fallback |
| 3D-12 | Complete for M4: matching Canvas2D and direct-WebGPU adapters exist for main-thread canvas and worker OffscreenCanvas, the shipped WASM build contains them, and a 500-event Chromium burst test proves latest-pointer/one-frame coalescing |
| 3D-13 | Complete for M4: native winit and Chromium main/worker sessions present retained Axis3 scenes as `gpu3d-surface` with zero readback, zero CPU framebuffer upload, no Canvas2D paint, and cumulative present/upload diagnostics; broader platform evidence remains an M5 gate |
| 3D-14 | Not started |

`render`, `render_png_bytes`, `save`, and `render_to_svg` now execute the
deterministic CPU 3D backend. `save` selects PNG, hybrid SVG, or hybrid PDF from
the extension. With `interactive-gpu`, native `show` opens the retained direct
wgpu surface adapter; without that feature it returns an
interaction-unavailable error. GPUI and web currently expose image-backed
correctness adapters. No 3D call routes through the 2D series match graph.

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
DataLengthMismatch3D { operation, x_len, y_len, z_len, series_index }
GridShapeMismatch {
    operation,
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
ruviz = { version = "...", features = ["3d"] }
```

Feature composition:

| Features | Behavior |
| --- | --- |
| `3d` | CPU/static 3D |
| `3d,gpu` | Direct offscreen wgpu 3D |
| `3d,interactive-gpu` | Native interactive 3D |
| `full` | Includes `3d` |

The stable usability target is for the canonical code examples to work with the
normal/default ruviz dependency. After an alpha/RC feedback cycle, add
`3d` to the default feature set if compile size, platform support, and API
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
adapter_name
sample_count
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

- `3d` feature contract;
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

- native winit presents directly through wgpu;
- the GPUI image path still consumes a CPU image;
- the macOS GPUI "surface fast path" converts CPU RGBA into YUV and is not a
  direct wgpu texture bridge;
- the web crate retains Canvas2D/ImageData as a correctness fallback and also
  exposes direct WebGPU canvas and worker-owned OffscreenCanvas sessions.

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
- consider adding `3d` to default features.

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
--no-default-features --features 3d
--no-default-features --features 3d,gpu
--no-default-features --features 3d,interactive-gpu
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
- packaged external consumer with `features = ["3d"]`;
- registered examples and deterministic gallery freshness.
- final `greptile review --agent`; actionable findings are fixed or explicitly
  recorded in this plan before the alpha is declared complete.

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
- Named the experimental Cargo feature exactly `3d` at user request. Because
  the key begins with a digit it is quoted in `Cargo.toml`; Rust feature gates
  use `#[cfg(feature = "3d")]`.
- Implemented the first feature/API/math slice: the four top-level functions,
  concrete builders, multi-series chaining, common styling and labels, typed
  camera validation, f64 scene normalization, and wgpu-depth-range projection.
- A compile test exposed that `&[T]` variables inferred unsized `[T]` at the
  canonical `&X + ?Sized` boundary. Added unsized slice ingestion so callers do
  not need an artificial double borrow.
- Replaced the old heatmap-specific ragged-row ingestion string with structured
  `RaggedData2D` context. Surface and wireframe errors now name the originating
  operation and the exact row, expected column count, and actual count.
- Added `operation` to 3D length/shape errors after validation showed that the
  original field set could not produce the required function-specific
  diagnostics for both surface and wireframe.
- Verified the slice with no defaults, normal defaults, and `3d` plus defaults.
  The 3D API integration suite passes in both feature configurations; the
  no-default `3d` library suite passes 1,261 tests; rustdoc passes with warnings
  denied.
- Attempting to align `.clippy.toml` from Rust 1.87 to the crate MSRV 1.92
  enabled 45 pre-existing `collapsible_if` findings across unrelated 2D code.
  Kept the existing Clippy MSRV for this slice and made the alignment a separate
  maintenance prerequisite rather than mixing a broad 2D rewrite into 3D-00.
- Added inverse projection and normalized data-space screen rays with explicit
  wgpu depth semantics (`0..=1`). Orthographic and perspective round trips are
  tested on large-offset data; f64 offset removal happens before f32 camera
  math, and the representative inverse error stays below `1e-3` data units.
- Added Sutherland-Hodgman homogeneous triangle clipping and homogeneous segment
  clipping for all six wgpu clip planes. Property tests cover arbitrary finite
  clip-space triangles without panics or non-finite generated vertices.
- Split retained scene state into appearance-independent normalized geometry and
  styled scene batches. Camera-only updates reuse the exact scene allocation;
  style-only updates reuse geometry allocations; data changes rebuild geometry.
- Lowered high-level plots to the backend-neutral point/segment/indexed-mesh
  vocabulary. Surface cells use the specified row-major split, smooth normals
  are area-weighted, flat shading duplicates triangle vertices, wireframes emit
  unique row/column edges, and source indices survive lowering for later
  picking.
- Confirmed NaN semantics during lowering: line segments do not bridge gaps and
  each surface triangle is removed only when one of its own vertices is
  non-finite. Infinity remains an ingestion error.
- `SurfaceSampling::Auto` remains full-resolution for this static preparation
  path. `MaxGrid` deterministically retains both endpoints and reports
  `sampling_mode=max-grid`; interactive diagnosed LOD still belongs to the GPU
  and interaction milestones.
- Added the public serializable `RenderDiagnostics3D` schema and a hidden
  benchmark compile terminal. Before a renderer executes it truthfully reports
  `actual_backend=unresolved`, zero draw/readback/upload counters, and the
  retained-scene compilation work that actually occurred.
- Enforced the opaque MVP at validation time: translucent fixed colors and
  translucent colormap entries are rejected instead of producing
  order-dependent output.
- Kept Axis3 separate from the 2D layout graph while reusing the existing leaf
  tick formatter, text renderer, line/polygon drawing, DPI conversion, and
  image compositor. The retained Axis3 snapshot contains one resolved viewport
  and camera plus projected panes, grid lines, box edges, tick marks, tick
  labels, axis labels, and title anchors.
- The first rendered perspective preview exposed colliding endpoint labels at
  the common projected box corner. Tick directions now use each projected
  edge's outward-facing perpendicular, and a deterministic text-box estimate
  nudges later labels outward until anchors no longer overlap.
- Implemented the CPU reference renderer as 32x32 disjoint tiles with
  24-bit-quantized depth, stable primitive-ID tie breaking, six-plane
  homogeneous clipping, a top-left triangle rule, perspective-correct scalar
  and normal interpolation, screen-space thick/dashed lines, depth-tested
  marker billboards, and deterministic ambient-plus-Lambert shading.
- Static output uses four fixed sample locations per pixel; the renderer also
  has a one-sample mode for the later interactive CPU fallback. A feature-gated
  test proves serial and Rayon tile execution produce byte-identical raster
  layers.
- Stable IDs intentionally assign points and lines before surfaces so coplanar
  wireframes and markers win exact quantized-depth ties without a
  camera-dependent epsilon or submission-order race.
- Hybrid SVG and PDF contain one PNG-encoded, fully depth-tested 3D raster layer
  plus vector panes, grid, box, ticks, labels, and title. PNG, SVG, and PDF
  terminal tests pass, and diagnostics truthfully report
  `actual_backend=cpu3d`, zero readback bytes, draw calls, submitted geometry,
  and culled primitives.
- Added the canonical `examples/3d_surface.rs` example and a dedicated
  `benches/three_d.rs` Criterion target. The short profile covers 10K/100K
  scatter and 32/100 surfaces; `RUVIZ_3D_BENCH_FULL=1` enables 1M scatter and
  512/1024 surfaces.
- Recorded the first local cold/export baseline on Apple M4/32 GiB,
  aarch64 macOS, Rust 1.94.1, release mode, 640x480, features `3d,parallel`.
  Median Criterion times were 6.64 ms for 10K scatter, 35.13 ms for 100K
  scatter, 325.69 ms for 1M scatter, 5.99 ms for a 32x32 surface, 9.02 ms for
  100x100, 81.07 ms for 512x512, and 306.66 ms for 1024x1024. These are cold
  builder-to-image measurements including ingestion, scene compilation, 4x
  sampling, Axis3 text, and image composition, so they are deliberately not
  labeled as the later retained warm-frame measurements.
- Added a deterministic benchmark dataset manifest and generator. Each case
  hashes the dimensions and little-endian `f64` bit patterns with FNV-1a so
  performance results cannot silently change their workload. The committed
  cases cover 100K/1M/10M scatter and 100/512/1024 square surfaces, with fixed
  viewport and camera contracts.
- Added a `trybuild` API matrix designed as a small-model gate. The canonical
  call compiles through labels, explicit limits, camera, render, PNG bytes,
  SVG, save, and show; missing `z`, matrix scatter input, implicit surface
  grids, one-dimensional surface `z`, and unitless camera angles fail to
  compile. Runtime ecosystem probes also cover ndarray arrays/views and
  nalgebra vectors/matrices behind their existing feature flags.
- The compile matrix exposed that the first builder slice omitted explicit
  `xlim`, `ylim`, and `zlim`. Added those controls with finite ascending
  validation. Limit changes invalidate geometry and layout because
  normalization and ticks change, but do not invalidate appearance.
- Implemented a lazy deterministic triangle BVH for retained surface geometry.
  Normal rendering does not build it, camera-only changes reuse the exact BVH
  allocation, and geometry changes invalidate it. CPU screen picking returns
  the series, triangle/source indices, barycentric coordinates, data-space
  position, and ray distance; point and line picking remain M4 work.
- Added an exact four-sample raster-layer hash over a mixed
  triangle/line/point depth scene. The probe locks down depth ties, top-left
  fill, clipping, shading, line coverage, marker coverage, and deterministic
  compositing without coupling the fixture to font rasterization.
- Added a large-offset semantic render probe for both orthographic and
  perspective cameras. Both outputs contain non-background geometry, keep the
  requested dimensions, and differ by projection as expected after f64
  normalization.
- Added the requested final Greptile gate: after the complete M5 alpha
  implementation, run `greptile review --agent`, address actionable findings,
  and record the result in this plan.
- Verified this slice with the no-default, `3d`, `3d,parallel`, and
  all-features compile rows; strict 3D Clippy; rustdoc with warnings denied;
  the small-model `trybuild` matrix; ndarray, nalgebra, PDF, and parallel
  determinism probes; and the complete no-default 3D library suite. The library
  suite passed 1,304 tests with zero failures.
- The planned production-unwrap checker is not yet a valid release gate for
  this long-lived branch. It diffs all 3D work against `main` and lexically
  reports test-only `expect` calls inside `#[cfg(test)]` modules as production
  because those modules live in `src/`. Before 3D-14, teach the checker to
  exclude test-only syntax (or provide an audited branch baseline), then remove
  any genuine production findings and wire the corrected check into CI.
- Re-audited the existing wgpu feature before M3. The reusable boundary is
  wgpu's instance/adapter/device/queue request machinery, not the existing 2D
  `GpuBackend`, `DeviceSelector`, pipeline cache, or compute renderer. Those
  layers impose guessed 4K-texture and 256-MiB-buffer admission thresholds,
  use `Features::empty()` as if it were a capability, expose a color-only
  format list to depth creation, and center the rendering path on compute plus
  CPU readback. The 3D renderer therefore gets an independent
  `render::three_d::gpu` context, exact attachment-format queries, explicit
  depth/MSAA targets, direct render passes, and its own retention diagnostics.
- Implemented the first direct-wgpu 3D path behind `3d,gpu`. The renderer owns
  an independent device-loss-aware context, exact color/depth format checks,
  four-sample MSAA when both attachments advertise it, persistent offscreen
  and padded readback targets, one camera uniform, indexed mesh draws,
  instanced screen-width line quads, and instanced SDF marker billboards.
  `render_gpu()` is an explicit request and never relabels CPU output; the
  canonical `render()` remains the deterministic CPU reference.
- Split GPU retention by geometry and appearance identity. Camera/DPI changes
  update the per-frame camera/layout uniform without rebuilding material
  buffers; style changes can replace appearance without uploading geometry.
  Caches retain only the current geometry and appearance to prevent unbounded
  scene churn, while a shared static renderer reuses the device, pipelines,
  attachments, and readback buffer across explicit static calls.
- The local required-adapter job executed all four high-level primitives on
  Apple Metal and asserted `actual_backend=gpu3d`, a non-empty adapter name,
  queried sample count, draw/upload/readback counters, and visible output. A
  retained second camera frame performed one camera uniform write with zero
  vertex uploads, index uploads, or buffer creations.
- Added a backend-neutral CPU/GPU semantic probe over an isolated surface
  layer. It requires at least 0.80 projected coverage intersection-over-union
  and passed on Metal. Visual inspection also confirmed the surface,
  wireframe/line quads, and point billboards are present under the same Axis3.
- WGSL validation caught and corrected illegal swizzle assignment before any
  GPU success was reported. The required-adapter test failed hard on the
  rejected submission, demonstrating that the path does not silently fall
  back or claim `gpu3d` after validation failure.
- Predefined line dash styles fit the direct GPU material's eight-entry
  uniform. Longer custom patterns now return an explicit unsupported-GPU
  error instead of being truncated; the deterministic CPU renderer remains
  the reference for those patterns until a variable-length GPU material is
  added.
- Added a separately labeled `3d/gpu/scene-upload-export` Criterion profile.
  It reuses the process device, pipelines, and attachments, but every
  iteration still includes ingestion, scene lowering, geometry upload, one
  static readback, Axis3 composition, and diagnostics; it is not a warm orbit
  measurement. On the same Apple M4 release setup, the quick profile measured
  100K scatter at a 15.69 ms midpoint and a 100x100 surface at 6.54 ms,
  compared with the earlier CPU cold/export midpoints of 35.13 ms and 9.02 ms.
- Added a retained `3d/gpu/retained-camera-no-readback` session and Criterion
  profile. It resolves and uploads the scene once, then measures Axis3 camera
  layout, one uniform write, direct draw submission, and GPU completion without
  image composition or texture readback. Its invariant test requires zero warm
  scene/triangulation/normal/BVH/upload/buffer/readback work. On Apple M4 the
  quick midpoint was 6.67 ms for 100K scatter and 1.60 ms for a 100x100
  surface, both within the provisional 16.7 ms orbit budget at these sizes.
- Completed the local M3 checkpoint matrix with `3d,gpu`: all 1,328 library
  tests, all 11 public 3d API tests, and all four required-adapter integration
  tests passed. The adapter tests exercise mixed and individual mesh, line,
  wireframe, and point draws; PNG and hybrid SVG export; truthful adapter,
  sample-count, upload, draw, and readback diagnostics; and a retained warm
  camera frame with no readback or resource upload. The dedicated resize and
  forced-device-loss unit probes also passed, as did the serde diagnostics
  schema probe.
- The M4 camera audit found that azimuth/elevation/zoom alone cannot represent
  a portable pan. `Camera3D` now carries an optional `f64` data-space look-at
  target, defaulting to the resolved bounds center. Core pan unprojects the
  dragged target plane and writes that target back into the camera, so camera
  snapshots contain the complete view and frontend adapters need no hidden
  camera state.
- Added a retained `InteractivePlot3DSession` with the small direct methods
  `orbit`, `pan`, `zoom_by`, `reset_view`, `camera_snapshot`,
  `restore_camera`, `pick`, and `render`, plus a compact adapter-neutral event
  enum. Left drag orbits, middle/right drag pans, positive wheel deltas zoom
  in, clicks below the three-pixel drag threshold pick, and double-left or
  Escape resets.
- Extended the CPU picker across all opaque MVP primitives. Scatter uses the
  rendered marker radius, line and wireframe use clipped screen-space segment
  distance and rendered width, and surfaces retain the lazy triangle BVH.
  Candidates are resolved by projected depth with deterministic point/line/
  surface tie priority. Results carry scene and camera generations so adapters
  can reject stale asynchronous hits.
- Added a retained GPU image path to the interaction session, but deliberately
  label it `gpu3d-readback-fallback` with a fallback reason and nonzero
  readback diagnostics. It is a correctness bridge for current CPU-image
  frontends and does not satisfy the later direct-presentation gate.
- A generation audit found that per-session counters starting from the same
  value were insufficient: a hit from a replaced plot could accidentally match
  the new session. Scene generations now come from a checked process-wide
  allocator, while camera generations remain local to that unique scene.
  `interactive_session_with_view(snapshot)` creates a new unique scene while
  restoring the previous camera, which is the explicit keep-view contract.
- The initial screen-space point and line picker intentionally uses conservative
  rendered envelopes. It does not yet reject the empty corners of non-circular
  marker SDFs or gaps inside dashed lines. Depth ordering, source interpolation,
  clip-volume handling, and stale-result checks remain exact; sub-shape picking
  can be tightened later without changing the public result type.
- Completed the local 3D-10 gate: five focused interaction/session unit tests,
  all 12 public 3d API tests, both small-model compile-test groups, and all five
  required-adapter GPU integration tests pass. Strict `3d,gpu` all-target
  Clippy and warnings-denied rustdoc also pass, apart from the already recorded
  repository-level Clippy MSRV mismatch warning.
- The first `ruviz-gpui` `3d-gpu` compile exposed a pre-existing macOS adapter
  dependency split: the crate directly selected `core-video 0.4`, while the
  pinned workspace GPUI revision consumes `core-video 0.5`, so
  `Window::paint_surface` received two nominally different `CVPixelBuffer`
  types. The direct dependency is now aligned to `core-video 0.5`; strict
  `3d-gpu` Clippy and all 47 GPUI library tests pass with the pinned revision.
- The first wasm `3d-gpu` compile confirmed that the native synchronous renderer
  cannot be reused as a browser singleton: WebGPU buffer/device handles are
  intentionally `!Send`/`!Sync`, so the native `OnceLock<Mutex<_>>` is invalid,
  and synchronous `pollster` initialization is not a browser API. Keep native
  sync GPU terminals off wasm; the direct browser path must be an async
  main-thread/thread-local surface session.
- Strict wasm Clippy also exposed existing target-specific noise outside 3d:
  its zero-sized frame timer triggered `let_unit_value`, wasm's `File` stub
  made two explicit close-before-rename drops look redundant, and the existing
  browser wgpu backend intentionally uses `Arc` around handles that are
  `!Send`/`!Sync` on the single-threaded target. Scope these allowances to the
  exact wasm sites rather than weakening native Clippy.
- Added a native winit adapter behind `3d,interactive-gpu`. It maps left-drag
  orbit, middle/right-drag pan, wheel zoom, click pick, double-click reset,
  Escape reset, resize/DPI changes, and redraw requests into the retained core
  session. Presentation currently converts the diagnosed GPU-readback image
  into softbuffer pixels; its docs and diagnostics call it a correctness
  fallback, never direct presentation.
- Added a GPUI `RuvizPlot3D` component and `plot3d(session, cx)` constructor.
  It uses the same retained session and event semantics, caches camera-stable
  frames, converts component coordinates into backing pixels, and uses CPU
  images or the diagnosed GPU-readback fallback according to features. Direct
  GPUI texture interop remains an isolated post-surface adapter gap.
- Added JavaScript-owned `JsPlot3D` plus main-thread `Web3DCanvasSession` and
  worker-capable `Offscreen3DCanvasSession`. Both adapters expose matching
  pointer, wheel, double-click, reset, selection, resize, render, PNG export,
  and destroy methods over one shared implementation. They intentionally use
  deterministic CPU Canvas2D presentation until the asynchronous WebGPU
  surface session is implemented.
- The adapter checkpoint passes `cargo check --all-features`, strict native
  all-feature Clippy, strict GPUI `3d-gpu` Clippy, strict wasm
  `3d-gpu` Clippy, the full 1,309-test no-default `3d` library suite, both
  focused winit mapping tests, and all 47 GPUI library tests. The only emitted
  warning is the already recorded repository-level `.clippy.toml` Rust 1.87
  versus Cargo Rust 1.92 mismatch.
- The native direct-presentation audit found that a surface-only scene pass
  would silently omit Axis3 because panes/grid and box/ticks/text were composed
  around the GPU image after readback. The implemented surface path therefore
  retains the existing RGBA scene target as a sampled GPU texture, then draws
  GPU pane/grid primitives, the scene texture, GPU box/tick primitives, and a
  retained text atlas into an sRGB swapchain. It requests a
  surface-compatible adapter and never blocks on `device.poll` while
  presenting.
- Native camera frames update the camera uniform and small screen-space Axis3
  vertex buffers. Text glyphs are rasterized into a straight-alpha sRGB atlas
  only when text, theme, font, or DPI changes; a camera-only layout change
  reuses the atlas and reports zero presentation texture upload. Diagnostics
  now separate presentation vertex and texture uploads, surface presents and
  reconfigurations, and queue waits from geometry uploads and readback.
- A local Apple Metal native-display smoke opened
  `examples/interactive_orbit3d.rs`, created the surface-compatible adapter,
  compiled the BGRA presentation shaders, uploaded the retained text atlas,
  and presented continuously for ten seconds without a validation or runtime
  error. Four pure presentation layout/format/atlas tests and all five
  required-adapter offscreen GPU tests also pass, as does the complete
  1,387-test `3d,interactive-gpu` library suite. Cross-platform display runners
  are still required before closing 3D-13.
- The wgpu 29 surface API returns `CurrentSurfaceTexture` statuses rather than
  the older `Result<SurfaceTexture, SurfaceError>`. The native presenter skips
  timeout/occluded frames, reconfigures outdated/suboptimal surfaces at safe
  texture-lifetime boundaries, recreates a lost surface, and rebuilds the
  surface-compatible device/renderer/compositor after device loss.
- The browser audit confirmed that the existing `3d-gpu` compile row is not a
  WebGPU implementation: native synchronous 3d GPU modules remain excluded
  and Canvas2D still performs `ImageData`/`putImageData`. The direct browser
  path must be an async per-JavaScript-realm session, choose the backend before
  claiming the canvas context, render Axis3 and data to a WebGPU surface
  without blocking, and batch pointer/resize work into at most one submission
  per animation frame. Worker GPU handles remain worker-local.
- Browser acceptance counters must distinguish zero interactive readback and
  zero CPU framebuffer upload from legitimate one-time colormap/text-atlas
  texture uploads. A mandatory Chromium WebGPU lane must instrument
  `putImageData`, `copyTextureToBuffer`, `mapAsync`, and texture uploads, while
  a burst test proves main/worker event coalescing and latest-camera wins.
- The first direct-browser code audit found a browser-fatal surface-format
  assumption before runtime testing: wgpu's WebGPU backend advertises base
  canvas formats such as `Rgba8Unorm` and `Bgra8Unorm`, not necessarily their
  sRGB-suffixed variants. Requiring an advertised `*Srgb` format rejects a
  valid browser before `configure`. The browser path must configure an
  advertised base format, request its sRGB view in `view_formats`, and render
  the compositor through that explicit view. A capability-list unit test and
  Chromium screenshot smoke are required before this checkpoint is complete.
- The same audit confirmed that surface-only renderers must not merely report
  zero readback: they must omit the MAP_READ buffer and COPY_SRC texture usage
  entirely. The renderer now reserves those resources only for the explicit
  native image-readback constructor; direct native/browser surface sessions
  use render-attachment plus texture-binding usage only.
- Browser API review simplified the generated contract for small models:
  `WebGPU3DCanvasSession.create(canvas, plot)` and
  `OffscreenWebGPU3DCanvasSession.create(canvas, plot)` are typed async
  factories returning `Promise<...>`; JavaScript uses the standard `WebGPU`
  acronym spelling; and `JsPlot3D.title(...)` matches the Rust and TypeScript
  builders. The generated declarations confirm these exact signatures.
- The npm/WASM build scripts previously used `ruviz-web`'s empty default
  feature set, so all 3d bindings disappeared from the distributable artifact.
  Both wasm-pack and pinned raw-bindgen builds now explicitly enable
  `3d-gpu`. The release build contains `JsPlot3D`,
  `WebGPU3DCanvasSession`, and `OffscreenWebGPU3DCanvasSession`; TypeScript and
  the multi-page Vite demo build successfully with the 3d artifact.
- Replaced the browser-fatal sRGB capability assumption with a base/view
  selection contract. The surface uses the browser's preferred advertised
  `Rgba8Unorm` or `Bgra8Unorm` format, permits its sRGB-suffixed view through
  `view_formats`, builds the compositor for that view, and explicitly acquires
  it for presentation. Unit coverage includes wgpu's actual WebGPU capability
  list (`Rgba8Unorm`, `Bgra8Unorm`, `Rgba16Float`).
- Added a permanent `/3d.html` WebGPU demo and Chromium Playwright gate for
  both main-thread canvas and worker-owned OffscreenCanvas. On local Chromium,
  both adapters initialized and presented as `gpu3d-surface`; a burst of 500
  pointer moves collapsed to one applied move and one presented frame; GPU
  readback, CPU framebuffer upload, and instrumented Canvas2D `putImageData`
  calls remained zero; the authoritative CPU PNG export changed after orbit;
  and explicit teardown completed cleanly. The test passed in 0.823 seconds.
- Browser counters are cumulative rather than last-frame snapshots, allowing
  the smoke test to assert exactly one additional surface presentation for the
  coalesced burst. Texture diagnostics now include the one-time 1 KiB
  colormap upload per mesh as well as presentation-atlas uploads, while warm
  camera frames remain zero-texture-upload frames.
- Remaining browser hardening after the M4 gate is intentionally tracked for
  M5/follow-up: share adapter/device/pipelines per JavaScript realm, preserve a
  camera snapshot across device-loss recreation, expose typed retry state for
  skipped frames, and consider merging scene/compositor command submission
  after measuring whether the current two-submit design misses frame budgets.
- The M5 audit found that 3D-14 remains blocked after direct presentation by
  incomplete warm/update benchmark integration, Axis3/export whole-image
  goldens, cross-vendor GPU evidence, browser smoke tests, feature/platform CI,
  package consumers, documentation/gallery/migration material, structured
  performance artifacts, and prerelease publishing semantics. It also found
  that Cargo auto-discovered `examples/3d_surface.rs` without its required
  feature; the example is now explicitly registered with
  `required-features = ["3d"]`.

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
