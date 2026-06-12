# Migration Guide

[English](migration.md) | 한국어 | [문서 인덱스](../README.ko.md)

이 문서는 Lattice의 safety model을 약화하지 않고 현재 `v0.8.1` release line에서
`v0.9.x`와 `v1.0.0`로 이동하는 방법을 설명합니다.

## 지원하는 upgrade 형태

Lattice는 v1.0 경로에서도 Git tag distribution을 유지합니다. 명시적인 tag를 설치합니다.

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.8.1 --locked
```

향후 release에서는 tag만 검토한 목표 tag로 바꾸고 다음을 확인합니다.

```bash
lattice --version
lattice validate
lattice doctor
lattice bootstrap check
lattice plan <service>
```

Upgrade 직후 `restore --force`를 실행하지 마세요. 먼저 대상 service에 대해 `plan`과
`restore --dry-run`을 실행합니다.

## v0.8.1에서 v0.9.x로

`v0.9.0`은 public-readiness line으로 계획합니다. Core backup/restore model은 바꾸지 않고
positioning, installation, migration, issue template, change policy, completion/manpage,
home/work documentation을 개선합니다.

예상 migration 영향:

- 기존 service TOML file은 계속 load되어야 합니다.
- 기존 backup repo와 snapshot history는 계속 사용할 수 있어야 합니다.
- JSON top-level contract는 compatible하게 유지하거나 새 optional field를 문서화합니다.
- Context label을 받아들이더라도 opt-in additive 기능이어야 합니다. Context condition이 없는
  service는 이전처럼 동작해야 합니다.

## v0.9.x에서 v1.0.0으로

`v1.0.0`은 stable contract release입니다. Upgrade 전 정확한 tag의 changelog와 stability
reference를 읽습니다.

권장 upgrade checklist:

1. 대상 tag를 명시적으로 설치합니다.
2. Backup/restore 전에 `lattice validate`를 실행하고 config warning을 해결합니다.
3. Machine-readable output에 의존하는 automation은 `lattice status --json`과
   `lattice plan --json`을 실행합니다.
4. 낮은 위험의 `backup --dry-run` 하나를 실행하고 출력을 검토합니다.
5. 실제 restore 전 `restore --dry-run`을 실행합니다.
6. Rollback이 명시적이도록 이전 install command/tag를 기록해 둡니다.

Rollback도 같은 explicit tag install 형태를 사용합니다.

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.8.1 --locked
```

## v1.0에서 안정화할 것

v1.0 contract는 다음을 포함해야 합니다.

- 문서화된 command name과 subcommand shape;
- 문서화된 config key와 default path behavior;
- automation surface의 문서화된 JSON top-level key;
- dry-run-first safety behavior;
- forced overwrite 전 snapshot과 undo inspection;
- secret 값을 읽거나 materialize하지 않는 secret reference.

## 계속 scope 밖인 것

v1.0 경로에서 package installation, remote repo creation, secret value management,
group backup/restore mutation, GUI, database-backed state, per-file alternate,
full conditional template을 기대하지 않습니다.
