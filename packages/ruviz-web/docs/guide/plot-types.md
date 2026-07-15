# Plot Types

The fluent builder covers the plot families exposed by the current Web package
API. The live gallery below uses the canonical files from
`packages/ruviz-web/examples/`.

## Builder Methods

| Plot | Method |
| --- | --- |
| Line | `line({ x, y })` |
| Scatter | `scatter({ x, y })` |
| Bar | `bar({ categories, values })` |
| Histogram | `histogram(values)` |
| Boxplot | `boxplot(values)` |
| Heatmap | `heatmap(rows)` |
| Vertical error bars | `errorBars({ x, y, yErrors })` |
| Horizontal and vertical error bars | `errorBarsXY({ x, y, xErrors, yErrors })` |
| Kernel density estimate | `kde(values)` |
| ECDF | `ecdf(values)` |
| Contour | `contour({ x, y, z })` |
| Pie | `pie(values, labels?)` |
| Radar | `radar({ labels, series })` |
| Violin | `violin(values)` |
| Polar line | `polarLine({ r, theta })` |

Numeric inputs accept `number[]`, `Float64Array`, or `ArrayLike<number>`.
Length checks are enforced for paired series. Heatmap rows must be rectangular,
and contour data must provide exactly `x.length * y.length` `z` values.

## Snapshots

`plot.toSnapshot()` returns a `PlotSnapshot`:

```ts,ignore,reason=abbreviated-public-type
// Abbreviated shape; import PlotSnapshot from ruviz for executable code.
type PlotSnapshot = {
  sizePx?: [number, number];
  theme?: "light" | "dark";
  ticks?: boolean;
  title?: string;
  xLabel?: string;
  yLabel?: string;
  series: PlotSeriesSnapshot[];
};
```

Snapshot series use `kind` values matching the builder families, including
`"line"`, `"scatter"`, `"bar"`, `"histogram"`, `"boxplot"`, `"heatmap"`,
`"error-bars"`, `"error-bars-xy"`, `"kde"`, `"ecdf"`, `"contour"`, `"pie"`,
`"radar"`, `"violin"`, and `"polar-line"`.

Static and observable numeric sources snapshot as `{ kind, values }`. Sine
signals snapshot as `{ kind: "sine-signal", options }`. Rehydrate snapshots
with `createPlotFromSnapshot(snapshot)` or `PlotBuilder.fromSnapshot(snapshot)`.

<PlotGallery :categories="['basic', 'statistical', 'matrix', 'categorical', 'specialized']" />
