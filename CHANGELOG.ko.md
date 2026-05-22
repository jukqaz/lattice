# 변경 로그

[English](CHANGELOG.md) | 한국어 | [문서 인덱스](docs/README.ko.md)

## Unreleased

### 추가

- 일반 service config 위의 public app-catalog surface로 `lattice app list/show/add`.
- 새 머신 readiness check를 위한 human/JSON `lattice bootstrap check`.
- backup/restore 전 권장 preflight surface인 human/JSON `lattice plan`.
- forced restore snapshot을 확인하고 rollback을 dry-run하며 history를 보수적으로
  정리하기 위한 `lattice snapshot list/show/prune`과 `lattice undo`.
- config를 변경하지 않고 local service 후보를 보수적으로 찾는 human/JSON
  `lattice discover`.
- CLI help와 유지 관리 중인 문서가 app/service terminology를 유지하고 예전 catalog
  wording으로 되돌아가지 않도록 `cargo run -p xtask -- verify`에
  product-surface verification 추가.

### 변경

- `lattice init`은 이제 tool-specific service를 기본 생성하지 않고 범용 Lattice
  config와 storage directory만 만들며, 다음 safe bootstrap command를 출력한다.
- README와 사용자 문서는 app/service 예시에서 시작하되 app entry를 제품 중심이
  아니라 선택적 shortcut으로 유지한다.
- v0.4 후보 command surface에 맞춰 workspace package version을 `0.4.0`으로
  올렸다.

### 제거

- old public catalog command/flag wording을 제거하고 `app`과 generic service
  config로 대체했다.

## v0.3.3

### 수정

- backup 또는 restore 전에 service root/repo overlap을 거부해 recursive copy와
  self-restore를 막는다.
- portable UTF-8이 아니거나 control character를 포함하거나 Unicode
  normalization과 case folding 이후 충돌하는 추적 path를 거부한다.
- copy backup이 보존하지 못하는 hard link, extended attribute, macOS resource
  fork를 기본적으로 거부한다.
- xattr list를 지원하지 않는 filesystem에서는 이를 non-fatal로 처리해 모든
  backup이 실패하지 않게 한다.

### 추가

- 검토 후 metadata loss를 허용할 수 있는 파일을 위한
  `backup --allow-metadata-loss`와 `adopt --allow-metadata-loss`.

## v0.3.2

### 수정

- forced restore가 추적된 디렉터리로 덮어써야 하는 위치에 Unix socket 같은 특수
  filesystem entry가 있을 때, 이를 일반 파일처럼 복사하지 않고 metadata
  snapshot으로 남긴다.

### 변경

- 공개 문서에서 Lattice를 generic service-scoped dotfiles manager로 설명하도록
  조정했다. 구체적인 명령 예시는 service 예시이며 제품 방향이 아니다.

## v0.3.1

### 수정

- include된 빈 디렉터리를 backup manifest에 보존하고 restore 시 다시 생성한다.
  file이 없어도 의미가 있는 empty skill directory 같은 service path를 커버한다.

## v0.3.0

public git-distributed Lattice 릴리스 후보.

### 추가

- `lattice-core`, `lattice` CLI, `xtask`로 나뉜 Rust workspace.
- service, include/exclude pattern, permission, preset, repository operation,
  secret metadata, `track`, `adopt`, `diff`, `tui` 관리 command.
- `$XDG_DATA_HOME/lattice/repos` 아래 service별 기본 repo 위치.
- `codex`, `git`, `zsh`, `mise`, `ssh` preset.
- restore safety check, overwrite snapshot, symlink restore mode, OS/hostname
  condition, 단순 environment-variable template rendering.
- dependency policy, typo scan, unused dependency check, LCOV 생성,
  Docker-backed Linux verification, GitHub Actions matrix verification.
- 영어/한국어 공개 문서와 영어 전용 LLM workflow guidance.

### 변경

- Lattice는 git-distributed only로 유지한다. crates는 `publish = false`다.
- release verification은 `cargo run -p xtask -- verify`, `linux-verify`,
  `quality`로 모은다.
- `doctor`는 가벼운 environment check로 유지하고, config parsing은
  `validate`가 담당한다.

### 보안

- 명백한 secret-looking content는 명시적 bypass 없이는 backup을 막는다.
- secret command는 `rbw`, `bw` metadata만 저장하며 secret 값을 읽거나
  출력하지 않는다.
- path traversal, unsafe symlink, manifest escape, restore conflict, binary diff
  exposure case를 harness test로 검증한다.

## v0.2.0

- restore conflict detection과 forced-restore snapshot.
- minimal lifecycle hook.
- secret-looking content guard.
- `validate`와 강화된 isolated dry-run harness coverage.

## v0.1.0

- service-scoped backup/restore를 위한 초기 Rust CLI spike와 명시적인 example
  service.
- XDG path, TOML config, `codex` preset, permission manifest, backup, restore,
  status, 첫 Rust `xtask` verification harness.
