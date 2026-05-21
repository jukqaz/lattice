# Changelog

English | [한국어](CHANGELOG.ko.md) | [Documentation Index](docs/README.md)

## v0.3.1

### Fixed

- Preserve included empty directories in the backup manifest and recreate them
  during restore. This covers service paths such as empty skill directories that
  are meaningful even without tracked files.

## v0.3.0

Release candidate for a public git-distributed Lattice release.

### Added

- Rust workspace layout with `lattice-core`, `lattice` CLI, and `xtask`.
- CLI management commands for services, include/exclude patterns, permissions,
  presets, repository operations, secret metadata, `track`, `adopt`, `diff`, and
  `tui`.
- Default per-service repo locations under `$XDG_DATA_HOME/lattice/repos`.
- Presets for `codex`, `git`, `zsh`, `mise`, and `ssh`.
- Restore safety checks, overwrite snapshots, symlink restore mode, OS/hostname
  conditions, and simple environment-variable template rendering.
- Dependency policy, typo scanning, unused dependency checks, LCOV generation,
  Docker-backed Linux verification, and GitHub Actions matrix verification.
- Public English/Korean documentation and English-only LLM workflow guidance.

### Changed

- Lattice is git-distributed only. The crates are marked `publish = false`.
- Release verification is centralized through `cargo run -p xtask -- verify`,
  `linux-verify`, and `quality`.
- `doctor` remains a lightweight environment check; config parsing lives in
  `validate`.

### Security

- Backups reject obvious secret-looking content unless explicitly bypassed.
- Secret commands store only metadata for `rbw` and `bw`; they do not read or
  print secret values.
- Path traversal, unsafe symlink, manifest escape, restore conflict, and binary
  diff exposure cases are covered by harness tests.

## v0.2.0

- Restore conflict detection and forced-restore snapshots.
- Minimal lifecycle hooks.
- Secret-looking content guard.
- `validate` and stronger real-Codex dry-run harness coverage.

## v0.1.0

- Initial Rust CLI spike for Codex-scoped backup and restore.
- XDG paths, TOML config, Codex preset, permission manifests, backup, restore,
  status, and the first Rust `xtask` verification harness.
