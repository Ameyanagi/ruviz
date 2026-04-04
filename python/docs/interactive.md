# Interactivity

`ruviz` uses different interactive paths depending on where the plot is shown.

## Jupyter Default Behavior

In notebooks, a bare plot result and `plot.show()` both render a static PNG
snapshot. This keeps notebook output predictable and avoids starting a widget
frontend unless you ask for one explicitly.

```python
import ruviz

plot = ruviz.plot().line([0, 1, 2], [0, 1, 0]).title("Notebook Snapshot")
plot.show()
```

## Explicit Widgets

Use `plot.widget()` when you want the synced browser/WASM widget:

```python
import numpy as np
import ruviz

x = np.linspace(0.0, 6.0, 200)
y = ruviz.observable(np.sin(x))

plot = ruviz.plot().line(x, y).title("Live Sine Wave")
widget = plot.widget()
```

Observable updates stay live in the widget:

```python
y.replace(np.cos(x))
```

## Desktop Windows

Outside notebooks, `plot.show()` opens the native interactive window:

```python
import ruviz

ruviz.plot().scatter([0, 1, 2], [1.2, 0.4, 1.7]).show()
```

## Widget Bundles

The widget frontend is bundled from `python/python/ruviz/widget.entry.js` and
the web SDK. Rebuild it from the repository root after frontend changes:

```sh
bun run build:python-widget
```

The release workflow rebuilds the canonical bundled widget before packaging.
