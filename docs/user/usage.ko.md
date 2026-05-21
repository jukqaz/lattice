# 사용자 가이드

[English](usage.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

이 문서는 Lattice를 직접 사용하는 사람을 위한 가이드입니다. LLM 전용 작업
규칙은 [docs/llm](../llm/) 아래에 있습니다.

## Lattice가 관리하는 것

Lattice는 이름이 붙은 service의 선택된 파일을 백업하고 복원합니다. Service는
하나의 도구나 앱 설정 root를 뜻합니다. 이 가이드는 명령을 구체적으로 보여주기
위해 내장 `codex` service를 예시로 사용하며, 같은 모델은 사용자가 정의한 어떤
service에도 적용됩니다.

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

## 1. 설치

최신 tagged release 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.2 --locked
```

Lattice를 개발 중이면 local checkout에서 설치:

```bash
cargo install --path crates/lattice-cli
```

설치된 binary 확인:

```bash
lattice --version
```

## 2. 로컬 설정 만들기

global config와 예시 `codex` service를 만듭니다.

```bash
lattice init
```

환경과 service 설정을 확인합니다.

```bash
lattice doctor
lattice validate
lattice service list
lattice status codex
```

기본 설정 파일 위치:

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/codex.toml
```

예시 service에서 `repo`를 생략하면 backup copy는 여기에 저장됩니다.

```text
~/.local/share/lattice/repos/codex
```

## 3. 첫 백업 만들기

항상 dry-run을 먼저 실행합니다.

```bash
lattice backup --dry-run codex
```

출력에는 copy될 파일과 추적할 빈 디렉터리가 나옵니다. Service repo에 쓰기
전에 이 목록을 확인합니다.

백업 실행:

```bash
lattice backup codex
```

live service root와 repo copy 사이 drift 확인:

```bash
lattice diff codex
```

출력이 없으면 추적 대상 파일 기준으로 내용 차이가 없다는 뜻입니다.

## 4. 안전하게 복원하기

복원도 먼저 preview합니다.

```bash
lattice restore --dry-run codex
```

충돌을 덮어쓰지 않고 복원:

```bash
lattice restore codex
```

local file 충돌이 있고 repo copy를 의도적으로 우선하려면 `--force`를 사용합니다.

```bash
lattice restore --force codex
```

Forced restore는 쓰기 전에 덮어쓸 파일을 XDG state 아래에 snapshot으로 남깁니다.

```text
~/.local/state/lattice/snapshots/
```

## 5. 백업 repo 동기화

Service repo는 일반 directory입니다. Git을 직접 써도 되고 Lattice helper를 써도
됩니다.

```bash
lattice repo status codex
lattice repo pull codex
lattice repo commit --message "backup codex config" codex
lattice repo push codex
```

private GitHub repository를 쓸 때:

1. private remote repository를 만듭니다.
2. service repo에 remote를 추가합니다.
3. backup commit 이후 `lattice repo push <service>`를 실행합니다.

Lattice는 remote repository를 자동 생성하지 않습니다. credential, 접근 권한,
repository 소유권을 명시적으로 관리하기 위해서입니다.

## 6. 다른 서비스 추가하기

앱 설정 directory 하나를 service로 만듭니다. 실제 service name, root, include
pattern은 원하는 값으로 바꿉니다.

```bash
lattice service add <service> --root <path> --include <pattern>
lattice service show <service>
lattice backup --dry-run <service>
```

나중에 추적 path를 추가하거나 제거합니다.

```bash
lattice include add <service> <pattern>
lattice include remove <service> <pattern>
lattice exclude add <service> <pattern>
lattice exclude remove <service> <pattern>
```

복원 권한 보존:

```bash
lattice permission set <service> <path> 0600
lattice permission remove <service> <path>
```

기존 파일을 service로 가져오기:

```bash
lattice track <service> <path>
lattice adopt <service> <path>
```

## 7. Preset 사용하기

Preset은 흔한 도구의 include/exclude 형태를 제공합니다.

```bash
lattice preset list
lattice preset show codex
```

Preset 기반 service 생성:

```bash
lattice service add <service> --root <path> --preset <preset>
```

내장 preset은 `codex`, `git`, `zsh`, `mise`, `ssh`입니다.

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
lattice service add <service> --root <path> --preset <preset> --os macos
```

## 10. Prompt UI

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
lattice service show codex
lattice status codex
```

백업이 secret-looking content 때문에 실패하면, `--allow-secret-looking-files`를
쓰기 전에 해당 파일을 직접 검토합니다.

Restore conflict가 나오면 다음을 실행합니다.

```bash
lattice restore --dry-run codex
lattice diff codex
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
