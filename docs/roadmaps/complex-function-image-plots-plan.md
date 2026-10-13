# Complex-function and image plots

Issue: [#191](https://github.com/Ameyanagi/ruviz/issues/191)

## Design references

- [Matplotlib imshow](https://matplotlib.org/stable/api/_as_gen/matplotlib.pyplot.imshow.html): scalar values and explicit RGB/RGBA colors, image-edge extents, row origin, interpolation, straight alpha, and data aspect.
- [Makie heatmap](https://docs.makie.org/stable/reference/plots/heatmap) and [image](https://docs.makie.org/stable/reference/plots/image): distinct scalar-grid and regular-image interfaces, with eager function sampling as a convenience.
- [Seaborn heatmap](https://seaborn.pydata.org/generated/seaborn.heatmap.html): scalar normalization, masking, colorbars, annotations, and optional equal aspect remain heatmap concerns.

## Implementation plan

1. Add `Plot::image(&Image)` and `image_with` with a fluent `ImageConfig` builder. Reuse the existing straight-alpha RGBA `Image`; validate nonzero dimensions, checked byte counts, and finite increasing image-edge extents. Keep scalar normalization and colorbars on `heatmap`.
2. Define image origin independently of displayed axis direction. Use upper origin by default, matching the existing heatmap convention; allow lower origin for mathematical fields. Support nearest-neighbor and bilinear sampling, applying bilinear interpolation in premultiplied-alpha space to avoid transparent-edge color fringes.
3. Integrate images with bounds, clipping, reversed/nonlinear axes, insertion-ordered overlays, prepared rendering, and PNG/SVG/PDF output. Reuse one projection/sampling implementation across raster and SVG output. Make embedded PNG support available to 2D images without requiring the `3d` feature.
4. Add eager function sampling helpers with explicit source dimensions, extent, and cell-center sampling. Sample once when building data, independent of output DPI or interactive zoom. Callers retain control over complex arithmetic, magnitude/phase mapping, poles, and missing samples.
5. Add an explicit plot-level data-aspect setting. Fit the axes box to the requested ratio without changing data limits; preserve consistent geometry in raster, vector export, subplots, and interaction. Existing plots keep automatic aspect unless configured.
6. Add a runnable Rust example showing the third-order Butterworth magnitude in grayscale/log scale and a caller-colored phase/magnitude image, with a unit-circle overlay. Commit generated PNG previews and link the example from documentation.
7. Validate invalid inputs; origin and axis reversal; physical extents and clipping; nonlinear coordinates; transparency and interpolation; overlay order; sampling conventions; equal aspect; DPI; default/no-default feature builds; and SVG/PDF image embedding. Run formatting and relevant existing regression tests, visually inspect generated previews, and review the final diff.
8. Commit on `feat/complex-function-image-plots`, push the branch, and open a PR against `main` with the plan, validation, runnable example, and embedded generated image previews.

## Scope

This change targets the Rust core API and its export paths. Dedicated complex-number plot types, adaptive resampling, and Python/JavaScript convenience bindings are future work. Regular images retain explicit source resolution; increased export DPI changes output sampling, not the number of function evaluations. Arbitrary RGBA images have no automatic scalar colorbar.

## Acceptance criteria

- A caller can populate an RGBA buffer pixel by pixel and place it within normal axes with line/scatter overlays.
- The reported grayscale magnitude plot can be generated with the scalar heatmap path.
- Colors and placement agree between PNG and SVG/PDF output, including transparency and reversed axes.
- Unit circles remain circular when equal data aspect is selected.
- The PR contains source code and reproducible, visually checked example images.
