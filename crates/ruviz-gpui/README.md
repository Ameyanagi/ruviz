# ruviz-gpui

`ruviz-gpui` is the native GPUI adapter for embedding `ruviz` plots inside a
desktop GPUI application.

It keeps `ruviz` plot construction and interaction behavior, while letting GPUI
own the window, layout tree, and surrounding application shell.

## Install

```toml
[dependencies]
ruviz = "0.4.11"
ruviz-gpui = "0.4.11"
```

## What This Crate Provides

- an embeddable GPUI plot view for static and interactive plots
- configurable image and hybrid presentation modes
- pan, zoom, hover, selection, and context-menu integration
- PNG save and clipboard-copy actions routed through the host platform

## Platform Notes

`ruviz-gpui` currently supports:

- macOS
- Linux
- Windows

On Linux the crate uses GTK-backed native dialogs. Install GTK3 development
headers before building desktop examples.

## Examples

Runnable examples live in the crate:

```sh
cargo run -p ruviz-gpui --example static_embed
cargo run -p ruviz-gpui --example observable_embed
cargo run -p ruviz-gpui --example streaming_embed
```

## Related Docs

- Root crate docs: <https://docs.rs/ruviz>
- Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
- GPUI example directory: <https://github.com/Ameyanagi/ruviz/tree/main/crates/ruviz-gpui/examples>
- Release notes: <https://github.com/Ameyanagi/ruviz/tree/main/docs/releases>
