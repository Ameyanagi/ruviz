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
cd python
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
the output `Path`. `render_png()` returns `bytes`; `render_svg()` returns `str`.

## DataFrame Inputs

The high-level API accepts named columns through `data=...`:

```python
import pandas as pd
import ruviz

frame = pd.DataFrame({"time": [0, 1, 2], "value": [0.2, 0.8, 1.1]})
plot = ruviz.plot().line("time", "value", data=frame)
```

This works with:

- pandas `DataFrame` and `Series`
- Polars `DataFrame` and `Series`
- `dict`-backed column data
- plain NumPy arrays, lists, and other array-like inputs

The `data=` column lookup is available on `line`, `scatter`, `bar`,
`histogram`, `boxplot`, `error_bars`, `error_bars_xy`, `kde`, `ecdf`,
`contour`, `pie`, `violin`, and `polar_line`.

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
Other plot types accept observable values as array-like input but snapshot them
when the series is added.

## Examples

Runnable examples live in `python/examples/`. The gallery page is generated from
those source files.
