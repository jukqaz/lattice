# 변경 로그

[English](CHANGELOG.md) | 한국어 | [문서 인덱스](docs/README.ko.md)

## Unreleased

## v0.8.1 - 2026-06-12

### 수정

- Release-check changelog validation이 이제 전체 changelog가 아니라 요청한
  release section 안에서만 note를 확인해, 이전 release note가 현재 release
  contract를 대신 만족하지 못하게 했다.
- v0.8 patch release tag에 Codex review follow-up을 포함해
  `cargo run -p xtask -- release-check 0.8.1`을 더 엄격하게 만들었다.

### 변경

- Post-review release-gate hardening patch에 맞춰 workspace package version을
  `0.8.1`으로 올렸다.
- README, user docs, product scope, TODO, install snippet, product-surface
  verification이 `v0.8.1` release contract를 가리킨다.

## v0.8.0 - 2026-06-12

### 변경

- Rust, shell, workflow 유지보수를 위한 check-only local lint entrypoint로
  `scripts/lint.sh`를 추가하고, 같은 shell style로 real-HOME health script를
  정리했다.

- CLI parser 정의를 `crates/lattice-cli/src/cli.rs`로 분리해 command surface는
  고정하면서 main command runner의 크기를 줄였다.
- CLI smoke-test helper를 `crates/lattice-cli/tests/support/`로 옮겨, 이후
  domain별 smoke file이 같은 isolated XDG harness를 공유할 수 있게 했다.
- CLI help-surface smoke를 `crates/lattice-cli/tests/help_surface.rs`로 옮겨,
  domain별 integration test 분리를 시작했다.
- 전용 service-group smoke coverage를 `crates/lattice-cli/tests/group_surface.rs`로
  옮겨 domain별 분리를 이어가되, 넓은 JSON contract smoke는 일반 suite에 남겼다.
- 전용 snapshot/undo smoke coverage를 `crates/lattice-cli/tests/snapshot_surface.rs`로
  옮겨 general smoke suite를 줄이고 recovery safety regression을 독립적으로 유지한다.
- 전용 discovery 및 JSON-contract smoke coverage를
  `crates/lattice-cli/tests/discover_surface.rs`와
  `crates/lattice-cli/tests/json_contract_surface.rs`로 옮겨, 공유 smoke suite가
  남은 cross-command flow에 집중하도록 했다.
- `discover` command runner를 `crates/lattice-cli/src/commands/discover.rs`로
  옮겨 큰 CLI 구현의 focused command module 분리를 시작했다.
- 전용 bootstrap diagnostics 및 restore-plan trust smoke coverage를
  `crates/lattice-cli/tests/bootstrap_surface.rs`로 옮기고, bootstrap command
  runner를 `crates/lattice-cli/src/commands/bootstrap.rs`로 분리해 service-report
  inspection을 한 곳으로 모았다.
- 전용 service lifecycle smoke coverage를
  `crates/lattice-cli/tests/service_lifecycle_surface.rs`로 옮겨, init/doctor/backup/restore
  end-to-end flow는 유지하면서 공유 `cli_smoke.rs` suite를 더 줄였다.
- 전용 service config 및 secret-guard smoke coverage를
  `crates/lattice-cli/tests/service_config_surface.rs`로 옮겨, service CRUD,
  default repo fallback, secret-looking content guard regression을 독립시켰다.
- 전용 adopt, repo, diff smoke coverage를
  `crates/lattice-cli/tests/adopt_repo_diff_surface.rs`로 옮겨, happy-path와
  safety regression을 공유 `cli_smoke.rs` suite에서 분리했다.
- 전용 selector smoke coverage를 `crates/lattice-cli/tests/selector_surface.rs`로
  옮겨, `--only`/`--exclude` JSON 동작을 공유 `cli_smoke.rs` suite에서 분리했다.
- 전용 restore-mode 및 safety smoke coverage를
  `crates/lattice-cli/tests/restore_surface.rs`와
  `crates/lattice-cli/tests/safety_surface.rs`로 옮겨, `cli_smoke.rs`는 얇은
  cross-command smoke guard로 남겼다.
- CLI command 구현을 app, group, permission, repo, secret, snapshot, TUI별
  `crates/lattice-cli/src/commands/` 모듈로 분리했다.
