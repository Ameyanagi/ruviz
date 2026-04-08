# Release Notes

Versioned release notes are stored in this directory using the tag name format:

- `vX.Y.Z.md`
- `vX.Y.Z-rcN.md`

Examples:

- `v0.3.3.md`
- `v0.4.0-rc1.md`

## Workflow Integration

The release workflow (`.github/workflows/release.yml`) automatically:

1. Resolves the pushed tag name (for example, `v0.3.3`)
2. Validates that the Rust crates, npm package, and Python package versions all match the tag
3. Publishes Rust crates to crates.io and the JS SDK to npm for final releases
4. Publishes the Python package to PyPI for both final and prerelease tags
5. Looks for `docs/releases/<tag>.md`
6. Uses that file as the GitHub Release body when found
7. Falls back to a minimal generated release body when missing

Releases are expected to publish from this GitHub workflow, not from local
`cargo publish`, `npm publish`, `gh release create`, or manual PyPI uploads.
The release workflow is the single supported publishing path.

## One-Time Setup

Before the first Python release, configure PyPI Trusted Publishing for this repository:

1. Create or claim the `ruviz` project on PyPI
2. Add a trusted publisher for `Ameyanagi/ruviz`
3. Point it at `.github/workflows/release.yml`
4. Use the `pypi` GitHub Actions environment

## Authoring Checklist

Before creating a tag:

1. Add or update `docs/releases/vX.Y.Z.md`
2. Ensure `CHANGELOG.md` includes a matching `X.Y.Z` section
3. Keep release versions aligned across `Cargo.toml`, `crates/ruviz-web`, `packages/ruviz-web/package.json`, `python/Cargo.toml`, and `python/pyproject.toml`
4. For prerelease tags such as `vX.Y.Z-rc1`, use the matching PEP 440 version in `python/pyproject.toml` (`X.Y.Zrc1`)
5. Verify documentation snippets reflect the target release version where needed

## Release Procedure

1. Merge the version bump, changelog, and release notes to `main`
2. Create and push the release tag, for example `git tag -a vX.Y.Z -m "Release vX.Y.Z"` followed by `git push origin vX.Y.Z`
3. Let `.github/workflows/release.yml` publish the crates, npm package, Python distributions, and GitHub Release from that tag
4. Use `workflow_dispatch` only to recover or rerun an existing tag release; it still publishes from `refs/tags/<tag>`, not from a branch tip or ad hoc commit
