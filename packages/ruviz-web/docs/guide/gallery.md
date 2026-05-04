# Gallery

The web gallery is sourced from `packages/ruviz-web/examples/`.

Each example doubles as:

- a runnable development example
- a gallery source for the VitePress docs
- a regression surface for the high-level Web SDK flows

The files import from `../src/index.js` so the local docs always exercise the
workspace source. Published package snippets should import the same symbols from
`ruviz`.

Use the local docs build to preview the same gallery content that ships with the
package docs:

```sh
bun run --cwd packages/ruviz-web docs:dev
```

<PlotGallery />
