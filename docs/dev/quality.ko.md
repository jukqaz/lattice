# Quality Gates

[English](quality.md) | 한국어 | [문서 인덱스](../README.ko.md)

Release-oriented change를 준비하거나 CI quality job을 로컬에서 재현할 때 이 문서를
사용합니다.

## 빠른 로컬 게이트

일반 개발 작업에서는 shared verification harness를 실행합니다.

```bash
cargo run -p xtask -- verify
git diff --check
```

`xtask verify`는 formatting check, Clippy, workspace test, CLI smoke test,
product-surface harness check를 실행합니다. `wasm32-wasip2` target이 설치되어
있으면 `lattice-core`의 non-Unix compile check도 실행합니다.

Non-Unix check까지 로컬에서 확인하려면 optional target을 설치합니다.

```bash
rustup target add wasm32-wasip2
cargo run -p xtask -- verify
```

## 전체 Quality Gate

Release/CI quality gate는 아래 도구들이 `PATH`에 있어야 합니다.

```bash
cargo install cargo-deny --locked
cargo install cargo-machete --locked
cargo install cargo-llvm-cov --locked
cargo install typos-cli --locked
rustup component add llvm-tools-preview
```

그 다음 실행합니다.

```bash
cargo run -p xtask -- quality
```

`xtask quality`는 먼저 `xtask verify`를 실행한 뒤 아래 명령을 실행합니다.

```bash
cargo-deny check
cargo-machete --with-metadata --skip-target-dir
typos --config _typos.toml
cargo llvm-cov --workspace --all-features --locked --lcov --output-path target/llvm-cov/lcov.info
```

## 실제 HOME Read-Only Health Check

릴리스 태그 전에는 현재 binary를 실제 `HOME`/XDG 환경에 대해 dogfood할 수
있습니다. 이 스크립트는 config 생성, service 등록, backup, restore, snapshot
prune, repo commit/push를 실행하지 않습니다.

```bash
scripts/real-home-readonly-health-check.sh
```

이미 설치된 binary를 확인하려면 `LATTICE_BIN=/path/to/lattice`를 지정합니다.
또는 먼저 `cargo build -p lattice`를 실행해서 `target/debug/lattice`가 있게
합니다. 이 스크립트는 live HOME 점검 중 build/cache side effect를 만들지 않기
위해 `cargo run` fallback을 사용하지 않습니다. 실행하는 read-only 명령은
`doctor`, `validate`, `bootstrap check`, `service list`, service별
`status --json`/`plan --json`, `discover --json`, `group list --json`, group별
`status --json`/`plan --json`입니다. 개별 진단 명령의 non-zero exit은 health
finding으로 보고하고 script도 non-zero로 종료합니다. 따라서 CI나 release checklist가
실패한 진단을 실수로 pass로 취급하지 않습니다.

실제 HOME mutation을 사용자가 명시적으로 승인하지 않는 한 이 스크립트를
`init`, `backup`, `restore`, `adopt`, `track`, `snapshot prune`, `undo --yes`,
repo push/commit 흐름으로 대체하지 마세요.

## Workflow Lint

`.github/workflows/ci.yml`이 바뀌면 가능할 때 `actionlint`도 실행합니다.

```bash
actionlint .github/workflows/ci.yml
```

현재 CI workflow는 quality tool을 설치한 뒤 `xtask quality`를 실행합니다. 로컬에서
required quality tool missing 오류가 나면 product test 실패가 아니라 개발 머신이
아직 bootstrap되지 않았다는 뜻입니다.
