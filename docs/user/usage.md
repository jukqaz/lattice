# User Guide

English | [한국어](usage.ko.md) | [Documentation Index](../README.md) |
[Repository README](../../README.md)

This is the public, human-facing guide for Lattice. LLM-specific workflow rules
live under `docs/llm/`.

## Install

From a checkout:

```bash
cargo install --path crates/lattice-cli
```

From GitHub:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --locked
```

Install a tagged release:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag vX.Y.Z --locked
```

## Quick Start

```bash
lattice init
lattice doctor
lattice validate
lattice status codex
lattice backup --dry-run codex
lattice restore --dry-run codex
```

## Services

Lattice manages files per service. Service files live under:

```text
~/.config/lattice/services/*.toml
```

When a service omits `repo`, Lattice stores backup files under:

```text
$XDG_DATA_HOME/lattice/repos/<service-name>
```

Create a custom service:

```bash
lattice service add editor --root ~/.config/editor --include settings.toml
lattice service show editor
lattice backup --dry-run editor
```

## Safety Defaults

- Backups refuse obvious secret-looking content unless
  `--allow-secret-looking-files` is passed.
- Restore refuses to overwrite conflicting local files unless `--force` is
  passed.
- Forced restore snapshots overwritten files under XDG state.
- Restore paths, permission rules, and manifest entries must stay inside the
  service root.
- Secret commands store metadata only. They do not read or print secret values.

## Verification

For local development:

```bash
cargo run -p xtask -- verify
```

For Docker-backed Linux verification:

```bash
cargo run -p xtask -- linux-verify
```

For the heavier quality gate:

```bash
cargo run -p xtask -- quality
```
