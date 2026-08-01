# ruviz for Python

`ruviz` for Python exposes the Rust plotting runtime through a fluent Python
API with three main workflows:

- static export with `save()`, `render_png()`, and `render_svg()`
- explicit notebook widgets with `plot.widget()`
- native interactive desktop windows with `plot.show()`

## Why Use It

- the same plot builder works across scripts, notebooks, and desktop sessions
- pandas, Polars, dict, and array-like inputs work through the same API
- notebook widgets reuse the browser runtime instead of a separate Python-only frontend
- native static export stays in Rust; `save()` writes PNG, SVG, or PDF files

## Install

```sh
pip install ruviz
```

Install `ruviz[dataframes]`, `ruviz[pandas]`, or `ruviz[polars]` when you want
named dataframe column inputs. The package requires Python 3.10 or newer.

## First Plot

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

## Where To Go Next

- Use **Getting Started** for installation, dataframe inputs, and export basics.
- Use **Interactivity** for Jupyter widgets and native `show()` behavior.
- Use **Gallery** for runnable example-backed screenshots.
- Use **API Reference** for the full generated Python reference.
