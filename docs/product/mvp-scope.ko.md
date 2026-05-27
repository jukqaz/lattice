# Lattice 제품 범위

[English](mvp-scope.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

## 제품 포지션

Lattice는 이 제품 라인의 canonical dotfiles manager다. 작은 Rust CLI로,
명시적인 TOML 설정, 예측 가능한 XDG 저장 위치, permission 보존, 안전한
restore 동작을 기반으로 service 단위 파일과 디렉터리를 관리한다.

이전 dotfiles-manager 실험들은 병렬 제품이 아니라 검증된 범용 아이디어를
Lattice로 흡수하는 feature-mining source로 둔다. 핵심 제품은 작게 유지한다:
scan, plan, backup, restore, diff, 좁게 설정된 lifecycle hook 실행.

Lattice는 full system configuration manager, package manager, secret manager가
아니다. 특정 tool 하나를 중심으로 제품을 만들지 않는다. 제품 용어로 흔한 관리
대상을 **앱**이라고 부른다. 앱 지식은 범용 dotfile-management workflow를 개선할
때만 선택적 app catalog에 둔다. Codex는 제품의 중심이 아니라 예시 앱 중 하나다.

## 릴리스된 기준선: v0.3.3

v0.3.3은 일반적인 개인 dotfiles 관리에 사용할 수 있는 첫 릴리스 라인이다.
v0.2의 safety layer, 첫 CLI-first 관리 계층, 빈 디렉터리 보존, portable
filesystem safety check를 포함한다.

릴리스된 v0.3.3 범위:

- `lattice-core`, `lattice` CLI, `xtask`로 나뉜 Rust workspace.
- XDG-aware config, data, state, cache path.
- global TOML config와 per-service TOML config.
- service root와 optional repo path.
- 기본 service repo 위치: `$XDG_DATA_HOME/lattice/repos/<service>`.
- include/exclude glob.
- 흔한 dotfile layout을 위한 선택적 app catalog entry.
- `init`, `doctor`, `validate`, `service list/show/add/remove`, `status`.
- `include add/remove`, `exclude add/remove`, `permission set/remove`.
- `backup`, `backup --dry-run`, `restore`, `restore --dry-run`,
  `restore --force`.
- backup manifest와 restore의 빈 디렉터리 추적.
- permission manifest capture와 restore.
- restore conflict detection과 overwrite 전 XDG state snapshot.
- restore-time secure directory creation.
- confirmation과 timeout을 지원하는 minimal lifecycle hook.
- backup 전 secret-looking content guard.
- case-insensitive 및 Unicode-normalized name을 위한 portable path collision
  check.
- root/repo overlap rejection.
- hard link, extended attribute, macOS resource fork를 위한 metadata-loss
  guard와 명시적 `--allow-metadata-loss` bypass.
- secret 값을 읽지 않는 `rbw`, `bw` secret metadata command.
- `repo status/pull/commit/push` git repo command.
- 기존 file을 service에 가져오는 `track`, `adopt`.
- binary redaction과 template-aware output을 포함한 `diff`.
- optional symlink restore mode.
- OS/hostname service condition.
- restore 시 단순 environment-variable template rendering.
- 같은 config model 위에서 동작하는 prompt-based TUI.
- Rust-only `xtask` verification, Linux Docker verification, quality gate.
- Linux x86_64, Linux ARM64, macOS Apple Silicon, dependency/coverage/typo
  quality check를 도는 GitHub Actions.

## 릴리스된 automation 라인: v0.4.0

v0.4.0은 안전한 개인 backup 기준선 위에 automation-friendly surface를 추가한다.

- `lattice init`은 tool-specific service를 기본 생성하지 않고 범용 Lattice config와
  storage directory만 만든다.
- service별 status, file count, root path, repo path, action summary를 보여주는
  풍부한 `lattice tui --dry-run` dashboard.
- best-effort TUI dashboard 동작: 한 service의 root/repo가 unavailable이어도
  다른 service와 action 목록은 계속 출력.
- `status`, `plan`, `backup --dry-run`, `diff`, `restore --dry-run`,
  `bootstrap check`, `snapshot`, `undo`, `discover`의 machine-readable JSON output.
- `plan`을 backup/restore 전 단일 human/JSON preflight surface로 둔다.
- `bootstrap check`는 새 머신 readiness diagnostic을 제공한다.
- `app list`, `app show <app>`, `app add <app>`를 app catalog command
  surface로 둔다.
- `snapshot list/show/prune`과 `undo`로 forced-restore history 확인, rollback
  dry-run, 보수적인 cleanup을 지원한다.
- `discover`로 config mutation 없이 보수적인 local service 후보를 제안한다.
- status, plan, backup, diff, restore flow의 `--only`, `--exclude` path selector.
- JSON, selector, app-catalog, bootstrap contract를 고정하는 CLI smoke와
  product-surface harness coverage.

## 현재 릴리스: v0.5.1

v0.5.1은 service-groups release line을 harden한다. Group은 기존 service의 named
bundle이며, 두 번째 service type이나 app catalog redesign이 아니다.

v0.5.1 범위:

- Global config 안의 service group. Group은 기존 service를 순서 있는 named bundle로
  묶는다.
- 읽기 전용 multi-service 점검과 dry-run planning을 위한 `group list`,
  `group show`, `group status`, `group plan`.
- 모든 group command의 machine-readable JSON output.
- Single-service flow와 같은 selector semantics를 재사용하는 `group status`,
  `group plan` path selector.
- Group invariant validation: unique group name, non-empty group, known service
  reference, duplicate service member 금지.
- 현재 host에서 실행 가능한 total을 위해 active-only aggregate를 사용하고,
  inactive member는 JSON의 skipped per-service row로 유지.
- Numeric `conflict_count`와 service-keyed structured `conflicts`를 포함한 group
  plan JSON.
- Human `group status`의 `root_exists` output으로 missing-root visibility 제공.
- Tampered metadata, symlink traversal attempt, partial restore prevention을 위한
  CLI-level restore/manifest/snapshot safety regression.
- Secret/auth/session/cache/database exclusion을 human/JSON output의
  suggestion-level warning으로 보여주는 보수적인 `discover` output. 모든 파일이
  제외된 warning-only candidate도 포함한다.
- `undo --dry-run`은 성공을 보고하기 전에 restore preflight를 실행해서 실제 snapshot
  undo blocker와 dry-run 실패가 일치하게 한다.
- 실제 HOME read-only health check는 `LATTICE_BIN` 또는 기존 `target/debug/lattice`
  binary를 요구하며 `cargo run` fallback을 사용하지 않는다.
- Real HOME에서 restore하기 전에 read-only discovery, planning, reviewed backup부터
  시작하도록 문서화한 safe first-adoption playbook.
- `wasm32-wasip2` non-Unix `lattice-core` compile check를 위한 CI coverage.
- Group help, docs, JSON example, read-only command exposure, release docs,
  unsupported `group backup` / `group restore`를 고정하는 product-surface harness coverage.

Group backup, group restore, 기타 batch mutation flow는 읽기 전용 group
status/plan surface의 안전성이 검증될 때까지 의도적으로 scope 밖이다.

## 로드맵

| 라인 | 이름 | 목표 | 완료 기준 |
| --- | --- | --- | --- |
| `v0.3.x` | Safe Personal Backup | 개인 dotfiles를 안전하게 backup/restore. | full safety harness, platform CI, install smoke, v0.3.3 tag smoke 통과. |
| `v0.4.x` | Automation, Bootstrap, Recovery, And Discovery | script와 agent가 human stdout parsing 없이 Lattice를 호출하고, 새 머신 restore, recovery history, 보수적 discovery를 first-class로 만든다. | generic init, JSON output, selector, `plan`, `bootstrap check`, `app` command, snapshot/undo, `discover`, product-surface harness coverage가 v0.4.0 release line에 문서화되고 테스트됨. |
| `v0.5.x` | Service Groups | Batch mutation 없이 관련 service를 함께 inspect/plan. | `group list/show/status/plan`, JSON output, selector, group invariant validation, active-only aggregate, missing-root visibility를 group backup/restore 동작보다 먼저 문서화하고 테스트. |
| `v1.0` | Public Stable CLI | 외부 사용자에게 추천 가능한 안정 CLI. | install, changelog, release, migration, change policy, issue workflow 안정화. |

## 의도적으로 하지 않는 것

- public stable line 전 crates.io publish.
- automatic remote repository creation.
- automatic package installation.
- secret value materialization from `rbw` or `bw`.
- full plugin system.
- Home Manager 또는 Nix-style declarative program module.
- GUI.
- database-backed state.
- generic dotfile manager 안의 tool-specific product feature.

## 설정 형태

Service config는 읽기 쉬운 TOML을 유지한다.

```toml
name = "shell"
root = "~/.config/shell"
include = ["config.toml", "scripts/**"]
exclude = ["cache/**", "state/**"]

[restore]
create_dirs = [
  { path = "cache", mode = "0700" },
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[hooks.after_restore]]
name = "reload shell config"
command = "/bin/sh"
args = ["-c", "true"]
timeout_sec = 60
confirm = false
```

`repo`를 생략하면 Lattice는 `$XDG_DATA_HOME/lattice/repos/<service-name>`을
사용한다. custom repository path가 필요할 때만 `repo`를 명시한다.

## 릴리스 완료 기준

모든 release candidate는 다음 조건을 만족하면 release-ready다.

- `cargo run -p xtask -- verify` 통과.
- release-oriented change에서는 `cargo run -p xtask -- linux-verify` 통과.
- `cargo run -p xtask -- quality` 통과.
- workflow 변경 시 `actionlint .github/workflows/ci.yml` 통과.
- `git diff --check` 통과.
- `cargo install --path crates/lattice-cli` path install smoke 통과.
- GitHub Actions의 Linux x86_64, Linux ARM64, macOS Apple Silicon, quality job
  통과.
- release tag push 이후 tag install smoke 통과.
