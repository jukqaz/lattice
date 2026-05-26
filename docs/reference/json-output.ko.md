# JSON Output Reference

[English](json-output.md) | 한국어 | [문서 인덱스](../README.ko.md)

이 reference는 script와 agent가 사람이 읽는 stdout을 parsing하지 않고 사용할 수
있는 machine-readable output을 설명합니다. 아래 shape는 v0.5.0 service-groups
release line의 일부이지만, Lattice는 아직 pre-v1.0입니다. 따라서 이 field들은
영구 public API가 아니라 release-line contract로 다룹니다.

## 일반 규칙

- JSON mode는 output format만 바꿉니다. `--json`을 받는 non-dry-run command는
  요청한 write를 그대로 수행합니다.
- 쓰기 작업 전에는 dry-run JSON을 먼저 사용합니다: `plan --json`,
  `backup --dry-run --json`, `restore --dry-run --json`,
  `snapshot prune --dry-run --json`, `undo --dry-run --json`.
- `--only`, `--exclude` 같은 path selector는 single-service flow와 읽기 전용
  group status/plan flow에서 같은 tracked-path semantics를 사용합니다.
- Service-group aggregate total은 현재 host에서 실행 가능한 값입니다. Inactive
  service는 per-service row에 남지만 active-only aggregate에는 더하지 않습니다.

## Service Group JSON

Global config에 group을 정의합니다.

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

Configured group name과 순서 있는 service member를 열거할 때 사용합니다. Group
command 실행 전에 validation이 duplicate group name, empty group, unknown service
reference, duplicate service member를 거부합니다.

### `lattice group show --json <group>`

Top-level shape:

```json
{
  "name": "dev-shell",
  "description": "Shell and CLI development environment",
  "services": ["zsh", "git", "mise", "ssh"]
}
```

Automation이 특정 group definition 하나만 필요할 때 사용합니다.

### `lattice group status --json <group>`

중요 field:

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

- `service_count`는 configured member 수입니다.
- `active_services`는 현재 host에서 active인 member 수입니다.
- `included_files`는 active-service aggregate입니다.
- `root_exists=true`는 active member root가 존재한다는 뜻입니다.
- `root_exists=false`는 active member root가 실제로 없다는 뜻입니다.
- `root_exists=null`은 보통 해당 member가 inactive라 root inspection을 skip했다는
  뜻입니다.
- Root inspection 중 I/O error는 `root_exists=false`로 숨기지 않고 error로
  드러냅니다.

### `lattice group plan --json <group>`

중요 field:

```json
{
  "group": "dev-shell",
  "service_count": 4,
  "active_services": 3,
  "backup_would_copy": 7,
  "restore_would_restore": 2,
  "create_dirs": 1,
  "conflict_count": 1,
  "ready": false,
  "conflicts": [
    {
      "service": "zsh",
      "path": "config.toml"
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

- `backup_would_copy`, `restore_would_restore`, `create_dirs`, `conflict_count`는
  active-service aggregate입니다.
- `conflict_count`는 numeric aggregate입니다.
- `conflicts`는 service별 structured data입니다. Scalar count로 다루지 않습니다.
- `ready=false`는 restore conflict 같은 blocking plan issue가 active member 중
  하나 이상에 있다는 뜻입니다.

## v0.5의 의도적 제한

v0.5에는 `group backup`이나 `group restore`가 없습니다. Batch mutation safety가
의도적으로 설계되고 테스트되기 전까지 service group은 읽기 전용 점검과 planning
surface입니다.
