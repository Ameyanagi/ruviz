# ruviz Python

`ruviz` provides Python bindings and notebook integration for the Rust `ruviz` plotting runtime.

## Local development

```sh
cd python
uv sync
uv run maturin develop
```

The notebook widget reuses the browser runtime assets vendored under `python/ruviz/web/`.

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
