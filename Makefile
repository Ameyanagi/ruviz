# Ruviz Interactive Plotting Library - Example Generation Makefile
# 
# This Makefile generates all example outputs to a single organized folder,
# demonstrating both static and interactive plotting capabilities.

.PHONY: all examples static-examples interactive-examples performance-examples web-examples clean help setup-hooks check fmt clippy check-web doc-images bench-plotting bench-plotting-smoke bench-rust-features bench-rust-features-smoke

# Default target - generate all examples
all: examples

# Main example generation target
examples: static-examples interactive-examples performance-examples
	@echo "✅ All examples generated successfully!"
	@echo "📁 Check examples/output/ directory for results"

# Static plotting examples (traditional plot outputs)
static-examples: setup-dirs
	@echo "📊 Generating static plotting examples..."
	@echo "  - GPU performance comparison..."
	cargo run --release --features gpu --example gpu_performance_plot
	
	@echo "  - Scientific showcase..."
	cargo run --release --example scientific_showcase 2>/dev/null || echo "  (Scientific showcase example not found - skipping)"
	
	@echo "  - Memory optimization demo..."
	cargo run --release --example memory_optimization_demo 2>/dev/null || echo "  (Memory demo example not found - skipping)"
	
	@echo "  - Box plot demonstration..."
	cargo run --release --example boxplot_example 2>/dev/null || echo "  (Box plot example not found - skipping)"
	
	@echo "  - Histogram demonstration..."
	cargo run --release --example histogram_example 2>/dev/null || echo "  (Histogram example not found - skipping)"
	
	@echo "  - Performance scaling plots..."
	cargo run --release --example create_performance_plot 2>/dev/null || echo "  (Performance plot example not found - skipping)"
	@echo "✅ Static examples completed"

# Interactive plotting examples (with fallback to static when interactive not available)
interactive-examples: setup-dirs
	@echo "🎮 Generating interactive plotting examples..."
	@echo "  - Basic interaction demo..."
	cargo run --release --features interactive --example basic_interaction --no-window --output examples/output/interactive/ 2>/dev/null || \
	cargo run --release --example basic_interaction 2>/dev/null || true
	
	@echo "  - Data brushing demo..."
	cargo run --release --features interactive --example data_brushing --no-window --output examples/output/interactive/ 2>/dev/null || \
	cargo run --release --example data_brushing 2>/dev/null || true
	
	@echo "  - Real-time performance demo..."
	cargo run --release --features interactive --example real_time_performance --no-window --output examples/output/interactive/ 2>/dev/null || \
	cargo run --release --example real_time_performance 2>/dev/null || true
	@echo "✅ Interactive examples completed"

# Performance benchmarks and analysis
performance-examples: setup-dirs
	@echo "🔬 Running performance benchmarks..."
	@echo "  - Interactive performance analysis..."
	cargo run --release --features interactive --example real_time_performance > examples/output/performance/interactive_perf_report.txt 2>&1 || \
	echo "Interactive performance test failed or not available" > examples/output/performance/interactive_perf_report.txt
	
	@echo "  - GPU vs CPU comparison..."
	cargo run --release --features gpu --example gpu_vs_cpu_benchmark > examples/output/performance/gpu_cpu_benchmark.txt 2>&1 || \
	echo "GPU benchmark failed or not available" > examples/output/performance/gpu_cpu_benchmark.txt
	
	@echo "  - Memory usage analysis..."
	cargo run --release --features gpu --example performance_data_collector > examples/output/performance/memory_analysis.txt 2>&1 || \
	echo "Memory analysis failed or not available" > examples/output/performance/memory_analysis.txt
	
	@echo "  - Scaling performance test..."
	@echo "Testing performance with different dataset sizes..." > examples/output/performance/scaling_analysis.txt
	@for size in 1000 10000 50000 100000; do \
		echo "Testing $$size points..." >> examples/output/performance/scaling_analysis.txt; \
		timeout 30s cargo run --release --features gpu --example minimal_gpu_benchmark -- --points $$size >> examples/output/performance/scaling_analysis.txt 2>&1 || \
		echo "  Test with $$size points failed or timed out" >> examples/output/performance/scaling_analysis.txt; \
	done
	@echo "✅ Performance analysis completed"

