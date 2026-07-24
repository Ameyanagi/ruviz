# ruviz 3d preset research

Status: research complete; implementation proposed
Date: 2026-07-24
Branch: `feat/3d-implementation`

## Question

Can ruviz provide a preset-oriented API that makes a professional 3d plot
easy to produce without exposing every Axis3, camera, typography, pane,
lighting, and layout option?

Yes. Mature plotting packages consistently provide reusable defaults, but the
best systems do not treat every kind of default as one undifferentiated
theme. They separate visual themes, camera/view controls, axis fitting, and
per-plot overrides.

## What other packages do

| Package | Reusable mechanism | 3d-specific handling | Relevant lesson for ruviz |
| --- | --- | --- | --- |
| Matplotlib | Named style sheets, `rcParams`, temporary style contexts, and ordered style composition | `Axes3D` separately exposes elevation, azimuth, roll, projection, focal length, box aspect, ticks, and manual axis appearance controls | Keep the existing generic `Theme`, but do not expect it alone to solve Axis3 layout or camera quality |
| Makie | Global/local `Theme`, `set_theme!`, `with_theme`, and mergeable themes | `Axis3` separately exposes `viewmode`, aspect, perspective, protrusions, panes, grids, spines, tick padding, and label offsets | Add a first-class 3d axis style and explicit fit mode; camera fitting is part of professional appearance |
| Plotly | Named templates, custom registered templates, default templates, and template composition | A scene separately owns camera, projection, aspect mode, axes, background, and trace defaults | A named ruviz preset should be a curated bundle, while camera/view remains independently replaceable |
| PyVista | Global and per-plot themes with named `document`, `dark`, and `paraview` choices | Named isometric/orthogonal views, tight camera fitting, padding, lighting, axes, bounds, and scalar-bar configuration | Provide task-oriented names such as `Publication` and explicit views such as `Isometric` and `Top` |

### Matplotlib

Matplotlib style sheets are predefined `rcParams` sets. A user can apply a
named style, a dictionary, a file, or an ordered list of styles; a context
manager applies a style temporarily. `Axes3D` keeps camera and box controls
outside that mechanism: its constructor and methods expose elevation,
azimuth, roll, perspective/orthographic projection, focal length, and box
aspect. Matplotlib also documents that some mplot3d look-and-feel controls
remain lower-level axis information rather than polished style-sheet
contracts.

Implication: ruviz already has the equivalent of Matplotlib style sheets in
`Theme`. Extending `Theme` with every 3d-only parameter would make the 2d
contract harder to understand without solving view fitting cleanly.

Official references:

- <https://matplotlib.org/stable/api/style_api.html>
- <https://matplotlib.org/stable/api/toolkits/mplot3d/axes3d.html>
- <https://matplotlib.org/stable/api/toolkits/mplot3d/view_angles.html>

### Makie

Makie themes can be global, scoped with `with_theme`, updated, and merged.
Predefined themes include dark, light, minimal, black, ggplot-like, and LaTeX
font variants. `Axis3` then supplies a large independent styling and camera
surface: pane colors and visibility, grid and spine appearance, tick padding,
label offsets, front spines, aspect, projection strength, and protrusions.

Its `viewmode` distinction is especially relevant:

- `fit` keeps apparent size stable during rotation but may leave unused space;
- `fitzoom` fills more space but can visually pump while rotating;
- `stretch` fills the frame but does not preserve the requested aspect.

Makie also documents that Axis3 protrusions use a heuristic because camera
rotation makes fully automatic decoration margins difficult. This matches the
label/canvas-utilization problems visible in the current ruviz examples.

Implication: ruviz needs an explicit Axis3 fit policy and decoration metrics,
not only new colors.

Official references:

- <https://docs.makie.org/stable/explanations/theming/themes>
- <https://docs.makie.org/stable/explanations/theming/predefined_themes>
- <https://docs.makie.org/stable/reference/blocks/axis3>

### Plotly

Plotly templates can be named, registered, selected globally, applied to one
figure, and composed. Built-ins include `plotly`, `plotly_white`,
`plotly_dark`, `ggplot2`, `seaborn`, and `simple_white`. Templates can supply
both layout defaults and trace-type defaults.

Plotly's 3d scene keeps camera and aspect explicit. Its aspect mode is one of
`auto`, `cube`, `data`, or `manual`, and camera configuration separately
controls eye, center, up direction, and projection.

Implication: ruviz should make the common professional result one named call,
but it must allow `.view(...)` and `.axis_aspect(...)` to replace the preset's
camera choices without rebuilding the visual style.

Official references:

- <https://plotly.com/python/templates/>
- <https://plotly.com/python/3d-camera-controls/>
- <https://plotly.com/python-api-reference/generated/plotly.graph_objects.layout.html>

### PyVista

PyVista provides named global and per-plot themes. Its `document` theme is
explicitly intended for papers and presentations; it also provides dark and
ParaView-like themes. A custom theme can include font, lighting, axes,
background, mesh defaults, edges, colorbars, and silhouettes.

