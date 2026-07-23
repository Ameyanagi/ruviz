# API Reference

The generated reference below covers the Python package surface that is shipped
to PyPI:

- `plot()` for creating new plot builders
- `Plot` for fluent plotting, export, widgets, native display, and safe Python copy/deepcopy support
- `plot3d()` and `Plot3D` for the static opaque 3D alpha
- `scatter3d()`, `line3d()`, `surface()`, and `wireframe()` for direct 3D construction
- `observable()` and `ObservableSeries` for synced notebook data, elementwise arithmetic, and NumPy ufuncs
- `RuvizWidget` for explicit notebook widget embedding

The public import surface is:

```python
from ruviz import (
    ObservableSeries,
    Plot,
    Plot3D,
    RuvizWidget,
    line3d,
    observable,
    plot,
    plot3d,
    scatter3d,
    surface,
    wireframe,
)
```

`Plot3D` currently provides deterministic CPU PNG and hybrid SVG/PDF export.
It uses orthographic projection by default and requires regular surface grids
with `z.shape == (len(y), len(x))`. Python interactive orbit widgets and
transparent 3D geometry are intentionally outside the alpha surface.

::: ruviz