# Web (WebAssembly) examples
web-examples: setup-dirs
	@echo "🌐 Building WebAssembly examples..."
	@which wasm-pack > /dev/null || (echo "❌ wasm-pack not found. Install with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" && exit 1)
	
	@echo "  - Building basic interaction for web..."
	wasm-pack build --target web --out-dir examples/output/web/basic_interaction -- --features interactive 2>/dev/null || \
	echo "Web build failed - interactive features may not be fully WASM compatible yet" > examples/output/web/build_status.txt
	
	@echo "  - Creating web demo page..."
	@echo "<!DOCTYPE html>" > examples/output/web/index.html
	@echo "<html><head><title>Ruviz Interactive Plotting Demo</title></head>" >> examples/output/web/index.html
	@echo "<body><h1>🎮 Ruviz Interactive Plotting Demo</h1>" >> examples/output/web/index.html
	@echo "<p>WebAssembly-powered interactive plotting in the browser.</p></body></html>" >> examples/output/web/index.html
	@echo "✅ Web examples prepared (WASM implementation in progress)"

# Set up output directory structure
setup-dirs:
	@mkdir -p examples/output/static
	@mkdir -p examples/output/interactive
	@mkdir -p examples/output/performance
	@mkdir -p examples/output/web
	@mkdir -p examples/output/documentation

# Generate comprehensive documentation
docs: setup-dirs
	@echo "📖 Generating example documentation..."
	@echo "# Ruviz Example Gallery" > examples/output/documentation/README.md
	@echo "" >> examples/output/documentation/README.md
	@echo "This directory contains examples demonstrating the capabilities of the Ruviz interactive plotting library." >> examples/output/documentation/README.md
	@echo "" >> examples/output/documentation/README.md
	@echo "Generated on $$(date)" >> examples/output/documentation/README.md
	@echo "📖 Documentation generated"

# Quick demo - generate a small subset for quick testing
demo: setup-dirs
	@echo "🚀 Generating quick demo (subset of examples)..."
	cargo run --release --features gpu --example gpu_performance_plot
	cargo run --release --example basic_interaction 2>/dev/null || true
	@echo "✅ Quick demo completed"

# Test that examples compile without running them
test-compile:
	@echo "🔨 Testing example compilation..."
	cargo check --example basic_interaction --features interactive
	cargo check --example data_brushing --features interactive  
	cargo check --example real_time_performance --features interactive
	cargo check --example gpu_performance_plot --features gpu
	@echo "✅ All examples compile successfully"

# Clean all generated files
clean:
	@echo "🧹 Cleaning generated outputs..."
	./scripts/clean-outputs.sh
	cargo clean
	@echo "✅ Cleanup completed"

# ============================================================================
# Development Workflow
# ============================================================================

# Setup git hooks for pre-commit checks
setup-hooks:
	@echo "🔧 Setting up git hooks..."
	git config core.hooksPath .githooks
	@echo "✓ Git hooks configured to use .githooks/"
	@echo "Pre-commit will now run: cargo fmt --check, cargo clippy, oxfmt, and oxlint"

# Run all code quality checks (same as pre-commit)
check: fmt clippy check-web
	@echo "✓ All checks passed!"

# Check code formatting
fmt:
	@echo "Checking formatting..."
	cargo fmt --all -- --check

# Run clippy with strict warnings
clippy:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

# Run JS/TS formatting and lint checks
check-web:
	@echo "Running JS/TS checks..."
	bun run check:web

# Cross-runtime large-dataset plotting benchmark suite
bench-plotting:
	@echo "Running full large-dataset plotting benchmarks..."
	bun install --frozen-lockfile
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../benchmarks/plotting/run.py --mode full
	@echo "✓ Benchmark results written to benchmarks/plotting/results/reference/"

bench-plotting-smoke:
	@echo "Running smoke large-dataset plotting benchmarks..."
	bun install --frozen-lockfile
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../benchmarks/plotting/run.py --mode smoke --output-dir ../benchmarks/plotting/results/smoke --docs-output ../benchmarks/plotting/results/smoke/report.md
	@echo "✓ Smoke benchmark results written to benchmarks/plotting/results/smoke/"

bench-rust-features:
	@echo "Running full Rust feature-impact plotting benchmarks..."
	cd python && uv sync --group bench
	cd python && uv run python ../benchmarks/plotting/run_rust_features.py --mode full
	@echo "✓ Rust feature benchmark results written to benchmarks/plotting/results/rust-features/reference/"

