# Lattice 제품 범위

[English](mvp-scope.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

## 제품 포지션

Lattice는 가벼운 Rust dotfiles manager다. 명시적인 TOML 설정, 예측 가능한
XDG 저장 위치, permission 보존, 안전한 restore 동작을 기반으로 service 단위
파일과 디렉터리를 관리한다.

Lattice는 full system configuration manager, package manager, secret manager가
아니다. 핵심 제품은 작게 유지한다: scan, plan, backup, restore, 좁게 설정된
lifecycle hook 실행.

## 현재 릴리스: v0.3.1

v0.3.1은 일반적인 개인 dotfiles 관리에 사용할 수 있는 첫 릴리스 라인이다.
v0.2의 safety layer, 첫 CLI-first 관리 계층, 실제 Codex 설정 백업에 필요한
빈 디렉터리 보존 fix를 포함한다.

현재 범위:

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

## 의도적으로 하지 않는 것

- crates.io publish.
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

v0.3.1은 다음 조건을 만족하면 release-ready다.

- `cargo run -p xtask -- verify` 통과.
- `cargo run -p xtask -- linux-verify` 통과.
- `cargo run -p xtask -- quality` 통과.
- `actionlint .github/workflows/ci.yml` 통과.
- `git diff --check` 통과.
- `cargo install --path crates/lattice-cli` path install smoke 통과.
- GitHub Actions의 Linux x86_64, Linux ARM64, macOS Apple Silicon, quality job
  통과.
- `v0.3.1` push 이후 tag install smoke 통과.
