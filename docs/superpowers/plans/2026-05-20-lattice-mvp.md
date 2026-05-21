# Lattice MVP Implementation Plan

Status: archived after completion. The file structure below reflects the
pre-workspace v0.1 implementation path; current source lives under
`crates/lattice-core`, `crates/lattice-cli`, and `xtask`.

> **For agentic workers:** This is historical implementation evidence, not an
> active plan.

**Goal:** Build the first working Rust CLI slice for backing up and restoring a configured Codex service.

**Architecture:** The binary parses commands and delegates to a small library. The library owns XDG path resolution, TOML config loading, service scanning, backup/restore copying, and permission manifests. v0.1 avoids git automation and secret materialization.

**Tech Stack:** Rust 2024, `clap`, `serde`, `toml`, `walkdir`, `globset`, `anyhow`, `tempfile` for tests.

---

## File Structure

- `Cargo.toml`: crate metadata and dependencies.
- `src/main.rs`: CLI entrypoint and command dispatch.
- `src/lib.rs`: module exports.
- `src/paths.rs`: XDG path resolver.
- `src/config.rs`: global and service TOML models plus defaults.
- `src/preset.rs`: Codex include/exclude preset.
- `src/scanner.rs`: include/exclude file scanning.
- `src/manifest.rs`: permission manifest read/write.
- `src/ops.rs`: backup and restore operations.
- `tests/cli_smoke.rs`: end-to-end CLI smoke tests with temporary directories.
- `xtask/src/main.rs`: Rust task-completion harness for formatting, full tests, and isolated XDG backup/restore smoke.

## Task Completion Gate

Every task must end with:

```bash
cargo run -p xtask -- verify
```

Expected: `cargo fmt --check`, `cargo test`, and the isolated XDG CLI harness all pass. A task is not complete until this command succeeds.

### Task 1: Dependencies And Module Skeleton

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/paths.rs`
- Create: `src/config.rs`
- Create: `src/preset.rs`
- Create: `src/scanner.rs`
- Create: `src/manifest.rs`
- Create: `src/ops.rs`

- [x] **Step 1: Add dependencies**

Use:

```toml
[dependencies]
anyhow = "1"
clap = { version = "4.5", features = ["derive"] }
globset = "0.4"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
walkdir = "2"

[dev-dependencies]
tempfile = "3"
```

- [x] **Step 2: Create empty modules and compile**

Run: `cargo test`

Expected: the crate compiles with zero tests or the default generated test set.

- [x] **Step 3: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

### Task 2: Config And XDG Paths

**Files:**
- Modify: `src/paths.rs`
- Modify: `src/config.rs`

- [x] **Step 1: Write tests for fallback XDG paths and config parsing**

Use temp directories and explicit environment overrides in tests. Verify that `lattice.toml` and `services/codex.toml` parse into typed structs.

- [x] **Step 2: Run tests and verify RED**

Run: `cargo test`

Expected: tests fail because the modules are not implemented yet.

- [x] **Step 3: Implement path resolution and TOML models**

Implement `LatticePaths`, `GlobalConfig`, `ServiceConfig`, and `PermissionRule`.

- [x] **Step 4: Run tests and verify GREEN**

Run: `cargo test`

Expected: all config and path tests pass.

- [x] **Step 5: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

### Task 3: Codex Preset And Scanner

**Files:**
- Modify: `src/preset.rs`
- Modify: `src/scanner.rs`

- [x] **Step 1: Write scanner tests**

Create a fake Codex tree with `config.toml`, `agents/reviewer.toml`, `auth.json`, `sessions/current.jsonl`, `logs_2.sqlite`, and `bin/mcp-rbw`. Verify only included non-excluded files are returned.

- [x] **Step 2: Run tests and verify RED**

Run: `cargo test`

Expected: tests fail because scan behavior is missing.

- [x] **Step 3: Implement preset expansion and glob scanning**

Use `globset` and `walkdir`. Return relative paths sorted for stable output.

- [x] **Step 4: Run tests and verify GREEN**

Run: `cargo test`

Expected: scanner tests pass.

- [x] **Step 5: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

### Task 4: Backup, Manifest, And Restore

**Files:**
- Modify: `src/manifest.rs`
- Modify: `src/ops.rs`

- [x] **Step 1: Write operation tests**

Use temp source and repo directories. Back up `config.toml` and `bin/mcp-rbw`, assert files exist in repo, assert `.lattice/manifest.toml` exists, then restore into an empty destination and assert file contents and modes are restored.

- [x] **Step 2: Run tests and verify RED**

Run: `cargo test`

Expected: tests fail because operations are not implemented yet.

- [x] **Step 3: Implement backup and restore**

Copy regular files, create parent directories, write a TOML manifest with relative paths and Unix modes, restore files, and apply modes on Unix.

- [x] **Step 4: Run tests and verify GREEN**

Run: `cargo test`

Expected: operation tests pass.

- [x] **Step 5: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

### Task 5: CLI Commands

**Files:**
- Modify: `src/main.rs`
- Create: `tests/cli_smoke.rs`

- [x] **Step 1: Write CLI smoke tests**

Test `init`, `doctor`, `service list`, `backup codex`, and `restore codex` with isolated XDG env vars.

- [x] **Step 2: Run tests and verify RED**

Run: `cargo test --test cli_smoke`

Expected: tests fail because CLI behavior is incomplete.

- [x] **Step 3: Implement CLI commands**

Use `clap` subcommands. `init` writes default TOML. `doctor` checks paths and command availability. `service list` prints configured services. `backup` and `restore` call library operations.

- [x] **Step 4: Run tests and verify GREEN**

Run: `cargo test --test cli_smoke`

Expected: smoke tests pass.

- [x] **Step 5: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

### Task 6: Final Verification

**Files:**
- Modify: `TODO.md`

- [x] **Step 1: Format**

Run: `cargo fmt --check`

Expected: no formatting diff.

- [x] **Step 2: Test**

Run: `cargo test`

Expected: all tests pass.

- [x] **Step 3: Manual smoke**

Run: `cargo run -- init --force`

Expected: default config appears under the active XDG config directory.

- [x] **Step 4: Run task-completion harness**

Run: `cargo run -p xtask -- verify`

Expected: formatting, full tests, and isolated CLI smoke pass.

- [x] **Step 5: Update TODO**

Mark completed v0.1 items in `TODO.md`.