bench-rust-features-smoke:
	@echo "Running smoke Rust feature-impact plotting benchmarks..."
	cd python && uv sync --group bench
	cd python && uv run python ../benchmarks/plotting/run_rust_features.py --mode smoke --output-dir ../benchmarks/plotting/results/rust-features/smoke --docs-output ../benchmarks/plotting/results/rust-features/smoke/report.md
	@echo "✓ Rust feature smoke results written to benchmarks/plotting/results/rust-features/smoke/"

# Generate documentation images at 300 DPI
doc-images:
	@echo "📸 Generating documentation images at 300 DPI..."
	./scripts/generate-doc-images.sh
	@echo "✓ Documentation images generated"

# ============================================================================
# Help
# ============================================================================

# Show help
help:
	@echo "Ruviz Example Generation System"
	@echo "================================"
	@echo ""
	@echo "Available targets:"
	@echo ""
	@echo "Development:"
	@echo "  setup-hooks         - Configure git pre-commit hooks"
	@echo "  check               - Run all code quality checks (fmt + clippy + web)"
	@echo "  fmt                 - Check code formatting"
	@echo "  clippy              - Run clippy linter"
	@echo "  check-web           - Run JS/TS oxfmt + oxlint checks"
	@echo "  doc-images          - Generate documentation images"
	@echo "  bench-plotting      - Run the full cross-runtime plotting benchmark suite"
	@echo "  bench-plotting-smoke - Run the smoke cross-runtime plotting benchmark suite"
	@echo "  bench-rust-features - Run the full Rust feature-impact plotting benchmark suite"
	@echo "  bench-rust-features-smoke - Run the smoke Rust feature-impact plotting benchmark suite"
	@echo ""
	@echo "Examples:"
	@echo "  all                 - Generate all examples (default)"
	@echo "  examples            - Generate all examples"
	@echo "  static-examples     - Generate static plot outputs"
	@echo "  interactive-examples - Generate interactive demos"
	@echo "  performance-examples - Run performance benchmarks"
	@echo "  web-examples        - Build WebAssembly demos"
	@echo "  docs                - Generate documentation"
	@echo "  demo                - Quick demo (subset)"
	@echo "  test-compile        - Test compilation without running"
	@echo "  clean               - Remove all generated files"
	@echo "  help                - Show this help"
	@echo ""
	@echo "Output directory: examples/output/"
	@echo "  static/       - PNG plot outputs"
	@echo "  interactive/  - Interactive demo results" 
	@echo "  performance/  - Benchmark reports"
	@echo "  web/          - WebAssembly builds"
	@echo "  documentation/ - Example guides and README"
	@echo ""
	@echo "Requirements:"
	@echo "  - Rust 1.75+ with cargo"
	@echo "  - Optional: wasm-pack for web examples"
	@echo "  - Features: interactive, gpu (recommended)"

# Progress monitoring (show what will be generated)
preview:
	@echo "📋 Example Generation Preview"
	@echo "============================="
	@echo ""
	@echo "Static Examples (examples/output/static/):"
	@echo "  - gpu_cpu_throughput.png - GPU vs CPU performance comparison"
	@echo "  - gpu_speedup.png - GPU acceleration scaling analysis"  
	@echo "  - scientific_showcase.png - Multi-panel scientific figure"
	@echo "  - boxplot_example.png - Statistical box plot"
	@echo "  - histogram_example.png - Data distribution histogram"
	@echo ""
	@echo "Interactive Examples (examples/output/interactive/):"
	@echo "  - basic_interaction demo - Zoom/pan functionality"
	@echo "  - data_brushing demo - Multi-plot selection"
	@echo "  - real_time_performance demo - Large dataset interaction"
	@echo ""
	@echo "Performance Reports (examples/output/performance/):"  
	@echo "  - gpu_cpu_benchmark.txt - Detailed benchmark results"
	@echo "  - interactive_perf_report.txt - Real-time analysis"
	@echo "  - memory_analysis.txt - Memory profiling"
	@echo "  - scaling_analysis.txt - Performance vs dataset size"
	@echo ""
	@echo "Web Examples (examples/output/web/):"
	@echo "  - index.html - Browser demo page"
	@echo "  - WASM modules (when available)"
	@echo ""
	@echo "Run 'make examples' to generate all outputs"
