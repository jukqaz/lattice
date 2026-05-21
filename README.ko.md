# Lattice

[English](README.md) | 한국어

Lattice는 서비스 단위 dotfiles를 백업하고 복원하는 작은 Rust CLI다.

첫 지원 preset은 `codex`다. `~/.codex` 아래에서 사용자가 관리하는 파일은
백업하고, auth, sessions, logs, sqlite database, cache, worktree, generated
file 같은 runtime state는 제외한다.

## 설치

checkout에서 개발 중 설치:

```bash
cargo install --path crates/lattice-cli
```

GitHub repository에서 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --locked
```

tagged release 설치:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.3.0 --locked
```

private repository 또는 SSH 환경:

```bash
cargo install --git ssh://git@github.com/jukqaz/lattice.git lattice --locked
```

개발 중 실행:

```bash
cargo run -- init --force
cargo run -- status codex
cargo run -- backup --dry-run codex
cargo run -- restore --dry-run codex
```

## 문서

- 문서 인덱스: [docs/README.ko.md](docs/README.ko.md)
- 사용자 가이드: [docs/user/usage.ko.md](docs/user/usage.ko.md)
- 제품 범위: [docs/product/mvp-scope.ko.md](docs/product/mvp-scope.ko.md)
- 변경 로그: [CHANGELOG.ko.md](CHANGELOG.ko.md)
- LLM 에이전트 가이드: [docs/llm/README.md](docs/llm/README.md)
- LLM 브랜치/릴리스 규칙:
  [docs/llm/branch-release-policy.md](docs/llm/branch-release-policy.md)

## 기본 위치

Lattice 설정과 상태는 XDG 위치를 사용한다.

```text
~/.config/lattice/lattice.toml
~/.config/lattice/services/*.toml
~/.local/share/lattice/repos/*
~/.local/state/lattice/
~/.cache/lattice/
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

## 안전 기본값

- 명백한 secret 형태의 내용은 기본적으로 백업을 막는다.
- restore는 충돌하는 local file을 기본적으로 덮어쓰지 않는다.
- `restore --force`는 덮어쓰기 전에 XDG state 아래에 snapshot을 만든다.
- restore path, permission rule, manifest entry는 service root 내부에 있어야 한다.
- secret command는 metadata만 저장하고 secret 값을 읽거나 출력하지 않는다.

## 검증

일반 검증:

```bash
cargo run -p xtask -- verify
```

Docker 기반 Linux 검증:

```bash
cargo run -p xtask -- linux-verify
```

무거운 품질 게이트:

```bash
cargo run -p xtask -- quality
```
