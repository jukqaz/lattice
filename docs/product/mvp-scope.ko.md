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
- secret 값을 읽지 않는 `rbw`, `bw`, env passthrough reference용 secret
  metadata command.
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

## 현재 릴리스: v0.8.1

v0.8.1은 v0.8 maintainability와 modularization 라인의 post-review patch
release다. v0.8.0 CLI 동작은 유지하면서 release-check changelog validation을
요청한 release section으로 제한하는 review follow-up을 배포한다.

v0.8.1 범위:

- Release-check changelog assertion이 요청한 release section만 읽어, 이전
  release note가 현재 release contract를 대신 만족하지 못하게 한다.
- Version metadata, install snippet, changelog, product scope, TODO,
  product-surface harness expectation을 v0.8.1 patch release에 맞춘다.
- v0.8.0에서 modularized된 command surface가 현재 user-facing CLI contract로
  유지된다.

## 이전 릴리스: v0.8.0

v0.8.0은 maintainability와 modularization 라인을 완료한다. CLI 동작은 유지하면서
parser, smoke harness, command runner, output helper, config storage, runtime probe,
공유 service state를 검토와 확장이 쉬운 focused file로 분리했다.

v0.8.0 범위:

- CLI parser 정의와 command runner를 큰 `main.rs` 대신 focused module에 둔다.
- Domain별 smoke coverage를 shared isolated XDG support harness를 재사용하는
  focused integration test로 분리한다.
- 공유 JSON/human output helper, config persistence, runtime probe, service state,
  setup command, service CRUD, backup/restore/status/diff flow를 전용 모듈로 분리한다.
- `scripts/lint.sh`, `cargo run -p xtask -- verify`,
  `cargo run -p xtask -- quality`가 release-line quality gate를 담당한다.
- README, user docs, product scope, TODO, changelog, product-surface harness
  expectation을 v0.8.0 release line에 맞춘다.

## 이전 릴리스: v0.7.0

v0.7.0은 Lattice를 secret manager가 아니라 dotfiles/config manager로 유지하면서
env secret passthrough metadata를 추가한다. API key, token, password 값은 repo 밖에
두고 environment-variable reference와 restore template만 관리한다.

v0.7.0 범위:

- Secret metadata command가 `rbw`, `bw`와 함께 `env` passthrough reference를
  지원하며 secret 값 대신 environment-variable 이름을 저장한다.
- `secret check`는 env reference를 `set`, `unset`, `missing-env-reference`로
  보고하고 `value=not-read`를 유지한다.
- README, user docs, product scope, TODO, changelog, product-surface harness
  expectation을 v0.7.0 release line에 맞춘다.

## 이전 릴리스: v0.6.0

v0.6.0은 기존 command surface 전반의 automation contract를 harden한다. Batch
mutation이나 remote bootstrap behavior를 추가하지 않고, 문서화된 JSON shape를 확장하고
fixture 기반 contract coverage와 release-check helper를 추가한다.

v0.6.0 범위:

- v0.5의 service-groups 읽기 전용 모델은 그대로 유지한다: `group list`,
  `group show`, `group status`, `group plan` only.
- JSON output reference coverage를 group 외에도 `bootstrap check`, single-service
  `status`/`plan`, dry-run `backup`/`restore`, `diff`, snapshot list/show/prune,
  `undo --dry-run`, `discover`까지 확장한다.
- Fixture 기반 CLI smoke coverage가 bootstrap, service, group, discovery,
  snapshot, undo automation surface의 top-level JSON key와 중요한 nested field name을
  고정한다.
- `discover`는 top-level `next_actions`와 suggestion-level `next_command`를 노출하되,
  app catalog root/include contract가 명시적으로 검토되지 않은 app-named discovery
  hint는 service-scoped로 유지한다.
- `cargo run -p xtask -- release-check`는 tag 전 version metadata, changelog/install
  snippet, locked metadata, path install, installed binary smoke를 검증한다.
- README, user docs, product scope, TODO, changelog, product-surface harness
  expectation을 v0.6.0 release line에 맞춘다.

Group backup, group restore, 기타 batch mutation flow, automatic remote repo
creation, package installation, MCP prototype, crates.io publish는 의도적으로 scope 밖이다.

## 로드맵

