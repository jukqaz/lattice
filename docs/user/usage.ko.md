# 사용자 가이드

[English](usage.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

이 문서는 Lattice를 직접 사용하는 사람을 위한 가이드입니다. LLM 전용 작업
규칙은 [docs/llm](../llm/) 아래에 있습니다.

## Lattice가 관리하는 것

Lattice는 이름이 붙은 service의 선택된 파일을 백업하고 복원합니다. Service는
하나의 도구나 앱 설정 root를 뜻합니다. 이 문서의 service 이름은 범용 예시이며,
실제로는 자신의 dotfiles에 맞는 이름과 path를 사용하면 됩니다.

각 service는 다음을 가질 수 있습니다.

- root directory
- include할 파일과 디렉터리
- 제외할 path
- restore 때 보존할 권한
- 선택적 OS/hostname 조건
- 선택적 Git repo 위치
- 선택적 restore hook

Lattice는 package manager, secret manager, full system configuration manager가
아닙니다. dotfile sync 계층을 작고 명시적으로 유지합니다.

용어: **앱**은 `git`, `ssh`, `zsh`, `starship`, `mise`, `codex`처럼 흔히
관리하는 대상입니다. App catalog entry는 일반 service config를 만들기 위한
shortcut일 뿐입니다. 어떤 앱도 제품을 정의하지 않으며, Codex도 예시 앱 중
하나일 뿐입니다. CLI는 이 catalog surface에 `lattice app ...`을 직접 사용합니다.

## 1. 설치

이 가이드에 문서화된 현재 v0.4 후보 command surface 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --branch main --locked
```

최신 tagged stable 기준선은 아직 `v0.3.3`입니다. v0.4.0 tag를 자르기 전에
`app`, `plan`, `bootstrap check`를 테스트하려면 current branch나 local checkout을
사용합니다.

Lattice를 개발 중이면 local checkout에서 설치:

```bash
cargo install --path crates/lattice-cli
```

설치된 binary 확인:

```bash
lattice --version
```

## 2. 로컬 설정 만들기

Global config와 저장 directory를 만듭니다.

```bash
lattice init
```

환경을 확인합니다.

```bash
lattice doctor
lattice validate
lattice bootstrap check
lattice service list
```

기본 설정 파일 위치:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
```

Service에서 `repo`를 생략하면 backup copy는 여기에 저장됩니다.

```text
~/.local/share/lattice/repos/<service-name>
```

## 3. 첫 app-backed service 추가하기

Catalog가 흔한 include/exclude 형태를 알고 있는 앱 설정 directory 하나를 service로 만듭니다. 예시 앱과 root는 관리하려는 dotfiles에 맞게 바꿉니다.

```bash
lattice app list
lattice app show zsh
lattice app add zsh --root ~/.config/zsh
lattice plan zsh
```

나중에 추적 path를 추가하거나 제거합니다.

```bash
lattice include add shell scripts/**
lattice include remove shell scripts/**
lattice exclude add shell cache/**
lattice exclude remove shell cache/**
```

복원 권한 보존:

```bash
lattice permission set shell config.toml 0600
lattice permission remove shell config.toml
```

기존 파일을 service로 가져오기:

```bash
lattice track shell config.toml
lattice adopt shell scripts/sync
```

## 4. 첫 백업 만들기

항상 단일 preflight plan을 먼저 확인합니다.

```bash
lattice plan zsh
lattice backup --dry-run zsh
```

출력에는 copy될 파일과 추적할 빈 디렉터리가 나옵니다. Service repo에 쓰기
전에 이 목록을 확인합니다.

백업 실행:

```bash
lattice backup zsh
```

Live service root와 repo copy 사이 drift 확인:

```bash
lattice diff zsh
```

출력이 없으면 추적 대상 파일 기준으로 내용 차이가 없다는 뜻입니다.

## 5. 안전하게 복원하기

복원도 먼저 plan surface로 preview합니다.

```bash
lattice plan zsh
lattice restore --dry-run zsh
```

충돌을 덮어쓰지 않고 복원:

```bash
lattice restore zsh
```

Local file 충돌이 있고 repo copy를 의도적으로 우선하려면 `--force`를 사용합니다.

```bash
lattice restore --force zsh
```

Forced restore는 쓰기 전에 덮어쓸 파일을 XDG state 아래에 snapshot으로 남깁니다.

```text
~/.local/state/lattice/snapshots/
```

## 6. 백업 repo 동기화

Service repo는 일반 directory입니다. Git을 직접 써도 되고 Lattice helper를 써도
됩니다.

```bash
lattice repo status shell
lattice repo pull shell
lattice repo commit --message "backup shell config" shell
lattice repo push shell
```

Private GitHub repository를 쓸 때:

1. private remote repository를 만듭니다.
2. service repo에 remote를 추가합니다.
3. backup commit 이후 `lattice repo push <service>`를 실행합니다.

Lattice는 remote repository를 자동 생성하지 않습니다. credential, 접근 권한,
repository 소유권을 명시적으로 관리하기 위해서입니다.

## 7. App catalog 사용하기

App catalog entry는 흔한 도구와 앱의 include/exclude 형태를 제공합니다.

```bash
lattice app list
lattice app show <app>
```

앱 기반 service 생성:

```bash
lattice app add <app> --root <path>
```

앱은 선택적 shortcut입니다. Core model은 여전히 같은 service config,
include/exclude 규칙, backup, diff, restore flow입니다.

## 8. Secret 안전하게 다루기

Lattice는 secret 값을 백업하지 않습니다. Secret command는 backend, item,
field, environment variable name, folder 같은 metadata만 저장합니다.

Secret metadata 추가:

```bash
lattice secret add --backend rbw --item "<vault item>" --field password --env <ENV_NAME> <service> <name>
```

Metadata 목록과 상태 확인:

```bash
lattice secret list <service>
lattice secret check <service>
```

`secret check`는 `rbw`, `bw` 같은 backend tool 사용 가능 여부만 확인하고,
secret 값을 읽거나 출력하지 않습니다.

백업은 명백한 secret 형태의 파일 내용도 기본적으로 막습니다. 파일을 직접 검토한
뒤에만 `--allow-secret-looking-files`를 사용합니다.

```bash
lattice backup --allow-secret-looking-files <service>
```

Backup은 regular file과 include된 empty directory를 scan합니다. symlink를
따라가지 않으며 socket, FIFO, device file 같은 특수 filesystem entry를 file
content로 복사하지 않습니다.

Service root와 repo는 서로 겹치면 안 됩니다. 추적 path는 portable UTF-8이어야
하고 control character를 포함하거나 Unicode normalization과 case-insensitive comparison 이후
충돌하면 안 됩니다. 이렇게 해야 case-sensitive Linux filesystem과
case-insensitive macOS filesystem 사이를 오갈 때 조용한 데이터 손실을 막을 수
있습니다.

Copy backup은 hard-link 관계, extended attribute, macOS resource fork를
보존하지 않습니다. Lattice는 이런 파일을 기본적으로 막습니다. 영향을 받는
파일을 직접 검토한 뒤에만 `--allow-metadata-loss`를 사용합니다.

```bash
lattice backup --allow-metadata-loss <service>
```

## 9. 고급 복원 옵션

복원된 파일이 service repo를 가리키게 하려면 symlink restore mode를 사용합니다.

```bash
lattice service add <service> --root <path> --include <pattern> --symlink
```

Repo file에는 placeholder를 두고 restore된 file에는 environment variable을
render하고 싶으면 template mode를 사용합니다.

```bash
lattice service add <service> --root <path> --include <pattern> --template
```

Symlink와 template mode가 모두 켜져 있으면, template 값이 render된 파일은 repo
copy의 placeholder를 보존하기 위해 regular file로 복원됩니다.

특정 OS나 hostname에서만 service를 활성화할 수 있습니다.

```bash
lattice service add <service> --root <path> --include <pattern> --os macos
```

## 10. Automation과 JSON output

Script, CI job, agent가 안정적인 machine-readable output을 필요로 할 때는
`--json`을 사용합니다.

```bash
lattice status --json shell
lattice plan --json shell
lattice backup --dry-run --json shell
lattice diff --json shell
lattice restore --dry-run --json shell
```

쓰기 flow에서는 dry-run JSON 명령을 먼저 사용합니다. 계획을 parsing하고 예상 밖
파일, directory, entry, conflict가 있으면 멈춥니다.

```bash
plan="$(lattice plan --json shell)"
printf '%s\n' "$plan" | jq '.conflicts'
```

`--only`와 `--exclude`로 status, backup, diff, restore 작업을 특정 tracked path로
좁힐 수 있습니다. Shell이 glob selector를 expand하지 않도록 quote합니다.

```bash
lattice status --json --only config.toml shell
lattice backup --dry-run --json --only config.toml shell
lattice diff --json --exclude 'cache/**' shell
lattice restore --dry-run --json --only config.toml shell
```

Selector는 service-group orchestration이 아니라 path-scoped 도구입니다. 바뀐 설정
파일 하나만 백업하거나, diff에서 noisy generated state를 제외하는 것처럼 작고
검토 가능한 작업에 사용합니다.

## 11. Prompt UI

Prompt 기반 UI 열기:

```bash
lattice tui
```

TUI는 CLI와 같은 config model을 사용합니다. 별도 source of truth가 아니라 편의
계층입니다.

## 문제 해결

먼저 config를 검증합니다.

```bash
lattice validate
```

Service 상태를 확인합니다.

```bash
lattice service show shell
lattice status shell
```

백업이 secret-looking content 때문에 실패하면, `--allow-secret-looking-files`를
쓰기 전에 해당 파일을 직접 검토합니다.

Restore conflict가 나오면 다음을 실행합니다.

```bash
lattice plan zsh
lattice restore --dry-run zsh
lattice diff zsh
```

Repo copy가 local copy를 대체해야 한다는 것을 확인했을 때만 `restore --force`를
사용합니다.

Forced restore가 특수 filesystem entry를 교체해야 하는 경우, Lattice는 이를
regular file content처럼 복사하지 않고 snapshot에 metadata로 남깁니다.

## 개발 검증

Local development:

```bash
cargo run -p xtask -- verify
```

Docker-backed Linux verification:

```bash
cargo run -p xtask -- linux-verify
```

릴리스 전 무거운 quality gate:

```bash
cargo run -p xtask -- quality
```
