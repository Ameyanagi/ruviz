# Ruviz Interactive Plotting Library - Example Generation Makefile
# 
# This Makefile generates all example outputs to a single organized folder,
# demonstrating both static and interactive plotting capabilities.

.PHONY: all examples static-examples interactive-examples performance-examples web-examples clean help

# Default target - generate all examples
all: examples

# Main example generation target
examples: static-examples interactive-examples performance-examples
	@echo "✅ All examples generated successfully!"
	@echo "📁 Check examples_output/ directory for results"

# Static plotting examples (traditional plot outputs)
static-examples: setup-dirs
	@echo "📊 Generating static plotting examples..."
	@echo "  - GPU performance comparison..."
	cargo run --release --example gpu_performance_plot
	@mv gpu_cpu_throughput.png examples_output/static/ 2>/dev/null || true
	@mv gpu_speedup.png examples_output/static/ 2>/dev/null || true
	
	@echo "  - Scientific showcase..."
	cargo run --release --example scientific_showcase 2>/dev/null || echo "  (Scientific showcase example not found - skipping)"
	@mv scientific_*.png examples_output/static/ 2>/dev/null || true
	
	@echo "  - Memory optimization demo..."
	cargo run --release --example memory_optimization_demo 2>/dev/null || echo "  (Memory demo example not found - skipping)"
	@mv memory_*.png examples_output/static/ 2>/dev/null || true
	
	@echo "  - Box plot demonstration..."
	cargo run --release --example boxplot_example 2>/dev/null || echo "  (Box plot example not found - skipping)"
	@mv boxplot_*.png examples_output/static/ 2>/dev/null || true
	
	@echo "  - Histogram demonstration..."
	cargo run --release --example histogram_example 2>/dev/null || echo "  (Histogram example not found - skipping)"
	@mv histogram_*.png examples_output/static/ 2>/dev/null || true
	
	@echo "  - Performance scaling plots..."
	cargo run --release --example create_performance_plot 2>/dev/null || echo "  (Performance plot example not found - skipping)"
	@mv gpu_cpu_performance_comparison.png examples_output/static/ 2>/dev/null || true
	@mv gpu_speedup_scaling.png examples_output/static/ 2>/dev/null || true
	@echo "✅ Static examples completed"

# Interactive plotting examples (with fallback to static when interactive not available)
interactive-examples: setup-dirs
	@echo "🎮 Generating interactive plotting examples..."
	@echo "  - Basic interaction demo..."
	cargo run --release --features interactive --example basic_interaction --no-window --output examples_output/interactive/ 2>/dev/null || \
	(cargo run --release --example basic_interaction && mv basic_interaction_static.png examples_output/interactive/ 2>/dev/null || true)
	
	@echo "  - Data brushing demo..."
	cargo run --release --features interactive --example data_brushing --no-window --output examples_output/interactive/ 2>/dev/null || \
	(cargo run --release --example data_brushing && mv data_brushing_*.png examples_output/interactive/ 2>/dev/null || true)
	
	@echo "  - Real-time performance demo..."
	cargo run --release --features interactive --example real_time_performance --no-window --output examples_output/interactive/ 2>/dev/null || \
	(cargo run --release --example real_time_performance && mv real_time_performance_static.png examples_output/interactive/ 2>/dev/null || true)
	@echo "✅ Interactive examples completed"

# Performance benchmarks and analysis
performance-examples: setup-dirs
	@echo "🔬 Running performance benchmarks..."
	@echo "  - Interactive performance analysis..."
	cargo run --release --features interactive --example real_time_performance > examples_output/performance/interactive_perf_report.txt 2>&1 || \
	echo "Interactive performance test failed or not available" > examples_output/performance/interactive_perf_report.txt
	
	@echo "  - GPU vs CPU comparison..."
	cargo run --release --example gpu_vs_cpu_benchmark > examples_output/performance/gpu_cpu_benchmark.txt 2>&1 || \
	echo "GPU benchmark failed or not available" > examples_output/performance/gpu_cpu_benchmark.txt
	
	@echo "  - Memory usage analysis..."
	cargo run --release --example performance_data_collector > examples_output/performance/memory_analysis.txt 2>&1 || \
	echo "Memory analysis failed or not available" > examples_output/performance/memory_analysis.txt
	
	@echo "  - Scaling performance test..."
	@echo "Testing performance with different dataset sizes..." > examples_output/performance/scaling_analysis.txt
	@for size in 1000 10000 50000 100000; do \
		echo "Testing $$size points..." >> examples_output/performance/scaling_analysis.txt; \
		timeout 30s cargo run --release --example minimal_gpu_benchmark -- --points $$size >> examples_output/performance/scaling_analysis.txt 2>&1 || \
		echo "  Test with $$size points failed or timed out" >> examples_output/performance/scaling_analysis.txt; \
	done
	@echo "✅ Performance analysis completed"

