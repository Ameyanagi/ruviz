SHELL := /bin/bash

RELEASE_DOCS_BRANCH := docs/release-0.4.0-refresh
PYTHON_SITE_DIR := ../generated/python/site

.PHONY: help assert-release-branch clean-generated release-docs release-docs-rust release-docs-python release-docs-web build-generated-preview generated-manifest check-doc-asset-refs fmt clippy check-web check bench-plotting bench-plotting-smoke bench-rust-features bench-rust-features-smoke

help:
	@echo "ruviz release documentation workflow"
	@echo ""
	@echo "Primary targets:"
	@echo "  make release-docs        Regenerate release media, docs, and validation output"
	@echo "  make release-docs-rust   Refresh Rust README/rustdoc/gallery/golden assets"
	@echo "  make release-docs-python Refresh Python gallery and build the MkDocs site"
	@echo "  make release-docs-web    Build the npm package docs site and API reference"
	@echo "  make build-generated-preview Rebuild local preview outputs under generated/"
	@echo "  make generated-manifest  Refresh generated/manifest.json from local outputs"
	@echo "  make check-doc-asset-refs Fail if published docs reference generated/ assets"
	@echo "  make clean-generated     Remove generated/ and retired local output roots"
	@echo ""
	@echo "Validation targets:"
	@echo "  make fmt                 cargo fmt --all -- --check"
	@echo "  make clippy              cargo clippy --all-targets --all-features -- -D warnings"
	@echo "  make check-web           bun run check:web"
	@echo "  make check               Run fmt, clippy, and check-web"
	@echo ""
	@echo "Benchmark targets:"
	@echo "  make bench-plotting"
	@echo "  make bench-plotting-smoke"
	@echo "  make bench-rust-features"
	@echo "  make bench-rust-features-smoke"
	@echo ""
	@echo "Generated developer preview root: generated/"
	@echo "Committed release media: docs/assets/ and tests/fixtures/golden/"

assert-release-branch:
	@current="$$(git branch --show-current)"; \
	if [ "$$current" != "$(RELEASE_DOCS_BRANCH)" ]; then \
		echo "release docs must run on $(RELEASE_DOCS_BRANCH), found $$current"; \
		exit 1; \
	fi

clean-generated:
	./scripts/clean-outputs.sh

release-docs: assert-release-branch clean-generated release-docs-rust release-docs-python release-docs-web generated-manifest check-doc-asset-refs
	@echo "Release docs refresh complete."

release-docs-rust:
	cargo run --example readme_quickstart
	./scripts/generate-doc-images.sh
	cargo run --bin generate_gallery
	cargo run --example generate_golden_images
	cargo test --all-features
	cargo doc -p ruviz --all-features --no-deps
	cargo doc -p ruviz-web --no-deps
	cargo doc -p ruviz-gpui --no-deps

release-docs-python:
	bun run build:python-widget
	cd python && uv run maturin develop
	cd python && uv run python scripts/generate_gallery.py
	cd python && uv run mkdocs build --site-dir $(PYTHON_SITE_DIR)
	cd python && uv run pytest

release-docs-web:
	bun run --cwd packages/ruviz-web build
	bun run --cwd packages/ruviz-web docs:api
	bun run --cwd packages/ruviz-web docs:build

build-generated-preview: clean-generated
	cargo run --example readme_quickstart
	./scripts/generate-doc-images.sh
	cargo run --bin generate_gallery
	cargo run --example generate_golden_images
	bun run build:python-widget
	cd python && uv run maturin develop
	cd python && uv run python scripts/generate_gallery.py
	cd python && uv run mkdocs build --site-dir $(PYTHON_SITE_DIR)
	bun run --cwd packages/ruviz-web build
	bun run --cwd packages/ruviz-web docs:api
	bun run --cwd packages/ruviz-web docs:build
	$(MAKE) generated-manifest
	$(MAKE) check-doc-asset-refs

generated-manifest:
	uv run python scripts/generate_output_manifest.py

check-doc-asset-refs:
	uv run python scripts/check_no_generated_asset_refs.py

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check-web:
	bun run check:web

check: fmt clippy check-web

bench-plotting:
	bun install --frozen-lockfile
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../benchmarks/plotting/run.py --mode full

bench-plotting-smoke:
	bun install --frozen-lockfile
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../benchmarks/plotting/run.py --mode smoke --output-dir ../benchmarks/plotting/results/smoke --docs-output ../benchmarks/plotting/results/smoke/report.md

bench-rust-features:
	cd python && uv sync --group bench
	cd python && uv run python ../benchmarks/plotting/run_rust_features.py --mode full

bench-rust-features-smoke:
	cd python && uv sync --group bench
	cd python && uv run python ../benchmarks/plotting/run_rust_features.py --mode smoke --output-dir ../benchmarks/plotting/results/rust-features/smoke --docs-output ../benchmarks/plotting/results/rust-features/smoke/report.md
