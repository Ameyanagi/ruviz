# ruviz Web Examples

The files in this directory are the source of truth for the web gallery and the
manual docs examples.

Each example should be suitable for:

- local SDK debugging
- gallery/documentation embedding
- published npm package onboarding

## Local Preview

```sh
bun install
bun run --cwd packages/ruviz-web docs:dev
```

## Authoring Expectations

- keep examples self-contained
- prefer published package entrypoints over internal imports
- update the docs pages when you add a new example category or workflow
