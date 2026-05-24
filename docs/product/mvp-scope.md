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

## Current Baseline: v0.3.3

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
- Secret metadata commands for `rbw` and `bw` without reading secret values.
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

## Main Branch After v0.3.3

The current tagged release is v0.4.0. It adds automation-friendly surfaces on
top of the safe personal backup baseline:

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

## Roadmap

| Line | Name | Goal | Acceptance |
| --- | --- | --- | --- |
| `v0.3.x` | Safe Personal Backup | Safely back up and restore personal dotfiles. | Full safety harness, platform CI, install smoke, and v0.3.3 tag smoke pass. |
| `v0.4.x` | Automation, Bootstrap, Recovery, And Discovery | Let scripts and agents call Lattice without parsing human stdout, then make new-machine restore, recovery history, and conservative discovery first-class. | Generic init, JSON output, selectors, `plan`, `bootstrap check`, `app` commands, snapshot/undo, `discover`, and product-surface harness coverage are documented and tested in the v0.4.0 release line. |
| `v0.5.x` | Service Groups | Inspect and plan related services together without introducing batch mutation. | `group list/show/status/plan`, JSON output, and selectors are documented and tested before any group backup/restore behavior. |
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
