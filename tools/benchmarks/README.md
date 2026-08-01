# Benchmark Harnesses

- `interactive/` measures retained interactive-session behavior across Rust and
  browser runtimes.
- `plotting/` compares plotting workloads across Python, Rust, wasm, and Rust
  feature combinations.

Run the supported plotting entry points from the repository root:

```sh
make bench-plotting
make bench-plotting-smoke
make bench-rust-features
make bench-rust-features-smoke
```

Committed baselines and reference results live below each suite's `results/`
directory. The plotting suite's locally regenerated smoke directories remain
ignored; the interactive suite keeps its recorded smoke baseline intentionally.
