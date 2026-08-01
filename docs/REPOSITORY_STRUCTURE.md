# Repository Structure

Ruviz groups tracked files by responsibility. New top-level directories should
be rare: add one only when none of the existing ownership boundaries fit, and
update the structure checker in the same change.

| Path | Responsibility |
| --- | --- |
| `.github/` | GitHub Actions, issue templates, and repository automation |
| `adapters/` | Native framework integrations that wrap the core crate |
| `apps/` | Runnable applications and deployable demos |
| `benches/` | Cargo benchmark targets for the root Rust crate |
| `bindings/` | Language and target bindings, currently Python and wasm |
| `docs/` | Maintained documentation and committed documentation assets |
| `examples/` | Runnable examples for the root Rust crate |
| `generated/` | Ignored build/output staging; only control files are tracked |
| `packages/` | Packages published through non-Cargo package managers |
| `proptest-regressions/` | Reproducible property-test regression cases |
| `scripts/` | Repository automation and policy checks |
| `src/` | Source for the root `ruviz` crate |
| `tests/` | Root-crate integration, contract, and visual tests |
| `tools/` | Developer tools, including the gallery and benchmark suites |

Root-level project files such as `Cargo.toml`, `package.json`, `README.md`, and
license files remain at the root.

## Placement Rules

- Put host-framework wrappers under `adapters/`; put language or compilation-
  target APIs under `bindings/`.
- Put an application under `apps/`, a published package under `packages/`, and
  developer-only programs or workflows under `tools/`.
- Keep Cargo's conventional root-crate targets in `benches/`, `examples/`,
  `src/`, and `tests/`.
- Treat `generated/` as disposable local output. See [Build Outputs](BUILD_OUTPUTS.md)
  for the tracked-file and cleanup policy.
- Do not introduce a generic `crates/` bucket. Choose the directory that
  describes why the crate exists.

The root Cargo workspace includes `bindings/wasm` and `bindings/python`.
`adapters/gpui` and `adapters/gui` are separate Cargo workspaces; see
[Architecture](ARCHITECTURE.md#repository-layout) for the reason.

## Retired Paths

These former locations must not be restored:

| Retired path | Canonical path |
| --- | --- |
| `benchmarks/` | `tools/benchmarks/` |
| `crates/gui-adapters/` | `adapters/gui/` |
| `crates/ruviz-gpui/` | `adapters/gpui/` |
| `crates/ruviz-web/` | `bindings/wasm/` |
| `demo/` | `apps/web-demo/` |
| `gallery/` | `tools/gallery/` |
| `packages/ruviz-web/` | `packages/ruviz/` |
| `python/` | `bindings/python/` |

`scripts/check_repository_structure.py` enforces this layout against
`git ls-files`. It intentionally ignores untracked and gitignored local
directories, so old local build artifacts do not affect the check; only files
proposed for tracking are in scope.
