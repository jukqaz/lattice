# Lattice

[English](README.md) | 한국어

Lattice는 dotfiles를 service 단위로 백업하고 복원하는 작은 Rust CLI입니다.
각 도구마다 root, include 규칙, restore 권한, 선택적 sync repository를 따로
둘 수 있게 만드는 개인 설정 관리자입니다.

Lattice는 범용 도구입니다. 특정 tool이나 service 하나가 제품의 중심이 아닙니다.
관리하려는 service를 직접 만들고, 같은 안전한 workflow로 backup/restore합니다.

제품 용어로는 `git`, `ssh`, `zsh`, `starship`, `mise`, `codex` 같은 관리
대상을 **앱**이라고 부릅니다. 앱은 제품의 중심이 아니라, 일반 service 설정으로
확장되는 catalog entry입니다. CLI는 이 surface를 `app`으로 직접 이름 붙이고,
낡은 preset 용어를 끌고 가지 않습니다.

## 먼저 할 일

최신 tagged release 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.3 --locked
```

로컬 설정 만들기:

```bash
lattice init
lattice doctor
lattice validate
```

첫 service를 명시적으로 추가합니다. `shell`, `~/.config/shell`, `config.toml`은
관리하려는 앱이나 도구 설정에 맞게 바꿉니다.

```bash
lattice service add shell --root ~/.config/shell --include config.toml
lattice service show shell
lattice status shell
```

처음에는 실제 백업 전에 계획만 확인합니다.

```bash
lattice backup --dry-run shell
```

대상이 맞으면 백업합니다.

```bash
lattice backup shell
```

## 안전하게 복원하기

복원도 먼저 dry-run으로 확인합니다.

```bash
lattice restore --dry-run shell
```

예상 밖 충돌이 없으면 복원합니다.

```bash
lattice restore shell
```

로컬 파일을 의도적으로 덮어쓰려면 `--force`를 사용합니다. Forced restore는
쓰기 전에 XDG state 아래에 덮어쓸 파일 snapshot을 남깁니다.

```bash
lattice restore --force shell
```

## 서비스 더 추가하기

Lattice는 설정을 service 단위로 관리합니다. Service에는 root directory,
include/exclude 규칙, 권한, 선택적 repo path가 있습니다. `repo`를 생략하면
`$XDG_DATA_HOME/lattice/repos/<service-name>`에 저장합니다.

```bash
lattice service add <service> --root <path> --include <pattern>
lattice service show <service>
lattice backup --dry-run <service>
lattice backup <service>
```

이미 알려진 형태는 app catalog를 사용합니다.

```bash
lattice app list
lattice app show <app>
lattice app add <app> --root <path>
```

## 자주 쓰는 명령

| 목적 | 명령 |
| --- | --- |
| 설치와 외부 도구 점검 | `lattice doctor` |
| 설정 파일 검증 | `lattice validate` |
| 서비스 상태 확인 | `lattice status shell` |
| 백업 미리보기 | `lattice backup --dry-run shell` |
| 백업 실행 | `lattice backup shell` |
| 복원 미리보기 | `lattice restore --dry-run shell` |
| 복원 실행 | `lattice restore shell` |
| 로컬 파일과 repo copy 비교 | `lattice diff shell` |
| prompt 기반 UI 열기 | `lattice tui` |

## Automation과 JSON output

Script나 agent가 사람이 읽는 stdout을 parsing하지 않게 하려면 `--json`을
사용합니다.

```bash
lattice status --json shell
lattice backup --dry-run --json shell
lattice diff --json shell
lattice restore --dry-run --json shell
```

특정 tracked path만 계획하려면 `--only`와 `--exclude`를 사용합니다. 이 selector는
`status`, `backup`, `diff`, `restore` flow에서 사용할 수 있습니다.

```bash
lattice status --json --only config.toml shell
lattice backup --dry-run --json --only config.toml shell
lattice diff --json --exclude 'cache/**' shell
lattice restore --dry-run --json --only config.toml shell
```

Automation에서는 쓰기 작업 전에 dry-run JSON 명령을 먼저 사용합니다. 계획의
`files`, `dirs`, `entries`, `conflicts` field를 확인한 뒤 괜찮을 때만 non-dry-run
명령을 실행합니다.

## Git으로 동기화하기

Service repo는 일반 directory입니다. 직접 Git을 써도 되고, 내장 helper를
써도 됩니다.

```bash
lattice repo status shell
lattice repo pull shell
lattice repo commit --message "backup shell config" shell
lattice repo push shell
```

private GitHub repo를 쓸 때는 remote repository를 직접 만든 뒤 일반
`git remote` 명령으로 service repo에 연결하면 됩니다.

## 안전 모델

- 명백한 secret 형태의 내용은 `--allow-secret-looking-files` 없이는 백업을
  막습니다.
- secret command는 metadata만 저장합니다. secret 값을 읽거나 출력하거나
  백업하지 않습니다.
- restore는 `--force` 없이는 충돌하는 local file을 덮어쓰지 않습니다.
- forced restore는 덮어쓰기 전에 snapshot을 만듭니다.
- restore path, manifest entry, permission rule은 service root 내부에 있어야
  합니다.
- service root와 service repo는 서로 겹치면 안 됩니다.
- 추적 path는 portable UTF-8이어야 하며 control character를 포함하거나 Unicode
  normalization과 case folding 이후 충돌하면 안 됩니다.
- restore 안전을 위해 source symlink와 symlink된 destination parent는
  거부합니다.
- backup은 regular file과 include된 empty directory를 추적합니다. symlink,
  socket, FIFO 같은 특수 filesystem entry는 file content로 따라가지 않습니다.
- copy backup이 보존하지 못하는 hard link, extended attribute, macOS resource
  fork는 기본적으로 거부합니다. 해당 파일을 직접 검토한 뒤에만
  `--allow-metadata-loss`를 사용합니다.
- forced restore는 기존 특수 filesystem entry를 metadata snapshot으로 남긴 뒤
  추적된 directory로 교체합니다.

## 설정 위치

Lattice는 XDG 위치를 사용합니다.

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
~/.local/share/lattice/repos/*
~/.local/state/lattice/
~/.cache/lattice/
```

`XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME`으로
위치를 바꿀 수 있습니다.

## 서비스 설정 예시

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

[[permissions]]
path = "scripts/sync"
mode = "0700"
```

수동 편집 대신 `lattice service add`, `lattice include`, `lattice exclude`,
`lattice permission`, `lattice secret`, `lattice track` 명령으로 service TOML을
관리할 수 있습니다.

## 문서

처음 보는 사용자는 이 순서로 읽으면 됩니다.

1. [사용자 가이드](docs/user/usage.ko.md)
2. [제품 범위](docs/product/mvp-scope.ko.md)
3. [변경 로그](CHANGELOG.ko.md)
4. [문서 인덱스](docs/README.ko.md)

영어 문서:

1. [User Guide](docs/user/usage.md)
2. [Product Scope](docs/product/mvp-scope.md)
3. [Changelog](CHANGELOG.md)
4. [Documentation Index](docs/README.md)

LLM 전용 에이전트 지침은 [docs/llm](docs/llm/) 아래에 분리되어 있습니다.

## 개발

checkout에서 설치:

```bash
cargo install --path crates/lattice-cli
```

일반 검증 harness:

```bash
cargo run -p xtask -- verify
```

Docker 기반 Linux 검증:

```bash
cargo run -p xtask -- linux-verify
```

릴리스 전 무거운 품질 게이트:

```bash
cargo run -p xtask -- quality
```

Workspace 구조:

```text
crates/lattice-core/  core config, scan, backup, restore, hook, path logic
crates/lattice-cli/   `lattice` binary and CLI smoke tests
xtask/                development verification harness
```
