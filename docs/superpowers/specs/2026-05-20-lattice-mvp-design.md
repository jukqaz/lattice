# Lattice MVP Design

## Goal

Build a small Rust dotfiles manager that can back up and restore one explicitly configured service. Presets are optional shortcuts, not the product center.

## Product Boundary

Lattice manages user-owned configuration files. It does not manage application runtime state, sessions, logs, auth tokens, caches, or generated artifacts by default. Services are configured with a root directory, a local repository directory, include globs, exclude globs, and restore-time permission rules.

## XDG Layout

- Config: `$XDG_CONFIG_HOME/lattice` or `~/.config/lattice`
- Data: `$XDG_DATA_HOME/lattice` or `~/.local/share/lattice`
- State: `$XDG_STATE_HOME/lattice` or `~/.local/state/lattice`
- Cache: `$XDG_CACHE_HOME/lattice` or `~/.cache/lattice`

The main config lives at `lattice.toml`. Service configs live under `services/*.toml`.

## v0.1 Commands

- `lattice init`: create default config files without overwriting existing files unless `--force` is passed.
- `lattice doctor`: print XDG paths, config availability, and `rbw`/`bw` command availability without reading secret values.
- `lattice service list`: list configured services.
- `lattice backup shell`: copy included files for an example `shell` service into the configured repo directory and write `.lattice/manifest.toml`.
- `lattice restore shell`: copy files from the repo directory back to the service root, then apply stored and configured permissions.

## Presets

Presets may include known configuration assets for common tools. They should remain optional shortcuts over the same generic service model.

Presets exclude sensitive or runtime-owned surfaces such as auth files, history, sessions, databases, logs, caches, temp directories, plugin caches, worktrees, backups, generated images, browser state, and global app state files.

## Permissions

Backups capture file modes in `.lattice/manifest.toml`. Restore reapplies captured modes and configured restore directories. Runtime directories such as caches may be created with secure permissions without tracking their contents.

## Secret Policy

Lattice v0.1 only checks whether `rbw` and `bw` commands are available. It stores secret metadata only in future service config, never secret values. No command prints secret values.

## Testing

The MVP uses unit tests and temporary directories to verify config parsing, default XDG path fallback, include/exclude scanning, backup copy behavior, manifest writing, restore copy behavior, and mode preservation on Unix.

Every implementation task must finish by running the Rust harness with `cargo run -p xtask -- verify`. The harness runs formatting, the full Rust test suite, and an isolated XDG smoke flow covering `init`, `doctor`, `service list`, `backup shell`, and `restore shell`.
