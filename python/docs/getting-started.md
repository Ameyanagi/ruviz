# Getting Started

## Install

For normal use:

```sh
pip install ruviz
```

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

## Plot Lifecycle

- `plot()` creates a fluent builder
- plot methods append series and update presentation state
- `save()` writes a file
- `render_png()` and `render_svg()` return in-memory export data
- `to_snapshot()` serializes the current state for widget sync and inspection

## Examples

Runnable examples live in `python/examples/`. The gallery page is generated from
those source files.
