# JSON Output Reference

English | [한국어](json-output.ko.md) | [Documentation Index](../README.md)

This reference documents the machine-readable output that scripts and agents can
use without parsing human text. The shapes below are part of the v0.5.1 hardened
service-groups release line, but Lattice is still pre-v1.0: treat these fields as
release-line contracts rather than forever-stable public API.

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
      ]
    }
  ],
  "mutated": false,
  "services_dir": "/home/alice/.config/lattice/services"
}
```

Notes:

- `discover` never writes service files. Add reviewed suggestions explicitly with
  `app add` or `service add`.
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

## Intentional v0.5 Limits

There is no `group backup` or `group restore` in v0.5. Service groups are
read-only inspection and planning surfaces until batch mutation safety is
intentionally designed and tested.
