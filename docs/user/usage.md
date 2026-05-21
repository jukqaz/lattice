# User Guide

English | [한국어](usage.ko.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

This guide is for people using Lattice directly. LLM-specific workflow rules
live under [docs/llm](../llm/).

## What Lattice Manages

Lattice backs up and restores selected files for named services. A service is
one tool or app configuration root. The service names in this guide are generic
examples; use the names and paths that match your own dotfiles.

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

Terminology: an **app** is a common managed target, such as `git`, `ssh`,
`zsh`, `starship`, `mise`, or `codex`. An app catalog entry is only a shortcut
for creating ordinary service config. No app is product-defining, and Codex is
only one example app. The CLI should use `lattice app ...` directly rather than
preserving older preset terminology.

## 1. Install

Install the latest tagged release:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.3 --locked
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

Create the global config and storage directories:

```bash
lattice init
```

Check the environment:

```bash
lattice doctor
lattice validate
lattice service list
```

The default config files are stored here:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
```

When a service omits `repo`, Lattice stores its backup copy here:

```text
~/.local/share/lattice/repos/<service-name>
```

## 3. Add A First Service

Create a service for one app config directory. Replace the example name, root,
and include pattern with the dotfiles you want to manage:

```bash
lattice service add shell --root ~/.config/shell --include config.toml
lattice service show shell
lattice status shell
```

Add or remove tracked paths later:

```bash
lattice include add shell scripts/**
lattice include remove shell scripts/**
lattice exclude add shell cache/**
lattice exclude remove shell cache/**
```

Preserve restore permissions:

```bash
lattice permission set shell config.toml 0600
lattice permission remove shell config.toml
```

Import existing files into a service:

```bash
lattice track shell config.toml
lattice adopt shell scripts/sync
```

## 4. Create The First Backup

Always run a dry-run first:

```bash
lattice backup --dry-run shell
```

The output lists files that would be copied and empty directories that would be
tracked. Review this list before writing to the service repo.

Create the backup:

```bash
lattice backup shell
```

Check for drift between the live service root and the repo copy:

```bash
lattice diff shell
```

No output means there is no file-content drift for the tracked files.

## 5. Restore Safely

Preview restore work first:

```bash
lattice restore --dry-run shell
```

Restore without overwriting conflicts:

```bash
lattice restore shell
```

If local files conflict and you intentionally want the repo copy to win, use
`--force`:

```bash
lattice restore --force shell
```

Forced restore snapshots overwritten files under XDG state before writing:

```text
~/.local/state/lattice/snapshots/
```

## 6. Sync The Backup Repo

Service repos are normal directories. Use Git directly or use Lattice helpers:

```bash
lattice repo status shell
lattice repo pull shell
lattice repo commit --message "backup shell config" shell
lattice repo push shell
```

For a private GitHub repository:

1. Create the private remote repository.
2. Add it as the remote for the service repo.
3. Run `lattice repo push <service>` after each backup commit.

Lattice does not create remotes automatically. That keeps credentials, access
control, and repository ownership explicit.

## 7. Use The App Catalog

App catalog entries provide known include/exclude shapes for common tools and
apps:

```bash
lattice app list
lattice app show <app>
```

Create an app-backed service:

```bash
lattice app add <app> --root <path>
```

Apps are optional shortcuts. The core model remains the same service config,
include/exclude rules, backup, diff, and restore flow.

## 8. Manage Secrets Safely

Lattice does not back up secret values. Secret commands store metadata such as
backend, item, field, environment variable name, and folder.

Add secret metadata:

```bash
lattice secret add --backend rbw --item "<vault item>" --field password --env <ENV_NAME> <service> <name>
```

List and check metadata:

```bash
lattice secret list <service>
lattice secret check <service>
```

`secret check` verifies tool availability for backends such as `rbw` and `bw`
without reading or printing secret values.

Backups also block obvious secret-looking file contents by default. Use
`--allow-secret-looking-files` only after reviewing the affected files:

```bash
lattice backup --allow-secret-looking-files <service>
```

Backup scans regular files and included empty directories. It does not follow
symlinks or copy sockets, FIFOs, device files, and other special filesystem
entries as file content.

Service roots and repos must not overlap. Tracked paths must be portable UTF-8,
must not contain control characters, and must not collide after Unicode
normalization plus case folding. This prevents silent data loss when moving
backups between case-sensitive Linux filesystems and case-insensitive macOS
filesystems.

Copy backup does not preserve hard-link relationships, extended attributes, or
macOS resource forks. Lattice blocks those files by default. Use
`--allow-metadata-loss` only after reviewing the affected files:

```bash
lattice backup --allow-metadata-loss <service>
```

## 9. Advanced Restore Options

Use symlink restore mode when you want restored files to point into the service
repo:

```bash
lattice service add <service> --root <path> --include <pattern> --symlink
```

Use template mode when repo files should contain placeholders and restored
files should render environment variables:

```bash
lattice service add <service> --root <path> --include <pattern> --template
```

When symlink and template modes are both enabled, files with rendered template
values are restored as regular files so the repo copy keeps placeholders.

Use OS or hostname conditions to make a service active only on matching
machines:

```bash
lattice service add <service> --root <path> --preset <preset> --os macos
```

## 10. Automation And JSON Output

Use `--json` when scripts, CI jobs, or agents need stable machine-readable
output:

```bash
lattice status --json shell
lattice backup --dry-run --json shell
lattice diff --json shell
lattice restore --dry-run --json shell
```

For write flows, prefer the dry-run JSON command first. Parse the plan and stop
if unexpected files, directories, entries, or conflicts appear:

```bash
plan="$(lattice restore --dry-run --json shell)"
printf '%s\n' "$plan" | jq '.conflicts'
```

Use `--only` and `--exclude` to narrow status, backup, diff, and restore work to
specific tracked paths. Quote glob selectors so the shell does not expand them:

```bash
lattice status --json --only config.toml shell
lattice backup --dry-run --json --only config.toml shell
lattice diff --json --exclude 'cache/**' shell
lattice restore --dry-run --json --only config.toml shell
```

The selectors are intentionally path-scoped, not service-group orchestration.
Use them for small, reviewable operations such as backing up one changed config
file or excluding noisy generated state from a diff.

## 11. Prompt UI

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
lattice service show shell
lattice status shell
```

If backup fails because of secret-looking content, inspect the file before
deciding whether `--allow-secret-looking-files` is appropriate.

If restore reports conflicts, run:

```bash
lattice restore --dry-run shell
lattice diff shell
```

Use `restore --force` only when you have confirmed the repo copy should replace
the local copy.

If a forced restore replaces a special filesystem entry, Lattice writes metadata
about that entry into the snapshot instead of trying to copy it as regular file
contents.

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
