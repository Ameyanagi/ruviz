# Interactivity

`ruviz` chooses the interactive runtime based on where it is running.

## Jupyter

In notebooks, `plot.show()` returns and displays a `RuvizWidget` backed by the WASM frontend.

```python
import ruviz

source = ruviz.observable([0.2, 0.9, 0.5, 1.3, 0.8])
plot = ruviz.plot().line([0, 1, 2, 3, 4], source)
widget = plot.show()

source.replace([0.3, 1.1, 0.7, 1.0, 0.6])
```

The widget UI includes PNG and SVG export actions for the current interactive view.

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
