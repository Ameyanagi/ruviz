# ruviz

Rust-powered plotting for Python. `ruviz` wraps the [ruviz](https://github.com/Ameyanagi/ruviz)
rendering engine in a fluent, fully typed Python API: build a plot by chaining
method calls, then export it to PNG, SVG, or PDF, display it in a Jupyter cell,
drive it live from an observable, or open it in a native desktop window. NumPy
arrays cross into Rust with a single `memcpy` and rendering runs with the GIL
released, so million-point series stay interactive.

## Install

```sh
pip install ruviz
```

The base install pulls in NumPy only. Everything else is an extra:

| Extra | Install | Adds |
| --- | --- | --- |
| — | `pip install ruviz` | Static PNG/SVG/PDF export, native `show()`, static notebook display |
| `widget` | `pip install "ruviz[widget]"` | `anywidget` + `traitlets` for `plot.widget()` and `RuvizWidget` |
| `pandas` | `pip install "ruviz[pandas]"` | pandas `DataFrame` columns through `data=` |
| `polars` | `pip install "ruviz[polars]"` | Polars `DataFrame` columns through `data=` |
| `dataframes` | `pip install "ruviz[dataframes]"` | Both dataframe backends |
| `all` | `pip install "ruviz[all]"` | Every extra |

`import ruviz` works without any extra; `plot.widget()` and `ruviz.RuvizWidget`
raise an `ImportError` that names `ruviz[widget]` when the extra is missing.

## Quick Start

```python,check
import numpy as np
import ruviz

x = np.linspace(0.5, 12.0, 60)
fast = 8.0 * np.exp(-x * 0.62)
slow = 6.0 * np.exp(-x * 0.22)

(
    ruviz.plot()
    .size_px(760, 420)
    .title("Decay Rates")
    .xlabel("time")
    .ylabel("intensity")
    .line(x, fast, label="fast decay", color="#2563eb", width=2.0)
    .line(x, slow, label="slow decay", color="orange", linestyle="dashed")
    .yscale("log")
    .grid(True)
    .legend("upper_right")
    .save("decay.png")
)
```

## Features

- **15 plot types** — line, scatter, bar, histogram, boxplot, violin, kde, ecdf,
  error bars (y and xy), heatmap, contour, pie, radar, polar line.
- **Per-series styling** — labels, colors, alpha, widths, line styles, markers,
  plus kind-specific `bins`, `density`, `bandwidth`, and `levels`.
- **Axis control** — `legend()`, `grid()`, `xlim`/`ylim`, and linear, log, or
  symlog scales.
- **Static export** — `save()` writes PNG, SVG, or PDF; `render_png()` returns
  bytes and `render_svg()` returns a string.
- **Jupyter** — plots display as a static PNG by default; `plot.widget()` gives
  you the synced, zoomable WASM widget with the `ruviz[widget]` extra.
- **Live data** — `ruviz.observable(...)` series support elementwise arithmetic
  and NumPy ufuncs, and push updates into attached widgets.
- **DataFrames** — pandas, Polars, plain dicts, and anything else indexable by
  column name through `data=`.
- **Experimental 3D alpha** — deterministic static export for `scatter3d`,
  `line3d`, `surface`, and `wireframe`.
- **Typed** — inline annotations with a `py.typed` marker, so a type checker
  rejects a bad `marker=`, `linestyle=`, legend position, or axis scale before
  the call reaches the renderer.
- **Fast** — adding a 1,000,000-point line series takes about 1 ms (it was
  141 ms before the arrays were passed as a single `memcpy`), and rendering,
  saving, and native display all release the GIL.

## Styling

Series style arguments are keyword-only, and each kind accepts exactly what the
renderer honors for it:

| method | keywords |
| --- | --- |
| `line` | `label`, `color`, `alpha`, `width`, `linestyle`, `marker`, `marker_size` |
| `scatter` | `label`, `color`, `alpha`, `marker`, `marker_size` |
| `bar` | `label`, `color`, `alpha` |
| `histogram` | `label`, `color`, `alpha`, `bins`, `density` |
| `boxplot` | `label`, `color`, `alpha`, `width`, `linestyle` |
| `kde` | `label`, `color`, `alpha`, `width`, `bandwidth` |
| `ecdf`, `violin`, `polar_line`, `error_bars`, `error_bars_xy` | `label`, `color`, `alpha`, `width` |
| `contour` | `alpha`, `width`, `levels` |

`heatmap`, `pie`, and `radar` take no style keywords. `radar` series that carry
a `name` are labelled once you add `.legend(...)` to the plot.

- `histogram(density=True)` normalizes the bars to a probability density, which
  is what a `kde()` overlay is drawn on; without it the KDE sits flat at zero
  against a counts axis.
- `color` takes a hex string (`"#2563eb"`, `"#25f"`, `"#2563eb80"`) or a named
  color such as `"red"`, `"orange"`, `"teal"`, or `"crimson"`; a typo raises
  `ValueError` with a "did you mean" suggestion.
- `linestyle` is one of `solid`, `dashed`, `dotted`, `dash-dot`, `dash-dot-dot`.
- `marker` is one of `circle`, `square`, `triangle`, `triangle-down`, `diamond`,
  `plus`, `cross`, `star`, `circle-open`, `square-open`, `triangle-open`,
  `diamond-open`.
- The matplotlib shorthands are accepted as aliases — `"o"`, `"s"`, `"^"`,
  `"v"`, `"D"`, `"+"`, `"x"`, `"*"` for `marker` and `"-"`, `"--"`, `":"`,
  `"-."` for `linestyle` — and snapshots store the canonical name.
- Unsupported names raise `ValueError` listing the accepted values at the call
  that used them, not at render time.

Plot-level settings are `legend(position="best")` — `"best"` plus lowercase
position names such as `"upper_right"`, `"center"`, or `"outside_right"` —
`grid(enabled=True)`, `dpi(dpi)`, `xlim(min, max)`, `ylim(min, max)`, and
`xscale(scale, linthresh=None)` / `yscale(...)` with `"linear"`, `"log"`, or
`"symlog"`. `dpi` scales the exported pixels from `size_px(...)`, so
`size_px(640, 480).dpi(200)` writes a 1280×960 image. Axis limits must be finite
and different; passing them inverted, as in `xlim(10, 0)`, renders a descending
axis (`Plot3D` limits stay strictly ascending).

Notebook widgets render these settings too: the WASM runtime applies series
styles, `dpi`, `legend`, `grid`, axis limits, and axis scales from the
snapshot.

## Notebook and Desktop Usage

- In Jupyter, a bare plot result and `plot.show()` both display a static PNG.
- Use `plot.widget()` when you want the synced WASM-backed notebook widget.
- `plot.size_px(width, height)` also controls the widget's displayed size and aspect ratio.
- Without `size_px(...)`, the widget uses the default PNG size (`640x480`) and shrinks proportionally if the notebook column is narrower.
- Drag the widget's bottom-right handle to resize the display freely; hold `Shift` or `Ctrl` while dragging to preserve the aspect ratio.
- In the widget, the mouse wheel zooms, left drag pans, right drag box-zooms, and right click opens the export menu.
- Outside notebooks, `plot.show()` opens the native interactive window.
- The published Linux wheel focuses on static rendering and notebook widgets. Install from source on Linux if you need the native desktop `plot.show()` window.
- `plot.render_png()` returns PNG bytes and `plot.render_svg()` returns an SVG string.
- `plot.save(path)` writes PNG, SVG, or PDF according to the file extension and returns the output `Path`; any other extension, or a path without one, raises `ValueError`.

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
`error_bars`, and `error_bars_xy`; other plot types reject them with a
`TypeError` and expect static values.

```python,check
import numpy as np

scaled = np.sin(y * 2.0 + 0.25)
plot.line(x, scaled)
y.replace(np.cos(x))
```

`replace()` is atomic: when the new length would break a bound series — directly
or through a derived observable — it raises `ValueError` before anything
mutates. Derived observables resize along with their source, so a plot of `x`
against `np.sin(x)` stays consistent when `x` grows, and writing to an
observable with `replace()` or `set_at()` permanently detaches it from its own
sources. `len(series)` and `series[i]` read the current values.
`deepcopy(plot)` creates an independent live copy with fresh observables, while
`plot.clone()` remains a static snapshot copy.

## Experimental 3D Alpha

The Python wheel includes the Rust crate's opt-in Cargo feature named exactly
`3d`. The initial Python surface provides deterministic CPU export for opaque
`scatter3d`, `line3d`, regular-grid `surface`, and `wireframe` plots:

```python,check
import numpy as np
import ruviz

x = np.linspace(-2.0, 2.0, 32)
y = np.linspace(-2.0, 2.0, 24)
grid_x, grid_y = np.meshgrid(x, y)
z = np.sin(grid_x**2 + grid_y**2)

(
    ruviz.surface(x, y, z)
    .size_px(720, 480)
    .title("3D surface alpha")
    .xlabel("x")
    .ylabel("y")
    .zlabel("z")
    .save("surface.png")
)
```

For surfaces and wireframes, rows of `z` correspond to `y` and columns
correspond to `x`, so `z.shape == (len(y), len(x))`. Orthographic projection is
the default; `.perspective_deg(45.0)` opts into perspective. This alpha is
static-only in Python: interactive orbit widgets, transparency, volume plots,
arbitrary meshes, and mixed 2D/3D axes are not yet exposed.

## Supported Python Versions and Platforms

- Python 3.10 or newer. Wheels are built as a single `abi3` artifact per
  platform and are tested against 3.10 and 3.13.
- Wheels: macOS x86\_64 and arm64, Windows x86\_64, Linux x86\_64 and aarch64
  (manylinux 2\_28). A source distribution is published as well.
- The Linux wheels are built without the native interactive backend, so
  `plot.show()` raises there and asks you to install from source.

## Documentation

- [Python docs source](https://github.com/Ameyanagi/ruviz/tree/main/bindings/python/docs)
  — getting started, interactivity, gallery, and the generated API reference
- [Python examples](https://github.com/Ameyanagi/ruviz/tree/main/bindings/python/examples)
  — the runnable scripts the gallery is generated from
- [Project README](https://github.com/Ameyanagi/ruviz/blob/main/README.md) and
  [Rust API docs](https://docs.rs/ruviz)

## Contributor Workflow

```sh
cd bindings/python
uv sync
uv run maturin develop
uv run python scripts/generate_gallery.py
uv run mkdocs serve
```

Rebuild the bundled widget frontend from the repository root when you change
the web SDK or `packages/ruviz/src/python-widget.ts`:

```sh
bun run build:python-widget
```
