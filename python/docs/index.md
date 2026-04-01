# ruviz for Python

`ruviz` exposes a fluent plotting builder for static exports, notebook widgets, and native
interactive windows.

## Highlights

- Fluent builder API with chained plot methods such as `plot().line(...).title(...)`
- Static PNG, SVG, and PDF export via the native `pyO3` binding
- Notebook interactivity through a WASM-backed `anywidget`
- Native console interactivity through the existing `winit` runtime
- Dataframe-friendly `data=` inputs for pandas, Polars, and dict-backed column data

## Install for Local Development

```sh
cd python
uv sync
uv run maturin develop
```

## First Plot

```python
import ruviz

plot = (
    ruviz.plot()
    .title("Quadratic")
    .xlabel("x")
    .ylabel("y")
    .line([0, 1, 2, 3], [0, 1, 4, 9])
)

plot.save("quadratic.png")
```

Use the navigation for the full example gallery, interactive notebook guidance, and API reference.
