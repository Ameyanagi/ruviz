# ruviz for Python

`ruviz` for Python wraps the Rust plotting runtime with a fluent Python API,
static export helpers, native desktop `show()`, and notebook widget support.

## Install

```sh
pip install ruviz
```

## Quick Start

```python
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
- Outside notebooks, `plot.show()` opens the native interactive window.
- `plot.render_png()` and `plot.render_svg()` return in-memory export data.

## Reactive Notebook Data

Use `ruviz.observable(...)` for notebook-driven updates that keep widget state
in sync:

```python
import numpy as np
import ruviz

x = np.linspace(0.0, 6.0, 200)
y = ruviz.observable(np.sin(x))

plot = ruviz.plot().line(x, y).title("Live Sine Wave")
widget = plot.widget()
```

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
the web SDK or `python/python/ruviz/widget.entry.js`:

```sh
bun run build:python-widget
```
