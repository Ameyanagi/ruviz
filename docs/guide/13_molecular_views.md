# Molecular spheres

Enable `3d` and use `spheres3d(&atoms)` for atoms with physical radii. Each atom
has a position, radius, color (including alpha), and stable `u32` identity.
There is no new dependency or feature flag; GPU rendering uses `gpu` as before.

```rust,check,features=3d
use ruviz::prelude::*;

fn main() -> PlotResult<()> {
    let atoms = [
        Sphere3D::new(10, Point3D::new(0.0, 0.0, 0.0), 0.7, Color::BLUE),
        Sphere3D::new(20, Point3D::new(2.0, 0.0, 0.0), 0.4, Color::RED),
    ];
    spheres3d(&atoms)
        .line3d(&[0.0, 2.0], &[0.0, 0.0], &[0.0, 0.0])
        .color(Color::BLACK)
        .line_width(3.0)
        .save("molecule.png")
}
```

`Sphere3D::radius` uses the units of its center: Å in the example workflow.
`scatter3d(...).marker_size(...)` continues to mean a diameter in typographic
points. Ordinary scatter markers retain their default appearance and renderer.

The sphere factory defaults to `AxisAspect3D::Data`, which preserves length per
data unit, and `.stable_scale(true)`, which prevents orbit from automatically
refitting or resizing the scene. Explicit zoom and resize still work. `AxisAspect3D::Equal` means equal *axis lengths*, even if their data
ranges differ. When adding spheres to a plot that started with `line3d` or
`surface`, add `.axis_aspect(AxisAspect3D::Data).stable_scale(true)` for the same behavior.
Explicit axis aspect settings apply to all geometry, including spheres.
For a custom X:Y:Z box ratio, use
`.axis_aspect(AxisAspect3D::fixed(1.0, 2.0, 1.0))`; this can intentionally change
physical proportions. See [fixed proportions](12_3d.md#fixed-proportions-during-rotation).
Automatic bounds include the full radius of every sphere, including faded ones.

## Lighting and geometry

Spheres default to ambient/diffuse illumination plus a small highlight.
`.specular(0.0, 32.0)` disables the highlight; strength accepts `0..=1` and the
gloss exponent accepts `1..=256`. Defaults are strength `0.15` and gloss `32`.
`.shading(false)` disables lighting while retaining the exact sphere shape and
surface depth. It does not replace the spheres with screen-space scatter markers.

The light is camera-relative: upper left and toward the viewer, along normalized
`(-0.35, 0.45, 0.82)` in view coordinates (right, up, toward viewer). Ambient
intensity is `0.3`, diffuse weight `0.7`. The small Blinn highlight uses the view
axis. Colors are decoded from sRGB, shaded and blended in linear RGB, then encoded
to sRGB. Orbiting changes the visible surface while keeping the light attached
to the camera. There are no cast shadows, ambient occlusion, or PBR materials.

Both renderers intersect the view ray with each sphere analytically. They use
the intersection for depth, silhouette, and normal, including perspective and
nonuniform axis aspect. GPU spheres use instanced quads and fragment depth;
[sample interpolation](https://www.w3.org/TR/WGSL/#interpolation) antialiases their
analytic edges when multisampling is available. Opaque spheres share the depth
buffer with lines and surfaces. No atoms are dropped during drag.

## Picking and interaction

`PickPrimitive3D::Sphere` identifies a sphere surface hit. `hit.sources()[0]`
returns the application's atom ID; `primitive_index` is its current position in
the series. IDs must be unique within a series. Use `(series_index, atom_id)`
when multiple series reuse IDs. Reordering the input does not change atom IDs.
`hit.point` is the intersection on the visible sphere surface, in data units.

`InteractivePlot3DSession::set_sphere_shading(bool)` toggles all sphere series
without rebuilding geometry or resetting camera, zoom, selection, or active drag.
It supersedes outstanding frames and refreshes the retained pick's stamp.
Previously held stamped picks should be refreshed from `current_pick()`.
Use `replace_keep_camera` for geometry updates and structure changes. Store an
atom ID in host state if selection must survive a complete scene replacement.

The GPUI view exposes the same operation:

```rust,ignore,reason=requires-a-gpui-view-context
let view = ruviz_gpui::plot3d_builder(ruviz::spheres3d(&atoms))
    .interactive()
    .on_pick(|hit| println!("atom IDs: {:?}", hit.sources()))
    .build(cx);

view.update(cx, |view, cx| view.set_sphere_shading(false, cx))?;
```

## Transparency and overlays

Use `atom.color.with_alpha(0.15)` for faded context. Fully transparent spheres
are skipped. Spheres with byte alpha `0..=12` (below approximately 5%) do not
intercept picking. Among the remaining spheres, the nearest surface wins,
including translucent spheres. Picking does not try to infer the strongest
contributor to a blended pixel.

Opaque geometry renders first. Faded spheres render back to front by center
depth, test against opaque depth, and do not write depth. Each sphere contributes
its nearest visible shell once; alpha is a surface opacity, not volume absorption.
This supports separated context atoms. Intersecting translucent spheres can
have sorting artifacts: this is not order-independent transparency. GPU rendering
currently uses one draw per translucent sphere, so extensive faded context costs
more than an opaque instanced batch.

**Translucent surface/polyhedron faces, lines, and ordinary scatter markers remain
unsupported.** Opaque surfaces and bonds occlude spheres correctly. Existing
line widths are screen-space strokes, not cylindrical molecular bonds. Arbitrary
polyhedron triangle input and cylinder primitives remain separate work.

Use `.axes(false)` for a clear molecular view with a small x/y/z orientation
cue. This hides the panes, grid, box, ticks, and axis labels while keeping the
title and legend. Axes remain enabled by default for ordinary plots.

All line-based path arrows and arrowheads use ordinary depth testing. For
deliberately always-on-top arrows or numbered labels, draw a host GPUI overlay.
The orientation cue and legend remain foreground plot decorations. This
milestone adds no arbitrary 3D annotation or overlay API.

## Software, export, and examples

`render()` and `save()` use the analytic software renderer; `render_gpu()` and
`save_gpu()` use the GPU. PNG output contains the rendered spheres. SVG and PDF
embed the depth-tested raster scene with vector axis text, as other 3D plots do.
Exports preserve lighting and transparency; there is no gradient-disk fallback.
Interactive CPU rendering uses one sample per pixel and export uses four;
GPU multisampling depends on adapter support. Small edge differences are expected.

GPUI's `3d` feature uses the background software renderer. `3d-gpu` uses retained
GPU rendering with image readback and GPUI upload. It is not zero-copy. The Rust
builder can also enter other adapters through `TryIntoPlot3DSession`; dedicated
Python and browser sphere APIs are not part of this change.

```sh
cargo run --release --example molecular_spheres --features 3d,gpu
cargo run --release --manifest-path adapters/gpui/Cargo.toml --example molecular_spheres --features 3d-gpu
```

The GPUI example contains mixed radii/colors, an absorber, bonds, an 8 Å cutoff,
faded context, and outward/return paths. The export example reconstructs Ru hcp
and RuO₂ from COD lattice data; the 8 Å clusters contain 147 and 209 atoms.
See the [benchmark report](../benchmarks/spheres-2026-09-05.md) for measured costs.
