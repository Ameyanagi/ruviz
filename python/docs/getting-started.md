# Getting Started

The Python package is a mixed `uv` + `maturin` project. Build the extension first, then use the
pure-Python surface from regular Python code.

## Development Setup

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
```

## DataFrame Inputs

The fluent API accepts dataframe columns via `data=...`:

```python
import pandas as pd
import ruviz

frame = pd.DataFrame({"time": [0, 1, 2], "value": [0.2, 0.8, 1.1]})

plot = ruviz.plot().line("time", "value", data=frame)
```

See `examples/dataframe_line.py` and the gallery page for the full example set.
