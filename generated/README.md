# Generated Preview Artifacts

This tree stores local rebuild output and CI preview artifacts.

Examples:

- `examples/` contains rendered example images
- `tests/` contains optional rendered test and export artifacts
- `python/site/` contains the built Python docs site
- `web/docs/` contains the built npm/web docs site

Only this file and `generated/manifest.json` are tracked in git. The rebuilt
contents under the subdirectories above are ignored locally and uploaded from CI
for pull request review when the tracked preview manifest changes.

Regenerate the tree locally with:

```sh
make build-generated-preview
```

Refresh the tracked manifest only with:

```sh
make generated-manifest
```

The tracked manifest intentionally includes only `examples/`, `python/site/`,
and `web/docs/` so preview rebuilds stay deterministic from a clean checkout.
If you run test suites that emit files under `generated/tests/`, those outputs
remain local developer artifacts and are not part of the default PR preview.

Published docs must continue to read from committed assets under `docs/assets/`,
`python/docs/assets/gallery/`, and `tests/fixtures/golden/`.
