# API Reference

The generated reference below covers the Python package surface that is shipped
to PyPI:

- `plot()` for creating new plot builders
- `Plot` for fluent plotting, export, widgets, native display, and safe Python copy/deepcopy support
- the `Plot` series methods (`line`, `scatter`, `bar`, `histogram`, `boxplot`,
  `heatmap`, `error_bars`, `error_bars_xy`, `kde`, `ecdf`, `contour`, `pie`,
  `radar`, `violin`, `polar_line`) with their keyword-only style arguments
- the `Plot` axis methods `legend`, `grid`, `xlim`, `ylim`, `xscale`, and `yscale`
- `plot3d()` and `Plot3D` for the static opaque 3D alpha
- `scatter3d()`, `line3d()`, `surface()`, and `wireframe()` for direct 3D construction
- `observable()` and `ObservableSeries` for synced notebook data, elementwise arithmetic, and NumPy ufuncs
- `RuvizWidget` for explicit notebook widget embedding (needs the `ruviz[widget]` extra)
- `__version__` plus the exported type aliases and snapshot `TypedDict`s

The public import surface is:

```python
from ruviz import (
    ObservableSeries,
    Plot,
    Plot3D,
    RuvizWidget,
    __version__,
    line3d,
    observable,
    plot,
    plot3d,
    scatter3d,
    surface,
    wireframe,
)
```

The package ships inline types with a `py.typed` marker, so type checkers read
the annotations directly. Alongside the classes above, `ruviz` exports every
name used in those annotations:

- input aliases — `ArrayLike`, `MatrixLike`, `LabelsLike`, `DataSource`
- the structural protocols behind them — `NumericVector`, `NumericMatrix`,
  `ColumnSource` — which describe NumPy arrays and pandas/Polars objects without
  importing those packages
- literal name unions — `Theme`, `LineStyleName`, `MarkerName`,
  `LegendPositionName`, `ScaleName`
- snapshot shapes — `PlotSnapshot`, `SeriesSnapshot`, `StyleDict`,
  `RadarSeriesDict`, `Plot3DSnapshot`, `Series3DSnapshot`

Because the style and axis names are literal unions, a checker rejects an
unsupported `marker=`, `linestyle=`, `theme=`, legend position, or axis scale
before the call reaches the renderer. Snapshots carry `schemaVersion: 1`;
consumers must ignore keys they do not recognize.

`Plot3D` currently provides deterministic CPU PNG and hybrid SVG/PDF export.
It uses orthographic projection by default and requires regular surface grids
with `z.shape == (len(y), len(x))`. Python interactive orbit widgets and
transparent 3D geometry are intentionally outside the alpha surface.

::: ruviz
