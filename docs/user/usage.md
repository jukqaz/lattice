# User Guide

English | [한국어](usage.ko.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

This guide is for people using Lattice directly. LLM-specific workflow rules
live under [docs/llm](../llm/).

## What Lattice Manages

Lattice backs up and restores selected files for a named service. A service is
one tool or app, such as `codex`, `zsh`, `git`, or a custom app config folder.

Each service can define:

- a root directory
- files and directories to include
- paths to exclude
- permissions to preserve on restore
- optional OS and hostname conditions
- an optional Git repo location
- optional restore hooks

Lattice is not a package manager, secret manager, or full system configuration
manager. It keeps the dotfile sync layer small and explicit.

## 1. Install

Install the latest tagged release:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.1 --locked
```

Install from a local checkout while developing Lattice:

```bash
cargo install --path crates/lattice-cli
```

Confirm the installed binary:

```bash
lattice --version
```

## 2. Initialize Local Config

Create the global config and the default `codex` service:

```bash
lattice init
```

Check the environment and configured services:

```bash
lattice doctor
lattice validate
lattice service list
lattice status codex
```

The default files are stored here:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/codex.toml
```

If the Codex service omits `repo`, Lattice stores its backup copy here:

```text
~/.local/share/lattice/repos/codex
```

## 3. Create The First Backup

Always run a dry-run first:

```bash
lattice backup --dry-run codex
```

The output lists files that would be copied and empty directories that would be
tracked. Review this list before writing to the service repo.

Create the backup:

```bash
lattice backup codex
```

Check for drift between the live service root and the repo copy:

```bash
lattice diff codex
```

No output means there is no file-content drift for the tracked files.

## 4. Restore Safely

Preview restore work first:

```bash
lattice restore --dry-run codex
```

Restore without overwriting conflicts:

```bash
lattice restore codex
```

If local files conflict and you intentionally want the repo copy to win, use
`--force`:

```bash
lattice restore --force codex
```

Forced restore snapshots overwritten files under XDG state before writing:

```text
~/.local/state/lattice/snapshots/
```

## 5. Sync The Backup Repo

Service repos are normal directories. Use Git directly or use Lattice helpers:

```bash
lattice repo status codex
lattice repo pull codex
lattice repo commit --message "backup codex config" codex
lattice repo push codex
```

For a private GitHub repository:

1. Create the private remote repository.
2. Add it as the remote for the service repo.
3. Run `lattice repo push <service>` after each backup commit.

Lattice does not create remotes automatically. That keeps credentials, access
control, and repository ownership explicit.

## 6. Add Another Service

Create a service for one app config directory:

```bash
lattice service add editor --root ~/.config/editor --include settings.toml --include 'themes/**'
lattice service show editor
lattice backup --dry-run editor
```

Add or remove tracked paths later:

```bash
lattice include add editor keybindings.toml
lattice include remove editor themes/old/**
lattice exclude add editor cache/** state/**
lattice exclude remove editor state/**
```

Preserve restore permissions:

```bash
lattice permission set editor settings.toml 0600
lattice permission remove editor settings.toml
```

Import existing files into a service:

```bash
lattice track editor settings.toml themes/**
lattice adopt editor settings.toml
```

## 7. Use Presets

Presets provide known include/exclude shapes for common tools:

```bash
lattice preset list
lattice preset show codex
lattice preset show zsh
```

Create a preset-backed service:

```bash
lattice service add shell --root ~ --preset zsh --os macos
```

The built-in presets are `codex`, `git`, `zsh`, `mise`, and `ssh`.

## 8. Manage Secrets Safely

Lattice does not back up secret values. Secret commands store metadata such as
backend, item, field, environment variable name, and folder.

Add secret metadata:

```bash
lattice secret add --backend rbw --item "Editor API" --field password --env EDITOR_API_TOKEN editor api-token
```

List and check metadata:

```bash
lattice secret list editor
lattice secret check editor
```

`secret check` verifies tool availability for backends such as `rbw` and `bw`
without reading or printing secret values.

Backups also block obvious secret-looking file contents by default. Use
`--allow-secret-looking-files` only after reviewing the affected files:

```bash
lattice backup --allow-secret-looking-files editor
```

## 9. Advanced Restore Options

Use symlink restore mode when you want restored files to point into the service
repo:

```bash
lattice service add linked --root ~/.config/tool --include config.toml --symlink
```

Use template mode when repo files should contain placeholders and restored
files should render environment variables:

```bash
lattice service add templated --root ~/.config/tool --include config.toml --template
```

When symlink and template modes are both enabled, files with rendered template
values are restored as regular files so the repo copy keeps placeholders.

Use OS or hostname conditions to make a service active only on matching
machines:

```bash
lattice service add shell --root ~ --preset zsh --os macos
```

## 10. Prompt UI

Open the prompt-based UI:

```bash
lattice tui
```

The TUI uses the same config model as the CLI. It is a convenience layer, not a
separate source of truth.

## Troubleshooting

Validate config first:

```bash
lattice validate
```

Inspect a service:

```bash
lattice service show codex
lattice status codex
```

If backup fails because of secret-looking content, inspect the file before
deciding whether `--allow-secret-looking-files` is appropriate.

If restore reports conflicts, run:

```bash
lattice restore --dry-run codex
lattice diff codex
```

Use `restore --force` only when you have confirmed the repo copy should replace
the local copy.

## Development Verification

For local development:

```bash
cargo run -p xtask -- verify
```

For Docker-backed Linux verification:

```bash
cargo run -p xtask -- linux-verify
```

For the heavier release-oriented quality gate:

```bash
cargo run -p xtask -- quality
```
