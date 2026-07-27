# ruviz audit remediation plan

This plan consolidates three multi-agent reviews run against `feat/3d-implementation`
on 2026-07-25:

1. **Engineering audit** — API design, architecture, 2D correctness, 3D/GPU, performance,
   build/test health. 49 findings survived adversarial verification.
2. **Taste & visual audit** — idiomatic Rust craft plus a review of the rendered output
   against matplotlib/seaborn/ggplot2. 29 findings survived.
3. **Full-catalog review** — every wired plot type rendered at one size, one DPI, default
   theme, then reviewed for visual/API/core problems. 45 images, 50 findings survived.

Every finding cited below was confirmed by a second agent that re-read the code and
re-opened the images. Findings that failed that check are excluded.

Working reports: `ruviz-audit-part1.md`, `ruviz-audit-part2.md`,
`ruviz-catalog-review.md` (session scratchpad).

## Guiding assessment

> A competent, genuinely well-engineered single-threaded 2D rasterizer wrapped in a public
> surface that promises several things it does not deliver.

> Seen side by side at one size with one theme, this reads as N plot types built one at a
> time on top of a good shared frame, not as one library.

The shared frame is now strong: `CoordinateTransform` genuinely consolidated the data↔screen
math, `src/axes/scale.rs` handles reversed/degenerate/sub-epsilon ranges with property tests,
the software 3D clipper is a textbook homogeneous Sutherland–Hodgman, and
`scripts/verify_packaged_crates.py` is a stronger publish gate than most Rust projects ship.

The problems are concentrated in three places, and they are structural rather than cosmetic:

- **Shared primitives that quietly rewrite their inputs**, so every plot type built on them
  inherits the same defect.
- **Per-plot-type duplication of geometry, bounds, ticks, and legend layout**, so the raster
  and SVG backends have already diverged in user-visible ways.
- **Public surface that advertises capability it does not have** — inert setters, unwired
  plot types, and a default feature that is a hardcoded `false`.

Ordering below follows leverage, not severity: a core fix that resolves eight symptoms
outranks a more severe fix that resolves one.

## Standing priority: a simple, clean, consistent API (2026-07-26)

ruviz is a library, so its public surface *is* the product. Public-API ergonomics outrank
internal correctness cleanups unless a correctness bug is user-visible. **Consistency is a
first-class requirement**: sibling plot types must take the same shape of arguments, return
the same builder type, spell the same knob the same way, and support the same chain. Prefer a
mechanism that makes divergence impossible — one generic builder, one shared macro — over
fixing each plot type by hand.

Judge every change by what a *downstream* developer sees, not what the crate's own tests see.
Build a scratch crate with `ruviz = { path = ... }` and compile against it: that is how the
prelude `Result` shadow (E0107 on every `Result<T, E>` after `use ruviz::prelude::*`) stayed
invisible despite a green test suite.

**This promotes the builder unification out of Phase 9 to run immediately after Phase 2**, and
its "Large" tag was wrong. Measured 2026-07-26:

- **15 of 19** public series methods already return `PlotBuilder<C>`. Only **four** diverge —
  `histogram` (`series_api.rs:653`), `boxplot` (`:765`), `heatmap` (`:873`) and `error_bars`
  (`:974`) return `PlotSeriesBuilder`.
- **All four config types already exist**: `HistogramConfig` (`src/plots/histogram.rs:10`),
  `BoxPlotConfig` (`src/plots/boxplot.rs:15`), `HeatmapConfig` (`src/plots/heatmap.rs:75`),
  `ErrorBarConfig` (`src/plots/error/errorbar.rs:20`). Nothing new has to be designed.
- `PlotSeriesBuilder` is a two-field struct (`plot`, `series`, `series_builders.rs:298`) whose
  `impl std::ops::Deref` (`:738`) fakes inheritance from `Plot` with no `DerefMut` — which is
  why `.histogram(&d, None).theme(..)` fails with E0507 rather than a missing-method error.
- The cost of the divergence is concrete: roughly **60 config methods** exist only on
  `PlotBuilder<C>`, so those four plot types silently lack them. `legend_best` is on
  `PlotBuilder<C>` only; `legend(Position)` on `PlotSeriesBuilder` only.

So the work is: switch those four to `PlotBuilder<C>` over their existing configs, then delete
`PlotSeriesBuilder` and its `Deref`. Medium, not Large.

Also in scope for "one obvious way to do it": four entry points currently draw a line plot —
`Plot::new().line()`, `ruviz::line()` (`src/lib.rs:979`), `simple::line_plot` and
`simple::line_plot_with_title` (`src/simple.rs:48,:63`).

## Status (2026-07-27)

Phases 1–8 have landed on `feat/3d-implementation`; Phases 9 and 10
are partly done. **The phase sections below describe each problem as it was
found, not as the code is now** — every one carries a status line naming the
commit that closed it. Read the status line first.

Phases 6 and 10 landed as an uncommitted tranche on top of `258f3e8`; the rest
name their commit.

| Phase | State | Commit |
| --- | --- | --- |
| 1 — Shared primitives | done | `61d9a36`, follow-ups `846977e`, `c5a77cd` |
| 2 — Public surface | done | `f9eac62`; the last of §2.3 (`flow`) in this tranche |
| 3.1 — Bar geometry | done | `846977e` (`bar_pixel_rect`) |
| 3.2 — Bounds | done | `e2cd5ca` |
| 3.3 — Ticks | done | `e2cd5ca` |
| 3.4 — Legend layout | done | `258f3e8` (`layout_legend`) |
| 3.5 — Categorical / axis scale | done | `e2cd5ca`; the distribution family joined the one category axis in this tranche (`CategoryAxis::harvest`, `category_slot_span`, `impl_category_axis!`) |
| 3.6 — `BarConfig` threading | done | `846977e` |
| 4 — Correctness hazards | done | `e2cd5ca` |
| 5 — Truth in advertising | done | `258f3e8` |
| 6 — Rendering presentation | done | 6.2 (`Option<f32>` colorbar fonts), 6.3 (`POLAR_LABEL_RADIUS`/`POLAR_BOUNDS_RADIUS`, real polar grid) and 6.4 (z-axis edge from the corner hull, orthographic fit) landed alongside Phase 10. §6.1 — the measured/rotated/thinned x tick label row and the quiver colour key — closed in this tranche |
| 7 — 3D correctness | done | `258f3e8` |
| 8 — Test and CI credibility | done | `258f3e8` |
| 9 — Structural | **partly done** | builder unification `d027f46`; duplicate `PlotArea` retired with the parallel renderer in `258f3e8`; `ARCHITECTURE.md` now matches the tree. `thiserror`, the `Vec<f64>` fast path and the owned-field `Styled<T>` are closed. The gpui workspace split and the `rfd` pin are open |
| 10 — Unwired plot types | done | `flow` deleted, regplot CI fixed and the catalog made self-checking in this tranche; the five renderer-only types (`rug`, `strip`, `swarm`, `hexbin`, `dendrogram`) are wired through one `SeriesType::Computed` variant and the `ComputedSeries` trait |

Documentation assets were regenerated twice as the renderer changed: `21bf2d1`
and `e8c9266`.

Open, in rough order of value: the grouped/stacked bar and stacked-area builders
(which need their own multi-series shape, not another `ComputedSeries`), the 3D
colorbar caption, `add_axes`, then the rest of Phase 9 (the gpui workspace
split, the `rfd` pin).

