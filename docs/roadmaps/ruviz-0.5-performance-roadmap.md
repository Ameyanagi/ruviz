# ruviz 0.5 performance-first plotting roadmap

This roadmap records the competitive review that drove the 0.5 branch work and
maps each user-facing requirement to branch artifacts.

## Competitive scan

Sources checked on 2026-05-19:

- Matplotlib plot types: <https://matplotlib.org/stable/plot_types/index.html>
- Makie plot reference: <https://docs.makie.org/stable/reference/plots/>
- Charton announcement and repository: <https://users.rust-lang.org/t/charton-a-versatile-plotting-library-for-rust/137339>, <https://github.com/wangjiawen2013/charton>
- Kuva repository and documentation: <https://github.com/Psy-Fer/kuva>, <https://psy-fer.github.io/kuva/>

Findings:

| Library | Strengths relevant to ruviz | Gap this branch targets |
| --- | --- | --- |
| Matplotlib | Broad scientific vocabulary and stable high-level names. | Make existing low-level ruviz plot types available through nonbreaking `Plot::...` builders. |
| Makie | Large plot catalog, first-class vector fields, 3D/volume plots, and Datashader-style large-data rendering. | Make backend execution truthful and add a high-level vector-field path. |
| Charton | Rust-native declarative/layered API, Polars-oriented workflow, Wasm readiness, and bridge escape hatches. | Keep ruviz fluent APIs concise while preserving explicit Rust error handling and render diagnostics. |
| Kuva | Scientific Rust focus, broad plot catalog, CLI, SVG-first output, and bioinformatics-specific charts. | Prioritize core scientific plots and benchmarkable public PNG/SVG paths before domain-specific catalogs. |

## Implemented targets

| Requirement | Branch artifact |
| --- | --- |
| Truthful backend selection | `BackendType::as_str`, `get_backend_name()` for configured preference, `resolved_backend_name()` for public PNG execution, benchmark `actualBackend` diagnostics. |
| Public render/save performance | Direct PNG encoding from the Skia renderer for `render_png_bytes()` and `save()`, raster line reduction/canonicalization diagnostics, and explicit DataShader diagnostics without changing `auto_optimize()` scatter visuals. |
| Idiomatic readable Rust | Hot-path helpers split into named functions, iterator-based collection for derived series, `total_cmp` used where float sorting is required. |
| Robust error handling | New builder ingestion errors preserve `DataLengthMismatch`; public render checks pending ingestion errors before fallback rendering; new production unwrap/expect/panic guard. |
| Nonbreaking high-level plot APIs | `Plot::step`, `Plot::area`, `Plot::stem`, `Plot::boxen`, and `Plot::quiver` with typed builders and rustdoc examples. |
| Scientific feature additions | Boxen/letter-value plots promoted to the public API; quiver vector fields promoted to the public API and exported through the prelude. |
| Benchmark comparison | `benchmarks/plotting/compare.py`, benchmark runner `--auto-optimize`, public render/save boundaries, and Markdown regression reports. |
| Verification | `tests/nonbreaking_plot_api_test.rs`, focused backend/DataShader tests in `src/core/plot/tests.rs`, benchmark smoke output with `actualBackend`, and `scripts/check_no_new_production_unwraps.py`. |

## Deliberate next targets

These are intentionally not part of the current branch's completion criteria:

- 3D surface/volume parity with Makie.
- Streamplot and function-backed vector fields.
- Charton-style grammar-of-graphics/layer specification.
- Kuva-style CLI and bioinformatics-specific plots such as Manhattan, UpSet, and synteny.
- Polars-first dataframe grammar.

The branch establishes the API, diagnostics, benchmark, and guardrail foundation
needed to add those without obscuring which backend and performance path actually
ran.
