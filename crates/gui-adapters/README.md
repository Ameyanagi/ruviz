# Ruviz native GUI adapters

This isolated Cargo workspace contains the framework-native adapters for:

- [`ruviz-egui`](ruviz-egui)
- [`ruviz-iced`](ruviz-iced)
- [`ruviz-slint`](ruviz-slint)

Keeping these crates outside the root workspace prevents GUI framework
dependencies from entering the core `ruviz` dependency graph. Build all three
adapter crates with:

```sh
cargo check --manifest-path crates/gui-adapters/Cargo.toml
```

All three crates provide framework-native, image-backed static and interactive
2D widgets plus optional 3D support. Their default dependencies do not select
an application shell, window backend, or renderer. Enable `3d` for software 3D
or `3d-gpu` for retained GPU rendering followed by explicit image readback.

Run the headless behavioral and documentation suites with:

```sh
cargo test --manifest-path crates/gui-adapters/Cargo.toml --workspace --all-features
cargo test --manifest-path crates/gui-adapters/Cargo.toml --workspace --doc --all-features
```
