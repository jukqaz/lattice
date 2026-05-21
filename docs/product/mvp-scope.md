# Lattice Product Scope

English | [한국어](mvp-scope.ko.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

## Product Positioning

Lattice is a lightweight Rust dotfiles manager. It manages service-scoped files
and directories with explicit TOML configuration, predictable XDG storage,
permission preservation, and safe restore behavior.

It is not a full system configuration manager, package manager, or secret
manager. The core product should stay small: scan, plan, backup, restore, and
run narrowly configured lifecycle hooks.

## Current Release: v0.3.3

v0.3.3 is the first release line intended for regular personal use. It includes
the v0.2 safety layer, the first CLI-first management layer, empty directory
preservation, and additional portable filesystem safety checks.

Current scope:

- Rust workspace split into `lattice-core`, `lattice` CLI, and `xtask`.
- XDG-aware config, data, state, and cache paths.
- Global TOML config and per-service TOML config.
- Service root and optional repo path.
- Default service repos at `$XDG_DATA_HOME/lattice/repos/<service>`.
- Include and exclude globs.
- Presets for `codex`, `git`, `zsh`, `mise`, and `ssh`.
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

## Deliberate Non-Goals

- crates.io publish.
- Automatic remote repository creation.
- Automatic package installation.
- Secret value materialization from `rbw` or `bw`.
- Full plugin system.
- Home Manager or Nix-style declarative program modules.
- GUI.
- Database-backed state.

## Configuration Shape

Service config should remain readable TOML:

```toml
name = "codex"
root = "~/.codex"
preset = "codex"

[restore]
create_dirs = [
  { path = "shell_snapshots", mode = "0700" },
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[hooks.after_restore]]
name = "codex doctor"
command = "codex"
args = ["doctor", "--summary"]
timeout_sec = 60
confirm = false
```

When `repo` is omitted, Lattice resolves it to
`$XDG_DATA_HOME/lattice/repos/<service-name>`. Set `repo` only when a service
needs a custom repository location.

## Release Acceptance

v0.3.3 is release-ready when:

- `cargo run -p xtask -- verify` passes.
- `cargo run -p xtask -- linux-verify` passes.
- `cargo run -p xtask -- quality` passes.
- `actionlint .github/workflows/ci.yml` passes.
- `git diff --check` passes.
- Path install smoke passes with `cargo install --path crates/lattice-cli`.
- GitHub Actions passes on Linux x86_64, Linux ARM64, macOS Apple Silicon, and
  the quality job.
- The tag install smoke passes after `v0.3.3` is pushed.
