# Stability Reference

[English](stability.md) | 한국어 | [문서 인덱스](../README.ko.md)

이 문서는 v1.0.0 public stable CLI contract를 정의합니다. Stable line에서 script,
agent, 사용자가 의존할 수 있는 부분과 제품 범위 밖에 남는 부분을 정리합니다.

## Stable command surface

Stable command name은 다음과 같습니다.

- Setup/diagnostics: `init`, `doctor`, `validate`, `bootstrap check`, `discover`;
- Service: `service list`, `service show`, `service add`, `service remove`;
- App shortcut: `app list`, `app show`, `app add`;
- Tracked path/permission: `include add/remove`, `exclude add/remove`,
  `permission set/remove`;
- Safety-first operation: `status`, `plan`, `backup`, `restore`, `diff`;
- Snapshot/rollback inspection: `snapshot list/show/prune`, `undo`;
- Git repo helper: `repo status`, `repo pull`, `repo commit`, `repo push`;
- Read-only group: `group list`, `group show`, `group status`, `group plan`;
- Context inspection: `context show`.

Patch release는 문서화된 command를 rename/remove하지 않아야 합니다. 새 command는
additive여야 하며 dry-run-first safety model을 약화하면 안 됩니다.

## Stable config key

Stable config는 다음을 포함합니다.

- Global config version, local context label, named service group;
- Service `root`, optional `repo`, `include`, `exclude`, `permissions`,
  `restore`, `hooks`, `secrets`, `conditions`;
- `os`, `hostname`, explicit `contexts` label 조건;
- Secret value를 읽거나 저장하지 않는 secret reference metadata.

미래 config key는 additive여야 합니다. Breaking config change에는 migration note와
명시적인 release decision이 필요합니다.

## Stable JSON contract

Automation은 [JSON Output Reference](json-output.ko.md)에 문서화된 top-level key에
의존할 수 있습니다. Patch release는 문서화된 top-level key를 remove/rename하지 않아야
합니다. 새 field는 optional/additive여야 합니다.

Best-effort 또는 platform-dependent 값은 script가 의존하기 전에 문서화해야 합니다.

## Safety behavior

다음 safety behavior는 stable contract의 일부이며 약화되면 release blocker입니다.

- Write 전 `plan`과 `--dry-run` preflight;
- Restore 전 conflict check;
- Forced overwrite 전 snapshot, rollback 전 `undo` inspection;
- Traversal, symlink escape, root/repo overlap, portable path collision guard;
- Secret-looking content check와 non-disclosing secret reference handling;
- Hard link, xattr, resource fork에 대한 metadata-loss warning.

## Deprecation policy

Removal 전 deprecation은 changelog, migration guide, 관련 reference doc에 문서화해야
합니다. Unsafe behavior는 더 빨리 제거할 수 있지만 release note가 safety reason을
설명해야 합니다.

## v1.0 범위 밖

Stable line은 package installation, app installation, remote repo creation,
secret value management, group backup/restore mutation, GUI, database-backed state,
plugin/MCP surface, yadm-style per-file alternate, chezmoi-style full conditional
template을 포함하지 않습니다.
