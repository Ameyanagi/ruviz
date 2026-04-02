# Interactivity

`ruviz` chooses the interactive runtime based on where it is running.

## Jupyter

In notebooks, a bare `plot` result and `plot.show()` both render a static PNG snapshot by default.
Use `plot.widget()` when you explicitly want the interactive `RuvizWidget` backed by the WASM
frontend. The notebook frontend is bundled with Bun from `python/python/ruviz/widget.entry.js`
and uses the main-thread canvas runtime for compatibility with `anywidget`'s blob-based module
loader.

```python
import ruviz

source = ruviz.observable([0.2, 0.9, 0.5, 1.3, 0.8])
plot = ruviz.plot().line([0, 1, 2, 3, 4], source)
image = plot.show()
widget = plot.widget()

source.replace([0.3, 1.1, 0.7, 1.0, 0.6])
```

Observable updates stay live only in the widget view. Static notebook output is a one-time PNG
snapshot.

The widget UI includes PNG and SVG export actions for the current interactive view.

After changing the notebook frontend or the web SDK, regenerate the checked-in widget bundle from
the repository root. The build bootstraps the repo-pinned `wasm-pack` tool
automatically:

```sh
bun run build:python-widget
```

## Console

Outside notebooks, `plot.show()` opens the native `winit` interactive window:

```python
import ruviz

ruviz.plot().scatter([0, 1, 2], [1.2, 0.4, 1.7]).show()
```

## Examples

- `examples/notebook_observable.py`
- `examples/notebook_export.py`
- `examples/console_interactive.py`
