# Images and complex-function plots

Use `heatmap_fn` for a function that returns scalar values, `image_fn` for a
function that returns colors, or `image` for an existing RGBA buffer. Each method
returns a normal series builder, so labels, axes, and overlays work as usual.

## Plot a scalar function

Choose the source resolution and the region to sample. In this example, `320`
columns and `320` rows cover the outer edges `(xmin, xmax, ymin, ymax)`:

```rust,check
use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    Plot::new()
        .heatmap_fn(320, 320, (-2.0, 2.0, -2.0, 2.0), |x, y| {
            (-x * x - y * y).exp()
        })
        .aspect_equal()
        .save("field.png")?;
    Ok(())
}
```

`.aspect_equal()` gives x and y units equal displayed lengths. Add `.cmap("gray")`
to choose a colormap, or `.colorbar_label("value")` to label the scalar colorbar.
For a matrix you have already calculated, use `heatmap(&rows)` instead.

## Plot an RGBA image

`Image` stores row-major RGBA bytes: four bytes per pixel, with each row running
from left to right. For example, these four opaque pixels form a 2 × 2 image:

```rust,check
use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    let pixels = Image::new(2, 2, vec![
        255, 0, 0, 255,   0, 255, 0, 255,
        0, 0, 255, 255,   255, 255, 0, 255,
    ]);

    Plot::new()
        .image(&pixels)
        .aspect_equal()
        .save("image.png")?;
    Ok(())
}
```

The default extent is `(0, width, 0, height)`, measured at the outer pixel edges.
The default interpolation is nearest, which preserves individual pixel colors.
Images keep the colors you supply; scalar colormaps and colorbars belong to
heatmaps.

The row-origin defaults follow the kind of input:

| Input | Default row origin | Source row zero touches |
| --- | --- | --- |
| `image(&pixels)` / `image_with(...)` | Upper | `ymax` |
| `heatmap_fn(...)` / `image_fn(...)` | Lower | `ymin` |

Origin describes physical placement. Reversing the displayed axis with
`invert_y()` or descending `ylim` limits reverses the view while preserving the
source's row identity.

## Calculate colors directly

Use `image_fn` when the function determines each sample's color. It takes the
same arguments as `heatmap_fn`: column count, row count, outer-edge extent
`(xmin, xmax, ymin, ymax)`, and a function of `(x, y)`.

```rust,check
use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    Plot::new()
        .image_fn(200, 160, (-2.0, 2.0, -1.6, 1.6), |x, y| {
            let red = ((x + 2.0) / 4.0 * 255.0) as u8;
            let blue = ((y + 1.6) / 3.2 * 255.0) as u8;
            Color::from_rgba(red, 80, blue, 255)
        })
        .aspect_equal()
        .save("colors.png")?;
    Ok(())
}
```

The callback can capture mutable state. Callers choose the complex arithmetic,
phase colors, magnitude clipping, and pole handling. Return `NaN` for a missing
scalar heatmap sample or alpha zero for a missing RGBA image sample.

## Configure placement and appearance

Add settings to the image builder as needed:

- `.extent(xmin, xmax, ymin, ymax)` places the outer pixel edges in data
  coordinates. Extents stay increasing even when displayed axes are reversed.
- `.origin(ImageOrigin::Lower)` places source row zero beside `ymin`.
- `.interpolation(ImageInterpolation::Bilinear)` smooths between pixel centers.
  Interpolation uses premultiplied alpha to avoid transparent-edge color fringes.
- `.alpha(0.9)` multiplies source pixel opacity. RGBA buffers use straight alpha:
  a half-opaque red pixel is `[255, 0, 0, 128]`.

Images are clipped to the axes. Chain `.line(...)` or `.scatter(...)` after the
image to draw overlays; later series draw over earlier ones.

For reusable placement settings, pass an `ImageConfig` to `image_with`:

```rust,check
use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    let pixels = Image::new(1, 1, vec![40, 120, 220, 255]);
    let config = ImageConfig::new()
        .extent(-2.0, 2.0, -1.0, 1.0)
        .origin(ImageOrigin::Lower);

    Plot::new().image_with(&pixels, config).save("placed_image.png")?;
    Ok(())
}
```

Image dimensions must be nonzero, the buffer must contain exactly
`width × height × 4` bytes, and extent edges must be finite and strictly
increasing with finite spans. Builder validation errors are returned by terminal
operations such as `render`, `save`, and `render_to_svg`.

## Sampling and output resolution

Both function helpers sample once, at source-cell centers. For column `i` and
row `j`, the callback receives:

```text
x = xmin + (i + 0.5) × (xmax − xmin) / width
y = ymin + (j + 0.5) × (ymax − ymin) / height
```

Sampling runs eagerly in row-major order, starting at the lower physical edge.
A valid grid invokes the callback exactly `width × height` times during the
helper call. Rendering at another DPI, exporting again, or changing the viewport
uses the stored samples. To evaluate the function more finely, build another
series with a higher source resolution.

PNG output samples the source into the output axes rectangle. SVG embeds that
image as a PNG while retaining vector axes and overlays, and is available without
the `3d` feature. PDF uses the same image path with the `pdf` feature enabled.

## Adjust data proportions

`.aspect_equal()` keeps a unit circle circular on linear axes, including when a
heatmap reserves room for a colorbar. It fits the axes rectangle inside the
available layout while preserving the resolved data limits. Without an explicit
aspect, axes fill the available space automatically.

For another ratio, use `.data_aspect(ratio)`. A ratio of `2.0` gives a y unit twice
the displayed length of an x unit; `1.0` is equivalent to `.aspect_equal()`.
For nonlinear axes, units are measured after transformation: one decade for
logarithmic axes, or the symmetric-log coordinate. Invalid ratios and geometry
too narrow to represent produce a rendering error.

## Reproduce the Butterworth example

Run this command from the repository root:

```sh
cargo run --example complex_function
```

The [example source](../../examples/complex_function.rs) evaluates the
third-order Butterworth function `H(s) = 1 / (1 + 2s + 2s² + s³)` on a
320 × 320 grid. The grayscale view clips `|H(s)|` to `[0.01, 20]` and uses a
logarithmic heatmap color scale. Its scalar colorbar retains the numerical meaning
of the shading. The dashed unit circle and the three pole crosses use ordinary
line and scatter series.

![Butterworth magnitude with logarithmic grayscale and pole overlays](../assets/gallery/rust/advanced/complex_function_magnitude.png)

The second view computes explicit RGBA colors: hue represents phase, with red at
phase zero, and brightness increases with clipped logarithmic magnitude. Because
the colors encode two quantities, the image does not add a scalar colorbar.

![Butterworth phase and magnitude with unit circle and pole overlays](../assets/gallery/rust/advanced/complex_function_domain.png)

The example saves both images under `generated/examples/complex_function/`.
`make rust-gallery` publishes these outputs to the committed gallery above.
