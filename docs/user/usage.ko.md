# 사용자 가이드

[English](usage.md) | 한국어 | [문서 인덱스](../README.ko.md) |
[Repository README](../../README.ko.md)

이 문서는 Lattice의 공개 사용자 가이드다. LLM 전용 작업 규칙은
`docs/llm/` 아래에 있으며 영어로 유지한다.

## 설치

checkout에서 설치:

```bash
cargo install --path crates/lattice-cli
```

GitHub에서 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --locked
```

tagged release 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag vX.Y.Z --locked
```

## 빠른 시작

```bash
lattice init
lattice doctor
lattice validate
lattice status codex
lattice backup --dry-run codex
lattice restore --dry-run codex
```

## 서비스

Lattice는 파일을 service 단위로 관리한다. Service file 위치:

```text
~/.config/lattice/services/*.toml
```

service에서 `repo`를 생략하면 Lattice는 backup file을 아래 위치에 저장한다.

```text
$XDG_DATA_HOME/lattice/repos/<service-name>
```

custom service 생성:

```bash
lattice service add editor --root ~/.config/editor --include settings.toml
lattice service show editor
lattice backup --dry-run editor
```

## 안전 기본값

- 명백한 secret 형태의 내용은 `--allow-secret-looking-files`를 넘기지 않으면
  백업하지 않는다.
- restore는 `--force`를 넘기지 않으면 충돌하는 local file을 덮어쓰지 않는다.
- forced restore는 덮어쓴 file을 XDG state 아래에 snapshot으로 남긴다.
- restore path, permission rule, manifest entry는 service root 내부에 있어야 한다.
- secret command는 metadata만 저장한다. secret 값을 읽거나 출력하지 않는다.

## 검증

local development:

```bash
cargo run -p xtask -- verify
```

Docker-backed Linux verification:

```bash
cargo run -p xtask -- linux-verify
```

heavier quality gate:

```bash
cargo run -p xtask -- quality
```
