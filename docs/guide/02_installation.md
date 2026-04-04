# Installation

Complete guide to setting up ruviz in your Rust project.

## Prerequisites

### Rust Installation

ruviz requires **Rust 1.92 or later**. Check your version:

```bash
rustc --version
```

If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### System Dependencies

ruviz is pure Rust with no C dependencies, but some optional features may require system libraries.

#### Linux
```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config

# Fedora/RHEL
sudo dnf install gcc pkg-config

# Arch
sudo pacman -S base-devel
```

#### macOS
```bash
# Xcode Command Line Tools
xcode-select --install
```

#### Windows
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
- Or use [rustup-init.exe](https://rustup.rs/) which handles dependencies

## Adding ruviz to Your Project

### New Project

Create a new Rust project and add ruviz:

```bash
cargo new my_visualization
cd my_visualization
cargo add ruviz
```

### Existing Project

Add ruviz to your `Cargo.toml`:

```toml
[dependencies]
ruviz = "0.4.0"
```

Or with specific features:

```toml
[dependencies]
ruviz = { version = "0.4.0", features = ["ndarray_support", "parallel"] }
```

## Feature Flags

ruviz uses feature flags to enable optional functionality. Choose based on your needs.

### Default Features

```toml
ruviz = "0.4.0"  # Includes: ndarray, parallel
```

**Enabled by default**:
- `ndarray` - ndarray support for scientific computing
- `parallel` - Multi-core rendering with rayon

### Core Features

| Feature | Description | Use Case |
|---------|-------------|----------|
| `ndarray_support` | ndarray integration | Scientific computing, numpy-like arrays |
| `nalgebra_support` | nalgebra integration | Dense vectors/matrices, linear algebra |
| `polars_support` | polars integration | Data analysis, DataFrame support |
| `parallel` | Multi-core rendering | >10K points, batch processing |
| `simd` | SIMD optimization | >100K points, maximum speed |
| `gpu` | GPU acceleration | >1M points, real-time visualization |
| `interactive` | Interactive plots | Real-time exploration, data brushing |
| `window` | Window support | Desktop applications |
| `serde` | Serialization | Save/load plot configurations |
| `pdf` | PDF export | Publication-ready vector output |

SVG export is available without an extra feature flag. The legacy `svg` feature remains a no-op compatibility alias.

### Performance Bundles

```toml
# High performance (parallel + SIMD)
ruviz = { version = "0.4.0", features = ["performance"] }

# Maximum capability (all features)
ruviz = { version = "0.4.0", features = ["full"] }

# Minimal (no default features)
ruviz = { version = "0.4.0", default-features = false }
```

### Feature Combinations

**Scientific Computing**:
```toml
ruviz = { version = "0.4.0", features = ["ndarray_support", "parallel"] }
```

**Data Analysis**:
```toml
ruviz = { version = "0.4.0", features = ["polars_support", "performance"] }
```

**Publication Quality**:
```toml
ruviz = { version = "0.4.0", features = ["serde", "pdf"] }
```

**Real-time Visualization**:
```toml
ruviz = { version = "0.4.0", features = ["interactive-gpu"] }
```

**Large Datasets**:
```toml
ruviz = { version = "0.4.0", features = ["parallel", "simd", "gpu"] }
```

## Verification

### Quick Test

Create `src/main.rs`:

```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    Plot::new()
        .line(&x, &y)
        .title("Installation Test")
        .xlabel("X")
        .ylabel("Y")
        .save("test.png")?;

    println!("✅ Installation successful! Check test.png");
    Ok(())
}
```

Run:

```bash
cargo run
```

Expected output:
```
✅ Installation successful! Check test.png
```

### Feature Verification

Test specific features:

**ndarray**:
```rust
use ruviz::prelude::*;
use ndarray::Array1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x = Array1::linspace(0.0, 10.0, 100);
    let y = x.mapv(|v| v.sin());

    Plot::new()
        .line(&x, &y)
        .save("ndarray_test.png")?;

    println!("✅ ndarray support working");
    Ok(())
}
```

**Performance (parallel)**:
```rust
use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

    let start = std::time::Instant::now();
    let _image = Plot::new()
        .line(&x, &y)
        .render()?;

    println!("✅ Rendered 100K points in {:?}", start.elapsed());
    Ok(())
}
```

## IDE Setup

### VS Code

**Recommended Extensions**:
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) - Rust language support
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml) - TOML syntax

**Settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.cargo.features": ["ndarray", "parallel"],
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### IntelliJ IDEA / CLion

