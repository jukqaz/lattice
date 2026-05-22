# Changelog

English | [한국어](CHANGELOG.ko.md) | [Documentation Index](docs/README.md)

## v0.4.0 - 2026-05-22

### Added

- `lattice app list/show/add` as the public app-catalog surface over ordinary
  service config.
- `lattice bootstrap check` with human and JSON output for new-machine readiness
  checks.
- `lattice plan` with human and JSON output as the preferred preflight surface
  before backup or restore.
- `lattice snapshot list/show/prune`, plus `lattice undo`, for inspecting forced
  restore snapshots, dry-running rollback, and pruning history conservatively.
- `lattice discover` with human and JSON output for conservative local service
  candidate discovery without mutating config.
- Product-surface verification in `cargo run -p xtask -- verify` so CLI help and
  maintained docs keep app/service terminology and do not drift back to the old
  catalog wording.

### Changed

- `lattice init` now creates generic Lattice config and storage directories
  without creating a tool-specific service by default, and prints the next safe
  bootstrap commands.
- README and user docs now start from app/service examples while keeping app
  entries as optional shortcuts, not the product center.
- Workspace package version is now `0.4.0` for the v0.4 release command
  surface.

### Removed

- Removed the old public catalog command/flag wording in favor of `app` and
  generic service config.

## v0.3.3

### Fixed

- Reject service root/repo overlap before backup or restore to prevent recursive
  copies and self-restores.
- Reject tracked paths that are not portable UTF-8, contain control characters,
  or collide after Unicode normalization plus case-insensitive comparison.
- Reject hard-linked files, extended attributes, and macOS resource forks by
  default because copy backup does not preserve that metadata.
- Treat unsupported xattr listing as non-fatal so filesystems without xattr
  support do not fail every backup.

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
  dotfiles manager. Concrete command examples are service examples, not product
  direction.

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
- `validate` and stronger isolated dry-run harness coverage.

## v0.1.0

- Initial Rust CLI spike for service-scoped backup and restore with an explicit
  example service.
- XDG paths, TOML config, `codex` preset, permission manifests, backup, restore,
  status, and the first Rust `xtask` verification harness.
