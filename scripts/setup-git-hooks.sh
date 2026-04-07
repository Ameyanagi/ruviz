#!/usr/bin/env bash

set -euo pipefail

if ! command -v git >/dev/null 2>&1; then
  exit 0
fi

if [ -n "${CI:-}" ] || [ -n "${GITHUB_ACTIONS:-}" ]; then
  echo "Skipping git hook setup in CI."
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "${repo_root}" ]; then
  echo "Skipping git hook setup outside a git worktree."
  exit 0
fi

lefthook_bin="${repo_root}/node_modules/.bin/lefthook"
legacy_hook_path=".githooks"
git_hook_file="${repo_root}/.git/hooks/pre-commit"

if [ ! -x "${lefthook_bin}" ]; then
  echo "Lefthook is not installed yet. Run 'bun install' before configuring git hooks."
  exit 1
fi

current_hook_path="$(git -C "${repo_root}" config --local --get core.hooksPath || true)"
if [ -n "${current_hook_path}" ] && [ "${current_hook_path}" != "${legacy_hook_path}" ]; then
  echo "Leaving existing core.hooksPath=${current_hook_path}"
  exit 0
fi

if [ "${current_hook_path}" = "${legacy_hook_path}" ]; then
  git -C "${repo_root}" config --local --unset core.hooksPath
  echo "Removed legacy core.hooksPath=${legacy_hook_path}"
fi

(
  cd "${repo_root}"
  "${lefthook_bin}" install
)

cat >"${git_hook_file}" <<EOF
#!/bin/sh
set -eu

repo_root="\$(git rev-parse --show-toplevel)"
lefthook_bin="\${repo_root}/node_modules/.bin/lefthook"

if [ ! -x "\${lefthook_bin}" ]; then
  echo "Lefthook is not installed yet. Run 'bun install' before committing."
  exit 1
fi

exec "\${lefthook_bin}" run pre-commit "\$@"
EOF
chmod +x "${git_hook_file}"

echo "Installed Lefthook hooks"