The `Plot` builder now exposes **26** plot types, plus 4 from `Plot3D` — 30 in
total. `src/plots/mod.rs::catalog_is_true` reads the builder's own source and
fails if that count, either doc table, or the "no builder yet" list drifts from
the API.

## Completed (2026-07-25)

Landed on `feat/3d-implementation`; 1300 lib tests, clippy, and `cargo fmt --check` clean.

| Item | Change |
| --- | --- |
| Unbounded tick loop | `generate_nice_ticks` rewritten as a bounded integer-index loop with `MAX_TICK_STEPS = 100` and a non-advance guard (`src/axes/ticks.rs:244,:289`). Previously hung forever on `min ≈ 1e16`. |
| Invisible default grid | `#CCCCCC @ 0.3 @ 0.5pt` → `#B0B0B0 @ 1.0 @ 0.8pt` (`src/core/grid_style.rs:70-75`), plus the seven downstream overrides that made the default inert (`src/core/config.rs:287`, `src/render/theme.rs:111,185,250,320,343,552`). |
| Sub-pixel grid floor | `MIN_GRID_LINE_WIDTH_PX = 1.0` applied at the live render paths; minor grid now emitted as its own layer, so `minor_alpha`/`minor_line_width` are no longer inert. |
| Contour holes | Open-ended `(-inf, levels[0])` and `(levels.last(), +inf)` bands (`src/plots/continuous/contour.rs:333,:347,:392`). White pixel count in the fill region: 14 630 → 0. |
| Autoscale margins | matplotlib-style 5%/side at one funnel point, padded in each axis' own transform space, with sticky zero for bar/histogram and explicit limits never padded (`src/core/plot/mixed_render.rs:13`, `src/core/config.rs:37`). |
| Radar label bounds | `RADAR_LABEL_RADIUS`/`RADAR_BOUNDS_RADIUS` drive raster, SVG, and `data_bounds` from one place (`src/plots/polar/radar.rs:42,:52`). |
| docs.rs + semver | `[package.metadata.docs.rs]` on the three publishable manifests; `#[non_exhaustive]` on `PlottingError` (`src/core/error.rs:12`). |

Carried forward from that work: 26 golden baselines regenerated; the 3D goldens are
deliberately left stale (see *Decisions required*).

## Phase 1 — Shared primitives (S, highest return per line changed)

> **Status: done in `61d9a36`**, with follow-ups `846977e` (edges the
> implicit-border removal exposed) and `c5a77cd` (polar closure, non-finite
> samples). Everything below is the state before those commits.

These are one-line or one-file changes with no API break. Almost all of the remaining visual
return in the library is concentrated here.

### 1.1 Split the rectangle primitive — **do this first**

`draw_rectangle_with_mask` (`src/render/skia/primitives.rs:393-438`) multiplies alpha by
0.85 and unconditionally strokes a 1.0-px border at `0.8×` the fill. Neither is a parameter.

Measured consequences: bar and histogram fill render `#408bbf` where line and marker render
`#1f77b4` — exactly `0.85·(31,119,180) + 0.15·white`; a user's `.alpha(0.5)` silently becomes
0.425; and the raw-pixel border is the only stroke in `skia.rs` that bypasses `render_scale`,
so bars are not DPI-invariant while lines are.

Split into `draw_rectangle(fill)` and `draw_rectangle_styled(fill, edge)`.

**Fixed for free:** bar/histogram colour mismatch; `.color(black)` yielding grey; PNG≠SVG
colour divergence (SVG emits plain `fill="rgb(31,119,180)"`, `mixed_render.rs:612`);
DPI-varying bar outlines. **Unblocks:** boxplot fill and `BarConfig::edge_color` (Phase 3).

### 1.2 Correctness fixes in single files

| Fix | Location | Why |
| --- | --- | --- |
| Reverse the boxen level index | `src/plots/distribution/boxen.rs:206,:224,:350` | `letter_values_sorted` puts the median at index 0 and widens outward, but index 0 is drawn at full width and painted innermost-last — so the most extreme band occludes everything and ~half the sample is flagged as an outlier. Iterate `.rev()` in `compute_boxen`, which also makes the outlier test correct as written. **Taper direction, corrected during implementation:** seaborn's `_LVPlotter` draws the *median* band widest and tapers to slivers at the tails, so with the reversed index the factor is `(level + 1) / num_levels` (level 0 = most extreme = narrowest). An earlier draft of this plan said `0.25 + 0.75·(n−level)/n`, which is backwards; visual verification caught it. Pinned by a test asserting `boxes[0].width == config.width / 5.0` at 5 levels. Was the worst image in the catalog. |
| Series colour inheritance | `src/core/plot/series_api.rs:463,:502-508` | `area()` fills with `FillStyle::default()` (`Color::BLUE @ 0.3`) and `stem()` emits uncoloured arrows — a purple fill under a teal curve. Resolve `theme.get_color(series_index)` first; push stems as an underlay so markers stay on top. |
| Polar fill seam | `src/plots/polar/polar_plot.rs:293` | The origin vertex is appended unconditionally, producing a degenerate out-and-back spike — the white seam cutting the fill at 0°. Append only when the sweep is not a full turn. |
| Radar per-series style | `src/plots/polar/radar.rs:651`, `src/core/plot/mixed_render.rs:1607` | `per_series_fill_alphas`/`per_series_line_widths` have zero readers. Two lines each. |
| Contour line colour | `src/plots/continuous/contour.rs:660` | Hardcoded rather than `theme.foreground`. |
| Sticky zero for KDE/ECDF | `src/core/plot/mixed_render.rs:1742` | Density baselines should behave like bar baselines. |
| Pie start angle and label index | `src/plots/composition/pie.rs:186,:196,:504` | Labels bind to the wrong slice index. |
| 3D colorbar ticks | `src/core/plot3d/layout.rs:604` | Prints `0.982343 / 0.382643 / −0.217057` via raw `format!`. `generate_ticks` is already imported at `layout.rs:4`. |

## Phase 2 — Public surface, before 1.0 freezes it (S)

> **Status: done in `f9eac62`.** The one remainder, §2.3's empty `flow`
> module, was deleted in the Phase 10 tranche; `composite` and `regression`
> are `#[doc(hidden)]` rather than feature-gated (rationale in
> `src/plots/mod.rs`), and `hierarchical` is now wired.

Cheap now, breaking later. None of this requires new machinery.

### 2.1 Prelude and error type

- `HistogramConfig` and `BoxPlotConfig` appear in public signatures
  (`src/core/plot/series_api.rs:616-620,:726-730`) but are the **only** family configs missing
  from `ruviz::prelude` (`src/lib.rs:846-852`) — so a user's first line fails with E0599 on a
  type the signature demands. Add them plus `BinMethod`, `OutlierMethod`, `WhiskerMethod`,
  `KdeConfig`, `EcdfConfig`, `EcdfStat`, `BandwidthMethod`.
- Remove `Result` from the prelude — it shadows `std::result::Result`, so downstream
  `use ruviz::prelude::*` gives E0107 on every `Result<T, E>`, a pattern `README.md:35`
  actively teaches. Export `PlottingError` itself, currently absent while its alias is present.
- Downgrade `SeriesStyle` (`src/core/plot/builder.rs:393-429`, 18 pub fields) and `PlotInput`
  to `pub(crate)` — prelude-exporting internal representations makes every refactor breaking.
