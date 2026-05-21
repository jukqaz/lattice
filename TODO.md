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

## v0.2 Light MVP

See `docs/product/mvp-scope.md`.

- [x] Add restore conflict detection.
- [x] Add restore snapshots before overwrite.
- [x] Add minimal lifecycle hooks.
- [x] Add a custom service fixture.
- [x] Add real `~/.codex` read-only dry-run coverage to `xtask verify`.
- [x] Add lightweight secret-looking content scan.
- [x] Add `validate` if it stays small.
