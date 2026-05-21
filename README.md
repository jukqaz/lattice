# Lattice

English | [한국어](README.ko.md)

Lattice is a small Rust CLI for backing up and restoring service-scoped dotfiles.

The first supported preset is `codex`: it backs up user-managed files under `~/.codex` while excluding auth, sessions, logs, sqlite databases, caches, worktrees, generated files, and other runtime state.

## Install

From this checkout while developing:

```bash
cargo install --path crates/lattice-cli
```

From the git repository after changes are pushed:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --locked
```

From a tagged release:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.0 --locked
```

Use the SSH URL form for a private repository:

```bash
cargo install --git ssh://git@github.com/jukqaz/lattice.git lattice --locked
```

During development:

```bash
cargo run -- init --force
cargo run -- status codex
cargo run -- backup --dry-run codex
cargo run -- restore --dry-run codex
```

## Workspace Layout

```text
crates/lattice-core/  core config, scan, backup, restore, hook, and path logic
crates/lattice-cli/   `lattice` binary and CLI smoke tests
xtask/                development verification harness
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
lattice service add editor --root ~/.config/editor --include settings.toml
lattice service add shell --root ~ --preset zsh --os macos
lattice service add linked --root ~/.config/tool --include config.toml --symlink
lattice service add templated --root ~/.config/tool --include config.toml --template
lattice service show editor
lattice service remove --yes editor
lattice include add editor settings.toml themes/**
lattice include remove editor themes/**
lattice exclude add editor cache/** state/**
lattice exclude remove editor state/**
lattice permission set editor settings.toml 0600
lattice permission remove editor settings.toml
lattice preset list
lattice preset show zsh
lattice track editor settings.toml themes/**
lattice adopt editor settings.toml
lattice diff editor
lattice repo status editor
lattice repo pull editor
lattice repo commit editor --message "backup editor config"
lattice repo push editor
lattice secret add editor api-token --backend rbw --item "Editor API" --field password --env EDITOR_API_TOKEN
lattice secret list editor
lattice secret check editor
lattice tui
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
If `repo` is omitted, Lattice stores that service under
`$XDG_DATA_HOME/lattice/repos/<service-name>`.

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

[conditions]
os = "macos"

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
include = [".zshrc", ".config/starship.toml"]
exclude = []
```

Set `repo = "..."` only when a service needs a custom repository location.
Set `--symlink` to restore tracked files as links to the repo and `--template`
to render `{{env:NAME}}` placeholders from environment variables on restore.
When both are enabled, files with rendered template values are restored as
regular files so the repo copy keeps placeholders. `lattice diff` renders the
repo template before comparison and hides line output for templated files that
still differ.
Use `--os` and `--hostname` to make a service active only on matching machines.

Use `lattice service add`, `lattice include`, `lattice exclude`,
`lattice permission`, `lattice secret`, and `lattice track` to manage service
TOML without editing files by hand.

## Secret Policy

Lattice does not back up secret values. Secret commands store metadata such as
backend, item, field, environment key, and folder only. `secret check` verifies
whether `rbw` or `bw` is available without reading or printing secret values.

Backups fail by default when file contents contain obvious secret-looking
markers such as common API token prefixes. Use `--allow-secret-looking-files`
only after reviewing the affected files.

## Filesystem Safety

Manifest entries, restore directories, and permission rules must be relative
paths that stay inside the configured service root. Parent traversal, absolute
paths, unsupported manifest versions, repo source symlinks, and symlinked
destination parents are rejected. Backup also refuses repo destination symlinks
so an existing repository cannot redirect writes outside the repo.

## Restore Safety

Restore refuses to overwrite conflicting local files by default. Use `restore --dry-run <service>` to inspect conflicts and `restore --force <service>` to overwrite intentionally. Forced restores snapshot overwritten files under XDG state before applying repo contents.

## Verification

Run the Rust harness:

```bash
cargo run -p xtask -- verify
```

The harness runs formatting, Clippy, the full Rust test suite, an isolated XDG
backup/restore smoke test, CLI failure-path checks, repo secret-guard checks,
binary diff redaction checks, config mismatch checks, failed-adopt rollback
checks, and a non-Unix `lattice-core` compile check when the `wasm32-wasip2`
target is installed. Clippy uses warning-level workspace lint rules, so
findings are visible without turning every recommendation into a hard failure.

Run the same harness inside a Linux Docker container:

```bash
cargo run -p xtask -- linux-verify
```

Set `LATTICE_LINUX_IMAGE` to override the default `rust:1.95-bookworm` image.
The Docker harness mounts the workspace read-only, copies it into the container
without `.git` or `target`, then runs `cargo run -p xtask -- verify`.

Run the heavier quality gate before release-oriented changes:

```bash
cargo run -p xtask -- quality
```

The quality gate requires `cargo-deny`, `cargo-machete`, `cargo-llvm-cov`, and
`typos`. It runs the normal verification harness, checks dependency advisories,
licenses, banned sources, unused dependencies, typo scanning, and writes LCOV
coverage to `target/llvm-cov/lcov.info`.

GitHub Actions runs the same `xtask verify` harness on:

- Linux x86_64: `ubuntu-latest`
- Linux ARM64: `ubuntu-24.04-arm`
- macOS Apple Silicon: `macos-latest`

GitHub Actions also runs `xtask quality` on Linux x86_64.

## Documentation

- Documentation index: [docs/README.md](docs/README.md)
- Public user guide: [docs/user/usage.md](docs/user/usage.md)
- Product scope: [docs/product/mvp-scope.md](docs/product/mvp-scope.md)
- Changelog: [CHANGELOG.md](CHANGELOG.md)
- LLM agent guidance: [docs/llm/README.md](docs/llm/README.md)
- LLM branch and release rules:
  [docs/llm/branch-release-policy.md](docs/llm/branch-release-policy.md)

## Dependency Choices

- `inquire`: prompt-based TUI and wizard flow.
- `which`: external tool discovery for `rbw`, `bw`, and future helpers.
- `similar`: text diff output for `lattice diff`.