- Drop `#![allow(unconditional_recursion)]` and `#![allow(unreachable_patterns)]`
  (`src/lib.rs:5,:12`) and fix what they surface. Near-zero-false-positive lints.

### 2.2 Naming, settled once

- `Color::from_rgb(r,g,b)` → `from_rgb` (`src/render/color.rs:48`); every peer library spells it that way.
- Four spellings of one knob: `HeatmapConfig::colormap(ColorMap)`, `colormap_name(&str)`,
  quiver's `cmap<S: Into<String>>`, `HexbinConfig::cmap`. Standardise on
  `cmap(impl Into<ColorMapSpec>)` accepting both forms.
- Two legend-position enums, both in the prelude, different vocabulary
  (`Position::TopRight` vs `LegendPosition::UpperRight`, `src/core/position.rs:5`,
  `src/core/legend.rs:42`), with nothing signalling which is canonical. Deprecate one.
- `.size()`/`.size_px()` in 2D vs `.figure_size()` in 3D (`src/core/plot3d/builder.rs:391`) — add aliases.
- `Plot::dimensions` (`src/core/plot/construction.rs:693`) is deprecated in favour of
  `size_pixels()`, **a method that does not exist**. It is `size_px`.
- Opposite colorbar defaults: heatmap `true`, contour `false`. Default `true` for anything
  with a colour scale.

### 2.3 Stop advertising what does not exist

`src/plots/mod.rs` opens with "30+ plot types" and tabulates Hexbin, Grouped Bar and Stacked
Bar, none of which have a builder method. `pub mod flow;` (`src/plots/mod.rs:43`) exports five
lines of doc comment and zero code to docs.rs. Correct the table; `#[doc(hidden)]` or
feature-gate `flow`/`hierarchical`/`composite`/`regression` behind `unstable-plots`.

### 2.4 Deprecate the inert setters

Roughly **35 public methods currently compile and do nothing**: all ten `BoxPlotConfig` style
fields (`src/plots/boxplot.rs:10-37` vs hardcoded `0.3`/`0.6`/`×1.5`/`4.0` at
`series_internal.rs:1050-1191`); `ViolinConfig::scale`/`show_points`;
`BandwidthMethod::Silverman` (silently becomes Scott, `violin.rs:266-269`);
`KdeConfig::shade`/`vertical_lines`; `HeatmapConfig::aspect`/`xticklabels`/`yticklabels`/
`annotation_format`; `Interpolation::Bilinear` (emits byte-identical Nearest output);
`PieConfig::clockwise`; five `DendrogramConfig` fields; `RegPlotConfig::fit_through_origin`.

Implement the cheap ones (`annotation_format`, `silvermans_rule`, radar per-series lookups,
`width_ratio`/`cap_width`/`fill_alpha`) and `#[deprecated(note = "not yet implemented")]` the
rest so the compiler warns at the call site.

> A silent no-op is worse than a missing feature: the user changes the call, re-runs, and
> sees an identical image.

## Phase 3 — Unify the duplicated machinery (M)

> **Status: done.** 3.1 and 3.6 in `846977e`, 3.2/3.3/3.5 in `e2cd5ca`,
> 3.4 in `258f3e8`.

This is where the "N plot types, not one library" problem actually lives. Each item below
collapses a per-backend or per-plot-type clone into one implementation.

### 3.1 Bar geometry — one function

Bar rectangles are computed **four times**: raster (`series_internal.rs:980-1005`, 0.8
fraction, baseline literal `0.0`), SVG (`mixed_render.rs:595-613`, `plot_width/n × 0.7` with
**no axis mapping**), parallel (`parallel_render.rs:469-528`), and bounds
(`render.rs:1863-1871`).

They have already diverged in a user-visible way: the SVG copy drifts ±28 px against its own
tick labels on an 800 px canvas (Q1 bar centre 95.03 vs label 122.76; Q6 704.97 vs 677.24,
sign-reversing) and uses a different bar/gap ratio. Since SVG feeds `save_pdf`, **your PNG and
PDF of the same figure differ**. `tests/backend_parity_test.rs:9-59` never actually compares
backends.

Extract one `bar_rects` helper; replace the parity test with one that diffs the two backends.

### 3.2 Bounds computation — one routine

Three ~230-line near-clones in one file (`parallel_render.rs:1380,:1651,:1863`) whose bodies
diff to four trivial hunks. Annotation expansion is inline at `:1632` but post-hoc at
`mixed_render.rs:1700-1704`, and **absent** at `mixed_render.rs:1688-1690` and at six direct
callers — which silently drop annotation-driven axis expansion and clip `HSpan`/`FillBetween`.

Collapse to one routine generic over a `SeriesBoundsSource` view, with a `sticky_edges()`
method on `PlotData` and error-bar extents folded in.

**Fixed for free:** clipped `with_yerr` whiskers; retires the Phase 1 sticky-edge patches into
the trait; prerequisite for categorical positions (3.5).

### 3.3 Tick generation — one generator

Three formatters in play: raster uses `format_tick_labels_for_scale`
(`src/render/skia.rs:1978`), SVG uses `TickLayout::labels` (`src/axes/tick_layout.rs:118-178`)
whose per-value scientific switch produces mixed plain/scientific notation on a single axis,
and layout measurement uses a third (`render.rs:1761-1762`).

Delete `TickLayout::format_labels`/`format_number`, hoist `format_tick_labels_for_scale` into
`axes`, and rewrite `generate_nice_ticks` with candidate scoring.

**Fixed for free:** the heatmap colorbar's 2-tick axis, round contour levels, and every
starved axis at once.

### 3.4 Legend layout — one implementation

Implemented four times, and the **reservation formula differs from the drawing formula**:
near-clone drawing at `src/render/skia.rs:2752-2884` and `src/export/svg.rs:1825-1975`
(sharing a magic constant); inside-legend sizing uses `label.len()` **byte** counts ×
`font_size · 0.6` while `measure_legend` (`render.rs:1344-1376`) is outside-only; SVG has a
Typst measurement branch skia lacks, so overflow differs per backend; Plot3D hardcodes its own
(`src/core/plot3d/layout.rs:505-556`) and ignores user legend config entirely.

Extract `layout_legend(items, legend, bounds, measure) -> LegendLayout` into
`src/core/legend.rs`; all four consume it.

Related, same pass: `LegendPosition::Best` is `#[default]` but all three call sites pass `None`
for data bboxes (`render.rs:571-577,:2991-2997`, `parallel_render.rs:1333-1339`), so
`find_best_position` always sees overlap 0.0 and returns `UpperRight`. Build a 6×6 occupancy
grid from the already-projected screen points.

