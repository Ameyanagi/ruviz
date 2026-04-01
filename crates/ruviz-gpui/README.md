# ruviz-gpui

`ruviz-gpui` contains the native `gpui` integration layer used for embedded interactive plots.

## Example Programs

The crate already ships runnable examples in [`examples/`](examples):

- `static_embed.rs`
- `observable_embed.rs`
- `streaming_embed.rs`
- `fixed_bounds_dashboard.rs`

Run one locally with:

```sh
cargo run -p ruviz-gpui --example static_embed
```

Use this crate when you want an embedded native surface rather than the browser-first WASM stack.
