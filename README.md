# Lattice

English | [한국어](README.ko.md)

Lattice is a small Rust CLI for backing up and restoring dotfiles by service.
It is designed for personal configuration repos where each tool can have its
own root, include rules, restore permissions, and optional sync repository.

Lattice is generic: no single tool or service is the product center. Create the
services you want to manage, then back up and restore them through the same
safe workflow.

Product language uses **apps** for common managed targets such as `git`, `ssh`,
`zsh`, `starship`, `mise`, or `codex`. Apps are not product centers; they are
catalog entries that expand into ordinary service config. The CLI should name
this surface directly as `app`.

## Start Here

Install the current v0.4 candidate command surface documented below:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --branch main --locked
```

The latest tagged stable baseline is still `v0.3.3`; use the current branch or
a local checkout when testing `app`, `plan`, and `bootstrap check` before the
v0.4.0 tag is cut.

Initialize local config and check whether the machine is ready for managed
config restores:

```bash
lattice init
lattice doctor
lattice validate
lattice bootstrap check
```

Add a first app-backed service when the common shape is already known. Replace
`zsh` and `~/.config/zsh` with the app config you want to manage:

```bash
lattice app list
lattice app show zsh
lattice app add zsh --root ~/.config/zsh
lattice plan zsh
```

If the plan looks right, create the first backup:

```bash
lattice backup zsh
```

## Restore Safely

Always inspect the restore plan before writing anything:

```bash
lattice plan zsh
lattice restore --dry-run zsh
```

Apply the restore when there are no unexpected conflicts:

```bash
lattice restore zsh
```

If you intentionally want to overwrite local files, use `--force`. Forced
restores snapshot overwritten files under XDG state before writing repo
contents.

```bash
lattice restore --force zsh
```

## Add More Services

Lattice manages configuration per service. A service has a root directory,
include/exclude rules, permissions, and an optional repo path. If `repo` is not
set, Lattice stores it at `$XDG_DATA_HOME/lattice/repos/<service-name>`.

```bash
lattice service add <service> --root <path> --include <pattern>
lattice service show <service>
lattice backup --dry-run <service>
lattice backup <service>
```

Use the app catalog when the common shape is already known:

```bash
lattice app list
lattice app show <app>
lattice app add <app> --root <path>
```

## Daily Commands

| Goal | Command |
| --- | --- |
| Check installation and configured tools | `lattice doctor` |
| Check new-machine readiness | `lattice bootstrap check` |
| Validate config files | `lattice validate` |
| See one service | `lattice status zsh` |
| Inspect backup/restore preflight | `lattice plan zsh` |
| Preview backup | `lattice backup --dry-run zsh` |
| Backup now | `lattice backup zsh` |
| Preview restore | `lattice restore --dry-run zsh` |
| Restore now | `lattice restore zsh` |
| Compare local files with repo copy | `lattice diff zsh` |
| Open the prompt-based UI | `lattice tui` |

## Automation And JSON Output

Use `--json` when scripts or agents need stable machine-readable output instead
of human text:

```bash
lattice bootstrap check --json
lattice status --json zsh
lattice plan --json zsh
lattice backup --dry-run --json zsh
lattice diff --json zsh
lattice restore --dry-run --json zsh
```

Use `--only` and `--exclude` to narrow a plan to specific tracked paths. These
selectors are available on `status`, `backup`, `diff`, and `restore` flows:

```bash
lattice status --json --only config.toml shell
lattice backup --dry-run --json --only config.toml shell
lattice diff --json --exclude 'cache/**' shell
lattice restore --dry-run --json --only config.toml shell
```

For automation, prefer the dry-run JSON commands before any write. Inspect the
planned `files`, `dirs`, `entries`, and `conflicts` fields, then run the
non-dry-run command only after the plan is acceptable.

## Sync With Git

Service repos are plain directories, so you can manage Git yourself or use the
built-in helpers:

```bash
lattice repo status shell
lattice repo pull shell
lattice repo commit --message "backup shell config" shell
lattice repo push shell
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

[[permissions]]
path = "scripts/sync"
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
