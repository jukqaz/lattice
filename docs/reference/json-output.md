# JSON Output Reference

English | [한국어](json-output.ko.md) | [Documentation Index](../README.md)

This reference documents the machine-readable output that scripts and agents can
use without parsing human text. The shapes below are part of the v0.8.0
automation and maintainability release line, but Lattice is still pre-v1.0: treat
these fields as release-line contracts rather than forever-stable public API.

## General Rules

- JSON mode changes output format only. A non-dry-run command that accepts
  `--json` still performs the requested write.
- Prefer dry-run JSON before any write: `plan --json`, `backup --dry-run --json`,
  `restore --dry-run --json`, `snapshot prune --dry-run --json`, and
  `undo --dry-run --json`.
- Path selectors such as `--only` and `--exclude` use the same tracked-path
  semantics in single-service and read-only group status/plan flows.
- Service-group aggregate totals are current-host actionable totals: inactive
  services stay visible in per-service rows but do not contribute to active-only
  aggregate counts.
- The v0.6 fixture contract tests pin top-level keys for every command listed in
  this reference. Add fields by documenting and testing them together.

## Bootstrap JSON

### `lattice bootstrap check --json`

Top-level keys: `ok`, `config`, `config_exists`, `services_dir`,
`services_dir_exists`, `services`, `ready_services`, `git`, `diagnostics`, and
`next_actions`.

Use this as a new-machine readiness summary. `ok=false` means at least one
blocking readiness issue remains. Repository metadata such as local-only or dirty
Git state is diagnostic unless a command specifically requires publish/sync
readiness.

## Single-Service JSON

### `lattice status --json <service>`

Top-level keys: `service`, `root`, `repo`, `active`, `manifest`,
`included_files`, and `files`.

`files` is the selected tracked-file view after `--only`/`--exclude` filters.
`manifest` is a compact state string such as `present` or `missing`.

### `lattice plan --json <service>`

Top-level keys: `service`, `root`, `repo`, `active`, `root_exists`, `manifest`,
`ready`, `requires_force`, `safe_to_restore_without_force`,
`snapshot_on_conflict`, `snapshot_policy`, `backup_would_copy`,
`restore_would_restore`, `restore_would_create_dirs`, `files`, `dirs`, `entries`,
and `conflicts`.

Use `ready=false`, `requires_force=true`, and non-empty `conflicts` as stop signs
before a restore. `conflicts` is structured data; do not treat it as a numeric
count.

### `lattice backup --dry-run --json <service>`

Top-level keys: `service`, `dry_run`, `destination`, `files`, `dirs`, `hooks`,
`would_copy`, and `would_track_dirs`.

The dry-run form does not copy files or write manifests. Non-dry-run JSON backup
still performs the backup.

### `lattice diff --json <service>`

Top-level keys: `service` and `diffs`.

`diffs` contains one entry per differing tracked path. Binary differences are
reported without exposing line-level binary content.

### `lattice restore --dry-run --json <service>`

Top-level keys: `service`, `dry_run`, `destination`, `snapshot_policy`,
`requires_force`, `safe_to_restore_without_force`, `would_restore`,
`would_create_dirs`, `entries`, `dirs`, `hooks`, and `conflicts`.

The dry-run form performs the same restore preflight checks that reject unsafe
snapshot inputs, invalid manifest paths, symlink escapes, and conflicts before a
real restore.

## Snapshot And Undo JSON

### `lattice snapshot list --json`

Top-level keys: `snapshots`.

Each snapshot row includes the snapshot identifier and metadata needed to inspect
or prune safety snapshots.

### `lattice snapshot show --json <snapshot-id>`

Top-level keys: `id`, `service`, `path`, `files`, and `entries`.

Use this before `undo` when deciding whether a forced-restore safety snapshot is
the rollback source you expect.

### `lattice undo --dry-run --json <snapshot-id>`

Top-level keys: `snapshot`, `service`, `destination`, `dry_run`, `would_restore`,
`entries`, and `preflight`.

Dry-run undo runs restore preflight before reporting success, so a successful
JSON dry run is stronger than a simple snapshot existence check.

### `lattice snapshot prune --dry-run --json --keep <n>`

Top-level keys: `dry_run`, `keep`, `remove`, and `would_remove`.

Use the dry-run form first. Non-dry-run JSON prune still deletes eligible
snapshots.

## Discover JSON

### `lattice discover --json`

Top-level shape:

