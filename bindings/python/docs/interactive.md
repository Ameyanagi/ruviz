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

Widgets live in the optional `widget` extra (`pip install "ruviz[widget]"`),
which adds `anywidget` and `traitlets`. Without it, `plot.widget()` and
`ruviz.RuvizWidget` raise an `ImportError` naming the extra.

Use `plot.widget()` when you want the synced browser/WASM widget:

```python
import numpy as np
import ruviz

x = np.linspace(0.0, 6.0, 200)
y = ruviz.observable(np.sin(x))

plot = ruviz.plot().size_px(640, 360).line(x, y).title("Live Sine Wave")
widget = plot.widget()
```

`plot.widget()` returns a `RuvizWidget` AnyWidget instance bound to the plot.
When observable data changes, the widget receives a refreshed JSON-friendly
snapshot from `plot.to_snapshot()`.

When a plot has `size_px(width, height)` configured, the widget uses that as
its display size inside the notebook. If the notebook column is narrower than
the configured width, the widget shrinks proportionally while preserving the
same aspect ratio as the PNG/export output. If no plot size is configured, the
widget falls back to the plot's default PNG size (`640x480`) and still shrinks
proportionally when the notebook column is narrower.

Notebook widget controls:

- `Mouse wheel`: zoom in/out under the cursor
- `Left drag`: pan
- `Right drag`: box zoom
- `Bottom-right drag handle`: resize the widget display freely
- `Shift` or `Ctrl` + drag handle: resize while preserving the current aspect ratio
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

### Which Plot Kinds Track Observables

Live observable updates are supported by `line`, `scatter`, `bar`, `histogram`,
`boxplot`, `error_bars`, and `error_bars_xy`. The remaining kinds — `kde`,
`ecdf`, `contour`, `pie`, `violin`, `polar_line`, `heatmap`, and `radar` —
cannot track a live source and raise `TypeError` when handed one. Pass
`series.snapshot_values()` to add a static copy of the current values instead.

### Resizing Is Atomic

`set_at(index, value)` never changes the length, so it always succeeds.
`replace(values)` may change the length, and that is where a plot can veto the
write: before anything mutates, `replace()` walks every series bound to this
observable *and* every series bound to an observable derived from it. If any of
them would end up with mismatched inputs, it raises `ValueError` and the whole
graph is left untouched.

```python
y = ruviz.observable([1.0, 2.0, 3.0])
plot = ruviz.plot().line([0, 1, 2], y)

y.replace([3.0, 2.0, 1.0])       # fine: same length
y.replace([1.0, 2.0, 3.0, 4.0])  # ValueError: the static x still has length 3
```

In practice:

- Single-vector kinds (`histogram`, `boxplot`) accept a resize freely.
- A kind with sibling inputs (`line`, `scatter`, `error_bars`,
  `error_bars_xy`, and `bar` with its category labels) rejects a resize that
  would desynchronize them, so those series are effectively fixed-length once
  bound.
- An observable that is not bound to any plot resizes freely.
- Deriving `y2 = y * 2.0` and plotting `y2` still guards `y`: resizing `y` is
  rejected if the resize would break the series bound to `y2`.

### Styling in the Widget

Series style keywords and the plot-level `legend`, `grid`, limit, and scale
settings are carried in the widget's snapshot, but the WASM runtime does not
paint them yet — it draws size, theme, ticks, title, and axis labels. Static
export (`save()`, `render_png()`, `render_svg()`) renders every style setting.

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

The widget frontend is bundled from `packages/ruviz/src/python-widget.ts` and
the web SDK. Rebuild it from the repository root after frontend changes:

```sh
bun run build:python-widget
```

The release workflow rebuilds the canonical bundled widget before packaging.
