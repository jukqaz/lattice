# Branch And Release Policy For LLM Agents

[LLM Documentation Index](README.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

This document is the LLM-oriented execution contract for Lattice branch,
pull-request, and release work. Public user documentation lives in `README.md`
and `docs/user/`.

## Defaults

- Answer the user in Korean, but keep LLM-readable project guidance in English.
- Keep public human-facing docs bilingual with English as the default and Korean
  translations as `.ko.md` sibling files.
- Treat `main` as protected and releasable.
- Do not publish, tag, push, or create releases unless the user explicitly asks.
- Do not store or print secret values.
- Use English Conventional Commits.
- Prefer small, reviewable changes over broad cleanup.

## Branch Rules

Use short-lived branches with these names:

- `feat/<topic>` for user-facing features.
- `fix/<topic>` for defects.
- `docs/<topic>` for documentation.
- `ci/<topic>` for GitHub Actions, harnesses, lint, and automation.
- `chore/<topic>` for repository structure, dependency, or build maintenance.
- `security/<topic>` for security fixes.
- `release/vX.Y.Z` only for short release preparation.

Never force-push `main`. Never rewrite public release tags.

## Required Verification

Before claiming code or CI work is complete, run the smallest credible local
checks and report exact results. For normal code changes, run:

```bash
cargo run -p xtask -- verify
```

For release-oriented or platform-sensitive changes, also run:

```bash
cargo run -p xtask -- linux-verify
cargo run -p xtask -- quality
```

For GitHub Actions changes, also run:

```bash
actionlint .github/workflows/ci.yml
ruby -e 'require "yaml"; YAML.load_file(ARGV.fetch(0)); puts "yaml ok"' .github/workflows/ci.yml
```

Always run:

```bash
git -c core.fsmonitor=false diff --check
```

## GitHub Actions Gate

The CI workflow must keep `cargo run -p xtask -- verify` green on:

- Linux x86_64: `ubuntu-latest`
- Linux ARM64: `ubuntu-24.04-arm`
- macOS Apple Silicon: `macos-latest`

The workflow should install stable Rust with `rustfmt`, `clippy`, and
`wasm32-wasip2`. It should run the heavier `xtask quality` gate on Linux x86_64.
Do not add automatic publishing to CI.

## PR Rules

A PR is ready for review only when:

- Local verification has passed.
- GitHub Actions matrix has passed or the user knows it has not run yet.
- README or docs are updated for user-visible CLI, config, safety, or release
  behavior changes.
- The PR body lists verification evidence.

Merge preference:

- Prefer rebase merge for curated logical commits.
- Use squash merge for noisy external contributions.
- Use merge commits only when preserving branch topology is intentional.

## Release Rules

Lattice is git-distributed. Do not publish to crates.io.

Use SemVer:

- Patch for fixes, docs, and harness hardening.
- Minor for compatible CLI commands, presets, config keys, or behavior.
- Major for breaking CLI or config contract changes.

Release checklist:

1. Confirm `main` is green.
2. Update workspace version and `Cargo.lock` if a version bump is required.
3. Run local `cargo run -p xtask -- verify`.
4. Run local Docker `cargo run -p xtask -- linux-verify`.
5. Run local `cargo run -p xtask -- quality`.
6. Confirm GitHub Actions passed on Linux x86_64, Linux ARM64, and macOS Apple
   Silicon.
7. Confirm the GitHub Actions quality gate passed on Linux x86_64.
8. Create an annotated `vX.Y.Z` tag only after user approval.
9. Create a GitHub Release only after user approval.
10. Smoke test tag install:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag vX.Y.Z --locked
```

## Do Not

- Do not run `git push`, create tags, or create GitHub Releases without explicit
  user approval.
- Do not add crates.io, Homebrew, binary installer, or auto-publish steps unless
  explicitly requested.
- Do not weaken secret scanning, path safety, restore conflict checks, or CI
  matrix coverage to make a release pass.
- Do not call a workflow green until local validation and remote Actions status
  are distinguished clearly.
