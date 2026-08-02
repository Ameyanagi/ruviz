# Getting Started

## Install

For normal use:

```sh
pip install ruviz
```

Install dataframe extras if you want pandas or Polars inputs:

```sh
pip install "ruviz[dataframes]"
pip install "ruviz[pandas]"
pip install "ruviz[polars]"
```

`ruviz` requires Python 3.10 or newer. The base install includes `anywidget`,
`numpy`, and `traitlets`; pandas and Polars are optional.

For local contributor builds:

```sh
cd bindings/python
uv sync
uv run maturin develop
```

## Static Export

```python
import ruviz

plot = (
    ruviz.plot()
    .size_px(800, 420)
    .title("Static export")
    .xlabel("time")
    .ylabel("value")
    .line([0, 1, 2], [0.5, 0.75, 1.4])
)

plot.save("plot.png")
svg = plot.render_svg()
png_bytes = plot.render_png()
```

`save(path)` writes PNG, SVG, or PDF according to the file extension and returns
the output `Path`. Any other extension, or a path without one, raises
`ValueError`. `render_png()` returns `bytes`; `render_svg()` returns `str`.

## DataFrame Inputs

The high-level API accepts named columns through `data=...`:

```python
import pandas as pd
import ruviz

frame = pd.DataFrame({"time": [0, 1, 2], "value": [0.2, 0.8, 1.1]})
plot = ruviz.plot().line("time", "value", data=frame)
```

`data=` accepts a pandas `DataFrame`, a Polars `DataFrame`, or a `dict` of
columns. A pandas or Polars `Series`, a NumPy array, or a list is a direct x/y
input instead — pass it positionally without `data=`:

```python
plot = ruviz.plot().line(frame["time"], frame["value"])
```

The `data=` column lookup is available on `line`, `scatter`, `bar`,
`histogram`, `boxplot`, `heatmap`, `error_bars`, `error_bars_xy`, `kde`,
`ecdf`, `contour`, `pie`, `violin`, and `polar_line`.

Numeric inputs must be one-dimensional; `heatmap` takes a 2D matrix and
`contour` takes a flattened `z` grid.

## Plot Types

The fluent builder appends each series in call order:

```python
import ruviz

ruviz.plot().line([0, 1, 2], [0.0, 0.8, 0.3])
ruviz.plot().scatter([0, 1, 2], [0.0, 0.8, 0.3])
ruviz.plot().bar(["CPU", "WASM", "Jupyter"], [3.8, 4.9, 4.1])
ruviz.plot().histogram([0.2, 0.4, 0.4, 0.9])
ruviz.plot().boxplot([0.2, 0.4, 0.4, 0.9])
ruviz.plot().heatmap([[0.1, 0.4], [0.8, 0.2]])
ruviz.plot().error_bars([0, 1, 2], [1.0, 1.2, 0.9], [0.1, 0.2, 0.1])
ruviz.plot().error_bars_xy([0, 1], [1.0, 1.2], [0.1, 0.1], [0.2, 0.2])
ruviz.plot().kde([0.2, 0.4, 0.4, 0.9])
ruviz.plot().ecdf([0.2, 0.4, 0.4, 0.9])
ruviz.plot().contour([0, 1], [0, 1], [0.1, 0.2, 0.3, 0.4])
ruviz.plot().pie([30, 70], ["static", "widget"])
ruviz.plot().radar(["API", "Docs"], [{"name": "Python", "values": [4.5, 4.7]}])
ruviz.plot().violin([0.2, 0.4, 0.4, 0.9])
ruviz.plot().polar_line([1.0, 1.2, 1.1], [0.0, 1.57, 3.14])
```

## Styling and Axes

Every series takes keyword-only style arguments, and the plot itself takes
legend, grid, limit, and scale settings:

```python
import ruviz

plot = (
    ruviz.plot()
    .line([0, 1, 2], [1.0, 2.5, 4.0], label="Revenue", color="#2563eb", width=2.0)
    .scatter([0, 1, 2], [1.2, 2.2, 3.6], label="Samples", marker="diamond", marker_size=8.0)
    .legend("upper_left")
    .grid(True)
    .ylim(0.0, 5.0)
)
```

`color` accepts a hex string (`"#2563eb"`, `"#25f"`, `"#2563eb80"`) or a named
color such as `"red"`. `linestyle` is one of `solid`, `dashed`, `dotted`,
`dash-dot`, `dash-dot-dot`, and `marker` is one of `circle`, `square`,
`triangle`, `triangle-down`, `diamond`, `plus`, `cross`, `star`, and their
`-open` variants. Unsupported names raise `ValueError` listing the accepted
values, at the call that used them rather than at render time.

Which keywords a series takes follows what the renderer honors for that kind:

| method | keywords |
| --- | --- |
| `line` | `label`, `color`, `alpha`, `width`, `linestyle`, `marker`, `marker_size` |
| `scatter` | `label`, `color`, `alpha`, `marker`, `marker_size` |
| `bar` | `label`, `color`, `alpha` |
| `histogram` | `label`, `color`, `alpha`, `bins` |
| `boxplot` | `label`, `color`, `alpha`, `width`, `linestyle` |
| `kde` | `label`, `color`, `alpha`, `width`, `bandwidth` |
| `ecdf`, `violin`, `polar_line`, `error_bars`, `error_bars_xy` | `label`, `color`, `alpha`, `width` |
| `contour` | `alpha`, `width`, `levels` |

`heatmap`, `pie`, and `radar` take no style keywords.

Plot-level settings:

- `legend(position="best")` — `"best"` plus the core positions as lowercase
  names, such as `"upper_right"`, `"center"`, or `"outside_right"`
- `grid(enabled=True)`
- `xlim(min, max)` / `ylim(min, max)` — finite and strictly ascending
- `xscale(scale, linthresh=None)` / `yscale(...)` — `"linear"`, `"log"`, or
  `"symlog"`, where `linthresh` (default `1.0`) applies to `"symlog"` only

All of these round-trip through `to_snapshot()`, `clone()`, and `deepcopy`.
Notebook widgets carry them in the snapshot but do not paint them yet; the WASM
runtime renders styled series in a later phase.

## Plot Lifecycle

- `plot()` creates a fluent builder
- plot methods append series and update presentation state
- `save()` writes a PNG, SVG, or PDF file and returns the output `Path`
- `render_png()` returns PNG bytes and `render_svg()` returns an SVG string
- `to_snapshot()` serializes the current state for widget sync and inspection
- `copy.deepcopy(plot)` creates an independent live copy, while `plot.clone()` stays snapshot-only

## Reactive Data

`ObservableSeries` works as both a mutable data source and a live math input:

```python
from copy import deepcopy
import numpy as np
import ruviz

source = ruviz.observable([0.2, 0.8, 1.3])
scaled = np.sin(source * 2.0)
plot = ruviz.plot().line([0, 1, 2], scaled)
template = deepcopy(plot)
```

`scaled` updates when `source` changes. If you write to `scaled` directly, it
detaches from `source` and becomes its own mutable observable.

Live observables are passed through to the native renderer for `line`,
`scatter`, `bar`, `histogram`, `boxplot`, `error_bars`, and `error_bars_xy`.
Other plot types cannot track observables and raise `TypeError` when given one;
pass `series.snapshot_values()` if you want a static copy.

## Examples

Runnable examples live in `bindings/python/examples/`. The gallery page is generated from
those source files.