Also `LegendPosition::from_position` flips the Y axis (`src/core/legend.rs:128-133` vs the
renderer's `1.0 − y` at `:811-818`), so `Position::custom(0.1, 0.05)` renders bottom-left.

### 3.5 Categorical positioning and axis-scale plumbing

- **`AxisScale` is silently ignored by five renderers.** Bars (`series_internal.rs:989-992`),
  histogram (`:1015-1026`), boxplot (`:1054-1198`), heatmap (`src/plots/traits.rs:111-113`)
  and attached error bars (`mixed_render.rs:1817-1902`) call the linear-only
  `map_data_to_pixels` while the axes and ticks for the same figure are drawn scale-aware.
  `.boxplot(&d).yscale(Log)` renders a log-labelled axis with a linearly-positioned box.
  Switch to `map_data_to_pixels_scaled`, or return `InvalidInput` where a renderer genuinely
  cannot support it.
- Add `x_position` + `category` to `BoxPlotConfig`/`BoxenConfig`, threaded through all three
  render paths. Gives grouped boxplot/violin/boxen and removes the meaningless 0–1 axis.
- Consolidate the boxplot renderer (`impl PlotRender for BoxPlotData`) in the same pass — it
  also fixes the parallel DPI bug and the unfilled box.

### 3.6 Thread `BarConfig` into the renderer

`SeriesType::Bar` (`src/core/plot/types.rs:823-826`) carries only categories and values, so
`bar_width`, `edge_color`, `orientation` and `bottom` reach no geometry.
`BarOrientation::Horizontal` is public and unit-tested (`src/plots/basic/bar.rs:204`) but
unreachable. Add `config: BarConfig` to the variant.

**Unblocks:** horizontal bars, and grouped/stacked bar on top of the existing
`compute_grouped_bars`/`compute_stacked_bars`.

## Phase 4 — Correctness hazards (M)

> **Status: done in `e2cd5ca`.**

Independent of the above; can proceed in parallel.

| Hazard | Location | Fix |
| --- | --- | --- |
| KDE/violin of near-constant data | `src/stats/kde.rs:146-158` | `scotts_rule` returns 0.0 for constant input → zero-width grid → `0.0/0.0` NaN density → blank 0..1 plot with no error. Clamp bandwidth to a positive floor as scipy/matplotlib do; reject non-positive `BandwidthMethod::Fixed`. |
| Log axes clamp invalid samples | `src/axes/scale.rs:293-296` | `normalized_position` returns 0.0 for `value <= 0.0`, so non-positive samples snap to the axis floor and read as real data. Add `is_valid_value`, return NaN from the projection, split polylines on the gaps. |
| Public `marching_squares` panics | `src/stats/contour.rs:53-60` | `let nx = z[0].len();` with no guard, then indexes `x[i+1]`/`y[j+1]` from `z`'s dimensions. Validate shapes and return `Result`. |
| `transform_y_coordinates_pooled` argument swap | `src/render/gpu/renderer.rs:263` | Destructures `(left,_top,right,bottom)` and passes `(y_data, min, max, bottom, left)` into a `(y_data, min, max, top, bottom)` signature — wrong y pixels at the live construction site (`src/interactive/renderer.rs:42`). |
| `null_policy` ignored | `series_api.rs:1182,:1213,:1397,:1431` | `kde`, `ecdf`, `violin`, `boxen` drop `None` silently. Under `NullPolicy::Error` the user explicitly asked to be told — and for quantiles and bandwidth estimation, dropping nulls biases the estimate rather than merely losing points. |
| Contour pads silently | `series_api.rs:1257-1285` | `.unwrap_or(0.0)` for missing entries produces a wrong plot instead of an error. Return `DataLengthMismatch` when `z.len() != x.len()·y.len()`. |
| `get_points_in_region` fabricates | `src/interactive/renderer.rs:299-322` | Returns invented point IDs to any caller. Wire to the real `PointHitIndex` or make it `pub(crate)`. |

## Phase 5 — Truth in advertising (M)

> **Status: done in `258f3e8`.** The unreachable 2D parallel renderer and
> the memory pool are gone, the implicit-feature namespace is a hard error,
> and `tests/feature_hygiene_test.rs` keeps every declared feature honest.
> `parallel` remains a default feature, but now for a real reason: the
> software 3D rasterizer renders tiles across a rayon pool.

### 5.1 The `parallel` feature does nothing

`RenderExecutionMode::allows_parallel()` is a literal `false`
(`src/core/plot/mod.rs:93-98`) with exactly one caller (`render.rs:590`). So
`src/core/plot/parallel_render.rs` (2 097 lines) and `src/render/parallel.rs` (959 lines) are
unreachable from every public `render()`/`save()`, while `parallel` remains a **default
feature** that `README.md:143` describes as "enables the internal parallel renderer". The
crate's own `docs/benchmarks/rust-feature-impact.md:129-136` measures it at 0.94×–1.05×.

Delete both and drop `parallel` from defaults: ~3 000 lines removed, the 4-way copy-paste of
per-series drawing collapses to 2 (raster + SVG), and the latent SIMD-vs-reference projection
mismatch (`src/render/simd.rs:270-304` vs `raster_batches.rs:291-297`) goes with it. Also
delete the `rayon::ThreadPool::build_global()` call at `src/render/parallel.rs:109-115` — a
library must never mutate the process-global pool as a side effect of rendering.

### 5.2 Memory pool never recycles

`into_inner()` takes the `Option` (`src/data/memory.rs:724-727`) so `Drop` never recycles and
never decrements `active_allocations`. Every consumer is on the dead parallel path. The pool is
strictly slower than `Vec::with_capacity` and `MemoryStats` reports a phantom leak growing
forever. `with_memory_pooling()` documents "reduces allocation overhead by 30-50%" while
nothing reads `enable_pooled_rendering`.

### 5.3 Feature hygiene

- 18 optional dependencies leak as implicit features (bare names instead of `dep:` at
  `Cargo.toml:157,161,168,169,170,175,179,181,182`). `cargo add ruviz --features rayon`
  compiles rayon while leaving all 32 `feature = "parallel"` sites disabled.
- `window` and `svg` gate **nothing** — zero `feature = "window"`/`feature = "svg"` cfg sites
  exist anywhere, yet `window` pulls winit + softbuffer + rfd(GTK3) + arboard. Same for
  `animation-gif`/`animation-hq-gif`, which pull an unused `gifski`.
- Add a CI grep asserting every declared feature has at least one cfg site.

### 5.4 Benchmarks measure the wrong thing

`.save(...png)` sits inside `b.iter` throughout `benches/baseline_benchmarks.rs:15-108`, so
every number includes PNG encode + disk I/O. `bench_auto_optimize_speed` (`:113-130`) times
`auto_optimize`, which just sets `backend = Some(Skia)` and returns. Nothing in `benches/`
isolates projection, line reduction, or marker compositing — the three optimizations that are
real — so regressions there are invisible. `docs/BENCHMARK_RESULTS.md` should point at
`docs/benchmarks/`, the only trustworthy source in the repo.

## Phase 6 — Rendering presentation (M)

> **Status: done.** 6.2, 6.3 and 6.4 landed alongside Phase 10 — both
> `colorbar_tick_font_size` and `colorbar_label_font_size` are `Option<f32>`
> defaulting to the theme on heatmap and contour alike;
> `POLAR_LABEL_RADIUS`/`POLAR_BOUNDS_RADIUS` mirror the radar pair and
> `show_rgrid`/`show_thetagrid` reach a real grid; the z axis is anchored from
> the projected corner hull and the orthographic camera fits the box to the
> frame. §6.1 and the quiver colour key closed in the following tranche.

### 6.1 Categorical tick-label collisions

> **Status: done.**

`system__longlabels.png` was an illegible solid run of ten region names, and the canvas clamp
at `src/render/skia.rs:2134-2145` additionally shifted end labels off their own bars. Neither
backend measured the categorical row at all, and the bottom margin was reserved from the
numeric x tick labels a bar chart never draws.

What landed is one mechanism rather than a fix per backend:

- one measurement — `SkiaRenderer::measure_x_tick_row` → `XTickRowMetrics`;
- one policy — `XTickRowMetrics::plan` → `XTickLabelPlan { rotated, stride, extent, bounds }`;
- one centre formula — `SkiaRenderer::categorical_label_centers`, which replaced the SVG
  path's hand-rolled copy;
- one horizontal-placement formula — `XTickRowBounds::label_left`;
- one row drawer — `draw_x_tick_label_row`, generic over the backend-neutral
  `ColorbarCanvas`, called by both the raster drawer and the SVG path.

The old canvas clamp was replaced rather than simply deleted. Deleting it outright let a
first or last label wider than the outer margin run off the figure and be cut by the canvas
edge — a 400×240 bar chart with 35-character category names lost the left end of its first
label. `XTickRowBounds` is the replacement: the row is kept inside the canvas less one
`X_TICK_LABEL_GAP_EM` gutter — the same clearance a label keeps from its neighbour it also
keeps from the figure edge — and `XTickRowBounds::label_left` is asked by `clearing_stride`
*and* by `draw_x_tick_label_row`, so a label is measured where it will actually land. That
last part is the difference from the clamp that was removed: sliding an end label inwards can
no longer create the overlap the stride was chosen to avoid.

The plan is resolved once in `Plot::resolve_x_tick_label_row`, shared by the raster and SVG
entry points, so the row that was measured is the row that is drawn. The row is measured
against the horizontal pass's plot area, then the layout is **re-computed** with
`plan.extent` as the reserved x-tick height before `plot_area_from_layout`. "Does a rotated
row fit?" is answered by computing a trial layout and asking it (`x_tick_row_fits`), so it is
correct for content-driven margins (capped by canvas fraction) and for fixed/proportional
margins that cannot grow at all; the latter fall back to every k-th label.

The knob is `xtick_rotation(XTickRotation)` on both `Plot` and `PlotBuilder<C>`, with
`XTickRotation` re-exported from `ruviz::render` and the prelude:
`Auto` (default) / `Horizontal` / `Vertical`.

Two deviations from the sketch above, both deliberate: the turn is a quarter turn rather than
−45°, because `draw_text_rotated` is 90°-only in the raster text engine and an SVG-only −45°
would put the two backends back out of step; and the SVG path needed no measurement code of
its own, because it already builds a `SkiaRenderer` to measure with.

### 6.2 One colorbar

Three looks: heatmap hardcodes 12pt/14pt (`src/plots/heatmap.rs:117-118`, whose own docs claim
10/11), contour uses 10/11 (`contour.rs:89-90`), 3D uses raw `format!`. Make both font-size
fields `Option<f32>` defaulting to `None` → theme values, so `Theme::ieee()`'s 8pt ticks stop
getting a 12pt colorbar. Add `label: Option<String>` to `Colorbar3D`
(`src/core/plot3d/types.rs:88-95`).

Give quiver a colour key: `QuiverConfig` had `color_by_magnitude` and `cmap` but no
`colorbar` field, and `colorbar_measurement_spec` matched only Heatmap and Contour — so
`.color_by_magnitude(true)` produced an undecodable colour channel. **Done:** `QuiverConfig`
carries the same four colorbar fields and the same `colorbar_font_sizes(&Theme)` resolver as
contour/heatmap/hexbin, `QuiverPlotData::colorbar(&Theme) -> Option<ColorbarRequest>` is fed
from the existing `magnitude_range`, and `series_colorbar_request` dispatches to it — which
gives the right-margin reservation, the raster draw and the SVG draw together. The four
builder-side setters are no longer written per plot type at all: one
`impl_colorbar_builder_methods!` generates `colorbar`, `colorbar_label`,
`colorbar_tick_font_size` and `colorbar_label_font_size` for all four colour-key builders,
which is what stopped heatmap and quiver shipping a colorbar their builders could not reach
while contour and hexbin exposed two of the four.

### 6.3 Polar grid

`polar_grid()` and `circle_vertices()` (`src/plots/polar/polar_plot.rs:357,:380`, re-exported
from `src/plots/polar/mod.rs:43`) are unit-tested and **called by nothing outside their own
tests**; `show_rgrid`/`show_thetagrid` are dead config. Mirror
`radar.rs:580-626`. Drop `label_margin = r_max · 1.5` (`polar_plot.rs:410-413`) to a
`POLAR_LABEL_RADIUS`/`POLAR_BOUNDS_RADIUS` pair — the cardioid currently fills ~63% of its
square where radar fills ~80%.

### 6.4 3D presentation

- Z ticks and the `z` label are drawn on the front vertical edge, inside the silhouette, on
  top of the surface. `layout.rs:191-193` anchors all three axes at `outer_anchor_corner`,
  correct for x/y only. Pick the z edge from the convex hull of the 8 projected corners
  (already computed at `layout.rs:130`).
- The scene occupies ~41%×78% of the frame vs 92%×97% for a 2D line plot:
  `plot3d/types.rs:504-511` fits a fixed orthographic half-extent of `1.8/zoom` when the
  default camera needs ~1.33, and `layout.rs:629-636` hardcodes 14/10/14/16% margins.
- Also stop round-tripping every composited 3D frame through PNG encode+decode
  (`overlay.rs:18-21` → `skia.rs:3132-3135`): a 1920×1440 orbit frame pays a full
  deflate+inflate of ~11 MB per frame.

## Phase 7 — 3D correctness (M, separable)

> **Status: done in `258f3e8`.**

| Issue | Location | Fix |
| --- | --- | --- |
| Alpha convention mismatch | `software/raster.rs:231-254`, `gpu/pipelines.rs:186-188`, `presenter.rs:521-525` | Both backends emit coverage-premultiplied colour that every compositor treats as straight alpha, haloing every silhouette. An opaque red edge at 50% coverage over white renders (191,127,127) instead of (255,127,127). Tests currently lock the bug in (`raster.rs:966,:1017`). Pick one convention; enforce end to end with an analytic 50%-blend test. |
| Lighting in different colour spaces | `software/shading.rs:23-33` vs `gpu/resources.rs:508-523` + `mesh.wgsl:63-66` | sRGB on CPU, linear on GPU — up to ~2× luminance difference. Since `render_auto` picks GPU when an adapter exists, the same code produces different figures on different machines. Shade in linear space on both sides from one shared module. |
| Normals ignore `axis_aspect` | `software/raster.rs:493-507`, `mesh.wgsl:37-39` | Positions are scaled by aspect, normals copied untransformed. With `fixed(1,1,0.2)` a visually flat plateau is shaded as if it had full relief. `normalize(normal / axis_aspect.xyz)` in both. Both backends are wrong identically, so no parity test would catch it. |
| Hardcoded near/far | `plot3d/types.rs:528-532` | `.look_at(1e6,0,0)` passes validation and puts the scene behind the eye, where `line.wgsl:40-41` divides by a negative `w` unguarded and smears quads across the frame. |
| Shared GPU renderer leaks | `gpu/renderer.rs:63,:265-271`, `resources.rs:74,:113-130` | A never-torn-down static with retained MSAA/depth/readback attachments and a strong `Arc` to the last scene behind a single-entry cache. A 2000×2000 grid holds 100+ MB after the caller believes it is done. |
| **No parity test** | — | Zero pixel-parity coverage between the two rasterizers. Known divergences: triangle markers are upside-down on CPU vs GPU vs the 2D reference; points are centre-culled on CPU but edge-clipped on GPU, so edge markers pop during orbit. Add `tests/three_d_parity_test.rs` on a software adapter (lavapipe/WARP). |

## Phase 8 — Test and CI credibility (M)

> **Status: done in `258f3e8`.**

- **CI compiles 45 test files and runs about 17.** No `cargo test` invocation in
  `.github/workflows/ci.yml` enables `pdf`, `gpu`, `interactive`, `polars_support`, or
  `nalgebra_support`. Nothing in CI ever produces a PDF byte, ingests a polars DataFrame, or
  exercises the interactive event pipeline. On this branch,
  `tests/three_d_correctness_test.rs`'s 19 geometric assertions — the only correctness gate on
  the headline feature — never execute.
- **~2 000 lines of test code provide zero protection.** `tests/integration/*.rs` (1 611 lines)
  is never compiled — no `[[test]]` entry, no `#[path]` include. `tests/dpi_api_test.rs` has 6
  tests and **0 assertions** (save, then `println!`), so a `.dpi()` that became a no-op passes.
  `tests/line_style_tests.rs`' 8 assertions are all `assert!(result.is_ok())`, so Dashed,
  Dotted and DashDot all rendering solid passes. `tests/property_tests.rs` has 8 of 9 tests
  `#[ignore]`d with no CI job running `--ignored` — despite
  `proptest-regressions/render/three_d/software/clip_correctness_tests.txt` proving proptest
  has found real failures locally.
- **CI never asserts which rustc it runs**, while a tracked `rust-toolchain.toml` pinning
  1.94.1 outranks the MSRV job's claimed 1.92 (dtolnay/rust-toolchain activates via
  `rustup default`, ranked *below* a toolchain file). If the override ever stops applying, the
  MSRV job compiles with 1.94.1 and goes green while the crate no longer builds on 1.92.
