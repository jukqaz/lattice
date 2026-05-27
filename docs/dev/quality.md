# Quality Gates

English | [한국어](quality.ko.md) | [Documentation Index](../README.md)

Use this page when preparing a release-oriented change or reproducing the CI
quality job locally.

## Fast Local Gate

Run the shared verification harness for normal development tasks:

```bash
cargo run -p xtask -- verify
git diff --check
```

`xtask verify` runs formatting checks, Clippy, workspace tests, CLI smoke tests,
product-surface harness checks, and a non-Unix `lattice-core` compile check when
the `wasm32-wasip2` target is installed.

For release dogfood against your real HOME, run the read-only real HOME health
check below after the regular isolated harness is green.

Install the optional compile target when you want the non-Unix check locally:

```bash
rustup target add wasm32-wasip2
cargo run -p xtask -- verify
```

## Full Quality Gate

The release/CI quality gate expects these tools on `PATH`:

```bash
cargo install cargo-deny --locked
cargo install cargo-machete --locked
cargo install cargo-llvm-cov --locked
cargo install typos-cli --locked
rustup component add llvm-tools-preview
```

Then run:

```bash
cargo run -p xtask -- quality
```

`xtask quality` first runs `xtask verify`, then runs:

```bash
cargo-deny check
cargo-machete --with-metadata --skip-target-dir
typos --config _typos.toml
cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/llvm-cov/lcov.info
```

## Real HOME Read-Only Health Check

Before a release tag, you can dogfood the current binary against your live
`HOME`/XDG environment without creating config, registering services, backing up,
restoring, pruning snapshots, or committing repos:

```bash
scripts/real-home-readonly-health-check.sh
```

Set `LATTICE_BIN=/path/to/lattice` to check an already installed binary, or run
`cargo build -p lattice` first so `target/debug/lattice` exists. The script never
falls back to `cargo run`, because a release dogfood health check should not
create build/cache side effects while inspecting a live HOME. The script prints
the exact read-only commands it runs: `doctor`, `validate`, `bootstrap check`,
`service list`, per-service `status --json` and `plan --json`, `discover --json`,
`group list --json`, and per-group `status --json` and `plan --json`. Non-zero
command exits are reported as health findings and make the script exit non-zero,
so CI or release checklists cannot accidentally treat a failing diagnostic as a
pass.

This is a read-only real HOME health check. Do not replace it with `init`,
`backup`, `restore`, `adopt`, `track`, `snapshot prune`, `undo --yes`, or repo
push/commit flows unless the user explicitly approves live HOME mutation.

## Workflow Lint

When `.github/workflows/ci.yml` changes, also run `actionlint` if available:

```bash
actionlint .github/workflows/ci.yml
```

The current CI workflow installs quality tools before running `xtask quality`; a
local failure that says a required quality tool is missing means the development
machine is not bootstrapped yet, not that product tests failed.
