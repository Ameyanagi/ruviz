# Python Examples

The examples in this directory are the source of truth for the Python docs and gallery.

Examples import the installed `ruviz` package and local helpers from
`examples/_shared.py`. Install optional dataframe dependencies before running
`dataframe_line.py`; `uv sync` includes them for contributors.

Run a single example:

```sh
cd python
uv run python examples/line.py
```

Run the example-backed docs smoke tests:

```sh
cd python
uv run pytest tests/test_examples.py
```

Regenerate the gallery previews and the generated gallery page:

```sh
cd python
uv run maturin develop
uv run python scripts/generate_gallery.py
```
