# Change Policy

[English](change-policy.md) | 한국어 | [문서 인덱스](../README.ko.md)

이 정책은 stable line의 변경을 안내합니다. Lattice의 public surface를 안정적으로
유지하되 작은 safety/doc fix를 막지 않기 위한 기준입니다.

## Compatibility Promise

Stable line에서는 다음을 보존해야 합니다.

- 문서화된 command name과 subcommand;
- 문서화된 config key와 default XDG path behavior;
- automation surface의 문서화된 JSON top-level key;
- restore safety behavior: dry run, conflict check, snapshot, undo;
- 값을 읽지 않고 reference만 저장하는 secret handling behavior.

Breaking change에는 migration note, changelog entry, 명시적 release decision이 필요합니다.
Unsafe behavior는 더 빠르게 제거할 수 있지만 release note에 safety reason을 설명해야 합니다.

## Versioning Rules

- Patch release는 bug, docs, release tooling, compatibility regression을 수정합니다.
- Minor release는 opt-in command, optional JSON field, additive config key를 추가할 수 있습니다.
- Major release는 deprecation과 migration guidance 후 stable contract를 제거하거나 바꿀 수 있습니다.

## JSON Compatibility

Automation user는 문서화된 top-level field에 의존할 수 있어야 합니다. 새 field는 가능하면
optional로 추가합니다. Patch release에서 문서화된 field를 rename/remove하지 않습니다. Best-effort
또는 unstable field라면 그 field를 도입하는 release 전에 JSON reference에 표시합니다.

## Safety Regression Policy

Safety regression은 release blocker입니다. Packaging 또는 public onboarding을 쉽게 하려고 다음
게이트를 약화하지 않습니다.

- restore dry-run과 conflict check;
- forced overwrite 전 snapshot;
- traversal, symlink escape, portable path collision guard;
- secret-looking content check와 non-disclosing secret reference;
- metadata-loss warning.

## Release Approval

Tag와 GitHub Release는 release step에 대한 명시 승인 후에만 생성합니다. Tag 전에는 문서화된
local gate를 실행하고, GitHub Actions를 확인하며, release/Linear closeout에 install-smoke evidence를
기록합니다.
