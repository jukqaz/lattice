# Stability Reference

English | [한국어](stability.ko.md) | [Documentation Index](../README.md)

This reference defines the v1.0.0 public stable CLI contract. It describes what
scripts, agents, and users can rely on across the stable line, and what remains
outside the product scope.

## Stable Command Surface

The stable command names are:

- setup and diagnostics: `init`, `doctor`, `validate`, `bootstrap check`, `discover`;
- services: `service list`, `service show`, `service add`, `service remove`;
- app shortcuts: `app list`, `app show`, `app add`;
- tracked paths and permissions: `include add/remove`, `exclude add/remove`,
  `permission set/remove`;
- safety-first operations: `status`, `plan`, `backup`, `restore`, `diff`;
- snapshots and rollback inspection: `snapshot list/show/prune`, `undo`;
- Git repo helpers: `repo status`, `repo pull`, `repo commit`, `repo push`;
- read-only groups: `group list`, `group show`, `group status`, `group plan`;
- context inspection: `context show`.

Patch releases should not rename or remove documented commands. New commands must
be additive and must not weaken the dry-run-first safety model.

## Stable Config Keys

Stable config covers:

- global config version, local context labels, and named service groups;
- service `root`, optional `repo`, `include`, `exclude`, `permissions`,
  `restore`, `hooks`, `secrets`, and `conditions`;
- service conditions for `os`, `hostname`, and explicit `contexts` labels;
- secret references that store backend metadata and do not read or persist secret
  values.

Unknown future config keys should be additive. Breaking config changes require a
migration note and an explicit release decision.

## Stable JSON Contract

Automation may depend on documented top-level keys in
[JSON Output Reference](json-output.md). Patch releases should not remove or
rename documented top-level keys. New fields should be optional and additive.

Best-effort or platform-dependent values must be documented before scripts are
expected to depend on them.

## Safety Behavior

The following safety behavior is part of the stable contract and blocks release
if weakened:

- `plan` and `--dry-run` preflight before writes;
- conflict checks before restore;
- snapshots before forced overwrite and `undo` inspection before rollback;
- traversal, symlink escape, root/repo overlap, and portable path collision
  guards;
- secret-looking content checks and non-disclosing secret reference handling;
- metadata-loss warnings for hard links, xattrs, and resource forks.

## Deprecation Policy

Deprecations must be documented in the changelog, migration guide, and relevant
reference docs before removal. Unsafe behavior can be removed faster, but the
release notes must explain the safety reason.

## Out Of Scope For v1.0

The stable line does not include package installation, app installation, remote
repo creation, secret value management, group backup/restore mutation, GUI,
database-backed state, plugin/MCP surfaces, yadm-style per-file alternates, or
chezmoi-style full conditional templates.
