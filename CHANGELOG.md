# Changelog

English | [한국어](CHANGELOG.ko.md) | [Documentation Index](docs/README.md)

## Unreleased

### Changed

- Added `scripts/lint.sh` as a check-only local lint entrypoint for Rust, shell,
  and workflow maintenance, and formatted the real-HOME health script with the
  same shell style.

- CLI parser definitions now live in `crates/lattice-cli/src/cli.rs`, keeping the
  command surface pinned while reducing the size of the main command runner.
- CLI smoke-test helpers now live under `crates/lattice-cli/tests/support/` so
  future domain-specific smoke files can share the same isolated XDG harness.
- The CLI help-surface smoke now lives in `crates/lattice-cli/tests/help_surface.rs`,
  starting the domain-by-domain integration test split.
- Dedicated service-group smoke coverage now lives in
  `crates/lattice-cli/tests/group_surface.rs`, continuing the domain-by-domain
  split while the broader JSON contract smoke remains in the general suite.
- Dedicated snapshot/undo smoke coverage now lives in
  `crates/lattice-cli/tests/snapshot_surface.rs`, shrinking the general smoke
  suite while keeping recovery safety regressions isolated.
- Dedicated discovery and JSON-contract smoke coverage now lives in
  `crates/lattice-cli/tests/discover_surface.rs` and
  `crates/lattice-cli/tests/json_contract_surface.rs`, keeping the shared smoke
  suite focused on the remaining cross-command flows.
- The `discover` command runner now lives in
  `crates/lattice-cli/src/commands/discover.rs`, starting the focused command
  module split for the large CLI implementation.
- Dedicated bootstrap diagnostics and restore-plan trust smoke coverage now lives
  in `crates/lattice-cli/tests/bootstrap_surface.rs`, and the bootstrap command
  runner now lives in `crates/lattice-cli/src/commands/bootstrap.rs` with
  centralized service-report inspection.
- Dedicated service lifecycle smoke coverage now lives in
  `crates/lattice-cli/tests/service_lifecycle_surface.rs`, further shrinking
  the shared `cli_smoke.rs` suite while preserving the init/doctor/backup/restore
  end-to-end flow.
- Dedicated service config and secret-guard smoke coverage now lives in
  `crates/lattice-cli/tests/service_config_surface.rs`, isolating service CRUD,
  default repo fallback, and secret-looking content guard regressions.
- Dedicated adopt, repo, and diff smoke coverage now lives in
  `crates/lattice-cli/tests/adopt_repo_diff_surface.rs`, isolating happy-path
  and safety regressions from the shared `cli_smoke.rs` suite.
- Dedicated selector smoke coverage now lives in
  `crates/lattice-cli/tests/selector_surface.rs`, isolating `--only`/`--exclude`
  JSON behavior from the shared `cli_smoke.rs` suite.

### Fixed

- Release-state cleanup now records v0.7.0 acceptance as complete in `TODO.md`
  instead of leaving a stale pre-tag blocker after the `v0.7.0` release.
- Product-surface verification now rejects stale “Before tagging v0.7.0” wording
  so completed release acceptance cannot drift back into active TODOs.

## v0.7.0 - 2026-06-04

### Added

- Secret metadata now supports an `env` passthrough backend for API keys, tokens,
  and passwords that should stay outside Lattice repos and be rendered through
  `{{env:NAME}}` templates at restore time.
- `secret check` now reports env passthrough status without reading values and
  keeps the explicit `value=not-read` marker.

### Changed

- Workspace package version is now `0.7.0` for the env secret passthrough release line.
- README, user docs, product scope, TODO, install snippets, and product-surface
  verification now point at the `v0.7.0` release contract.

## v0.6.0 - 2026-06-03

### Added

- The JSON output reference now covers the documented automation surfaces beyond
  service groups: `bootstrap check --json`, single-service `status/plan`,
  dry-run backup/restore, `diff --json`, snapshot list/show/prune, and
  `undo --dry-run --json`.
