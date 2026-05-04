# ruviz Web Examples

The files in this directory are the source of truth for the web gallery and the
manual docs examples.

Each example should be suitable for:

- local SDK debugging
- gallery/documentation embedding
- source-aligned npm package onboarding snippets

## Local Preview

```sh
bun install
bun run --cwd packages/ruviz-web build:js
bun run --cwd packages/ruviz-web docs:dev
```

## Authoring Expectations

- keep examples self-contained
- import from `../src/index.js` in gallery source files so local docs exercise the workspace SDK
- translate those imports to `ruviz` in published package snippets
- update the docs pages when you add a new example category or workflow
