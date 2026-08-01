SHELL := /bin/bash

RELEASE_DOCS_BRANCH := docs/release-0.4.0-refresh
PYTHON_SITE_DIR := ../generated/python/site

.PHONY: help setup-hooks assert-release-branch clean-generated release-docs release-docs-rust release-docs-python release-docs-web rust-gallery check-rust-gallery build-generated-preview build-generated-preview-rust build-generated-preview-python build-generated-preview-web generated-manifest check-doc-asset-refs check-docs check-ci-test-coverage fmt clippy clippy-gpui check-web check bench-plotting bench-plotting-smoke bench-rust-features bench-rust-features-smoke

help:
	@echo "ruviz release documentation workflow"
	@echo ""
	@echo "Primary targets:"
	@echo "  make setup-hooks         Install Lefthook git hooks"
	@echo "  make release-docs        Regenerate release media, docs, and validation output"
	@echo "  make release-docs-rust   Refresh Rust README/rustdoc/gallery/golden assets"
	@echo "  make release-docs-python Refresh Python gallery and build the MkDocs site"
	@echo "  make release-docs-web    Build the npm package docs site and API reference"
	@echo "  make rust-gallery        Refresh rustdoc images, then synchronize the Rust gallery"
	@echo "  make build-generated-preview Rebuild docs-facing preview outputs under generated/"
	@echo "  make build-generated-preview-rust Rebuild only generated/examples/ preview assets"
	@echo "  make build-generated-preview-python Rebuild only generated/python/site/"
	@echo "  make build-generated-preview-web Rebuild only generated/web/docs/"
	@echo "  make generated-manifest  Refresh generated/manifest.json from local outputs"
	@echo "  make check-doc-asset-refs Fail if published docs reference generated/ assets"
	@echo "  make check-docs          Validate Markdown links/fences and README quick start syntax"
	@echo "  make check-rust-gallery  Verify committed Rust gallery freshness without writes"
	@echo "  make clean-generated     Remove generated/ and retired local output roots"
	@echo ""
	@echo "Validation targets:"
	@echo "  make fmt                 cargo fmt --all -- --check (both workspaces)"
	@echo "  make clippy              cargo clippy --all-targets --all-features -- -D warnings"
	@echo "  make clippy-gpui         Lint the separate crates/ruviz-gpui workspace (pulls the zed GPUI checkout)"
	@echo "  make check-web           bun run check:web"
	@echo "  make check-ci-test-coverage Fail if CI compiles a test target it never runs"
	@echo "  make check               Run fmt, clippy, check-web, check-docs, and CI test coverage"
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

setup-hooks:
	bash ./scripts/setup-git-hooks.sh

clean-generated:
	./scripts/clean-outputs.sh

release-docs: assert-release-branch clean-generated release-docs-rust release-docs-python release-docs-web generated-manifest check-doc-asset-refs
	@echo "Release docs refresh complete."

release-docs-rust:
	cargo run --example readme_quickstart
	$(MAKE) rust-gallery
	cargo run --example generate_golden_images
	cargo test --all-features
	cargo doc -p ruviz --all-features --no-deps
	cargo doc -p ruviz-web --no-deps
	cargo doc --manifest-path crates/ruviz-gpui/Cargo.toml --no-deps

rust-gallery:
	./scripts/generate-doc-images.sh
	cargo run --bin generate_gallery

check-rust-gallery:
	cargo run --bin generate_gallery -- --check

release-docs-python:
	bun run build:python-widget
	cd python && uv run maturin develop
	cd python && uv run python scripts/generate_gallery.py
	cd python && uv run python -m mkdocs build --site-dir $(PYTHON_SITE_DIR)
	cd python && uv run python -m pytest

release-docs-web:
	bun run --cwd packages/ruviz build
	bun run --cwd packages/ruviz docs:api
	bun run --cwd packages/ruviz docs:build:preview

build-generated-preview: clean-generated
	$(MAKE) build-generated-preview-rust
	$(MAKE) build-generated-preview-python
	$(MAKE) build-generated-preview-web
	$(MAKE) generated-manifest
	$(MAKE) check-doc-asset-refs

build-generated-preview-rust:
	rm -rf generated/examples
	cargo run --bin generate_gallery -- --preview-only

build-generated-preview-python:
	rm -rf generated/python/site
	bun run build:python-widget
	cd python && uv run maturin develop
	cd python && uv run python scripts/generate_gallery.py
	cd python && uv run python -m mkdocs build --site-dir $(PYTHON_SITE_DIR)

build-generated-preview-web:
	rm -rf generated/web/docs
	bun run --cwd packages/ruviz build
	bun run --cwd packages/ruviz docs:api
	bun run --cwd packages/ruviz docs:build:preview

generated-manifest:
	uv run python scripts/generate_output_manifest.py

check-doc-asset-refs:
	uv run python scripts/check_no_generated_asset_refs.py

check-docs:
	uv run python scripts/check_docs.py

# tests/integration/ci_test_coverage.rs fails when .github/workflows/ci.yml
# compiles a tests/*.rs target that no pull-request job names in a `--test`
# flag, and when a job pins a toolchain without asserting the rustc it actually
# got. Adding a test file therefore requires assigning it to a CI lane; the
# whole-suite `--all-features --tests` run is a safety net and does not count,
# because a run that executes every target by construction could never let the
# guard fail.
check-ci-test-coverage:
	cargo test --test integration

# `crates/ruviz-gpui` is its own workspace (see the root Cargo.toml), so `-p`
# and `--all` cannot reach it from here and it needs an explicit manifest path.
# That split is what keeps every other cargo command in this repository from
# resolving the zed GPUI checkout, so the extra line is the price of it. The
# `fmt` half is free — `cargo fmt` never resolves dependencies.
GPUI_MANIFEST := crates/ruviz-gpui/Cargo.toml

fmt:
	cargo fmt --all -- --check
	cargo fmt --all --manifest-path $(GPUI_MANIFEST) -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

clippy-gpui:
	cargo clippy --manifest-path $(GPUI_MANIFEST) --all-targets --all-features -- -D warnings

check-web:
	bun run check:web

# `clippy-gpui` is deliberately not here: it is the one command that resolves
# the zed GPUI checkout, and making the default local check pay for it would
# undo the workspace split. CI runs it in its own job.
check: fmt clippy check-web check-docs check-ci-test-coverage

bench-plotting:
	bun install --frozen-lockfile --ignore-scripts
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../tools/benchmarks/plotting/run.py --mode full

bench-plotting-smoke:
	bun install --frozen-lockfile --ignore-scripts
	cd python && uv sync --group bench && uv run maturin develop --release
	cd python && uv run python ../tools/benchmarks/plotting/run.py --mode smoke --output-dir ../tools/benchmarks/plotting/results/smoke --docs-output ../tools/benchmarks/plotting/results/smoke/report.md

bench-rust-features:
	cd python && uv sync --group bench
	cd python && uv run python ../tools/benchmarks/plotting/run_rust_features.py --mode full

bench-rust-features-smoke:
	cd python && uv sync --group bench
	cd python && uv run python ../tools/benchmarks/plotting/run_rust_features.py --mode smoke --output-dir ../tools/benchmarks/plotting/results/rust-features/smoke --docs-output ../tools/benchmarks/plotting/results/rust-features/smoke/report.md