- **183 of 222 guide snippets are unchecked.** `scripts/check_docs.py:17-18` already compiles
  `rust,check` fences; `04_plot_types.md` is 0/31 and `05_styling.md` 2/35. Invert the default:
  bare ` ```rust ` becomes an error requiring explicit `check` or `ignore`.

## Phase 9 — Structural (L)

> **Status: partly done.** The builder unification landed in `d027f46` —
> all series methods return `PlotBuilder<C>`, `PlotSeriesBuilder` and its
> `Deref` are deleted, and `legend_position` is defined once on the generic
> impl instead of being macro-generated 13 times. The duplicate public
> `PlotArea` went with the parallel renderer in `258f3e8`, and
> `ARCHITECTURE.md` now describes files that exist. The `thiserror` derive and
> the `Vec<f64>` fast path and the owned-field `Styled<T>` closed in the
> following tranches. Still open: moving `crates/ruviz-gpui` to its own
> workspace, and relaxing the `rfd` pin.

Do these only after Phases 1–3; several become much smaller once the duplication is gone.

- **`Plot` is a 323-method god object** across 7 `impl` blocks and 13 276 lines, and
  `docs/ARCHITECTURE.md:208-251` describes a four-manager decomposition that never happened —
  `layout_manager.rs`, `series_manager.rs` and `render_pipeline.rs` contain no `impl Plot` at
  all. **Fix the doc first**; it actively misdirects contributors.
- **Builder unification.** `line` returns `PlotBuilder<LineConfig>` but
  `histogram`/`boxplot`/`heatmap`/`error_bars` return `PlotSeriesBuilder`, which fakes
  inheritance via `Deref` with no `DerefMut` (`series_builders.rs:738-744`) — so
  `.histogram(&d,None).theme(..)` fails with E0507 rather than a missing-method error.
  `docs/guide/09_data_integration.md:169-175` ships a chain that cannot compile. Give the four
  real config types, return `PlotBuilder<C>` everywhere, delete `PlotSeriesBuilder` and the
  `Deref`. Introduce `trait FinalizeSeries` to collapse the 12 `impl_terminal_methods!`
  instantiations and ~61 hand-written forwarders.
- **`Styled<T>`** to replace the 12 hand-expanded `Option<T>`/`Option<ReactiveValue<T>>` field
  pairs duplicated across `types.rs:275-397` and `builder.rs:432-508`, plus the six-way
  expansion repeated in six traversals. Adding a seventh reactive property was 11 mechanical
  edit sites, each silently omissible — omit one from `collect_source_versions` and you get a
  permanently stale plot with no error.

  A first tranche introduced the borrowing `Styled<'a, T>` write half and the type-erased
  `StyleSource` read half, which got the six traversals iterating instead of naming six fields
  each and folded `construction.rs`'s `has_dynamic_style_sources` and `apply_resolved_style`
  into single calls. That took the silently-omissible edit sites from ~11 to 0, but a seventh
  property still cost 7 compile-forced edits.

  The `PlotSeries { .. }` literals in `series_internal.rs` are gone: the nine `add_*_series`
  constructors for kde, ecdf, contour, pie, radar, violin, boxen, polar and quiver each
  restated the same twelve-field `SeriesStyle` → `PlotSeries` copy and the same palette
  block, and all nine now call the `series_from_style` + `push_builder_series` funnels the
  rest of the crate already used (−300 lines). That cut the edit sites for a seventh reactive
  property in that file from nine to zero. Two drifts died with them: those nine advanced the
  auto-colour counter even for an explicitly coloured series while every other plot type did
  not, and pie/radar/polar each defaulted their inset placement in their own constructor —
  the placement rule now lives in `series_from_style`, keyed on
  `Plot::is_non_cartesian_series_type`.

  **Done.** `Styled<T>` is now one *owned* field: `value`, `source` and the property's
  `normalize` rule, all private, so the "exactly one half is set" invariant has no bypass.
  The twelve field pairs on `PlotSeries` and `SeriesStyle` collapsed into one
  `props: SeriesStyleProps` on each, and both hold the *same* type — they can no longer
  disagree about what a property is or what setting it means.

  The property list is declared once, by `series_style_properties!` in `types.rs`. From that
  one list the macro generates the fields, `Default` (which is where each property's range
  rule is attached, so a setter cannot forget to clamp), `Clone`/`Debug`, the `sources()`
  destructure, and every traversal built on it — `collect_versions`, `has_reactive_sources`,
  `has_temporal_sources`, `subscribe`, `clear_sources` — plus the traversal tests' per-property
  fixtures. **Adding a reactive style property is now literally one edit**: measured by adding
  a seventh (`zorder: f32, normalize = |z| z.max(0.0)`) to a copy of the tree and running
  `cargo check --all-targets --all-features`, which passed with that single line as the whole
  diff, and `reactive_style_tests` then covered the new property automatically (8/8, the
  fixtures being generated from the same list). The 12 hand-written setters are gone with it:
  every setter is `props.<name>.set(value.into())`.

  `resolve_style` collapsed too — `Styled::resolve(time, cache)` owns the source-vs-value
  precedence and the per-frame de-duplication, so `construction.rs`'s six five-line blocks are
  six one-liners and its `resolve_reactive_style` helper is deleted. `apply_resolved_style`
  writes through `replace_resolved` (value in, source out) and still calls `clear_sources()`
  as the catch-all, so a property this loop does not name cannot survive as a live source.

  The three remaining `PlotSeries { .. }` literals (one production funnel, two test fixtures)
  name no style property at all. `series_internal.rs`'s three remaining hand-expanded
  constructors — line, scatter and bar — now call `series_from_style` and apply their config
  defaults through `Styled::or_value`, and the palette block those three each restated is one
  `push_grouped_series` funnel.