- Fixture-based CLI smoke coverage now pins the top-level JSON keys for the
  stable automation contract across bootstrap, service, group, discovery,
  snapshot, and undo surfaces.
- `cargo run -p xtask -- release-check` now performs release-line preflight:
  version/doc/changelog consistency, locked metadata, path install, and installed
  binary smoke checks.

### Changed

- Workspace package version is now `0.6.0` for the automation-contract hardening
  release line.
- README, user docs, product scope, TODO, install snippets, and product-surface
  verification now point at the `v0.6.0` release contract.
- Service groups remain read-only in v0.6; automation hardening expands contracts
  without adding batch backup or restore behavior.

### Fixed

- `discover` now includes per-suggestion `next_command` hints and a top-level
  `next_actions` checklist in JSON and human output so first-adoption flows move
  from review to `plan` to `backup --dry-run` before any write.
- `discover` no longer suggests `app add` for app-named config directories or
  warning-only candidates when the discovered root and include set do not match
  the app catalog root contract; copyable hints stay review-first and
  service-scoped.

## v0.5.1 - 2026-05-26

### Fixed

- Restore and snapshot safety smoke tests now cover tampered manifests,
  destination symlink escapes, symlinked snapshot inputs, and partial-restore
  prevention at the CLI level.
- `discover` now reports suggestion-level warnings when conservative include and
  exclude patterns omit secret/auth/session/cache/database-looking material,
  including warning-only candidates where every file is excluded, instead of
  silently hiding those exclusions.
- `undo --dry-run` now runs the same snapshot restore preflight used by real
  undo before reporting success.
- The real HOME read-only health check no longer falls back to `cargo run`; use
  `LATTICE_BIN` or build `target/debug/lattice` first to avoid build/cache side
  effects during live-HOME dogfood.
- First-adoption docs now explicitly say: Do not run restore first on a real HOME;
  start from read-only discovery, planning, and one reviewed backup.
- CI now runs an explicit `wasm32-wasip2` non-Unix `lattice-core` compile check
  so platform-surface regressions are not hidden behind host-only tests.

### Changed

- Workspace package version is now `0.5.1` for the hardening patch release.
- Product-surface verification now pins the v0.5.1 release docs, install
  snippets, changelog, and CI target-coverage contract.

## v0.5.0 - 2026-05-26

### Added

- Service groups in global config, keeping groups as conservative named bundles
  of existing services.
- `lattice group list/show/status/plan` with human-readable output for read-only
  multi-service inspection and planning.
- Machine-readable JSON output for all group commands, including aggregate status
  and plan summaries.
- Path selectors on `group status` and `group plan` so automation can narrow the
  per-service status/plan view without introducing batch mutation.

### Changed

- Workspace package version is now `0.5.0` for the service-groups release line.
- `lattice validate` now rejects ambiguous or broken group config: duplicate
  group names, empty groups, unknown service references, and duplicate service
  members.
- Group aggregate status and plan totals now count active services only while
  retaining inactive members as skipped per-service rows in JSON.
- Group plan JSON now reports `conflict_count` plus service-keyed `conflicts`
  objects instead of overloading `conflicts` as a scalar.
- Human `group status` output now shows `root_exists` so missing roots are
  distinguishable from empty included file sets.

### Fixed

- `bootstrap check` now preserves service-root inspection I/O errors with
  `Path::try_exists()` instead of reporting every inaccessible path as a missing
  root.

### Not Included

- Group backup, group restore, and other batch mutation flows remain out of
  scope until the read-only group status/plan surface is proven safe.

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

- The CLI now prints actionable help for app, bootstrap, plan, snapshot, undo,
  discover, and group surfaces.

## v0.3.2

### Fixed

- Service config parsing and path handling now reject additional malformed input
  before backup or restore.

## v0.3.1

### Fixed

- Restore safety checks now avoid following unsafe paths during conflict
  detection.

## v0.3.0

### Added

- Initial service-oriented backup, restore, diff, status, repo, include, exclude,
  permission, hook, and validation surfaces.
