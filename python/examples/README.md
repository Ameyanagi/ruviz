# Python Examples

The examples in this directory are the source of truth for the Python docs and gallery.

Run a single example:

```sh
cd python
uv run python examples/line.py
```

Regenerate the gallery previews and the generated gallery page:

```sh
cd python
uv run maturin develop
uv run python scripts/generate_gallery.py
```
