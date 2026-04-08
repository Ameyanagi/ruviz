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

plot = ruviz.plot().size_px(640, 360).line(x, y).title("Live Sine Wave")
widget = plot.widget()
```

When a plot has `size_px(width, height)` configured, the widget uses that as
its display size inside the notebook. If the notebook column is narrower than
the configured width, the widget shrinks proportionally while preserving the
same aspect ratio as the PNG/export output. If no plot size is configured, the
widget falls back to the default fixed height.

Notebook widget controls:

- `Mouse wheel`: zoom in/out under the cursor
- `Left drag`: pan
- `Right drag`: box zoom
- `Right click`: open the export menu with `Save PNG` and `Save SVG`

Observable updates stay live in the widget:

```python
y.replace(np.cos(x))
```

Observable math also stays live:

```python
phase = ruviz.observable(np.linspace(0.0, 1.0, x.size))
signal = np.sin((phase * 2.0) + 0.5)
```

Derived observables detach on the first direct write, so `signal.set_at(...)`
turns `signal` into an independent mutable series without mutating `phase`.

## Desktop Windows

Outside notebooks, `plot.show()` opens the native interactive window:

```python
import ruviz

ruviz.plot().scatter([0, 1, 2], [1.2, 0.4, 1.7]).show()
```

The published Linux wheel focuses on static rendering and notebook widgets. If
you need the native desktop window on Linux, install `ruviz` from source so the
interactive backend can be compiled against the local desktop stack.

## Widget Bundles

The widget frontend is bundled from `packages/ruviz-web/src/python-widget.ts` and
the web SDK. Rebuild it from the repository root after frontend changes:

```sh
bun run build:python-widget
```

The release workflow rebuilds the canonical bundled widget before packaging.
