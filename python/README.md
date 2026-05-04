# ruviz for Python

`ruviz` for Python wraps the Rust plotting runtime with a fluent Python API,
static export helpers, native desktop `show()`, and notebook widget support.
It requires Python 3.10 or newer and installs the runtime dependencies needed
for NumPy inputs, static rendering, and AnyWidget notebooks.

## Install

```sh
pip install ruviz
```

If you want pandas or Polars dataframe inputs, install the matching optional
extra:

```sh
pip install "ruviz[dataframes]"
# or only one dataframe backend:
pip install "ruviz[pandas]"
pip install "ruviz[polars]"
```

## Quick Start

```python,check
import numpy as np
import ruviz

x = np.linspace(0.0, 4.0, 50)
y = x**2

(
    ruviz.plot()
    .line(x, y)
    .title("Quadratic")
    .xlabel("x")
    .ylabel("y = x^2")
    .save("quadratic.png")
)
```

## Notebook and Desktop Usage

- In Jupyter, `plot.show()` displays a static PNG in the cell output.
- Use `plot.widget()` when you want the synced WASM-backed notebook widget.
- `plot.size_px(width, height)` also controls the widget's displayed size and aspect ratio.
- Without `size_px(...)`, the widget uses the default PNG size (`640x480`) and shrinks proportionally if the notebook column is narrower.
- Drag the widget's bottom-right handle to resize the display freely.
- Hold `Shift` or `Ctrl` while dragging the handle to preserve the current aspect ratio.
- In the widget, right click opens the export menu and right drag performs box zoom.
- Outside notebooks, `plot.show()` opens the native interactive window.
- The published Linux wheel focuses on static rendering and notebook widgets. Install from source on Linux if you need the native desktop `plot.show()` window.
- `plot.render_png()` returns PNG bytes and `plot.render_svg()` returns an SVG string.
- `plot.save(path)` writes PNG, SVG, or PDF according to the file extension and returns the output `Path`.

## Reactive Notebook Data

Use `ruviz.observable(...)` for notebook-driven updates that keep explicit
widgets in sync:

```python,check
import numpy as np
import ruviz

x = np.linspace(0.0, 6.0, 200)
y = ruviz.observable(np.sin(x))

plot = ruviz.plot().size_px(640, 360).line(x, y).title("Live Sine Wave")
widget = plot.widget()
```

`ObservableSeries` supports elementwise arithmetic and NumPy ufuncs. Derived
observables stay live until you write to them directly. Live observable series
are supported by `line`, `scatter`, `bar`, `histogram`, `boxplot`,
`error_bars`, and `error_bars_xy`; other plot types snapshot their inputs.

```python,check
import numpy as np

scaled = np.sin(y * 2.0 + 0.25)
plot.line(x, scaled)
y.replace(np.cos(x))
```

`deepcopy(plot)` creates an independent live copy with fresh observables, while
`plot.clone()` remains a static snapshot copy.

## Documentation

- Python docs source: `python/docs/`
- Python examples: `python/examples/`
- Root project README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>

## Contributor Workflow

```sh
cd python
uv sync
uv run maturin develop
uv run python scripts/generate_gallery.py
uv run mkdocs serve
```

Rebuild the bundled widget frontend from the repository root when you change
the web SDK or `packages/ruviz-web/src/python-widget.ts`:

```sh
bun run build:python-widget
```
