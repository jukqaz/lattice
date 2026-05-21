# Lattice 제품 범위

[English](mvp-scope.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

## 제품 포지션

Lattice는 이 제품 라인의 canonical dotfiles manager다. 작은 Rust CLI로,
명시적인 TOML 설정, 예측 가능한 XDG 저장 위치, permission 보존, 안전한
restore 동작을 기반으로 service 단위 파일과 디렉터리를 관리한다.

이전 dotfiles-manager 실험들은 병렬 제품이 아니라 검증된 아이디어를 Lattice로
흡수하는 feature-mining source로 둔다. 핵심 제품은 작게 유지한다: scan,
plan, backup, restore, diff, 좁게 설정된 lifecycle hook 실행.

Lattice는 full system configuration manager, package manager, secret manager가
아니다. Codex-specific 기능은 core를 Codex-only tool로 만들지 말고 preset,
docs, `doctor` check에 얇게 얹는다.

## 현재 기준선: v0.3.3

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
- `codex`, `git`, `zsh`, `mise`, `ssh` preset.
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

## v0.3.3 이후 main branch

현재 main branch는 v0.4 후보 라인이다. 안전한 개인 backup 기준선 위에
automation-friendly surface를 추가한다.

- service별 status, file count, root path, repo path, action summary를 보여주는
  풍부한 `lattice tui --dry-run` dashboard.
- best-effort TUI dashboard 동작: 한 service의 root/repo가 unavailable이어도
  다른 service와 action 목록은 계속 출력.
- `status`, `backup --dry-run`, `diff`, `restore --dry-run`의 machine-readable
  JSON output.
- status, backup, diff, restore flow의 `--only`, `--exclude` path selector.
- JSON과 selector contract를 고정하는 CLI smoke coverage.

## 로드맵

| 라인 | 이름 | 목표 | 완료 기준 |
| --- | --- | --- | --- |
| `v0.3.x` | Safe Personal Backup | 개인 dotfiles를 안전하게 backup/restore. | full safety harness, platform CI, install smoke, v0.3.3 tag smoke 통과. |
| `v0.4.x` | Automation-Friendly CLI | script와 agent가 human stdout parsing 없이 Lattice를 호출. | JSON output과 selector가 문서화되고 CI/Hermes 사용에 충분히 안정적. |
| `v0.5.x` | New Machine Bootstrap | 새 머신에서 developer home baseline을 몇 분 안에 restore. | 새 VM/Mac에서 install, init, preset enable, repo pull, dry-run restore, restore가 명확한 진단과 함께 동작. |
| `v0.6.x` | Codex Baseline | core를 키우지 않고 Codex power-user 지원을 얇게 추가. | Codex preset/docs/doctor check가 흔한 config risk를 다루고 secrets/runtime state는 Lattice 밖에 둠. |
| `v0.7.x` | Service Groups | 여러 service 작업을 안전하게 plan/run. | group status와 dry-run plan이 명확하고 보수적이며 machine-readable. |
| `v1.0` | Public Stable CLI | 외부 사용자에게 추천 가능한 안정 CLI. | install, changelog, release, migration, compatibility, issue workflow 안정화. |

## 의도적으로 하지 않는 것

- public stable line 전 crates.io publish.
- automatic remote repository creation.
- automatic package installation.
- secret value materialization from `rbw` or `bw`.
- full plugin system.
- Home Manager 또는 Nix-style declarative program module.
- GUI.
- database-backed state.

## 설정 형태

Service config는 읽기 쉬운 TOML을 유지한다.

```toml
name = "codex"
root = "~/.codex"
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