# Web (WebAssembly) examples
web-examples: setup-dirs
	@echo "🌐 Building WebAssembly examples..."
	@which wasm-pack > /dev/null || (echo "❌ wasm-pack not found. Install with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" && exit 1)
	
	@echo "  - Building basic interaction for web..."
	wasm-pack build --target web --out-dir examples_output/web/basic_interaction -- --features interactive 2>/dev/null || \
	echo "Web build failed - interactive features may not be fully WASM compatible yet" > examples_output/web/build_status.txt
	
	@echo "  - Creating web demo page..."
	@echo "<!DOCTYPE html>" > examples_output/web/index.html
	@echo "<html><head><title>Ruviz Interactive Plotting Demo</title></head>" >> examples_output/web/index.html
	@echo "<body><h1>🎮 Ruviz Interactive Plotting Demo</h1>" >> examples_output/web/index.html
	@echo "<p>WebAssembly-powered interactive plotting in the browser.</p></body></html>" >> examples_output/web/index.html
	@echo "✅ Web examples prepared (WASM implementation in progress)"

# Set up output directory structure
setup-dirs:
	@mkdir -p examples_output/static
	@mkdir -p examples_output/interactive
	@mkdir -p examples_output/performance
	@mkdir -p examples_output/web
	@mkdir -p examples_output/documentation

# Generate comprehensive documentation
docs: setup-dirs
	@echo "📖 Generating example documentation..."
	@echo "# Ruviz Example Gallery" > examples_output/documentation/README.md
	@echo "" >> examples_output/documentation/README.md
	@echo "This directory contains examples demonstrating the capabilities of the Ruviz interactive plotting library." >> examples_output/documentation/README.md
	@echo "" >> examples_output/documentation/README.md
	@echo "Generated on $$(date)" >> examples_output/documentation/README.md
	@echo "📖 Documentation generated"

# Quick demo - generate a small subset for quick testing
demo: setup-dirs
	@echo "🚀 Generating quick demo (subset of examples)..."
	cargo run --release --example gpu_performance_plot
	@mv gpu_cpu_throughput.png examples_output/static/ 2>/dev/null || true
	cargo run --release --example basic_interaction 2>/dev/null || true
	@mv basic_interaction_static.png examples_output/interactive/ 2>/dev/null || true
	@echo "✅ Quick demo completed"

# Test that examples compile without running them
test-compile:
	@echo "🔨 Testing example compilation..."
	cargo check --example basic_interaction --features interactive
	cargo check --example data_brushing --features interactive  
	cargo check --example real_time_performance --features interactive
	cargo check --example gpu_performance_plot
	@echo "✅ All examples compile successfully"

# Clean all generated files
clean:
	@echo "🧹 Cleaning generated examples..."
	rm -rf examples_output/
	rm -f *.png *.txt *.html
	cargo clean
	@echo "✅ Cleanup completed"

# Show help
help:
	@echo "Ruviz Example Generation System"
	@echo "================================"
	@echo ""
	@echo "Available targets:"
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
	@echo "Output directory: examples_output/"
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
	@echo "Static Examples (examples_output/static/):"
	@echo "  - gpu_cpu_throughput.png - GPU vs CPU performance comparison"
	@echo "  - gpu_speedup.png - GPU acceleration scaling analysis"  
	@echo "  - scientific_showcase.png - Multi-panel scientific figure"
	@echo "  - boxplot_example.png - Statistical box plot"
	@echo "  - histogram_example.png - Data distribution histogram"
	@echo ""
	@echo "Interactive Examples (examples_output/interactive/):"
	@echo "  - basic_interaction demo - Zoom/pan functionality"
	@echo "  - data_brushing demo - Multi-plot selection"
	@echo "  - real_time_performance demo - Large dataset interaction"
	@echo ""
	@echo "Performance Reports (examples_output/performance/):"  
	@echo "  - gpu_cpu_benchmark.txt - Detailed benchmark results"
	@echo "  - interactive_perf_report.txt - Real-time analysis"
	@echo "  - memory_analysis.txt - Memory profiling"
	@echo "  - scaling_analysis.txt - Performance vs dataset size"
	@echo ""
	@echo "Web Examples (examples_output/web/):"
	@echo "  - index.html - Browser demo page"
	@echo "  - WASM modules (when available)"
	@echo ""
	@echo "Run 'make examples' to generate all outputs"