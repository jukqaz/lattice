# Lattice MVP Scope

## Product Positioning

Lattice is a lightweight Rust dotfiles manager. It manages service-scoped files and directories with explicit TOML configuration, predictable XDG storage, permission preservation, and safe restore behavior.

It is not a full system configuration manager, package manager, or secret manager. It can call external tools later, but the core product should stay small: scan, plan, backup, restore, and run narrowly configured lifecycle hooks.

## Current Baseline: v0.1.0

v0.1.0 proves the core backup/restore model:

- XDG-aware config and state paths.
- TOML global config and per-service config.
- Service root and repo path.
- Include and exclude globs.
- Codex preset.
- `init`, `doctor`, `service list`, `status`.
- `backup`, `backup --dry-run`.
- `restore`, `restore --dry-run`.
- Permission manifest capture and restore.
- Restore-time secure directory creation.
- Rust-only `xtask` verification harness.

This is enough as a released spike, but not enough to trust broad real-home restores without a stronger safety layer.

## MVP Target: v0.2 Light

v0.2 should be the first version that is reasonable to use on real personal dotfiles without feeling heavy.

### Must Include

1. **Restore Conflict Detection**
   - Detect when a restore would overwrite an existing file whose contents differ.
   - Default behavior: refuse to overwrite conflicts.
   - Allow explicit override with `--force`.
   - `restore --dry-run` must list conflicts.

2. **Restore Snapshots**
   - Before a non-dry-run restore, copy overwritten files into an XDG state snapshot.
   - Store snapshots under `$XDG_STATE_HOME/lattice/snapshots/<timestamp>/<service>/`.
   - Print the snapshot path after restore.
   - Do not snapshot files that are newly created.

3. **Minimal Lifecycle Hooks**
   - Support hooks in service TOML.
   - MVP hooks are command-only, no plugin system.
   - Supported phases:
     - `before_backup`
     - `after_backup`
     - `before_restore`
     - `after_restore`
   - Supported fields:
     - `name`
     - `command`
     - `args`
     - `timeout_sec`
     - `confirm`
   - Hooks do not run during `--dry-run`; dry-run only prints what would run.
   - Hooks with `confirm = true` must be skipped unless the CLI receives `--yes`.

4. **Custom Service Fixture**
   - Add a second test fixture for a non-Codex service.
   - This proves Lattice is a dotfiles manager, not a Codex-only tool.

5. **Real Codex Dry-Run Harness**
   - Extend `xtask verify` with a read-only real `~/.codex` status/dry-run check when `~/.codex` exists.
   - It must not write to the real Codex repo path.
   - It should only prove that the preset scans real local structure without crashing.

6. **Secret Pattern Scan**
   - Add a lightweight scan before backup writes files.
   - Default behavior: warn and fail on obvious token patterns.
   - Allow explicit bypass with `--allow-secret-looking-files`.
   - This is not a full DLP system; it is a guardrail for common mistakes.

### Should Include If Small

- `lattice validate` to parse all config files and report unknown or invalid service references.
- Better human-readable plan output for backup/restore dry-runs.
- `--json` output for `status` and dry-run commands if it stays simple.

### Must Not Include

- Automatic git commit, pull, push, or repo creation.
- Remote GitHub integration.
- Secret value materialization from `rbw` or `bw`.
- Template rendering.
- Multi-profile orchestration.
- Full plugin system.
- Package installation.
- Home Manager or Nix-style declarative program modules.
- GUI or TUI.
- Database-backed state.

## MVP 2 Deferred Scope

MVP 2 can start after v0.2 safety is stable.

Candidate MVP 2 features:

- Git sync commands:
  - `repo status`
  - `repo pull`
  - `repo commit`
  - `repo push`
- Vaultwarden-backed secret metadata:
  - `rbw`
  - `bw`
  - no secret values stored in repo
- App preset catalog:
  - `codex`
  - `git`
  - `zsh`
  - `mise`
  - `ssh`
- `track` / `adopt` command for importing existing files into a service.
- Optional symlink mode.
- OS and hostname conditions.
- Simple template rendering.

## Configuration Shape

The v0.2 service config should remain TOML and readable:

```toml
name = "codex"
root = "~/.codex"
repo = "~/.local/share/lattice/repos/codex"
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

## Command Shape

v0.2 should keep the CLI small:

```bash
lattice init
lattice doctor
lattice validate
lattice service list
lattice status <service>
lattice backup --dry-run <service>
lattice backup <service>
lattice restore --dry-run <service>
lattice restore <service>
lattice restore --force <service>
lattice restore --yes <service>
```

## Acceptance Criteria

v0.2 is complete when:

- Existing conflicting files are not overwritten by default.
- `restore --force` can intentionally overwrite conflicts.
- Restore creates a snapshot for every overwritten file.
- Hook dry-runs print hook actions without executing them.
- Non-dry-run hooks execute in phase order.
- `confirm = true` hooks require `--yes`.
- The custom service fixture passes.
- The real `~/.codex` dry-run harness passes without writing to `~/.codex` or its configured repo.
- Secret-looking content is blocked before backup unless explicitly bypassed.
- `cargo run -p xtask -- verify` passes.

