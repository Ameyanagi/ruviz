# Contributing

## Workflow

- Open an issue or discussion first for non-trivial changes.
- Keep pull requests focused and small enough to review.
- Update docs and examples when public behavior changes.
- Add or update tests when fixing bugs or changing APIs.

## Setup

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --all-features
cargo test --doc
```

Run the relevant examples or targeted tests when you touch rendering, export, or interactive behavior.

## Pull Requests

- Describe the user-visible change clearly.
- Call out any feature flags required to exercise the change.
- Include screenshots or generated output when a visual behavior changes.

## Documentation

- Keep [README.md](README.md) and the files under [docs](docs) consistent with the current API.
- Prefer examples that compile against the current public surface.
