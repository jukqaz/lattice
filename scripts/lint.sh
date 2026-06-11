#!/usr/bin/env bash
# Fast local lint gate for day-to-day Lattice maintenance.
#
# This is intentionally check-only: it never formats files in place and it does
# not run the full test suite. Use `cargo run -p xtask -- verify` before a PR.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

if command -v shellcheck > /dev/null 2>&1; then
  shellcheck scripts/*.sh
else
  printf 'lattice local lint: skipped shellcheck; command not found\n' >&2
fi

if command -v shfmt > /dev/null 2>&1; then
  shfmt -d -i 2 -ci -sr scripts/*.sh
else
  printf 'lattice local lint: skipped shfmt; command not found\n' >&2
fi

if command -v actionlint > /dev/null 2>&1; then
  actionlint .github/workflows/*.yml
else
  printf 'lattice local lint: skipped actionlint; command not found\n' >&2
fi

printf 'lattice local lint: ok\n'
