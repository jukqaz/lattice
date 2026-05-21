# Lattice MVP TODO

## v0.1 Scope

- [x] Create a Rust CLI repository.
- [x] Write the MVP design spec.
- [x] Write the implementation plan.
- [x] Add config and service TOML models.
- [x] Add XDG path resolution.
- [x] Add `init`, `doctor`, `service list`, `backup`, and `restore` commands.
- [x] Add Codex preset include/exclude defaults.
- [x] Add permission manifest capture and restore.
- [x] Add focused tests for config loading, scanning, backup, and restore.
- [x] Run formatting, tests, and a local smoke flow.
- [x] Add `status`, `backup --dry-run`, and `restore --dry-run` as minimal safety UX.
- [x] Add release-ready README, license, and Cargo metadata.
- [x] Run `cargo run -p xtask -- verify` at the end of every implementation task before marking it complete.
- [x] Add warning-level workspace lint rules and include Clippy in the shared verification harness.
- [x] Split the repository into a Rust workspace with `lattice-core`, `lattice` CLI, and `xtask`.

## Explicit Non-Goals For v0.1

- No automatic git commit or push.
- No remote repository creation.
- No secret value materialization.
- No multi-profile orchestration.
- No GUI or TUI.
- No state database.

## Task Completion Gate

Every implementation task must end with the shared harness:

```bash
cargo run -p xtask -- verify
```

Do not mark a task complete until formatting, full tests, and the isolated XDG backup/restore smoke pass.
Use `cargo run -p xtask -- linux-verify` for Docker-backed Linux verification before release-oriented changes.
Use `cargo run -p xtask -- quality` for dependency policy, unused dependency, typo, and coverage checks.
GitHub Actions must keep the same harness green on Linux x86_64, Linux ARM64, and macOS Apple Silicon.

## v0.2 Light MVP

See `docs/product/mvp-scope.md`.

- [x] Add restore conflict detection.
- [x] Add restore snapshots before overwrite.
- [x] Add minimal lifecycle hooks.
- [x] Add a custom service fixture.
- [x] Add real `~/.codex` read-only dry-run coverage to `xtask verify`.
- [x] Add lightweight secret-looking content scan.
- [x] Add `validate` if it stays small.
- [x] Default omitted service repos to `$XDG_DATA_HOME/lattice/repos/<service>`.

## MVP 2

- [x] Add `service add`, `service show`, and `service remove`.
- [x] Add `include add/remove` and `exclude add/remove`.
- [x] Add `permission set/remove`.
- [x] Add git repo commands: `repo status`, `repo pull`, `repo commit`, `repo push`.
- [x] Add Vaultwarden-backed secret metadata with `rbw` and `bw`.
- [x] Add preset catalog for `git`, `zsh`, `mise`, and `ssh`.
- [x] Add `track` / `adopt` command.
- [x] Add TUI over the same core/config model.
- [x] Add optional symlink restore mode.
- [x] Add OS and hostname service conditions.
- [x] Add simple env-var template rendering on restore.
