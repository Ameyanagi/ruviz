# Getting Started

## Install

For normal use:

```sh
pip install ruviz
```

Install the widget extra for notebook widgets, and the dataframe extras if you
want pandas or Polars inputs:

```sh
pip install "ruviz[widget]"
pip install "ruviz[dataframes]"
pip install "ruviz[pandas]"
pip install "ruviz[polars]"
pip install "ruviz[all]"
```

`ruviz` requires Python 3.10 or newer. The base install only pulls in `numpy`;
`anywidget` and `traitlets` come with `ruviz[widget]`, and pandas and Polars
are optional. Calling `plot.widget()` or importing `ruviz.RuvizWidget` without
the widget extra raises an `ImportError` that names the extra to install.
`ruviz.__version__` reports the installed distribution version.

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

`data=` accepts anything indexable by column name: a pandas or Polars
`DataFrame`, any `Mapping` (a `dict`, a `MappingProxyType`, a `UserDict`), or
any other object with a `__getitem__`. A name that is not in the source raises
`KeyError`. A pandas or Polars `Series`, a NumPy array, or a list is a direct
x/y input instead — pass it positionally without `data=`:

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
ruviz.plot().bar(
    ["Documentation", "Rendering", "Bindings"],
    [42, 31, 18],
    orientation="horizontal",
)
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
color such as `"red"`, `"orange"`, `"teal"`, or `"crimson"`. `linestyle` is one
of `solid`, `dashed`, `dotted`, `dash-dot`, `dash-dot-dot`, and `marker` is one
of `circle`, `square`, `triangle`, `triangle-down`, `diamond`, `plus`, `cross`,
`star`, `circle-open`, `square-open`, `triangle-open`, `diamond-open`. The
matplotlib shorthands work too: `"o"`, `"s"`, `"^"`, `"v"`, `"D"`, `"+"`, `"x"`,
`"*"` for markers and `"-"`, `"--"`, `":"`, `"-."` for line styles; snapshots
store the canonical name they resolve to.
Unsupported names raise `ValueError` listing the accepted values, at the call
that used them rather than at render time; an unknown color name also gets a
"did you mean" suggestion.

Which keywords a series takes follows what the renderer honors for that kind:

| method | keywords |
| --- | --- |
| `line` | `label`, `color`, `alpha`, `width`, `linestyle`, `marker`, `marker_size` |
| `scatter` | `label`, `color`, `alpha`, `marker`, `marker_size` |
| `bar` | `label`, `color`, `alpha`, `orientation` (`"vertical"` or `"horizontal"`) |
| `histogram` | `label`, `color`, `alpha`, `bins`, `density` |
| `boxplot` | `label`, `color`, `alpha`, `width`, `linestyle`, `orientation` (`"vertical"` or `"horizontal"`), `show_mean`, `width_ratio` |
| `kde` | `label`, `color`, `alpha`, `width`, `bandwidth` |
| `ecdf`, `violin`, `polar_line`, `error_bars`, `error_bars_xy` | `label`, `color`, `alpha`, `width` |
| `contour` | `alpha`, `width`, `levels` |

`heatmap`, `pie`, and `radar` take no style keywords; a `radar` series that
carries a `name` is labelled once you add `.legend(...)` to the plot.

`histogram(density=True)` normalizes the bars to a probability density, which is
the scale a `kde()` overlay is drawn on; without it the KDE curve sits flat at
zero against a counts axis.

Plot-level settings:

- `legend(position="best")` — `"best"` plus the core positions as lowercase
  names, such as `"upper_right"`, `"center"`, or `"outside_right"`
- `grid(enabled=True)`
- `dpi(dpi)` — scales the exported pixels from `size_px(...)`, so
  `size_px(640, 480).dpi(200)` exports a 1280×960 image
- `xlim(min, max)` / `ylim(min, max)` — finite and different; inverted bounds
  such as `xlim(10, 0)` render a descending axis
- `xscale(scale, linthresh=None)` / `yscale(...)` — `"linear"`, `"log"`, or
  `"symlog"`, where `linthresh` (default `1.0`) applies to `"symlog"` only

All of these round-trip through `to_snapshot()`, `clone()`, and `deepcopy`.
Notebook widgets render them too: the WASM runtime applies series styles and
the plot-level settings from the snapshot.

## Validation Worth Knowing

The API rejects ambiguous input at the call that made it rather than at render
time, so a mistake surfaces with the arguments still in scope.

- **1D means 1D.** Numeric inputs to a 1D series must be one-dimensional; a 2D
  array raises `TypeError` instead of being silently flattened. `heatmap` is
  the exception and takes a rectangular 2D matrix, and `contour` takes a `z`
  that is already flattened row-major (`len(z) == len(x) * len(y)`).
- **A DataFrame is never a vector.** Passing one positionally raises
  `TypeError`; select a column or use `data=`.
- **`data=` takes a column source**, not a Series: a DataFrame, a `Mapping`,
  or anything else indexable by name. A pandas or Polars `Series` is a direct
  value — pass it positionally.
- **`save()` accepts `.png`, `.svg`, and `.pdf` only.** Any other extension, or
  a path without one, raises `ValueError`.
- **`size_px(width, height)`** and **`dpi(dpi)`** take whole numbers and raise
  `ValueError` on a non-positive or fractional value; `bins=` and `levels=` are
  the same, so `bins=2.9` is rejected rather than truncated to `2`.
- **`theme()`** takes one of `"light"`, `"dark"`, `"seaborn"`, `"publication"`,
  `"minimal"`, or `"presentation"`. It is case-insensitive and raises
  `ValueError` on an unknown name. `"seaborn"` reproduces `seaborn.set_theme()`
  — lavender-gray panel, white grid, no spines or tick marks, `deep` palette —
  so a migrated script keeps the look it had. `Plot3D.theme()` still takes only
  `"light"` and `"dark"`.
- **2D axis limits** must be finite and different; passing them inverted keeps
  a descending axis. `Plot3D` limits stay strictly ascending. `linthresh` must
  be a finite positive number that only applies to the `"symlog"` scale.
- **Style names are checked eagerly.** An unknown color, line style, marker,
  legend position, or scale raises `ValueError` listing the accepted values,
  and an unknown color adds a "did you mean" suggestion.
- **Observables only go where they can be tracked** — see Reactive Data below.

## Plot Lifecycle

- `plot()` creates a fluent builder
- plot methods append series and update presentation state
- `save()` writes a PNG, SVG, or PDF file and returns the output `Path`
- `render_png()` returns PNG bytes and `render_svg()` returns an SVG string
- PNG output records the effective DPI in `pHYs` metadata for print/layout software
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

`source.replace(...)` may change the length, but only when every series bound to
`source` — or to an observable derived from it — still holds together
afterwards. Otherwise it raises `ValueError` and nothing mutates. See
[Interactivity](interactive.md) for the details.

## Examples

Runnable examples live in `bindings/python/examples/`. The gallery page is generated from
those source files.