- **`thiserror` derive** on the 60-variant error enum with its 330-line hand-written `Display`
  and hand-written `Error::source`; `IoError(Arc<std::io::Error>)` to unblock `Clone` and
  delete the 150-line `PendingIngestionErrorKind` mirror. ~450 lines deleted. **Done.** All 55
  variants carry their own `#[error("…")]`, the six messages whose wording depends on an
  optional field share one `optional_clause` helper, and every variant's own message text is
  unchanged (verified by set-diff of the 61 old format strings against the new attributes,
  empty both ways — including the log-axis rejection wording, which reaches the user through
  `InvalidInput(String)` from a `const` in `src/axes/scale.rs`). Two
  breaking changes fell out and are intended: `PlottingError::IoError` holds
  `Arc<std::io::Error>`, so build it with `PlottingError::from(io_error)`; and the `source`
  *field* of `DataTypeUnsupported`/`NullValueNotAllowed`/`DataExtractionFailed` is renamed to
  `origin`, because `thiserror` claims any field literally named `source` as the
  `Error::source` and `String: Error` does not hold — `origin` is the honester name anyway.
  `PendingIngestionError` now holds the real `PlottingError`, so a deferred ingestion failure
  keeps its variant instead of being flattened into a string.

  **One composed message did change**, and it is the only one: the deleted mirror enum had no
  `RaggedData2D` arm, so a ragged 2D input was flattened into
  `DataExtractionFailed { origin: "ruviz::plot-ingestion" }` on the way in and then wrapped in
  that same prefix again by the multi-error wrapper on the way out.
  `Plot::new().heatmap(&vec![vec![1.0, 2.0], vec![3.0]])` used to report
  `Failed to extract numeric data from ruviz::plot-ingestion: Failed to extract numeric data
  from ruviz::plot-ingestion: NumericData2D: row 1 has 1 values, expected 2 (and 1 additional
  ingestion error)` and now reports it with the prefix once. Nothing asserted the stutter,
  which is how it survived; `test_pending_ingestion_error_wraps_the_first_error_exactly_once`
  now pins the single-prefix form so the wrapper cannot grow a second one back.
