# Migrating from Matplotlib 3D

Enable Ruviz's exact `3d` feature:

```toml
ruviz = { version = "0.9.0", features = ["3d"] }
```

The closest API mappings are:

| Matplotlib `Axes3D` | Ruviz |
| --- | --- |
| `ax.scatter(x, y, z)` | `scatter3d(&x, &y, &z)` |
| `ax.plot(x, y, z)` | `line3d(&x, &y, &z)` |
| `ax.plot_surface(X, Y, Z)` | `surface(&x, &y, &z)` |
| `ax.plot_wireframe(X, Y, Z)` | `wireframe(&x, &y, &z)` |
| `ax.view_init(elev, azim, roll)` | `.elevation_deg(elev).azimuth_deg(azim).roll_deg(roll)` |
| `ax.set_*label(...)` | `.xlabel(...).ylabel(...).zlabel(...)` |
| `ax.set_*lim(...)` | `.xlim(...).ylim(...).zlim(...)` |
| `plt.savefig(path)` | `.save(path)` |

## Scatter and line

Matplotlib:

```python
fig = plt.figure()
ax = fig.add_subplot(projection="3d")
ax.scatter(x, y, z, s=36, color="tab:blue")
ax.set(xlabel="x", ylabel="y", zlabel="z")
plt.savefig("scatter.png")
```

Ruviz:

```rust,ignore,reason=abridged-migration-snippet
use ruviz::prelude::*;

scatter3d(&x, &y, &z)
    .marker_size(6.0)
    .color(Color::BLUE)
    .xlabel("x")
    .ylabel("y")
    .zlabel("z")
    .save("scatter.png")?;
```

Ruviz marker size is a diameter in typographic points. Matplotlib's `s`
argument is an area in points squared, so use approximately the square root of
an existing Matplotlib `s` value as the initial Ruviz size. Ruviz uses a real
depth buffer for 3D occlusion and does not expose Matplotlib's `depthshade`
switch.

For lines, replace `ax.plot(x, y, z, ...)` with
`line3d(&x, &y, &z).line_width(...).line_style(...)`. A `NaN` coordinate
creates a line break.

## Surface and wireframe grids

Matplotlib commonly receives three two-dimensional arrays made by `meshgrid`.
Ruviz takes the unique one-dimensional `x` and `y` coordinate vectors plus a
height matrix:

```text
z.shape() == (y.len(), x.len())
z[y_index][x_index]
```

```rust,ignore,reason=abridged-migration-snippet
surface(&x, &y, &z)
    .cmap(ColorMap::viridis())
    .sampling(SurfaceSampling::MaxGrid {
        rows: 50,
        columns: 50,
    })
    .save("surface.png")?;
```

This representation avoids duplicating `x` and `y` across the grid. Matplotlib
uses `rcount` and `ccount` to cap surface samples (both default to 50 in the
current `plot_surface` API). Ruviz provides `SurfaceSampling::Auto`, `Full`,
and `MaxGrid { rows, columns }`. `Auto` is the default, but it and `Full`
currently preserve the full grid. Use `MaxGrid` to bound geometry explicitly.
Wireframes use the same shape and sampling model.

## Camera

Matplotlib and Ruviz both name view angles in degrees. Ruviz defaults to
azimuth `-60°`, elevation `30°`, roll `0°`, and orthographic projection:

```rust,ignore,reason=abridged-migration-snippet
surface(&x, &y, &z)
    .azimuth_deg(-60.0)
    .elevation_deg(30.0)
    .perspective_deg(45.0)
    .save("view.png")?;
```

Use `.orthographic()` when parallel projection is desired. `.look_at(x, y, z)`
sets the camera target. Unlike Matplotlib, Ruviz does not infer whether an
unlabeled angle is radians or degrees: the `_deg` suffix is explicit.

## Construction and output

Matplotlib separates a mutable `Figure`, `Axes3D`, artists, and
`savefig`/`show`. Ruviz starts with the first series and returns an owned
builder. Chain `.scatter3d`, `.line3d`, `.surface`, or `.wireframe` to add
series, then consume or borrow the builder through its render/output method.

Static CPU PNG and SVG output require only `3d`. GPU rendering additionally
requires `gpu`, and the interactive viewer requires `interactive-gpu`.

Ruviz's initial 3D scope intentionally omits Matplotlib features such as
contours projected onto 3D axes, bar charts, quivers, voxels, triangulated
surfaces, arbitrary 3D collections, and fine-grained pane/tick styling.

## Official Matplotlib references

- [`mplot3d` toolkit overview](https://matplotlib.org/stable/users/explain/toolkits/mplot3d.html)
- [`Axes3D.scatter`](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.scatter.html)
- [`Axes3D.plot`](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.plot.html)
- [`Axes3D.plot_surface`](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.plot_surface.html)
- [`Axes3D.plot_wireframe`](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.plot_wireframe.html)
- [`Axes3D.view_init`](https://matplotlib.org/stable/api/_as_gen/mpl_toolkits.mplot3d.axes3d.Axes3D.view_init.html)