Camera views are separate: isometric and orthogonal plane views are named
operations. `Camera.tight` fits visible actors to the render window with
padding and a selected plane, using parallel projection.

Implication: task-oriented preset names and named view directions are easier
to remember and generate than raw camera angles.

Official references:

- <https://docs.pyvista.org/api/plotting/theme.html>
- <https://docs.pyvista.org/examples/02-plot/themes>
- <https://docs.pyvista.org/api/core/_autosummary/pyvista.Camera.tight.html>

## Proposed ruviz design

Do not replace the existing `Theme`. Add three small, orthogonal concepts:

1. `Preset3D`: a curated professional bundle.
2. `View3D`: a named camera orientation.
3. `Fit3D`: how the projected box uses the available viewport.

The smallest Rust API should be:

```rust,ignore,reason=proposed-api
surface(&x, &y, &z)
    .preset(Preset3D::Publication)
    .save("surface.png")?;
```

Camera selection remains independently readable:

```rust,ignore,reason=proposed-api
surface(&x, &y, &z)
    .preset(Preset3D::Presentation)
    .view(View3D::Isometric)
    .save("surface.png")?;
```

Proposed public enums:

```rust,ignore,reason=proposed-api
pub enum Preset3D {
    Default,
    Publication,
    Presentation,
    Dark,
    Minimal,
    Technical,
}

pub enum View3D {
    Isometric,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

pub enum Fit3D {
    Stable,
    Tight,
    Stretch,
}
```

Rust should use enums so invalid names do not compile and small code-generation
models have a short vocabulary. Python and TypeScript should expose the exact
lowercase string union:

```text
default | publication | presentation | dark | minimal | technical
```

### Preset responsibilities

A preset may set:

- generic `Theme` colors and typography;
- pane visibility and opacity;
- front/back spine color, opacity, and width;
- major grid visibility and visual weight;
- tick count, tick length, tick-label padding, and axis-label offset;
- title spacing;
- legend and colorbar placement;
- orthographic or mild-perspective projection;
- axis aspect and viewport fit policy;
- light direction and ambient/diffuse balance;
- default canvas utilization target.

A preset must not silently change:

- source data or sampling density;
- backend selection;
- CPU/GPU correctness behavior;
- output format;
- interactive controls;
- explicit limits;
- user-provided camera, aspect, theme, or individual style overrides.

### Override order

Use one deterministic rule:

```text
library defaults < preset < explicit theme/view/fit < individual setters
```

This is easier to document and generate than call-order-dependent merging.
`Preset3D` should be resolved once into the retained frame, so applying a
preset adds no per-frame dynamic dispatch and no camera-frame allocation.

## Proposed built-in behavior

| Preset | Intended result |
| --- | --- |
| `Default` | Neutral light background, stable orthographic view, restrained panes, automatic fit |
| `Publication` | High-contrast print output, orthographic projection, thin rear structure, strong foreground axes, larger label clearance |
| `Presentation` | Larger typography and marks, mild perspective, tighter canvas use, simplified ticks |
| `Dark` | Dark background, accessible foreground contrast, subdued panes and grids |
| `Minimal` | No panes, minimal rear spines/grid, labels and foreground axes retained |
| `Technical` | Data/equal aspect emphasis, full box and grids, more ticks, orthographic projection |

The current `Theme::publication()`, `Theme::dark()`, and `Theme::minimal()`
should be reused inside these bundles, not duplicated.

## Recommended first implementation

1. Add `Axis3Style`, `View3D`, and `Fit3D` internal resolved types.
2. Make the default fit consume more of the canvas while reserving measured
   decoration protrusions.
3. Implement `Publication`, `Presentation`, and `Dark` first.
4. Add `.preset(...)`, `.view(...)`, and `.fit(...)` to all four 3d builders.
5. Mirror preset names in Python and TypeScript.
6. Generate the same surface under every preset through CPU and GPU paths.
7. Add exact fixed-camera goldens and layout assertions.
8. Add `Minimal` and `Technical` after the first three pass visual review.

## Professional-quality acceptance criteria

- No title, tick label, axis label, legend, or colorbar intersects another
  decoration or the projected data box in the canonical golden suite.
- The projected Axis3 box uses 68–82% of the available short canvas dimension
  for standard single-plot figures.
- Rear grids and spines are visibly subordinate to foreground axes.
- All preset text/background pairs meet WCAG AA contrast for normal text.
- CPU, native GPU, and browser WebGPU use the same resolved layout and style.
- Camera-only retained frames allocate no preset objects and upload no
  unchanged style buffers.
- Preset resolution adds less than 1% to retained camera-frame time.
- Explicit builder settings override preset values identically in Rust,
  Python, and TypeScript.

## Recommendation

Implement presets before presenting the current 3d output as publication
quality. The renderer does not need architectural changes. The missing layer
is a compact Axis3 visual-style contract, named camera views, and a better fit
policy. A single `.preset(Preset3D::Publication)` call should produce the
professional default that currently requires many manual adjustments.
