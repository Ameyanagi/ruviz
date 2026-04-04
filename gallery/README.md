# Gallery Sources

The `gallery/` tree contains source programs and utilities that support the Rust
gallery, benchmark visuals, and rendering experiments.

## What Lives Here

- `gallery/basic/` for simple plot demonstrations
- `gallery/advanced/` for typography and styling-heavy experiments
- `gallery/publication/` for publication-oriented layouts
- `gallery/scientific/` for research-style examples
- `gallery/utility/` for helper programs used during visual/debug work

## Output Policy

These gallery source programs should write transient artifacts under
`generated/bench/` or another documented `generated/` subdirectory.

Committed published gallery media is not stored here. The published Rust gallery
assets live under `docs/assets/gallery/rust/`, and the markdown index pages live
under `docs/gallery/`.

## Regeneration

For the release-facing Rust gallery, use:

```sh
cargo run --bin generate_gallery
```

For the full release docs/media refresh, use:

```sh
make release-docs
```

See [docs/BUILD_OUTPUTS.md](../docs/BUILD_OUTPUTS.md) for the full artifact
layout.
