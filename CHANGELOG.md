# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Long 2D plot titles now wrap at Unicode line-break opportunities and shrink
  only when two automatic lines do not fit. Authored newlines remain hard
  breaks, the configured title size remains the preferred maximum, and PNG
  and SVG rendering share the same resolved text layout (#175).
- `LegendPosition::Best` now scores a bounded 64×64 screen-space mask of the
  visible line, marker, bar, and error-bar geometry. Dense sampling no longer
  changes the score, unlabeled visible series still reserve space, hidden
  series do not, and PNG and SVG resolve the same inside placement (#176).

## [0.12.0] - 2026-08-22

### Added

- Regular bar charts now support vertical and horizontal orientation end to
  end. Rust adds `.horizontal()` / `.vertical()` convenience methods, while
  Python accepts `bar(..., orientation="horizontal")`; snapshots, notebook
  replay, Wasm, and the TypeScript SDK preserve the same setting. Horizontal
  bars use numeric x values and categorical y labels in raster and SVG output.
- `Plot::boxplot(..)` accepts `.horizontal()` and `.vertical()`, the same
  spelling bar, violin and boxen plots already used. Categories move to the y
  axis, the value axis keeps its scale, and several named boxes stack up the
  axis the way vertical ones spread across it.
- `Plot::invert_x()` / `Plot::invert_y()` flip an axis after its range
  resolves, so they compose with auto-scaled bounds — descending
  `xlim`/`ylim` limits, the previous spelling, required knowing the data's
  extent. On a horizontal categorical chart, `.invert_y()` puts the first
  category at the top so ranked bars read in the order they were given.
- Every categorical builder now spells both directions: strip, swarm, grouped
  and stacked bar builders gain the `.vertical()` their `.horizontal()` was
  missing, so switching back is written the same way everywhere.

- A violin's legend key carries the outline its body is drawn with —
  the configured line colour, or the fill darkened through the shared edge
  rule — instead of the flat swatch it showed. Boxen and pie keys stay flat
  deliberately: a boxen outline matches its base fill and pie wedges carry
  no edge, so a flat key already tells the truth there.
- Python's `boxplot()` gains `show_mean=` and `width_ratio=`, and the
  TypeScript SDK the matching `showMean`/`widthRatio` style keys, carried
  through snapshots, replay and the notebook widget like every other style.
- The bindings speak the new knobs. Python's `boxplot()` accepts
  `orientation="horizontal"` like `bar()` does, `Plot.invert_x()` /
  `invert_y()` land in Python with snapshot round-tripping, and the
  TypeScript SDK mirrors both — `style.orientation` on `boxplot(..)` and
  `invertX()` / `invertY()` on the builder — with the notebook widget
  replaying all of it.

### Changed

- Grouped and stacked bars are drawn with an edge, like every other filled
  patch: both configurations now default to `edge_width: 0.8`, and an
  `edge_color` of `None` derives the edge from the fill through the same rule
  `StyleResolver::patch_edge` applies to a plain bar, rather than meaning "no
  edge". Adjacent stacked segments are separated as a result. Pass
  `.edge_width(0.0)` for the previous flat look.
- `ComputedStyle` carries the theme's `patch_edge_color`, so a filled patch
  drawn through `ComputedSeries::primitives` follows the theme's edge rule
  instead of only the renderer path doing so. Code constructing a
  `ComputedStyle` literally needs the new field; `ComputedStyle::opaque(..)`
  and the new `.with_patch_edge_color(..)` cover it otherwise.

- Grouped and stacked bar legend swatches are stroked with the edge their
  bars actually carry, through the new `ComputedSeries::patch_edge_spec`
  hook — a flat key for an outlined bar misdescribed the picture.
- A horizontal grouped bar chart lays each group out in legend order: the
  first series is the topmost bar of its group, so the group reads
  top-to-bottom the way the legend beside it does. Vertical groups are
  unchanged. (Previously the offsets simply grew upward, so every horizontal
  group listed its bars in the reverse of the legend — the same quirk
  matplotlib's `barh` has and seaborn corrects.)

### Fixed

- `.show_mean(true)` on a box plot draws its mean again. The diamond marker
  existed only in the legacy `PlotRender` path, so through `Plot::boxplot(..)`
  — the path the documentation demonstrates — the setter was accepted and
  drew nothing. Both backends now place the diamond through the shared
  projection, in the same edge ink as the median, in either orientation.
- `BoxPlotConfig::orientation` is honoured end to end. It was read when the
  input was validated and ignored when the box was drawn — the projection put
  the quantiles on y whatever the setting said — so a box plot configured
  horizontal came out vertical, and on a log axis it validated one axis and
  drew against the other. The shared projection now places the quantiles on
  the axis the orientation names, the box claims its slot on the other one,
  and bounds and axis-scale support follow.
- PNG output from `save()` and `render_png_bytes()` now stores the effective
  configured DPI in a standard `pHYs` chunk, so print and page-layout software
  recovers the intended physical figure size. Interactive-session exports —
  the web canvas session and the 3D sessions — stamp the same chunk, so a
  session export and a direct export of the same figure stay byte-identical.
- SVG export no longer writes an empty `<text>` element for an unnamed category
  slot. A lone box plot or violin holds its place on the axis without a label,
  which is what the raster backend already drew.

## [0.11.0] - 2026-08-16

### Added

- Series-aware interaction queries and per-series visibility (#162). The
  interactive session now answers what it already knows: `hit_at(x, y)`
  (web sessions; `hit_test` + `series_label` in core) resolves a pointer
  position to `{seriesIndex, seriesLabel, pointIndex, dataX, dataY,
  distancePx}` for embedder-rendered tooltips, `legend_entry_at(x, y)`
  resolves a legend entry to its series, and `set_series_visible(index,
  visible)` toggles a series without rebuilding the plot — rendering and
  hit testing skip it, its legend entry stays dimmed, axis bounds hold
  still, grouped series toggle together, and restoring reproduces the
  previous frame byte-for-byte. Rust adds `Plot::series_visible(index,
  visible)` for static plots, and the session exposes
  `series_count`/`series_label`/`series_visible`. The TypeScript SDK
  mirrors everything on `CanvasSession` (sync) and `WorkerSession`
  (async), typed as `SeriesHit`.

### Changed

- The built-in hover tooltip leads with the hovered series' legend label
  and formats coordinates adaptively — three decimals in the readable
  range, scientific notation outside `1e-3`–`1e5` — so overlaid curves are
  distinguishable and log-scale values never display as `0.000`.
- `PlotData` merges its `Static(Vec<f64>)` and `SharedStatic(Arc<Vec<f64>>)`
  variants into a single reference-counted `Static(Arc<Vec<f64>>)`. Code
  using `IntoPlotData`/`into_plot_data()` (every builder API) is
  unaffected; only direct `PlotData::Static(vec)` constructions need
  `Arc::new`/`.into()`, and `PlotData::SharedStatic` matches become
  `PlotData::Static`.

### Fixed

- A dashed or dotted line whose on-screen path exceeded roughly ten
  million pixels rendered as nothing: tiny-skia refuses to produce more
  than one million dash segments and then strokes an empty path. Such
  lines are now stroked in phase-continuous chunks, so the pattern runs
  unbroken across the whole series. This also corrects earlier line-chart
  benchmark impressions — a "fast" dense dashed line was fast because it
  was blank; the honest cost is proportional to path length.

## [0.10.0] - 2026-08-16

### Added

- Annotations across every binding: `vline(x)`, `hline(y)`, and
  `annotate_text(x, y, text)` on the Rust `Plot` builder, the Python `Plot`,
  the web SDK's `PlotBuilder` (`annotateText`), and the raw wasm handle,
  with optional styles — color, width, and linestyle for reference lines;
  color and `fontSize` for text. Annotations are recorded under snapshot
  schema version 3, so they survive cloning, worker transfer, notebook
  widget replay, and cross-binding round trips; the un-styled default is
  the core's 1pt dashed gray, defined once as
  `Annotation::reference_line_defaults()` and filled from there by every
  binding, so partial styles can never drift from un-styled lines. The
  Python package exports the annotation snapshot types alongside
  `SeriesSnapshot`.

### Fixed

- Snapshot replay treats malformed foreign annotations the way it treats
  every other foreign field: a missing or null coordinate, a bad style
  value, an unknown kind, or a non-array annotations container is skipped
  instead of throwing wasm-side and blanking the notebook widget. A `null`
  style is accepted as no-style everywhere, empty styles stay out of
  snapshots (making TS and Python snapshots of equivalent plots identical),
  and style numbers that would overflow f32 to infinity are rejected at
  validation in both native layers — while KDE's genuinely-f64 bandwidth
  keeps its full range. An end-to-end browser test pins all of it.
- The GUI behavioral test suites run single-threaded in CI: issue #152's
  render-worker timeout reproduced on a 4-core Windows runner with the
  instrumented message showing the waiters had CPU while the render
  workers did not — four concurrent 3D renders plus four spinning waiters
  oversubscribed the runner. Serializing the suites removes the
  competition without touching the waiting primitive.

## [0.9.0] - 2026-08-16

### Added

- `scatter(..., density=True)` (Rust `.density(true)`, wasm and Python
  bindings included): opt-in density aggregation for large scatters. Points
  are counted into a plot-area grid in one parallel pass, the counts are
  spread over the series' marker footprint — exact for every marker whose
  rows are one centered span (circle, square, diamond, both triangles,
  plus); cross and star use their bounding disk, open markers render
  filled — and each pixel takes the series color at the scatter-equivalent
  alpha `1-(1-a)^covering_markers`, so the result keeps the exact render's
  silhouette while its cost scales with pixels rather than points. The flag
  is tri-state: unset leaves the choice to the plot, and an explicit value
  wins in both directions. Density series report a clear unsupported error
  from SVG export and the GPU backend.
- `Plot::fast(true)` / Python `plot.fast()`: opt-in speed-over-fidelity
  mode. A scatter series holding more points than the plot has visible
  pixels renders through density aggregation, and a marked solid line's
  stroke takes the min/max column reduction a bare solid line always gets
  while every marker stays at its exact position. Everything below those
  thresholds stays byte-identical with fast mode on or off, and default
  rendering is untouched. The performance guide documents the guarantees,
  the marker footprint table, and every limitation, alongside committed
  exact-vs-fast comparison images.
- `docs/benchmarks.md` and `scripts/bench_scatter_vs.py` /
  `bench_scatter_warm.py`: the measured scatter comparison against
  reflex-dev/xy and matplotlib on xy's own benchmark contract, with cold and
  warm tables and reproduction commands.

### Changed

- First render in a process no longer pays the system font scan on macOS and
  Windows: the discovered font file list is cached on disk (scan order
  preserved, guarded by the mtime and size of every font file and every
  directory under the platform font roots) and rebuilt through the same
  fontdb loader in ~9ms instead of ~60-70ms. Any guard mismatch falls back
  to a full scan, so a stale cache can only cost time, never resolve fonts
  differently; `RUVIZ_FONT_CACHE=0` disables it. Linux always scans, because
  fontconfig makes its font set configuration-driven.
- Marker compositing is parallel: the sprite compositor splits the canvas
  into load-balanced horizontal pixel bands under the existing `parallel`
  feature (now enabled in the Python binding), with every pixel seeing the
  exact serial compositing sequence — output is byte-identical and pinned by
  a serial/parallel equality test. A 1M-point scatter drops from ~280ms to
  ~150ms warm, 10M from ~3.3s to ~1.7s.
- Static numeric data is shared (`PlotData::SharedStatic`) instead of being
  cloned three times through handle rebuild, `to_plot_data`, and
  `prepare()` — roughly 480MB less memory traffic per 10M-point render, and
  lower peak memory.

### Fixed

- `size_px(900, 420)` produced a 900x419 canvas: pixel sizes stored as
  inches hit f32 representation noise and truncated. Canvas conversion now
  snaps within 1e-6 relative of an integer — tight enough that genuinely
  fractional sizes, including fitted-frame DPI computations that rely on
  truncation, are untouched.
- The committed README, rustdoc, and gallery media were regenerated for the
  1.5pt default line width 0.8.0 introduced, and the README's hello-plot
  snippet now matches the figure below it.
- The Python extension ships with `parallel` enabled; the documentation
  covers the fork-after-render caveat this brings to Linux
  `multiprocessing` (use the spawn start method), and that the notebook
  widget ignores `fast` and renders exactly.

## [0.8.0] - 2026-08-15

### Added

- A grouped `figure()` presentation call in the Python and web bindings
  (`Plot.figure(...)` / `PlotBuilder.figure({...})`, with matching setters on
  the raw wasm handle): figure size in inches, dpi, font and title size, font
  family, typography scale, line width, margin, tight-layout padding,
  scientific notation and a maximum resolution, chosen as one set the way a
  journal specifies them. The call is atomic — every argument is validated
  before any is applied — and the settings are recorded under snapshot schema
  version 2, so copies, notebook widgets and worker replay reproduce them.
  Validation rejects the values the core would silently clamp: sizes below
  1.0 inch, dpi below 72, font sizes below 4pt and line widths below 0.1pt.
- `Theme` gains four knobs that let a theme express a panel-based look:
  `panel_background: Option<Color>` fills the plot area before the grid,
  `frame: bool` removes every spine, `tick_marks: bool` removes the tick marks
  while leaving the tick labels alone, and `patch_edge_color: Option<Color>`
  sets the default edge for bars, histogram bins and boxes instead of darkening
  the fill. All four are honored by both 2D backends (raster and SVG) and are
  available on `Theme::builder()`. Every existing theme keeps the values that
  reproduce today's output exactly.
- Named themes reach the bindings. `plot.theme(...)` in Python and
  `plot.theme(...)`/`setTheme(...)` in the web SDK now accept `"seaborn"`,
  `"publication"`, `"minimal"` and `"presentation"` alongside `"light"` and
  `"dark"`; the accepted names are exported as `ruviz.Theme` (Python) and
  `PLOT_THEME_NAMES` (TypeScript). `Plot3D` still takes `"light"`/`"dark"`
  only, now typed as `Theme3D`.
- The Python binding's `histogram()` gains a keyword-only `density=` flag
  (mirrored in the web SDK's histogram style), normalizing bins to probability
  density so a `kde()` overlay shares the axis instead of rendering flat at
  zero. The 3D tick-label layout also separates the x and y corner labels that
  previously collided into one string.

### Changed

- The default data line width is now the documented matplotlib 1.5pt. The
  renderer used to hard-code 2.0pt for a line series with no explicit width,
  ignoring both `line_width_pt()` and the width the active theme asked for
  (publication's 0.75pt included); the fallback now reads the configured
  width, so those finally apply.
- `max_resolution()` is a true cap when `dpi()` was set explicitly: a DPI
  whose output already fits the bounds is kept, an overflowing one is reduced
  to fit. Without an explicit DPI it scales to fit the bounds as before.
  Bounds small enough to imply a DPI below the render minimum are honoured
  like an exact pixel size instead of failing every subsequent render.
- `scientific_notation(bool)` now forces tick-label notation on both axes —
  `true` always scientific, `false` always plain decimals — where unset keeps
  the automatic per-axis choice. A symlog axis keeps its linear region's
  labels plain either way.
- `Theme::seaborn()` is now a faithful port of `seaborn.set_theme()` rather than
  a loose approximation: an `#EAEAF2` data panel on a white figure, white grid
  lines drawn on that panel, no spines and no tick marks, soft near-black text
  (`#262626`), seaborn's font sizes, and the `deep` palette (`#4C72B0 #DD8452
  #55A868 #C44E52 #8172B3 #937860 #DA8BC3 #8C8C8C #CCB974 #64B5CD`) in place of
  the tab10 colors it used to carry. Bars and histogram bins take seaborn's
  white hairline instead of a darkened edge. The default theme is unchanged and
  its output is byte-identical.
- The web binding gains `JsPlot.theme(name)`, which takes any of the named
  themes, matching how `legend(position)` already works. The existing
  `theme_light()`/`theme_dark()` keep working and are now shorthands for
  `theme("light")`/`theme("dark")`.
- The 2D draw order now reads grid, data, frame. The grid already sat under the
  series in both the raster and the SVG path, but the axis frame and its tick
  marks were drawn *before* the series, so a bar or histogram resting on zero
  painted over the bottom spine and tinted it. Both paths now emit the frame
  and ticks after the series, matching matplotlib, where spines outrank
  patches. Annotations keep their existing underlay/overlay split.
- A pie's wedges now run **clockwise** from 12 o'clock in input order (plotly's
  and d3's convention), so reading the chart the way a clock is read follows
  the order the values were given. They used to run counter-clockwise, which
  reversed the apparent order. Donut mode follows, and both the raster and SVG
  paths agree. `PieConfig::counter_clockwise()` (and the matching pie-builder
  method) restores matplotlib's direction.
- Pie wedge labels now take white or near-black per wedge, by the relative
  luminance of the fill they sit on, instead of always black — black on tab10
  blue, green or red was close to unreadable. `PieConfig::text_color` became
  `Option<Color>`: left unset the label color is derived, and
  `PieConfig::text_color(color)` forces one color for every wedge. The name
  line and the percentage line get the same treatment.
- A donut's wedge labels now land on the ring. `label_distance` runs from the
  inner edge (0) to the rim (1), where it used to scale the ring's midpoint and
  so pushed the labels back over the hole, where the background ate half of
  each one. A full pie is the `inner_radius = 0` case of the same expression
  and is unchanged.
- A log or symlog axis now labels its decades only, and picks one notation for
  the whole axis: plain decimals while every ticked magnitude stays inside
  `0.0001..=100000`, the `10ⁿ` / `2×10ⁿ` exponent form otherwise. A log axis
  used to interleave `10⁰`-style decade labels with plain intermediates
  (`0.5`, `2`, `5`) on the same axis. The 2/5 intermediates remain as unlabelled
  minor ticks, and are kept as major ticks only on a range too short to produce
  three decades. A symlog axis keeps its linear region's labels linear.
- Pie percentage labels drop a trailing `.0`: a fifth of the pie reads `20%`
  rather than `20.0%`, while `20.5%` is unchanged.
- The default radar fill alpha drops from 0.25 to 0.15, so two filled series no
  longer composite into a muddy tan where they overlap. Line strokes and
  markers are unchanged; `fill_alpha`/`series_fill_alpha` still override it.
- A violin body is now half of its category slot wide, the same fraction a box
  plot uses, instead of 0.8 — a lone violin filled nearly three quarters of the
  plot frame. The interior box, quartile lines and median marker are sized from
  the body width and follow it.
- 3D tick labels: the x and y axes meet at the same projected corner, so their
  corner-most tick labels used to come to rest a few pixels apart and read as
  one number ("2 2"). The y label is now stepped clear of the x label, and
  dropped if it cannot be.

### Fixed

- `margin()` and `scientific_notation()` were recorded but never read by any
  render path; both now reach the layout and the tick formatter. `margin()`
  writes the proportional margins the layout actually consumes, on all four
  sides.
- A font family with surrounding whitespace (`"Arial "`) now matches the
  registered family instead of silently falling back to the default face, and
  the wasm binding rejects empty family names like the other bindings.
- A web build without the `embedded-font` feature could never follow its own
  error message: module initialization demanded a registered font before
  `registerFont()` could supply one. Initialization now succeeds font-free
  and the render entry points keep reporting the actionable error.
- Snapshot replay tolerates malformed foreign metadata — a null, sparse or
  out-of-range `maxResolution` is skipped instead of aborting the plot
  rebuild.

## [0.7.0] - 2026-08-05

### Breaking

- The Python binding rejects input it previously mis-handled silently:
  non-1D arrays into 1D series (including `contour` with a 2D `z`), unknown
  or missing `save()` extensions (previously wrote PNG bytes to any path),
  non-positive or fractional `size_px`/`dpi`/`bins`/`levels`, DataFrames
  passed as vectors, `data=` with a Series, bare string columns without
  `data=`, and observables passed to plot kinds that cannot track them.
- `anywidget`/`traitlets` moved to the optional `ruviz[widget]` extra;
  `import ruviz` no longer requires them and widget entry points raise an
  `ImportError` naming the extra. New `ruviz[all]` extra.
- The private `ruviz._native` module no longer exports the legacy JSON
  snapshot functions (`render_png_bytes`, `render_svg`, `save`,
  `show_native`); rendering goes exclusively through the native handles.
- `ObservableSeries.replace()` validates resizes across the whole derivation
  graph before mutating and raises `ValueError` on any bound-series
  violation; previously invalid resizes partially committed.

### Added

- Per-series styling in the Python binding wherever the renderer honors it:
  `label`, `color` (hex and named, with did-you-mean suggestions), `alpha`,
  `width`, `linestyle`, `marker`, `marker_size`, plus `histogram(bins=)`,
  `kde(bandwidth=)`, and `contour(levels=)`; matplotlib shorthand aliases
  (`"o"`, `"--"`, ...) are accepted and normalized.
- Plot-level `legend(position)`, `grid()`, `dpi()`, `xlim`/`ylim`
  (descending bounds render an inverted axis), and `xscale`/`yscale` with
  linear, log, and symlog scales.
- Full typing support: `py.typed`, `_native.pyi` stubs, runtime-valid
  public aliases and snapshot TypedDicts, `ruviz.__version__`, and a
  pyright-checked consumer contract in CI.
- The notebook widget and web runtime render the styled snapshot schema
  (`schemaVersion: 1`): series styles, `dpi`, legend, grid, limits, and
  scales, with unknown fields tolerated and preserved for forward
  compatibility. The TS `PlotBuilder` gains the matching styling and axis
  methods with eager validation.
- `data=` accepts any `Mapping` or object indexable by column name;
  `ObservableSeries` supports `len()`, indexing, and the NumPy 2
  `__array__` copy contract.
- Linux aarch64 wheels (manylinux 2_28); wheels for every platform are
  smoke tested before upload; CI tests Python 3.10 and 3.13.

### Changed

- NumPy arrays cross the Python boundary with a single `memcpy` instead of
  a Python-list round trip (1M-point series add: 141 ms to 0.9 ms), plot
  building and rendering release the GIL, snapshots materialize lazily
  (cached `to_snapshot()` at 1M points: 477 ms to 4 ms), first-plot latency
  drops from ~185 ms to ~0.2 ms, and widget refreshes coalesce under a
  running event loop.
- Native handles are `Send`: plots and observables built on one thread can
  be used from another (GIL-serialized) instead of aborting with a panic.
- The Python `numpy` floor relaxed to `>=1.26`.
- Python examples, docs, and the PyPI README were rewritten around the new
  API, with regenerated gallery assets.

### Fixed

- Vetoed observable resizes are atomic across derivation graphs (diamond
  and unequal-depth paths included) and their error messages are
  deterministic; derived updates propagate in dependency order.
- `Plot3D.size_px`/`dpi` no longer accept sub-1 floats that truncated to
  zero and failed at render; 3D methods reject observables instead of
  silently freezing them.
- `typing.get_type_hints` works on the whole public surface.
- The web runtime's snapshot clones no longer drop or alias unknown fields.

## [0.6.0] - 2026-07-28

### Breaking

- Removed the unreachable 2D series-parallel renderer. `Plot::with_parallel`, `Plot::parallel_threshold`, `ruviz::render::{ParallelRenderer, ParallelConfig, SeriesRenderData, PerformanceStats, DetailedPerformanceInfo}` and `RenderPipeline::{parallel_renderer, parallel_renderer_mut}` are gone. No public `render()`/`save()` could reach that path, and `.backend(BackendType::Parallel)` still resolves to Skia with an explicit `UnsupportedOperation` fallback reason (previously `FeatureDisabled` without the `parallel` feature).
- The `parallel` cargo feature remains a default feature. It is not inert: the software 3D rasterizer parallelizes its tiles with it. It has no effect on 2D output or 2D timings.
- Removed the inert pooled-rendering knobs `Plot::with_memory_pooling`, `Plot::with_pool_sizes`, `Plot::pool_stats` and `RenderPipeline::{set_pooled_rendering, pooled_rendering_enabled, set_pooled_renderer, pooled_renderer, pooled_renderer_mut}`. Nothing on the render path consulted them. `ruviz::render::PooledRenderer` itself is unchanged and still usable directly.
- Removed the `animation-gif` (an exact duplicate of `animation`) and `animation-hq-gif` cargo features, along with the unused `gifski` dependency. Neither gated any code; use `animation`.
- Optional dependencies no longer leak as implicit cargo features. `--features rayon`, `--features wgpu`, `--features gif` and similar bare crate names are now unknown features; `--features polars`, `--features ndarray` and `--features nalgebra` remain as explicit aliases.
- `--features window` now enables the `interactive` code as well as the desktop dependencies; previously it pulled winit + softbuffer + rfd + arboard and gated nothing.
- Removed `Legend::calculate_size`; use `measure_legend_size` or `layout_legend`.
- Replaced `ruviz::interactive::test_utils::MockEventHandler::assert_60fps_compliance(Duration)` with `assert_no_frames_dropped(frames_recorded: usize)`; `PerformanceMonitor` gains `frame_count()`.
- `SkiaRenderer::draw_legend_full` and `SvgRenderer::draw_legend_full` take `Option<&LegendOccupancy>` where they took `Option<&[(f32, f32, f32, f32)]>`.

### Added

- Added the separately packaged `ruviz-egui`, `ruviz-iced`, and `ruviz-slint`
  native GUI adapters, with static and interactive 2D/3D plots, responsive
  HiDPI rendering, framework-native events, reactive redraws, replacement
  with optional view preservation, and optional GPU rendering followed by
  readback.
- Added shared native-adapter contracts for plot/session conversion, logical
  and fitted-image geometry, opaque latest-request scheduling, explicit RGBA
  alpha conversion, stamped 2D frames, and retained background 3D rendering.
- Added a packaged Slint `@Ruviz` component library with a multi-slot runtime
  and retained Rust controller.
- Added a GPUI 3D builder matching the 2D adapter, including static and
  interactive modes, fill/fixed sizing, fitted input mapping, replacement,
  pick/camera/error callbacks, and background CPU or GPU-readback rendering.
- Added `Plot3D::legend(Legend)`, so 3D figures configure their legend with the same `Legend` type, position, font size, colours, spacing, style and layout routine as 2D. Without it the legend is derived from the theme, as before; a legend with `enabled` false suppresses both the legend and the band reserved for it.
- Added `SkiaRenderer::draw_image_layer`, which composes a straight-alpha RGBA image without the PNG encode/decode round-trip `draw_subplot` used to perform.
- Added `layout_legend`, `measure_legend_size`, `LegendLayout`, `LegendPlacement`, `LegendOccupancy`, `LegendEntryLayout`, `LegendTitleLayout` and `estimated_label_width` to `ruviz::core`.
- Added `release_3d_gpu_resources` to the prelude, and a public entry point that tears down the process-wide 3D renderer, its scene buffers and its offscreen attachments. It is a no-op without the `gpu` feature.
- Added `RenderDiagnostics3D::buffer_evictions`, reporting evictions from the now-bounded LRU GPU resource cache.

### Changed

- Native GUI renders now use one in-flight request plus one coalesced latest
  request, retain the last good image while work is pending, and reject input
  against stale frame/view stamps. Rendering no longer runs in adapter paint
  or view callbacks.
- Added `PlottingError::RenderSuperseded` to the already non-exhaustive error
  enum so obsolete background work can be handled without parsing strings.
- 3D render jobs retain shared resolved scene data and reusable CPU/GPU worker
  resources instead of copying a scene or rebuilding a renderer per frame.
- Rendered `Image` values, including `SkiaRenderer::into_image()`, now have
  canonical straight-alpha RGBA semantics, with explicit conversion helpers
  for consumers that require premultiplied alpha. Callers that previously
  treated `SkiaRenderer::into_image()` bytes as premultiplied must update.
- `LegendPosition::Best` now consults where the data actually is. Both the raster and the SVG paths build an occupancy grid from the projected samples and score candidate corners against it; previously `Best` always answered `UpperRight`.
- 3D surfaces are lit in linear space. `shade()` used to scale sRGB bytes directly, which systematically darkened and desaturated every lit surface; it now decodes to linear, applies the Lambert term and re-encodes, which is what the GPU already did for free from its sRGB target. `render()` and `render_gpu()` therefore agree instead of producing different figures on different machines. Every lit 3D image is brighter.
- Both 3D backends emit straight (non-premultiplied) alpha layers, matching PNG, the SVG data URI and every `SkiaRenderer` entry point. Antialiased 3D silhouettes are no longer darkened by the compositor, so markers, lines and surface edges lose the dark halo they used to carry.
- 3D surface normals are aspect-corrected, so shading is correct for every non-`Equal` axis aspect including the default.
- The 3D camera derives its scene radius from the real aspect-scaled bounding box instead of a constant tuned for the default aspect. Perspective figures with a non-default `axis_aspect` are framed correctly; previously the camera sat too close and the plotting box could nearly overflow its viewport. The default orthographic camera is bit-identical.
- `Camera3D::look_at` targets outside the resolved bounds are clamped to the plotting box, and `pan` stores the clamped target so a long drag cannot accumulate an unreachable one.
- CPU `MarkerStyle::Triangle` now points up, matching the GPU and the 2D marker paths; it used to be drawn upside down.
- CPU point markers whose centre leaves the side clip planes now draw their visible sliver instead of being dropped whole, matching the GPU's per-pixel clipping, so edge markers no longer pop during an orbit.
- The GPU 3D resource cache is a bounded LRU, and the GPU scene pipelines blend instead of overwriting, so translucent 3D colours composite.
- Inside legends are sized from the text the renderer actually draws rather than from a byte count times a guessed advance, so every PNG/SVG golden with an inside legend shifts slightly and non-ASCII labels are sized correctly.
- The CI test-coverage guard no longer credits a `cargo test` invocation to a job that cannot run on a pull request. A schedule- or dispatch-gated job now covers nothing, so moving the whole-suite run out of the pull-request lanes is a test failure instead of a silent hole.
- The 2D data-bounds routine moved from `src/core/plot/parallel_render.rs` to `src/core/plot/bounds.rs`. There is still exactly one implementation; no public API changed.

### Fixed

- Fixed the 3D legend painting its labels at `Theme::legend_font_size` while sizing its frame from `Legend::font_size`. The two disagreed for every user-set size: a small font produced a label wider than its own box and clipped by the canvas edge, a large one produced a box full of air. The overlay now draws at the size the shared layout measured, and honours `Legend::text_color`, `Legend::style` and `Legend::title`, which it previously ignored (the title was measured into the frame but never painted). Theme-derived 3D legends are byte-identical.
- Fixed `Legend::position` being ignored by 3D figures. An inside position — including `Best`, which resolves to one — now places the legend inside the plotting box; outside positions resolve to the right-hand decoration band, which no longer reserves width for a legend that is not in it.
- Fixed a 3D legend wider than its capped decoration band painting its label off the edge of the canvas. The frame now grows leftwards over the plotting box instead, so the label always fits inside the frame that was measured for it.
- Fixed `MockEventHandler::assert_60fps_compliance` asserting a frame count its own caller could never reach: the loop paces itself with `thread::sleep(16ms)`, which always overshoots a 60fps budget, so the assertion measured `thread::sleep` accuracy rather than this crate. Replaced by `assert_no_frames_dropped`, which checks the handler's counters against the independently recorded frame count; `PerformanceMonitor::record_frame` also no longer fabricates a 16.67ms first sample.
- Fixed `ManagedBuffer::into_inner` never recycling its buffer and never decrementing `MemoryStats::active_allocations`, which made the reported allocation count grow monotonically and never shrink.
- Fixed `SkiaRenderer::draw_datashader_image` writing B, G, R, A into tiny-skia's premultiplied **RGBA** pixel buffer, swapping red and blue in the DataShader tint. Invisible with every stock theme, whose foreground is black, white or grey.
- Fixed the 2D subplot compositor handing premultiplied pixels to a compositor that expects straight alpha. Output is unchanged for opaque subplots, which is every subplot today.
- Fixed 3D composition paying a full PNG encode and decode of the whole canvas for every frame — roughly 11 MB per 1920x1440 orbit frame.
- Fixed `clipped_bounds` saturating a far-off-screen primitive onto the viewport edge pixel, which reported a culled primitive as drawn and binned it into a tile it never touches. A point whose centre projects to a non-finite position is now culled rather than aborting the render.
- `ParallelRenderer::process_series_parallel` no longer called `rayon::build_global()`, so nothing in the crate attempts to resize the application's global thread pool (the whole type is now gone).

## [0.5.0] - 2026-07-17

### Breaking

- `HeatmapConfig` gained the public `origin` field: exhaustive struct literals of `HeatmapConfig` must add it; builder-style construction via `HeatmapConfig::new()` is unaffected.
- `PlottingError` gained the `InvalidAnnotation` and `UnknownAnnotationId` variants: exhaustive matches must handle them; matches with a `_` arm are unaffected.

### Added

- Added atomic `StreamingXY::replace` for one-call paired data replacement with capacity truncation, a single notification, and full-redraw acknowledgement; empty streams now render safely, including on log axes.
- Added the `BuilderWhen` trait providing a conditional `.when(condition, |b| ...)` combinator across plot, series, group, subplot, config, theme, and interactive window builders without conflicting with GPUI's `FluentBuilder`.
- Added configurable heatmap row origin via `HeatmapOrigin::{Upper, Lower}` on `HeatmapConfig`, with consistent rendering, cell bounds, and interactive hit testing, including reversed Y axes.
- Added movable interactive annotations: session-scoped `AnnotationId` with fallible add/query/update/remove over every `Annotation` variant, scale-aware validation, and overlay-only invalidation that reuses cached base geometry.
- Added `RuvizPlot::set_plot_keep_view` to the GPUI adapter for replacing a plot while preserving a customized pan/zoom view; plain `set_plot` keeps its documented destructive reset semantics.
- Added GPUI plot coordinate mapping and pointer events for embedding applications.
- Added subplot suptitle measurement with a title-size API and clarified spacing semantics.
- Added a deterministic scientific Unicode light-on-dark text regression test using the repository-owned font.
- Added deterministic Rust gallery freshness checks: a no-write `generate_gallery --check`, `make rust-gallery`/`check-rust-gallery` targets, byte-identity golden coverage, and a path-scoped CI job.

### Changed

- Outside legend positions (`OutsideRight`, `OutsideLeft`, `OutsideUpper`, `OutsideLower`) are now honored in layout and rendering across PNG, SVG, parallel, interactive, and subplot paths, reserving side bands that account for labels, DPI, margins, and colorbars.
- Improved subplot gallery content: renamed and clarified example figures, tightened scientific showcase gutters, an English reference panel in the international figure, and repository-relative source links.

### Fixed

- Fixed streaming acknowledgement watermarks being shared across all consumers of a stream: incremental append-only rendering is now gated per consumer, so a second session or a prepared export no longer paints new points onto a stale pre-replacement base.
- Fixed `PlotLayout` and `MeasuredDimensions` losing external struct-literal constructibility: legend placement state moved to crate-internal composition types, restoring the exact v0.4.20 public field sets, with an external-crate regression test.

## [0.4.20] - 2026-07-15

### Added

- Added frame-coherent, lazily constructed point hit-test indexing for large interactive plots, including scaled, reversed, reactive, and streaming frames.
- Added exact bundled-font golden-image CI coverage for all committed deterministic visual fixtures and stricter checked documentation-fence validation.
- Added packaged-crate verification for clean external `ruviz` and `ruviz-gpui` consumers, including release artifact and VCS provenance checks.

### Changed

- Unified resolved plot data, series styling, typography, markers, legends, annotations, and error bars across raster, SVG, parallel, prepared, and interactive render paths.
- Made backend selection and diagnostics report the renderer that actually executed, with truthful fallbacks for unsupported Parallel, GPU, and DataShader operations.
- Made coordinate transforms, hit testing, overlays, subplots, and interactive rendering scale-aware while preserving exact fixed-size output contracts.
- Shared runtime font registration across renderers and completed the public multiline `TextStyle` contract for plain and Typst text.

### Fixed

- Fixed animation completion/reentrancy races, transactional reactive notifications, memory-manager lock ordering, and same-session reentrant interactive render deadlocks.
- Fixed text alpha compositing, font-family precedence, SVG marker/legend parity, asymmetric error bars, subplot DPI handling, margin validation, and stale per-frame resolution.
- Fixed feature aliases and release gating so ndarray compatibility, packaged GPUI consumers, and tag publication are checked against the exact required CI runs.
- Corrected stale backend, performance, sizing, font, API, and deprecation documentation and made documentation/visual CI failures retain actionable artifacts.

## [0.4.19] - 2026-06-03

### Fixed

- Fixed GPUI interactive output-dimension rendering so typography, ticks, borders, and series style metrics scale with requested render pixels while preserving the configured figure size model.
- Kept `ruviz-gpui` `FixedPixels` sizing exact and applied aspect fitting only to `Fill`, preventing mismatched backing surfaces in non-GPUI interactive render paths.
- Tightened prepared-frame DPI fitting so advertised fitted dimensions round-trip to the actual render canvas, including difficult aspect-ratio cases and low-resolution interactive panes.

## [0.4.18] - 2026-05-24

### Fixed

- Fixed heatmap and contour colorbar DPI scaling so tick labels, rotated labels, width, margin, and border stroke use the documented point/logical-pixel units consistently. Existing colorbar font sizes that were tuned as raw pixels may render larger because they are now honored as typographic points.

## [0.4.17] - 2026-05-24

### Added

- Added benchmark comparison tooling for plotting performance runs, including missing-row detection so incomplete candidate results fail the regression gate.
- Added non-breaking high-level plot APIs and rendering coverage for area, stem, boxen, step, and quiver workflows across PNG/SVG where supported.
- Added generated benchmark output examples for line, scatter, histogram, boxplot, multi-series, throughput, and memory scenarios.

### Changed

- Improved large line and scatter rendering performance while preserving the reference-quality public image output contract.
- Refined backend resolution so explicit backend selection and auto-optimization behave predictably, including safe fallbacks for non-linear axes.
- Updated plotting docs and performance guidance for non-degrading optimization behavior and backend choices.

### Fixed

- Fixed visual regressions in area, stem, boxen, quiver, annotation, and DataShader-related paths found during review.
- Fixed quiver validation, bounds, DPI scaling, axis-scale mapping, and diagnostic preservation across public render paths.
- Fixed benchmark comparison reporting so output targets are compared explicitly and omitted candidate rows are treated as failures.

## [0.4.16] - 2026-05-04

### Added

- Added `scripts/check_docs.py` to validate Markdown links, fenced code block metadata, and checked Rust, TypeScript, and Python documentation snippets across all tracked Markdown files.
- Added documentation validation to the pre-commit hook and GitHub Actions docs workflow so README and package docs examples stay aligned with the published APIs.

### Changed

- Refreshed the root README, Python README, npm README, crate READMEs, and guide examples to match the current Rust, Python, and browser APIs.
- Expanded documentation validation beyond the curated docs roots to all tracked Markdown files, including crate READMEs and top-level/test documentation.
- Made TypeScript documentation snippet checks independent of generated wasm bindings so docs validation can run before web artifacts are built.

### Fixed

- Fixed stale optional-result documentation examples that used `?` on API calls that no longer return `Result`.
- Fixed root README Rust examples so every copyable snippet using `?` is a complete `fn main() -> Result<()>` program.
- Fixed README and package documentation examples that had drifted from the current code and package entrypoints.
- Hardened documentation validation so checked Rust snippets using `?` cannot pass as partial snippets, and Markdown `fn main()` examples cannot use `?` without returning a fallible type.

## [0.4.13] - 2026-04-27

### Added

- Added scale-aware coordinate projection across the public PNG, SVG, and parallel render paths, including minor ticks and grid handling for logarithmic axes.
- Added regression coverage for log-scale legend rendering, Typst-valid symbol labels, parallel legends, marker sprite cache eviction, and cold/warm PNG byte equality.

### Changed

- Public PNG marker rendering now reuses a bounded process-wide marker sprite cache across renderer instances, improving repeated large-scatter rendering while preserving output parity.
- Registered the performance benchmark target and fixed the multi-series benchmark loop so `cargo bench --bench performance` runs the intended series set.

### Fixed

- Fixed public-render legend sizing so point-based legend text, frame, corner radius, and shadow offsets scale from the render DPI instead of fixed pixel assumptions.
- Fixed public PNG legend rendering through the parallel plot path, including issue 68/69 coverage for log axes and Typst-valid symbol labels without adding LaTeX support.
- Fixed box plot projection in the parallel renderer so quartiles and whiskers map through the y-axis scale.
- Fixed a notebook widget session sizing race that could initialize WebKit test canvases at `1x1` before the notebook wrapper was attached.

## [0.4.12] - 2026-04-12

### Fixed

- Fixed the canonical Python widget release build by committing the matching `Cargo.lock` workspace version bump, so the release workflow's `--locked` wasm widget build no longer fails after the version update.
- Fixed the npm package verifier so subprocess failures now surface the exact command, exit status, stdout, and stderr in CI logs.

### Changed

- Completed the full synchronized `0.4.12` release after the partial `0.4.11` npm-only publish, restoring aligned Rust, npm, and PyPI release semantics.

## [0.4.11] - 2026-04-12

### Fixed

- Fixed the published npm package so the tarball now includes the WebAssembly runtime under `generated/raw`, which restores `bun install ruviz` / `npm install ruviz` for browser and wasm consumers.
- Fixed npm release validation so CI and the GitHub release workflow verify the real `npm pack` tarball contents and smoke-install that tarball before publish.

### Changed

- Added an interactive Rust/wasm benchmark suite and committed smoke baselines for hover, pan, time, `setPlot`, and export flows.
- Browser-side repeated `renderPng()` / `renderSvg()` exports and identical `setPlot(plot)` attaches now reuse cached wasm-side state instead of rebuilding unchanged plots and session exports.

## [0.4.10] - 2026-04-10

### Changed

- Public static raster rendering now follows one reference renderer across `render()`, PNG export/save, and `render_to_renderer()`, so those entrypoints stay visually aligned instead of drifting by backend choice.
- Added exact-parity renderer diagnostics and reference-consistency regression coverage for the shared raster pipeline and its public render/save surfaces.
- The static raster backend now batches shared work and caches prepared raster geometry for repeated uncached renders, reducing repeated-render setup overhead in both Rust and Python.

### Fixed

- Fixed static raster hot paths so large scatter, line, and heatmap renders use parity-safe CPU accelerators while preserving the reference output contract.
- Fixed dense filled-marker scatter performance with scanline blitters for `Circle`, `Square`, `Triangle`, and `TriangleDown` markers.
- Fixed the branch’s optimized parity candidate path so it no longer routes through the parallel renderer when that backend would violate the reference-parity test contract.

## [0.4.9] - 2026-04-09

### Fixed

- Fixed Python large-scatter PNG renders that could black out the full plot area when automatic DataShader rendering activated on datasets around `100_000` points and above.
- Corrected DataShader image composition so empty bins stay transparent, plotted density aligns with screen-space `y` orientation, and large scatter subplots rendered through `render_to_renderer()` use the same safe composition path as the main render/save flows.
- Kept large histograms on the normal renderer instead of the scatter-oriented auto-DataShader path, preventing incorrect histogram rendering at large input sizes.

### Changed

- Added broad large-dataset regression coverage across Rust, Python, notebook widget, and interactive render surfaces for line, scatter, histogram, bar, boxplot, violin, KDE, ECDF, heatmap, contour, error-bars, and polar-line plots.

## [0.4.8] - 2026-04-09

### Fixed

- Notebook widgets now force transparent notebook host surfaces through the VS Code Jupyter wrapper chain, preventing host-injected white backgrounds from surrounding smaller plots in notebook outputs.
- Added browser regression coverage for notebook-like white wrapper shells so the widget stays transparent and content-sized across Chromium, Firefox, and WebKit.

## [0.4.7] - 2026-04-09

### Fixed

- Notebook widgets now shrink-wrap their notebook host container instead of reserving a full-width output box, so smaller plots no longer show a large white notebook area around the figure.
- Added browser regression coverage for roomy, constrained, and manually resized notebook widget hosts so the outer widget box keeps tracking the displayed plot width.

## [0.4.6] - 2026-04-08

### Changed

- Notebook widgets can now be resized directly in notebook outputs with a drag handle; free resize is the default and `Shift` or `Ctrl` preserves the current aspect ratio while resizing.
- The default notebook widget display size now matches the default exported PNG size when `size_px(...)` is not set, while still shrinking proportionally in narrower notebook cells.

### Fixed

- Improved notebook widget context-menu contrast so the right-click export menu remains legible on light notebook surfaces.
- Hardened notebook widget pointer handling for resize drags across mouse, touch, and pen input, including better constrained-resize behavior near minimum sizes.

## [0.4.5] - 2026-04-08

### Fixed

- The `Python Packaging` CI job now installs the `wasm32-unknown-unknown` target before rebuilding the notebook widget bundle, fixing the tag-triggered packaging failure that blocked the `v0.4.4` release workflow.

## [0.4.4] - 2026-04-08

### Changed

- Python release CI/CD now builds and publishes a manylinux `x86_64` wheel so Linux `uv add ruviz` installs can use a prebuilt wheel instead of falling back to a source build.
- The GitHub release workflow now publishes from `refs/tags/<tag>` even for manual recovery runs, and the release runbook now documents GitHub Actions as the single supported publishing path.

### Fixed

- Python source distributions now include the root `benches/` tree required by the workspace `Cargo.toml`, fixing source-install failures caused by missing bench targets during Cargo manifest parsing.
- Added Python packaging CI coverage that smoke-tests both the sdist install path and the Linux wheel artifact path before release.

## [0.4.3] - 2026-04-08

### Changed

- Notebook widgets now use `plot.size_px(width, height)` as their on-screen display size in Jupyter and VS Code notebook outputs, shrinking proportionally when the cell is narrower than the configured width.
- Notebook widget exports now move behind a right-click menu with `Save PNG` and `Save SVG`, removing the always-visible download buttons while keeping right-drag box zoom intact.
- Refreshed the release-facing version snippets, quickstart, and release notes for `0.4.3`.

### Fixed

- Fixed notebook widget sizing so the displayed widget aspect ratio and clamped size now match the exported PNG/SVG output instead of stretching to the notebook width.
- Added browser regression coverage for notebook-like host sizing, right-click export menu behavior, and preserved secondary-button zoom interactions in the blob-backed widget runtime.

## [0.4.2] - 2026-04-08

### Added

- Added live `ObservableSeries` arithmetic and NumPy ufunc derivations in the Python API, with copied observables preserving independent live state.
- Added Python package extras for dataframe workflows (`dataframes`, `pandas`, and `polars`) plus new docs/examples for dataframe input, deepcopy templates, and observable math.

### Changed

- The Python notebook widget now builds from the shared web SDK frontend, and `plot.size_px(width, height)` drives the widget aspect ratio in notebook displays.
- Refreshed the release-facing version snippets, quickstart, and release notes for `0.4.2`.

### Fixed

- Fixed Python copy/deepcopy behavior so copied plots keep independent native plot handles and live observables instead of aliasing shared state.
- Fixed Python CI and preview lanes to invoke the packaged tools consistently and to avoid running the full generated-preview pipeline when unaffected paths change.

## [0.4.1] - 2026-04-07

### Added

- Added physical heatmap extents through `HeatmapConfig::extent(...)`, opt-in cell borders, heatmap-specific log colorbar subtick control, and automatic SymLog `linthresh` derivation.
- Added a dedicated `examples/montecarlo_heatmap.rs` example for a synthetic absorbed-energy style log heatmap.

### Changed

- Heatmap log colorbars now use superscript decade labels again, reserve enough right-side layout space to avoid clipping, keep decade labels centered on their ticks, and draw visible logarithmic subticks by default.
- Log-scaled heatmaps now mask `<= 0` and non-finite cells out of auto range detection instead of coloring them as the minimum valid bin, matching Matplotlib-style `LogNorm` behavior more closely.

### Fixed

- Fixed the new pixel-aligned heatmap and filled-contour fast path so translucent fills keep normal alpha compositing and subpixel-width tiles still contribute visible coverage.
- Fixed interactive heatmap hit testing so masked log-scale cells do not produce hover or selection hits and stale masked hits are dropped during viewport refresh.

## [0.4.0] - 2026-04-05

### Added

- Added empty-plot rendering support across the Rust core renderers, the Python notebook path, and the Python widget/browser session path, so a plot with zero series now renders as a valid empty chart instead of failing.

### Changed

- Reworked tick-aware layout handling so shared layout respects explicit margin modes (`Fixed`, `Auto`, and `Proportional`) instead of collapsing them into the default content-driven layout path.
- Aligned render-time tick layout with configured major-tick settings so `render()`, save/export flows, and interactive rendering use the same tick-count strategy.
- Refreshed release-facing documentation, package READMEs, and generated media for the `0.4.0` release surface across Rust, Python, and npm/web.

### Fixed

- Restored the `center_plot` content-centering option in the layout path and added regression coverage for centered versus asymmetric layouts.
- Fixed the heatmap raster parity regression introduced by the `0.3.5` output-surface fast path by restoring the normal per-cell renderer for heatmaps and adding regression coverage for downsampled narrow-feature visibility.
- Fixed `draw_datashader_image` so image-backed raster paths apply their scale-and-translate transform correctly when blitting into the plot area.
- Fixed the Python widget build bootstrap so wasm-bindgen version lookup is deterministic and concurrent installs do not race on the cached CLI directory.
- Stabilized notebook/widget media regeneration and golden-image refresh flows so release assets can be rebuilt consistently from one documented command.

## [0.3.6] - 2026-04-04

### Added

- Added a Rust-only feature-impact benchmark suite for plotting render/save paths, covering `baseline_cpu`, `default`, `parallel_only`, `parallel_simd`, `performance_alias`, and `gpu_only`, with committed reference artifacts and documentation.
- Added heatmap value scaling through `HeatmapConfig::value_scale(AxisScale)`, including log-aware color normalization, colorbar ticks, and a dedicated `examples/heatmap_scale_reversed_axes.rs` example.

### Changed

- Descending manual axis limits such as `.xlim(4.0, 0.0)` and `.ylim(16.0, 0.0)` are now preserved across static rendering and interactive viewport updates instead of being normalized away.
- Updated the README, quickstart, and guide snippets to point at the `0.3.6` release and document reversed manual limits plus log-scaled heatmaps.

### Fixed

- Fixed `Plot::xlim()` and `Plot::ylim()` dropping inverted manual bounds when `min > max`, including interactive zoom, pan, and zoom-rect flows.
- Fixed heatmap colorbar placement so non-linear heatmap value scales render ticks and labels in the correct transformed positions.

## [0.3.5] - 2026-04-04

### Added

- Added a cross-runtime large-dataset benchmark suite covering Rust, Python, and wasm `ruviz`, plus Python `matplotlib` and Rust `plotters` comparison baselines.
- Added committed benchmark reference artifacts and documentation for the large-dataset plotting suite, including methodology, environment metadata, and report generation.

### Changed

- Accelerated raster PNG export for large datasets with automatic line envelope reduction, cached static histogram bins, and output-surface heatmap rasterization fast paths.
- Reworked the Python binding to keep native plot handles and prepared render state alive across renders, removing the old JSON snapshot round-trip from the hot render/export path.
- Extended the Python benchmark path and benchmark reporting so `render_only` measures real uncached rendering from a reused built plot, with smoke outputs separated from the published reference artifacts.

### Fixed

- Fixed Python static histograms to use Rust's cached static histogram path instead of the slower source-backed reactive path.
- Fixed nearest-neighbor heatmap fast-path sampling so the last source row and column are included in downsampled raster exports.
- Hardened the benchmark report generator against partial runtime result sets and corrected the benchmark statistics and probe-timing methodology used in the published report.

## [0.3.4] - 2026-04-03

### Added

- Added a notebook-safe `ruviz-web` runtime entrypoint plus a self-contained `anywidget` frontend bundle for the Python package, so Jupyter widgets no longer depend on worker loading or `import.meta.url`.
- Added browser regression coverage for blob-backed widget loading, widget export behavior, and single-point sine-signal snapshots in the notebook runtime path.

### Changed

- Bare `Plot` output and `plot.show()` now render a static PNG by default in Jupyter notebooks, while interactive notebook rendering is explicitly opt-in via `plot.widget()`.
- Python CI now verifies the notebook widget bundle is reproducible on the Linux runner, and the release workflow rebuilds one canonical Linux widget bundle before packaging Python artifacts.

### Fixed

- Fixed Jupyter/VS Code widget loading by bundling the notebook frontend as a single Bun-built module with inline WASM bytes instead of runtime-relative imports.
- Fixed notebook widget edge cases around single-point sine signals, export download URL lifetime, and duplicate image rendering from `plot.show()` in notebook cells.
- Fixed release determinism for widget builds by pinning the Rust and `wasm-pack` toolchains used to generate the Python notebook bundle.

## [0.3.3] - 2026-04-01

### Fixed

- Stopped publishing the plain `linux_x86_64` Python wheel to PyPI, since PyPI rejects that host-native Linux platform tag for public uploads.
- Kept the Python release path on PyPI by publishing the source distribution plus macOS Intel, macOS Apple Silicon, and Windows wheels while Linux falls back to source installs for now.

## [0.3.2] - 2026-04-01

### Fixed

- Updated the Python release packaging lane to use `maturin` `1.12.6`, fixing the duplicate-README source distribution failure seen in the `0.3.1` CI/CD release.
- Switched the macOS Intel Python wheel lane to the supported `macos-15-intel` GitHub-hosted runner so the unified release workflow can produce Intel macOS wheels again.

## [0.3.1] - 2026-04-01

### Added

- Added the first tag-driven PyPI publishing path for the `ruviz` Python package, including trusted publishing from GitHub Actions and multi-platform wheel builds.
- Added dedicated Python CI coverage so `uv sync`, the native extension tests, package tests, and Ruff linting run on every PR and release tag.
- Added published Python examples, docs, and notebook widget support alongside the browser runtime bridge bundled into the Python package.

### Changed

- Aligned the unified release workflow across crates.io, npm, and PyPI with shared version validation and release-note handling.

### Fixed

- Fixed mixed named and unnamed radar-series handling in the browser/web bridge so partial labels no longer fail render or mount.
- Fixed Python observable listener cleanup so discarded plots do not stay strongly referenced by long-lived observables.

## [0.3.0] - 2026-04-01

### Breaking Changes

- Raised the repository MSRV to Rust `1.92`. Builds on older toolchains now fail earlier instead of drifting behind CI and release validation.

### Added

- Added Linux and Windows desktop bootstrap support for `ruviz-gpui` examples and integrations, alongside the existing macOS path.
- Added explicit `ruviz-gpui` example compilation checks to CI and release validation so desktop integration regressions surface before publishing.

### Changed

- Switched the workspace GPUI patching flow to an upstream `zed-industries/zed` revision while keeping the required macOS right-drag fix pinned consistently across the workspace.
- Split oversized plotting, rendering, and observable modules into focused internal submodules while preserving the public module paths and re-exports.
- Defaulted contributor toolchains to the latest stable Rust via `rust-toolchain.toml`, while retaining a dedicated CI lane that enforces the `1.92` MSRV floor.

### Fixed

- Made GPUI examples fail cleanly in headless desktop environments, with session-specific hints for Linux (`DISPLAY` / `WAYLAND_DISPLAY`) and Windows desktop sessions.
- Refreshed the committed README quickstart image so the top-level documentation matches current rendering output again.

## [0.2.0] - 2026-03-31

### Breaking Changes

- Typst selection is now compile-time gated behind `typst-math`. `Plot::typst(true)`, builder forwarding to `.typst(true)`, and `TextEngineMode::Typst` are unavailable unless the feature is enabled. If your crate makes Typst optional, guard those calls with `#[cfg(feature = "typst-math")]` instead of expecting a runtime `FeatureNotEnabled` error. Without the feature, `.typst(true)` now fails with a compile error such as `no method named 'typst' found`.

### Added

- Added experimental browser and `wasm32` support via the `ruviz-web` crate and the public npm `ruviz` SDK.
- Added mixed-coordinate inset rendering so Cartesian plots can embed polar, pie, and radar series with configurable inset layout.
- Added builder chaining parity for common continuation flows and styled annotations, removing explicit `end_series()` workarounds in the supported cases.

### Changed

- Unified browser package naming on `ruviz` and made the JS workspace Bun-first for build, lint, and packaging flows.
- Aligned raster and SVG mixed-inset rendering behavior, including clipping, DPI-scaled strokes, and auto-placement spacing.
- Restored and committed golden-image visual fixtures so release validation has stable baseline artifacts again.

### Fixed

- Stabilized interactive zoom/pan, wheel direction, context menu, and save/copy shortcuts across the interactive and GPUI paths.
- Fixed ndarray view recursion and several export-path DPI, validation, and overwrite edge cases in PNG/SVG rendering.
- Fixed browser session timing and destroy races, and kept wasm/browser builds continuously checked in CI.

## [0.1.5] - 2026-03-23

### Breaking Changes

- Default tick marks now render on all four sides of the plot frame instead of only the bottom and left axes. To preserve the previous look, call `.ticks_bottom_left()`.

### Added

- Added `ruviz-gpui`, a GPUI component adapter crate for interactive and reactive plotting integrations.
- Added GPUI interactive session support and reactive plotting hooks for embedded and streaming use cases.

### Changed

- Preserved the public `SubscriberCallback` API while moving runtime subscription dispatch to internal shared callbacks.
- Raster DPI now changes output density without intentionally enlarging fonts, line widths, marker sizes, or layout spacing. If you tuned visuals around the old DPI-coupled output, re-check explicit `.line_width(...)`, `.marker_size(...)`, and font-size settings.

### Fixed

- Fixed GPUI reactive rendering issues around interactive invalidation, streaming redraws, overlay refresh, and source setter updates.
- Fixed manual axis-limit handling in the GPUI/reactive plotting path.
- Eagerly release `lift2` cross-source subscriptions when either source is dropped.
- Prevent `lift2` source-drop cleanup hooks from accumulating on long-lived source observables.
- Validate floating-point DPI values directly before rendering, including negative and fractional out-of-range inputs.
- Keep `set_output_pixels` geometry consistent with the actual configured DPI, even on invalid pre-validation states.
- Retry atomic temp-file creation on stale collisions and document why stale-temp cleanup is safe.
- Reuse the same per-series validation for saved snapshots so reactive saves keep NaN and error-bar checks aligned with render validation.
- Validate rendered reactive snapshots after capture so render, SVG export, and external renderer paths stop re-reading live series for validation.
- Preserve existing Windows export targets on overwrite failures by using native replace semantics and keeping the temporary file for recovery.
- Evict stale Typst cache entries when a replacement grows beyond the cache byte limit, including oversized render results that skip recaching.
- Restore snapshot-based bounds calculation for heatmap, density, polar, radar, contour, and other non-Cartesian series.
- Restore series validation before DataShader and parallel render fast paths, and preserve POSIX symlink destinations during atomic export overwrites.
- Make DataShader renders consume the same validated snapshot as the main render path, and keep invalid zero-DPI pixel sizing from surfacing misleading dimension errors.
- Tighten DataShader bounds handling for the reactive/interactive rendering path.
- Apply tick-side and tick-direction settings consistently in `render()`-based outputs, keep SVG frame strokes DPI-aware when ticks are disabled, and preserve exact framebuffer sizes on fractional HiDPI interactive surfaces.

## [0.1.4] - 2026-02-11

### Added

- Added grouped series API via `Plot::group(|g| ...)` for shared styling across line/scatter/bar series.
- Added grouped legend collapse with `group_label(...)` so grouped series render as a single legend entry.

### Changed

- Group auto-color behavior now reuses one palette-generated color for all group members when no fixed group color is set.
- Updated release documentation workflow to support versioned release notes under `docs/releases/`.

### Fixed

- Made dashed line spacing DPI-independent for consistent appearance across output resolutions.

## [0.1.3] - 2026-02-10

### Breaking Changes

- Removed `Plot::latex(...)` API. Use `Plot::typst(true)` for Typst text rendering.

### Added

- Added global Typst text mode via `Plot::typst(bool)` and builder forwarding.
- Added optional `typst-math` feature for Typst-backed text rendering across PNG/SVG/PDF export.
- Added strict Typst error behavior: invalid Typst now fails render/export with `TypstError`.
- Added ecosystem data ingestion support for `polars`, `ndarray`, and `nalgebra`.

### Changed

- Improved Typst text layout fidelity by aligning baseline/anchor semantics across layout, raster, and SVG paths.
- Fixed Typst title/label clipping and spacing drift in visual outputs (no public API changes).
- Removed Typst raster text oversampling and simplified native-scale raster handling.
- Stabilized the test suite and split CI into focused lanes for more reliable export and visual checks.

## [0.1.2] - 2026-01-30

### Platform Fixes

- Fixed macOS and Windows platform build errors (#4)
- Added FreeBSD support (#1)
- Added cross-platform CI build checks for Linux, macOS, Windows, and FreeBSD
- Pinned cross to v0.2.5 with `--locked` for reproducible CI builds

### Contributors

- [@yonas](https://github.com/yonas) - FreeBSD support (#1)
- [@Ameyanagi](https://github.com/Ameyanagi) - Cross-platform build fixes (#4)

[Unreleased]: https://github.com/Ameyanagi/ruviz/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Ameyanagi/ruviz/compare/v0.4.20...v0.5.0
[0.4.20]: https://github.com/Ameyanagi/ruviz/compare/v0.4.19...v0.4.20
[0.4.19]: https://github.com/Ameyanagi/ruviz/compare/v0.4.18...v0.4.19
[0.4.18]: https://github.com/Ameyanagi/ruviz/compare/v0.4.17...v0.4.18
[0.4.17]: https://github.com/Ameyanagi/ruviz/compare/v0.4.16...v0.4.17
[0.4.16]: https://github.com/Ameyanagi/ruviz/compare/v0.4.13...v0.4.16
[0.4.13]: https://github.com/Ameyanagi/ruviz/compare/v0.4.12...v0.4.13
[0.4.12]: https://github.com/Ameyanagi/ruviz/compare/v0.4.11...v0.4.12
[0.4.11]: https://github.com/Ameyanagi/ruviz/compare/v0.4.10...v0.4.11
[0.4.10]: https://github.com/Ameyanagi/ruviz/compare/v0.4.9...v0.4.10
[0.4.9]: https://github.com/Ameyanagi/ruviz/compare/v0.4.8...v0.4.9
[0.4.8]: https://github.com/Ameyanagi/ruviz/compare/v0.4.7...v0.4.8
[0.4.7]: https://github.com/Ameyanagi/ruviz/compare/v0.4.6...v0.4.7
[0.4.6]: https://github.com/Ameyanagi/ruviz/compare/v0.4.5...v0.4.6
[0.4.5]: https://github.com/Ameyanagi/ruviz/compare/v0.4.4...v0.4.5
[0.4.4]: https://github.com/Ameyanagi/ruviz/compare/v0.4.3...v0.4.4
[0.4.3]: https://github.com/Ameyanagi/ruviz/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/Ameyanagi/ruviz/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/Ameyanagi/ruviz/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/Ameyanagi/ruviz/compare/v0.3.6...v0.4.0
[0.3.6]: https://github.com/Ameyanagi/ruviz/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/Ameyanagi/ruviz/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/Ameyanagi/ruviz/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/Ameyanagi/ruviz/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/Ameyanagi/ruviz/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/Ameyanagi/ruviz/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Ameyanagi/ruviz/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Ameyanagi/ruviz/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/Ameyanagi/ruviz/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/Ameyanagi/ruviz/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/Ameyanagi/ruviz/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/Ameyanagi/ruviz/compare/v0.1.1...v0.1.2
