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

hook_path=".githooks"
hook_file="${repo_root}/${hook_path}/pre-commit"

if [ ! -f "${hook_file}" ]; then
  echo "Skipping git hook setup because ${hook_file} is missing."
  exit 0
fi

current_hook_path="$(git -C "${repo_root}" config --local --get core.hooksPath || true)"
if [ -n "${current_hook_path}" ] && [ "${current_hook_path}" != "${hook_path}" ]; then
  echo "Leaving existing core.hooksPath=${current_hook_path}"
  exit 0
fi

chmod +x "${hook_file}"

if [ "${current_hook_path}" = "${hook_path}" ]; then
  echo "Git hooks already configured at ${hook_path}"
  exit 0
fi

git -C "${repo_root}" config --local core.hooksPath "${hook_path}"
echo "Configured git hooks path to ${hook_path}"