| 라인 | 이름 | 목표 | 완료 기준 |
| --- | --- | --- | --- |
| `v0.3.x` | Safe Personal Backup | 개인 dotfiles를 안전하게 backup/restore. | full safety harness, platform CI, install smoke, v0.3.3 tag smoke 통과. |
| `v0.4.x` | Automation, Bootstrap, Recovery, And Discovery | script와 agent가 human stdout parsing 없이 Lattice를 호출하고, 새 머신 restore, recovery history, 보수적 discovery를 first-class로 만든다. | generic init, JSON output, selector, `plan`, `bootstrap check`, `app` command, snapshot/undo, `discover`, product-surface harness coverage가 v0.4.0 release line에 문서화되고 테스트됨. |
| `v0.5.x` | Service Groups | Batch mutation 없이 관련 service를 함께 inspect/plan. | `group list/show/status/plan`, JSON output, selector, group invariant validation, active-only aggregate, missing-root visibility를 group backup/restore 동작보다 먼저 문서화하고 테스트. |
| `v0.6.x` | Automation Contract Hardening | 기존 machine-readable surface를 script와 agent가 신뢰할 수 있게 만든다. | JSON reference coverage, fixture-based contract test, release-check automation, release docs를 batch mutation 없이 정렬. |
| `v0.7.x` | Secret Passthrough References | Secret 값을 repo 밖에 두면서 env reference를 명시적으로 점검 가능하게 만든다. | `env` secret metadata, restore-time `{{env:NAME}}` guidance, non-disclosure smoke coverage, bilingual docs 정렬. |
| `v0.8.x` | Maintainability And Modularization | Pre-1.0 CLI를 동작 변경 없이 검토와 확장이 쉬운 구조로 만든다. | Parser, command module, smoke domain, output helper, release docs, xtask structure contract 정렬. |
| `v0.9.0` | Public Readiness | Core backup/restore 동작을 바꾸지 않고 새 외부 사용자가 이해하고 설치할 수 있게 만든다. | Public positioning, 첫 15분 walkthrough, install/update/rollback docs, migration docs, change policy, issue template, home/work guidance 정렬. |
| `v0.9.1` | Release Candidate Hardening | Stable freeze 전에 public-ready surface를 dogfood하고 contract ambiguity를 제거한다. | Command/help/JSON contract audit, real-HOME read-only dogfood, safety review, optional context inactive-reason coverage, accepted read-only audit/guidance surface 완료. |
| `v1.0.0` | Public Stable CLI | 기존 safety-first command/config/JSON contract를 외부 사용자용으로 freeze한다. | Version/docs/changelog/release-check 정렬, stability reference, local/CI gate, GitHub Release, tag install smoke, Linear closeout을 명시 승인 후 완료. |

## v1.0 제품 정의

Public Stable CLI는 기존 safety-first 제품의 command, config, JSON contract를
안정화하고, 외부 사용자가 repository history를 읽지 않아도 시도할 수 있는 onboarding과
release hygiene를 갖춘 상태를 뜻합니다. `v1.0.0` 경로는 큰 feature line이 아니라
stabilization line입니다.

`v1.0.0` 배포 정책 권장안:

- `cargo install --git https://github.com/jukqaz/lattice lattice --tag vX.Y.Z --locked`를
  canonical install path로 유지합니다.
- Rust 없는 설치 경로가 필요하면 CI-covered target용 GitHub Release binary archive를
  선택적으로 고려합니다.
- v1.0에서는 crates.io publish를 하지 않습니다. 현재 project policy는 git-distributed이고
  `lattice` crate name은 이미 사용 중입니다.

Home/work 지원은 v1.0 경로에서 작고 명시적으로 유지합니다. 오늘은 기존 OS/hostname
service condition과 read-only group을 사용합니다. Stable freeze 전에 context feature를
받아들이면, local label인 `contexts = ["work"]`, 그 label을 match하는 service condition,
read-only context inspection command, machine-readable inactive reason 정도로 제한합니다.
Per-file alternate suffix, full conditional template language, secret value materialization,
package/app installation은 추가하지 않습니다.

## 의도적으로 하지 않는 것

- public stable line 전 crates.io publish.
- automatic remote repository creation.
- automatic package installation.
- secret value materialization from `rbw` or `bw`.
- full plugin system.
- Home Manager 또는 Nix-style declarative program module.
- yadm-style per-file alternate 또는 chezmoi-style full conditional template.
- context-selected secret value 또는 context-driven package/app installation.
- batch group backup/restore mutation.
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

[conditions]
os = "linux"
hostname = "workstation"
# Stable freeze 전에 받아들이는 경우의 optional v1.0-path decision:
# contexts = ["work"]

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
