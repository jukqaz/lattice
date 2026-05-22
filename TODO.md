# Lattice MVP TODO

## Product Direction

- [x] Keep Lattice as the canonical dotfiles manager for this product line.
- [x] Treat earlier dotfiles-manager experiments as feature-mining sources, not
  parallel products.
- [x] Keep the core generic: service-scoped scan, plan, backup, restore, diff,
  and narrow lifecycle hooks.
- [x] Keep tool-specific behavior out of the product direction; app catalog
  entries are optional shortcuts, not the center of the manager.

## v0.1 Scope

- [x] Create a Rust CLI repository.
- [x] Write the MVP design spec.
- [x] Write the implementation plan.
- [x] Add config and service TOML models.
- [x] Add XDG path resolution.
- [x] Add `init`, `doctor`, `service list`, `backup`, and `restore` commands.
- [x] Add an initial catalog-backed example for early smoke coverage.
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
- [x] Add read-only dry-run coverage for a real local service root when available.
- [x] Add lightweight secret-looking content scan.
- [x] Add `validate` if it stays small.
- [x] Default omitted service repos to `$XDG_DATA_HOME/lattice/repos/<service>`.

## v0.3.x Safe Personal Backup

- [x] Add `service add`, `service show`, and `service remove`.
- [x] Add `include add/remove` and `exclude add/remove`.
- [x] Add `permission set/remove`.
- [x] Add git repo commands: `repo status`, `repo pull`, `repo commit`, `repo push`.
- [x] Add secret metadata with `rbw` and `bw` without materializing values.
- [x] Add a small initial catalog for common dotfile layouts.
- [x] Add `track` / `adopt` command.
- [x] Add TUI over the same core/config model.
- [x] Add optional symlink restore mode.
- [x] Add OS and hostname service conditions.
- [x] Add simple env-var template rendering on restore.
- [x] Add empty directory preservation.
- [x] Add portable path collision checks.
- [x] Add metadata-loss guards for hard links, xattrs, and macOS resource forks.
- [x] Tag the first regular personal-use release line as `v0.3.3`.

## v0.4.x Automation-Friendly CLI

- [x] Make `lattice init` generic by default: create config/storage only, not a tool-specific service.
- [x] Add richer `lattice tui --dry-run` dashboard output.
- [x] Keep TUI dashboard best-effort per service when a root or repo path is unavailable.
- [x] Add machine-readable JSON output for `status`.
- [x] Add machine-readable JSON output for `backup --dry-run`.
- [x] Add machine-readable JSON output for `diff`.
- [x] Add machine-readable JSON output for `restore --dry-run`.
- [x] Add `--only` and `--exclude` selectors for status, backup, diff, and restore flows.
- [x] Cover JSON and selector behavior in CLI smoke tests.
- [x] Document automation examples in README and user docs.
- [x] Treat the current main branch as the `v0.4.0` candidate once docs and verification are green.

## v0.5.x New Machine Bootstrap

- [x] Document a complete new-machine bootstrap flow:
  `install -> init -> app add/service add -> repo pull -> plan -> restore`.
- [ ] Improve first-run guidance after `init`.
- [x] Add `bootstrap check` with human and JSON output for new-machine readiness.
- [x] Add `plan` as the single human/JSON preflight surface before backup or restore.
- [ ] Add diagnostics for missing tools and disconnected repos without installing anything automatically.
- [ ] Make restore dry-run summaries easy to trust before the real restore.

## v0.6.x App Catalog And Diagnostics Polish

- [x] Replace the catalog command surface with `app` commands:
  `app list`, `app show <app>`, and `app add <app>`.
- [x] Remove old catalog wording and command surface outright.
- [ ] Keep app catalog entries optional and documented as shortcuts over the generic service model.
- [ ] Treat Codex as an example app only, not a default or product-defining path.
- [ ] Add deterministic, tool-agnostic diagnostics before adding any app-specific checks.
- [ ] Ensure no app or example becomes product-defining.

## v0.6.x+ Safety And Recovery Polish

- [ ] Expose snapshot/history commands for forced restore backups.
- [ ] Add `undo` or snapshot restore dry-run before any destructive rollback.
- [ ] Add safe snapshot pruning with `--dry-run` and conservative defaults.

## v0.6.x+ Discovery Polish

- [ ] Add `discover` to suggest service/app candidates from the local home directory.
- [ ] Keep discovery generic and conservative: exclude secrets, sessions, caches, databases, auth, and large files by default.
- [ ] Make discovery output machine-readable with `--json`.

## v0.7.x Service Groups

- [ ] Design a conservative service-group model before implementation.
- [ ] Add group status and group dry-run planning before any batch restore behavior.
- [ ] Keep group output machine-readable from the start.

## v1.0 Public Stable CLI

- [ ] Stabilize install, release, changelog, and migration notes.
- [ ] Decide crates.io publish policy.
- [ ] Add shell completions and polished help/manpage surfaces if they remain small.
- [ ] Add issue templates and a clear pre-1.0 change policy.
