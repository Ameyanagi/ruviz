# Migrating from Makie 3D

Enable Ruviz's exact `3d` feature:

```toml
ruviz = { version = "0.9.0", features = ["3d"] }
```

The closest API mappings are:

| Makie | Ruviz |
| --- | --- |
| `scatter(x, y, z)` / `scatter!(ax, ...)` | `scatter3d(&x, &y, &z)` |
| `lines(x, y, z)` / `lines!(ax, ...)` | `line3d(&x, &y, &z)` |
| `surface(x, y, z)` | `surface(&x, &y, &z)` |
| `wireframe(x, y, z)` | `wireframe(&x, &y, &z)` |
| `Axis3(fig[pos], ...)` | created by the first Ruviz 3D builder |
| `ax.azimuth[]`, `ax.elevation[]` | `.azimuth_deg(...)`, `.elevation_deg(...)` |
| `save(path, fig)` | `.save(path)` |

## Builders instead of scenes and bang functions

Makie can create a new figure/axis implicitly or mutate an existing `Axis3`
with bang functions. Ruviz starts with the first series and returns an owned
builder:

```rust,ignore,reason=abridged-migration-snippet
use ruviz::prelude::*;

scatter3d(&x, &y, &z)
    .label("samples")
    .line3d(&model_x, &model_y, &model_z)
    .label("model")
    .xlabel("x")
    .ylabel("y")
    .zlabel("z")
    .save("combined.png")?;
```

The continuation methods share one camera, set of axes, and depth buffer.

## Surface orientation

Makie accepts `surface(x, y, z)` with one-dimensional coordinates. Ruviz uses
the same compact idea but makes the memory orientation explicit:

```text
z.shape() == (y.len(), x.len())
z[y_index][x_index]
```

```rust,ignore,reason=abridged-migration-snippet
surface(&x, &y, &z)
    .cmap(ColorMap::viridis())
    .sampling(SurfaceSampling::MaxGrid {
        rows: 100,
        columns: 100,
    })
    .save("surface.png")?;
```

`SurfaceSampling::Auto` is the default and currently preserves the full grid,
as does `Full`. Choose `MaxGrid { rows, columns }` for an explicit geometry
cap. Wireframes use the same input shape and sampling choices.

## Camera units and projection

Makie's `Axis3` azimuth and elevation values are angles commonly expressed
with Julia's `pi` notation. Ruviz camera setters are explicitly in degrees:

```rust,ignore,reason=abridged-migration-snippet
surface(&x, &y, &z)
    .azimuth_deg(-45.0)
    .elevation_deg(25.0)
    .perspective_deg(45.0)
    .look_at(0.0, 0.0, 0.0)
    .save("view.png")?;
```

Ruviz defaults to azimuth `-60°`, elevation `30°`, roll `0°`, orthographic
projection, automatic `4:4:3` axis aspect, and zoom `1`. Call
`.orthographic()` to switch back from perspective projection.

## Styling and rendering

Ruviz maps the most common Makie attributes to typed builder methods:
`.color`, `.marker`, `.marker_size`, `.line_width`, `.line_style`,
`.colormap`, `.shading`, and `.sampling`. It deliberately exposes a smaller
initial surface than Makie's recipe, observable, lighting, material,
transparency, and scene systems.

Static PNG and SVG output use Ruviz's CPU renderer with only `3d` enabled. Add
`gpu` for `.render_gpu()`/`.save_gpu()`, or `interactive-gpu` for `.show()`.
The CPU and GPU paths use the same prepared geometry and camera contract.

Ruviz's initial 3D scope does not include Makie volume plots, meshes,
tri-surfaces, contours, arrows, text in data space, arbitrary scene graphs,
multiple linked `Axis3` blocks, or custom shader/material recipes.

## Official Makie references

- [`scatter`](https://docs.makie.org/stable/reference/plots/scatter.html)
- [`lines`](https://docs.makie.org/stable/reference/plots/lines.html)
- [`surface`](https://docs.makie.org/stable/reference/plots/surface.html)
- [`wireframe`](https://docs.makie.org/stable/reference/plots/wireframe.html)
- [`Axis3`](https://docs.makie.org/stable/reference/blocks/axis3.html)