```json
{
  "suggestions": [
    {
      "name": "shell",
      "root": "/home/alice",
      "include": [".zshrc"],
      "exclude": [".cache/**", ".config/**", ".profile"],
      "reason": "common shell startup files",
      "warnings": [
        "excluded .profile because it contains secret-looking content (github token)"
      ],
      "uses_app_catalog": false,
      "next_command": "lattice service add shell --root /home/alice --include .zshrc"
    }
  ],
  "mutated": false,
  "services_dir": "/home/alice/.config/lattice/services",
  "next_actions": [
    "review suggestions and choose one service to add",
    "run lattice plan <service> before backup or restore",
    "run lattice backup --dry-run <service> before writing repo files"
  ]
}
```

Notes:

- `discover` never writes service files. Add reviewed suggestions explicitly with
  `service add`; use `app add` only after confirming the catalog entry's root
  contract matches the root you intend to manage.
- `next_command` is a conservative copyable starting point for one suggestion;
  review its include/exclude set before running it. Warning-only candidates use
  a review message instead of an add command.
- `next_actions` is the stable top-level checklist for first adoption: review a
  single candidate, run `plan`, then run `backup --dry-run` before any write.
- `warnings` is suggestion-local. Treat it as a stop-and-review signal, not as a
  safe-to-back-up decision.
- Warning-only candidates can have `include=[]` and non-empty `exclude` and
  `warnings`; they remain visible so automation can explain why every file was
  excluded.
- Warning text includes pattern classes only, not matched secret values.

## Service Group JSON

Define groups in the global config:

```toml
[[groups]]
name = "dev-shell"
description = "Shell and CLI development environment"
services = ["zsh", "git", "mise", "ssh"]
```

### `lattice group list --json`

Top-level shape:

```json
{
  "groups": [
    {
      "name": "dev-shell",
      "description": "Shell and CLI development environment",
      "services": ["zsh", "git", "mise", "ssh"]
    }
  ]
}
```

Use it to enumerate configured group names and ordered service members. Group
config validation rejects duplicate group names, empty groups, unknown service
references, and duplicate service members before group commands run.

### `lattice group show --json <group>`

Top-level shape:

```json
{
  "name": "dev-shell",
  "description": "Shell and CLI development environment",
  "services": ["zsh", "git", "mise", "ssh"]
}
```

Use it when automation needs one exact group definition.

### `lattice group status --json <group>`

Important fields:

```json
{
  "group": "dev-shell",
  "service_count": 4,
  "active_services": 3,
  "included_files": 12,
  "services": [
    {
      "service": "zsh",
      "active": true,
      "root_exists": true,
      "included_files": 5,
      "manifest": "present"
    },
    {
      "service": "ssh",
      "active": false,
      "root_exists": null,
      "included_files": 0,
      "manifest": "skipped"
    }
  ]
}
```

Notes:

- `service_count` is the number of configured members.
- `active_services` counts members active on the current host.
- `included_files` is an active-service aggregate.
- `root_exists=true` means the active member root exists.
- `root_exists=false` means the active member root is genuinely missing.
- `root_exists=null` means root inspection was skipped, usually because the
  member is inactive on this host.
- I/O errors during root inspection are surfaced instead of being collapsed into
  `root_exists=false`.

### `lattice group plan --json <group>`

Important fields:

```json
{
  "group": "dev-shell",
  "service_count": 4,
  "active_services": 3,
  "backup_would_copy": 7,
  "restore_would_restore": 2,
  "restore_would_create_dirs": 1,
  "conflict_count": 1,
  "ready": false,
  "conflicts": [
    {
      "service": "zsh",
      "paths": ["config.toml"]
    }
  ],
  "services": [
    {
      "service": "zsh",
      "active": true,
      "root_exists": true,
      "backup_would_copy": 3,
      "restore_would_restore": 1,
      "ready": false
    }
  ]
}
```

Notes:

- `backup_would_copy`, `restore_would_restore`, `restore_would_create_dirs`, and
  `conflict_count` are active-service aggregates.
- `conflict_count` is the numeric aggregate.
- `conflicts` remains structured data grouped by service. Do not treat it as a
  scalar count.
- `ready=false` means at least one active member has a blocking plan issue such
  as a restore conflict.

## Intentional v0.6 Limits

There is no `group backup` or `group restore` in v0.7. Service groups are
read-only inspection and planning surfaces until batch mutation safety is
intentionally designed and tested.