- 공유 JSON 및 human-output helper를 `crates/lattice-cli/src/output.rs`로
  옮겨, 고정된 output contract는 유지하면서 command-local output formatting
  중복을 제거했다.
- 남은 CLI infrastructure도 `main.rs` 밖으로 옮겼다. config read/write helper,
  runtime probe, 공유 service state, setup command, service CRUD, backup/restore/status/diff
  flow를 focused module로 나누고, v0.8 layout을 고정하는 `xtask` module-structure
  harness를 추가했다.
- Maintainability 및 modularization release line에 맞춰 workspace package version을
  `0.8.0`으로 올렸다.
- README, user docs, product scope, TODO, install snippet, product-surface
  verification이 `v0.8.0` release contract를 가리킨다.

### 수정

- `v0.7.0 release acceptance가 완료됐음을 TODO에 기록`하고, `v0.7.0`
  release 이후에도 pre-tag blocker처럼 보이던 stale 항목을 정리했다.
- Product-surface verification이 stale “Before tagging v0.7.0” 문구를 거부해,
  완료된 release acceptance가 active TODO로 되돌아오는 drift를 막는다.

## v0.7.0 - 2026-06-04

### 추가

- Secret metadata가 API key, token, password를 Lattice repo 밖에 두고 restore 시
  `{{env:NAME}}` template로 render할 수 있는 `env` passthrough backend를 지원한다.
- `secret check`는 값을 읽지 않고 env passthrough 상태를 보고하며 명시적인
  `value=not-read` marker를 유지한다.

### 변경

- Env secret passthrough release line에 맞춰 workspace package version을 `0.7.0`으로 올렸다.
- README, user docs, product scope, TODO, install snippet, product-surface
  verification이 `v0.7.0` release contract를 가리킨다.

## v0.6.0 - 2026-06-03

### 추가

- JSON output reference가 service group 외의 문서화된 automation surface까지 확장됐다:
  `bootstrap check --json`, single-service `status/plan`, dry-run backup/restore,
  `diff --json`, snapshot list/show/prune, `undo --dry-run --json`.
- Fixture 기반 CLI smoke coverage가 bootstrap, service, group, discovery, snapshot,
  undo surface의 top-level JSON key를 stable automation contract로 고정한다.
- `cargo run -p xtask -- release-check`가 release-line preflight를 수행한다:
  version/doc/changelog consistency, locked metadata, path install, installed binary
  smoke check.

### 변경

- Automation-contract hardening release line에 맞춰 workspace package version을
  `0.6.0`으로 올렸다.
- README, user docs, product scope, TODO, install snippet, product-surface
  verification이 `v0.6.0` release contract를 가리킨다.
- Service group은 v0.6에서도 읽기 전용으로 유지된다. Automation hardening은 batch
  backup/restore behavior 없이 contract를 확장한다.

### 수정

- `discover`는 이제 suggestion별 `next_command` hint와 top-level `next_actions`
  checklist를 JSON과 human output에 포함한다. 첫 도입 flow가 review에서 `plan`,
  그리고 어떤 write보다 먼저 `backup --dry-run`으로 이어지도록 고정한다.
- `discover`는 app 이름과 같은 config directory나 warning-only candidate에 대해,
  발견된 root/include set이 app catalog root contract와 맞지 않으면 더 이상
  `app add`를 제안하지 않는다. 복사 가능한 hint는 review-first와 service-scoped로
  유지한다.

## v0.5.1 - 2026-05-26

### 수정

- Restore와 snapshot safety smoke test가 tampered manifest, destination symlink
  escape, symlinked snapshot input, partial restore prevention을 CLI level에서
  검증한다.
- `discover`는 이제 보수적인 include/exclude pattern이 secret/auth/session/cache/
  database-looking material을 제외할 때 이를 숨기지 않고 suggestion-level warning으로
  보고한다. 모든 파일이 제외된 warning-only candidate도 숨기지 않는다.
- `undo --dry-run`은 성공을 보고하기 전에 실제 undo와 같은 snapshot restore
  preflight를 실행한다.
