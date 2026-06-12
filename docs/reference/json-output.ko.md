# JSON Output Reference

[English](json-output.md) | 한국어 | [문서 인덱스](../README.ko.md)

이 reference는 script와 agent가 사람이 읽는 stdout을 parsing하지 않고 사용할 수 있는
machine-readable output을 설명합니다. 아래 shape는 v0.8.1 automation and maintainability patch
hardening release line의 일부이지만, Lattice는 아직 pre-v1.0입니다. 따라서 이 field들은
영구적인 public API라기보다 release-line contract로 취급하세요.

## 일반 규칙

- JSON mode는 output format만 바꿉니다. `--json`을 받는 non-dry-run command는 여전히
  요청한 write를 수행합니다.
- write 전에는 `plan --json`, `backup --dry-run --json`, `restore --dry-run --json`,
  `snapshot prune --dry-run --json`, `undo --dry-run --json` 같은 dry-run JSON을 먼저
  사용하세요.
- `--only`, `--exclude` selector는 single-service와 읽기 전용 group status/plan에서 같은
  tracked-path semantics를 사용합니다.
- v0.6 fixture contract test는 이 reference의 모든 command에 대해 top-level key를
  고정합니다. Field를 추가할 때는 문서와 테스트를 함께 업데이트하세요.

## Bootstrap JSON

### `lattice bootstrap check --json`

Top-level key: `ok`, `config`, `config_exists`, `services_dir`,
`services_dir_exists`, `services`, `ready_services`, `git`, `diagnostics`,
`next_actions`.

새 머신 readiness summary로 사용합니다. `ok=false`는 blocking readiness issue가 남아
있다는 뜻입니다.

## Single-Service JSON

### `lattice status --json <service>`

Top-level key: `service`, `root`, `repo`, `active`, `manifest`,
`included_files`, `files`.

### `lattice plan --json <service>`

Top-level key: `service`, `root`, `repo`, `active`, `root_exists`, `manifest`,
`ready`, `requires_force`, `safe_to_restore_without_force`, `snapshot_on_conflict`,
`snapshot_policy`, `backup_would_copy`, `restore_would_restore`,
`restore_would_create_dirs`, `files`, `dirs`, `entries`, `conflicts`.

`ready=false`, `requires_force=true`, non-empty `conflicts`는 restore 전 stop sign입니다.
`conflicts`는 structured data이며 numeric count가 아닙니다.

### `lattice backup --dry-run --json <service>`

Top-level key: `service`, `dry_run`, `destination`, `files`, `dirs`, `hooks`,
`would_copy`, `would_track_dirs`.

### `lattice diff --json <service>`

Top-level key: `service`, `diffs`.

### `lattice restore --dry-run --json <service>`

Top-level key: `service`, `dry_run`, `destination`, `snapshot_policy`,
`requires_force`, `safe_to_restore_without_force`, `would_restore`,
`would_create_dirs`, `entries`, `dirs`, `hooks`, `conflicts`.

Dry-run restore는 실제 restore가 거부할 unsafe snapshot input, invalid manifest path,
symlink escape, conflict preflight를 같은 방식으로 실행합니다.

## Snapshot And Undo JSON

### `lattice snapshot list --json`

Top-level key: `snapshots`.

### `lattice snapshot show --json <snapshot-id>`

Top-level key: `id`, `service`, `path`, `files`, `entries`.

### `lattice undo --dry-run --json <snapshot-id>`

Top-level key: `snapshot`, `service`, `destination`, `dry_run`, `would_restore`,
`entries`, `preflight`.

### `lattice snapshot prune --dry-run --json --keep <n>`

Top-level key: `dry_run`, `keep`, `remove`, `would_remove`.

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

`discover`는 service file을 쓰지 않습니다. `next_command`는 review-first 시작점이며,
warning-only candidate는 add command 대신 review message를 사용합니다.

## Service Group JSON

Global config에 group을 정의합니다:

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

### `lattice group show --json <group>`

Top-level shape:

```json
{
  "name": "dev-shell",
  "description": "Shell and CLI development environment",
  "services": ["zsh", "git", "mise", "ssh"]
}
```

### `lattice group status --json <group>`

Important field: `group`, `service_count`, `active_services`, `included_files`,
`services[].service`, `services[].active`, `services[].root_exists`,
`services[].included_files`, `services[].manifest`.

### `lattice group plan --json <group>`

Important field: `group`, `service_count`, `active_services`, `backup_would_copy`,
`restore_would_restore`, `restore_would_create_dirs`, `conflict_count`, `ready`,
`conflicts`, `services`.

`conflict_count`는 numeric aggregate이고, `conflicts`는 service별 structured data입니다.

## v0.6의 의도적 제한

v0.6에는 `group backup`이나 `group restore`가 없습니다. Batch mutation safety가
의도적으로 설계되고 테스트되기 전까지 service group은 읽기 전용 점검과 planning surface로
유지됩니다.
