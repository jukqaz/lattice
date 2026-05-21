# Lattice

Lattice is a small Rust CLI for backing up and restoring service-scoped dotfiles.

The first supported preset is `codex`: it backs up user-managed files under `~/.codex` while excluding auth, sessions, logs, sqlite databases, caches, worktrees, generated files, and other runtime state.

## Install

From this checkout:

```bash
cargo install --path .
```

During development:

```bash
cargo run -- init --force
cargo run -- status codex
cargo run -- backup --dry-run codex
cargo run -- restore --dry-run codex
```

## XDG Layout

Lattice stores its own config in XDG locations:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
~/.local/share/lattice/repos/*
~/.local/state/lattice/
~/.cache/lattice/
```

Environment overrides are supported through `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, and `XDG_CACHE_HOME`.

## Commands

```bash
lattice init
lattice doctor
lattice validate
lattice service list
lattice status codex
lattice backup --dry-run codex
lattice backup codex
lattice backup --yes codex
lattice backup --allow-secret-looking-files codex
lattice restore --dry-run codex
lattice restore codex
lattice restore --force codex
lattice restore --yes codex
```

## Service Config

Service configs live under `~/.config/lattice/services`.

```toml
name = "codex"
root = "~/.codex"
repo = "~/.local/share/lattice/repos/codex"
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

[[hooks.after_restore]]
name = "codex doctor"
command = "codex"
args = ["doctor", "--summary"]
timeout_sec = 60
confirm = false
```

Custom services can provide their own `include` and `exclude` globs:

```toml
name = "shell"
root = "~"
repo = "~/.local/share/lattice/repos/shell"
include = [".zshrc", ".config/starship.toml"]
exclude = []
```

## Secret Policy

Lattice does not back up secret values. The v0.1 `doctor` command only checks whether `rbw` and `bw` are available. Future Vaultwarden integration should store secret metadata only and resolve secret values at runtime.

Backups fail by default when file contents contain obvious secret-looking markers such as common API token prefixes. Use `--allow-secret-looking-files` only after reviewing the affected files.

## Restore Safety

Restore refuses to overwrite conflicting local files by default. Use `restore --dry-run <service>` to inspect conflicts and `restore --force <service>` to overwrite intentionally. Forced restores snapshot overwritten files under XDG state before applying repo contents.

## Verification

Run the Rust harness:

```bash
cargo run -p xtask -- verify
```

The harness runs formatting, the full Rust test suite, and an isolated XDG backup/restore smoke test.
