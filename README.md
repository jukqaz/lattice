# Lattice

English | [한국어](README.ko.md)

Lattice is a small Rust CLI for backing up and restoring dotfiles by service.
It is designed for personal configuration repos where each tool can have its
own root, include rules, restore permissions, and optional sync repository.

The examples below use the built-in `codex` service so the commands stay
concrete. The same workflow applies to any service you define.

## Start Here

Install the latest tagged release:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.3 --locked
```

Initialize local config and inspect the example service:

```bash
lattice init
lattice doctor
lattice validate
lattice status codex
```

Preview the first backup before writing anything:

```bash
lattice backup --dry-run codex
```

If the plan looks right, create the backup:

```bash
lattice backup codex
```

## Restore Safely

Always preview restore changes first:

```bash
lattice restore --dry-run codex
```

Apply the restore when there are no unexpected conflicts:

```bash
lattice restore codex
```

If you intentionally want to overwrite local files, use `--force`. Forced
restores snapshot overwritten files under XDG state before writing repo
contents.

```bash
lattice restore --force codex
```

## Add Another Service

Lattice manages configuration per service. A service has a root directory,
include/exclude rules, permissions, and an optional repo path. If `repo` is not
set, Lattice stores it at `$XDG_DATA_HOME/lattice/repos/<service-name>`.

```bash
lattice service add <service> --root <path> --include <pattern>
lattice service show <service>
lattice backup --dry-run <service>
lattice backup <service>
```

Use presets when the common shape is already known:

```bash
lattice preset list
lattice preset show codex
lattice service add <service> --root <path> --preset <preset>
```

## Daily Commands

| Goal | Command |
| --- | --- |
| Check installation and configured tools | `lattice doctor` |
| Validate config files | `lattice validate` |
| See one service | `lattice status codex` |
| Preview backup | `lattice backup --dry-run codex` |
| Backup now | `lattice backup codex` |
| Preview restore | `lattice restore --dry-run codex` |
| Restore now | `lattice restore codex` |
| Compare local files with repo copy | `lattice diff codex` |
| Open the prompt-based UI | `lattice tui` |

## Sync With Git

Service repos are plain directories, so you can manage Git yourself or use the
built-in helpers:

```bash
lattice repo status codex
lattice repo pull codex
lattice repo commit --message "backup codex config" codex
lattice repo push codex
```

For a private GitHub repo, create the remote yourself, then point the service
repo at it with normal `git remote` commands.

## Safety Model

- Secret-looking file contents are blocked during backup unless
  `--allow-secret-looking-files` is passed.
- Secret commands store metadata only. They do not read, print, or back up
  secret values.
- Restore refuses conflicting local files unless `--force` is passed.
- Forced restore creates a snapshot before overwriting files.
- Restore paths, manifest entries, and permission rules must stay inside the
  service root.
- Service roots and service repos must not overlap.
- Tracked paths must be portable UTF-8, must not contain control characters,
  and must not collide after Unicode normalization plus case-insensitive comparison.
- Source symlinks and symlinked destination parents are rejected for restore
  safety.
- Backup tracks regular files and included empty directories. Symlinks, sockets,
  FIFOs, and other special filesystem entries are not followed as file content.
- Backup rejects hard-linked files, extended attributes, and macOS resource
  forks by default because copy backup does not preserve that metadata. Use
  `--allow-metadata-loss` only after reviewing the affected files.
- Forced restore snapshots existing special filesystem entries as metadata before
  replacing them with tracked directories.

## Config Locations

Lattice uses XDG locations:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
~/.local/share/lattice/repos/*
~/.local/state/lattice/
~/.cache/lattice/
```

Environment overrides are supported through `XDG_CONFIG_HOME`,
`XDG_DATA_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME`.

## Service Config Example

```toml
name = "codex"
root = "~/.codex"
preset = "codex"

[restore]
create_dirs = [
  { path = "shell_snapshots", mode = "0700" },
  { path = "bin", mode = "0755" },
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[permissions]]
path = "bin/mcp-rbw"
mode = "0700"
```

Use `lattice service add`, `lattice include`, `lattice exclude`,
`lattice permission`, `lattice secret`, and `lattice track` to manage service
TOML without editing files by hand.

## Documentation

Read these in order if you are new:

1. [User Guide](docs/user/usage.md)
2. [Product Scope](docs/product/mvp-scope.md)
3. [Changelog](CHANGELOG.md)
4. [Documentation Index](docs/README.md)

Korean versions:

1. [사용자 가이드](docs/user/usage.ko.md)
2. [제품 범위](docs/product/mvp-scope.ko.md)
3. [변경 로그](CHANGELOG.ko.md)
4. [문서 인덱스](docs/README.ko.md)

LLM-only agent guidance is kept separately under [docs/llm](docs/llm/).

## Development

Install from this checkout:

```bash
cargo install --path crates/lattice-cli
```

Run the normal verification harness:

```bash
cargo run -p xtask -- verify
```

Run Docker-backed Linux verification:

```bash
cargo run -p xtask -- linux-verify
```

Run the heavier release-oriented quality gate:

```bash
cargo run -p xtask -- quality
```

Workspace layout:

```text
crates/lattice-core/  core config, scan, backup, restore, hook, and path logic
crates/lattice-cli/   `lattice` binary and CLI smoke tests
xtask/                development verification harness
```