- 실제 HOME read-only health check는 더 이상 `cargo run` fallback을 사용하지 않는다.
  Live-HOME dogfood 중 build/cache side effect를 피하려면 `LATTICE_BIN`을 지정하거나
  먼저 `target/debug/lattice`를 build한다.
- 첫 도입 문서는 real HOME에서 restore를 먼저 실행하지 말고 read-only discovery,
  planning, reviewed backup부터 시작하라고 명시한다.
- CI는 host-only test 뒤에 platform surface regression이 숨지 않도록 명시적인
  `wasm32-wasip2` non-Unix `lattice-core` compile check를 실행한다.

### 변경

- Hardening patch release에 맞춰 workspace package version을 `0.5.1`로 올렸다.
- Product-surface verification은 이제 v0.5.1 release docs, install snippet,
  changelog, CI target-coverage contract를 고정한다.

## v0.5.0 - 2026-05-26

### 추가

- Global config의 service group. Group은 기존 service를 묶는 보수적인 named
  bundle로 유지한다.
- 읽기 전용 multi-service 점검과 planning을 위한 human output
  `lattice group list/show/status/plan`.
- Aggregate status와 plan summary를 포함한 모든 group command의 machine-readable
  JSON output.
- Batch mutation 없이 service별 status/plan view를 좁힐 수 있도록
  `group status`와 `group plan`에 path selector 추가.

### 변경

- Service-groups release line에 맞춰 workspace package version을 `0.5.0`으로
  올렸다.
- `lattice validate`는 이제 duplicate group name, empty group, unknown service
  reference, duplicate service member 같은 모호하거나 깨진 group config를 거부한다.
- Group aggregate status/plan total은 active service만 합산하고, inactive member는
  JSON의 skipped per-service row로 유지한다.
- Group plan JSON은 `conflicts`를 scalar로 overload하지 않고 `conflict_count`와
  service-keyed `conflicts` object를 보고한다.
- Human `group status` output은 `root_exists`를 표시해 missing root와 empty included
  file set을 구분한다.

### 수정

- `bootstrap check`는 이제 `Path::try_exists()`로 service-root inspection I/O error를
  보존하고, 접근 불가 path를 단순 missing root로 보고하지 않는다.

### 포함하지 않음

- Group backup, group restore, 기타 batch mutation flow는 읽기 전용 group
  status/plan surface의 안전성이 검증될 때까지 scope 밖으로 둔다.

## v0.4.0 - 2026-05-22

### 추가

- 일반 service config 위의 public app-catalog surface인 `lattice app list/show/add`.
- 새 머신 readiness check를 위한 human/JSON `lattice bootstrap check`.
- Backup이나 restore 전 preferred preflight surface인 human/JSON `lattice plan`.
- Forced restore snapshot을 점검하고 rollback을 dry-run하며 history를 보수적으로
  prune하기 위한 `lattice snapshot list/show/prune`과 `lattice undo`.
- Config를 변경하지 않고 local service candidate를 찾는 human/JSON `lattice discover`.
- CLI help와 유지 문서가 app/service 용어를 유지하고 예전 catalog wording으로
  drift하지 않도록 `cargo run -p xtask -- verify`에 product-surface verification 추가.

### 변경

- `lattice init`은 이제 tool-specific service를 기본으로 만들지 않고 generic Lattice
  config/storage directory를 만든 뒤 안전한 다음 bootstrap command를 출력한다.
- README와 user docs는 app entry를 optional shortcut으로 유지하면서 app/service 예시에서
  시작한다.
- v0.4 release command surface에 맞춰 workspace package version을 `0.4.0`으로 올렸다.

### 제거

- 예전 public catalog command/flag wording을 `app`과 generic service config로 대체했다.

## v0.3.3

### 수정

- CLI는 app, bootstrap, plan, snapshot, undo, discover, group surface에 대해 실행 가능한
  help를 출력한다.

## v0.3.2

### 수정

- Service config parsing과 path handling이 backup/restore 전에 더 많은 malformed input을
  거부한다.

## v0.3.1

### 수정

- Restore safety check가 conflict detection 중 unsafe path를 따라가지 않는다.

## v0.3.0

### 추가

- 초기 service-oriented backup, restore, diff, status, repo, include, exclude,
  permission, hook, validation surface.
