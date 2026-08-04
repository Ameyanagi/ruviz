# ruviz for Python

`ruviz` for Python exposes the Rust plotting runtime through a fluent Python
API with three main workflows:

- static export with `save()`, `render_png()`, and `render_svg()`
- explicit notebook widgets with `plot.widget()`
- native interactive desktop windows with `plot.show()`

## Why Use It

- the same plot builder works across scripts, notebooks, and desktop sessions
- pandas, Polars, dict, and array-like inputs work through the same API
- fifteen 2D plot types with per-series styling and full axis control
- notebook widgets reuse the browser runtime instead of a separate Python-only frontend
- native static export stays in Rust; `save()` writes PNG, SVG, or PDF files
- inline types with a `py.typed` marker, so checkers catch bad style names early
- NumPy arrays reach the renderer as a single `memcpy`, and rendering releases the GIL

## Install

```sh
pip install ruviz
```

Install `ruviz[widget]` for notebook widgets, and `ruviz[dataframes]`,
`ruviz[pandas]`, or `ruviz[polars]` when you want named dataframe column
inputs; `ruviz[all]` installs every extra. The package requires Python 3.10 or
newer.

## First Plot

```python
import numpy as np
import ruviz

x = np.linspace(0.0, 4.0, 50)

(
    ruviz.plot()
    .line(x, x**2, label="x^2", color="#2563eb", width=2.0)
    .line(x, x**1.5, label="x^1.5", color="orange", linestyle="dashed")
    .title("Power Curves")
    .xlabel("x")
    .ylabel("y")
    .grid(True)
    .legend("upper_left")
    .save("power-curves.png")
)
```

## Where To Go Next

- Use **Getting Started** for installation, dataframe inputs, and export basics.
- Use **Interactivity** for Jupyter widgets and native `show()` behavior.
- Use **Gallery** for runnable example-backed screenshots.
- Use **API Reference** for the full generated Python reference.
