# Lattice Product Scope

English | [한국어](mvp-scope.ko.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

## Product Positioning

Lattice is the canonical dotfiles manager for this product line: a small Rust
CLI that manages service-scoped files and directories with explicit TOML
configuration, predictable XDG storage, permission preservation, and safe
restore behavior.

Historical dotfiles-manager experiments should feed proven generic ideas into
Lattice instead of becoming parallel products. The core product should stay
small: scan, plan, backup, restore, diff, and run narrowly configured lifecycle
hooks.

Lattice is not a full system configuration manager, package manager, or secret
manager. It should not be shaped around one specific tool. Product-facing
language calls common managed targets **apps**. App knowledge belongs in an
optional app catalog only when it improves the generic dotfile-management
workflow. Codex is one example app, not the product center.

## Released Baseline: v0.3.3

v0.3.3 is the first release line intended for regular personal use. It includes
the v0.2 safety layer, the first CLI-first management layer, empty directory
preservation, and additional portable filesystem safety checks.

Released v0.3.3 scope:

- Rust workspace split into `lattice-core`, `lattice` CLI, and `xtask`.
- XDG-aware config, data, state, and cache paths.
- Global TOML config and per-service TOML config.
- Service root and optional repo path.
- Default service repos at `$XDG_DATA_HOME/lattice/repos/<service>`.
- Include and exclude globs.
- Optional app catalog entries for common dotfile layouts.
- `init`, `doctor`, `validate`, `service list/show/add/remove`, and `status`.
- `include add/remove`, `exclude add/remove`, and `permission set/remove`.
- `backup`, `backup --dry-run`, `restore`, `restore --dry-run`, and
  `restore --force`.
- Empty directory tracking in backup manifests and restore.
- Permission manifest capture and restore.
- Restore conflict detection and XDG state snapshots before overwrite.
- Restore-time secure directory creation.
- Minimal lifecycle hooks with confirmation and timeout support.
- Secret-looking content guard before backup.
- Portable path collision checks for case-insensitive and Unicode-normalized
  names.
- Root/repo overlap rejection.
- Metadata-loss guard for hard links, extended attributes, and macOS resource
  forks, with explicit `--allow-metadata-loss` bypass.
- Secret metadata commands for `rbw`, `bw`, and env passthrough references
  without reading secret values.
- Git repo commands: `repo status/pull/commit/push`.
- `track` and `adopt` for importing existing files into a service.
- `diff` with binary redaction and template-aware output.
- Optional symlink restore mode.
- OS and hostname service conditions.
- Simple environment-variable template rendering on restore.
- Prompt-based TUI over the same config model.
- Rust-only `xtask` verification, Linux Docker verification, and quality gates.
- GitHub Actions for Linux x86_64, Linux ARM64, macOS Apple Silicon, and
  dependency/coverage/typo quality checks.

## Released Automation Line: v0.4.0

v0.4.0 adds automation-friendly surfaces on top of the safe personal backup
baseline:

- `lattice init` creates generic Lattice config and storage directories without
  creating a tool-specific service by default.
- Richer `lattice tui --dry-run` dashboard with per-service status, file counts,
  root paths, repo paths, and action summaries.
- Best-effort TUI dashboard behavior: one service with an unavailable root or
  repo no longer prevents other services from being listed.
- Machine-readable JSON output for `status`, `plan`, `backup --dry-run`,
  `diff`, `restore --dry-run`, `bootstrap check`, `snapshot`, `undo`, and
  `discover`.
- `plan` as the single human/JSON preflight surface before backup or restore.
- `bootstrap check` for new-machine readiness diagnostics.
- `app list`, `app show <app>`, and `app add <app>` as the app catalog command
  surface.
- `snapshot list/show/prune` and `undo` for forced-restore history inspection,
  dry-run rollback, and conservative cleanup.
- `discover` for conservative local service suggestions without config mutation.
- `--only` and `--exclude` path selectors for status, plan, backup, diff, and
  restore flows.
- CLI smoke and product-surface harness coverage for the JSON, selector,
  app-catalog, and bootstrap contracts.

## Current Release: v0.8.1

v0.8.1 is the post-review patch release for the v0.8 maintainability and
modularization line. It keeps the v0.8.0 CLI behavior intact while shipping the
review follow-up that scopes release-check changelog validation to the requested
release section.

v0.8.1 scope:

- Release-check changelog assertions read only the requested release section, so
  older release notes cannot satisfy the current release contract.
- Version metadata, install snippets, changelogs, product scope, TODO, and
  product-surface harness expectations are aligned to the v0.8.1 patch release.
- The v0.8.0 modularized command surface remains the current user-facing CLI
  contract.

## Previous Release: v0.8.0

v0.8.0 completes the maintainability and modularization line. It keeps the CLI
behavior stable while splitting the parser, smoke harnesses, command runners,
output helpers, config storage, runtime probes, and shared service state into
focused files that are easier to review and extend.

v0.8.0 scope:

- CLI parser definitions and command runners live in focused modules instead of
  a large `main.rs` runner.
- Domain smoke coverage is split into focused integration tests that reuse the
  shared isolated XDG support harness.
- Shared JSON/human output helpers, config persistence, runtime probes, service
  state, setup commands, service CRUD, and backup/restore/status/diff flows are
  split into dedicated modules.
- `scripts/lint.sh`, `cargo run -p xtask -- verify`, and
  `cargo run -p xtask -- quality` cover the release-line quality gate.
- README, user docs, product scope, TODO, changelog, and product-surface harness
  expectations are aligned to the v0.8.0 release line.

## Earlier Release: v0.7.0

v0.7.0 adds env secret passthrough metadata while preserving Lattice's role as a
dotfiles/config manager rather than a secret manager. API keys, tokens, and
passwords stay outside the repo as environment-variable references and restore
templates.

v0.7.0 scope:

- Secret metadata commands support `env` passthrough references alongside `rbw`
  and `bw`, storing environment-variable names instead of secret values.
- `secret check` reports env references as `set`, `unset`, or
  `missing-env-reference` while keeping `value=not-read`.
- README, user docs, product scope, TODO, changelog, and product-surface harness
  expectations are aligned to the v0.7.0 release line.

## Earlier Release: v0.6.0

v0.6.0 hardens the automation contract across the existing command surface. It
expands documented JSON shapes, pins fixture-based contract coverage, and adds a
release-check helper without adding batch mutation or remote bootstrap behavior.

v0.6.0 scope:

- The service-groups read-only model from v0.5 remains intact: `group list`,
  `group show`, `group status`, and `group plan` only.
- JSON output reference coverage expands beyond groups to `bootstrap check`,
  single-service `status`/`plan`, dry-run `backup`/`restore`, `diff`, snapshot
  list/show/prune, `undo --dry-run`, and `discover`.
- Fixture-based CLI smoke coverage pins top-level JSON keys and important nested
  field names for bootstrap, service, group, discovery, snapshot, and undo
  automation surfaces.
- `discover` exposes top-level `next_actions` and suggestion-level
  `next_command` while keeping app-named discovery hints service-scoped unless
  the app catalog root/include contract is explicitly reviewed.
- `cargo run -p xtask -- release-check` verifies version metadata, changelog and
  install snippets, locked metadata, path install, and installed binary smoke
  before tagging.
- README, user docs, product scope, TODO, changelog, and product-surface harness
  expectations are aligned to the v0.6.0 release line.

Group backup, group restore, other batch mutation flows, automatic remote repo
creation, package installation, MCP prototypes, and crates.io publish remain
intentionally out of scope.

## Roadmap

| Line | Name | Goal | Acceptance |
| --- | --- | --- | --- |
| `v0.3.x` | Safe Personal Backup | Safely back up and restore personal dotfiles. | Full safety harness, platform CI, install smoke, and v0.3.3 tag smoke pass. |
| `v0.4.x` | Automation, Bootstrap, Recovery, And Discovery | Let scripts and agents call Lattice without parsing human stdout, then make new-machine restore, recovery history, and conservative discovery first-class. | Generic init, JSON output, selectors, `plan`, `bootstrap check`, `app` commands, snapshot/undo, `discover`, and product-surface harness coverage are documented and tested in the v0.4.0 release line. |
| `v0.5.x` | Service Groups | Inspect and plan related services together without introducing batch mutation. | `group list/show/status/plan`, JSON output, selectors, group invariant validation, active-only aggregates, and missing-root visibility are documented and tested before any group backup/restore behavior. |
| `v0.6.x` | Automation Contract Hardening | Make existing machine-readable surfaces trustworthy for scripts and agents. | JSON reference coverage, fixture-based contract tests, release-check automation, and release docs are aligned without adding batch mutation. |
| `v0.7.x` | Secret Passthrough References | Keep secret values out of repos while making env references explicit and checkable. | `env` secret metadata, restore-time `{{env:NAME}}` guidance, non-disclosure smoke coverage, and bilingual docs are aligned. |
| `v0.8.x` | Maintainability And Modularization | Make the pre-1.0 CLI easier to review and extend without behavior churn. | Parser, command modules, smoke domains, output helpers, release docs, and xtask structure contracts are aligned. |
| `v1.0` | Public Stable CLI | Make Lattice recommendable to external users. | Install, changelog, release, migration, change policy, and issue workflows are stable. |

## Deliberate Non-Goals

- crates.io publish before the public stable line.
- Automatic remote repository creation.
- Automatic package installation.
- Secret value materialization from `rbw` or `bw`.
- Full plugin system.
- Home Manager or Nix-style declarative program modules.
- GUI.
- Database-backed state.
- Tool-specific product features in the generic dotfile manager.

## Configuration Shape

Service config should remain readable TOML:

```toml
name = "shell"
root = "~/.config/shell"
include = ["config.toml", "scripts/**"]
exclude = ["cache/**", "state/**"]

[restore]
create_dirs = [
  { path = "cache", mode = "0700" },
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[hooks.after_restore]]
name = "reload shell config"
command = "/bin/sh"
args = ["-c", "true"]
timeout_sec = 60
confirm = false
```

When `repo` is omitted, Lattice resolves it to
`$XDG_DATA_HOME/lattice/repos/<service-name>`. Set `repo` only when a service
needs a custom repository location.

## Release Acceptance

Every release candidate is release-ready when:

- `cargo run -p xtask -- verify` passes.
- `cargo run -p xtask -- linux-verify` passes for release-oriented changes.
- `cargo run -p xtask -- quality` passes.
- `actionlint .github/workflows/ci.yml` passes when workflows change.
- `git diff --check` passes.
- Path install smoke passes with `cargo install --path crates/lattice-cli`.
- GitHub Actions passes on Linux x86_64, Linux ARM64, macOS Apple Silicon, and
  the quality job.
- The tag install smoke passes after a release tag is pushed.
