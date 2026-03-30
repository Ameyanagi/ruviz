# Release Notes

Versioned release notes are stored in this directory using the tag name format:

- `vX.Y.Z.md`

Examples:

- `v0.2.0.md`

## Workflow Integration

The release workflow (`.github/workflows/release.yml`) automatically:

1. Resolves the pushed tag name (for example, `v0.2.0`)
2. Looks for `docs/releases/<tag>.md`
3. Uses that file as the GitHub Release body when found
4. Falls back to a minimal generated release body when missing

## Authoring Checklist

Before creating a tag:

1. Add or update `docs/releases/vX.Y.Z.md`
2. Ensure `CHANGELOG.md` includes a matching `X.Y.Z` section
3. Verify documentation snippets reflect the target release version where needed