- **Unify the six rectangle types** — two of them publicly named `PlotArea`
  (`src/plots/traits.rs:41`, in the prelude, and `src/render/parallel.rs:638`) with
  incompatible field sets, plus `LayoutRect`, `Rectangle`, and 41 bare `(f32,f32,f32,f32)`
  tuples. Do this before 1.0 freezes two public `PlotArea`s.
- **`Vec<f64>` — the crate's primary input — is the one numeric type routed through
  `Box<dyn Iterator>`** (`src/data/traits.rs:364-366` omits f64, falling to the blanket impl at
  `:236-250`). A 1M-point `line(&x,&y)` does ~2M dynamically-dispatched `next()` calls where
  two memcpys would do, on top of ~5 full copies before a pixel is drawn.

  **Done — but not the way sketched here.** Adding `f64` to
  `impl_numeric_data_1d_for_primitive_collections!` produces eight simultaneous E0119s, not
  one, and the only way through is deleting the `impl<T: Data1D<f64> + ?Sized> NumericData1D`
  blanket — which silently un-plots `DataView<f64>` and every downstream `Data1D<f64>`
  implementor, and replaces one mechanism with ten hand-written impls that can drift from the
  f32/i64 set. Instead `Data1D` gained one defaulted `as_slice(&self) -> Option<&[T]>` bulk
  hook, overridden by the ten contiguous implementors (Vec, `&Vec`, `[T; N]`, `&[T; N]`,
  `&[T]`, `[T]`, `Array1`, `ArrayView1`, `DVector`, `SVector`, plus `DataView`). Contiguous
  f64 storage now ingests as one `to_vec()`; only genuinely non-contiguous sources still walk
  the boxed iterator. Purely additive, so nothing downstream breaks, and it de-boxes ndarray,
  nalgebra and downstream types too. The same hook now short-circuits
  `collect_finite_values` (histogram/boxplot/kde/ecdf/violin/boxen), the radar
  `series`/`add_series` ingestion, and the pooled coordinate transform.
  **The GAT conversion should be dropped rather than deferred:** the only production caller of
  `Data1D::iter` was the blanket impl the hook now bypasses, so a GAT would cost a breaking
  change to a prelude-exported trait and object safety to de-box a call site that no longer
  runs.
- **Move `crates/ruviz-gpui` into its own workspace.** The root `[patch.crates-io]` gpui
  override is workspace-scoped and `default-members` does not exempt it, so even
  `cargo check -p ruviz` needs the zed checkout. `Cargo.lock` carries 38 `git+` entries from 6
  repos, two full wgpu 29 builds, and 17 glam versions; ~22 CI jobs pay for it on a cold cache.
- **Relax `rfd = "=0.15.4"`**, exact-pinned in 7 places and reachable from the public `window`
  feature — any downstream needing `^0.15.5` gets an unresolvable conflict. Add
  `[workspace.package]`/`[workspace.dependencies]` while there.

## Phase 10 — Unwired plot types (L, partly deletion)

> **Status: done.** The reachable catalog is **26** from `Plot` plus 4 from
> `Plot3D` — 30 in total. Rug, strip, swarm, hexbin and dendrogram gained
> working renderers *and* `Plot::` builder methods in this tranche; the empty
> `flow` module was deleted; the regplot confidence band was verified
> statistically wrong and fixed.
>
> The five were wired as **one** `SeriesType::Computed { data: Arc<dyn
> ComputedSeries> }` variant, not five. `SeriesType` is matched exhaustively in
> eleven places, so five variants would have meant ~40 new match arms; one
> variant meant 8, written once. `ComputedSeries::primitives` describes the
> geometry in device pixels and both backends have a single loop over it
> (`draw_primitives` / `draw_primitives_svg`), so a plot type wired this way
> **cannot** render in PNG and not in SVG — there is no per-type SVG code to
> forget. Adding the next compute-only plot type costs a `Plot::` method, a
> `finalize()` and an `impl ComputedSeries`.
>
> The counts and both doc tables in `src/plots/mod.rs` are checked by
> `catalog_is_true` against the builder's own source, and the types that still
> have a renderer but no builder (grouped bar, stacked bar, stacked area) are
> tracked in an `AWAITING_A_BUILDER` list whose test fails the moment one of
> them grows a builder. **Nobody has to remember to update the docs.**

For each: wire it or delete it. Every one left half-built is a divergence
waiting to happen.