**Plugin**: [IntelliJ Rust](https://plugins.jetbrains.com/plugin/8182-rust)

**Configuration**:
- Enable "Use Clippy instead of Cargo Check"
- Set "Offline mode" to false for dependency resolution

### Vim/Neovim

**Plugins**:
- [rust.vim](https://github.com/rust-lang/rust.vim)
- [coc-rust-analyzer](https://github.com/fannheyward/coc-rust-analyzer)

**Configuration** (with coc.nvim):
```json
{
  "rust-analyzer.cargo.features": ["ndarray", "parallel"]
}
```

## Common Issues

### Build Errors

**Problem**: `error: failed to run custom build command for fontdue`

**Solution**: Update Rust toolchain:
```bash
rustup update
```

**Problem**: `error: linking with cc failed`

**Solution**: Install system build tools (see System Dependencies above)

### Feature Conflicts

**Problem**: `error: package ruviz v0.4.0 cannot be built because it requires rustc 1.92 or newer`

**Solution**: Update Rust:
```bash
rustup update stable
rustc --version  # Verify ≥ 1.92
```

### GPU Feature Issues

**Problem**: `error: failed to select Vulkan backend`

**Solution**: GPU features require graphics drivers:
```bash
# Linux - Install Vulkan drivers
sudo apt-get install vulkan-tools  # Ubuntu/Debian
sudo dnf install vulkan-tools       # Fedora

# Verify
vulkaninfo
```

Or disable GPU features:
```toml
ruviz = { version = "0.4.0", default-features = false, features = ["parallel"] }
```

### Memory Issues (Large Datasets)

**Problem**: Out of memory with large datasets

**Solution**: Save directly; the PNG export path already switches to large-dataset rendering internally:
```rust
let x: Vec<f64> = (0..10_000_000).map(|i| i as f64).collect();
let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

Plot::new()
    .line(&x, &y)
    .save("large_dataset.png")?;
```

## Performance Tuning

### Compile-Time Optimization

**Release builds** (essential for performance):
```bash
cargo build --release
cargo run --release
```

**Profile-guided optimization** (`Cargo.toml`):
```toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for max optimization
```

### Runtime Configuration

**CPU cores** (automatic detection):
```rust
// rayon uses the available cores when the parallel render path is selected
let x = vec![0.0, 1.0, 2.0];
let y = vec![0.0, 1.0, 4.0];

let _image = Plot::new()
    .line(&x, &y)
    .render()?;
```

**Memory pooling** (opt-in):
```rust
Plot::new()
    .with_memory_pooling(true)
    .line(&x, &y)
    .save("pooled.png")?;
```

## Platform-Specific Notes

### Linux

**Desktop interactive backends**: `interactive` and `ruviz-gpui` are supported
on Linux.

**Linux desktop build prerequisite**: install GTK3 development headers before
building `interactive` or `ruviz-gpui` because native save dialogs use
GTK-backed `rfd` on Linux:
```bash
sudo apt-get install libgtk-3-dev
```

**Font rendering**: System fonts automatically detected from:
- `/usr/share/fonts/`
- `~/.local/share/fonts/`
- `/usr/local/share/fonts/`

**Wayland/X11**: Both supported transparently

### macOS

**Font rendering**: Supports system fonts from:
- `/System/Library/Fonts/`
- `/Library/Fonts/`
- `~/Library/Fonts/`

**Retina displays**: Automatic HiDPI handling

### Windows

**Desktop interactive backends**: `interactive` and `ruviz-gpui` are supported
on the recommended `x86_64-pc-windows-msvc` target.

**Font rendering**: Fonts loaded from Windows registry

**MSVC vs GNU**: The core `ruviz` crate supports both toolchains, but the
desktop interactive verification path uses MSVC:
```bash
# MSVC (default)
rustup default stable-msvc

# GNU (MinGW) for core-only workflows
rustup default stable-gnu
```

### WebAssembly (WASM)

Experimental WASM support is now available:

- `ruviz` compiles for `wasm32-unknown-unknown` and supports in-memory PNG/SVG output.
- `crates/ruviz-web` provides browser canvas bindings for interactive rendering.
- `ruviz` registers a bundled browser fallback font automatically for canvas sessions.
- `ruviz` exposes `web_runtime_capabilities()` so apps can detect worker, touch, and WebGPU availability.
- `demo/web` contains a Vite example for main-thread, OffscreenCanvas, and Observable usage.

Current limitations:
- Desktop-only helpers such as file-path export and native window integration are not exposed on wasm.
- A dedicated WebGPU canvas fast path is not implemented yet; browser sessions currently render through the CPU image path.

## Next Steps

✅ **Installation complete!** Continue to:

- **[First Plot](03_first_plot.md)** - Create your first visualization in 5 minutes
- **[Plot Types](04_plot_types.md)** - Explore available plot types
- **[Examples](../../examples/)** - Browse working code samples

## Getting Help

- **Build Issues**: [GitHub Issues](https://github.com/Ameyanagi/ruviz/issues)
- **Questions**: [GitHub Discussions](https://github.com/Ameyanagi/ruviz/discussions)
- **Documentation**: [docs.rs/ruviz](https://docs.rs/ruviz)

---

**Ready to create your first plot?** → [First Plot Tutorial](03_first_plot.md)
