# ruviz logo

A hexagonal crate shell — the Rust ecosystem's convention — with an arrowed axis
pair and a tanh curve knocked out of it, sampled at one warm point.

The curve is a real `tanh`, generated from the function rather than drawn by
hand, and the marker sits at `x = 0.5` on it. Arrowheads were chosen over tick
marks and a plot grid: repeated fine detail dies at icon sizes, while single
gestures survive.

## Which file to use

| Context | File |
|---|---|
| Anywhere it can be vector | `ruviz-logo.svg` |
| README, docs, slides | `ruviz-logo-256.png` (or 512 / 1024) |
| Favicon | `favicon.ico` (multi-resolution) or `favicon.svg` |
| Anywhere rendered at 48px or below | `ruviz-logo-small.svg` / `ruviz-logo-small-*.png` |

**The small cut is not optional at small sizes.** The full mark's axis rule is
14 units wide against a 512-unit viewBox, so below about 32px it falls under one
device pixel and the interior turns to grey mush. `ruviz-logo-small.svg` keeps
the same drawing but thickens every stroke, so the silhouette stays legible.
`favicon.ico` already does this: it carries the small cut at 16, 32 and 48px and
the full mark from 64px up.

Even the small cut softens at 16px. A genuinely crisp 16px icon would have to be
drawn on the pixel grid rather than scaled down from this.

## Palette

| Role | Hex |
|---|---|
| Shell | `#13161B` |
| Plot knockout | `#FBFAF7` |
| Data point | `#C2510F` |
| Plot blue (used in alternates) | `#1F77B4` |

The PNGs have a transparent background, so the shell reads on both light and
dark pages.

## Regenerating

The SVGs are hand-authored — there is no source file behind them, and they
should be edited directly. The PNGs and the `.ico` are generated from them by
`scripts/render-svg.mjs` in the `ruviz_promo` repository, which rasterises
through headless Chrome.
