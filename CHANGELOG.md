# Changelog

English | [한국어](CHANGELOG.ko.md) | [Documentation Index](docs/README.md)

## v0.3.3

### Fixed

- Reject service root/repo overlap before backup or restore to prevent recursive
  copies and self-restores.
- Reject tracked paths that are not portable UTF-8, contain control characters,
  or collide after Unicode normalization plus case-insensitive comparison.
- Reject hard-linked files, extended attributes, and macOS resource forks by
  default because copy backup does not preserve that metadata.

### Added

- `backup --allow-metadata-loss` and `adopt --allow-metadata-loss` for files
  that have been reviewed and can safely lose hard-link/xattr/resource-fork
  metadata in the backup copy.

## v0.3.2

### Fixed

- Snapshot special filesystem entries such as Unix sockets as metadata before a
  forced restore replaces them with tracked directories. This avoids treating
  non-regular files as copyable file contents.

### Changed

- Reword public docs so Lattice is presented as a generic service-scoped
  dotfiles manager. The concrete command examples use `codex` only as a sample
  service.

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

- Initial Rust CLI spike for service-scoped backup and restore with `codex` as
  the first concrete example.
- XDG paths, TOML config, `codex` preset, permission manifests, backup, restore,
  status, and the first Rust `xtask` verification harness.
