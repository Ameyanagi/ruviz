# Plot Types

The fluent builder covers the plot families exposed by the current Web package
API. The live gallery below uses the canonical files from
`packages/ruviz/examples/`.

## Builder Methods

| Plot | Method |
| --- | --- |
| Line | `line({ x, y, style? })` |
| Scatter | `scatter({ x, y, style? })` |
| Bar | `bar({ categories, values, style? })` |
| Histogram | `histogram(values, style?)` |
| Boxplot | `boxplot(values, style?)` |
| Heatmap | `heatmap(rows)` |
| Vertical error bars | `errorBars({ x, y, yErrors, style? })` |
| Horizontal and vertical error bars | `errorBarsXY({ x, y, xErrors, yErrors, style? })` |
| Kernel density estimate | `kde(values, style?)` |
| ECDF | `ecdf(values, style?)` |
| Contour | `contour({ x, y, z, style? })` |
| Pie | `pie(values, labels?)` |
| Radar | `radar({ labels, series })` |
| Violin | `violin(values, style?)` |
| Polar line | `polarLine({ r, theta })` |

Numeric inputs accept `number[]`, `Float64Array`, or `ArrayLike<number>`.
Length checks are enforced for paired series. Heatmap rows must be rectangular,
and contour data must provide exactly `x.length * y.length` `z` values.

## Styling and Axes

Every series takes an optional `style`, typed to the options its plot family
supports: `label`, `color` (hex or named), `alpha`, `width`, `linestyle`,
`marker`, `markerSize`, plus `bins` for histograms, `bandwidth` for KDE, and
`levels` for contours. Heatmap, pie, and radar take no style.

Plot-level settings share the fluent form, each with a `setX` alias:

```ts,check
import { createPlot } from "ruviz";

const plot = createPlot()
  .line({
    x: [1, 2, 3],
    y: [1, 4, 9],
    style: { label: "signal", color: "#2563eb", linestyle: "dashed", marker: "square" },
  })
  .legend("upper_left")
  .grid(true)
  .xlim(0, 4)
  .yscale("symlog", 1)
  .sizePx(640, 480)
  .dpi(200);

await plot.renderPng();
```

`dpi` applies after `sizePx`, so it scales the exported pixels:
`sizePx(640, 480).dpi(200)` renders a 1280x960 image. `xlim`/`ylim` require
finite, different bounds and keep inverted ones, which render a descending axis.

Style values and axis names are checked at the builder call, not at the next
async render: an unknown color, marker, line style, legend position, or axis
scale throws a `RangeError` listing what the renderer accepts, with the same
message the renderer would have produced.

## Snapshots

`plot.toSnapshot()` returns a `PlotSnapshot`:

```ts,ignore,reason=abbreviated-public-type
// Abbreviated shape; import PlotSnapshot from ruviz for executable code.
type PlotSnapshot = {
  schemaVersion?: number;
  sizePx?: [number, number];
  dpi?: number;
  theme?: "light" | "dark";
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
  legend?: LegendPositionName;
  grid?: boolean;
  xLim?: [number, number];
  yLim?: [number, number];
  xScale?: [scale: AxisScaleName, linthresh?: number];
  yScale?: [scale: AxisScaleName, linthresh?: number];
  series: PlotSeriesSnapshot[];
};
```

Snapshot series use `kind` values matching the builder families, including
`"line"`, `"scatter"`, `"bar"`, `"histogram"`, `"boxplot"`, `"heatmap"`,
`"error-bars"`, `"error-bars-xy"`, `"kde"`, `"ecdf"`, `"contour"`, `"pie"`,
`"radar"`, `"violin"`, and `"polar-line"`.

Static and observable numeric sources snapshot as `{ kind, values }`. Sine
signals snapshot as `{ kind: "sine-signal", options }`, and a styled series
carries its options under `style`. Rehydrate snapshots with
`createPlotFromSnapshot(snapshot)` or `PlotBuilder.fromSnapshot(snapshot)`.

Snapshots are interchangeable with the ones `ruviz.Plot.to_snapshot()` emits in
the Python binding. Keys a build does not know -- including nested ones, and
including unknown `style` keys -- are ignored when rendering and preserved
across clones and round-trips, so older and newer snapshots both render.
`fromSnapshot` deep-copies its input, so a snapshot you keep mutating never
changes what the builder holds.

<PlotGallery :categories="['basic', 'statistical', 'matrix', 'categorical', 'specialized']" />