| Type | State when audited | Decision | Outcome |
| --- | --- | --- | --- |
| **rug** | `PlotRender` returned `Ok(())` — drew nothing, reported success | **Wire.** One `draw_line` per point from the existing `compute_rug_lines`. | done — `Plot::rug` |
| **dendrogram** | `dendrogram_lines()` returns segments; no renderer, no builder | **Wire.** Closest to done. | done — `Plot::dendrogram`, `plots::hierarchical` un-hidden. Leaf *labels* are computed but not drawn, so no label setter is exposed |
| **hexbin** | 580 lines with `PlotCompute` + `PlotRender`; no builder, no `SeriesType` | Wire (needs `hex_size_x`/`hex_size_y`) or delete and drop the doc row. | done — `Plot::hexbin` |
| **strip, swarm** | `PlotCompute` + `PlotRender` exist and are re-exported; no builder | Wire; wrap `config.size` in `points_to_pixels`. | done — `Plot::strip`, `Plot::swarm`. Categories take ordinal slots `0..n-1`; they do not yet carry tick labels |
| **grouped/stacked bar** | Compute exists and is re-exported | Falls out of Phase 3.6. | **open** — `BarConfig` reaches the renderer, the builder entry points do not exist |
| **jointplot, pairplot** | Layout math only; `.rugplot(bool)` doubly inert | Blocked on `add_axes`. | **open** — kept `#[doc(hidden)]`; every appearance field documented as inert and pinned by `config_fields_that_are_inert` |
| **regplot, residplot** | Compute returns a CI band nothing can display — and the band is statistically wrong | Fix the statistics *before* anything draws them. | statistics fixed; still `#[doc(hidden)]` compute, no renderer |
| **sankey, streamplot** | `flow/mod.rs` was four lines of doc comment, zero code, `pub mod` | Remove the `pub mod` or feature-gate. | done — module deleted |

The first four rows were the same shape five times: a `SeriesType` variant, a
`PlotBuilder<C>`-returning method, and a bounds arm. They were done **as one
change with one helper, not five** — `ComputedSeries` — for the same reason the
four legend layouts and the thirteen `legend_position` copies were collapsed.

The regplot finding was confirmed rather than inherited. `compute_regplot`
returned `ŷ ± z·σ̂`: constant half-width, no leverage term, and a normal instead
of a Student-t quantile. Measured against the textbook interval for the mean
response it was **9.9× too wide at the centre of a 100-point fit** and 5.0× too
wide at the ends, while at `n = 5` the missing `t` made it a third too *narrow* —
the two errors run in opposite directions, which is why it looked plausible at
small `n`. It now computes
`ŷ(x₀) ± t(level, n−p)·σ̂·√(x₀ᵀ(XᵀX)⁻¹x₀)`, with the `t` quantile from a
regularized incomplete beta and the leverage from a solve against the Gram
matrix. Both are checked against published tables and closed forms.

`add_axes` (arbitrary-rectangle axes) unblocks jointplot, pairplot, regplot,
residplot and a dendrogram-plus-heatmap clustermap together — it is now the only
thing standing between the compute-only modules and a renderer. 2D `axis_equal`
unblocks square heatmap cells and an undistorted quiver.

### Why `#[doc(hidden)]` and not an `unstable-plots` feature

Both were on the table for `composite` and `regression` (§2.3 offered either).
The rule that settled it: **a module is hidden if and only if it has no
renderer at all.** `hierarchical` has one — and now a builder too — so it is visible;
`composite` (layout rectangles only) and `regression` (numbers only) have
nothing to draw with, so they stay hidden.

`#[doc(hidden)]` beat a feature gate for those two because:

- A cargo feature is a promise about a **compilation** boundary, and there is
  nothing here to compile out — these are pure functions over `&[f64]` with no
  dependencies, no `cfg` sites and no cost when unused.
- `tests/feature_hygiene_test.rs` (Phase 5) exists precisely to forbid features
  that gate nothing. An `unstable-plots` feature gating only rustdoc visibility
  would need an exemption on the day it was added.
- The guide snippets and `tests/config_enum_defaults_test.rs` call these
  functions today; a feature gate breaks them for no gain.
- The actual harm was docs.rs listing plot types a user cannot draw.
  `#[doc(hidden)]` removes exactly that and nothing else.

## Decisions required

> **Status: 3 and 6 are settled; 1, 2, 4 and 5 need re-checking against the
> current tree before anyone acts on them.** Items are kept with their original
> wording so the reasoning is not lost.

1. **3D goldens are stale and their `--ignored` test fails.** The diff was verified to be
   100% the 3D pane grid tint — geometry, markers and text are unmoved. Fix is two coupled
   steps: `UPDATE_3D_GOLDENS=1 cargo test --features 3d --test three_d_visual_test -- --ignored`,
   then propagate the same 8 PNGs to `docs/assets/gallery/rust/3d/`, or
   `committed_gallery_assets_match_the_exact_golden_images` breaks.
2. **`docs/assets/` is stale repo-wide** — every committed gallery and rustdoc PNG still shows
   the old edge-to-edge framing and near-white grid. `make release-docs-rust` regenerates them
   but asserts it is on `docs/release-0.4.0-refresh`.
3. **`parallel` and `gpu`: delete or commit.** *Settled in `258f3e8`:* the
   unreachable 2D parallel renderer was deleted. `parallel` stays a default
   feature, but now only because the software 3D rasterizer genuinely renders
   tiles across a rayon pool; it changes no 2D output and no 2D timing.
4. **SymLog autoscale margins** are skipped because `SymLogScale::symlog` is private. Make it
   `pub(crate)` and add the arm if it matters.
5. **Box/violin framing** changed with the margin work: their by-construction 0..1 x range is
   now −0.05..1.05. Intentional, but a judgement call — Phase 3.5 supersedes it.
6. **`Plot::save()` to a `.svg` path silently writes PNG bytes** when the `svg` feature is off,
   rather than erroring. *Settled in `258f3e8`:* SVG export is always compiled
   in and `svg` gates nothing, so the path cannot be taken. The feature is
   retained as a documented no-op so existing `features = ["svg"]` selections
   keep resolving.

## Suggested sequencing

Batches 1–10 below are **complete except for the remainder of Phase 9** — see the status table at the top for what landed
where. Kept for the reasoning about ordering, which still applies to what is
left.

| Batch | Contents | Effort | State |
| --- | --- | --- | --- |
| 1 | Phase 1 in full — rectangle primitive first, then the single-file fixes | S | done |
| 2 | Phase 2 — prelude, naming, doc-truth, deprecate inert setters | S | done |
| 3 | Phase 4 correctness hazards (parallelisable with 1–2) | S–M | done |
| 4 | Phase 3.3 tick generator → unblocks colorbar and contour levels | M | done |
| 5 | Phase 3.2 bounds → unblocks error whiskers and categorical positions | M | done |
| 6 | Phase 3.1 bar geometry + 3.6 `BarConfig` → SVG alignment, horizontal/grouped/stacked bar | M | done |
| 7 | Phase 3.4 legend + 3.5 categorical + Phase 6 presentation | M | done |
| 8 | Phase 5 truth-in-advertising + Phase 8 CI credibility | M | done |
| 9 | Phase 7 3D correctness (independent; can start any time after Batch 1) | M | done |
| 10 | Phase 9 structural, then Phase 10 unwired types | L | builder unification, `thiserror`, the `Vec<f64>` fast path and the owned-field `Styled<T>` done; Phase 10 done, including the five `Plot::` builders; `add_axes` open |

### What to do next

1. **`add_axes`** — one feature that unblocks jointplot, pairplot, regplot,
   residplot and clustermaps at once, and retires two `#[doc(hidden)]` modules.
2. **Grouped/stacked bar builders** — the compute and `BarConfig` plumbing are
   already there; this is the last row of the Phase 10 table that is pure
   wiring.
3. **The rest of Phase 9** — the gpui workspace split and the `rfd` pin.
4. **Regenerate the goldens and gallery assets** if any categorical figure
   moved: the bottom margin of a categorical plot is now measured from the
   category strings rather than from numeric x tick labels that were never
   drawn.
