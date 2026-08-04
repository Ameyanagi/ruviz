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
  .yscale("symlog", 1);

await plot.renderPng();
```

## Snapshots

`plot.toSnapshot()` returns a `PlotSnapshot`:

```ts,ignore,reason=abbreviated-public-type
// Abbreviated shape; import PlotSnapshot from ruviz for executable code.
type PlotSnapshot = {
  schemaVersion?: number;
  sizePx?: [number, number];
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
the Python binding. Keys a build does not know are ignored and preserved across
clones, so older and newer snapshots both render.

<PlotGallery :categories="['basic', 'statistical', 'matrix', 'categorical', 'specialized']" />
