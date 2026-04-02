# ruviz Python

`ruviz` provides Python bindings and notebook integration for the Rust `ruviz` plotting runtime.

## Local development

```sh
cd python
uv sync
uv run maturin develop
```

Rebuild the notebook widget bundle from the repository root after changing
`python/python/ruviz/widget.entry.js` or the web SDK. The build bootstraps the
repo-pinned `wasm-pack` tool automatically and uses a reproducible wasm build
for the notebook bundle. CI and release rebuild the canonical Linux bundle
automatically:

```sh
bun run build:python-widget
```

## Examples

Runnable examples live in [`examples/`](examples).

```sh
cd python
uv run python examples/line.py
```

Regenerate the docs gallery:

```sh
cd python
uv run maturin develop
uv run python scripts/generate_gallery.py
```

## Docs

Build or serve the standalone MkDocs site:

```sh
cd python
uv run maturin develop
uv run python scripts/generate_gallery.py
uv run mkdocs serve
```
